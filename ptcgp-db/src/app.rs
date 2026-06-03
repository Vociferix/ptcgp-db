use std::time::Duration;

use dioxus::prelude::*;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::StreamExt as _;
use ptcgp_db_core::save_data::{CardVersionId, FilterConfig, Theme};
use ptcgp_db_core::storage::Storage as _;
use ptcgp_db_core::{AppSettings, ProfileStore, SavedQueries};
use ptcgp_db_data::CardSource;

use crate::pages::OnboardingPage;
use crate::routes::Route;

#[cfg(target_arch = "wasm32")]
use crate::drive::{DriveState, DriveSyncData};

// ---------------------------------------------------------------------------
// Dev helpers
// ---------------------------------------------------------------------------

/// Returns true when the `PTCGP_SKIP_ONBOARDING` environment variable is set.
/// Always false on WASM (env vars are unavailable there).
#[cfg(not(target_arch = "wasm32"))]
fn skip_onboarding() -> bool {
    std::env::var("PTCGP_SKIP_ONBOARDING").is_ok()
}

#[cfg(target_arch = "wasm32")]
fn skip_onboarding() -> bool {
    false
}

// ---------------------------------------------------------------------------
// Per-page persistent state — survives navigation
// ---------------------------------------------------------------------------

/// Persisted filter + UI state for the Trade page.
#[derive(Clone)]
pub(crate) struct TradePageState {
    pub config: FilterConfig,
    pub show_unobtainable: bool,
    pub active_tab: u8, // 0=Shares, 1=Trades, 2=Candidates
}

impl Default for TradePageState {
    fn default() -> Self {
        Self {
            config: FilterConfig {
                goal: 1,
                ..FilterConfig::default()
            },
            show_unobtainable: false,
            active_tab: 0,
        }
    }
}

/// Persisted filter config for the Summary page.
#[derive(Clone)]
pub(crate) struct SummaryPageState {
    pub config: FilterConfig,
}

impl Default for SummaryPageState {
    fn default() -> Self {
        Self {
            config: FilterConfig {
                goal: 1,
                obtainable: Some(true),
                ..FilterConfig::default()
            },
        }
    }
}

/// Where the user came from when navigating to the Card Detail page.
/// Set immediately before each `nav.push(Route::CardDetailPage { ... })`.
#[derive(Clone, PartialEq, Default)]
pub(crate) enum CardDetailOrigin {
    #[default]
    Catalog,
    Trade,
}

// ---------------------------------------------------------------------------
// Platform-specific storage type
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
pub type AppStorage = ptcgp_db_core::WebStorage;
#[cfg(not(target_arch = "wasm32"))]
pub type AppStorage = ptcgp_db_core::FileStorage;

// ---------------------------------------------------------------------------
// Platform-specific helpers
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
async fn open_storage() -> Result<AppStorage, String> {
    ptcgp_db_core::WebStorage::open()
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn open_storage() -> Result<AppStorage, String> {
    ptcgp_db_core::FileStorage::open().map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn sleep(dur: Duration) {
    let mut remaining = dur.as_millis();
    while remaining > u128::from(u32::MAX) {
        gloo_timers::future::TimeoutFuture::new(u32::MAX).await;
        remaining -= u128::from(u32::MAX);
    }
    gloo_timers::future::TimeoutFuture::new(remaining as u32).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use tokio::time::sleep;

// ---------------------------------------------------------------------------
// Auto-save signal type and helper
// ---------------------------------------------------------------------------

/// Sent through the auto-save coroutine's channel to schedule a debounced save.
pub(crate) struct ScheduleSave;

/// Schedules a debounced `ProfileStore` save. Call this inside any component
/// that mutates the store. The actual save fires 2 s after the last call.
#[allow(dead_code)]
pub(crate) fn schedule_save() {
    use_coroutine_handle::<ScheduleSave>().send(ScheduleSave);
}

/// Sets the owned count for a card version in the first active profile, then schedules a save.
///
/// Used by catalog and card detail components to commit `CountSpinner` changes. No-ops when
/// no profile is loaded or the active-profile list is empty.
pub(crate) fn set_card_count(
    cv_id: usize,
    new_count: u32,
    mut store: Signal<Option<ProfileStore<AppStorage>>>,
) {
    let name = {
        let s = store.read();
        let Some(s) = s.as_ref() else { return };
        s.active_profile_names().first().cloned()
    };
    let Some(name) = name else { return };
    {
        let mut s = store.write();
        let Some(s) = s.as_mut() else { return };
        let _ = s.set_owned_count(&name, CardVersionId(cv_id), new_count);
    }
    schedule_save();
}

const fn favicon() -> Asset {
    let mut srcs = CardSource::ALL;

    while let Some((src, tail)) = srcs.split_first() {
        srcs = tail;
        if src.name().as_str().eq_ignore_ascii_case("Pack") {
            return src.icon();
        }
    }

    panic!("Card source 'Pack' not found")
}

const FAVICON: Asset = favicon();

// ---------------------------------------------------------------------------
// Drive startup sync (web only)
// ---------------------------------------------------------------------------

/// Attempts a silent Drive token acquisition on startup and, if successful, loads the Drive
/// sync file and overwrites the in-memory app state with its contents.
///
/// This runs concurrently with (and after) the IndexedDB load. The Drive version wins when
/// it differs from local because Drive is the cross-device source of truth once enabled.
#[cfg(target_arch = "wasm32")]
async fn startup_drive_sync(
    drive_state: Signal<DriveState>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
    settings: Signal<AppSettings>,
    queries: Signal<SavedQueries>,
) {
    let mut ds = drive_state;
    let token = match crate::drive::acquire_token_silent().await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Drive silent auth failed on startup: {e}");
            ds.set(DriveState::Error(
                "Could not reconnect to Google Drive. Open Settings to reconnect.".to_string(),
            ));
            return;
        }
    };

    // Wait for IndexedDB load to finish before overwriting with Drive data.
    loop {
        if store.read().is_some() {
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    crate::drive::load_from_drive(&token, drive_state, store, settings, queries).await;
}

// ---------------------------------------------------------------------------
// App root
// ---------------------------------------------------------------------------

/// Root component. Provides all shared contexts, handles async storage
/// initialization, and manages the auto-save debounce coroutine.
#[component]
pub fn App() -> Element {
    // Contexts are always provided unconditionally (hooks must not be conditional).
    // They start empty/default and are populated once storage opens.
    let mut store: Signal<Option<ProfileStore<AppStorage>>> =
        use_context_provider(|| Signal::new(None));
    let mut settings: Signal<AppSettings> =
        use_context_provider(|| Signal::new(AppSettings::default()));
    let mut queries: Signal<SavedQueries> =
        use_context_provider(|| Signal::new(SavedQueries::default()));
    // Persistent catalog filter: survives navigation away and back. Other pages may
    // write to this before navigating to the catalog to pre-set a filter.
    let _: Signal<FilterConfig> = use_context_provider(|| Signal::new(FilterConfig::default()));
    // Per-page persistent states — each page reads on mount and writes on unmount.
    let _: Signal<TradePageState> = use_context_provider(|| Signal::new(TradePageState::default()));
    let _: Signal<SummaryPageState> =
        use_context_provider(|| Signal::new(SummaryPageState::default()));
    // Tracks which page the user navigated to CardDetailPage from, for the back button label/route.
    let _: Signal<CardDetailOrigin> =
        use_context_provider(|| Signal::new(CardDetailOrigin::default()));
    // Drive sync state (web only; always provided so hook count is stable per platform).
    #[cfg(target_arch = "wasm32")]
    let mut drive_state: Signal<DriveState> =
        use_context_provider(|| Signal::new(DriveState::default()));
    let mut load_error: Signal<Option<String>> = use_signal(|| None);

    // Auto-save coroutine: waits for ScheduleSave signals, debounces 2 s, then
    // saves without holding a write lock across the await point.
    // Trigger from any component via schedule_save().
    let _auto_save = use_coroutine(move |mut rx: UnboundedReceiver<ScheduleSave>| async move {
        while rx.next().await.is_some() {
            // Drain any immediately-queued signals before starting the timer.
            while rx.try_recv().is_ok() {}
            sleep(Duration::from_secs(2)).await;
            // Drain signals that arrived during the sleep to coalesce rapid edits.
            while rx.try_recv().is_ok() {}

            // Snapshot all save data before any async work.
            #[cfg(target_arch = "wasm32")]
            let drive_bundle: Option<DriveSyncData> = {
                if drive_state.read().is_connected() {
                    store.read().as_ref().map(|s| DriveSyncData {
                        profiles: s.save_data_snapshot().clone(),
                        settings: settings.read().as_save_data().clone(),
                        queries: queries.read().as_save_data().clone(),
                    })
                } else {
                    None
                }
            };

            // Save profiles to local storage if dirty.
            let pending = {
                let guard = store.read();
                guard.as_ref().and_then(|s| {
                    if s.needs_save() {
                        Some((s.storage().clone(), s.save_data_snapshot().clone()))
                    } else {
                        None
                    }
                })
            };

            if let Some((storage, data)) = pending {
                match storage.save_profiles(&data).await {
                    Ok(()) => {
                        if let Some(s) = store.write().as_mut() {
                            s.mark_clean();
                        }
                    }
                    Err(e) => tracing::error!("auto-save failed: {e}"),
                }
            }

            // Save to Drive if connected.
            #[cfg(target_arch = "wasm32")]
            if let Some(bundle) = drive_bundle {
                crate::drive::save_to_drive(drive_state, &bundle).await;
            }
        }
    });

    // Desktop: save profile data synchronously on window close, before the component tears down.
    // This covers the case where the debounce timer hasn't fired yet.
    #[cfg(not(target_arch = "wasm32"))]
    use_drop(move || {
        let guard = store.read();
        if let Some(s) = guard.as_ref()
            && s.needs_save()
            && let Err(e) = s.storage().save_profiles_sync(s.save_data_snapshot())
        {
            tracing::error!("close-time save failed: {e}");
        }
    });

    // Web: save profile data when the page is hidden (tab closed, navigated away, etc.).
    // The browser allows IndexedDB transactions started during `visibilitychange` to complete
    // before tearing down the page, so this reliably covers debounce-window data loss.
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        use wasm_bindgen::prelude::*;

        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };

        let closure = Closure::<dyn FnMut()>::new(move || {
            let hidden = web_sys::window()
                .and_then(|w| w.document())
                .map(|d| d.visibility_state() == web_sys::VisibilityState::Hidden)
                .unwrap_or(false);
            if !hidden {
                return;
            }

            let guard = store.read();
            let Some(s) = guard.as_ref() else { return };
            if !s.needs_save() {
                return;
            }
            let storage = s.storage().clone();
            let data = s.save_data_snapshot().clone();
            drop(guard);

            wasm_bindgen_futures::spawn_local(async move {
                if let Err(e) = storage.save_profiles(&data).await {
                    tracing::error!("visibility-change save failed: {e}");
                }
            });
        });

        document
            .add_event_listener_with_callback("visibilitychange", closure.as_ref().unchecked_ref())
            .unwrap_or_else(|_| tracing::error!("failed to register visibilitychange listener"));

        // Intentional leak: the listener must live for the entire app lifetime.
        closure.forget();
    });

    // Apply .dark class to <html> based on theme setting.
    use_effect(move || {
        let theme = settings.read().theme();
        let js = match theme {
            Theme::Dark => "document.documentElement.classList.add('dark')",
            Theme::Light => "document.documentElement.classList.remove('dark')",
            Theme::System => concat!(
                "if(window.matchMedia('(prefers-color-scheme:dark)').matches)",
                "{document.documentElement.classList.add('dark')}",
                "else{document.documentElement.classList.remove('dark')}"
            ),
        };
        let _ = document::eval(js);
    });

    // Async initialization: open storage, load all persisted state.
    use_effect(move || {
        spawn(async move {
            let storage = match open_storage().await {
                Ok(s) => s,
                Err(e) => {
                    load_error.set(Some(e));
                    return;
                }
            };

            let loaded_store = match ProfileStore::load(storage.clone()).await {
                Ok(s) => s,
                Err(e) => {
                    load_error.set(Some(e.to_string()));
                    return;
                }
            };
            let loaded_settings = AppSettings::load(&storage).await.unwrap_or_default();
            let loaded_queries = SavedQueries::load(&storage).await.unwrap_or_default();

            settings.set(loaded_settings);
            queries.set(loaded_queries);
            store.set(Some(loaded_store));
        });
    });

    // Web: attempt silent Drive auth on startup if the user previously connected.
    // Runs after the storage init above; if Drive data is loaded it overwrites the
    // IndexedDB-loaded state so the most recent version (across devices) wins.
    #[cfg(target_arch = "wasm32")]
    use_effect(move || {
        if !crate::drive::is_drive_enabled() {
            return;
        }
        drive_state.set(DriveState::Connecting);
        spawn(async move {
            startup_drive_sync(drive_state, store, settings, queries).await;
        });
    });

    let body = if let Some(ref err) = *load_error.read() {
        rsx! {
            div { class: "flex items-center justify-center h-screen text-red-600 p-8",
                "Failed to open storage: {err}"
            }
        }
    } else {
        match &*store.read() {
            None => rsx! {
                div { class: "flex items-center justify-center h-screen", "Loading…" }
            },
            Some(s) if s.is_first_run() && !skip_onboarding() => rsx! {
                OnboardingPage {}
            },
            Some(_) => rsx! {
                Router::<Route> {}
            },
        }
    };

    rsx! {
        document::Stylesheet { href: asset!("/public/tailwind.css") }
        document::Link { rel: "icon", r#type: "image/png", href: FAVICON }
        {body}
    }
}

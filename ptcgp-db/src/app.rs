use dioxus::prelude::*;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::StreamExt as _;
use ptcgp_db_core::storage::Storage as _;
use ptcgp_db_core::{AppSettings, ProfileStore, SavedQueries};

use crate::pages::OnboardingStub;
use crate::routes::Route;

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

/// Platform-appropriate 2-second sleep used by the auto-save debounce.
#[cfg(target_arch = "wasm32")]
async fn sleep_2s() {
    gloo_timers::future::TimeoutFuture::new(2_000).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn sleep_2s() {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
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
    let mut load_error: Signal<Option<String>> = use_signal(|| None);

    // Auto-save coroutine: waits for mutation signals, debounces 2 s, then
    // saves without holding a write lock across the await point.
    //
    // Child components that mutate ProfileStore call:
    //   use_coroutine_handle::<()>().send(())
    let _auto_save = use_coroutine(move |mut rx: UnboundedReceiver<()>| async move {
        while rx.next().await.is_some() {
            // Drain any immediately-queued signals before starting the timer.
            while rx.try_recv().is_ok() {}
            sleep_2s().await;
            // Drain signals that arrived during the sleep to coalesce rapid edits.
            while rx.try_recv().is_ok() {}

            // Read save data without holding the write lock during the await.
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
        }
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

    if let Some(ref err) = *load_error.read() {
        return rsx! {
            div {
                class: "flex items-center justify-center h-screen text-red-600 p-8",
                "Failed to open storage: {err}"
            }
        };
    }

    match &*store.read() {
        None => rsx! {
            div {
                class: "flex items-center justify-center h-screen",
                "Loading…"
            }
        },
        Some(s) if s.is_first_run() => rsx! { OnboardingStub {} },
        Some(_) => rsx! { Router::<Route> {} },
    }
}

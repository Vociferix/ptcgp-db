use dioxus::prelude::*;
use ptcgp_db_core::{
    AppSettings, ProfileStore, ProfilesSaveData, SavedQueries, migrate_profiles,
    storage::Storage as _,
};

use crate::app::{AppStorage, schedule_save};

fn do_submit(
    name: Signal<String>,
    mut error: Signal<Option<String>>,
    mut store: Signal<Option<ProfileStore<AppStorage>>>,
) {
    let trimmed = name.read().trim().to_string();
    if trimmed.is_empty() {
        error.set(Some("Profile name is required.".into()));
        return;
    }
    if let Some(Err(e)) = store.write().as_mut().map(|s| s.create_profile(trimmed)) {
        error.set(Some(e.to_string()));
        return;
    }
    schedule_save();
}

// ---------------------------------------------------------------------------
// Import — shared text processing
// ---------------------------------------------------------------------------

async fn apply_import_text(
    text: String,
    mut store: Signal<Option<ProfileStore<AppStorage>>>,
    mut import_error: Signal<Option<String>>,
) {
    let raw: ProfilesSaveData = match serde_json::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            import_error.set(Some(format!("Invalid JSON: {e}")));
            return;
        }
    };
    let data = match migrate_profiles(raw) {
        Ok(d) => d,
        Err(e) => {
            import_error.set(Some(format!("Incompatible format: {e}")));
            return;
        }
    };
    if data.profiles.is_empty() {
        import_error.set(Some("The file contains no profiles.".into()));
        return;
    }
    let (snapshot, storage) = {
        let mut guard = store.write();
        let Some(s) = guard.as_mut() else { return };
        for profile in data.profiles {
            let pname = profile.name.clone();
            let _ = s.create_profile(pname.clone());
            let _ = s.replace_profile_counts(&pname, profile.owned_counts);
        }
        let snapshot = s.save_data_snapshot().clone();
        let storage = s.storage().clone();
        s.mark_clean();
        (snapshot, storage)
    };
    if let Err(e) = storage.save_profiles(&snapshot).await {
        tracing::error!("onboarding import save: {e}");
    }
    // Store now has profiles → App re-renders → OnboardingPage is replaced by Router
}

// ---------------------------------------------------------------------------
// Import — platform-specific file reading
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
fn do_import(
    evt: Event<FormData>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
    mut import_error: Signal<Option<String>>,
) {
    let Some(file) = evt.files().into_iter().next() else {
        return;
    };
    spawn(async move {
        let text = match file.read_string().await {
            Ok(t) => t,
            Err(e) => {
                import_error.set(Some(format!("Could not read file: {e}")));
                return;
            }
        };
        apply_import_text(text, store, import_error).await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn do_import(
    store: Signal<Option<ProfileStore<AppStorage>>>,
    mut import_error: Signal<Option<String>>,
) {
    spawn(async move {
        let Some(handle) = rfd::AsyncFileDialog::new()
            .add_filter("JSON", &["json"])
            .pick_file()
            .await
        else {
            return;
        };
        let bytes = handle.read().await;
        let text = match String::from_utf8(bytes) {
            Ok(t) => t,
            Err(e) => {
                import_error.set(Some(format!("File is not valid UTF-8: {e}")));
                return;
            }
        };
        apply_import_text(text, store, import_error).await;
    });
}

// ---------------------------------------------------------------------------
// Import button element (platform-specific)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
fn import_button(
    store: Signal<Option<ProfileStore<AppStorage>>>,
    import_error: Signal<Option<String>>,
) -> Element {
    rsx! {
        label { class: "block cursor-pointer",
            input {
                r#type: "file",
                accept: ".json",
                class: "sr-only",
                onchange: move |evt| do_import(evt, store, import_error),
            }
            span { class: "flex w-full items-center justify-center rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 font-medium py-2 text-sm hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors select-none",
                "Import existing data"
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn import_button(
    store: Signal<Option<ProfileStore<AppStorage>>>,
    import_error: Signal<Option<String>>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "flex w-full items-center justify-center rounded-lg border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300 font-medium py-2 text-sm hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors",
            onclick: move |_| do_import(store, import_error),
            "Import existing data"
        }
    }
}

// ---------------------------------------------------------------------------
// Dismiss (skip onboarding)
// ---------------------------------------------------------------------------

fn do_dismiss(mut store: Signal<Option<ProfileStore<AppStorage>>>) {
    if let Some(s) = store.write().as_mut() {
        let _ = s.create_profile("Main".to_string());
    }
    schedule_save();
}

// ---------------------------------------------------------------------------
// Onboarding page
// ---------------------------------------------------------------------------

#[component]
pub fn OnboardingPage() -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let settings = use_context::<Signal<AppSettings>>();
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_variables))]
    let queries = use_context::<Signal<SavedQueries>>();
    let mut name = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let import_error = use_signal(|| None::<String>);

    // Drive sync state — only meaningful on web builds.
    #[cfg(target_arch = "wasm32")]
    let drive_state = use_context::<Signal<crate::drive::DriveState>>();

    // Build the Drive section once (web only) so the RSX below stays readable.
    #[cfg(target_arch = "wasm32")]
    let drive_section = {
        use crate::drive::DriveState;
        let ds = drive_state.read();
        match &*ds {
            DriveState::Disconnected | DriveState::Error(_) => {
                let is_error = matches!(*ds, DriveState::Error(_));
                let err_msg = if let DriveState::Error(ref m) = *ds {
                    Some(m.clone())
                } else {
                    None
                };
                drop(ds);
                rsx! {
                    div { class: "space-y-1.5",
                        if let Some(msg) = err_msg {
                            p { class: "text-xs text-center text-red-600 dark:text-red-400",
                                "{msg}"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "flex w-full items-center justify-center rounded-lg border \
                                    border-gray-200 dark:border-gray-700 text-gray-700 \
                                    dark:text-gray-300 font-medium py-2 text-sm \
                                    hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors",
                            onclick: move |_| {
                                let mut ds = drive_state;
                                ds.set(DriveState::Connecting);
                                spawn(async move {
                                    crate::drive::onboarding_connect_drive(drive_state, store, settings, queries)
                                        .await;
                                });
                            },
                            if is_error {
                                "Retry Google Drive"
                            } else {
                                "Sync with Google Drive"
                            }
                        }
                    }
                }
            }
            DriveState::Connecting => {
                drop(ds);
                rsx! {
                    p { class: "text-sm text-center text-gray-500 dark:text-gray-400",
                        "Connecting to Google Drive…"
                    }
                }
            }
            DriveState::Connected { .. } => {
                drop(ds);
                rsx! {
                    p { class: "text-sm text-center text-green-600 dark:text-green-400 font-medium",
                        "Connected to Google Drive"
                    }
                }
            }
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    let drive_section = rsx! {};

    // On web, hide the file-import and skip controls while Drive is connecting so the
    // user doesn't accidentally start two flows simultaneously.
    #[cfg(target_arch = "wasm32")]
    let drive_connecting = matches!(*drive_state.read(), crate::drive::DriveState::Connecting);
    #[cfg(not(target_arch = "wasm32"))]
    let drive_connecting = false;

    rsx! {
        div { class: "min-h-screen bg-gray-50 dark:bg-gray-900 flex items-center justify-center p-4",
            div { class: "w-full max-w-sm bg-white dark:bg-gray-800 rounded-2xl shadow-lg p-8 space-y-6",

                // Header
                div { class: "text-center space-y-2",
                    h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100",
                        "Welcome to PTCGP DB"
                    }
                    p { class: "text-sm text-gray-500 dark:text-gray-400",
                        "Create your first profile to start tracking your collection."
                    }
                }

                // Profile name input + submit
                div { class: "space-y-3",
                    div { class: "space-y-1.5",
                        label {
                            r#for: "profile-name",
                            class: "block text-sm font-medium text-gray-700 dark:text-gray-300",
                            "Profile name"
                        }
                        input {
                            id: "profile-name",
                            r#type: "text",
                            placeholder: "My profile",
                            autofocus: true,
                            class: "w-full rounded-lg border border-gray-300 dark:border-gray-600 \
                                    bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 \
                                    placeholder-gray-400 dark:placeholder-gray-500 \
                                    px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                            value: name.read().clone(),
                            oninput: move |e| {
                                name.set(e.value());
                                error.set(None);
                            },
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    do_submit(name, error, store);
                                }
                            },
                        }
                        if let Some(err) = error.read().as_deref() {
                            p { class: "text-xs text-red-600 dark:text-red-400", "{err}" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "w-full rounded-lg bg-blue-600 hover:bg-blue-700 active:bg-blue-800 \
                                text-white font-medium py-2 text-sm transition-colors",
                        onclick: move |_| do_submit(name, error, store),
                        "Get Started"
                    }
                }

                // Divider
                div { class: "flex items-center gap-3",
                    div { class: "flex-1 border-t border-gray-200 dark:border-gray-700" }
                    span { class: "text-xs text-gray-400 dark:text-gray-500", "or" }
                    div { class: "flex-1 border-t border-gray-200 dark:border-gray-700" }
                }

                // Alternative onboarding paths
                div { class: "space-y-2",
                    // Google Drive sync (web only)
                    {drive_section}

                    // Import from file
                    if !drive_connecting {
                        div { class: "space-y-1.5",
                            {import_button(store, import_error)}
                            if let Some(err) = import_error.read().as_deref() {
                                p { class: "text-xs text-center text-red-600 dark:text-red-400",
                                    "{err}"
                                }
                            }
                        }
                    }
                }

                // Skip
                if !drive_connecting {
                    div { class: "text-center pt-2",
                        button {
                            r#type: "button",
                            class: "text-xs text-gray-400 dark:text-gray-500 \
                                    hover:text-gray-600 dark:hover:text-gray-300 transition-colors",
                            onclick: move |_| do_dismiss(store),
                            "Skip — set up later"
                        }
                    }
                }
            }
        }
    }
}

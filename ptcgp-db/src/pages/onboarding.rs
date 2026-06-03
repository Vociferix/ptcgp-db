//! First-run onboarding flow.
//!
//! On web builds the user first chooses between Drive sync and local-only, then (if they
//! have no existing profiles) proceeds to the profile-creation step. On desktop the sync
//! choice is skipped and the user goes straight to profile creation.

use dioxus::prelude::*;
use ptcgp_db_core::{
    AppSettings, ProfileStore, ProfilesSaveData, SavedQueries, migrate_profiles,
    storage::Storage as _,
};

use crate::app::{AppStorage, schedule_save};

// ---------------------------------------------------------------------------
// Step discriminant
// ---------------------------------------------------------------------------

/// Which screen of the two-step onboarding flow is currently shown.
#[derive(Clone, PartialEq, Default)]
enum Step {
    /// "Sync with Google Drive" vs "Work locally only" (web only; skipped on desktop).
    #[default]
    ChooseSync,
    /// Create a profile or import existing data.
    SetupProfile,
}

// ---------------------------------------------------------------------------
// Step 1 — Drive connect handler (web only)
// ---------------------------------------------------------------------------

/// Launches the interactive Drive connect flow and advances to `SetupProfile` on success.
///
/// Extracted as a named function so that the RSX `onclick` closure remains a single line,
/// which prevents `dx fmt` from corrupting the multi-line `spawn(async move { … })` block.
#[cfg(target_arch = "wasm32")]
fn start_drive_connect(
    mut drive_state: Signal<crate::drive::DriveState>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
    settings: Signal<AppSettings>,
    queries: Signal<SavedQueries>,
    mut step: Signal<Step>,
) {
    use crate::drive::DriveState;
    drive_state.set(DriveState::Connecting);
    spawn(async move {
        crate::drive::onboarding_connect_drive(drive_state, store, settings, queries).await;
        // If Drive connected but no profiles were found, advance to profile setup.
        // When profiles were loaded, App re-renders to Router before this line is reached.
        if drive_state.read().is_connected() {
            step.set(Step::SetupProfile);
        }
    });
}

// ---------------------------------------------------------------------------
// Step 2 — Profile creation / import helpers
// ---------------------------------------------------------------------------

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
    // Store now has profiles → App re-renders → OnboardingPage is replaced by Router.
}

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

    // On web start at ChooseSync; on desktop skip straight to SetupProfile.
    #[cfg(target_arch = "wasm32")]
    let mut step = use_signal(|| Step::ChooseSync);
    #[cfg(not(target_arch = "wasm32"))]
    let step = use_signal(|| Step::SetupProfile);

    // Declared unconditionally so hook order is stable across renders.
    let mut name = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let import_error = use_signal(|| None::<String>);

    #[cfg(target_arch = "wasm32")]
    let drive_state = use_context::<Signal<crate::drive::DriveState>>();

    let body = if *step.read() == Step::SetupProfile {
        // ── Step 2: profile creation / import ─────────────────────────────

        #[cfg(target_arch = "wasm32")]
        let drive_badge = if drive_state.read().is_connected() {
            rsx! {
                p { class: "text-xs text-center font-medium text-green-600 dark:text-green-400",
                    "Connected to Google Drive — your profile will sync automatically."
                }
            }
        } else {
            rsx! {}
        };
        #[cfg(not(target_arch = "wasm32"))]
        let drive_badge = rsx! {};

        rsx! {
            div { class: "text-center space-y-2",
                h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100",
                    "Set up your profile"
                }
                p { class: "text-sm text-gray-500 dark:text-gray-400",
                    "Create a profile to start tracking your collection."
                }
            }

            {drive_badge}

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

            div { class: "flex items-center gap-3",
                div { class: "flex-1 border-t border-gray-200 dark:border-gray-700" }
                span { class: "text-xs text-gray-400 dark:text-gray-500", "or" }
                div { class: "flex-1 border-t border-gray-200 dark:border-gray-700" }
            }

            div { class: "space-y-1.5",
                {import_button(store, import_error)}
                if let Some(err) = import_error.read().as_deref() {
                    p { class: "text-xs text-center text-red-600 dark:text-red-400",
                        "{err}"
                    }
                }
            }

            div { class: "text-center",
                button {
                    r#type: "button",
                    class: "text-xs text-gray-400 dark:text-gray-500 \
                            hover:text-gray-600 dark:hover:text-gray-300 transition-colors",
                    onclick: move |_| do_dismiss(store),
                    "Skip — set up later"
                }
            }
        }
    } else {
        // ── Step 1: choose sync mode (web only) ───────────────────────────

        #[cfg(target_arch = "wasm32")]
        {
            use crate::drive::DriveState;
            let is_connecting = matches!(*drive_state.read(), DriveState::Connecting);
            let err_msg = if let DriveState::Error(ref m) = *drive_state.read() {
                Some(m.clone())
            } else {
                None
            };

            rsx! {
                div { class: "text-center space-y-2",
                    h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100",
                        "Welcome to PTCGP DB"
                    }
                    p { class: "text-sm text-gray-500 dark:text-gray-400",
                        "How would you like to store your collection?"
                    }
                }

                if is_connecting {
                    p { class: "text-sm text-center text-gray-500 dark:text-gray-400",
                        "Connecting to Google Drive…"
                    }
                } else {
                    div { class: "space-y-3",
                        if let Some(msg) = err_msg {
                            p { class: "text-xs text-center text-red-600 dark:text-red-400",
                                "{msg}"
                            }
                        }

                        div {
                            button {
                                r#type: "button",
                                class: "w-full rounded-lg bg-blue-600 hover:bg-blue-700 \
                                        active:bg-blue-800 text-white font-medium py-2.5 \
                                        text-sm transition-colors",
                                onclick: move |_| start_drive_connect(drive_state, store, settings, queries, step),
                                "Sync with Google Drive"
                            }
                            p { class: "mt-1.5 text-xs text-center text-gray-500 dark:text-gray-400",
                                "Keep your collection in sync across all your devices."
                            }
                        }

                        div {
                            button {
                                r#type: "button",
                                class: "w-full rounded-lg border border-gray-300 \
                                        dark:border-gray-600 text-gray-700 dark:text-gray-300 \
                                        font-medium py-2.5 text-sm \
                                        hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors",
                                onclick: move |_| step.set(Step::SetupProfile),
                                "Work locally only"
                            }
                            p { class: "mt-1.5 text-xs text-center text-gray-500 dark:text-gray-400",
                                "Your data stays in this browser."
                            }
                        }
                    }
                }
            }
        }

        // Desktop never reaches Step::ChooseSync, but the compiler requires a value here.
        #[cfg(not(target_arch = "wasm32"))]
        rsx! {}
    };

    rsx! {
        div { class: "min-h-screen bg-gray-50 dark:bg-gray-900 flex flex-col items-center justify-center gap-4 p-4",
            div { class: "w-full max-w-sm bg-white dark:bg-gray-800 rounded-2xl shadow-lg p-8 space-y-6",
                {body}
            }
            p { class: "w-full max-w-sm text-xs text-center text-gray-400 dark:text-gray-500 leading-relaxed",
                "The literal and graphical information presented in this application \
                about Pokémon Trading Card Game Pocket, including card data, text and \
                images, is copyright The Pokémon Company, DeNA Co., Ltd., and/or \
                Creatures, Inc. This application is not produced by, endorsed by, \
                supported by, or affiliated with any of those copyright holders."
            }
        }
    }
}

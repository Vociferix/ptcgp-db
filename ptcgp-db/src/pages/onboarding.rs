use dioxus::prelude::*;
use ptcgp_db_core::{ProfileStore, ProfilesSaveData, migrate_profiles, storage::Storage as _};

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

fn do_import(
    evt: Event<FormData>,
    mut store: Signal<Option<ProfileStore<AppStorage>>>,
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
    });
}

fn do_dismiss(mut store: Signal<Option<ProfileStore<AppStorage>>>) {
    if let Some(s) = store.write().as_mut() {
        let _ = s.create_profile("Main".to_string());
    }
    schedule_save();
}

#[component]
pub fn OnboardingPage() -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let mut name = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let import_error = use_signal(|| None::<String>);

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

                // Profile name input
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

                // Primary action
                button {
                    r#type: "button",
                    class: "w-full rounded-lg bg-blue-600 hover:bg-blue-700 active:bg-blue-800 \
                            text-white font-medium py-2 text-sm transition-colors",
                    onclick: move |_| do_submit(name, error, store),
                    "Get Started"
                }

                // Divider
                div { class: "flex items-center gap-3",
                    div { class: "flex-1 border-t border-gray-200 dark:border-gray-700" }
                    span { class: "text-xs text-gray-400 dark:text-gray-500", "or" }
                    div { class: "flex-1 border-t border-gray-200 dark:border-gray-700" }
                }

                // Import from file
                div { class: "space-y-1.5",
                    label { class: "block cursor-pointer",
                        input {
                            r#type: "file",
                            accept: ".json",
                            class: "sr-only",
                            onchange: move |evt| do_import(evt, store, import_error),
                        }
                        span { class: "flex w-full items-center justify-center rounded-lg \
                                       border border-gray-200 dark:border-gray-700 \
                                       text-gray-700 dark:text-gray-300 font-medium py-2 \
                                       text-sm hover:bg-gray-50 dark:hover:bg-gray-700/50 \
                                       transition-colors select-none",
                            "Import existing data"
                        }
                    }
                    if let Some(err) = import_error.read().as_deref() {
                        p { class: "text-xs text-center text-red-600 dark:text-red-400",
                            "{err}"
                        }
                    }
                }

                // Skip
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

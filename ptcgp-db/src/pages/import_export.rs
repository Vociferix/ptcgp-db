use dioxus::prelude::*;
use ptcgp_db_core::{
    ProfileData, ProfileStore, ProfilesSaveData, migrate_profiles, storage::Storage as _,
};

use crate::app::AppStorage;

// ---------------------------------------------------------------------------
// Import state machine
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum ImportStep {
    Idle,
    Error(String),
    Resolving {
        new_profiles: Vec<ProfileData>,
        /// (imported profile data, overwrite = true / skip = false)
        conflicts: Vec<(ProfileData, bool)>,
    },
    Success(usize),
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

fn do_export(store: Signal<Option<ProfileStore<AppStorage>>>) {
    let snapshot = store
        .read()
        .as_ref()
        .map(|s| s.save_data_snapshot().clone());
    let Some(data) = snapshot else { return };
    spawn(async move {
        let json = match serde_json::to_string_pretty(&data) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("export serialize: {e}");
                return;
            }
        };
        // Pass the JSON to JS and trigger a Blob download; no string interpolation of JSON
        // so special characters can't break the JS literal.
        let eval = document::eval(
            "const j=await dioxus.recv();\
             const b=new Blob([j],{type:'application/json'});\
             const u=URL.createObjectURL(b);\
             const a=document.createElement('a');\
             a.href=u;a.download='ptcgp-backup.json';\
             document.body.appendChild(a);a.click();\
             document.body.removeChild(a);URL.revokeObjectURL(u);",
        );
        if let Err(e) = eval.send(json) {
            tracing::error!("export eval send: {e}");
        }
    });
}

// ---------------------------------------------------------------------------
// Import — parsing
// ---------------------------------------------------------------------------

fn parse_import(text: &str) -> Result<ProfilesSaveData, String> {
    let raw: ProfilesSaveData =
        serde_json::from_str(text).map_err(|e| format!("Invalid JSON: {e}"))?;
    migrate_profiles(raw).map_err(|e| format!("Incompatible format version: {e}"))
}

// ---------------------------------------------------------------------------
// Import — applying mutations and saving
// ---------------------------------------------------------------------------

/// Applies the resolved import mutations to the store and persists immediately.
/// Called from async spawn tasks (both no-conflict and post-resolution paths).
async fn do_apply_import(
    new_profiles: Vec<ProfileData>,
    overwrite_profiles: Vec<ProfileData>,
    mut store: Signal<Option<ProfileStore<AppStorage>>>,
    mut import_step: Signal<ImportStep>,
) {
    let mut count = 0usize;

    let (snapshot, storage) = {
        let mut guard = store.write();
        let Some(s) = guard.as_mut() else {
            import_step.set(ImportStep::Error("Store not available.".into()));
            return;
        };
        for profile in new_profiles {
            let name = profile.name.clone();
            if s.create_profile(name.clone()).is_ok() {
                let _ = s.replace_profile_counts(&name, profile.owned_counts);
                count += 1;
            }
        }
        for profile in overwrite_profiles {
            if s.replace_profile_counts(&profile.name, profile.owned_counts)
                .is_ok()
            {
                count += 1;
            }
        }
        let snapshot = s.save_data_snapshot().clone();
        let storage = s.storage().clone();
        s.mark_clean();
        (snapshot, storage)
    };

    if let Err(e) = storage.save_profiles(&snapshot).await {
        tracing::error!("import save: {e}");
    }

    import_step.set(ImportStep::Success(count));
}

// ---------------------------------------------------------------------------
// Import — file event handler
// ---------------------------------------------------------------------------

fn handle_file_change(
    evt: Event<FormData>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
    mut import_step: Signal<ImportStep>,
) {
    let Some(file) = evt.files().into_iter().next() else {
        return;
    };
    spawn(async move {
        let text = match file.read_string().await {
            Ok(t) => t,
            Err(e) => {
                import_step.set(ImportStep::Error(format!("Could not read file: {e}")));
                return;
            }
        };
        let data = match parse_import(&text) {
            Ok(d) => d,
            Err(e) => {
                import_step.set(ImportStep::Error(e));
                return;
            }
        };

        let existing_names: Vec<String> = store
            .read()
            .as_ref()
            .map(|s| s.profiles().iter().map(|p| p.name.clone()).collect())
            .unwrap_or_default();

        let mut new_profiles = Vec::new();
        let mut conflicts = Vec::new();
        for profile in data.profiles {
            if existing_names.iter().any(|n| n == &profile.name) {
                conflicts.push((profile, true)); // default: overwrite
            } else {
                new_profiles.push(profile);
            }
        }

        if conflicts.is_empty() {
            do_apply_import(new_profiles, Vec::new(), store, import_step).await;
        } else {
            import_step.set(ImportStep::Resolving {
                new_profiles,
                conflicts,
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Import — conflict resolution helpers
// ---------------------------------------------------------------------------

fn set_conflict_choice(mut import_step: Signal<ImportStep>, idx: usize, overwrite: bool) {
    if let ImportStep::Resolving { conflicts, .. } = &mut *import_step.write() {
        if let Some(c) = conflicts.get_mut(idx) {
            c.1 = overwrite;
        }
    }
}

fn confirm_import(
    import_step: Signal<ImportStep>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
) {
    let step = import_step.read().clone();
    if let ImportStep::Resolving {
        new_profiles,
        conflicts,
    } = step
    {
        let overwrite = conflicts
            .into_iter()
            .filter(|(_, o)| *o)
            .map(|(p, _)| p)
            .collect::<Vec<_>>();
        spawn(async move {
            do_apply_import(new_profiles, overwrite, store, import_step).await;
        });
    }
}

// ---------------------------------------------------------------------------
// ConflictRow sub-component
// ---------------------------------------------------------------------------

#[component]
fn ConflictRow(
    profile_name: String,
    card_count: usize,
    overwrite: bool,
    index: usize,
    import_step: Signal<ImportStep>,
) -> Element {
    rsx! {
        div { class: "py-3 flex items-center justify-between gap-4",
            div { class: "min-w-0",
                p { class: "text-sm font-medium text-gray-900 dark:text-gray-100 truncate",
                    "{profile_name}"
                }
                p { class: "text-xs text-gray-500 dark:text-gray-400",
                    "{card_count} card(s) in imported file"
                }
            }
            div { class: "flex shrink-0 rounded-md overflow-hidden border border-gray-200 dark:border-gray-600",
                button {
                    r#type: "button",
                    class: if overwrite { "px-3 py-1.5 text-xs font-medium bg-blue-600 text-white" } else { "px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600" },
                    onclick: move |_| set_conflict_choice(import_step, index, true),
                    "Overwrite"
                }
                button {
                    r#type: "button",
                    class: if !overwrite { "px-3 py-1.5 text-xs font-medium bg-blue-600 text-white" } else { "px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 hover:bg-gray-100 dark:hover:bg-gray-600" },
                    onclick: move |_| set_conflict_choice(import_step, index, false),
                    "Skip"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Import / Export page
// ---------------------------------------------------------------------------

#[component]
pub fn ImportExportPage() -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let mut import_step = use_signal(|| ImportStep::Idle);

    let step = import_step();

    rsx! {
        div { class: "max-w-2xl mx-auto p-6 space-y-6",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Import & Export" }

            // ── Export ───────────────────────────────────────────────────────
            section { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6 space-y-3",
                h2 { class: "text-lg font-semibold text-gray-900 dark:text-gray-100",
                    "Export"
                }
                p { class: "text-sm text-gray-500 dark:text-gray-400",
                    "Download all profiles and their card counts as a JSON file."
                }
                button {
                    r#type: "button",
                    class: "rounded-lg bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white font-medium px-4 py-2 text-sm transition-colors",
                    onclick: move |_| do_export(store),
                    "Export to JSON"
                }
            }

            // ── Import ───────────────────────────────────────────────────────
            section { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6 space-y-4",
                h2 { class: "text-lg font-semibold text-gray-900 dark:text-gray-100",
                    "Import"
                }

                match step {
                    ImportStep::Idle | ImportStep::Error(_) => rsx! {
                        if let ImportStep::Error(msg) = import_step() {
                            p { class: "text-sm text-red-600 dark:text-red-400", "{msg}" }
                        }
                        p { class: "text-sm text-gray-500 dark:text-gray-400",
                            "Load profiles from a previously exported JSON file. Existing profiles not present in the file are unaffected."
                        }
                        label { class: "inline-flex cursor-pointer",
                            input {
                                r#type: "file",
                                accept: ".json",
                                class: "sr-only",
                                onchange: move |evt| handle_file_change(evt, store, import_step),
                            }
                            span { class: "rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 hover:bg-gray-50 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 font-medium px-4 py-2 text-sm transition-colors select-none",
                                "Choose JSON file…"
                            }
                        }
                    },

                    ImportStep::Resolving { new_profiles, conflicts } => rsx! {
                        div { class: "space-y-4",
                            p { class: "text-sm text-gray-700 dark:text-gray-300",
                                "Some imported profiles share names with existing ones. Choose how to handle each:"
                            }
                            div { class: "divide-y divide-gray-100 dark:divide-gray-700",
                                for (i, (profile, overwrite)) in conflicts.iter().enumerate() {
                                    ConflictRow {
                                        key: "{profile.name}",
                                        profile_name: "{profile.name}",
                                        card_count: profile.owned_counts.len(),
                                        overwrite: *overwrite,
                                        index: i,
                                        import_step,
                                    }
                                }
                            }
                            if !new_profiles.is_empty() {
                                p { class: "text-xs text-gray-500 dark:text-gray-400",
                                    "{new_profiles.len()} new profile(s) will also be added."
                                }
                            }
                            div { class: "flex gap-3 pt-2",
                                button {
                                    r#type: "button",
                                    class: "rounded-lg bg-blue-600 hover:bg-blue-700 active:bg-blue-800 text-white font-medium px-4 py-2 text-sm transition-colors",
                                    onclick: move |_| confirm_import(import_step, store),
                                    "Confirm Import"
                                }
                                button {
                                    r#type: "button",
                                    class: "rounded-lg border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 font-medium px-4 py-2 text-sm transition-colors hover:bg-gray-50 dark:hover:bg-gray-700",
                                    onclick: move |_| import_step.set(ImportStep::Idle),
                                    "Cancel"
                                }
                            }
                        }
                    },

                    ImportStep::Success(count) => rsx! {
                        div { class: "space-y-3",
                            p { class: "text-sm text-green-700 dark:text-green-400",
                                "Successfully imported {count} profile(s)."
                            }
                            button {
                                r#type: "button",
                                class: "text-sm text-blue-600 dark:text-blue-400 hover:underline",
                                onclick: move |_| import_step.set(ImportStep::Idle),
                                "Import another file"
                            }
                        }
                    },
                }
            }
        }
    }
}

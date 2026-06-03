use dioxus::prelude::*;
use ptcgp_db_core::{
    ProfileData, ProfileStore, ProfilesSaveData, migrate_profiles, storage::Storage as _,
};

use crate::app::{AppStorage, schedule_save};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn do_create(
    mut new_name: Signal<String>,
    mut create_error: Signal<Option<String>>,
    mut store: Signal<Option<ProfileStore<AppStorage>>>,
) {
    let trimmed = new_name.read().trim().to_string();
    if trimmed.is_empty() {
        create_error.set(Some("Profile name is required.".into()));
        return;
    }
    match store.write().as_mut().map(|s| s.create_profile(trimmed)) {
        Some(Ok(())) => new_name.set(String::new()),
        Some(Err(e)) => {
            create_error.set(Some(e.to_string()));
            return;
        }
        None => return,
    }
    schedule_save();
}

fn do_rename(
    old_name: String,
    mut editing: Signal<Option<String>>,
    edit_value: Signal<String>,
    mut rename_error: Signal<Option<String>>,
    mut store: Signal<Option<ProfileStore<AppStorage>>>,
) {
    let new_name = edit_value.read().trim().to_string();
    if new_name.is_empty() {
        rename_error.set(Some("Profile name is required.".into()));
        return;
    }
    if new_name == old_name {
        editing.set(None);
        return;
    }
    match store
        .write()
        .as_mut()
        .map(|s| s.rename_profile(&old_name, new_name))
    {
        Some(Ok(())) => editing.set(None),
        Some(Err(e)) => {
            rename_error.set(Some(e.to_string()));
            return;
        }
        None => return,
    }
    schedule_save();
}

// ---------------------------------------------------------------------------
// Profile row sub-components (extracted to avoid dx fmt runaway indentation
// on multiline string continuations inside nested rsx! blocks)
// ---------------------------------------------------------------------------

#[component]
fn EditRow(
    prof_name: String,
    edit_value: Signal<String>,
    rename_error: Signal<Option<String>>,
    editing: Signal<Option<String>>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
) -> Element {
    rsx! {
        div { class: "flex items-start gap-2 p-3",
            div { class: "flex-1 space-y-1",
                input {
                    r#type: "text",
                    autofocus: true,
                    class: "w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                    value: edit_value.read().clone(),
                    onmounted: move |_| {
                        let _ = document::eval(
                            "var el=document.querySelector('input[autofocus]');if(el)el.select();",
                        );
                    },
                    oninput: move |e| {
                        edit_value.set(e.value());
                        rename_error.set(None);
                    },
                    onkeydown: {
                        let prof_name = prof_name.clone();
                        move |e| match e.key() {
                            Key::Enter => {
                                do_rename(prof_name.clone(), editing, edit_value, rename_error, store)
                            }
                            Key::Escape => {
                                editing.set(None);
                                rename_error.set(None);
                            }
                            _ => {}
                        }
                    },
                }
                if let Some(err) = rename_error.read().as_deref() {
                    p { class: "text-xs text-red-600 dark:text-red-400", "{err}" }
                }
            }
            button {
                r#type: "button",
                class: "shrink-0 px-3 py-1.5 text-sm font-medium rounded-md bg-blue-600 hover:bg-blue-700 text-white",
                onclick: {
                    let prof_name = prof_name.clone();
                    move |_| do_rename(prof_name.clone(), editing, edit_value, rename_error, store)
                },
                "Save"
            }
            button {
                r#type: "button",
                class: "shrink-0 px-3 py-1.5 text-sm font-medium rounded-md border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700",
                onclick: move |_| {
                    editing.set(None);
                    rename_error.set(None);
                },
                "Cancel"
            }
        }
    }
}

#[component]
fn DisplayRow(
    prof_name: String,
    is_primary_row: bool,
    is_only: bool,
    editing: Signal<Option<String>>,
    edit_value: Signal<String>,
    rename_error: Signal<Option<String>>,
    delete_target: Signal<Option<String>>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-2 p-3",
            div { class: "flex items-center gap-2 flex-1 min-w-0",
                span { class: "text-sm font-medium text-gray-900 dark:text-gray-100 truncate",
                    "{prof_name}"
                }
                if is_primary_row {
                    span { class: "shrink-0 text-xs px-1.5 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 font-medium",
                        "Primary"
                    }
                }
            }
            if !is_primary_row {
                button {
                    r#type: "button",
                    class: "shrink-0 text-xs px-2 py-1 rounded-md border border-gray-200 dark:border-gray-600 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700",
                    onclick: {
                        let prof_name = prof_name.clone();
                        move |_| {
                            if let Some(s) = store.write().as_mut()
                                && let Err(e) = s.set_primary(&prof_name)
                            {
                                tracing::error!("set_primary failed: {e}");
                            }
                            schedule_save();
                        }
                    },
                    "Set primary"
                }
            }
            button {
                r#type: "button",
                class: "shrink-0 text-xs px-2 py-1 rounded-md border border-gray-200 dark:border-gray-600 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700",
                onclick: {
                    let prof_name = prof_name.clone();
                    move |_| {
                        edit_value.set(prof_name.clone());
                        rename_error.set(None);
                        editing.set(Some(prof_name.clone()));
                    }
                },
                "Rename"
            }
            button {
                r#type: "button",
                disabled: is_only,
                class: if is_only { "shrink-0 text-xs px-2 py-1 rounded-md border border-gray-200 dark:border-gray-700 text-gray-300 dark:text-gray-600 cursor-not-allowed" } else { "shrink-0 text-xs px-2 py-1 rounded-md border border-red-200 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20" },
                onclick: {
                    let prof_name = prof_name.clone();
                    move |_| {
                        if !is_only {
                            delete_target.set(Some(prof_name.clone()));
                        }
                    }
                },
                "Delete"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Delete confirmation dialog
// ---------------------------------------------------------------------------

#[component]
fn DeleteConfirmDialog(
    name: String,
    mut delete_target: Signal<Option<String>>,
    mut store: Signal<Option<ProfileStore<AppStorage>>>,
) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50",
            onclick: move |_| delete_target.set(None),
            div {
                class: "bg-white dark:bg-gray-800 rounded-xl shadow-2xl ring-1 ring-black/10 dark:ring-white/10 p-6 w-full max-w-sm mx-4 space-y-4",
                onclick: move |e| e.stop_propagation(),
                h2 { class: "text-base font-semibold text-gray-900 dark:text-gray-100",
                    "Delete profile?"
                }
                p { class: "text-sm text-gray-600 dark:text-gray-400",
                    "\"{name}\" will be permanently deleted. This cannot be undone."
                }
                div { class: "flex justify-end gap-2",
                    button {
                        r#type: "button",
                        class: "px-4 py-2 text-sm font-medium rounded-md border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700",
                        onclick: move |_| delete_target.set(None),
                        "Cancel"
                    }
                    button {
                        r#type: "button",
                        class: "px-4 py-2 text-sm font-medium rounded-md bg-red-600 hover:bg-red-700 text-white",
                        onclick: {
                            let name = name.clone();
                            move |_| {
                                if let Some(s) = store.write().as_mut()
                                    && let Err(e) = s.delete_profile(&name)
                                {
                                    tracing::error!("delete_profile failed: {e}");
                                }
                                delete_target.set(None);
                                schedule_save();
                            }
                        },
                        "Delete"
                    }
                }
            }
        }
    }
}

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
// Export — platform-specific
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
fn do_export(store: Signal<Option<ProfileStore<AppStorage>>>) {
    let snapshot = store
        .read()
        .as_ref()
        .map(|s| s.save_data_snapshot().clone());
    let Some(mut data) = snapshot else { return };
    data.active_profile_names.clear();
    spawn(async move {
        let json = match serde_json::to_string_pretty(&data) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("export serialize: {e}");
                return;
            }
        };
        // Pass JSON via dioxus.recv() so no special characters can break the JS literal.
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

#[cfg(not(target_arch = "wasm32"))]
fn do_export(store: Signal<Option<ProfileStore<AppStorage>>>) {
    let snapshot = store
        .read()
        .as_ref()
        .map(|s| s.save_data_snapshot().clone());
    let Some(mut data) = snapshot else { return };
    data.active_profile_names.clear();
    spawn(async move {
        let json = match serde_json::to_string_pretty(&data) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("export serialize: {e}");
                return;
            }
        };
        let Some(handle) = rfd::AsyncFileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name("ptcgp-backup.json")
            .save_file()
            .await
        else {
            return;
        };
        if let Err(e) = handle.write(json.as_bytes()).await {
            tracing::error!("export write: {e}");
        }
    });
}

// ---------------------------------------------------------------------------
// Import — shared parsing and application logic
// ---------------------------------------------------------------------------

fn parse_import(text: &str) -> Result<ProfilesSaveData, String> {
    let raw: ProfilesSaveData =
        serde_json::from_str(text).map_err(|e| format!("Invalid JSON: {e}"))?;
    migrate_profiles(raw).map_err(|e| format!("Incompatible format version: {e}"))
}

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

async fn process_import_text(
    text: String,
    store: Signal<Option<ProfileStore<AppStorage>>>,
    mut import_step: Signal<ImportStep>,
) {
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
}

// ---------------------------------------------------------------------------
// Import — file reading (platform-specific)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
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
        process_import_text(text, store, import_step).await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_file_pick(
    store: Signal<Option<ProfileStore<AppStorage>>>,
    mut import_step: Signal<ImportStep>,
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
                import_step.set(ImportStep::Error(format!("File is not valid UTF-8: {e}")));
                return;
            }
        };
        process_import_text(text, store, import_step).await;
    });
}

// ---------------------------------------------------------------------------
// Import trigger element (platform-specific)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
fn import_trigger(
    store: Signal<Option<ProfileStore<AppStorage>>>,
    import_step: Signal<ImportStep>,
) -> Element {
    rsx! {
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
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn import_trigger(
    store: Signal<Option<ProfileStore<AppStorage>>>,
    import_step: Signal<ImportStep>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 hover:bg-gray-50 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 font-medium px-4 py-2 text-sm transition-colors",
            onclick: move |_| handle_file_pick(store, import_step),
            "Choose JSON file…"
        }
    }
}

// ---------------------------------------------------------------------------
// Import — conflict resolution helpers
// ---------------------------------------------------------------------------

fn set_conflict_choice(mut import_step: Signal<ImportStep>, idx: usize, overwrite: bool) {
    if let ImportStep::Resolving { conflicts, .. } = &mut *import_step.write()
        && let Some(c) = conflicts.get_mut(idx)
    {
        c.1 = overwrite;
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
// Profile Manager page
// ---------------------------------------------------------------------------

#[component]
pub fn ProfileManagerPage() -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();

    let mut new_name = use_signal(String::new);
    let mut create_error = use_signal(|| None::<String>);
    let editing: Signal<Option<String>> = use_signal(|| None);
    let edit_value = use_signal(String::new);
    let rename_error: Signal<Option<String>> = use_signal(|| None);
    let delete_target: Signal<Option<String>> = use_signal(|| None);
    let mut import_step = use_signal(|| ImportStep::Idle);

    let (profiles, primary_name) = {
        let guard = store.read();
        guard
            .as_ref()
            .map(|s| (s.profiles().to_vec(), s.primary_profile_name().to_string()))
            .unwrap_or_default()
    };

    let is_only = profiles.len() <= 1;
    let delete_name = delete_target.read().clone();

    let step = import_step();

    rsx! {
        div { class: "max-w-2xl mx-auto p-6 space-y-6",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Profiles" }

            // ── Profile list ────────────────────────────────────────────────
            section {
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200/80 dark:border-gray-700/80 divide-y divide-gray-100 dark:divide-gray-700 shadow-sm",
                    for profile in &profiles {
                        {
                            let prof_name = profile.name.clone();
                            let is_primary_row = prof_name == primary_name;
                            let is_editing_row =
                                editing.read().as_deref() == Some(prof_name.as_str());

                            if is_editing_row {
                                rsx! {
                                    EditRow {
                                        key: "edit-{prof_name}",
                                        prof_name,
                                        edit_value,
                                        rename_error,
                                        editing,
                                        store,
                                    }
                                }
                            } else {
                                rsx! {
                                    DisplayRow {
                                        key: "display-{prof_name}",
                                        prof_name,
                                        is_primary_row,
                                        is_only,
                                        editing,
                                        edit_value,
                                        rename_error,
                                        delete_target,
                                        store,
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Create new profile ──────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3",
                    "New profile"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200/80 dark:border-gray-700/80 p-4 shadow-sm",
                    div { class: "flex items-start gap-2",
                        div { class: "flex-1 space-y-1",
                            input {
                                r#type: "text",
                                placeholder: "Profile name",
                                class: "w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
                                value: new_name.read().clone(),
                                oninput: move |e| {
                                    new_name.set(e.value());
                                    create_error.set(None);
                                },
                                onkeydown: move |e| {
                                    if e.key() == Key::Enter {
                                        do_create(new_name, create_error, store);
                                    }
                                },
                            }
                            if let Some(err) = create_error.read().as_deref() {
                                p { class: "text-xs text-red-600 dark:text-red-400",
                                    "{err}"
                                }
                            }
                        }
                        button {
                            r#type: "button",
                            class: "shrink-0 px-4 py-1.5 text-sm font-medium rounded-md bg-blue-600 hover:bg-blue-700 text-white",
                            onclick: move |_| do_create(new_name, create_error, store),
                            "Create"
                        }
                    }
                }
            }

            // ── Export ───────────────────────────────────────────────────────
            section { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200/80 dark:border-gray-700/80 p-6 space-y-3 shadow-sm",
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
            section { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200/80 dark:border-gray-700/80 p-6 space-y-4 shadow-sm",
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
                        {import_trigger(store, import_step)}
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

        // Delete confirmation dialog
        if let Some(name) = delete_name {
            DeleteConfirmDialog { name, delete_target, store }
        }
    }
}

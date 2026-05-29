use dioxus::prelude::*;
use ptcgp_db_core::ProfileStore;

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
    match store.write().as_mut().map(|s| s.rename_profile(&old_name, new_name)) {
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
                class: "bg-white dark:bg-gray-800 rounded-xl shadow-xl p-6 \
                        w-full max-w-sm mx-4 space-y-4",
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
                        class: "px-4 py-2 text-sm font-medium rounded-md border \
                                border-gray-300 dark:border-gray-600 \
                                text-gray-700 dark:text-gray-300 \
                                hover:bg-gray-100 dark:hover:bg-gray-700",
                        onclick: move |_| delete_target.set(None),
                        "Cancel"
                    }
                    button {
                        r#type: "button",
                        class: "px-4 py-2 text-sm font-medium rounded-md \
                                bg-red-600 hover:bg-red-700 text-white",
                        onclick: {
                            let name = name.clone();
                            move |_| {
                                if let Some(s) = store.write().as_mut() {
                                    if let Err(e) = s.delete_profile(&name) {
                                        tracing::error!("delete_profile failed: {e}");
                                    }
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
// Profile Manager page
// ---------------------------------------------------------------------------

#[component]
pub fn ProfileManagerPage() -> Element {
    let mut store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();

    let mut new_name = use_signal(String::new);
    let mut create_error = use_signal(|| None::<String>);
    let mut editing: Signal<Option<String>> = use_signal(|| None);
    let mut edit_value = use_signal(String::new);
    let mut rename_error: Signal<Option<String>> = use_signal(|| None);
    let mut delete_target: Signal<Option<String>> = use_signal(|| None);

    let (profiles, primary_name) = {
        let guard = store.read();
        guard
            .as_ref()
            .map(|s| (s.profiles().to_vec(), s.primary_profile_name().to_string()))
            .unwrap_or_default()
    };

    let is_only = profiles.len() <= 1;
    let delete_name = delete_target.read().clone();

    rsx! {
        div { class: "max-w-2xl mx-auto p-6 space-y-6",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Profiles" }

            // ── Profile list ────────────────────────────────────────────────
            section {
                div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                              border-gray-200 dark:border-gray-700 \
                              divide-y divide-gray-100 dark:divide-gray-700",

                    for profile in &profiles {
                        {
                            let prof_name = profile.name.clone();
                            let is_primary_row = prof_name == primary_name;
                            let is_editing_row =
                                editing.read().as_deref() == Some(prof_name.as_str());

                            if is_editing_row {
                                rsx! {
                                    div { key: "edit-{prof_name}", class: "flex items-start gap-2 p-3",
                                        div { class: "flex-1 space-y-1",
                                            input {
                                                r#type: "text",
                                                autofocus: true,
                                                class: "w-full rounded-md border border-gray-300 dark:border-gray-600 \
                                                                                                                                bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 \
                                                                                                                                px-3 py-1.5 text-sm \
                                                                                                                                focus:outline-none focus:ring-2 focus:ring-blue-500",
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
                                            class: "shrink-0 px-3 py-1.5 text-sm font-medium \
                                                                                                                            rounded-md bg-blue-600 hover:bg-blue-700 text-white",
                                            onclick: {
                                                let prof_name = prof_name.clone();
                                                move |_| {
                                                    do_rename(prof_name.clone(), editing, edit_value, rename_error, store)
                                                }
                                            },
                                            "Save"
                                        }
                                        button {
                                            r#type: "button",
                                            class: "shrink-0 px-3 py-1.5 text-sm font-medium \
                                                                                                                            rounded-md border border-gray-300 dark:border-gray-600 \
                                                                                                                            text-gray-700 dark:text-gray-300 \
                                                                                                                            hover:bg-gray-100 dark:hover:bg-gray-700",
                                            onclick: move |_| {
                                                editing.set(None);
                                                rename_error.set(None);
                                            },
                                            "Cancel"
                                        }
                                    }
                                }
                            } else {
                                rsx! {
                                    div { key: "display-{prof_name}", class: "flex items-center gap-2 p-3",
                                        // Name + primary badge
                                        div { class: "flex items-center gap-2 flex-1 min-w-0",
                                            span {
                                            class: "text-sm font-medium text-gray-900 dark:text-gray-100 truncate",
                                                "{prof_name}"
                                            }
                                            if is_primary_row {
                                                span {
                                                class: "shrink-0 text-xs px-1.5 py-0.5 rounded-full \
                                                                                                                                    bg-blue-100 dark:bg-blue-900 \
                                                                                                                                    text-blue-700 dark:text-blue-300 font-medium",
                                                    "Primary"
                                                }
                                            }
                                        }
                                        // Set primary (non-primary profiles only)
                                        if !is_primary_row {
                                            button {
                                                r#type: "button",
                                                class: "shrink-0 text-xs px-2 py-1 rounded-md border \
                                                                                                                                border-gray-200 dark:border-gray-600 \
                                                                                                                                text-gray-600 dark:text-gray-400 \
                                                                                                                                hover:bg-gray-100 dark:hover:bg-gray-700",
                                                onclick: {
                                                    let prof_name = prof_name.clone();
                                                    move |_| {
                                                        if let Some(s) = store.write().as_mut() {
                                                            if let Err(e) = s.set_primary(&prof_name) {
                                                                tracing::error!("set_primary failed: {e}");
                                                            }
                                                        }
                                                        schedule_save();
                                                    }
                                                },
                                                "Set primary"
                                            }
                                        }
                                        // Rename
                                        button {
                                            r#type: "button",
                                            class: "shrink-0 text-xs px-2 py-1 rounded-md border \
                                                                                                                            border-gray-200 dark:border-gray-600 \
                                                                                                                            text-gray-600 dark:text-gray-400 \
                                                                                                                            hover:bg-gray-100 dark:hover:bg-gray-700",
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
                                        // Delete
                                        button {
                                            r#type: "button",
                                            disabled: is_only,
                                            class: if is_only {
                                            "shrink-0 text-xs px-2 py-1 rounded-md border \
                                                                                                                         border-gray-200 dark:border-gray-700 \
                                                                                                                         text-gray-300 dark:text-gray-600 cursor-not-allowed"
                                                                                                                         } else {
                                                                                                                         "shrink-0 text-xs px-2 py-1 rounded-md border \
                                                                                                                         border-red-200 dark:border-red-800 \
                                                                                                                         text-red-600 dark:text-red-400 \
                                                                                                                         hover:bg-red-50 dark:hover:bg-red-900/20"
                                                                                                                         },
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
                        }
                    }
                }
            }

            // ── Create new profile ──────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider \
                              text-gray-500 dark:text-gray-400 mb-3",
                    "New profile"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                              border-gray-200 dark:border-gray-700 p-4",
                    div { class: "flex items-start gap-2",
                        div { class: "flex-1 space-y-1",
                            input {
                                r#type: "text",
                                placeholder: "Profile name",
                                class: "w-full rounded-md border border-gray-300 dark:border-gray-600 \
                                        bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 \
                                        placeholder-gray-400 dark:placeholder-gray-500 \
                                        px-3 py-1.5 text-sm \
                                        focus:outline-none focus:ring-2 focus:ring-blue-500",
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
                            class: "shrink-0 px-4 py-1.5 text-sm font-medium rounded-md \
                                    bg-blue-600 hover:bg-blue-700 text-white",
                            onclick: move |_| do_create(new_name, create_error, store),
                            "Create"
                        }
                    }
                }
            }
        }

        // Delete confirmation dialog (rendered outside the scrolling div so it overlays correctly)
        if let Some(name) = delete_name {
            DeleteConfirmDialog { name, delete_target, store }
        }
    }
}

use dioxus::prelude::*;

use crate::app::{AppStorage, schedule_save};

/// Profile selector embedded in the navigation bar.
///
/// Shows the active profile name(s) and opens a dropdown for switching profiles.
/// Single-profile selection is the primary path (clicking a row); checkboxes enable multi-select.
#[component]
pub fn ProfileSelector() -> Element {
    let mut store = use_context::<Signal<Option<ptcgp_db_core::ProfileStore<AppStorage>>>>();
    let mut open = use_signal(|| false);

    let (profile_names, active_names): (Vec<String>, Vec<String>) = {
        let guard = store.read();
        let Some(ref s) = *guard else {
            return rsx! {
                div { class: "text-sm text-gray-500", "Loading…" }
            };
        };
        (
            s.profiles().iter().map(|p| p.name.clone()).collect(),
            s.active_profile_names().to_vec(),
        )
    };

    let label = match active_names.len() {
        0 => "No profile".to_string(),
        1 => active_names[0].clone(),
        n => format!("{n} profiles"),
    };

    rsx! {
        div { class: "relative",
            // Trigger button
            button {
                r#type: "button",
                class: "flex items-center gap-1 px-2 py-1 rounded text-sm font-medium \
                        bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 \
                        text-gray-800 dark:text-gray-100 w-full",
                onclick: move |_| open.toggle(),
                span { class: "truncate flex-1 text-left", "{label}" }
                span { class: "ml-1 text-gray-500 dark:text-gray-400 shrink-0",
                    if *open.read() {
                        "▲"
                    } else {
                        "▼"
                    }
                }
            }

            // Dismiss overlay
            if *open.read() {
                div {
                    class: "fixed inset-0 z-10",
                    onclick: move |_| open.set(false),
                }
            }

            // Dropdown
            if *open.read() {
                div { class: "absolute left-0 z-20 mt-1 min-w-full w-48 rounded-md shadow-lg \
                            bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-600 \
                            py-1",
                    for name in profile_names {
                        {
                            let name = name.clone();
                            let is_active = active_names.contains(&name);
                            let only_active = active_names.len() == 1 && is_active;

                            rsx! {
                                div {
                                    key: "{name}",
                                    class: "flex items-center gap-2 px-3 py-2 text-sm cursor-pointer \
                                                                                                                                                                            hover:bg-gray-100 dark:hover:bg-gray-700 \
                                                                                                                                                                            text-gray-800 dark:text-gray-100",

                                    // Checkbox: toggles this profile in/out of the active set
                                    input {
                                        r#type: "checkbox",
                                        class: "shrink-0 accent-blue-500",
                                        checked: is_active,
                                        disabled: only_active,
                                        onchange: {
                                            let name = name.clone();
                                            move |_| {
                                                let mut guard = store.write();
                                                let Some(ref mut s) = *guard else { return };
                                                if is_active {
                                                    let _ = s.deactivate_profile(&name);
                                                } else {
                                                    let _ = s.activate_profile(&name);
                                                }
                                                drop(guard);
                                                schedule_save();
                                            }
                                        },
                                    }

                                    // Name: single-click selects only this profile
                                    span {
                                        class: "flex-1 truncate select-none",
                                        onclick: {
                                            let name = name.clone();
                                            move |e| {
                                                e.stop_propagation();
                                                let mut guard = store.write();
                                                let Some(ref mut s) = *guard else { return };
                                                // Activate only this profile by deactivating all others
                                                let all: Vec<String> = s
                                                    .active_profile_names()
                                                    .iter()
                                                    .filter(|n| n.as_str() != name)
                                                    .cloned()
                                                    .collect();
                                                for other in &all {
                                                    let _ = s.deactivate_profile(other);
                                                }
                                                let _ = s.activate_profile(&name);
                                                drop(guard);
                                                schedule_save();
                                                open.set(false);
                                            }
                                        },
                                        "{name}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

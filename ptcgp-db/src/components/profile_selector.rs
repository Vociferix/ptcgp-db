use dioxus::prelude::*;

use crate::app::{AppStorage, schedule_save};

/// Profile selector embedded in the navigation bar.
///
/// Regular click on a profile switches to only that profile and closes the menu.
/// Ctrl+Click toggles a profile in or out of the active set for multi-profile
/// aggregation.
///
/// Set `open_upward` when the selector sits near the bottom of the viewport so
/// the dropdown expands above the trigger instead of below it.
#[component]
pub fn ProfileSelector(#[props(default = false)] open_upward: bool) -> Element {
    let store = use_context::<Signal<Option<ptcgp_db_core::ProfileStore<AppStorage>>>>();
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

            // Dismiss backdrop
            if *open.read() {
                div {
                    class: "fixed inset-0 z-10",
                    onclick: move |_| open.set(false),
                }
            }

            // Dropdown
            if *open.read() {
                div { class: if open_upward { "absolute left-0 z-20 bottom-full mb-1 min-w-full w-48 rounded-md shadow-lg \
                         bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-600 py-1" } else { "absolute left-0 z-20 mt-1 min-w-full w-48 rounded-md shadow-lg \
                         bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-600 py-1" },
                    for name in profile_names {
                        ProfileRow {
                            key: "{name}",
                            name: name.clone(),
                            is_active: active_names.contains(&name),
                            open,
                            store,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ProfileRow(
    name: String,
    is_active: bool,
    mut open: Signal<bool>,
    mut store: Signal<Option<ptcgp_db_core::ProfileStore<AppStorage>>>,
) -> Element {
    rsx! {
        div {
            class: "flex items-center gap-2 px-3 py-2 text-sm cursor-pointer \
                    hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-800 dark:text-gray-100",
            onclick: {
                let name = name.clone();
                move |e: MouseEvent| {
                    let mut guard = store.write();
                    let Some(ref mut s) = *guard else { return };
                    if e.modifiers().ctrl() {
                        // Ctrl+Click: toggle this profile in the multi-select set.
                        // deactivate_profile is a no-op when it would leave the set empty.
                        if is_active {
                            let _ = s.deactivate_profile(&name);
                        } else {
                            let _ = s.activate_profile(&name);
                        }
                        drop(guard);
                        schedule_save();
                    } else {
                        // Regular click: switch to only this profile.
                        let others: Vec<String> = s
                            .active_profile_names()
                            .iter()
                            .filter(|n| n.as_str() != name)
                            .cloned()
                            .collect();
                        for other in &others {
                            let _ = s.deactivate_profile(other);
                        }
                        let _ = s.activate_profile(&name);
                        drop(guard);
                        schedule_save();
                        open.set(false);
                    }
                }
            },
            // Active indicator — checkmark for active profiles, blank space otherwise
            span { class: "shrink-0 w-4 text-blue-500 dark:text-blue-400 font-bold",
                if is_active {
                    "✓"
                } else {
                    ""
                }
            }
            span { class: "flex-1 truncate select-none", "{name}" }
        }
    }
}

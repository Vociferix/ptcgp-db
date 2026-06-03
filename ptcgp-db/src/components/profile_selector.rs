use crate::app::{AppStorage, schedule_save};
use crate::components::icons::{ChevronDown, ChevronUp};
use crate::components::toggle::ToggleCheckbox;
use dioxus::prelude::*;

/// Profile selector embedded in the navigation bar.
///
/// Tapping a profile row switches to only that profile and closes the menu.
/// Tapping the checkbox on the right of a row toggles it in or out of the
/// active set for multi-profile aggregation.
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
                        text-gray-800 dark:text-gray-100 w-full \
                        shadow-sm active:shadow-none active:translate-y-px",
                onclick: move |_| open.toggle(),
                span { class: "truncate flex-1 text-left", "{label}" }
                if *open.read() {
                    ChevronUp { class: "ml-1 w-4 h-4 text-gray-500 dark:text-gray-400 shrink-0" }
                } else {
                    ChevronDown { class: "ml-1 w-4 h-4 text-gray-500 dark:text-gray-400 shrink-0" }
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
                div { class: if open_upward { "absolute left-0 z-20 bottom-full mb-1 min-w-full w-48 rounded-md \
                         bg-white dark:bg-gray-700 border border-gray-200/60 dark:border-gray-600/60 \
                         shadow-xl dark:shadow-[0_4px_28px_rgba(0,0,0,0.7)] ring-1 ring-black/5 dark:ring-white/[0.09] py-1" } else { "absolute left-0 z-20 mt-1 min-w-full w-48 rounded-md \
                         bg-white dark:bg-gray-700 border border-gray-200/60 dark:border-gray-600/60 \
                         shadow-xl dark:shadow-[0_4px_28px_rgba(0,0,0,0.7)] ring-1 ring-black/5 dark:ring-white/[0.09] py-1" },
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

/// One profile row.
///
/// Tapping the row body activates only this profile and closes the dropdown.
/// Tapping the checkbox on the right toggles this profile in/out without closing.
/// `deactivate_profile` is a no-op when it would leave the active set empty, so
/// the last remaining active profile cannot be deselected via the checkbox.
#[component]
fn ProfileRow(
    name: String,
    is_active: bool,
    mut open: Signal<bool>,
    mut store: Signal<Option<ptcgp_db_core::ProfileStore<AppStorage>>>,
) -> Element {
    let row_cls = if is_active {
        "flex items-center gap-2 px-3 py-2 text-sm cursor-pointer select-none \
         bg-blue-50 dark:bg-blue-950/80 hover:bg-blue-100 dark:hover:bg-blue-900/60 \
         text-gray-800 dark:text-gray-100"
    } else {
        "flex items-center gap-2 px-3 py-2 text-sm cursor-pointer select-none \
         hover:bg-gray-100 dark:hover:bg-gray-600 text-gray-800 dark:text-gray-100"
    };

    // Activate only this profile and close the dropdown.
    // Activate first so there is always ≥1 active profile when deactivating
    // the others (deactivate_profile rejects an attempt to leave the set empty).
    let on_select = {
        let name = name.clone();
        move |_| {
            let mut guard = store.write();
            let Some(ref mut s) = *guard else { return };
            let _ = s.activate_profile(&name);
            let others: Vec<String> = s
                .active_profile_names()
                .iter()
                .filter(|n| n.as_str() != name)
                .cloned()
                .collect();
            for other in &others {
                let _ = s.deactivate_profile(other);
            }
            drop(guard);
            schedule_save();
            open.set(false);
        }
    };

    // Toggle this profile in/out of the active set without closing the dropdown.
    let on_toggle = {
        let name = name.clone();
        move |e: MouseEvent| {
            e.stop_propagation();
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
    };

    rsx! {
        div { class: "{row_cls}", onclick: on_select,
            span { class: "flex-1 truncate", "{name}" }
            button {
                r#type: "button",
                class: "shrink-0 p-2 -mr-1 rounded \
                        hover:bg-gray-200/60 dark:hover:bg-gray-500/40",
                onclick: on_toggle,
                ToggleCheckbox { checked: is_active }
            }
        }
    }
}

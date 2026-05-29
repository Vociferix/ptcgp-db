use dioxus::prelude::*;
use ptcgp_db_data::{CardSource, Pack, Set};

use ptcgp_db_core::save_data::FilterConfig;

// ---------------------------------------------------------------------------
// Set / Pack / Source dropdowns
// ---------------------------------------------------------------------------

#[component]
pub fn SetDropdown(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let ids: Vec<usize> = Set::ALL
        .iter()
        .filter(|set| {
            config
                .series
                .map_or(true, |sid| set.series().id() == sid)
        })
        .map(|set| set.id())
        .collect();
    let labels: Vec<String> = Set::ALL
        .iter()
        .filter(|set| {
            config
                .series
                .map_or(true, |sid| set.series().id() == sid)
        })
        .map(|set| format!("{} ({})", set.name().as_str(), set.code().as_str()))
        .collect();

    rsx! {
        CheckboxDropdown {
            picker_label: "Set",
            option_ids: ids,
            option_labels: labels,
            selected: config.sets.clone(),
            on_change: move |sets: Vec<usize>| {
                let mut c = config.clone();
                c.packs
                    .retain(|&pid| {
                        Pack::from_id(pid).is_some_and(|p| sets.contains(&p.set().id()))
                    });
                c.sets = sets;
                on_change.call(c);
            },
        }
    }
}

#[component]
pub fn PackDropdown(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let visible_packs: Vec<&Pack> = Pack::ALL
        .iter()
        .filter(|pack| {
            let series_ok = config
                .series
                .map_or(true, |sid| pack.series().id() == sid);
            let set_ok =
                config.sets.is_empty() || config.sets.contains(&pack.set().id());
            series_ok && set_ok
        })
        .collect();
    let ids: Vec<usize> = visible_packs.iter().map(|p| p.id()).collect();
    let labels: Vec<String> = visible_packs.iter().map(|p| p.title().to_string()).collect();

    rsx! {
        CheckboxDropdown {
            picker_label: "Pack",
            option_ids: ids,
            option_labels: labels,
            selected: config.packs.clone(),
            on_change: move |packs: Vec<usize>| {
                let mut c = config.clone();
                c.packs = packs;
                on_change.call(c);
            },
        }
    }
}

#[component]
pub fn SourceDropdown(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let ids: Vec<usize> = CardSource::ALL.iter().map(|src| src.id()).collect();
    let labels: Vec<String> = CardSource::ALL
        .iter()
        .map(|src| src.name().as_str().to_string())
        .collect();

    rsx! {
        CheckboxDropdown {
            picker_label: "Source",
            option_ids: ids,
            option_labels: labels,
            selected: config.sources.clone(),
            on_change: move |sources: Vec<usize>| {
                let mut c = config.clone();
                c.sources = sources;
                on_change.call(c);
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Generic multi-select dropdown with checkboxes
// ---------------------------------------------------------------------------

/// A labelled dropdown button that opens a checkbox list for multi-select filtering.
///
/// `option_ids` and `option_labels` are parallel vecs; indices must align.
#[component]
pub fn CheckboxDropdown(
    picker_label: &'static str,
    option_ids: Vec<usize>,
    option_labels: Vec<String>,
    selected: Vec<usize>,
    on_change: EventHandler<Vec<usize>>,
) -> Element {
    let mut open = use_signal(|| false);
    let count = selected.len();

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-md border \
                        border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 \
                        text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700",
                onclick: move |_| open.toggle(),
                "{picker_label}"
                if count > 0 {
                    span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                        "{count}"
                    }
                }
                span { class: "text-gray-400 dark:text-gray-500",
                    if *open.read() {
                        "▲"
                    } else {
                        "▼"
                    }
                }
            }

            if *open.read() {
                // Dismiss backdrop
                div {
                    class: "fixed inset-0 z-10",
                    onclick: move |_| open.set(false),
                }

                // Dropdown panel
                div { class: "absolute left-0 top-full mt-1 z-20 max-h-64 overflow-y-auto \
                            rounded-md border border-gray-200 dark:border-gray-700 \
                            bg-white dark:bg-gray-800 shadow-lg min-w-40",
                    if option_ids.is_empty() {
                        p { class: "px-3 py-2 text-sm text-gray-500 dark:text-gray-400",
                            "No options"
                        }
                    } else {
                        for i in 0..option_ids.len() {
                            CheckboxRow {
                                key: "{option_ids[i]}",
                                id: option_ids[i],
                                row_label: option_labels[i].clone(),
                                checked: selected.contains(&option_ids[i]),
                                all_selected: selected.clone(),
                                on_change: on_change.clone(),
                            }
                        }
                        if !selected.is_empty() {
                            div { class: "border-t border-gray-100 dark:border-gray-700 p-1",
                                button {
                                    r#type: "button",
                                    class: "w-full text-left px-2 py-1 text-xs rounded \
                                            text-gray-500 dark:text-gray-400 \
                                            hover:text-gray-700 dark:hover:text-gray-200",
                                    onclick: move |_| on_change.call(Vec::new()),
                                    "Clear all"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn CheckboxRow(
    id: usize,
    row_label: String,
    checked: bool,
    all_selected: Vec<usize>,
    on_change: EventHandler<Vec<usize>>,
) -> Element {
    rsx! {
        label { class: "flex items-center gap-2 px-3 py-1.5 cursor-pointer \
                    hover:bg-gray-50 dark:hover:bg-gray-700",
            input {
                r#type: "checkbox",
                checked,
                class: "rounded border-gray-300 dark:border-gray-600 text-blue-600 \
                        focus:ring-blue-500",
                onchange: move |evt| {
                    let mut sel = all_selected.clone();
                    if evt.checked() {
                        if !sel.contains(&id) {
                            sel.push(id);
                        }
                    } else {
                        sel.retain(|&x| x != id);
                    }
                    on_change.call(sel);
                },
            }
            span { class: "text-sm text-gray-700 dark:text-gray-300 select-none", "{row_label}" }
        }
    }
}

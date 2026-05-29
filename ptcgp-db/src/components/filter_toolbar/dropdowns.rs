use dioxus::prelude::*;
use ptcgp_db_data::{CardSource, Pack, Set};

use ptcgp_db_core::save_data::FilterConfig;

// ---------------------------------------------------------------------------
// Generic dropdown shell — ProfileSelector-style open/close with backdrop.
//
// Row sizing is left entirely to the caller: size images in item sub-components.
// ---------------------------------------------------------------------------

#[component]
pub fn FilterDropdown(
    picker_label: &'static str,
    /// Number of selected items — shown as a badge on the trigger button.
    count: usize,
    children: Element,
) -> Element {
    let mut open = use_signal(|| false);

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "flex items-center gap-1 px-2 py-1.5 rounded-md text-sm font-medium \
                        bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 \
                        text-gray-800 dark:text-gray-100",
                onclick: move |_| open.toggle(),
                span { class: "flex items-center gap-1",
                    "{picker_label}"
                    if count > 0 {
                        span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                            "{count}"
                        }
                    }
                }
                span { class: "ml-1 text-gray-500 dark:text-gray-400 shrink-0",
                    if *open.read() {
                        "▲"
                    } else {
                        "▼"
                    }
                }
            }

            if *open.read() {
                div {
                    class: "fixed inset-0 z-10",
                    onclick: move |_| open.set(false),
                }
                div { class: "absolute left-0 top-full mt-1 z-20 max-h-80 overflow-y-auto \
                            rounded-md border border-gray-200 dark:border-gray-700 \
                            bg-white dark:bg-gray-800 shadow-lg min-w-48 py-1",
                    {children}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Set dropdown — logo images, multi-select
// ---------------------------------------------------------------------------

#[component]
pub fn SetDropdown(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let visible_sets: Vec<&Set> = Set::ALL
        .iter()
        .filter(|s| config.series.map_or(true, |sid| s.series().id() == sid))
        .collect();
    let count = config.sets.len();

    rsx! {
        FilterDropdown { picker_label: "Set", count,
            for set in &visible_sets {
                SetItem {
                    key: "{set.id()}",
                    set,
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
            }
            if !config.sets.is_empty() {
                DropdownClearBtn {
                    on_clear: {
                        let c = config.clone();
                        move |_| on_change.call(clear_sets(c.clone()))
                    },
                }
            }
        }
    }
}

#[component]
fn SetItem(set: &'static Set, config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let id = set.id();
    let checked = config.sets.contains(&id);
    let row_cls = dropdown_row_cls(checked);

    rsx! {
        div {
            class: "{row_cls}",
            onclick: move |_| on_change.call(toggle_set(config.clone(), id, checked)),
            img {
                src: "{set.logo()}",
                alt: "{set.name()}",
                class: "h-10 w-auto max-w-36 object-contain",
            }
            if checked {
                span { class: "ml-auto pl-2 shrink-0 text-blue-500 dark:text-blue-400 font-bold",
                    "✓"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pack dropdown — logo images, multi-select
// ---------------------------------------------------------------------------

#[component]
pub fn PackDropdown(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let visible_packs: Vec<&Pack> = Pack::ALL
        .iter()
        .filter(|p| {
            let series_ok = config.series.map_or(true, |sid| p.series().id() == sid);
            let set_ok = config.sets.is_empty() || config.sets.contains(&p.set().id());
            series_ok && set_ok
        })
        .collect();
    let count = config.packs.len();

    rsx! {
        FilterDropdown { picker_label: "Pack", count,
            for pack in &visible_packs {
                PackItem {
                    key: "{pack.id()}",
                    pack,
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
            }
            if !config.packs.is_empty() {
                DropdownClearBtn {
                    on_clear: {
                        let c = config.clone();
                        move |_| on_change.call(clear_packs(c.clone()))
                    },
                }
            }
        }
    }
}

#[component]
fn PackItem(
    pack: &'static Pack,
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let id = pack.id();
    let checked = config.packs.contains(&id);
    let row_cls = dropdown_row_cls(checked);

    rsx! {
        div {
            class: "{row_cls}",
            onclick: move |_| on_change.call(toggle_pack(config.clone(), id, checked)),
            img {
                src: "{pack.logo()}",
                alt: "{pack.title()}",
                class: "h-10 w-auto max-w-36 object-contain",
            }
            if checked {
                span { class: "ml-auto pl-2 shrink-0 text-blue-500 dark:text-blue-400 font-bold",
                    "✓"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Source dropdown — icon + name, multi-select
// ---------------------------------------------------------------------------

#[component]
pub fn SourceDropdown(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let count = config.sources.len();

    rsx! {
        FilterDropdown { picker_label: "Source", count,
            for source in CardSource::ALL {
                SourceItem {
                    key: "{source.id()}",
                    source,
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
            }
            if !config.sources.is_empty() {
                DropdownClearBtn {
                    on_clear: {
                        let c = config.clone();
                        move |_| on_change.call(clear_sources(c.clone()))
                    },
                }
            }
        }
    }
}

#[component]
fn SourceItem(
    source: &'static CardSource,
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let id = source.id();
    let checked = config.sources.contains(&id);
    let row_cls = dropdown_row_cls(checked);

    rsx! {
        div {
            class: "{row_cls}",
            onclick: move |_| on_change.call(toggle_source(config.clone(), id, checked)),
            img {
                src: "{source.icon()}",
                alt: "{source.name()}",
                class: "h-5 w-5 object-contain shrink-0",
            }
            span { class: "text-sm text-gray-700 dark:text-gray-300", "{source.name()}" }
            if checked {
                span { class: "ml-auto pl-2 shrink-0 text-blue-500 dark:text-blue-400 font-bold",
                    "✓"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared row helpers
// ---------------------------------------------------------------------------

fn dropdown_row_cls(checked: bool) -> &'static str {
    if checked {
        "flex items-center gap-2 px-3 py-2 cursor-pointer select-none \
         bg-blue-50 dark:bg-blue-950 hover:bg-blue-100 dark:hover:bg-blue-900"
    } else {
        "flex items-center gap-2 px-3 py-2 cursor-pointer select-none \
         hover:bg-gray-50 dark:hover:bg-gray-700"
    }
}

#[component]
fn DropdownClearBtn(on_clear: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { class: "border-t border-gray-100 dark:border-gray-700 px-3 py-1.5",
            button {
                r#type: "button",
                class: "text-xs text-gray-400 dark:text-gray-500 \
                        hover:text-gray-600 dark:hover:text-gray-300",
                onclick: move |e| on_clear.call(e),
                "Clear"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// State mutation helpers — kept outside RSX so dx fmt cannot corrupt them.
// ---------------------------------------------------------------------------

fn toggle_set(mut config: FilterConfig, id: usize, was_checked: bool) -> FilterConfig {
    if was_checked {
        config.sets.retain(|&x| x != id);
        let sets = config.sets.clone();
        config.packs.retain(|&pid| Pack::from_id(pid).map_or(false, |p| sets.contains(&p.set().id())));
    } else {
        config.sets.push(id);
    }
    config
}

fn toggle_pack(mut config: FilterConfig, id: usize, was_checked: bool) -> FilterConfig {
    if was_checked {
        config.packs.retain(|&x| x != id);
    } else {
        config.packs.push(id);
    }
    config
}

fn toggle_source(mut config: FilterConfig, id: usize, was_checked: bool) -> FilterConfig {
    if was_checked {
        config.sources.retain(|&x| x != id);
    } else {
        config.sources.push(id);
    }
    config
}

fn clear_sets(mut config: FilterConfig) -> FilterConfig {
    config.sets.clear();
    config.packs.clear();
    config
}

fn clear_packs(mut config: FilterConfig) -> FilterConfig {
    config.packs.clear();
    config
}

fn clear_sources(mut config: FilterConfig) -> FilterConfig {
    config.sources.clear();
    config
}

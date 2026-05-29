use dioxus::prelude::*;
use ptcgp_db_data::{CardSource, Pack, Set};

use ptcgp_db_core::save_data::FilterConfig;

// ---------------------------------------------------------------------------
// Shared dropdown shell — backdrop + styled panel, reused by each dropdown.
// The `open` signal is owned by the caller so each dropdown manages its own state.
// ---------------------------------------------------------------------------

const TRIGGER_CLS: &str = "flex items-center gap-1 px-2 h-10 rounded-md text-sm font-medium \
    bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 \
    text-gray-800 dark:text-gray-100";

#[component]
fn DropdownPanel(open: Signal<bool>, extra_cls: &'static str, children: Element) -> Element {
    rsx! {
        if *open.read() {
            div {
                class: "fixed inset-0 z-10",
                onclick: move |_| open.set(false),
            }
            div { class: "absolute left-0 top-full mt-1 z-20 max-h-80 \
                        overflow-y-auto overflow-x-hidden \
                        rounded-md border border-gray-200 dark:border-gray-700 \
                        bg-white dark:bg-gray-800 shadow-lg py-1 {extra_cls}",
                {children}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Set dropdown — trigger shows set icon when exactly 1 is selected;
// each row shows icon (left) + logo (right).
// ---------------------------------------------------------------------------

#[component]
pub fn SetDropdown(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let mut open = use_signal(|| false);

    let visible_sets: Vec<&Set> = Set::ALL
        .iter()
        .filter(|s| config.series.map_or(true, |sid| s.series().id() == sid))
        .collect();
    let count = config.sets.len();

    // When exactly 1 set is selected, show its icon in the trigger.
    let single_icon_src: Option<String> = if count == 1 {
        Set::from_id(config.sets[0]).map(|s| s.icon().to_string())
    } else {
        None
    };

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "{TRIGGER_CLS}",
                onclick: move |_| open.toggle(),
                if let Some(ref src) = single_icon_src {
                    img {
                        src: "{src}",
                        class: "h-8 w-auto max-w-20 object-contain",
                        alt: "Set",
                    }
                } else {
                    span { "Set" }
                }
                if count > 1 {
                    span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                        "{count}"
                    }
                }
                span { class: "text-gray-500 dark:text-gray-400",
                    if *open.read() {
                        "▲"
                    } else {
                        "▼"
                    }
                }
            }

            DropdownPanel { open, extra_cls: "w-72",
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
}

/// One set row: compact icon on the left, full logo on the right.
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
                src: "{set.icon()}",
                alt: "{set.code()}",
                class: "h-8 w-auto max-w-20 object-contain shrink-0",
            }
            img {
                src: "{set.logo()}",
                alt: "{set.name()}",
                class: "h-10 w-auto max-w-32 object-contain",
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
// Pack dropdown — grouped by set with set icon as section header.
// ---------------------------------------------------------------------------

#[component]
pub fn PackDropdown(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let mut open = use_signal(|| false);
    let count = config.packs.len();

    // Build (set_id, [pack_ids]) groups preserving canonical order.
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for pack in Pack::ALL {
        let series_ok = config.series.map_or(true, |sid| pack.series().id() == sid);
        let set_ok = config.sets.is_empty() || config.sets.contains(&pack.set().id());
        if !series_ok || !set_ok {
            continue;
        }
        let set_id = pack.set().id();
        if let Some(g) = groups.iter_mut().find(|(sid, _)| *sid == set_id) {
            g.1.push(pack.id());
        } else {
            groups.push((set_id, vec![pack.id()]));
        }
    }

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "{TRIGGER_CLS}",
                onclick: move |_| open.toggle(),
                "Pack"
                if count > 0 {
                    span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                        "{count}"
                    }
                }
                span { class: "text-gray-500 dark:text-gray-400",
                    if *open.read() {
                        "▲"
                    } else {
                        "▼"
                    }
                }
            }

            DropdownPanel { open, extra_cls: "w-60",
                for (set_id, pack_ids) in &groups {
                    PackGroup {
                        key: "{set_id}",
                        set_id: *set_id,
                        pack_ids: pack_ids.clone(),
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
}

/// One set-group: header with set icon, then each pack in the group.
#[component]
fn PackGroup(
    set_id: usize,
    pack_ids: Vec<usize>,
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    rsx! {
        if let Some(set) = Set::from_id(set_id) {
            div { class: "flex items-center gap-1.5 px-3 py-1 \
                          bg-gray-50 dark:bg-gray-900",
                img {
                    src: "{set.icon()}",
                    alt: "{set.code()}",
                    class: "h-6 w-auto max-w-14 object-contain",
                }
                span { class: "text-xs font-semibold text-gray-400 dark:text-gray-500",
                    "{set.code()}"
                }
            }
        }
        for &pack_id in &pack_ids {
            if let Some(pack) = Pack::from_id(pack_id) {
                PackItem {
                    key: "{pack_id}",
                    pack,
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
            }
        }
    }
}

#[component]
fn PackItem(pack: &'static Pack, config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
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
                // Generous height — pack logos need more room than set logos.
                class: "h-14 w-auto max-w-40 object-contain",
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
    let mut open = use_signal(|| false);
    let count = config.sources.len();

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "{TRIGGER_CLS}",
                onclick: move |_| open.toggle(),
                "Source"
                if count > 0 {
                    span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                        "{count}"
                    }
                }
                span { class: "text-gray-500 dark:text-gray-400",
                    if *open.read() {
                        "▲"
                    } else {
                        "▼"
                    }
                }
            }

            DropdownPanel { open, extra_cls: "min-w-48",
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
                class: "h-7 w-7 object-contain shrink-0",
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
// State mutation helpers — outside RSX to avoid dx fmt corruption.
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

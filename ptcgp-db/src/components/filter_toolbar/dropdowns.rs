use crate::components::icons::{Check, ChevronDown, ChevronUp};
use dioxus::prelude::*;
use ptcgp_db_data::{CardSource, Pack, Set};

use ptcgp_db_core::save_data::FilterConfig;

// ---------------------------------------------------------------------------
// Shared dropdown shell — backdrop + styled panel, reused by each dropdown.
// The `open` signal is owned by the caller so each dropdown manages its own state.
// ---------------------------------------------------------------------------

const TRIGGER_CLS: &str = "flex items-center gap-1 px-2 h-8 rounded-md text-sm font-medium \
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
                        rounded-md border border-gray-200/60 dark:border-gray-600/60 \
                        bg-white dark:bg-gray-700 \
                        shadow-xl dark:shadow-[0_4px_28px_rgba(0,0,0,0.7)] ring-1 ring-black/5 dark:ring-white/[0.09] \
                        py-1 {extra_cls}",
                {children}
            }
        }
    }
}

/// Header hint shown at the top of every multi-select dropdown.
/// The `border-b` provides a visual separator from the item list below.
#[component]
fn DropdownHint() -> Element {
    rsx! {
        div { class: "px-3 pt-0.5 pb-1 border-b border-gray-100 dark:border-gray-600 \
                      text-xs text-gray-400 dark:text-gray-400",
            "Ctrl+Click to select multiple"
        }
    }
}

// ---------------------------------------------------------------------------
// Set dropdown — trigger shows set icon when exactly 1 is selected;
// each row shows icon (left) + logo (right).
// ---------------------------------------------------------------------------

#[component]
pub fn SetDropdown(config: Signal<FilterConfig>) -> Element {
    let mut open = use_signal(|| false);

    let cfg = config.read();
    let sets = cfg.sets.as_slice();
    let series = cfg.series;
    let visible_sets: Vec<&'static Set> = Set::ALL
        .iter()
        .filter(|s| series.is_none_or(|sid| s.series().id() == sid))
        .collect();
    let count = sets.len();
    let single_icon = if count == 1 {
        Set::from_id(sets[0]).map(|s| s.icon())
    } else {
        None
    };

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "{TRIGGER_CLS}",
                onclick: move |_| open.toggle(),
                if let Some(src) = single_icon {
                    // w-9 (36px) keeps the icon legible while staying within the 21px
                    // headroom at the @lg breakpoint (measured row content 491px vs 512px).
                    img {
                        src: "{src}",
                        class: "h-5 w-9 object-contain shrink-0",
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
                if *open.read() {
                    ChevronUp { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                } else {
                    ChevronDown { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                }
            }

            DropdownPanel { open, extra_cls: "w-72",
                DropdownHint {}
                for set in &visible_sets {
                    SetItem {
                        key: "{set.id()}",
                        set,
                        checked: sets.contains(&set.id()),
                        config,
                        open,
                    }
                }
                if !sets.is_empty() {
                    DropdownClearBtn {
                        on_clear: move |_| {
                            let mut cfg = config.write();
                            cfg.sets.clear();
                            cfg.packs.clear();
                        },
                    }
                }
            }
        }
    }
}

/// One set row: compact icon on the left, full logo on the right.
///
/// Regular click selects only this set; Ctrl+Click toggles it in/out.
#[component]
fn SetItem(
    set: &'static Set,
    checked: bool,
    config: Signal<FilterConfig>,
    mut open: Signal<bool>,
) -> Element {
    let id = set.id();
    let row_cls = dropdown_row_cls(checked);

    let on_click = move |e: MouseEvent| {
        if e.modifiers().ctrl() {
            toggle_set(&mut config.write(), id, checked);
        } else {
            select_only_set(&mut config.write(), id);
            open.set(false);
        }
    };

    rsx! {
        div { class: "{row_cls}", onclick: on_click,
            img {
                src: "{set.icon()}",
                alt: "{set.code()}",
                class: "h-5 w-auto max-w-14 object-contain shrink-0",
            }
            img {
                src: "{set.logo()}",
                alt: "{set.name()}",
                class: "h-10 w-auto max-w-32 object-contain",
            }
            if checked {
                span { class: "ml-auto pl-2 shrink-0",
                    Check { class: "w-4 h-4 text-blue-500 dark:text-blue-400" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pack dropdown — grouped by set with set icon as section header.
// ---------------------------------------------------------------------------

#[component]
pub fn PackDropdown(config: Signal<FilterConfig>) -> Element {
    let mut open = use_signal(|| false);

    let cfg = config.read();
    let packs = cfg.packs.as_slice();
    let sets = cfg.sets.as_slice();
    let series = cfg.series;
    let count = packs.len();

    // Build (set_id, [pack_ids]) groups preserving canonical order.
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for pack in Pack::ALL {
        let series_ok = series.is_none_or(|sid| pack.series().id() == sid);
        let set_ok = sets.is_empty() || sets.contains(&pack.set().id());
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
                if *open.read() {
                    ChevronUp { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                } else {
                    ChevronDown { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                }
            }

            DropdownPanel { open, extra_cls: "w-60",
                DropdownHint {}
                for (set_id, pack_ids) in groups {
                    PackGroup {
                        key: "{set_id}",
                        set_id,
                        pack_ids,
                        config,
                        open,
                    }
                }
                if !packs.is_empty() {
                    DropdownClearBtn { on_clear: move |_| config.write().packs.clear() }
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
    config: Signal<FilterConfig>,
    open: Signal<bool>,
) -> Element {
    let cfg = config.read();
    let checked_packs = cfg.packs.as_slice();
    rsx! {
        if let Some(set) = Set::from_id(set_id) {
            div { class: "flex items-center px-3 py-1 \
                          bg-gray-50 dark:bg-gray-800/60",
                img {
                    src: "{set.icon()}",
                    alt: "{set.code()}",
                    class: "h-6 w-auto max-w-14 object-contain",
                }
            }
        }
        for &pack_id in &pack_ids {
            if let Some(pack) = Pack::from_id(pack_id) {
                PackItem {
                    key: "{pack_id}",
                    pack,
                    checked: checked_packs.contains(&pack_id),
                    config,
                    open,
                }
            }
        }
    }
}

/// One pack row.
///
/// Regular click selects only this pack; Ctrl+Click toggles it in/out.
#[component]
fn PackItem(
    pack: &'static Pack,
    checked: bool,
    config: Signal<FilterConfig>,
    mut open: Signal<bool>,
) -> Element {
    let id = pack.id();
    let row_cls = dropdown_row_cls(checked);

    let on_click = move |e: MouseEvent| {
        if e.modifiers().ctrl() {
            toggle_pack(&mut config.write(), id, checked);
        } else {
            select_only_pack(&mut config.write(), id);
            open.set(false);
        }
    };

    rsx! {
        div { class: "{row_cls}", onclick: on_click,
            img {
                src: "{pack.logo()}",
                alt: "{pack.title()}",
                // Generous height — pack logos need more room than set logos.
                class: "h-14 w-auto max-w-40 object-contain",
            }
            if checked {
                span { class: "ml-auto pl-2 shrink-0",
                    Check { class: "w-4 h-4 text-blue-500 dark:text-blue-400" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Source dropdown — icon + name, multi-select
// ---------------------------------------------------------------------------

#[component]
pub fn SourceDropdown(config: Signal<FilterConfig>) -> Element {
    let mut open = use_signal(|| false);
    let cfg = config.read();
    let sources = cfg.sources.as_slice();
    let count = sources.len();

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
                if *open.read() {
                    ChevronUp { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                } else {
                    ChevronDown { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                }
            }

            DropdownPanel { open, extra_cls: "min-w-48",
                DropdownHint {}
                for source in CardSource::ALL {
                    SourceItem {
                        key: "{source.id()}",
                        source,
                        checked: sources.contains(&source.id()),
                        config,
                        open,
                    }
                }
                if !sources.is_empty() {
                    DropdownClearBtn { on_clear: move |_| config.write().sources.clear() }
                }
            }
        }
    }
}

/// One source row.
///
/// Regular click selects only this source; Ctrl+Click toggles it in/out.
#[component]
fn SourceItem(
    source: &'static CardSource,
    checked: bool,
    config: Signal<FilterConfig>,
    mut open: Signal<bool>,
) -> Element {
    let id = source.id();
    let row_cls = dropdown_row_cls(checked);

    let on_click = move |e: MouseEvent| {
        if e.modifiers().ctrl() {
            toggle_source(&mut config.write(), id, checked);
        } else {
            select_only_source(&mut config.write(), id);
            open.set(false);
        }
    };

    rsx! {
        div { class: "{row_cls}", onclick: on_click,
            img {
                src: "{source.icon()}",
                alt: "{source.name()}",
                class: "h-7 w-7 object-contain shrink-0",
            }
            span { class: "text-sm text-gray-700 dark:text-gray-300", "{source.name()}" }
            if checked {
                span { class: "ml-auto pl-2 shrink-0",
                    Check { class: "w-4 h-4 text-blue-500 dark:text-blue-400" }
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
         bg-blue-50 dark:bg-blue-950/80 hover:bg-blue-100 dark:hover:bg-blue-900/60"
    } else {
        "flex items-center gap-2 px-3 py-2 cursor-pointer select-none \
         hover:bg-gray-50 dark:hover:bg-gray-600"
    }
}

#[component]
fn DropdownClearBtn(on_clear: EventHandler<MouseEvent>) -> Element {
    rsx! {
        div { class: "px-3 py-1.5",
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
// State mutation helpers — operate in-place to avoid cloning FilterConfig.
// ---------------------------------------------------------------------------

fn toggle_set(config: &mut FilterConfig, id: usize, was_checked: bool) {
    if was_checked {
        config.sets.retain(|&x| x != id);
        let sets = config.sets.clone();
        config
            .packs
            .retain(|&pid| Pack::from_id(pid).is_some_and(|p| sets.contains(&p.set().id())));
    } else {
        config.sets.push(id);
    }
}

fn select_only_set(config: &mut FilterConfig, id: usize) {
    if config.sets.as_slice() == [id] {
        config.sets.clear();
        config.packs.clear();
    } else {
        config.sets = vec![id];
        config
            .packs
            .retain(|&pid| Pack::from_id(pid).is_some_and(|p| p.set().id() == id));
    }
}

fn toggle_pack(config: &mut FilterConfig, id: usize, was_checked: bool) {
    if was_checked {
        config.packs.retain(|&x| x != id);
    } else {
        config.packs.push(id);
    }
}

fn select_only_pack(config: &mut FilterConfig, id: usize) {
    if config.packs.as_slice() == [id] {
        config.packs.clear();
    } else {
        config.packs = vec![id];
    }
}

fn toggle_source(config: &mut FilterConfig, id: usize, was_checked: bool) {
    if was_checked {
        config.sources.retain(|&x| x != id);
    } else {
        config.sources.push(id);
    }
}

fn select_only_source(config: &mut FilterConfig, id: usize) {
    if config.sources.as_slice() == [id] {
        config.sources.clear();
    } else {
        config.sources = vec![id];
    }
}

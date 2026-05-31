mod controls;
mod dropdowns;
mod pickers;

use controls::{AnyVersionFilter, CountFilter, GoalFilter, KindFilter, NameFilter, TriStateFilter};
use dropdowns::{PackDropdown, SetDropdown, SourceDropdown};
use pickers::{ElementGroup, RarityGroup};

use crate::components::icons::Bars3;
use dioxus::prelude::*;
use ptcgp_db_core::AppSettings;
use ptcgp_db_core::save_data::FilterConfig;
use ptcgp_db_data::{Series, Stage};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub enum FilterMode {
    /// Card Catalog mode: owned-count threshold in advanced view.
    Catalog,
    /// Analysis / Trade mode: goal input always in primary row; no owned-count.
    /// Callers should initialize `FilterConfig::obtainable = Some(true)`.
    Analysis,
}

/// Single-row filter toolbar with a floating advanced panel.
///
/// Primary row (sm+): Name, [Goal if Analysis], Set, Pack, Source, Series, Kind.
/// Narrow (< sm): Name, [Goal if Analysis] + "Filters" button that opens the panel
/// with all filters including the primary ones.
/// Advanced floating panel: Rarity, Element, Stage, Ex, Mega, Foil, Obtainable,
/// Count (Catalog) / Any-version (Analysis) — plus primary filters on narrow.
#[component]
pub fn FilterToolbar(config: Signal<FilterConfig>, mode: FilterMode) -> Element {
    let settings = use_context::<Signal<AppSettings>>();
    let ignore_unobtainable = settings.read().ignore_unobtainable_sets();
    let mut panel_open = use_signal(|| false);

    let (total_active, ex, mega, foil, obtainable) = {
        let cfg = config.read();
        (
            count_active(&cfg, ignore_unobtainable),
            cfg.ex,
            cfg.mega,
            cfg.foil,
            cfg.obtainable,
        )
    };

    rsx! {
        // @container makes breakpoints respond to the available container width,
        // not the viewport width. This prevents toolbar overflow into the detail panel
        // at medium viewport widths where the list column is narrower than the viewport.
        div { class: "relative @container",
            // ── Primary row ─────────────────────────────────────────────────
            // flex-nowrap prevents wrapping; filters that don't fit at a given
            // breakpoint are hidden here and surfaced in the floating panel.
            div { class: "flex flex-nowrap items-end gap-2",
                // Name — always visible
                div { class: "flex-shrink-0",
                    NameFilter { config }
                }

                // Goal — Analysis mode only, always visible
                if mode == FilterMode::Analysis {
                    div { class: "flex-shrink-0",
                        GoalFilter { config }
                    }
                }

                // Set + Pack + Source — visible when container >= 640px
                div { class: "hidden @sm:flex items-end gap-2",
                    SetDropdown { config }
                    PackDropdown { config }
                    SourceDropdown { config }
                }

                // Series + Kind — visible when container >= 768px
                div { class: "hidden @md:flex items-end gap-2",
                    SeriesFilter { config }
                    KindFilter { config }
                }

                // Advanced button — always visible, badge shows total active filter count
                button {
                    r#type: "button",
                    title: "Advanced Filters",
                    class: "flex-shrink-0 flex items-center gap-1.5 px-2.5 py-1.5 rounded-md \
                            text-xs font-medium text-gray-600 dark:text-gray-300 \
                            bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600",
                    onclick: move |_| panel_open.toggle(),
                    Bars3 { class: "w-5 h-5" }
                    if total_active > 0 {
                        span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                            "{total_active}"
                        }
                    }
                }
            }

            // ── Floating panel ───────────────────────────────────────────────
            if *panel_open.read() {
                // Dismiss backdrop
                div {
                    class: "fixed inset-0 z-40",
                    onclick: move |_| panel_open.set(false),
                }

                // Panel box — floating below the primary row
                div { class: "absolute left-0 top-full mt-1 z-50 \
                            rounded-lg border border-gray-200 dark:border-gray-700 \
                            bg-gray-50 dark:bg-gray-900 shadow-lg \
                            p-4 flex flex-col gap-3 \
                            min-w-64 max-w-[min(640px,calc(100vw-1rem))]",

                    // ── Primary filters hidden from the row at narrow widths ──
                    // Set/Pack/Source: in panel when container < 640px
                    div { class: "flex flex-col gap-3 @sm:hidden",
                        SetDropdown { config }
                        PackDropdown { config }
                        SourceDropdown { config }
                    }
                    // Series/Kind: in panel when container < 768px
                    div { class: "flex flex-col gap-3 @md:hidden",
                        SeriesFilter { config }
                        KindFilter { config }
                    }

                    // ── Advanced filters (always in panel) ───────────────────
                    RarityGroup { config }
                    ElementGroup { config }
                    StageFilter { config }
                    TriStateFilter {
                        filter_label: "Ex",
                        only_text: "Ex only",
                        exclude_text: "No ex",
                        value: ex,
                        on_change: move |v: Option<bool>| config.write().ex = v,
                    }
                    TriStateFilter {
                        filter_label: "Mega",
                        only_text: "Mega only",
                        exclude_text: "No mega",
                        value: mega,
                        on_change: move |v: Option<bool>| config.write().mega = v,
                    }
                    TriStateFilter {
                        filter_label: "Foil",
                        only_text: "Foil only",
                        exclude_text: "Non-foil",
                        value: foil,
                        on_change: move |v: Option<bool>| config.write().foil = v,
                    }
                    if !ignore_unobtainable {
                        TriStateFilter {
                            filter_label: "Obtainable",
                            only_text: "Obtainable",
                            exclude_text: "Unobtainable",
                            value: obtainable,
                            on_change: move |v: Option<bool>| config.write().obtainable = v,
                        }
                    }
                    match &mode {
                        FilterMode::Catalog => rsx! {
                            CountFilter { config }
                        },
                        FilterMode::Analysis => rsx! {
                            AnyVersionFilter { config }
                        },
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Series and Stage — live here to avoid importing data types in controls.rs
// ---------------------------------------------------------------------------

#[component]
fn SeriesFilter(config: Signal<FilterConfig>) -> Element {
    let series = config.read().series;
    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Series" }
            div { class: "flex",
                SeriesBtn {
                    btn_label: "All",
                    active: series.is_none(),
                    target_id: None,
                    config,
                }
                for s in Series::ALL {
                    SeriesBtn {
                        key: "{s.id()}",
                        btn_label: "{s.code()}",
                        active: series == Some(s.id()),
                        target_id: Some(s.id()),
                        config,
                    }
                }
            }
        }
    }
}

#[component]
fn SeriesBtn(
    btn_label: String,
    active: bool,
    target_id: Option<usize>,
    config: Signal<FilterConfig>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            onclick: move |_| {
                let mut cfg = config.write();
                if cfg.series != target_id {
                    cfg.sets.clear();
                    cfg.packs.clear();
                }
                cfg.series = target_id;
            },
            "{btn_label}"
        }
    }
}

#[component]
fn StageFilter(config: Signal<FilterConfig>) -> Element {
    let stage = config.read().stage;
    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Stage" }
            div { class: "flex",
                StageBtn {
                    btn_label: "All",
                    active: stage.is_none(),
                    target_id: None,
                    config,
                }
                for s in Stage::ALL {
                    StageBtn {
                        key: "{s.id()}",
                        btn_label: "{s.name()}",
                        active: stage == Some(s.id()),
                        target_id: Some(s.id()),
                        config,
                    }
                }
            }
        }
    }
}

#[component]
fn StageBtn(
    btn_label: String,
    active: bool,
    target_id: Option<usize>,
    config: Signal<FilterConfig>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            onclick: move |_| config.write().stage = target_id,
            "{btn_label}"
        }
    }
}

// ---------------------------------------------------------------------------
// Shared visual helpers
// ---------------------------------------------------------------------------

pub(super) fn seg_btn_cls(active: bool) -> &'static str {
    if active {
        "relative px-2.5 py-1 text-xs font-medium border border-blue-600 dark:border-blue-500 \
         bg-blue-600 text-white z-10 -ml-px first:ml-0 first:rounded-l-md last:rounded-r-md \
         focus:outline-none"
    } else {
        "relative px-2.5 py-1 text-xs font-medium border border-gray-300 dark:border-gray-600 \
         bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 -ml-px first:ml-0 \
         first:rounded-l-md last:rounded-r-md hover:bg-gray-50 dark:hover:bg-gray-700 \
         focus:outline-none"
    }
}

fn count_active(config: &FilterConfig, ignore_unobtainable: bool) -> usize {
    let mut n = count_primary_active(config);
    n += count_advanced_active(config, ignore_unobtainable);
    n
}

fn count_primary_active(config: &FilterConfig) -> usize {
    let mut n = 0;
    if config.name_query.as_deref().is_some_and(|s| !s.is_empty()) {
        n += 1;
    }
    if config.series.is_some() {
        n += 1;
    }
    if !config.sets.is_empty() {
        n += 1;
    }
    if !config.packs.is_empty() {
        n += 1;
    }
    if !config.sources.is_empty() {
        n += 1;
    }
    if config.card_kind.is_some() {
        n += 1;
    }
    n
}

fn count_advanced_active(config: &FilterConfig, ignore_unobtainable: bool) -> usize {
    let mut n = 0;
    if !config.rarities.is_empty() {
        n += 1;
    }
    if !config.elements.is_empty() {
        n += 1;
    }
    if config.stage.is_some() {
        n += 1;
    }
    if config.ex.is_some() {
        n += 1;
    }
    if config.mega.is_some() {
        n += 1;
    }
    if config.foil.is_some() {
        n += 1;
    }
    if !ignore_unobtainable && config.obtainable.is_some() {
        n += 1;
    }
    if config.owned_count.is_some() {
        n += 1;
    }
    n
}

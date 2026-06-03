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
    /// Card Catalog mode: name search + owned-count threshold.
    Catalog,
    /// Trade mode: name search + goal input; no owned-count.
    Trade,
    /// Summary mode: goal input, no name search (saves primary-row space).
    Summary,
}

/// Single-row filter toolbar with a floating advanced panel.
///
/// Primary row: Name, [Goal if Trade/Summary], Set, Pack, Series, Source, Kind.
/// Controls are revealed progressively as the container widens; those that don't
/// fit are surfaced in the floating panel.
/// Advanced floating panel: Rarity, Element, Stage, Ex, Mega, Foil, Obtainable,
/// Count (Catalog) / Any-version (Trade/Summary) — plus primary filters on narrow.
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

    // Set + Pack: show as early as possible without overflowing the container.
    // Catalog/Summary fit at @lg (512px) — measured min content 491px, 21px margin.
    // Trade needs @xl (576px) — Name+Goal+Set+Pack fills ~541px; the sidebar appearing
    // at md drops the container to 528px, so @xl avoids rendering below that floor.
    let (sp_row_cls, sp_panel_cls) = if mode == FilterMode::Trade {
        (
            "hidden @xl:flex items-center gap-2",
            "flex flex-col gap-3 @xl:hidden",
        )
    } else {
        (
            "hidden @lg:flex items-center gap-2",
            "flex flex-col gap-3 @lg:hidden",
        )
    };
    // Source: Catalog/Summary can share the sp threshold (still fits at @lg).
    // Trade adds Source at @2xl (672px) — adding Source brings the total to ~625px,
    // which needs more room than @xl provides.
    let (source_row_cls, source_panel_cls) = if mode == FilterMode::Trade {
        (
            "hidden @2xl:flex items-center gap-2",
            "flex flex-col gap-3 @2xl:hidden",
        )
    } else {
        (sp_row_cls, sp_panel_cls)
    };
    // Series: second tier — logically groups with Set/Pack but needs more horizontal space.
    // Catalog container is fixed at 808px (840px list column − 32px padding) at xl+, so
    // @3xl (768px) is the right threshold — Series consistently shows on the primary row.
    // Trade/Summary use max-w-4xl (~848px container), so @3xl works there too.
    let (s_row_cls, s_panel_cls) = (
        "hidden @3xl:flex items-center gap-2",
        "flex flex-col gap-3 @3xl:hidden",
    );
    let (k_row_cls, k_panel_cls) = match mode {
        FilterMode::Summary => (
            "hidden @3xl:flex items-center gap-2",
            "flex flex-col gap-3 @3xl:hidden",
        ),
        // Trade: Kind doesn't fit alongside Name+Goal+Set+Pack+Source+Series (~760px total).
        // Catalog: the fixed 808px container can't fit Kind either (~870px needed), and
        // using a threshold above 808px causes a jarring jump when the xl detail panel appears.
        FilterMode::Trade | FilterMode::Catalog => ("hidden", "flex flex-col gap-3"),
    };

    rsx! {
        // @container makes breakpoints respond to the available container width,
        // not the viewport width. This prevents toolbar overflow into the detail panel
        // at medium viewport widths where the list column is narrower than the viewport.
        div { class: "relative @container",
            // ── Primary row ─────────────────────────────────────────────────
            // flex-nowrap prevents wrapping; filters that don't fit at a given
            // breakpoint are hidden here and surfaced in the floating panel.
            div { class: "flex flex-nowrap items-center gap-2 \
                          bg-white dark:bg-gray-800 \
                          border border-gray-200/80 dark:border-gray-700/80 \
                          rounded-lg px-3 py-2 \
                          shadow dark:shadow-[0_2px_12px_rgba(0,0,0,0.5)] dark:ring-1 dark:ring-white/[0.07]",
                // Name — Catalog and Trade only; Summary omits it to save space
                if mode != FilterMode::Summary {
                    NameFilter { config }
                }

                // Goal — Trade and Summary modes
                if mode == FilterMode::Trade || mode == FilterMode::Summary {
                    GoalFilter { config }
                }

                // Series → Set → Pack: series contain sets, sets contain packs
                div { class: "{s_row_cls}",
                    SeriesFilter { config, labeled: false }
                }

                // Set + Pack — first responsive tier (lower breakpoint than Series)
                div { class: "{sp_row_cls}",
                    SetDropdown { config }
                    PackDropdown { config }
                }

                // Source — same tier as Set/Pack, ordered after the set-hierarchy group
                div { class: "{source_row_cls}",
                    SourceDropdown { config }
                }

                // Kind
                div { class: "{k_row_cls}",
                    KindFilter { config, labeled: false }
                }

                // Push the advanced button to the right end of the toolbar
                div { class: "flex-1 min-w-0" }

                // Advanced button — always visible, badge shows total active filter count
                button {
                    r#type: "button",
                    title: "Advanced Filters",
                    class: "shrink-0 flex items-center gap-1.5 px-2.5 py-1.5 rounded-md \
                            text-xs font-medium text-gray-600 dark:text-gray-300 \
                            bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 \
                            shadow-sm active:shadow-none active:translate-y-px",
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
                            rounded-lg border border-gray-200/60 dark:border-gray-600/60 \
                            bg-white/95 dark:bg-gray-700/95 backdrop-blur-sm \
                            shadow-2xl dark:shadow-[0_8px_40px_rgba(0,0,0,0.75)] ring-1 ring-black/5 dark:ring-white/[0.09] \
                            p-4 flex flex-col gap-3 \
                            min-w-64 max-w-[min(640px,calc(100vw-1rem))]",

                    // ── Primary filters hidden from the row at narrow widths ──
                    div { class: "{s_panel_cls}",
                        SeriesFilter { config }
                    }
                    div { class: "{sp_panel_cls}",
                        SetDropdown { config }
                        PackDropdown { config }
                    }
                    div { class: "{source_panel_cls}",
                        SourceDropdown { config }
                    }
                    div { class: "{k_panel_cls}",
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
                        FilterMode::Trade | FilterMode::Summary => rsx! {
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
fn SeriesFilter(config: Signal<FilterConfig>, #[props(default = true)] labeled: bool) -> Element {
    let series = config.read().series;
    let wrapper_cls = if labeled {
        "flex flex-col gap-0.5"
    } else {
        "flex items-center gap-1"
    };
    let label_cls = if labeled {
        "text-xs font-medium text-gray-500 dark:text-gray-400"
    } else {
        "text-xs font-medium text-gray-400 dark:text-gray-500 select-none"
    };
    rsx! {
        div { class: "{wrapper_cls}",
            span { class: "{label_cls}", "Series" }
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
         shadow-inner focus:outline-none"
    } else {
        "relative px-2.5 py-1 text-xs font-medium border border-gray-300 dark:border-gray-600 \
         bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 -ml-px first:ml-0 \
         first:rounded-l-md last:rounded-r-md hover:bg-gray-50 dark:hover:bg-gray-700 \
         shadow-sm focus:outline-none"
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

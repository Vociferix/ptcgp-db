mod controls;
mod dropdowns;
mod pickers;

use controls::{AnyVersionFilter, CountFilter, GoalFilter, KindFilter, NameFilter, TriStateFilter};
use dropdowns::{PackDropdown, SetDropdown, SourceDropdown};
use pickers::{ElementGroup, RarityGroup};

use dioxus::prelude::*;
use ptcgp_db_core::save_data::FilterConfig;
use ptcgp_db_core::AppSettings;
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
pub fn FilterToolbar(
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
    mode: FilterMode,
) -> Element {
    let settings = use_context::<Signal<AppSettings>>();
    let ignore_unobtainable = settings.read().ignore_unobtainable_sets();
    let mut panel_open = use_signal(|| false);

    let total_active = count_active(&config, ignore_unobtainable);
    let adv_active = count_advanced_active(&config, ignore_unobtainable);

    rsx! {
        div { class: "relative",
            // ── Primary row ─────────────────────────────────────────────────
            div { class: "flex flex-wrap items-center gap-2",
                // Name — always visible
                NameFilter { config: config.clone(), on_change: on_change.clone() }

                // Goal — Analysis mode only, always primary
                if mode == FilterMode::Analysis {
                    GoalFilter { config: config.clone(), on_change: on_change.clone() }
                }

                // Dropdowns + Series + Kind — hidden on narrow, inline on sm+
                div { class: "hidden sm:contents",
                    SetDropdown { config: config.clone(), on_change: on_change.clone() }
                    PackDropdown { config: config.clone(), on_change: on_change.clone() }
                    SourceDropdown { config: config.clone(), on_change: on_change.clone() }
                    SeriesFilter { config: config.clone(), on_change: on_change.clone() }
                    KindFilter { config: config.clone(), on_change: on_change.clone() }
                }

                // Advanced button — sm+ only
                button {
                    r#type: "button",
                    class: "hidden sm:flex items-center gap-1 px-2.5 py-1.5 rounded-md \
                            text-xs font-medium text-gray-600 dark:text-gray-300 \
                            bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600",
                    onclick: move |_| panel_open.toggle(),
                    "Advanced"
                    if adv_active > 0 {
                        span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                            "{adv_active}"
                        }
                    }
                    if *panel_open.read() {
                        "▲"
                    } else {
                        "▼"
                    }
                }

                // Filters button — narrow only
                button {
                    r#type: "button",
                    class: "sm:hidden flex items-center gap-1.5 px-3 py-1.5 rounded-md border \
                            border-gray-300 dark:border-gray-600 text-sm \
                            bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 \
                            hover:bg-gray-50 dark:hover:bg-gray-700",
                    onclick: move |_| panel_open.toggle(),
                    "Filters"
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

                    // ── Narrow-only: primary filters that aren't in the row ──
                    div { class: "contents sm:hidden",
                        SetDropdown {
                            config: config.clone(),
                            on_change: on_change.clone(),
                        }
                        PackDropdown {
                            config: config.clone(),
                            on_change: on_change.clone(),
                        }
                        SourceDropdown {
                            config: config.clone(),
                            on_change: on_change.clone(),
                        }
                        SeriesFilter {
                            config: config.clone(),
                            on_change: on_change.clone(),
                        }
                        KindFilter {
                            config: config.clone(),
                            on_change: on_change.clone(),
                        }
                    }

                    // ── Advanced filters (always in panel) ───────────────────
                    RarityGroup { config: config.clone(), on_change: on_change.clone() }
                    ElementGroup { config: config.clone(), on_change: on_change.clone() }
                    StageFilter { config: config.clone(), on_change: on_change.clone() }
                    TriStateFilter {
                        filter_label: "Ex",
                        only_text: "Ex only",
                        exclude_text: "No ex",
                        value: config.ex,
                        on_change: {
                            let config = config.clone();
                            let on_change = on_change.clone();
                            move |v: Option<bool>| {
                                let mut c = config.clone();
                                c.ex = v;
                                on_change.call(c);
                            }
                        },
                    }
                    TriStateFilter {
                        filter_label: "Mega",
                        only_text: "Mega only",
                        exclude_text: "No mega",
                        value: config.mega,
                        on_change: {
                            let config = config.clone();
                            let on_change = on_change.clone();
                            move |v: Option<bool>| {
                                let mut c = config.clone();
                                c.mega = v;
                                on_change.call(c);
                            }
                        },
                    }
                    TriStateFilter {
                        filter_label: "Foil",
                        only_text: "Foil only",
                        exclude_text: "Non-foil",
                        value: config.foil,
                        on_change: {
                            let config = config.clone();
                            let on_change = on_change.clone();
                            move |v: Option<bool>| {
                                let mut c = config.clone();
                                c.foil = v;
                                on_change.call(c);
                            }
                        },
                    }
                    if !ignore_unobtainable {
                        TriStateFilter {
                            filter_label: "Obtainable",
                            only_text: "Obtainable",
                            exclude_text: "Unobtainable",
                            value: config.obtainable,
                            on_change: {
                                let config = config.clone();
                                let on_change = on_change.clone();
                                move |v: Option<bool>| {
                                    let mut c = config.clone();
                                    c.obtainable = v;
                                    on_change.call(c);
                                }
                            },
                        }
                    }
                    match &mode {
                        FilterMode::Catalog => rsx! {
                            CountFilter { config: config.clone(), on_change: on_change.clone() }
                        },
                        FilterMode::Analysis => rsx! {
                            AnyVersionFilter { config: config.clone(), on_change: on_change.clone() }
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
fn SeriesFilter(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    rsx! {
        div { class: "flex items-center gap-1.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400 shrink-0",
                "Series"
            }
            div { class: "flex",
                SeriesBtn {
                    btn_label: "All",
                    active: config.series.is_none(),
                    target_id: None,
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
                for series in Series::ALL {
                    SeriesBtn {
                        key: "{series.id()}",
                        btn_label: series.code().to_string(),
                        active: config.series == Some(series.id()),
                        target_id: Some(series.id()),
                        config: config.clone(),
                        on_change: on_change.clone(),
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
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            onclick: move |_| {
                let mut c = config.clone();
                if c.series != target_id {
                    c.sets.clear();
                    c.packs.clear();
                }
                c.series = target_id;
                on_change.call(c);
            },
            "{btn_label}"
        }
    }
}

#[component]
fn StageFilter(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    rsx! {
        div { class: "flex items-center gap-1.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400 shrink-0",
                "Stage"
            }
            div { class: "flex",
                StageBtn {
                    btn_label: "All",
                    active: config.stage.is_none(),
                    target_id: None,
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
                for stage in Stage::ALL {
                    StageBtn {
                        key: "{stage.id()}",
                        btn_label: stage.name().to_string(),
                        active: config.stage == Some(stage.id()),
                        target_id: Some(stage.id()),
                        config: config.clone(),
                        on_change: on_change.clone(),
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
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            onclick: move |_| {
                let mut c = config.clone();
                c.stage = target_id;
                on_change.call(c);
            },
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

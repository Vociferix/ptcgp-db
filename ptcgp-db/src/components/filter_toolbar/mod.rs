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

/// Controls which filter dimensions are visible and what mode-specific behavior applies.
#[derive(Clone, PartialEq)]
pub enum FilterMode {
    /// Card Catalog mode: shows owned-count threshold, no goal or any-version-owned.
    Catalog,
    /// Analysis / Trade mode: shows goal input + any-version-owned toggle; hides owned-count.
    /// Callers should initialize `FilterConfig::obtainable = Some(true)`.
    Analysis,
}

/// Configurable filter toolbar used by the Card Catalog, Analysis, and Trade pages.
///
/// The parent owns the [`FilterConfig`] state; every interaction calls `on_change` with an
/// updated clone. Reads `Signal<AppSettings>` from context.
#[component]
pub fn FilterToolbar(
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
    mode: FilterMode,
) -> Element {
    let settings = use_context::<Signal<AppSettings>>();
    let ignore_unobtainable = settings.read().ignore_unobtainable_sets();
    let mut filters_open = use_signal(|| false);
    let mut advanced_open = use_signal(|| false);

    let active = count_active(&config, ignore_unobtainable);
    let panel_class = if *filters_open.read() {
        "block"
    } else {
        "hidden sm:block"
    };

    rsx! {
        div { class: "space-y-2",
            // ── Always-visible bar: name input + narrow "Filters" button ────────
            div { class: "flex items-center gap-2",
                NameFilter { config: config.clone(), on_change: on_change.clone() }
                button {
                    r#type: "button",
                    class: "sm:hidden flex items-center gap-1.5 px-3 py-1.5 rounded-md border \
                            border-gray-300 dark:border-gray-600 text-sm \
                            bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 \
                            hover:bg-gray-50 dark:hover:bg-gray-700",
                    onclick: move |_| filters_open.toggle(),
                    "Filters"
                    if active > 0 {
                        span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                            "{active}"
                        }
                    }
                }
            }

            // ── Primary filter panel ─────────────────────────────────────────────
            div { class: "{panel_class} flex flex-col gap-3",
                // Row 1 — image dropdowns
                div { class: "flex flex-wrap gap-2 items-center",
                    SetDropdown { config: config.clone(), on_change: on_change.clone() }
                    PackDropdown { config: config.clone(), on_change: on_change.clone() }
                    SourceDropdown { config: config.clone(), on_change: on_change.clone() }
                }

                // Row 2 — series + kind segmented groups
                div { class: "flex flex-wrap gap-x-5 gap-y-2 items-center",
                    SeriesFilter { config: config.clone(), on_change: on_change.clone() }
                    KindFilter { config: config.clone(), on_change: on_change.clone() }
                }

                // Row 3 — rarity
                RarityGroup { config: config.clone(), on_change: on_change.clone() }

                // Row 4 — element
                ElementGroup { config: config.clone(), on_change: on_change.clone() }

                // ── Advanced toggle ──────────────────────────────────────────────
                button {
                    r#type: "button",
                    class: "self-start flex items-center gap-1 text-xs \
                            text-gray-400 dark:text-gray-500 \
                            hover:text-gray-600 dark:hover:text-gray-300",
                    onclick: move |_| advanced_open.toggle(),
                    if *advanced_open.read() {
                        "▲"
                    } else {
                        "▼"
                    }
                    "Advanced filters"
                }

                // ── Advanced section ─────────────────────────────────────────────
                if *advanced_open.read() {
                    // Row A — per-property tri-state toggles + stage
                    div { class: "flex flex-wrap gap-x-5 gap-y-2 items-center",
                        StageFilter {
                            config: config.clone(),
                            on_change: on_change.clone(),
                        }
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
                    }

                    // Row B — count / goal
                    div { class: "flex flex-wrap gap-x-5 gap-y-2 items-center",
                        match &mode {
                            FilterMode::Catalog => rsx! {
                                CountFilter { config: config.clone(), on_change: on_change.clone() }
                            },
                            FilterMode::Analysis => rsx! {
                                GoalFilter { config: config.clone(), on_change: on_change.clone() }
                                AnyVersionFilter { config: config.clone(), on_change: on_change.clone() }
                            },
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Series and Stage — kept here because they reference ptcgp_db_data types
// that would clutter controls.rs's import list unnecessarily.
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
                // Changing series invalidates set/pack selections
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

/// CSS classes for a segmented button group entry (active / inactive variant).
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

/// Count of non-default active filters for the badge.
pub(super) fn count_active(config: &FilterConfig, ignore_unobtainable: bool) -> usize {
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
    if !config.rarities.is_empty() {
        n += 1;
    }
    if config.card_kind.is_some() {
        n += 1;
    }
    if config.ex.is_some() {
        n += 1;
    }
    if config.mega.is_some() {
        n += 1;
    }
    if config.stage.is_some() {
        n += 1;
    }
    if !config.elements.is_empty() {
        n += 1;
    }
    if config.foil.is_some() {
        n += 1;
    }
    if !config.sources.is_empty() {
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

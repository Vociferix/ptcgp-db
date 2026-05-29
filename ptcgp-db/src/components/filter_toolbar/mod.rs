mod controls;
mod dropdowns;
mod pickers;

use controls::{
    AnyVersionFilter, CountFilter, GoalFilter, KindFilter, NameFilter, SeriesFilter, StageFilter,
    ThreeWayFilter,
};
use dropdowns::{PackDropdown, SetDropdown, SourceDropdown};
use pickers::{ElementPicker, RarityPicker};

use dioxus::prelude::*;
use ptcgp_db_core::save_data::FilterConfig;
use ptcgp_db_core::AppSettings;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Controls which filter dimensions are visible and what mode-specific behavior applies.
#[derive(Clone, PartialEq)]
pub enum FilterMode {
    /// Card Catalog mode: shows owned-count threshold, no goal or any-version-owned.
    Catalog,
    /// Analysis / Trade mode: shows goal input + any-version-owned toggle; hides owned-count.
    /// Callers are responsible for initializing `FilterConfig::obtainable = Some(true)`.
    Analysis,
}

/// Configurable filter toolbar used by the Card Catalog, Analysis, and Trade pages.
///
/// The parent owns the [`FilterConfig`] state. Every interaction calls `on_change` with an
/// updated clone of the config. Reads `Signal<AppSettings>` from context to conditionally
/// hide the Obtainable filter when `ignore_unobtainable_sets` is on.
#[component]
pub fn FilterToolbar(
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
    mode: FilterMode,
) -> Element {
    let settings = use_context::<Signal<AppSettings>>();
    let ignore_unobtainable = settings.read().ignore_unobtainable_sets();
    let mut expanded = use_signal(|| false);

    let active = count_active(&config, ignore_unobtainable);
    let panel_class = if *expanded.read() {
        "block"
    } else {
        "hidden sm:block"
    };

    rsx! {
        div { class: "space-y-2",
            // Narrow-viewport toggle button
            button {
                r#type: "button",
                class: "sm:hidden flex items-center gap-1.5 px-3 py-1.5 rounded-md border \
                        border-gray-300 dark:border-gray-600 text-sm \
                        bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 \
                        hover:bg-gray-50 dark:hover:bg-gray-700",
                onclick: move |_| expanded.toggle(),
                "Filters"
                if active > 0 {
                    span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                        "{active}"
                    }
                }
            }

            // Filter panel (always visible on sm:+, toggleable on narrow)
            div { class: "{panel_class} flex flex-col gap-3",
                // Row 1: text search + taxonomy selects
                div { class: "flex flex-wrap gap-2 items-center",
                    NameFilter { config: config.clone(), on_change: on_change.clone() }
                    SeriesFilter { config: config.clone(), on_change: on_change.clone() }
                    KindFilter { config: config.clone(), on_change: on_change.clone() }
                    StageFilter { config: config.clone(), on_change: on_change.clone() }
                }

                // Row 2: multi-select dropdowns
                div { class: "flex flex-wrap gap-2 items-center",
                    SetDropdown { config: config.clone(), on_change: on_change.clone() }
                    PackDropdown { config: config.clone(), on_change: on_change.clone() }
                    SourceDropdown { config: config.clone(), on_change: on_change.clone() }
                }

                // Row 3: rarity icon toggles
                RarityPicker { config: config.clone(), on_change: on_change.clone() }

                // Row 4: element icon toggles
                ElementPicker { config: config.clone(), on_change: on_change.clone() }

                // Row 5: property toggles
                div { class: "flex flex-wrap gap-x-4 gap-y-2 items-center",
                    ThreeWayFilter {
                        label: "Ex",
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
                    ThreeWayFilter {
                        label: "Mega",
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
                    ThreeWayFilter {
                        label: "Foil",
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
                        ThreeWayFilter {
                            label: "Obtainable",
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

                // Row 6: count / goal
                div { class: "flex flex-wrap gap-x-4 gap-y-2 items-center",
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

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

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

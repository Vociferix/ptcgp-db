use dioxus::prelude::*;
use ptcgp_db_core::save_data::{CardKindFilter, CountThreshold, FilterConfig};
use ptcgp_db_data::{Series, Stage};

use crate::components::toggle::Toggle;

// ---------------------------------------------------------------------------
// Row 1: text search, series, kind, stage
// ---------------------------------------------------------------------------

#[component]
pub fn NameFilter(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let value = config.name_query.clone().unwrap_or_default();
    rsx! {
        input {
            r#type: "text",
            placeholder: "Name or number…",
            value: "{value}",
            class: "px-2 py-1 text-sm rounded-md border border-gray-300 dark:border-gray-600 \
                    bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                    focus:outline-none focus:ring-2 focus:ring-blue-500 w-44",
            oninput: move |evt| {
                let mut c = config.clone();
                let v = evt.value();
                c.name_query = if v.is_empty() { None } else { Some(v) };
                on_change.call(c);
            },
        }
    }
}

#[component]
pub fn SeriesFilter(
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let sel = config.series.map(|id| id.to_string()).unwrap_or_default();
    rsx! {
        div { class: "flex items-center gap-1.5",
            label { class: "text-sm text-gray-600 dark:text-gray-400", "Series" }
            select {
                class: "py-1 pl-1.5 pr-6 text-sm rounded-md border border-gray-300 dark:border-gray-600 \
                        bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                        focus:outline-none focus:ring-2 focus:ring-blue-500",
                value: "{sel}",
                onchange: move |evt| {
                    let mut c = config.clone();
                    c.series = evt.value().parse::<usize>().ok();
                    // Clear set/pack filters when series changes — they may now be invalid
                    c.sets.clear();
                    c.packs.clear();
                    on_change.call(c);
                },
                option { value: "", "All" }
                for series in Series::ALL {
                    option { key: "{series.id()}", value: "{series.id()}", "{series.code()}" }
                }
            }
        }
    }
}

#[component]
pub fn KindFilter(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let btn_base = "px-2.5 py-1 text-sm border-y first:border-l first:rounded-l-md \
                    last:border-r last:rounded-r-md border-gray-300 dark:border-gray-600 \
                    focus:outline-none focus:z-10";
    let active = "bg-blue-600 text-white border-blue-600 dark:border-blue-600";
    let inactive = "bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 \
                    hover:bg-gray-50 dark:hover:bg-gray-700";

    rsx! {
        div { class: "flex items-center gap-1.5",
            span { class: "text-sm text-gray-600 dark:text-gray-400", "Kind" }
            div { class: "flex",
                button {
                    r#type: "button",
                    class: if config.card_kind.is_none() { "{btn_base} {active}" } else { "{btn_base} {inactive}" },
                    onclick: {
                        let config = config.clone();
                        move |_| {
                            let mut c = config.clone();
                            c.card_kind = None;
                            on_change.call(c);
                        }
                    },
                    "All"
                }
                button {
                    r#type: "button",
                    class: if config.card_kind == Some(CardKindFilter::Pokemon) { "{btn_base} {active}" } else { "{btn_base} {inactive}" },
                    onclick: {
                        let config = config.clone();
                        move |_| {
                            let mut c = config.clone();
                            c.card_kind = Some(CardKindFilter::Pokemon);
                            on_change.call(c);
                        }
                    },
                    "Pokémon"
                }
                button {
                    r#type: "button",
                    class: if config.card_kind == Some(CardKindFilter::Trainer) { "{btn_base} {active}" } else { "{btn_base} {inactive}" },
                    onclick: {
                        let config = config.clone();
                        move |_| {
                            let mut c = config.clone();
                            c.card_kind = Some(CardKindFilter::Trainer);
                            on_change.call(c);
                        }
                    },
                    "Trainer"
                }
            }
        }
    }
}

#[component]
pub fn StageFilter(
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let sel = config.stage.map(|id| id.to_string()).unwrap_or_default();
    rsx! {
        div { class: "flex items-center gap-1.5",
            label { class: "text-sm text-gray-600 dark:text-gray-400", "Stage" }
            select {
                class: "py-1 pl-1.5 pr-6 text-sm rounded-md border border-gray-300 dark:border-gray-600 \
                        bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                        focus:outline-none focus:ring-2 focus:ring-blue-500",
                value: "{sel}",
                onchange: move |evt| {
                    let mut c = config.clone();
                    c.stage = evt.value().parse::<usize>().ok();
                    on_change.call(c);
                },
                option { value: "", "Any" }
                for stage in Stage::ALL {
                    option { key: "{stage.id()}", value: "{stage.id()}", "{stage.name()}" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5: three-way property toggle (Ex, Mega, Foil, Obtainable)
// ---------------------------------------------------------------------------

/// Compact three-way selector: Any / only / exclude.
#[component]
pub fn ThreeWayFilter(
    label: &'static str,
    only_text: &'static str,
    exclude_text: &'static str,
    value: Option<bool>,
    on_change: EventHandler<Option<bool>>,
) -> Element {
    let val_str = match value {
        None => "",
        Some(true) => "only",
        Some(false) => "exclude",
    };

    rsx! {
        div { class: "flex items-center gap-1.5",
            span { class: "text-sm text-gray-600 dark:text-gray-400", "{label}" }
            select {
                class: "py-1 pl-1.5 pr-6 text-sm rounded-md border border-gray-300 dark:border-gray-600 \
                        bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                        focus:outline-none focus:ring-2 focus:ring-blue-500",
                value: "{val_str}",
                onchange: move |evt| {
                    let v = match evt.value().as_str() {
                        "only" => Some(true),
                        "exclude" => Some(false),
                        _ => None,
                    };
                    on_change.call(v);
                },
                option { value: "", "Any" }
                option { value: "only", "{only_text}" }
                option { value: "exclude", "{exclude_text}" }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6: owned-count threshold (Catalog) or goal + any-version (Analysis)
// ---------------------------------------------------------------------------

#[component]
pub fn CountFilter(
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let (op_str, n) = match config.owned_count {
        None => ("", 0u32),
        Some(CountThreshold::Equal(n)) => ("eq", n),
        Some(CountThreshold::LessThan(n)) => ("lt", n),
        Some(CountThreshold::AtLeast(n)) => ("gte", n),
    };
    // Pre-evaluate before any closures move config/on_change
    let has_count = config.owned_count.is_some();
    let config_for_select = config.clone();
    let on_change_for_select = on_change.clone();

    rsx! {
        div { class: "flex items-center gap-1.5",
            span { class: "text-sm text-gray-600 dark:text-gray-400", "Count" }
            select {
                class: "py-1 pl-1.5 pr-6 text-sm rounded-md border border-gray-300 dark:border-gray-600 \
                        bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                        focus:outline-none focus:ring-2 focus:ring-blue-500",
                value: "{op_str}",
                onchange: move |evt| {
                    let mut c = config_for_select.clone();
                    c.owned_count = match evt.value().as_str() {
                        "eq" => Some(CountThreshold::Equal(n)),
                        "lt" => Some(CountThreshold::LessThan(n)),
                        "gte" => Some(CountThreshold::AtLeast(n)),
                        _ => None,
                    };
                    on_change_for_select.call(c);
                },
                option { value: "", "Any count" }
                option { value: "eq", "= N" }
                option { value: "lt", "< N" }
                option { value: "gte", "≥ N" }
            }
            if has_count {
                input {
                    r#type: "text",
                    value: "{n}",
                    class: "w-14 px-2 py-1 text-sm text-center rounded-md border \
                            border-gray-300 dark:border-gray-600 \
                            bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                            focus:outline-none focus:ring-2 focus:ring-blue-500",
                    oninput: move |evt| {
                        if let Ok(val) = evt.value().trim().parse::<u32>() {
                            let mut c = config.clone();
                            c.owned_count = Some(
                                match op_str {
                                    "eq" => CountThreshold::Equal(val),
                                    "lt" => CountThreshold::LessThan(val),
                                    _ => CountThreshold::AtLeast(val),
                                },
                            );
                            on_change.call(c);
                        }
                    },
                }
            }
        }
    }
}

#[component]
pub fn GoalFilter(
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-1.5",
            label { class: "text-sm text-gray-600 dark:text-gray-400", "Goal" }
            input {
                r#type: "text",
                value: "{config.goal}",
                class: "w-14 px-2 py-1 text-sm text-center rounded-md border \
                        border-gray-300 dark:border-gray-600 \
                        bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                        focus:outline-none focus:ring-2 focus:ring-blue-500",
                oninput: move |evt| {
                    if let Ok(n) = evt.value().trim().parse::<u32>() {
                        let mut c = config.clone();
                        c.goal = n.max(1);
                        on_change.call(c);
                    }
                },
            }
        }
    }
}

#[component]
pub fn AnyVersionFilter(
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-2",
            Toggle {
                checked: config.any_version_owned,
                on_change: move |v: bool| {
                    let mut c = config.clone();
                    c.any_version_owned = v;
                    on_change.call(c);
                },
            }
            span { class: "text-sm text-gray-700 dark:text-gray-300", "Any version owned" }
        }
    }
}

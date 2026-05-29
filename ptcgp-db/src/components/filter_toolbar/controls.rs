use dioxus::prelude::*;
use ptcgp_db_core::save_data::{CardKindFilter, CountThreshold, FilterConfig};

use crate::components::toggle::Toggle;

use super::seg_btn_cls;

// ---------------------------------------------------------------------------
// Name / number text search
// ---------------------------------------------------------------------------

#[component]
pub fn NameFilter(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let value = config.name_query.clone().unwrap_or_default();
    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Name" }
            input {
                r#type: "text",
                placeholder: "Name or number…",
                value: "{value}",
                class: "px-2 py-1.5 text-sm rounded-md border border-gray-300 dark:border-gray-600 \
                        bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                        placeholder:text-gray-400 dark:placeholder:text-gray-500 \
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
}

// ---------------------------------------------------------------------------
// Kind — segmented button group (All / Pokémon / Trainer)
// ---------------------------------------------------------------------------

#[component]
pub fn KindFilter(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Kind" }
            div { class: "flex",
                KindBtn {
                    btn_label: "All",
                    active: config.card_kind.is_none(),
                    target: None,
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
                KindBtn {
                    btn_label: "Pokémon",
                    active: config.card_kind == Some(CardKindFilter::Pokemon),
                    target: Some(CardKindFilter::Pokemon),
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
                KindBtn {
                    btn_label: "Trainer",
                    active: config.card_kind == Some(CardKindFilter::Trainer),
                    target: Some(CardKindFilter::Trainer),
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
            }
        }
    }
}

#[component]
fn KindBtn(
    btn_label: &'static str,
    active: bool,
    target: Option<CardKindFilter>,
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
                c.card_kind = target;
                on_change.call(c);
            },
            "{btn_label}"
        }
    }
}

// ---------------------------------------------------------------------------
// TriStateFilter — segmented Any / <only> / <exclude> group for boolean dims
// ---------------------------------------------------------------------------

#[component]
pub fn TriStateFilter(
    filter_label: &'static str,
    only_text: &'static str,
    exclude_text: &'static str,
    value: Option<bool>,
    on_change: EventHandler<Option<bool>>,
) -> Element {
    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "{filter_label}" }
            div { class: "flex",
                TriBtn {
                    btn_label: "Any",
                    active: value.is_none(),
                    target: None,
                    on_change: on_change.clone(),
                }
                TriBtn {
                    btn_label: only_text,
                    active: value == Some(true),
                    target: Some(true),
                    on_change: on_change.clone(),
                }
                TriBtn {
                    btn_label: exclude_text,
                    active: value == Some(false),
                    target: Some(false),
                    on_change: on_change.clone(),
                }
            }
        }
    }
}

#[component]
fn TriBtn(
    btn_label: &'static str,
    active: bool,
    target: Option<bool>,
    on_change: EventHandler<Option<bool>>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            onclick: move |_| on_change.call(target),
            "{btn_label}"
        }
    }
}

// ---------------------------------------------------------------------------
// Count threshold (Catalog mode)
// ---------------------------------------------------------------------------

#[component]
pub fn CountFilter(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    let (op_str, n) = match config.owned_count {
        None => ("", 0u32),
        Some(CountThreshold::Equal(n)) => ("eq", n),
        Some(CountThreshold::LessThan(n)) => ("lt", n),
        Some(CountThreshold::AtLeast(n)) => ("gte", n),
    };
    let has_count = config.owned_count.is_some();
    let config_for_op = config.clone();
    let on_change_for_op = on_change.clone();

    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Count" }
            div { class: "flex items-center gap-2",
                div { class: "flex",
                    CountOpBtn {
                        btn_label: "Any",
                        active: !has_count,
                        op: "",
                        n,
                        config: config_for_op.clone(),
                        on_change: on_change_for_op.clone(),
                    }
                    CountOpBtn {
                        btn_label: "= N",
                        active: op_str == "eq",
                        op: "eq",
                        n,
                        config: config_for_op.clone(),
                        on_change: on_change_for_op.clone(),
                    }
                    CountOpBtn {
                        btn_label: "< N",
                        active: op_str == "lt",
                        op: "lt",
                        n,
                        config: config_for_op.clone(),
                        on_change: on_change_for_op.clone(),
                    }
                    CountOpBtn {
                        btn_label: "≥ N",
                        active: op_str == "gte",
                        op: "gte",
                        n,
                        config: config_for_op,
                        on_change: on_change_for_op,
                    }
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
}

#[component]
fn CountOpBtn(
    btn_label: &'static str,
    active: bool,
    op: &'static str,
    n: u32,
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
                c.owned_count = match op {
                    "eq" => Some(CountThreshold::Equal(n)),
                    "lt" => Some(CountThreshold::LessThan(n)),
                    "gte" => Some(CountThreshold::AtLeast(n)),
                    _ => None,
                };
                on_change.call(c);
            },
            "{btn_label}"
        }
    }
}

// ---------------------------------------------------------------------------
// Goal input and any-version toggle (Analysis mode)
// ---------------------------------------------------------------------------

#[component]
pub fn GoalFilter(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Goal" }
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
pub fn AnyVersionFilter(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    rsx! {
        div { class: "flex flex-col gap-1",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Any Version" }
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
}

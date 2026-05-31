use dioxus::prelude::*;
use ptcgp_db_core::save_data::{CardKindFilter, CountThreshold, FilterConfig};

use crate::components::toggle::Toggle;

use super::seg_btn_cls;

// ---------------------------------------------------------------------------
// Name / number text search
// ---------------------------------------------------------------------------

#[component]
pub fn NameFilter(config: Signal<FilterConfig>) -> Element {
    let value = config.read().name_query.clone().unwrap_or_default();
    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Name" }
            input {
                r#type: "text",
                placeholder: "Name…",
                value: "{value}",
                class: "px-2 py-1.5 text-sm rounded-md border border-gray-300 dark:border-gray-600 \
                        bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                        placeholder:text-gray-400 dark:placeholder:text-gray-500 \
                        focus:outline-none focus:ring-2 focus:ring-blue-500 w-44",
                oninput: move |evt| {
                    let v = evt.value();
                    config.write().name_query = if v.is_empty() { None } else { Some(v) };
                },
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Kind — segmented button group (All / Pokémon / Trainer)
// ---------------------------------------------------------------------------

#[component]
pub fn KindFilter(config: Signal<FilterConfig>) -> Element {
    let card_kind = config.read().card_kind;
    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Kind" }
            div { class: "flex",
                KindBtn {
                    btn_label: "All",
                    active: card_kind.is_none(),
                    target: None,
                    config,
                }
                KindBtn {
                    btn_label: "Pokémon",
                    active: card_kind == Some(CardKindFilter::Pokemon),
                    target: Some(CardKindFilter::Pokemon),
                    config,
                }
                KindBtn {
                    btn_label: "Trainer",
                    active: card_kind == Some(CardKindFilter::Trainer),
                    target: Some(CardKindFilter::Trainer),
                    config,
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
    config: Signal<FilterConfig>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            onclick: move |_| config.write().card_kind = target,
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
                    on_change,
                }
                TriBtn {
                    btn_label: only_text,
                    active: value == Some(true),
                    target: Some(true),
                    on_change,
                }
                TriBtn {
                    btn_label: exclude_text,
                    active: value == Some(false),
                    target: Some(false),
                    on_change,
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
pub fn CountFilter(config: Signal<FilterConfig>) -> Element {
    let owned_count = config.read().owned_count;
    let n = match owned_count {
        None => 0u32,
        Some(
            CountThreshold::Equal(n) | CountThreshold::LessThan(n) | CountThreshold::AtLeast(n),
        ) => n,
    };

    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Count" }
            div { class: "flex items-center gap-2",
                div { class: "flex",
                    CountOpBtn {
                        btn_label: "Any",
                        active: owned_count.is_none(),
                        threshold: None,
                        config,
                    }
                    CountOpBtn {
                        btn_label: "= N",
                        active: matches!(owned_count, Some(CountThreshold::Equal(_))),
                        threshold: Some(CountThreshold::Equal(n)),
                        config,
                    }
                    CountOpBtn {
                        btn_label: "< N",
                        active: matches!(owned_count, Some(CountThreshold::LessThan(_))),
                        threshold: Some(CountThreshold::LessThan(n)),
                        config,
                    }
                    CountOpBtn {
                        btn_label: "≥ N",
                        active: matches!(owned_count, Some(CountThreshold::AtLeast(_))),
                        threshold: Some(CountThreshold::AtLeast(n)),
                        config,
                    }
                }
                if let Some(oc) = owned_count {
                    input {
                        r#type: "text",
                        value: "{n}",
                        class: "w-14 px-2 py-1 text-sm text-center rounded-md border \
                                border-gray-300 dark:border-gray-600 \
                                bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                                focus:outline-none focus:ring-2 focus:ring-blue-500",
                        oninput: move |evt| {
                            if let Ok(val) = evt.value().trim().parse::<u32>() {
                                config.write().owned_count = Some(
                                    match oc {
                                        CountThreshold::Equal(_) => CountThreshold::Equal(val),
                                        CountThreshold::LessThan(_) => CountThreshold::LessThan(val),
                                        CountThreshold::AtLeast(_) => CountThreshold::AtLeast(val),
                                    },
                                );
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
    threshold: Option<CountThreshold>,
    config: Signal<FilterConfig>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            onclick: move |_| config.write().owned_count = threshold,
            "{btn_label}"
        }
    }
}

// ---------------------------------------------------------------------------
// Goal input and any-version toggle (Analysis mode)
// ---------------------------------------------------------------------------

#[component]
pub fn GoalFilter(config: Signal<FilterConfig>) -> Element {
    let goal = config.read().goal;
    rsx! {
        div { class: "flex flex-col gap-0.5",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Goal" }
            input {
                r#type: "text",
                value: "{goal}",
                class: "w-14 px-2 py-1 text-sm text-center rounded-md border \
                        border-gray-300 dark:border-gray-600 \
                        bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 \
                        focus:outline-none focus:ring-2 focus:ring-blue-500",
                oninput: move |evt| {
                    if let Ok(n) = evt.value().trim().parse::<u32>() {
                        config.write().goal = n.max(1);
                    }
                },
            }
        }
    }
}

#[component]
pub fn AnyVersionFilter(config: Signal<FilterConfig>) -> Element {
    let any_version_owned = config.read().any_version_owned;
    rsx! {
        div { class: "flex flex-col gap-1",
            span { class: "text-xs font-medium text-gray-500 dark:text-gray-400", "Any Version" }
            div { class: "flex items-center gap-2",
                Toggle {
                    checked: any_version_owned,
                    on_change: move |v: bool| config.write().any_version_owned = v,
                }
                span { class: "text-sm text-gray-700 dark:text-gray-300", "Any version owned" }
            }
        }
    }
}

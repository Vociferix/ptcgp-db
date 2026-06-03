use dioxus::prelude::*;
use ptcgp_db_core::save_data::{CardKindFilter, CountThreshold, FilterConfig};

use crate::components::icons::{Minus, Plus};
use crate::components::toggle::Toggle;

use super::seg_btn_cls;

// ---------------------------------------------------------------------------
// Name / number text search
// ---------------------------------------------------------------------------

#[component]
pub fn NameFilter(config: Signal<FilterConfig>) -> Element {
    let value = config.read().name_query.clone().unwrap_or_default();
    rsx! {
        input {
            r#type: "text",
            title: "Name",
            placeholder: "Name…",
            value: "{value}",
            class: "shrink-0 px-2 py-1.5 text-sm rounded-md \
                    border border-gray-300 dark:border-gray-600 \
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

// ---------------------------------------------------------------------------
// Kind — segmented button group (All / Pokémon / Trainer)
// ---------------------------------------------------------------------------

#[component]
pub fn KindFilter(config: Signal<FilterConfig>, #[props(default = true)] labeled: bool) -> Element {
    let card_kind = config.read().card_kind;
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
            span { class: "{label_cls}", "Kind" }
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
// Goal input and any-version toggle (Trade / Summary modes)
// ---------------------------------------------------------------------------

#[component]
pub fn GoalFilter(config: Signal<FilterConfig>) -> Element {
    let goal = config.read().goal.max(1);
    rsx! {
        div { class: "shrink-0 flex items-center border border-gray-300 dark:border-gray-600 \
                      rounded-md overflow-hidden bg-white dark:bg-gray-800",
            // Inline label prefix — gives context on mobile where tooltips don't appear
            span { class: "px-2 h-8 flex items-center text-xs font-medium select-none \
                           text-gray-500 dark:text-gray-400 \
                           bg-gray-50 dark:bg-gray-700 \
                           border-r border-gray-300 dark:border-gray-600",
                "Goal"
            }
            button {
                r#type: "button",
                disabled: goal <= 1,
                class: "flex items-center justify-center w-7 h-8 shrink-0 \
                        border-r border-gray-300 dark:border-gray-600 \
                        hover:bg-gray-100 dark:hover:bg-gray-700 \
                        disabled:opacity-40 disabled:cursor-not-allowed",
                onclick: move |_| {
                    let g = config.read().goal;
                    if g > 1 {
                        config.write().goal = g - 1;
                    }
                },
                Minus { class: "w-3.5 h-3.5 text-gray-600 dark:text-gray-400" }
            }
            span { class: "w-8 text-center text-sm select-none text-gray-900 dark:text-gray-100",
                "{goal}"
            }
            button {
                r#type: "button",
                class: "flex items-center justify-center w-7 h-8 shrink-0 \
                        border-l border-gray-300 dark:border-gray-600 \
                        hover:bg-gray-100 dark:hover:bg-gray-700",
                onclick: move |_| {
                    let g = config.read().goal;
                    config.write().goal = g.saturating_add(1);
                },
                Plus { class: "w-3.5 h-3.5 text-gray-600 dark:text-gray-400" }
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

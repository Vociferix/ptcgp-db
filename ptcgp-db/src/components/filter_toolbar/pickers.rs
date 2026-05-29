// Alias to avoid shadowing dioxus::prelude::Element (VNode return type).
use ptcgp_db_data::Element as PtcgpElement;
use ptcgp_db_data::RarityClass;

use dioxus::prelude::*;
use ptcgp_db_core::save_data::FilterConfig;

use super::seg_btn_cls;

// ---------------------------------------------------------------------------
// Rarity — segmented icon button group (multi-select)
// ---------------------------------------------------------------------------

#[component]
pub fn RarityGroup(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    rsx! {
        div { class: "flex flex-col gap-1",
            div { class: "flex items-center gap-2",
                span { class: "text-xs font-medium text-gray-500 dark:text-gray-400",
                    "Rarity"
                }
                if !config.rarities.is_empty() {
                    button {
                        r#type: "button",
                        class: "text-xs text-gray-400 dark:text-gray-500 \
                                hover:text-gray-600 dark:hover:text-gray-300",
                        onclick: {
                            let config = config.clone();
                            move |_| {
                                let mut c = config.clone();
                                c.rarities.clear();
                                on_change.call(c);
                            }
                        },
                        "Clear"
                    }
                }
            }
            div { class: "flex flex-wrap gap-y-px",
                for rarity in RarityClass::ALL {
                    RarityBtn {
                        key: "{rarity.id()}",
                        rarity,
                        active: config.rarities.contains(&rarity.id()),
                        config: config.clone(),
                        on_change,
                    }
                }
            }
        }
    }
}

#[component]
fn RarityBtn(
    rarity: &'static RarityClass,
    active: bool,
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            title: "{rarity.group().name()} {rarity.count()}",
            class: "{cls} !px-1.5",
            onclick: move |_| {
                let mut c = config.clone();
                let id = rarity.id();
                if active {
                    c.rarities.retain(|&x| x != id);
                } else if !c.rarities.contains(&id) {
                    c.rarities.push(id);
                }
                on_change.call(c);
            },
            img {
                src: "{rarity.icon()}",
                alt: "{rarity.group().name()} {rarity.count()}",
                class: "h-5 w-auto",
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Element — segmented icon button group (multi-select)
// ---------------------------------------------------------------------------

#[component]
pub fn ElementGroup(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    rsx! {
        div { class: "flex flex-col gap-1",
            div { class: "flex items-center gap-2",
                span { class: "text-xs font-medium text-gray-500 dark:text-gray-400",
                    "Element"
                }
                if !config.elements.is_empty() {
                    button {
                        r#type: "button",
                        class: "text-xs text-gray-400 dark:text-gray-500 \
                                hover:text-gray-600 dark:hover:text-gray-300",
                        onclick: {
                            let config = config.clone();
                            move |_| {
                                let mut c = config.clone();
                                c.elements.clear();
                                on_change.call(c);
                            }
                        },
                        "Clear"
                    }
                }
            }
            div { class: "flex flex-wrap gap-y-px",
                for element in PtcgpElement::ALL {
                    ElementBtn {
                        key: "{element.id()}",
                        element,
                        active: config.elements.contains(&element.id()),
                        config: config.clone(),
                        on_change,
                    }
                }
            }
        }
    }
}

#[component]
fn ElementBtn(
    element: &'static PtcgpElement,
    active: bool,
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            title: "{element.name()}",
            class: "{cls} !px-1.5",
            onclick: move |_| {
                let mut c = config.clone();
                let id = element.id();
                if active {
                    c.elements.retain(|&x| x != id);
                } else if !c.elements.contains(&id) {
                    c.elements.push(id);
                }
                on_change.call(c);
            },
            img {
                src: "{element.icon()}",
                alt: "{element.name()}",
                class: "h-5 w-5",
            }
        }
    }
}

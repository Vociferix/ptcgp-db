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
pub fn RarityGroup(config: Signal<FilterConfig>) -> Element {
    let cfg = config.read();
    let rarities = cfg.rarities.as_slice();
    rsx! {
        div { class: "flex flex-col gap-1",
            div { class: "flex items-center gap-2",
                span { class: "text-xs font-medium text-gray-500 dark:text-gray-400",
                    "Rarity"
                }
                if !rarities.is_empty() {
                    button {
                        r#type: "button",
                        class: "text-xs text-gray-400 dark:text-gray-500 \
                                hover:text-gray-600 dark:hover:text-gray-300",
                        onclick: move |_| config.write().rarities.clear(),
                        "Clear"
                    }
                }
            }
            div { class: "flex flex-wrap gap-y-px",
                for rarity in RarityClass::ALL {
                    RarityBtn {
                        key: "{rarity.id()}",
                        rarity,
                        active: rarities.contains(&rarity.id()),
                        config,
                    }
                }
            }
        }
    }
}

#[component]
fn RarityBtn(rarity: &'static RarityClass, active: bool, config: Signal<FilterConfig>) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            title: "{rarity.group().name()} {rarity.count()}",
            class: "{cls} !px-1.5",
            onclick: move |_| {
                let id = rarity.id();
                let mut cfg = config.write();
                if active {
                    cfg.rarities.retain(|&x| x != id);
                } else if !cfg.rarities.contains(&id) {
                    cfg.rarities.push(id);
                }
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
pub fn ElementGroup(config: Signal<FilterConfig>) -> Element {
    let cfg = config.read();
    let elements = cfg.elements.as_slice();
    rsx! {
        div { class: "flex flex-col gap-1",
            div { class: "flex items-center gap-2",
                span { class: "text-xs font-medium text-gray-500 dark:text-gray-400",
                    "Element"
                }
                if !elements.is_empty() {
                    button {
                        r#type: "button",
                        class: "text-xs text-gray-400 dark:text-gray-500 \
                                hover:text-gray-600 dark:hover:text-gray-300",
                        onclick: move |_| config.write().elements.clear(),
                        "Clear"
                    }
                }
            }
            div { class: "flex flex-wrap gap-y-px",
                for element in PtcgpElement::ALL {
                    ElementBtn {
                        key: "{element.id()}",
                        element,
                        active: elements.contains(&element.id()),
                        config,
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
    config: Signal<FilterConfig>,
) -> Element {
    let cls = seg_btn_cls(active);
    rsx! {
        button {
            r#type: "button",
            title: "{element.name()}",
            class: "{cls} !px-1.5",
            onclick: move |_| {
                let id = element.id();
                let mut cfg = config.write();
                if active {
                    cfg.elements.retain(|&x| x != id);
                } else if !cfg.elements.contains(&id) {
                    cfg.elements.push(id);
                }
            },
            img {
                src: "{element.icon()}",
                alt: "{element.name()}",
                class: "h-5 w-5",
            }
        }
    }
}

// Alias to avoid shadowing dioxus::prelude::Element (the VNode return type).
use ptcgp_db_data::Element as PtcgpElement;
use ptcgp_db_data::RarityClass;

use dioxus::prelude::*;
use ptcgp_db_core::save_data::FilterConfig;

// ---------------------------------------------------------------------------
// Rarity class picker
// ---------------------------------------------------------------------------

#[component]
pub fn RarityPicker(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center gap-1.5",
            span { class: "text-sm text-gray-600 dark:text-gray-400", "Rarity" }
            for rarity in RarityClass::ALL {
                RarityChip {
                    key: "{rarity.id()}",
                    rarity,
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
            }
            if !config.rarities.is_empty() {
                button {
                    r#type: "button",
                    class: "text-xs text-gray-500 dark:text-gray-400 \
                            hover:text-gray-700 dark:hover:text-gray-200 underline",
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
    }
}

#[component]
fn RarityChip(
    rarity: &'static RarityClass,
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let selected = config.rarities.contains(&rarity.id());
    let ring = if selected {
        "ring-2 ring-blue-500 ring-offset-1 dark:ring-offset-gray-900"
    } else {
        "ring-1 ring-gray-300 dark:ring-gray-600"
    };

    rsx! {
        button {
            r#type: "button",
            title: "{rarity.group().name()} {rarity.count()}",
            class: "rounded p-0.5 hover:opacity-80 transition-opacity {ring}",
            onclick: move |_| {
                let mut c = config.clone();
                let id = rarity.id();
                if selected {
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
// Element picker
// ---------------------------------------------------------------------------

#[component]
pub fn ElementPicker(config: FilterConfig, on_change: EventHandler<FilterConfig>) -> Element {
    rsx! {
        div { class: "flex flex-wrap items-center gap-1.5",
            span { class: "text-sm text-gray-600 dark:text-gray-400", "Element" }
            for element in PtcgpElement::ALL {
                ElementChip {
                    key: "{element.id()}",
                    element,
                    config: config.clone(),
                    on_change: on_change.clone(),
                }
            }
            if !config.elements.is_empty() {
                button {
                    r#type: "button",
                    class: "text-xs text-gray-500 dark:text-gray-400 \
                            hover:text-gray-700 dark:hover:text-gray-200 underline",
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
    }
}

#[component]
fn ElementChip(
    element: &'static PtcgpElement,
    config: FilterConfig,
    on_change: EventHandler<FilterConfig>,
) -> Element {
    let selected = config.elements.contains(&element.id());
    let ring = if selected {
        "ring-2 ring-blue-500 ring-offset-1 dark:ring-offset-gray-900"
    } else {
        "ring-1 ring-gray-300 dark:ring-gray-600"
    };

    rsx! {
        button {
            r#type: "button",
            title: "{element.name()}",
            class: "rounded p-0.5 hover:opacity-80 transition-opacity {ring}",
            onclick: move |_| {
                let mut c = config.clone();
                let id = element.id();
                if selected {
                    c.elements.retain(|&x| x != id);
                } else if !c.elements.contains(&id) {
                    c.elements.push(id);
                }
                on_change.call(c);
            },
            img {
                src: "{element.icon()}",
                alt: "{element.name()}",
                class: "h-6 w-6",
            }
        }
    }
}

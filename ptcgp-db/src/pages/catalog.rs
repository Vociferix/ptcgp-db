use dioxus::prelude::*;
use ptcgp_db_core::save_data::FilterConfig;

use crate::components::{FilterMode, FilterToolbar};

// TODO T20: replace this demo with the real Card Catalog implementation.
#[component]
pub fn CatalogPage() -> Element {
    let mut catalog_config = use_signal(FilterConfig::default);
    let mut analysis_config = use_signal(|| FilterConfig {
        obtainable: Some(true),
        ..FilterConfig::default()
    });

    rsx! {
        div { class: "p-4 space-y-10",
            section { class: "space-y-2",
                h2 { class: "text-base font-semibold text-gray-800 dark:text-gray-200",
                    "FilterMode::Catalog"
                }
                FilterToolbar {
                    config: catalog_config.read().clone(),
                    on_change: move |c| catalog_config.set(c),
                    mode: FilterMode::Catalog,
                }
            }

            section { class: "space-y-2",
                h2 { class: "text-base font-semibold text-gray-800 dark:text-gray-200",
                    "FilterMode::Analysis"
                }
                FilterToolbar {
                    config: analysis_config.read().clone(),
                    on_change: move |c| analysis_config.set(c),
                    mode: FilterMode::Analysis,
                }
            }
        }
    }
}

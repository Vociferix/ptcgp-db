//! Element icon group for a Pokémon card, styled after the in-game card layout.

use dioxus::prelude::*;
use ptcgp_db_data::PokemonCard;

use crate::components::CachedImage;

/// Renders a Pokémon's element icons in a row. Dual-element cards mirror the printed card:
/// the icons overlap and the primary (0th) element sits on top.
///
/// `class` sizes each individual icon (e.g. `"h-5 w-5"`). The overlap is a fixed 5px, a
/// quarter of the 20px size used everywhere today; revisit it if a different size is added.
#[component]
pub fn ElementIcons(pokemon: &'static PokemonCard, class: String) -> Element {
    rsx! {
        div { class: "isolate flex items-center",
            for (i, element) in pokemon.elements().iter().enumerate() {
                CachedImage {
                    key: "{i}",
                    src: "{element.icon()}",
                    alt: "{element.name()}",
                    class: if i == 0 { format!("relative z-10 object-contain {class}") } else { format!("-ml-[5px] object-contain {class}") },
                }
            }
        }
    }
}

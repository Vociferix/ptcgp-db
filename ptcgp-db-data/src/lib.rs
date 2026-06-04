//! All PTCGP card, pack, set, and pull rate data as compile-time constants.
//!
//! Data is generated from `ptcgp-data` JSON at build time — no runtime JSON parsing or database
//! access is required. All sequences are pre-sorted: sequences with a canonical display order
//! (e.g., [`CardVersion::ALL`]) use that order; others are sorted for binary search (numerically
//! by ID for ID-keyed data, alphabetically for string-keyed data).

#[cfg(test)]
mod tests;

mod ability;
mod attack;
mod base_pokemon;
mod card;
mod card_source;
mod card_version;
mod element;
mod pack;
mod pack_slot;
mod pack_variant;
mod prob;
mod rarity;
mod rarity_class;
mod rarity_group;
mod series;
mod set;
mod stage;
mod trainer_kind;

pub mod id_slice;
pub mod str_table;

pub use ability::Ability;
pub use attack::Attack;
pub use base_pokemon::BasePokemon;
pub use card::{Card, CardKind, PokemonCard, TrainerCard};
pub use card_source::CardSource;
pub use card_version::CardVersion;
pub use element::Element;
pub use pack::Pack;
pub use pack_slot::{CardVersionPullRate, PackSlot, RarityPullRate};
pub use pack_variant::PackVariant;
pub use prob::Prob;
pub use rarity::Rarity;
pub use rarity_class::RarityClass;
pub use rarity_group::RarityGroup;
pub use series::Series;
pub use set::Set;
pub use stage::Stage;
pub use trainer_kind::TrainerKind;

mod data {
    use crate::*;

    #[cfg(feature = "images")]
    use manganis::{AssetOptions, ImageFormat, asset};

    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

const unsafe fn get_unchecked<T>(slice: &[T], index: usize) -> &T {
    unsafe { &*slice.as_ptr().add(index) }
}

const unsafe fn slice_unchecked<T>(slice: &[T], start: usize, end: usize) -> &[T] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().add(start), end - start) }
}

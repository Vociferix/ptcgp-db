//! Pokédex entries.

use crate::str_table::{StrEntry, StrTable};

use std::num::NonZeroUsize;

/// A Pokédex entry shared across all [`PokemonCard`]s of the same species.
///
/// [`PokemonCard`]: crate::PokemonCard
pub struct BasePokemon {
    pub(crate) id: usize,
    pub(crate) natdex_num: NonZeroUsize,
    pub(crate) name_id: usize,
}

impl BasePokemon {
    /// All Pokédex entries, sorted by national Pokédex number.
    pub const ALL: &[Self] = crate::data::BASE_POKEMON;

    /// Species name strings (e.g., `"Bulbasaur"`, `"Pikachu"`).
    pub const NAMES: &StrTable = crate::data::BASE_POKEMON_NAMES;

    /// Returns the Pokédex entry with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the Pokédex entry with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`BasePokemon::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Species name (e.g., `"Bulbasaur"`, `"Pikachu"`).
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// National Pokédex number.
    pub const fn natdex_number(&self) -> NonZeroUsize {
        self.natdex_num
    }
}

impl std::fmt::Debug for BasePokemon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasePokemon")
            .field("id", &self.id)
            .field("name", &self.name())
            .field("natdex_number", &self.natdex_num)
            .finish()
    }
}

impl PartialEq for BasePokemon {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for BasePokemon {}

impl PartialOrd for BasePokemon {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for BasePokemon {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for BasePokemon {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for BasePokemon {
    const INDEXED: &'static [Self] = Self::ALL;
}

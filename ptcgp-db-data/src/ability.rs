//! Pokémon abilities.

use crate::str_table::{StrEntry, StrTable};

/// A Pokémon ability. Only some Pokémon cards have an ability; check [`PokemonCard::ability`].
///
/// [`PokemonCard::ability`]: crate::PokemonCard::ability
pub struct Ability {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
    pub(crate) effect_id: usize,
}

impl Ability {
    /// All abilities, sorted by ID.
    pub const ALL: &[Self] = crate::data::ABILITIES;

    /// Ability name strings.
    pub const NAMES: &StrTable = crate::data::ABILITY_NAMES;

    /// Ability effect text strings.
    pub const EFFECTS: &StrTable = crate::data::ABILITY_EFFECTS;

    /// Returns the ability with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the ability with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`Ability::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Ability name.
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// Effect text. May contain element placeholders (e.g., `[R]` for Fire) intended to be
    /// replaced with [`Element`] symbol images in the UI.
    ///
    /// [`Element`]: crate::Element
    pub const fn effect(&self) -> StrEntry {
        unsafe { Self::EFFECTS.get_entry_unchecked(self.effect_id) }
    }
}

impl std::fmt::Debug for Ability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ability")
            .field("id", &self.id)
            .field("name", &self.name())
            .field("effect", &self.effect())
            .finish()
    }
}

impl PartialEq for Ability {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Ability {}

impl PartialOrd for Ability {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Ability {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Ability {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Ability {
    const INDEXED: &'static [Self] = Self::ALL;
}

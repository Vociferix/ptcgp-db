//! Specific rarity tiers and their in-game costs.

use crate::{
    RarityClass, RarityGroup,
    str_table::{StrEntry, StrTable},
};

/// A specific rarity tier (e.g., Common, Art Rare, Immersive Rare).
///
/// Each rarity belongs to a [`RarityGroup`] (symbol shape) and a [`RarityClass`] (group +
/// symbol count). The UI represents rarity via [`RarityClass`] icons or symbols — rarity codes
/// and names are only shown as supplemental detail in card detail views.
pub struct Rarity {
    pub(crate) id: usize,
    pub(crate) code_id: usize,
    pub(crate) name_id: usize,
    pub(crate) class_id: usize,
    pub(crate) group_id: usize,
    pub(crate) craft_cost: u32,
    pub(crate) dupe_dust: u32,
}

impl Rarity {
    /// All rarity tiers, sorted by ID.
    pub const ALL: &[Self] = crate::data::RARITIES;

    /// Short rarity code strings (e.g., `"C"`, `"AR"`, `"IM"`, `"UR"`).
    pub const CODES: &StrTable = crate::data::RARITY_CODES;

    /// Human-readable rarity name strings (e.g., `"Common"`, `"Art Rare"`, `"Immersive Rare"`).
    pub const NAMES: &StrTable = crate::data::RARITY_NAMES;

    /// Returns the rarity with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the rarity with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`Rarity::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Short internal code (e.g., `"C"`, `"AR"`, `"UR"`). Not shown to users in the UI.
    pub const fn code(&self) -> StrEntry {
        unsafe { Self::CODES.get_entry_unchecked(self.code_id) }
    }

    /// Human-readable name (e.g., `"Common"`, `"Super Rare"`). Only shown as supplemental
    /// detail in card detail views, not in list rows.
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// The (group, symbol count) class this rarity belongs to. Use this for displaying
    /// rarity icons and symbols.
    pub const fn class(&self) -> &'static RarityClass {
        unsafe { RarityClass::from_id_unchecked(self.class_id) }
    }

    /// Symbol shape group (Diamond, Star, Shiny, or Crown).
    pub const fn group(&self) -> &'static RarityGroup {
        unsafe { RarityGroup::from_id_unchecked(self.group_id) }
    }

    /// In-game dust cost to craft a card of this rarity.
    pub const fn craft_cost(&self) -> u32 {
        self.craft_cost
    }

    /// In-game dust earned when receiving a duplicate card of this rarity.
    pub const fn dupe_dust(&self) -> u32 {
        self.dupe_dust
    }
}

impl std::fmt::Debug for Rarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rarity")
            .field("id", &self.id)
            .field("code", &self.code())
            .field("name", &self.name())
            .field("class", self.class())
            .field("craft_cost", &self.craft_cost)
            .field("dupe_dust", &self.dupe_dust)
            .finish()
    }
}

impl PartialEq for Rarity {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Rarity {}

impl PartialOrd for Rarity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Rarity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Rarity {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Rarity {
    const INDEXED: &[Self] = Self::ALL;
}

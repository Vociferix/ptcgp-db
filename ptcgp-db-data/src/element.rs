//! Pokémon energy types.

use crate::str_table::{StrEntry, StrTable};

/// A Pokémon energy type (Grass, Fire, Water, Lightning, Fighting, Psychic, Darkness, Metal,
/// Dragon, or Colorless).
pub struct Element {
    pub(crate) id: usize,
    pub(crate) code: Option<char>,
    pub(crate) name_id: usize,
    pub(crate) icon: &'static str,
    pub(crate) symbol: &'static str,
}

impl Element {
    /// All elements in canonical display order (Grass, Fire, Water, Lightning, Fighting,
    /// Psychic, Darkness, Metal, Dragon, Colorless).
    pub const ALL: &[Self] = crate::data::ELEMENTS;

    /// Element name strings (e.g., `"Grass"`, `"Fire"`).
    pub const NAMES: &StrTable = crate::data::ELEMENT_NAMES;

    /// Icon URL to display for an attack with zero energy cost, in place of any element icon.
    pub const NO_COST: &'static str =
        "https://cdn.jsdelivr.net/gh/Vociferix/ptcgp-images@v0.8.1/elements/icons/no_cost.png";

    /// Returns the element with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the element with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`Element::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Single-letter code used in card effect text placeholders (e.g., `'R'` for Fire,
    /// `'G'` for Grass). `None` for the Dragon element only — Dragon is not a real energy
    /// type in PTCGP; Dragon-type Pokémon use a mix of other energy types for their attack
    /// costs and Dragon never appears as an energy in effect text.
    pub const fn code(&self) -> Option<char> {
        self.code
    }

    /// Display name (e.g., `"Grass"`, `"Fire"`, `"Colorless"`).
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// Element icon URL, suitable for non-text UI contexts (e.g., filter chips, attack cost display).
    pub const fn icon(&self) -> &'static str {
        self.icon
    }

    /// Element symbol image URL (text-height), suitable for replacing letter placeholders inline
    /// in card effect text.
    pub const fn symbol(&self) -> &'static str {
        self.symbol
    }
}

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("id", &self.id)
            .field("code", &self.code)
            .field("name", &self.name())
            .finish()
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Element {}

impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Element {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Element {
    const INDEXED: &'static [Self] = Self::ALL;
}

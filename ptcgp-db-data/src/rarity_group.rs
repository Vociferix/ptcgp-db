//! Rarity groups: symbol shape categories.

use crate::str_table::{StrEntry, StrTable};

/// Symbol shape category for a rarity (Diamond, Star, Shiny, or Crown).
///
/// A rarity group identifies the shape of the symbols printed on a card. Together with the
/// symbol count it forms a [`RarityClass`], which is the granularity at which rarity images
/// are defined.
///
/// [`RarityClass`]: crate::RarityClass
pub struct RarityGroup {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
    pub(crate) icon: &'static str,
    pub(crate) symbol: &'static str,
}

impl RarityGroup {
    /// All rarity groups, sorted by ID.
    pub const ALL: &[Self] = crate::data::RARITY_GROUPS;

    /// Rarity group name strings (e.g., `"Diamond"`, `"Star"`, `"Shiny"`, `"Crown"`).
    pub const NAMES: &StrTable = crate::data::RARITY_GROUP_NAMES;

    /// Returns the rarity group with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the rarity group with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`RarityGroup::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Display name (e.g., `"Diamond"`, `"Star"`, `"Shiny"`, `"Crown"`).
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// Rarity group icon URL, suitable for non-text UI contexts.
    pub const fn icon(&self) -> &'static str {
        self.icon
    }

    /// Rarity group symbol image URL (text-height, suitable for inline use in card effect text).
    pub const fn symbol(&self) -> &'static str {
        self.symbol
    }
}

impl std::fmt::Debug for RarityGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RarityGroup")
            .field("id", &self.id)
            .field("name", &self.name())
            .finish()
    }
}

impl PartialEq for RarityGroup {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for RarityGroup {}

impl PartialOrd for RarityGroup {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for RarityGroup {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for RarityGroup {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for RarityGroup {
    const INDEXED: &'static [Self] = Self::ALL;
}

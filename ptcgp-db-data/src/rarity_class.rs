//! Rarity classes: (group, symbol count) pairs at which icons and symbols are defined.

use crate::RarityGroup;

#[cfg(feature = "images")]
use manganis::Asset;

/// The (group, symbol count) pair identifying a rarity class (e.g., Diamond-2, Star-1).
///
/// This is the granularity at which rarity icon and symbol images exist in `ptcgp-images`.
/// The UI always represents rarity via the rarity class icon or symbol — not by rarity code or
/// group name. Star-2 is the only class that contains more than one specific [`Rarity`]
/// (SR and SAR).
///
/// [`Rarity`]: crate::Rarity
pub struct RarityClass {
    pub(crate) id: usize,
    pub(crate) group_id: usize,
    pub(crate) count: usize,
    #[cfg(feature = "images")]
    pub(crate) icon: Asset,
    #[cfg(feature = "images")]
    pub(crate) symbol: Asset,
}

impl RarityClass {
    /// All rarity classes, sorted by ID.
    pub const ALL: &[Self] = crate::data::RARITY_CLASSES;

    /// Returns the rarity class with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the rarity class with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`RarityClass::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Symbol shape group (Diamond, Star, Shiny, or Crown).
    pub const fn group(&self) -> &'static RarityGroup {
        unsafe { RarityGroup::from_id_unchecked(self.group_id) }
    }

    /// Number of symbols for this class (e.g., `2` for Diamond-2 or Star-2).
    pub const fn count(&self) -> usize {
        self.count
    }

    /// Rarity class icon, suitable for non-text UI contexts such as filter chips.
    #[cfg(feature = "images")]
    pub const fn icon(&self) -> Asset {
        self.icon
    }

    /// Rarity class symbol image (text-height), suitable for inline use within card effect text.
    #[cfg(feature = "images")]
    pub const fn symbol(&self) -> Asset {
        self.symbol
    }
}

impl std::fmt::Debug for RarityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RarityClass")
            .field("id", &self.id)
            .field("group", self.group())
            .field("count", &self.count)
            .finish()
    }
}

impl PartialEq for RarityClass {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for RarityClass {}

impl PartialOrd for RarityClass {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for RarityClass {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for RarityClass {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for RarityClass {
    const INDEXED: &'static [Self] = Self::ALL;
}

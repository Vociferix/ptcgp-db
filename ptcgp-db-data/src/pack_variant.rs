//! Pack variants (normal, rare, plus1, themed) and their pull rates.

use crate::{
    Pack, PackSlot, Prob, Series, Set,
    str_table::{StrEntry, StrTable},
};

use std::ops::Range;

/// One variant of a pack opening (e.g., normal, rare, plus1, themed).
///
/// Each pack opening selects exactly one variant. Variants for a pack are mutually exclusive
/// and their [`pull_rate`]s sum to 1. Not all packs have all four variant types.
///
/// [`pull_rate`]: PackVariant::pull_rate
pub struct PackVariant {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
    pub(crate) series_id: usize,
    pub(crate) set_id: usize,
    pub(crate) pack_id: usize,
    pub(crate) pull_rate: Prob,
    pub(crate) slot_ids: Range<usize>,
}

impl PackVariant {
    /// All pack variants, sorted by ID.
    pub const ALL: &[Self] = crate::data::PACK_VARIANTS;

    /// Display name strings for each variant type (e.g., `"Regular Pack"`, `"Rare Pack"`,
    /// `"Regular Pack +1"`, `"Themed Rare Pack"`).
    pub const NAMES: &StrTable = crate::data::PACK_VARIANT_NAMES;

    /// Returns the variant with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the variant with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`PackVariant::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Display name (e.g., `"Normal"`, `"Rare"`, `"Plus1"`, `"Themed"`).
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// Series this variant belongs to.
    pub const fn series(&self) -> &'static Series {
        unsafe { Series::from_id_unchecked(self.series_id) }
    }

    /// Set this variant belongs to.
    pub const fn set(&self) -> &'static Set {
        unsafe { Set::from_id_unchecked(self.set_id) }
    }

    /// Pack this variant belongs to.
    pub const fn pack(&self) -> &'static Pack {
        unsafe { Pack::from_id_unchecked(self.pack_id) }
    }

    /// Probability that this variant is selected when a pack is opened. All variants for
    /// a given pack sum to 1.
    pub const fn pull_rate(&self) -> Prob {
        self.pull_rate
    }

    /// Draw slots for this variant. Slot 0 is the first card shown to the player. The number
    /// of slots equals the number of cards yielded by this variant (e.g., 5 for normal, 6 for
    /// plus1).
    pub const fn slots(&self) -> &'static [PackSlot] {
        unsafe { crate::slice_unchecked(PackSlot::ALL, self.slot_ids.start, self.slot_ids.end) }
    }
}

impl std::fmt::Debug for PackVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackVariant")
            .field("id", &self.id)
            .field("name", &self.name())
            .field("series", &self.series().code())
            .field("set", &self.set().code())
            .field("pack", &self.pack().subtitle())
            .field("pull_rate", &self.pull_rate)
            .field("slots", &self.slots())
            .finish()
    }
}

impl PartialEq for PackVariant {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for PackVariant {}

impl PartialOrd for PackVariant {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for PackVariant {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for PackVariant {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for PackVariant {
    const INDEXED: &[Self] = Self::ALL;
}

//! Pack slot pull rates: per-rarity and per-card rates within a single draw position.

use crate::{CardVersion, PackVariant, Prob, Rarity};

/// One draw position within a [`PackVariant`].
///
/// A variant with 5 slots yields 5 cards; a plus1 variant has 6. Slot numbering starts at 0
/// (the first card shown to the player). Pull rates differ per slot.
pub struct PackSlot {
    pub(crate) id: usize,
    pub(crate) variant_id: usize,
    pub(crate) pull_number: usize,
    pub(crate) rarities: &'static [RarityPullRate],
    pub(crate) card_versions: &'static [CardVersionPullRate],
}

/// Pull rates for a rarity tier within a [`PackSlot`], split by finish (normal vs. foil).
pub struct RarityPullRate {
    pub(crate) rarity_id: usize,
    pub(crate) normal: Prob,
    pub(crate) foil: Prob,
}

/// Pull rate for one [`CardVersion`] within a [`PackSlot`].
pub struct CardVersionPullRate {
    pub(crate) card_version_id: usize,
    pub(crate) pull_rate: Prob,
}

impl PackSlot {
    /// All pack slots, sorted by ID.
    pub const ALL: &[Self] = crate::data::PACK_SLOTS;

    /// Returns the slot with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the slot with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`PackSlot::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Variant this slot belongs to.
    pub const fn variant(&self) -> &'static PackVariant {
        unsafe { PackVariant::from_id_unchecked(self.variant_id) }
    }

    /// 0-indexed draw position within the variant (slot 0 = first card shown to the player).
    pub const fn pull_number(&self) -> usize {
        self.pull_number
    }

    /// Per-rarity pull rates for this slot, split by normal and foil finish.
    pub const fn rarities(&self) -> &'static [RarityPullRate] {
        self.rarities
    }

    /// Per-card pull rates for this slot. Only cards that can appear in this slot are listed;
    /// absent cards have an implicit pull rate of zero.
    pub const fn card_versions(&self) -> &'static [CardVersionPullRate] {
        self.card_versions
    }
}

impl std::fmt::Debug for PackSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackSlot")
            .field("id", &self.id)
            .field("set", &self.variant().pack().set().code())
            .field("pack", &self.variant().pack().subtitle())
            .field("variant", &self.variant().name())
            .field("pull_number", &self.pull_number)
            .field("rarity_pull_rates", &self.rarities)
            .field("card_version_pull_rates", &self.card_versions)
            .finish()
    }
}

impl PartialEq for PackSlot {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for PackSlot {}

impl PartialOrd for PackSlot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for PackSlot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for PackSlot {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for PackSlot {
    const INDEXED: &[Self] = Self::ALL;
}

impl RarityPullRate {
    /// Rarity tier this entry describes.
    pub const fn rarity(&self) -> &'static Rarity {
        unsafe { Rarity::from_id_unchecked(self.rarity_id) }
    }

    /// Probability of pulling this rarity with a non-foil finish in this slot.
    pub const fn normal_pull_rate(&self) -> Prob {
        self.normal
    }

    /// Probability of pulling this rarity with a foil finish in this slot.
    pub const fn foil_pull_rate(&self) -> Prob {
        self.foil
    }
}

impl std::fmt::Debug for RarityPullRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RarityPullRate")
            .field("rarity", &self.rarity().code())
            .field("normal_pull_rate", &self.normal)
            .field("foil_pull_rate", &self.foil)
            .finish()
    }
}

impl CardVersionPullRate {
    /// Card version this entry describes.
    pub const fn card_version(&self) -> &'static CardVersion {
        unsafe { CardVersion::from_id_unchecked(self.card_version_id) }
    }

    /// Probability of this card version appearing in the associated slot.
    pub const fn pull_rate(&self) -> Prob {
        self.pull_rate
    }
}

impl std::fmt::Debug for CardVersionPullRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardVersionPullRate")
            .field(
                "card_version",
                &format_args!(
                    "{}-{:03}",
                    self.card_version().set().code(),
                    self.card_version().number()
                ),
            )
            .field("pull_rate", &self.pull_rate)
            .finish()
    }
}

//! Physical card versions: a specific printing within a set.

use crate::{
    Card, CardSource, Pack, Rarity, Series, Set,
    id_slice::IdSlice,
    str_table::{StrEntry, StrTable},
};

#[cfg(feature = "images")]
use manganis::{Asset, asset};

use std::num::NonZeroUsize;
use std::ops::Range;

/// A specific physical card: one set, collector number, rarity, illustrator, and finish.
///
/// This is the unit at which owned counts are tracked. One abstract [`Card`] may have many
/// `CardVersion`s across different sets and with different rarities or illustrators.
pub struct CardVersion {
    pub(crate) id: usize,
    pub(crate) series_id: usize,
    pub(crate) set_id: usize,
    pub(crate) card_id: usize,
    pub(crate) pack_ids: Range<usize>,
    pub(crate) number: NonZeroUsize,
    pub(crate) rarity_id: usize,
    pub(crate) illustrator_id: usize,
    pub(crate) source_id: usize,
    pub(crate) is_foil: bool,
    pub(crate) is_original: bool,
    pub(crate) is_tradable: bool,
    pub(crate) duplicate_ids: &'static [usize],
    #[cfg(feature = "images")]
    pub(crate) image: Asset,
}

impl CardVersion {
    /// All card versions in canonical display order: canonical set order (series alphabetically,
    /// then by release date within a series; promo sets last within their series), then collector
    /// number ascending within each set.
    pub const ALL: &[Self] = crate::data::CARD_VERSIONS;

    /// Illustrator name strings.
    pub const ILLUSTRATORS: &StrTable = crate::data::ILLUSTRATORS;

    /// Image of the card back, which is the same for every card.
    #[cfg(feature = "images")]
    pub const BACK: Asset = asset!("ptcgp-images/cards/back.png");

    /// Returns the version with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the version with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`CardVersion::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Series this version belongs to.
    pub const fn series(&self) -> &'static Series {
        unsafe { Series::from_id_unchecked(self.series_id) }
    }

    /// Set this version belongs to.
    pub const fn set(&self) -> &'static Set {
        unsafe { Set::from_id_unchecked(self.set_id) }
    }

    /// Abstract card holding this version's game mechanics (name, stats, attacks, effects).
    pub const fn card(&self) -> &'static Card {
        unsafe { Card::from_id_unchecked(self.card_id) }
    }

    /// Packs this version can be pulled from. Empty for cards whose [`source`] is not `"Pack"`.
    ///
    /// [`source`]: CardVersion::source
    pub const fn packs(&self) -> &'static [Pack] {
        unsafe { crate::slice_unchecked(Pack::ALL, self.pack_ids.start, self.pack_ids.end) }
    }

    /// Collector number within the set (1-indexed). Displayed as zero-padded in card codes
    /// (e.g., `A2b 025`).
    pub const fn number(&self) -> NonZeroUsize {
        self.number
    }

    /// Specific rarity tier for this version.
    pub const fn rarity(&self) -> &'static Rarity {
        unsafe { Rarity::from_id_unchecked(self.rarity_id) }
    }

    /// Illustrator credit.
    pub const fn illustrator(&self) -> StrEntry {
        unsafe { Self::ILLUSTRATORS.get_entry_unchecked(self.illustrator_id) }
    }

    /// How this card version is obtained in PTCGP. The source code `"Pack"` means the card
    /// has pull rate data; all other sources have no associated pack or pull rate data.
    pub const fn source(&self) -> &'static CardSource {
        unsafe { CardSource::from_id_unchecked(self.source_id) }
    }

    /// True if this card version has a foil finish.
    pub const fn is_foil(&self) -> bool {
        self.is_foil
    }

    /// True for the original printing; false for reprints. Exactly one version per duplicate
    /// group has `is_original == true`. When a recommendation refers to a specific version,
    /// prefer the one where `is_original` is true.
    pub const fn is_original(&self) -> bool {
        self.is_original
    }

    /// True if this version is eligible for trading or sharing between accounts in PTCGP.
    pub const fn is_tradable(&self) -> bool {
        self.is_tradable
    }

    /// Other [`CardVersion`]s that are physically identical reprints of this one: same rarity,
    /// illustrator, and finish, released in a different set. Empty if this version has no
    /// known reprints.
    pub const fn duplicates(&self) -> &'static IdSlice<Self> {
        unsafe { IdSlice::new_unchecked(self.duplicate_ids) }
    }

    /// Card front image.
    #[cfg(feature = "images")]
    pub const fn image(&self) -> Asset {
        self.image
    }
}

impl std::fmt::Debug for CardVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardVersion")
            .field("id", &self.id)
            .field("set", &self.set().code())
            .field("number", &self.number)
            .field("rarity", self.rarity())
            .field("illustrator", &self.illustrator())
            .field("source", self.source())
            .field("is_foil", &self.is_foil)
            .field("is_original", &self.is_original)
            .finish()
    }
}

impl PartialEq for CardVersion {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for CardVersion {}

impl PartialOrd for CardVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for CardVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for CardVersion {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for CardVersion {
    const INDEXED: &'static [Self] = Self::ALL;
}

use crate::{
    Card, CardSource, Pack, Rarity, Series, Set,
    id_slice::IdSlice,
    str_table::{StrEntry, StrTable},
};

#[cfg(feature = "images")]
use manganis::{Asset, asset};

use std::num::NonZeroUsize;
use std::ops::Range;

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
    pub const ALL: &[Self] = crate::data::CARD_VERSIONS;

    pub const ILLUSTRATORS: &StrTable = crate::data::ILLUSTRATORS;

    // image of the back of a card, which is the same for all cards
    #[cfg(feature = "images")]
    pub const BACK: Asset = asset!("ptcgp-images/cards/back.png");

    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    pub const fn id(&self) -> usize {
        self.id
    }

    pub const fn series(&self) -> &'static Series {
        unsafe { Series::from_id_unchecked(self.series_id) }
    }

    pub const fn set(&self) -> &'static Set {
        unsafe { Set::from_id_unchecked(self.set_id) }
    }

    pub const fn card(&self) -> &'static Card {
        unsafe { Card::from_id_unchecked(self.card_id) }
    }

    pub const fn packs(&self) -> &'static [Pack] {
        unsafe { crate::slice_unchecked(Pack::ALL, self.pack_ids.start, self.pack_ids.end) }
    }

    pub const fn number(&self) -> NonZeroUsize {
        self.number
    }

    pub const fn rarity(&self) -> &'static Rarity {
        unsafe { Rarity::from_id_unchecked(self.rarity_id) }
    }

    pub const fn illustrator(&self) -> StrEntry {
        unsafe { Self::ILLUSTRATORS.get_entry_unchecked(self.illustrator_id) }
    }

    pub const fn source(&self) -> &'static CardSource {
        unsafe { CardSource::from_id_unchecked(self.source_id) }
    }

    pub const fn is_foil(&self) -> bool {
        self.is_foil
    }

    pub const fn is_original(&self) -> bool {
        self.is_original
    }

    pub const fn is_tradable(&self) -> bool {
        self.is_tradable
    }

    pub const fn duplicates(&self) -> &'static IdSlice<Self> {
        unsafe { IdSlice::new_unchecked(self.duplicate_ids) }
    }

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

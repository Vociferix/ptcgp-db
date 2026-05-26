use crate::{
    CardVersion, PackVariant, Series, Set,
    id_slice::IdSlice,
    str_table::{StrEntry, StrTable},
};

#[cfg(feature = "images")]
use manganis::Asset;

use std::ops::Range;

pub struct Pack {
    pub(crate) id: usize,
    pub(crate) series_id: usize,
    pub(crate) set_id: usize,
    pub(crate) subtitle_id: usize,
    pub(crate) card_version_ids: &'static [usize],
    pub(crate) variant_ids: Range<usize>,
    #[cfg(feature = "images")]
    pub(crate) image: Asset,
    #[cfg(feature = "images")]
    pub(crate) logo: Asset,
}

impl Pack {
    pub const ALL: &[Self] = crate::data::PACKS;

    pub const SUBTITLES: &StrTable = crate::data::PACK_SUBTITLES;

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

    pub const fn subtitle(&self) -> StrEntry {
        unsafe { Self::SUBTITLES.get_entry_unchecked(self.subtitle_id) }
    }

    pub const fn series(&self) -> &'static Series {
        unsafe { Series::from_id_unchecked(self.series_id) }
    }

    pub const fn set(&self) -> &'static Set {
        unsafe { Set::from_id_unchecked(self.set_id) }
    }

    pub const fn card_versions(&self) -> &'static IdSlice<CardVersion> {
        unsafe { IdSlice::new_unchecked(self.card_version_ids) }
    }

    pub const fn variants(&self) -> &'static [PackVariant] {
        unsafe {
            crate::slice_unchecked(
                PackVariant::ALL,
                self.variant_ids.start,
                self.variant_ids.end,
            )
        }
    }

    #[cfg(feature = "images")]
    pub const fn image(&self) -> Asset {
        self.image
    }

    #[cfg(feature = "images")]
    pub const fn logo(&self) -> Asset {
        self.logo
    }
}

impl std::fmt::Debug for Pack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pack")
            .field("id", &self.id)
            .field("series", &self.series().code())
            .field("set", &self.set().code())
            .field("subtitle", &self.subtitle())
            .field("card_versions", &self.card_versions())
            .field("variants", &self.variants())
            .finish()
    }
}

impl PartialEq for Pack {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Pack {}

impl PartialOrd for Pack {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Pack {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Pack {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Pack {
    const INDEXED: &[Self] = Self::ALL;
}

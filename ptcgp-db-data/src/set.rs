use crate::{
    CardVersion, Pack, Series,
    str_table::{StrEntry, StrTable},
};

#[cfg(feature = "images")]
use manganis::Asset;

use chrono::NaiveDate;

use std::ops::Range;

pub struct Set {
    pub(crate) id: usize,
    pub(crate) series_id: usize,
    pub(crate) code_id: usize,
    pub(crate) name_id: usize,
    pub(crate) release_date: Option<NaiveDate>,
    pub(crate) retirement_date: Option<NaiveDate>,
    pub(crate) is_promo: bool,
    pub(crate) pack_ids: Range<usize>,
    pub(crate) card_version_ids: Range<usize>,
    #[cfg(feature = "images")]
    pub(crate) logo: Asset,
    #[cfg(feature = "images")]
    pub(crate) icon: Asset,
}

impl Set {
    pub const ALL: &[Self] = crate::data::SETS;

    pub const CODES: &StrTable = crate::data::SET_CODES;

    pub const NAMES: &StrTable = crate::data::SET_NAMES;

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

    pub const fn code(&self) -> StrEntry {
        unsafe { Self::CODES.get_entry_unchecked(self.code_id) }
    }

    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    pub const fn series(&self) -> &'static Series {
        unsafe { Series::from_id_unchecked(self.series_id) }
    }

    pub const fn packs(&self) -> &'static [Pack] {
        unsafe { crate::slice_unchecked(Pack::ALL, self.pack_ids.start, self.pack_ids.end) }
    }

    pub const fn card_versions(&self) -> &'static [CardVersion] {
        unsafe {
            crate::slice_unchecked(
                CardVersion::ALL,
                self.card_version_ids.start,
                self.card_version_ids.end,
            )
        }
    }

    pub const fn release_date(&self) -> Option<NaiveDate> {
        self.release_date
    }

    pub const fn retirement_date(&self) -> Option<NaiveDate> {
        self.retirement_date
    }

    pub const fn is_promo(&self) -> bool {
        self.is_promo
    }

    #[cfg(feature = "images")]
    pub const fn logo(&self) -> Asset {
        self.logo
    }

    #[cfg(feature = "images")]
    pub const fn icon(&self) -> Asset {
        self.icon
    }
}

impl std::fmt::Debug for Set {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Set")
            .field("id", &self.id)
            .field("series", &self.series().code())
            .field("code", &self.code())
            .field("name", &self.name())
            .field("release_date", &self.release_date)
            .field("retirement_date", &self.retirement_date)
            .field("is_promo", &self.is_promo)
            .field("packs", &self.packs())
            .finish()
    }
}

impl PartialEq for Set {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Set {}

impl PartialOrd for Set {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Set {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Set {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Set {
    const INDEXED: &[Self] = Self::ALL;
}

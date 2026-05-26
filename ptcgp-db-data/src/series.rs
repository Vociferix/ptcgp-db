use crate::{
    CardVersion, Pack, Set,
    str_table::{StrEntry, StrTable},
};

use std::ops::Range;

pub struct Series {
    pub(crate) id: usize,
    pub(crate) code_id: usize,
    pub(crate) set_ids: Range<usize>,
    pub(crate) pack_ids: Range<usize>,
    pub(crate) card_version_ids: Range<usize>,
}

impl Series {
    pub const ALL: &[Self] = crate::data::SERIES;

    pub const CODES: &StrTable = crate::data::SERIES_CODES;

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

    pub const fn code(&self) -> StrEntry {
        unsafe { Self::CODES.get_entry_unchecked(self.code_id) }
    }

    pub const fn sets(&self) -> &'static [Set] {
        unsafe { crate::slice_unchecked(Set::ALL, self.set_ids.start, self.set_ids.end) }
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
}

impl std::fmt::Debug for Series {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Series")
            .field("id", &self.id)
            .field("code", &self.code())
            .field("sets", &self.sets())
            .finish()
    }
}

impl PartialEq for Series {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Series {}

impl PartialOrd for Series {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Series {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Series {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Series {
    const INDEXED: &[Self] = Self::ALL;
}

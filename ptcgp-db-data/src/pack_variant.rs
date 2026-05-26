use crate::{
    Pack, PackSlot, Prob, Series, Set,
    str_table::{StrEntry, StrTable},
};

use std::ops::Range;

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
    pub const ALL: &[Self] = crate::data::PACK_VARIANTS;

    pub const NAMES: &StrTable = crate::data::PACK_VARIANT_NAMES;

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

    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    pub const fn series(&self) -> &'static Series {
        unsafe { Series::from_id_unchecked(self.series_id) }
    }

    pub const fn set(&self) -> &'static Set {
        unsafe { Set::from_id_unchecked(self.set_id) }
    }

    pub const fn pack(&self) -> &'static Pack {
        unsafe { Pack::from_id_unchecked(self.pack_id) }
    }

    pub const fn pull_rate(&self) -> Prob {
        self.pull_rate
    }

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

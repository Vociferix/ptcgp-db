use crate::str_table::{StrEntry, StrTable};

#[cfg(feature = "images")]
use manganis::Asset;

pub struct CardSource {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
    pub(crate) description_id: usize,
    #[cfg(feature = "images")]
    pub(crate) icon: Asset,
}

impl CardSource {
    pub const ALL: &[Self] = crate::data::CARD_SOURCES;

    pub const NAMES: &StrTable = crate::data::CARD_SOURCE_NAMES;

    pub const DESCRIPTIONS: &StrTable = crate::data::CARD_SOURCE_DESCRIPTIONS;

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

    pub const fn description(&self) -> StrEntry {
        unsafe { Self::DESCRIPTIONS.get_entry_unchecked(self.description_id) }
    }

    #[cfg(feature = "images")]
    pub const fn icon(&self) -> Asset {
        self.icon
    }
}

impl std::fmt::Debug for CardSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardSource")
            .field("id", &self.id)
            .field("name", &self.name())
            .field("description", &self.description())
            .finish()
    }
}

impl PartialEq for CardSource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for CardSource {}

impl PartialOrd for CardSource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for CardSource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for CardSource {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for CardSource {
    const INDEXED: &[Self] = Self::ALL;
}

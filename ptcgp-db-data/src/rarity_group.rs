use crate::str_table::{StrEntry, StrTable};

#[cfg(feature = "images")]
use manganis::Asset;

pub struct RarityGroup {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
    #[cfg(feature = "images")]
    pub(crate) icon: Asset,
    #[cfg(feature = "images")]
    pub(crate) symbol: Asset,
}

impl RarityGroup {
    pub const ALL: &[Self] = crate::data::RARITY_GROUPS;

    pub const NAMES: &StrTable = crate::data::RARITY_GROUP_NAMES;

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

    #[cfg(feature = "images")]
    pub const fn icon(&self) -> Asset {
        self.icon
    }

    #[cfg(feature = "images")]
    pub const fn symbol(&self) -> Asset {
        self.symbol
    }
}

impl std::fmt::Debug for RarityGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RarityGroup")
            .field("id", &self.id)
            .field("name", &self.name())
            .finish()
    }
}

impl PartialEq for RarityGroup {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for RarityGroup {}

impl PartialOrd for RarityGroup {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for RarityGroup {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for RarityGroup {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for RarityGroup {
    const INDEXED: &[Self] = Self::ALL;
}

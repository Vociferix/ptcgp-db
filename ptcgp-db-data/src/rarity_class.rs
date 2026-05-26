use crate::RarityGroup;

#[cfg(feature = "images")]
use manganis::Asset;

pub struct RarityClass {
    pub(crate) id: usize,
    pub(crate) group_id: usize,
    pub(crate) count: usize,
    #[cfg(feature = "images")]
    pub(crate) icon: Asset,
    #[cfg(feature = "images")]
    pub(crate) symbol: Asset,
}

impl RarityClass {
    pub const ALL: &[Self] = crate::data::RARITY_CLASSES;

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

    pub const fn group(&self) -> &'static RarityGroup {
        unsafe { RarityGroup::from_id_unchecked(self.group_id) }
    }

    pub const fn count(&self) -> usize {
        self.count
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

impl std::fmt::Debug for RarityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RarityClass")
            .field("id", &self.id)
            .field("group", self.group())
            .field("count", &self.count)
            .finish()
    }
}

impl PartialEq for RarityClass {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for RarityClass {}

impl PartialOrd for RarityClass {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for RarityClass {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for RarityClass {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for RarityClass {
    const INDEXED: &[Self] = Self::ALL;
}

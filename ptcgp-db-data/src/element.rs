use crate::str_table::{StrEntry, StrTable};

#[cfg(feature = "images")]
use manganis::{Asset, asset};

pub struct Element {
    pub(crate) id: usize,
    pub(crate) code: Option<char>,
    pub(crate) name_id: usize,
    #[cfg(feature = "images")]
    pub(crate) icon: Asset,
    #[cfg(feature = "images")]
    pub(crate) symbol: Asset,
}

impl Element {
    pub const ALL: &[Self] = crate::data::ELEMENTS;

    pub const NAMES: &StrTable = crate::data::ELEMENT_NAMES;

    // Energy symbol used for attack cost when cost is zero energies.
    #[cfg(feature = "images")]
    pub const NO_COST: Asset = asset!("ptcgp-images/elements/icons/no_cost.png");

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

    pub const fn code(&self) -> Option<char> {
        self.code
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

impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Element")
            .field("id", &self.id)
            .field("code", &self.code)
            .field("name", &self.name())
            .finish()
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Element {}

impl PartialOrd for Element {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Element {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Element {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Element {
    const INDEXED: &'static [Self] = Self::ALL;
}

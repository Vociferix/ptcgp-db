use crate::{
    RarityClass, RarityGroup,
    str_table::{StrEntry, StrTable},
};

pub struct Rarity {
    pub(crate) id: usize,
    pub(crate) code_id: usize,
    pub(crate) name_id: usize,
    pub(crate) class_id: usize,
    pub(crate) group_id: usize,
    pub(crate) craft_cost: u32,
    pub(crate) dupe_dust: u32,
}

impl Rarity {
    pub const ALL: &[Self] = crate::data::RARITIES;

    pub const CODES: &StrTable = crate::data::RARITY_CODES;

    pub const NAMES: &StrTable = crate::data::RARITY_NAMES;

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

    pub const fn class(&self) -> &'static RarityClass {
        unsafe { RarityClass::from_id_unchecked(self.class_id) }
    }

    pub const fn group(&self) -> &'static RarityGroup {
        unsafe { RarityGroup::from_id_unchecked(self.group_id) }
    }

    pub const fn craft_cost(&self) -> u32 {
        self.craft_cost
    }

    pub const fn dupe_dust(&self) -> u32 {
        self.dupe_dust
    }
}

impl std::fmt::Debug for Rarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rarity")
            .field("id", &self.id)
            .field("code", &self.code())
            .field("name", &self.name())
            .field("class", self.class())
            .field("craft_cost", &self.craft_cost)
            .field("dupe_dust", &self.dupe_dust)
            .finish()
    }
}

impl PartialEq for Rarity {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Rarity {}

impl PartialOrd for Rarity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Rarity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Rarity {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Rarity {
    const INDEXED: &[Self] = Self::ALL;
}

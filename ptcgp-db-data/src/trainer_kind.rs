use crate::str_table::{StrEntry, StrTable};

pub struct TrainerKind {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
}

impl TrainerKind {
    pub const ALL: &[Self] = crate::data::TRAINER_KINDS;

    pub const NAMES: &StrTable = crate::data::TRAINER_KIND_NAMES;

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
}

impl std::fmt::Debug for TrainerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrainerKind")
            .field("id", &self.id)
            .field("name", &self.name())
            .finish()
    }
}

impl PartialEq for TrainerKind {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TrainerKind {}

impl PartialOrd for TrainerKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for TrainerKind {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for TrainerKind {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for TrainerKind {
    const INDEXED: &'static [Self] = Self::ALL;
}

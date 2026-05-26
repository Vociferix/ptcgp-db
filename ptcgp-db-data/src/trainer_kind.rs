//! Trainer card categories.

use crate::str_table::{StrEntry, StrTable};

/// Category of a Trainer card (Item, Supporter, Stadium, or Tool).
pub struct TrainerKind {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
}

impl TrainerKind {
    /// All trainer kinds, sorted by ID.
    pub const ALL: &[Self] = crate::data::TRAINER_KINDS;

    /// Trainer kind name strings (e.g., `"Item"`, `"Supporter"`, `"Stadium"`, `"Tool"`).
    pub const NAMES: &StrTable = crate::data::TRAINER_KIND_NAMES;

    /// Returns the trainer kind with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the trainer kind with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`TrainerKind::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Display name (e.g., `"Item"`, `"Supporter"`, `"Stadium"`, `"Tool"`).
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

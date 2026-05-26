//! Pokémon evolution stages.

use crate::str_table::{StrEntry, StrTable};

/// Pokémon evolution stage (Basic, Stage 1, or Stage 2).
pub struct Stage {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
}

impl Stage {
    /// All stages, sorted by ID.
    pub const ALL: &[Self] = crate::data::STAGES;

    /// Stage name strings (e.g., `"Basic"`, `"Stage 1"`, `"Stage 2"`).
    pub const NAMES: &StrTable = crate::data::STAGE_NAMES;

    /// Returns the stage with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the stage with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`Stage::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Display name (e.g., `"Basic"`, `"Stage 1"`, `"Stage 2"`).
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }
}

impl std::fmt::Debug for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stage")
            .field("id", &self.id)
            .field("name", &self.name())
            .finish()
    }
}

impl PartialEq for Stage {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Stage {}

impl PartialOrd for Stage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Stage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Stage {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Stage {
    const INDEXED: &'static [Self] = Self::ALL;
}

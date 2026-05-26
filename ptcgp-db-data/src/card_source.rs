//! Card acquisition sources.

use crate::str_table::{StrEntry, StrTable};

#[cfg(feature = "images")]
use manganis::Asset;

/// How a [`CardVersion`] is obtained in PTCGP.
///
/// The source code `"Pack"` is special: it means the card has pull rate data and associated
/// packs. All other source codes (e.g., `"Premium Mission"`, `"Gold Shop"`) mean the card
/// has no pack data and must be obtained through other in-game means.
///
/// [`CardVersion`]: crate::CardVersion
pub struct CardSource {
    pub(crate) id: usize,
    pub(crate) name_id: usize,
    pub(crate) description_id: usize,
    #[cfg(feature = "images")]
    pub(crate) icon: Asset,
}

impl CardSource {
    /// All card sources, sorted by ID.
    pub const ALL: &[Self] = crate::data::CARD_SOURCES;

    /// Source name strings (e.g., `"Pack"`, `"Premium Mission"`, `"Gold Shop"`).
    pub const NAMES: &StrTable = crate::data::CARD_SOURCE_NAMES;

    /// Human-readable descriptions of each source, suitable for display in card detail views.
    pub const DESCRIPTIONS: &StrTable = crate::data::CARD_SOURCE_DESCRIPTIONS;

    /// Returns the source with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the source with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`CardSource::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Short source name (e.g., `"Pack"`, `"Premium Mission"`).
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// Human-readable description of how to obtain the card via this source.
    pub const fn description(&self) -> StrEntry {
        unsafe { Self::DESCRIPTIONS.get_entry_unchecked(self.description_id) }
    }

    /// Icon representing this source, used in place of a pack logo for non-Pack cards.
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
    const INDEXED: &'static [Self] = Self::ALL;
}

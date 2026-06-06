//! Packs and their card pools.

use crate::{
    CardVersion, PackVariant, Series, Set,
    id_slice::IdSlice,
    str_table::{StrEntry, StrTable},
};

use std::ops::Range;

/// A pack within a set, identified by its subtitle (e.g., `"Charizard"`, `"Mewtwo"`).
///
/// The full display name of a pack is derived as:
/// - `set.name` when `set.name == pack.subtitle` (the common single-pack case)
/// - `"{set.name}: {pack.subtitle}"` otherwise (e.g., `"Genetic Apex: Charizard"`)
pub struct Pack {
    pub(crate) id: usize,
    pub(crate) series_id: usize,
    pub(crate) set_id: usize,
    pub(crate) subtitle_id: usize,
    pub(crate) card_version_ids: &'static [usize],
    pub(crate) variant_ids: Range<usize>,
    pub(crate) image: &'static str,
    pub(crate) logo: &'static str,
}

impl Pack {
    /// All packs in canonical order.
    pub const ALL: &[Self] = crate::data::PACKS;

    /// Pack subtitle strings (e.g., `"Charizard"`, `"Mewtwo"`).
    pub const SUBTITLES: &StrTable = crate::data::PACK_SUBTITLES;

    /// Returns the pack with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the pack with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`Pack::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Pack subtitle (e.g., `"Charizard"`). See the type-level doc for full display name
    /// derivation.
    pub const fn subtitle(&self) -> StrEntry {
        unsafe { Self::SUBTITLES.get_entry_unchecked(self.subtitle_id) }
    }

    /// Formats the full display name of the pack: just `set.name` when `set.name == subtitle`,
    /// otherwise `"{set.name}: {subtitle}"` (e.g., `"Genetic Apex: Charizard"`).
    pub const fn title(&self) -> impl std::fmt::Display {
        struct TitleFmt<'a>(&'a Pack);

        impl std::fmt::Display for TitleFmt<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let set_name = self.0.set().name();
                let subtitle = self.0.subtitle();
                let set_name = set_name.as_str();
                let subtitle = subtitle.as_str();

                if set_name.contains(subtitle) {
                    f.write_str(set_name)
                } else {
                    write!(f, "{set_name}: {subtitle}")
                }
            }
        }

        TitleFmt(self)
    }

    /// Series this pack belongs to.
    pub const fn series(&self) -> &'static Series {
        unsafe { Series::from_id_unchecked(self.series_id) }
    }

    /// Set this pack belongs to.
    pub const fn set(&self) -> &'static Set {
        unsafe { Set::from_id_unchecked(self.set_id) }
    }

    /// Card versions that can appear in this pack, sorted by ID.
    pub const fn card_versions(&self) -> &'static IdSlice<CardVersion> {
        unsafe { IdSlice::new_unchecked(self.card_version_ids) }
    }

    /// Pull variants for this pack (e.g., normal, rare, plus1, themed). All variant
    /// [`pull_rate`]s sum to 1. Not all packs have all four variants.
    ///
    /// [`pull_rate`]: PackVariant::pull_rate
    pub const fn variants(&self) -> &'static [PackVariant] {
        unsafe {
            crate::slice_unchecked(
                PackVariant::ALL,
                self.variant_ids.start,
                self.variant_ids.end,
            )
        }
    }

    /// Full pack artwork URL.
    pub const fn image(&self) -> &'static str {
        self.image
    }

    /// Pack logo URL, suitable for space-constrained contexts.
    pub const fn logo(&self) -> &'static str {
        self.logo
    }
}

impl std::fmt::Debug for Pack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pack")
            .field("id", &self.id)
            .field("series", &self.series().code())
            .field("set", &self.set().code())
            .field("subtitle", &self.subtitle())
            .field("card_versions", &self.card_versions())
            .field("variants", &self.variants())
            .finish()
    }
}

impl PartialEq for Pack {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Pack {}

impl PartialOrd for Pack {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Pack {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Pack {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Pack {
    const INDEXED: &[Self] = Self::ALL;
}

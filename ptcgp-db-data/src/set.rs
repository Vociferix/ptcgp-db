//! Card sets and their metadata.

use crate::{
    CardVersion, Pack, Series,
    str_table::{StrEntry, StrTable},
};

use chrono::NaiveDate;

use std::ops::Range;

/// A card set (e.g., Genetic Apex, Triumphant Light).
///
/// Each set belongs to a [`Series`] and contains one or more [`Pack`]s. Promo sets are marked
/// with [`is_promo`] and have no availability window.
///
/// [`is_promo`]: Set::is_promo
pub struct Set {
    pub(crate) id: usize,
    pub(crate) series_id: usize,
    pub(crate) code_id: usize,
    pub(crate) name_id: usize,
    pub(crate) release_date: Option<NaiveDate>,
    pub(crate) retirement_date: Option<NaiveDate>,
    pub(crate) is_promo: bool,
    pub(crate) pack_ids: Range<usize>,
    pub(crate) card_version_ids: Range<usize>,
    pub(crate) logo: &'static str,
    pub(crate) icon: &'static str,
}

impl Set {
    /// All sets in canonical display order: series alphabetically, then by release date within
    /// each series (promo sets sort last within their series).
    pub const ALL: &[Self] = crate::data::SETS;

    /// Short set code strings (e.g., `"A1"`, `"B2a"`, `"P-A"`).
    pub const CODES: &StrTable = crate::data::SET_CODES;

    /// Full set display name strings (e.g., `"Genetic Apex"`, `"Triumphant Light"`).
    pub const NAMES: &StrTable = crate::data::SET_NAMES;

    /// Returns the set with the given ID without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `Self::ALL.len()`.
    pub const unsafe fn from_id_unchecked(id: usize) -> &'static Self {
        unsafe { crate::get_unchecked(Self::ALL, id) }
    }

    /// Returns the set with the given ID, or `None` if out of range.
    pub const fn from_id(id: usize) -> Option<&'static Self> {
        if id < Self::ALL.len() {
            Some(unsafe { Self::from_id_unchecked(id) })
        } else {
            None
        }
    }

    /// Numeric index into [`Set::ALL`].
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Short code identifying this set (e.g., `"A1"`, `"B2a"`, `"P-A"`).
    pub const fn code(&self) -> StrEntry {
        unsafe { Self::CODES.get_entry_unchecked(self.code_id) }
    }

    /// Full display name (e.g., `"Genetic Apex"`).
    pub const fn name(&self) -> StrEntry {
        unsafe { Self::NAMES.get_entry_unchecked(self.name_id) }
    }

    /// Release series this set belongs to.
    pub const fn series(&self) -> &'static Series {
        unsafe { Series::from_id_unchecked(self.series_id) }
    }

    /// Packs belonging to this set, in source order.
    pub const fn packs(&self) -> &'static [Pack] {
        unsafe { crate::slice_unchecked(Pack::ALL, self.pack_ids.start, self.pack_ids.end) }
    }

    /// All card versions in this set, sorted by collector number.
    pub const fn card_versions(&self) -> &'static [CardVersion] {
        unsafe {
            crate::slice_unchecked(
                CardVersion::ALL,
                self.card_version_ids.start,
                self.card_version_ids.end,
            )
        }
    }

    /// Date the set became available (`availability.start` in ptcgp-data). `None` for promo
    /// sets, which have no availability window.
    pub const fn release_date(&self) -> Option<NaiveDate> {
        self.release_date
    }

    /// Date the set became unobtainable (`availability.end` in ptcgp-data). A non-`None` date
    /// in the past means this set's packs can no longer be opened. `None` for sets that are
    /// still obtainable or for promo sets (which have no availability window).
    pub const fn retirement_date(&self) -> Option<NaiveDate> {
        self.retirement_date
    }

    /// True for promo sets. Promo sets have no availability window; individual promo card
    /// availability is not tracked at the set level.
    pub const fn is_promo(&self) -> bool {
        self.is_promo
    }

    /// Full-width set logo URL, suitable for contexts where space allows.
    pub const fn logo(&self) -> &'static str {
        self.logo
    }

    /// Compact set icon URL, suitable for space-constrained contexts.
    pub const fn icon(&self) -> &'static str {
        self.icon
    }
}

impl std::fmt::Debug for Set {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Set")
            .field("id", &self.id)
            .field("series", &self.series().code())
            .field("code", &self.code())
            .field("name", &self.name())
            .field("release_date", &self.release_date)
            .field("retirement_date", &self.retirement_date)
            .field("is_promo", &self.is_promo)
            .field("packs", &self.packs())
            .finish()
    }
}

impl PartialEq for Set {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Set {}

impl PartialOrd for Set {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Set {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.id, &other.id)
    }
}

impl std::hash::Hash for Set {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.id.hash(state);
    }
}

impl crate::id_slice::Indexed for Set {
    const INDEXED: &[Self] = Self::ALL;
}

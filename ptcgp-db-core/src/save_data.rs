//! Serializable types that represent persisted app state.
//!
//! Each top-level save data struct includes a `format_version` field. When loading, callers
//! must check this value and apply the migration logic in `ptcgp_db_core::migration` (T05)
//! before use. All structs derive `serde::Serialize` and `serde::Deserialize`; the canonical
//! user-facing export format is JSON, while internal storage formats may vary by backend.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Index into [`ptcgp_db_data::CardVersion::ALL`], used as the key for owned-count storage.
pub type CardVersionId = usize;

/// Current format version written when creating new [`ProfilesSaveData`].
pub const PROFILES_FORMAT_VERSION: u32 = 1;

/// Current format version written when creating new [`AppSettingsSaveData`].
pub const SETTINGS_FORMAT_VERSION: u32 = 1;

/// Current format version written when creating new [`SavedQueriesSaveData`].
pub const QUERIES_FORMAT_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

/// Persisted collection data for all profiles.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProfilesSaveData {
    /// Identifies the schema version; used for migration on load.
    pub format_version: u32,

    /// All profiles, in the order they were created.
    pub profiles: Vec<ProfileData>,

    /// Name of the profile designated as the primary profile. Must match one entry in
    /// [`profiles`](Self::profiles). On first run this is the name the user entered during
    /// onboarding.
    pub primary_profile_name: String,
}

/// Persisted data for a single named profile.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProfileData {
    /// Display name; unique across all profiles.
    pub name: String,

    /// Owned count per card version. Keys are [`CardVersionId`] values (indices into
    /// `CardVersion::ALL`); absent entries implicitly have a count of zero. JSON serializes
    /// these keys as decimal strings.
    pub owned_counts: HashMap<CardVersionId, u32>,
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// Persisted app-wide preferences.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppSettingsSaveData {
    /// Identifies the schema version; used for migration on load.
    pub format_version: u32,

    /// Color scheme preference.
    pub theme: Theme,

    /// When `true`, sets with a past retirement date are hidden from the entire app.
    pub ignore_unobtainable_sets: bool,

    /// When `true`, cards whose source name is `"Premium Mission"` are hidden from the
    /// entire app.
    pub ignore_premium_mission: bool,

    /// When `true`, cards whose source name is `"Gold Shop"` are hidden from the entire app.
    pub ignore_gold_shop: bool,

    /// When `true`, card versions linked by `CardVersion::duplicates()` are treated as a
    /// single logical card throughout the app.
    pub merge_duplicate_printings: bool,

    /// Names of the currently active profiles. Must be a non-empty subset of the profile
    /// names stored in [`ProfilesSaveData`].
    pub active_profile_names: Vec<String>,
}

/// UI color scheme preference.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    /// Follow the operating system or browser preference (the app default).
    #[default]
    System,
}

// ---------------------------------------------------------------------------
// Saved queries
// ---------------------------------------------------------------------------

/// Persisted list of named filter configurations shared across all profiles.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedQueriesSaveData {
    /// Identifies the schema version; used for migration on load.
    pub format_version: u32,

    /// All saved queries, in the order they were created.
    pub queries: Vec<SavedQuery>,
}

/// A named, saved filter configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SavedQuery {
    /// User-defined name for this query.
    pub name: String,

    /// The saved filter state. Fields are populated in T07/T14.
    pub config: FilterConfig,
}

/// Serializable representation of a filter toolbar configuration.
///
/// This struct is a stub at format version 1. All filter fields will be added in T07
/// (AppSettings and SavedQueries types), coordinated with T14 (Shared Filter Toolbar).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct FilterConfig {
    // Fields to be added in T07 / T14.
}

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

    /// Names of the currently active profiles. Always a non-empty subset of the profile names
    /// in [`profiles`](Self::profiles) whenever profiles is non-empty. Deserialized with a
    /// default of empty so that save files written before this field was added can be loaded
    /// cleanly; [`ProfileStore`](crate::ProfileStore) fills in the primary profile when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_profile_names: Vec<String>,
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

/// Serializable representation of a filter toolbar configuration, used for saved queries.
///
/// Fields that are `None` or empty have no filtering effect (they accept all cards).
/// This struct is shared between the Card Catalog, Analysis, and Trade pages.
/// Fields with `#[serde(default, skip_serializing_if = "...")]` are omitted from JSON when
/// unset, keeping saved queries compact.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct FilterConfig {
    /// Case-insensitive substring match on card name or collector number string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_query: Option<String>,

    /// Series filter (single-select). Value is a `Series::id()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<usize>,

    /// Set filter (multi-select). Values are `Set::id()`s.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sets: Vec<usize>,

    /// Pack filter (multi-select). Values are `Pack::id()`s.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packs: Vec<usize>,

    /// Rarity class filter (multi-select). Values are `RarityClass::id()`s.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rarities: Vec<usize>,

    /// Card kind filter (single-select). `None` accepts both Pokémon and Trainer cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_kind: Option<CardKindFilter>,

    /// Ex filter. `Some(true)` = ex only; `Some(false)` = exclude ex; `None` = no filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ex: Option<bool>,

    /// Mega filter. `Some(true)` = Mega only; `Some(false)` = exclude Mega; `None` = no filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mega: Option<bool>,

    /// Stage filter (single-select). Value is a `Stage::id()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<usize>,

    /// Element filter (multi-select). Values are `Element::id()`s.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<usize>,

    /// Foil filter. `Some(true)` = foil only; `Some(false)` = non-foil only; `None` = no filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foil: Option<bool>,

    /// Card source filter (multi-select). Values are `CardSource::id()`s.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<usize>,

    /// Obtainable filter. `Some(true)` = obtainable only; `Some(false)` = unobtainable only;
    /// `None` = no filter. The Analysis page defaults this to `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obtainable: Option<bool>,

    /// Owned count threshold (Card Catalog only). `None` = no filter. The Analysis and Trade
    /// pages use [`goal`] instead and ignore this field.
    ///
    /// [`goal`]: FilterConfig::goal
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owned_count: Option<CountThreshold>,

    /// Goal count T (Analysis and Trade pages only). Cards where `count < goal` are "desired"
    /// and drive pack probability calculations. Defaults to `1`.
    #[serde(default = "FilterConfig::default_goal")]
    pub goal: u32,

    /// Any-version-owned toggle (Analysis and Trade pages only). When `true`, a card version
    /// is treated as owned if any version of the same abstract card has aggregate count > 0.
    #[serde(default)]
    pub any_version_owned: bool,
}

impl FilterConfig {
    fn default_goal() -> u32 {
        1
    }
}

/// Card kind discriminant used for filtering.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardKindFilter {
    /// Filter to Pokémon cards only.
    Pokemon,
    /// Filter to Trainer cards only.
    Trainer,
}

/// Owned count comparison used as a filter threshold.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CountThreshold {
    /// Match cards where owned count equals `n`.
    Equal(u32),
    /// Match cards where owned count is strictly less than `n`.
    LessThan(u32),
    /// Match cards where owned count is greater than or equal to `n`.
    AtLeast(u32),
}

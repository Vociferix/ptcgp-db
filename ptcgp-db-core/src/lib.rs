//! Business logic: probability calculations, collection model, storage, and migrations.
//! See DESIGN.md §ptcgp-db-core for responsibilities.

pub mod queries;
pub mod save_data;
pub mod settings;
pub mod storage;

pub use queries::{RenameError, SavedQueries};
pub use save_data::{
    AppSettingsSaveData, CardKindFilter, CardVersionId, CountThreshold, FilterConfig,
    PROFILES_FORMAT_VERSION, ProfileData, ProfilesSaveData, QUERIES_FORMAT_VERSION,
    SETTINGS_FORMAT_VERSION, SavedQueriesSaveData, SavedQuery, Theme,
};
pub use settings::AppSettings;

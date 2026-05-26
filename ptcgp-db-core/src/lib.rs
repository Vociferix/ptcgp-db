//! Business logic: probability calculations, collection model, storage, and migrations.
//! See DESIGN.md §ptcgp-db-core for responsibilities.

pub mod save_data;
pub mod storage;

pub use save_data::{
    AppSettingsSaveData, CardVersionId, FilterConfig, PROFILES_FORMAT_VERSION, ProfileData,
    ProfilesSaveData, QUERIES_FORMAT_VERSION, SETTINGS_FORMAT_VERSION, SavedQueriesSaveData,
    SavedQuery, Theme,
};

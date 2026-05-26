//! Persistence interface for app data.
//!
//! [`Storage`] is an async trait implemented once per target platform:
//!
//! - Web (T03): IndexedDB via `web-sys` or `rexie`
//! - Desktop / Mobile (T04): a JSON file in the platform data directory
//!
//! Callers (T06 `ProfileStore`, T07 `AppSettings` / `SavedQueries`) hold a concrete backend
//! type rather than a `dyn Storage` pointer, so AFIT (async fn in trait) works without boxing.
//!
//! On load, callers must pass the returned data through the migration layer (T05) before use.

use crate::save_data::{AppSettingsSaveData, ProfilesSaveData, SavedQueriesSaveData};

/// Backend-independent persistence interface.
///
/// Implementations must be cheap to clone (wrapping shared state in `Arc` if necessary), since
/// `ProfileStore` may need to pass a handle to background save tasks.
///
/// `async fn` is used here rather than RPITIT + `Send` because the web backend (IndexedDB,
/// T03) uses `!Send` types. Callers always hold a concrete backend type, not `dyn Storage`,
/// so the missing auto-trait bounds on the returned futures are not an issue.
#[allow(async_fn_in_trait)]
pub trait Storage {
    /// Error type returned by all storage operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Loads persisted profile data, or returns `None` if no data has been saved yet.
    async fn load_profiles(&self) -> Result<Option<ProfilesSaveData>, Self::Error>;

    /// Persists profile data, overwriting any previously saved state.
    async fn save_profiles(&self, data: &ProfilesSaveData) -> Result<(), Self::Error>;

    /// Loads persisted app settings, or returns `None` if no settings have been saved yet.
    async fn load_settings(&self) -> Result<Option<AppSettingsSaveData>, Self::Error>;

    /// Persists app settings, overwriting any previously saved state.
    async fn save_settings(&self, data: &AppSettingsSaveData) -> Result<(), Self::Error>;

    /// Loads persisted saved queries, or returns `None` if none have been saved yet.
    async fn load_saved_queries(&self) -> Result<Option<SavedQueriesSaveData>, Self::Error>;

    /// Persists saved queries, overwriting any previously saved state.
    async fn save_saved_queries(&self, data: &SavedQueriesSaveData) -> Result<(), Self::Error>;
}

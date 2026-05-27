//! File-based JSON storage backend for desktop and mobile targets.
//!
//! Each of the three data types is stored as a separate JSON file in the platform data directory:
//! - `profiles.json` — [`ProfilesSaveData`]
//! - `settings.json` — [`AppSettingsSaveData`]
//! - `saved_queries.json` — [`SavedQueriesSaveData`]
//!
//! The data directory is located via [`dirs::data_dir`]:
//! - Linux: `~/.local/share/ptcgp-db/`
//! - Windows: `%APPDATA%\ptcgp-db\`
//! - macOS: `~/Library/Application Support/ptcgp-db/`
//! - Android/iOS: platform data directory provided by `dirs`
//!
//! Saves are written atomically: the JSON is first written to a `.tmp` file in the same
//! directory, then renamed to the final path. This prevents corruption if the process is
//! killed mid-write.

#![cfg(not(target_arch = "wasm32"))]

use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::save_data::{AppSettingsSaveData, ProfilesSaveData, SavedQueriesSaveData};
use crate::storage::Storage;

const APP_DIR_NAME: &str = "ptcgp-db";
const FILE_PROFILES: &str = "profiles.json";
const FILE_SETTINGS: &str = "settings.json";
const FILE_SAVED_QUERIES: &str = "saved_queries.json";

/// Error type for [`FileStorage`] operations.
#[derive(Error, Debug)]
pub enum FileStorageError {
    /// The platform data directory could not be determined.
    #[error("could not determine platform data directory")]
    NoDataDir,
    /// An I/O error occurred while reading or writing a file.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    /// JSON serialization or deserialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// File-backed [`Storage`] implementation for desktop and mobile targets.
///
/// Cheap to clone — the struct holds only a [`PathBuf`] for the data directory.
#[derive(Clone, Debug)]
pub struct FileStorage {
    dir: PathBuf,
}

impl FileStorage {
    /// Opens the storage backend, creating the data directory if it does not yet exist.
    pub fn open() -> Result<Self, FileStorageError> {
        let dir = dirs::data_dir()
            .ok_or(FileStorageError::NoDataDir)?
            .join(APP_DIR_NAME);
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Opens with an explicit directory path, creating it if it does not exist.
    ///
    /// Primarily used in tests to avoid touching the real user data directory.
    pub fn open_at(dir: PathBuf) -> Result<Self, FileStorageError> {
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn load<T: for<'de> Deserialize<'de>>(
        &self,
        filename: &str,
    ) -> Result<Option<T>, FileStorageError> {
        let path = self.dir.join(filename);
        match std::fs::read_to_string(&path) {
            Ok(content) => Ok(Some(serde_json::from_str(&content)?)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(FileStorageError::Io(e)),
        }
    }

    fn save<T: Serialize>(&self, filename: &str, data: &T) -> Result<(), FileStorageError> {
        let path = self.dir.join(filename);
        let tmp_path = self.dir.join(format!("{filename}.tmp"));
        let json = serde_json::to_string(data)?;
        std::fs::write(&tmp_path, &json)?;
        std::fs::rename(&tmp_path, &path)?;
        Ok(())
    }
}

impl Storage for FileStorage {
    type Error = FileStorageError;

    async fn load_profiles(&self) -> Result<Option<ProfilesSaveData>, Self::Error> {
        self.load(FILE_PROFILES)
    }

    async fn save_profiles(&self, data: &ProfilesSaveData) -> Result<(), Self::Error> {
        self.save(FILE_PROFILES, data)
    }

    async fn load_settings(&self) -> Result<Option<AppSettingsSaveData>, Self::Error> {
        self.load(FILE_SETTINGS)
    }

    async fn save_settings(&self, data: &AppSettingsSaveData) -> Result<(), Self::Error> {
        self.save(FILE_SETTINGS, data)
    }

    async fn load_saved_queries(&self) -> Result<Option<SavedQueriesSaveData>, Self::Error> {
        self.load(FILE_SAVED_QUERIES)
    }

    async fn save_saved_queries(&self, data: &SavedQueriesSaveData) -> Result<(), Self::Error> {
        self.save(FILE_SAVED_QUERIES, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_data::{
        PROFILES_FORMAT_VERSION, ProfileData, QUERIES_FORMAT_VERSION, SETTINGS_FORMAT_VERSION,
        Theme,
    };

    fn temp_storage() -> (FileStorage, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let storage = FileStorage::open_at(dir.path().to_path_buf()).expect("open_at failed");
        // Return the TempDir so the caller keeps it alive (dropping it removes the directory).
        (storage, dir)
    }

    fn sample_profiles() -> ProfilesSaveData {
        ProfilesSaveData {
            format_version: PROFILES_FORMAT_VERSION,
            profiles: vec![ProfileData {
                name: "Main".to_string(),
                owned_counts: Default::default(),
            }],
            primary_profile_name: "Main".to_string(),
        }
    }

    fn sample_settings() -> AppSettingsSaveData {
        AppSettingsSaveData {
            format_version: SETTINGS_FORMAT_VERSION,
            theme: Theme::System,
            ignore_unobtainable_sets: false,
            ignore_premium_mission: false,
            ignore_gold_shop: false,
            merge_duplicate_printings: false,
            active_profile_names: vec!["Main".to_string()],
        }
    }

    fn sample_queries() -> SavedQueriesSaveData {
        SavedQueriesSaveData {
            format_version: QUERIES_FORMAT_VERSION,
            queries: vec![],
        }
    }

    #[tokio::test]
    async fn profiles_round_trip() {
        let (storage, _dir) = temp_storage();
        assert!(storage.load_profiles().await.unwrap().is_none());
        let data = sample_profiles();
        storage.save_profiles(&data).await.unwrap();
        assert_eq!(storage.load_profiles().await.unwrap(), Some(data));
    }

    #[tokio::test]
    async fn settings_round_trip() {
        let (storage, _dir) = temp_storage();
        assert!(storage.load_settings().await.unwrap().is_none());
        let data = sample_settings();
        storage.save_settings(&data).await.unwrap();
        assert_eq!(storage.load_settings().await.unwrap(), Some(data));
    }

    #[tokio::test]
    async fn saved_queries_round_trip() {
        let (storage, _dir) = temp_storage();
        assert!(storage.load_saved_queries().await.unwrap().is_none());
        let data = sample_queries();
        storage.save_saved_queries(&data).await.unwrap();
        assert_eq!(storage.load_saved_queries().await.unwrap(), Some(data));
    }

    #[tokio::test]
    async fn overwrite_replaces_previous() {
        let (storage, _dir) = temp_storage();
        let first = sample_settings();
        storage.save_settings(&first).await.unwrap();
        let mut second = first.clone();
        second.theme = Theme::Dark;
        storage.save_settings(&second).await.unwrap();
        assert_eq!(storage.load_settings().await.unwrap(), Some(second));
    }

    #[tokio::test]
    async fn each_file_is_independent() {
        let (storage, _dir) = temp_storage();
        storage.save_profiles(&sample_profiles()).await.unwrap();
        // Settings and queries not yet saved — should still return None independently.
        assert!(storage.load_settings().await.unwrap().is_none());
        assert!(storage.load_saved_queries().await.unwrap().is_none());
    }
}

//! IndexedDB storage backend for web (WASM) targets.
//!
//! Uses three object stores — `"profiles"`, `"settings"`, `"saved_queries"` — each holding a
//! single JSON-serialized document keyed by the constant string `"data"`.

#![cfg(target_arch = "wasm32")]

use std::rc::Rc;

use rexie::{ObjectStore, Rexie, TransactionMode};
use thiserror::Error;
use wasm_bindgen::JsValue;

use crate::save_data::{AppSettingsSaveData, ProfilesSaveData, SavedQueriesSaveData};
use crate::storage::Storage;

const DB_NAME: &str = "ptcgp-db";
const DB_VERSION: u32 = 1;
const STORE_PROFILES: &str = "profiles";
const STORE_SETTINGS: &str = "settings";
const STORE_SAVED_QUERIES: &str = "saved_queries";
/// Fixed out-of-line key used for the single document in each object store.
const DOC_KEY: &str = "data";

/// Error type for [`WebStorage`] operations.
#[derive(Error, Debug)]
pub enum WebStorageError {
    /// IndexedDB operation failed; the underlying `rexie::Error` is converted to a string
    /// so that `WebStorageError` satisfies `Send + Sync + 'static`.
    #[error("IndexedDB error: {0}")]
    Rexie(String),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    /// The value read from the store was not a JavaScript string as expected.
    #[error("stored value has unexpected type (expected a JSON string)")]
    InvalidValueType,
}

impl From<rexie::Error> for WebStorageError {
    fn from(e: rexie::Error) -> Self {
        WebStorageError::Rexie(e.to_string())
    }
}

/// IndexedDB-backed [`Storage`] implementation for web targets.
///
/// Cheap to clone — the underlying [`Rexie`] handle is reference-counted.
#[derive(Clone)]
pub struct WebStorage {
    rexie: Rc<Rexie>,
}

impl WebStorage {
    /// Opens (or creates) the IndexedDB database and returns a ready-to-use handle.
    pub async fn open() -> Result<Self, WebStorageError> {
        let rexie = Rexie::builder(DB_NAME)
            .version(DB_VERSION)
            .add_object_store(ObjectStore::new(STORE_PROFILES))
            .add_object_store(ObjectStore::new(STORE_SETTINGS))
            .add_object_store(ObjectStore::new(STORE_SAVED_QUERIES))
            .build()
            .await?;
        Ok(Self {
            rexie: Rc::new(rexie),
        })
    }

    async fn load<T: for<'de> serde::Deserialize<'de>>(
        &self,
        store_name: &str,
    ) -> Result<Option<T>, WebStorageError> {
        let tx = self
            .rexie
            .transaction(&[store_name], TransactionMode::ReadOnly)?;
        let store = tx.store(store_name)?;
        let js_val = store.get(JsValue::from_str(DOC_KEY)).await?;
        tx.done().await?;
        match js_val {
            None => Ok(None),
            Some(v) => {
                let json = v.as_string().ok_or(WebStorageError::InvalidValueType)?;
                Ok(Some(serde_json::from_str(&json)?))
            }
        }
    }

    async fn save<T: serde::Serialize>(
        &self,
        store_name: &str,
        data: &T,
    ) -> Result<(), WebStorageError> {
        let json = serde_json::to_string(data)?;
        let js_val = JsValue::from_str(&json);
        let tx = self
            .rexie
            .transaction(&[store_name], TransactionMode::ReadWrite)?;
        let store = tx.store(store_name)?;
        store
            .put(&js_val, Some(&JsValue::from_str(DOC_KEY)))
            .await?;
        tx.done().await?;
        Ok(())
    }
}

impl Storage for WebStorage {
    type Error = WebStorageError;

    async fn load_profiles(&self) -> Result<Option<ProfilesSaveData>, Self::Error> {
        self.load(STORE_PROFILES).await
    }

    async fn save_profiles(&self, data: &ProfilesSaveData) -> Result<(), Self::Error> {
        self.save(STORE_PROFILES, data).await
    }

    async fn load_settings(&self) -> Result<Option<AppSettingsSaveData>, Self::Error> {
        self.load(STORE_SETTINGS).await
    }

    async fn save_settings(&self, data: &AppSettingsSaveData) -> Result<(), Self::Error> {
        self.save(STORE_SETTINGS, data).await
    }

    async fn load_saved_queries(&self) -> Result<Option<SavedQueriesSaveData>, Self::Error> {
        self.load(STORE_SAVED_QUERIES).await
    }

    async fn save_saved_queries(&self, data: &SavedQueriesSaveData) -> Result<(), Self::Error> {
        self.save(STORE_SAVED_QUERIES, data).await
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::wasm_bindgen_test;

    use super::*;
    use crate::save_data::{
        PROFILES_FORMAT_VERSION, QUERIES_FORMAT_VERSION, SETTINGS_FORMAT_VERSION, Theme,
    };

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn sample_profiles() -> ProfilesSaveData {
        ProfilesSaveData {
            format_version: PROFILES_FORMAT_VERSION,
            profiles: vec![crate::save_data::ProfileData {
                name: "Main".to_string(),
                owned_counts: Default::default(),
            }],
            primary_profile_name: "Main".to_string(),
            active_profile_names: vec!["Main".to_string()],
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
        }
    }

    fn sample_queries() -> SavedQueriesSaveData {
        SavedQueriesSaveData {
            format_version: QUERIES_FORMAT_VERSION,
            queries: vec![],
        }
    }

    #[wasm_bindgen_test]
    async fn profiles_round_trip() {
        let storage = WebStorage::open().await.unwrap();
        assert!(storage.load_profiles().await.unwrap().is_none());
        let data = sample_profiles();
        storage.save_profiles(&data).await.unwrap();
        assert_eq!(storage.load_profiles().await.unwrap(), Some(data));
    }

    #[wasm_bindgen_test]
    async fn settings_round_trip() {
        let storage = WebStorage::open().await.unwrap();
        assert!(storage.load_settings().await.unwrap().is_none());
        let data = sample_settings();
        storage.save_settings(&data).await.unwrap();
        assert_eq!(storage.load_settings().await.unwrap(), Some(data));
    }

    #[wasm_bindgen_test]
    async fn saved_queries_round_trip() {
        let storage = WebStorage::open().await.unwrap();
        assert!(storage.load_saved_queries().await.unwrap().is_none());
        let data = sample_queries();
        storage.save_saved_queries(&data).await.unwrap();
        assert_eq!(storage.load_saved_queries().await.unwrap(), Some(data));
    }

    #[wasm_bindgen_test]
    async fn overwrite_replaces_previous() {
        let storage = WebStorage::open().await.unwrap();
        let first = sample_settings();
        storage.save_settings(&first).await.unwrap();
        let mut second = first.clone();
        second.theme = Theme::Dark;
        storage.save_settings(&second).await.unwrap();
        assert_eq!(storage.load_settings().await.unwrap(), Some(second));
    }
}

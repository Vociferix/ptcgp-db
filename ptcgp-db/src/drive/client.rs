//! Drive REST API v3 client.
//!
//! Provides the four operations needed for sync: find, read, create, and update a single JSON
//! file in the user's `appDataFolder`.

use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

use super::{DriveSyncData, SYNC_FILE_NAME};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from Drive REST API operations.
#[derive(Error, Debug)]
pub enum DriveError {
    #[error("unauthenticated — token may be expired or revoked")]
    Unauthenticated,
    #[error("Drive API HTTP {status}: {reason}")]
    Http { status: u16, reason: String },
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Response shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct FileMeta {
    id: String,
}

#[derive(Deserialize)]
struct ListResponse {
    files: Vec<FileMeta>,
}

// ---------------------------------------------------------------------------
// DriveClient
// ---------------------------------------------------------------------------

/// Stateless Drive REST v3 helper. Create one per operation; `reqwest::Client` is lightweight
/// on WASM because the browser owns the underlying fetch machinery.
pub struct DriveClient {
    http: Client,
}

impl DriveClient {
    /// Creates a new client.
    pub fn new() -> Self {
        Self { http: Client::new() }
    }

    /// Searches `appDataFolder` for the sync file. Returns the Drive file ID, or `None` when
    /// the file does not exist yet.
    pub async fn find_sync_file(&self, token: &str) -> Result<Option<String>, DriveError> {
        let resp = self
            .http
            .get("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(token)
            .query(&[
                ("spaces", "appDataFolder"),
                ("fields", "files(id)"),
                ("q", &format!("name = '{SYNC_FILE_NAME}'")),
            ])
            .send()
            .await?;
        check_status(&resp)?;
        let list: ListResponse = resp.json().await?;
        Ok(list.files.into_iter().next().map(|f| f.id))
    }

    /// Downloads and deserializes the sync file.
    pub async fn read_sync_file(
        &self,
        token: &str,
        file_id: &str,
    ) -> Result<DriveSyncData, DriveError> {
        let url = format!("https://www.googleapis.com/drive/v3/files/{file_id}?alt=media");
        let resp = self.http.get(&url).bearer_auth(token).send().await?;
        check_status(&resp)?;
        let data: DriveSyncData = resp.json().await?;
        Ok(data)
    }

    /// Creates the sync file in `appDataFolder` using a single multipart upload.
    /// Returns the new Drive file ID.
    pub async fn create_sync_file(
        &self,
        token: &str,
        data: &DriveSyncData,
    ) -> Result<String, DriveError> {
        let boundary = "ptcgpdb";
        let metadata = format!(r#"{{"name":"{SYNC_FILE_NAME}","parents":["appDataFolder"]}}"#);
        let content = serde_json::to_string(data)?;
        let body = format!(
            "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n\
             {metadata}\r\n\
             --{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n\
             {content}\r\n\
             --{boundary}--"
        );
        let resp = self
            .http
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
            .bearer_auth(token)
            .header("Content-Type", format!("multipart/related; boundary={boundary}"))
            .body(body)
            .send()
            .await?;
        check_status(&resp)?;
        let meta: FileMeta = resp.json().await?;
        Ok(meta.id)
    }

    /// Overwrites the content of an existing sync file.
    pub async fn update_sync_file(
        &self,
        token: &str,
        file_id: &str,
        data: &DriveSyncData,
    ) -> Result<(), DriveError> {
        let url = format!(
            "https://www.googleapis.com/upload/drive/v3/files/{file_id}?uploadType=media"
        );
        let content = serde_json::to_string(data)?;
        let resp = self
            .http
            .patch(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/json; charset=UTF-8")
            .body(content)
            .send()
            .await?;
        check_status(&resp)?;
        Ok(())
    }

    /// Saves data to Drive, creating or updating the file as needed.
    ///
    /// `cached_file_id` is the Drive file ID from a prior successful save; pass `None` on the
    /// first call or after a cache miss. Returns the file ID to store for subsequent calls.
    pub async fn save(
        &self,
        token: &str,
        cached_file_id: Option<&str>,
        data: &DriveSyncData,
    ) -> Result<String, DriveError> {
        if let Some(id) = cached_file_id {
            self.update_sync_file(token, id, data).await?;
            return Ok(id.to_string());
        }
        // No cached ID: look up the file before creating to avoid duplicates.
        match self.find_sync_file(token).await? {
            Some(id) => {
                self.update_sync_file(token, &id, data).await?;
                Ok(id)
            }
            None => self.create_sync_file(token, data).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn check_status(resp: &reqwest::Response) -> Result<(), DriveError> {
    let status = resp.status();
    if status.as_u16() == 401 {
        return Err(DriveError::Unauthenticated);
    }
    if !status.is_success() {
        return Err(DriveError::Http {
            status: status.as_u16(),
            reason: status.canonical_reason().unwrap_or("unknown").to_string(),
        });
    }
    Ok(())
}

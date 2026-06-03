//! Google Drive sync for web builds.
//!
//! Provides OAuth token acquisition via Google Identity Services (GIS) and a lightweight
//! Drive REST v3 client for reading/writing a single bundled JSON file in the user's
//! `appDataFolder`. All types and functions in this module are WASM-only.

mod client;
mod gis;

pub use client::{DriveClient, DriveError};

use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;
use ptcgp_db_core::save_data::{AppSettingsSaveData, ProfilesSaveData, SavedQueriesSaveData};
use ptcgp_db_core::storage::Storage as _;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// OAuth2 client ID for Google Drive access.
pub(crate) const CLIENT_ID: &str =
    "353554631088-jf3omarc3aoh0dibng0g6l6up0u3vl8c.apps.googleusercontent.com";

/// Drive scope that grants access only to files created by this app.
const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";

/// `localStorage` key that records whether Drive sync is enabled in this browser.
const CONNECTED_KEY: &str = "ptcgp-db-drive-connected";

/// Name of the sync file in the user's Drive `appDataFolder`.
pub(crate) const SYNC_FILE_NAME: &str = "ptcgp-db-sync.json";

// ---------------------------------------------------------------------------
// Sync data bundle
// ---------------------------------------------------------------------------

/// All persisted app data bundled into a single Drive file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DriveSyncData {
    pub profiles: ProfilesSaveData,
    pub settings: AppSettingsSaveData,
    pub queries: SavedQueriesSaveData,
}

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

/// A live Google OAuth2 access token with its expiry time.
#[derive(Clone, Debug)]
pub struct DriveToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

impl DriveToken {
    /// Returns `true` if the token has expired or expires within the next 60 seconds.
    pub fn is_expired(&self) -> bool {
        Utc::now() + Duration::seconds(60) >= self.expires_at
    }
}

// ---------------------------------------------------------------------------
// Drive state
// ---------------------------------------------------------------------------

/// Sync connection state held in the `Signal<DriveState>` Dioxus context.
#[derive(Clone, Debug, Default)]
pub enum DriveState {
    /// Drive sync is not configured in this browser.
    #[default]
    Disconnected,
    /// Silent token acquisition is in progress (app startup).
    Connecting,
    /// Drive is connected with a valid token.
    Connected {
        token: DriveToken,
        /// Cached Drive file ID; `None` until the first successful save or load.
        file_id: Option<String>,
    },
    /// The last auth or Drive operation failed.
    Error(String),
}

impl DriveState {
    /// Returns `true` when connected (regardless of token expiry).
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
}

// ---------------------------------------------------------------------------
// localStorage helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the user has previously enabled Drive sync in this browser.
pub fn is_drive_enabled() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|ls| ls.get_item(CONNECTED_KEY).ok().flatten())
        .is_some()
}

/// Persists or clears the Drive-enabled flag in `localStorage`.
pub fn set_drive_enabled(enabled: bool) {
    let Some(ls) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
        return;
    };
    if enabled {
        let _ = ls.set_item(CONNECTED_KEY, "1");
    } else {
        let _ = ls.remove_item(CONNECTED_KEY);
    }
}

// ---------------------------------------------------------------------------
// Token acquisition
// ---------------------------------------------------------------------------

/// Attempts to acquire a Drive access token without user interaction.
///
/// Succeeds when the user has an active Google session and has previously granted the
/// Drive scope. Returns `Err` if interaction is required.
pub async fn acquire_token_silent() -> Result<DriveToken, String> {
    acquire_token("").await
}

/// Acquires a Drive access token via the Google account-chooser popup.
pub async fn acquire_token_interactive() -> Result<DriveToken, String> {
    acquire_token("select_account").await
}

async fn acquire_token(prompt: &str) -> Result<DriveToken, String> {
    gis::ensure_gis_loaded().await?;
    let resp = gis::request_token(CLIENT_ID, SCOPE, prompt).await?;
    let expires_at = Utc::now() + Duration::seconds(resp.expires_in as i64);
    Ok(DriveToken { access_token: resp.access_token, expires_at })
}

// ---------------------------------------------------------------------------
// Drive sync helpers (used by app.rs)
// ---------------------------------------------------------------------------

/// Saves `data` to Drive, refreshing the token first if it has expired.
///
/// On success, updates the `file_id` cache in `drive_state`. On auth failure, transitions
/// the state to `Error` and clears the Drive-enabled flag so the user is prompted to reconnect.
pub async fn save_to_drive(mut drive_state: Signal<DriveState>, data: &DriveSyncData) {
    // Snapshot what we need without holding the read guard across await points.
    let (mut token, file_id) = {
        let state = drive_state.read();
        let DriveState::Connected { ref token, ref file_id } = *state else { return };
        (token.clone(), file_id.clone())
    };

    // Refresh the token if it is about to expire.
    if token.is_expired() {
        match acquire_token_silent().await {
            Ok(new_token) => {
                if let DriveState::Connected { token: ref mut t, .. } =
                    *drive_state.write()
                {
                    *t = new_token.clone();
                }
                token = new_token;
            }
            Err(e) => {
                tracing::error!("Drive token refresh failed: {e}");
                drive_state.set(DriveState::Error(format!("Token refresh failed: {e}")));
                set_drive_enabled(false);
                return;
            }
        }
    }

    let client = DriveClient::new();
    match client.save(&token.access_token, file_id.as_deref(), data).await {
        Ok(new_id) => {
            if let DriveState::Connected { file_id: ref mut fid, .. } = *drive_state.write() {
                *fid = Some(new_id);
            }
        }
        Err(DriveError::Unauthenticated) => {
            tracing::error!("Drive save rejected with 401 — clearing connection");
            drive_state.set(DriveState::Error(
                "Google Drive access was revoked. Reconnect in Settings.".to_string(),
            ));
            set_drive_enabled(false);
        }
        Err(e) => {
            tracing::error!("Drive save failed: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Shared load helper
// ---------------------------------------------------------------------------

/// Downloads the Drive sync file and writes it into IndexedDB and the in-memory signals.
///
/// Called by both the startup silent-auth path and the onboarding interactive-connect path.
/// Returns the file ID that was read, or `None` if no sync file existed yet.
pub async fn load_from_drive(
    token: &DriveToken,
    mut drive_state: Signal<DriveState>,
    mut store: Signal<Option<ptcgp_db_core::ProfileStore<crate::app::AppStorage>>>,
    mut settings: Signal<ptcgp_db_core::AppSettings>,
    mut queries: Signal<ptcgp_db_core::SavedQueries>,
) -> Option<String> {
    let client = DriveClient::new();
    let file_id = match client.find_sync_file(&token.access_token).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Drive file lookup failed: {e}");
            drive_state.set(DriveState::Connected { token: token.clone(), file_id: None });
            return None;
        }
    };

    let Some(ref id) = file_id else {
        drive_state.set(DriveState::Connected { token: token.clone(), file_id: None });
        return None;
    };

    match client.read_sync_file(&token.access_token, id).await {
        Ok(data) => {
            let storage = store.read().as_ref().map(|s| s.storage().clone());
            if let Some(storage) = storage {
                let _ = storage.save_profiles(&data.profiles).await;
                let _ = storage.save_settings(&data.settings).await;
                let _ = storage.save_saved_queries(&data.queries).await;
                if let Ok(new_store) = ptcgp_db_core::ProfileStore::load(storage).await {
                    store.set(Some(new_store));
                }
            }
            settings.set(ptcgp_db_core::AppSettings::from_save_data(data.settings));
            queries.set(ptcgp_db_core::SavedQueries::from_save_data(data.queries));
            drive_state.set(DriveState::Connected { token: token.clone(), file_id: file_id.clone() });
        }
        Err(e) => {
            tracing::error!("Drive read failed: {e}");
            drive_state.set(DriveState::Connected { token: token.clone(), file_id: file_id.clone() });
        }
    }

    file_id
}

/// Interactive Drive connect for the first-run onboarding screen.
///
/// Unlike [`connect_drive`] (which uploads local data), this authenticates interactively and
/// then downloads any existing Drive data into the app — allowing a returning user to recover
/// their collection without a file export. If no Drive data exists yet the connection is
/// established so the next auto-save will create the sync file.
///
/// When Drive data with profiles is loaded, `store` changes and the app automatically exits
/// the onboarding screen. When Drive is connected but has no data, the onboarding stays
/// visible so the user can still create their first profile.
pub async fn onboarding_connect_drive(
    mut drive_state: Signal<DriveState>,
    store: Signal<Option<ptcgp_db_core::ProfileStore<crate::app::AppStorage>>>,
    settings: Signal<ptcgp_db_core::AppSettings>,
    queries: Signal<ptcgp_db_core::SavedQueries>,
) {
    let token = match acquire_token_interactive().await {
        Ok(t) => t,
        Err(e) => {
            drive_state.set(DriveState::Error(format!("Sign-in failed: {e}")));
            return;
        }
    };

    set_drive_enabled(true);
    load_from_drive(&token, drive_state, store, settings, queries).await;
}

// ---------------------------------------------------------------------------
// Settings UI component
// ---------------------------------------------------------------------------

/// Settings section for configuring Google Drive sync.
///
/// Reads `Signal<DriveState>` from context and provides connect/disconnect controls.
#[component]
pub fn DriveSyncSection(
    store: Signal<Option<ptcgp_db_core::ProfileStore<crate::app::AppStorage>>>,
    settings: Signal<ptcgp_db_core::AppSettings>,
    queries: Signal<ptcgp_db_core::SavedQueries>,
) -> Element {
    let mut drive_state = use_context::<Signal<DriveState>>();

    let status_line = match &*drive_state.read() {
        DriveState::Disconnected => rsx! {
            p { class: "text-sm text-gray-500 dark:text-gray-400",
                "Not connected. Your data stays in this browser only."
            }
        },
        DriveState::Connecting => rsx! {
            p { class: "text-sm text-gray-500 dark:text-gray-400", "Connecting…" }
        },
        DriveState::Connected { .. } => rsx! {
            p { class: "text-sm text-green-600 dark:text-green-400 font-medium",
                "Connected — data syncs across your devices."
            }
        },
        DriveState::Error(msg) => {
            let msg = msg.clone();
            rsx! {
                p { class: "text-sm text-red-600 dark:text-red-400", "{msg}" }
            }
        }
    };

    let is_connecting = matches!(*drive_state.read(), DriveState::Connecting);
    let is_connected = drive_state.read().is_connected();
    let has_error = matches!(*drive_state.read(), DriveState::Error(_));

    rsx! {
        section {
            h2 { class: "text-xs font-semibold uppercase tracking-wider \
                          text-gray-500 dark:text-gray-400 mb-3",
                "Sync"
            }
            div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                          border-gray-200 dark:border-gray-700 p-4 space-y-3",
                div {
                    p { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                        "Google Drive"
                    }
                    p { class: "text-sm text-gray-500 dark:text-gray-400 mt-0.5",
                        "Store your collection in your Google Drive so it syncs across browsers \
                         and devices. Your data is saved in a hidden app folder only this app \
                         can see."
                    }
                }
                {status_line}
                div { class: "flex gap-2",
                    if is_connected {
                        button {
                            r#type: "button",
                            class: "px-3 py-1.5 text-sm rounded-md border \
                                    border-gray-300 dark:border-gray-600 \
                                    text-gray-700 dark:text-gray-300 \
                                    hover:bg-gray-100 dark:hover:bg-gray-700",
                            onclick: move |_| {
                                set_drive_enabled(false);
                                drive_state.set(DriveState::Disconnected);
                            },
                            "Disconnect"
                        }
                    } else {
                        button {
                            r#type: "button",
                            disabled: is_connecting,
                            class: if is_connecting { "px-3 py-1.5 text-sm rounded-md bg-blue-600 text-white opacity-60 cursor-not-allowed" } else { "px-3 py-1.5 text-sm rounded-md bg-blue-600 text-white hover:bg-blue-700" },
                            onclick: move |_| {
                                let store = store;
                                let settings = settings;
                                let queries = queries;
                                drive_state.set(DriveState::Connecting);
                                spawn(async move {
                                    connect_drive(drive_state, store, settings, queries).await;
                                });
                            },
                            if has_error {
                                "Reconnect"
                            } else {
                                "Connect Google Drive"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Runs the interactive Drive connect flow and saves the initial data bundle to Drive.
async fn connect_drive(
    mut drive_state: Signal<DriveState>,
    store: Signal<Option<ptcgp_db_core::ProfileStore<crate::app::AppStorage>>>,
    settings: Signal<ptcgp_db_core::AppSettings>,
    queries: Signal<ptcgp_db_core::SavedQueries>,
) {
    let token = match acquire_token_interactive().await {
        Ok(t) => t,
        Err(e) => {
            drive_state.set(DriveState::Error(format!("Sign-in failed: {e}")));
            return;
        }
    };

    // Snapshot current app data for the initial upload.
    let sync_data = {
        let store_guard = store.read();
        let Some(ref s) = *store_guard else {
            drive_state.set(DriveState::Error("App not ready — try again.".to_string()));
            return;
        };
        DriveSyncData {
            profiles: s.save_data_snapshot().clone(),
            settings: settings.read().as_save_data().clone(),
            queries: queries.read().as_save_data().clone(),
        }
    };

    let client = DriveClient::new();
    match client.save(&token.access_token, None, &sync_data).await {
        Ok(file_id) => {
            set_drive_enabled(true);
            drive_state.set(DriveState::Connected { token, file_id: Some(file_id) });
        }
        Err(e) => {
            drive_state.set(DriveState::Error(format!("Drive upload failed: {e}")));
        }
    }
}

//! Google Drive sync for web builds.
//!
//! Uses the OAuth 2.0 Authorization Code flow with PKCE. Auth is initiated by redirecting
//! the browser to Google's auth endpoint (no popup); on return the authorization code is
//! exchanged for access + refresh tokens via a background fetch. The refresh token is stored
//! in `localStorage` and used for silent re-authentication on every subsequent page load.

mod client;
mod pkce;
mod token_exchange;

pub use client::{DriveClient, DriveError};
pub use token_exchange::DriveAuthError;

use std::cell::OnceCell;

use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;
use ptcgp_db_core::save_data::{AppSettingsSaveData, ProfilesSaveData, SavedQueriesSaveData};
use ptcgp_db_core::storage::Storage as _;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

// ---------------------------------------------------------------------------
// Startup URL capture
// ---------------------------------------------------------------------------

thread_local! {
    // Stores window.location.search as captured in main() before Dioxus launches.
    // HashHistory rewrites the URL during initialization, stripping query params;
    // capturing early ensures the OAuth callback code is never lost.
    static STARTUP_SEARCH: OnceCell<String> = const { OnceCell::new() };
}

/// Captures `window.location.search` before Dioxus initializes.
///
/// Must be called at the very top of `main()`, before `dioxus::launch()`, because
/// `HashHistory` calls `history.replaceState` during initialization to add `#/`, which
/// drops any query params (including the OAuth `?code=…`) from the URL.
pub fn capture_startup_search() {
    STARTUP_SEARCH.with(|cell| {
        let search = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();
        let _ = cell.set(search);
    });
}

fn startup_search() -> String {
    STARTUP_SEARCH.with(|cell| cell.get().cloned().unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// OAuth2 client ID for Google Drive access.
pub(crate) const CLIENT_ID: &str =
    "353554631088-jf3omarc3aoh0dibng0g6l6up0u3vl8c.apps.googleusercontent.com";

/// OAuth2 client secret for Google Drive access.
///
/// Web Application OAuth clients are confidential clients; Google requires the secret
/// in the token exchange even with PKCE. For a browser-only app the secret is
/// technically visible in the binary, but the `drive.appdata` scope limits exposure.
pub(crate) const CLIENT_SECRET: &str = "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";

/// Drive scope that grants access only to files created by this app.
const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";

/// `localStorage` key that records whether Drive sync is enabled in this browser.
const CONNECTED_KEY: &str = "ptcgp-db-drive-connected";

/// `localStorage` key that holds the OAuth 2.0 refresh token.
const REFRESH_TOKEN_KEY: &str = "ptcgp-db-drive-refresh-token";

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
    /// Token acquisition or Drive load is in progress.
    Connecting,
    /// Drive is connected with a valid access token.
    Connected {
        token: DriveToken,
        /// Cached Drive file ID; `None` until the first successful save or load.
        file_id: Option<String>,
    },
    /// The stored refresh token was revoked; the user must re-authorize.
    NeedsReconnect,
    /// The last Drive operation failed for a non-auth reason.
    Error(String),
}

impl DriveState {
    /// Returns `true` when connected with a valid token.
    pub fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }
}

// ---------------------------------------------------------------------------
// localStorage helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the user has previously enabled Drive sync in this browser.
pub fn is_drive_enabled() -> bool {
    local_storage()
        .and_then(|ls| ls.get_item(CONNECTED_KEY).ok().flatten())
        .is_some()
}

/// Persists or clears the Drive-enabled flag in `localStorage`.
pub fn set_drive_enabled(enabled: bool) {
    let Some(ls) = local_storage() else {
        return;
    };
    if enabled {
        let _ = ls.set_item(CONNECTED_KEY, "1");
    } else {
        let _ = ls.remove_item(CONNECTED_KEY);
    }
}

/// Returns `true` when an OAuth redirect is in progress (PKCE session data is in
/// `sessionStorage`). Used on page load to detect a returning auth callback even before
/// the Drive-enabled flag has been set (first-time connect).
pub fn is_auth_callback_pending() -> bool {
    pkce::has_session_data()
}

fn load_refresh_token() -> Option<String> {
    local_storage()?.get_item(REFRESH_TOKEN_KEY).ok().flatten()
}

fn store_refresh_token(token: &str) {
    if let Some(ls) = local_storage() {
        let _ = ls.set_item(REFRESH_TOKEN_KEY, token);
    }
}

fn clear_refresh_token() {
    if let Some(ls) = local_storage() {
        let _ = ls.remove_item(REFRESH_TOKEN_KEY);
    }
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

// ---------------------------------------------------------------------------
// Auth initiation
// ---------------------------------------------------------------------------

/// Initiates the OAuth 2.0 Authorization Code flow by redirecting the browser to Google.
///
/// Generates PKCE verifier + challenge and a CSRF state token, saves them to
/// `sessionStorage` (they survive the redirect), then navigates to Google's auth endpoint.
/// After the user authenticates, Google redirects back to the app's base URL with `?code=…`
/// in the query string. Call [`handle_auth_callback`] on page load to complete the flow.
pub fn initiate_auth_redirect() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();

    let verifier = pkce::generate_verifier();
    let challenge = pkce::derive_challenge(&verifier);
    let state = pkce::generate_state();
    let return_hash = location.hash().unwrap_or_default();

    pkce::save_to_session(&verifier, &state, &return_hash);

    let origin = location.origin().unwrap_or_default();
    let pathname = location.pathname().unwrap_or_default();
    let redirect_uri = format!("{origin}{pathname}");

    let redirect_uri_enc = String::from(js_sys::encode_uri_component(&redirect_uri));
    let scope_enc = String::from(js_sys::encode_uri_component(SCOPE));

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
         ?response_type=code\
         &client_id={CLIENT_ID}\
         &redirect_uri={redirect_uri_enc}\
         &scope={scope_enc}\
         &code_challenge={challenge}\
         &code_challenge_method=S256\
         &state={state}\
         &access_type=offline\
         &prompt=consent"
    );

    let _ = location.set_href(&auth_url);
}

// ---------------------------------------------------------------------------
// Auth callback handling
// ---------------------------------------------------------------------------

/// Processes the OAuth redirect callback, exchanging the authorization code for tokens.
///
/// Reads `?code=…&state=…` from the current URL, verifies the CSRF state against
/// `sessionStorage`, exchanges the code for an access + refresh token, stores the refresh
/// token in `localStorage`, and cleans the URL (restoring the pre-redirect hash).
///
/// Returns the fresh [`DriveToken`] on success. The `DriveState` signal is **not** updated
/// here; the caller updates it after loading Drive data.
pub async fn handle_auth_callback() -> Result<DriveToken, String> {
    let window = web_sys::window().ok_or("no window object")?;
    let location = window.location();

    // Use the search string captured in main() before HashHistory stripped it.
    let search = startup_search();

    let params = web_sys::UrlSearchParams::new_with_str(&search)
        .map_err(|_| "failed to parse URL query params")?;

    // Google redirects with ?error=… when the user denies or an error occurs.
    if let Some(error) = params.get("error") {
        let _ = pkce::take_from_session();
        restore_url(&window, "");
        let desc = params.get("error_description").unwrap_or_default();
        return Err(if error == "access_denied" {
            "Sign-in was cancelled.".to_string()
        } else {
            format!("{error}: {desc}")
        });
    }

    // If neither code nor error is present, the PKCE session data is stale (e.g., from an
    // abandoned auth flow). Clean it up so we don't loop on the next page load.
    let Some(code) = params.get("code") else {
        let _ = pkce::take_from_session();
        return Err("no authorization code in URL".to_string());
    };
    let returned_state = params.get("state").ok_or("no state in URL")?;

    // Verify CSRF state and retrieve the PKCE verifier.
    let (verifier, expected_state, return_hash) =
        pkce::take_from_session().ok_or("PKCE session data missing — possible stale tab")?;

    if returned_state != expected_state {
        restore_url(&window, "");
        return Err("OAuth state mismatch — request may have been tampered with".to_string());
    }

    // Build the redirect URI exactly as it was sent in the auth request.
    let origin = location.origin().map_err(|_| "failed to read origin")?;
    let pathname = location.pathname().map_err(|_| "failed to read pathname")?;
    let redirect_uri = format!("{origin}{pathname}");

    // Exchange the code for tokens.
    let (token, refresh_token) = token_exchange::exchange_code(&code, &verifier, &redirect_uri)
        .await
        .map_err(|e| e.to_string())?;

    store_refresh_token(&refresh_token);
    set_drive_enabled(true);

    // Clean the URL: remove query params and restore the pre-redirect hash.
    // If return_hash is empty (e.g., connected from onboarding), fall back to the hash
    // that HashHistory set during initialization so the router stays consistent.
    let hash = if return_hash.is_empty() {
        location.hash().unwrap_or_default()
    } else {
        return_hash
    };
    restore_url(&window, &hash);

    Ok(token)
}

/// Replaces the current URL with `pathname + hash`, removing all query params.
fn restore_url(window: &web_sys::Window, hash: &str) {
    let location = window.location();
    let pathname = location.pathname().unwrap_or_default();
    let target = format!("{pathname}{hash}");
    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&target));
    }
}

// ---------------------------------------------------------------------------
// Token acquisition
// ---------------------------------------------------------------------------

/// Error returned by [`acquire_token_silent`].
#[derive(Debug)]
pub enum DriveConnectError {
    /// No refresh token found in `localStorage` — Drive was never configured or was cleared.
    NotConfigured,
    /// The refresh token was revoked; the user must re-authorize via [`initiate_auth_redirect`].
    Revoked,
    /// Network or API error unrelated to auth.
    Other(String),
}

impl std::fmt::Display for DriveConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Drive sync not configured"),
            Self::Revoked => {
                write!(f, "Google Drive access was revoked — reconnect in Settings")
            }
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

/// Obtains a fresh access token using the stored refresh token. No user interaction, no popup.
///
/// On revocation clears both the refresh token and the drive-enabled flag before returning
/// [`DriveConnectError::Revoked`], so the caller only needs to update the UI state.
pub async fn acquire_token_silent() -> Result<DriveToken, DriveConnectError> {
    let refresh_token = load_refresh_token().ok_or(DriveConnectError::NotConfigured)?;
    token_exchange::refresh_access_token(&refresh_token)
        .await
        .map_err(|e| match e {
            DriveAuthError::Revoked => {
                clear_refresh_token();
                set_drive_enabled(false);
                DriveConnectError::Revoked
            }
            DriveAuthError::Other(msg) => DriveConnectError::Other(msg),
        })
}

/// Returns a valid access token, refreshing silently if the in-memory token has expired.
///
/// Updates `drive_state` on success (new token stored in `Connected`) and on failure
/// (`NeedsReconnect` on revocation, `Error` on network failure). Returns `None` on error
/// so callers can bail out cleanly without pattern matching.
pub async fn acquire_token(mut drive_state: Signal<DriveState>) -> Option<DriveToken> {
    // Return the cached token if it is still valid.
    {
        let state = drive_state.read();
        if let DriveState::Connected { ref token, .. } = *state {
            if !token.is_expired() {
                return Some(token.clone());
            }
        } else {
            return None;
        }
    }

    // Token expired — refresh silently using the stored refresh token.
    match acquire_token_silent().await {
        Ok(new_token) => {
            if let DriveState::Connected { ref mut token, .. } = *drive_state.write() {
                *token = new_token.clone();
            }
            Some(new_token)
        }
        Err(DriveConnectError::Revoked | DriveConnectError::NotConfigured) => {
            drive_state.set(DriveState::NeedsReconnect);
            None
        }
        Err(DriveConnectError::Other(e)) => {
            tracing::error!("Drive token refresh failed: {e}");
            drive_state.set(DriveState::Error(format!("Token refresh failed: {e}")));
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Drive sync helpers (used by app.rs)
// ---------------------------------------------------------------------------

/// Saves `data` to Drive, refreshing the access token silently if it has expired.
///
/// On auth failure transitions the state to `NeedsReconnect`. On non-auth Drive errors
/// logs the failure without changing state (next save will retry).
pub async fn save_to_drive(mut drive_state: Signal<DriveState>, data: &DriveSyncData) {
    // Snapshot the cached file ID without holding the read guard across awaits.
    let file_id = {
        let state = drive_state.read();
        let DriveState::Connected { ref file_id, .. } = *state else {
            return;
        };
        file_id.clone()
    };

    let Some(token) = acquire_token(drive_state).await else {
        return;
    };

    let client = DriveClient::new();
    match client
        .save(&token.access_token, file_id.as_deref(), data)
        .await
    {
        Ok(new_id) => {
            if let DriveState::Connected {
                file_id: ref mut fid,
                ..
            } = *drive_state.write()
            {
                *fid = Some(new_id);
            }
        }
        Err(DriveError::Unauthenticated) => {
            tracing::error!("Drive save rejected with 401");
            drive_state.set(DriveState::NeedsReconnect);
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
/// Called by both the startup silent-auth path and the OAuth callback path.
/// Sets `DriveState::Connected` on completion (whether or not a sync file was found).
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
            drive_state.set(DriveState::Connected {
                token: token.clone(),
                file_id: None,
            });
            return None;
        }
    };

    let Some(ref id) = file_id else {
        drive_state.set(DriveState::Connected {
            token: token.clone(),
            file_id: None,
        });
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
            drive_state.set(DriveState::Connected {
                token: token.clone(),
                file_id: file_id.clone(),
            });
        }
        Err(e) => {
            tracing::error!("Drive read failed: {e}");
            drive_state.set(DriveState::Connected {
                token: token.clone(),
                file_id: file_id.clone(),
            });
        }
    }

    file_id
}

// ---------------------------------------------------------------------------
// Reconnect modal
// ---------------------------------------------------------------------------

/// Full-screen modal shown whenever Drive access has been revoked (`NeedsReconnect` state).
///
/// Renders over whatever page is currently active so the user cannot unknowingly continue
/// with sync silently broken. Both buttons resolve the state: "Reconnect" starts a new
/// OAuth redirect; "Continue without sync" clears all Drive credentials and sets the state
/// to `Disconnected`.
#[component]
pub fn DriveReconnectModal() -> Element {
    let mut drive_state = use_context::<Signal<DriveState>>();

    if !matches!(*drive_state.read(), DriveState::NeedsReconnect) {
        return rsx! {};
    }

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4",
            div { class: "bg-white dark:bg-gray-800 rounded-xl shadow-2xl p-6 max-w-md w-full \
                        border border-gray-200 dark:border-gray-700 \
                        dark:shadow-[0_8px_40px_rgba(0,0,0,0.7)] dark:ring-1 dark:ring-white/[0.07]",
                h2 { class: "text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2",
                    "Google Drive sync disconnected"
                }
                p { class: "text-sm text-gray-600 dark:text-gray-400 mb-6",
                    "Your Google Drive access was revoked. Your local data is safe, but changes \
                     will not sync until you reconnect. Reconnect now, or continue using the app \
                     locally and reconnect later from Settings."
                }
                div { class: "flex flex-col-reverse sm:flex-row gap-3 sm:justify-end",
                    button {
                        r#type: "button",
                        class: "px-4 py-2 text-sm font-medium rounded-md \
                                border border-gray-300 dark:border-gray-600 \
                                text-gray-700 dark:text-gray-300 \
                                hover:bg-gray-100 dark:hover:bg-gray-700",
                        onclick: move |_| {
                            clear_refresh_token();
                            set_drive_enabled(false);
                            drive_state.set(DriveState::Disconnected);
                        },
                        "Continue without sync"
                    }
                    button {
                        r#type: "button",
                        class: "px-4 py-2 text-sm font-medium rounded-md \
                                bg-blue-600 text-white hover:bg-blue-700",
                        onclick: move |_| {
                            initiate_auth_redirect();
                        },
                        "Reconnect"
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Settings UI component
// ---------------------------------------------------------------------------

/// Settings section for configuring Google Drive sync.
///
/// Reads `Signal<DriveState>` from context and provides connect/disconnect controls.
#[component]
pub fn DriveSyncSection() -> Element {
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
        DriveState::NeedsReconnect => rsx! {
            p { class: "text-sm text-yellow-600 dark:text-yellow-400",
                "Google Drive access was revoked. Reconnect to continue syncing."
            }
        },
        DriveState::Error(msg) => {
            let msg = msg.clone();
            rsx! {
                p { class: "text-sm text-red-600 dark:text-red-400", "{msg}" }
            }
        }
    };

    let is_connected = drive_state.read().is_connected();
    let is_connecting = matches!(*drive_state.read(), DriveState::Connecting);
    let needs_reconnect = matches!(
        *drive_state.read(),
        DriveState::NeedsReconnect | DriveState::Error(_)
    );

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
                                let token = load_refresh_token();
                                clear_refresh_token();
                                set_drive_enabled(false);
                                drive_state.set(DriveState::Disconnected);
                                if let Some(t) = token {
                                    spawn(async move {
                                        token_exchange::revoke_token(&t).await;
                                    });
                                }
                            },
                            "Disconnect"
                        }
                    } else {
                        button {
                            r#type: "button",
                            disabled: is_connecting,
                            class: if is_connecting { "px-3 py-1.5 text-sm rounded-md bg-blue-600 text-white opacity-60 cursor-not-allowed" } else { "px-3 py-1.5 text-sm rounded-md bg-blue-600 text-white hover:bg-blue-700" },
                            onclick: move |_| {
                                initiate_auth_redirect();
                            },
                            if needs_reconnect {
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

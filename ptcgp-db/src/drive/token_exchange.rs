//! OAuth 2.0 token exchange and refresh for the PKCE Authorization Code flow.
//!
//! All requests hit Google's token endpoint directly via [`reqwest`]; no GIS script is used.

use chrono::{Duration, Utc};
use reqwest::Client;
use serde::Deserialize;

use super::{CLIENT_ID, CLIENT_SECRET, DriveToken};

const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error returned by token exchange or refresh operations.
#[derive(Debug)]
pub enum DriveAuthError {
    /// The refresh token was rejected (revoked or expired); the user must reconnect.
    Revoked,
    /// Network error, HTTP error, or unexpected response shape.
    Other(String),
}

impl std::fmt::Display for DriveAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Revoked => write!(f, "Google Drive access was revoked"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Response shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Deserialize, Default)]
struct TokenErrorBody {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Exchanges an authorization code for an access token and refresh token.
///
/// The `redirect_uri` must exactly match the one used in the original auth request.
/// Returns `(DriveToken, refresh_token)`. The refresh token should be stored in
/// `localStorage` and used for subsequent silent refreshes.
pub async fn exchange_code(
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<(DriveToken, String), DriveAuthError> {
    let params = [
        ("code", code),
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", verifier),
    ];

    let resp = Client::new()
        .post(TOKEN_ENDPOINT)
        .form(&params)
        .send()
        .await
        .map_err(|e| DriveAuthError::Other(format!("Token exchange request failed: {e}")))?;

    let status = resp.status();
    if !status.is_success() {
        let body: TokenErrorBody = resp.json().await.unwrap_or_default();
        return Err(DriveAuthError::Other(format!(
            "Token exchange HTTP {}: {} — {}",
            status.as_u16(),
            body.error,
            body.error_description
        )));
    }

    let data: TokenResponse = resp
        .json()
        .await
        .map_err(|e| DriveAuthError::Other(format!("Token exchange response parse error: {e}")))?;

    let refresh_token = data.refresh_token.ok_or_else(|| {
        DriveAuthError::Other("Token exchange did not return a refresh token".to_string())
    })?;

    Ok((
        make_token(data.access_token, data.expires_in),
        refresh_token,
    ))
}

/// Silently obtains a new access token using a stored refresh token.
///
/// Returns [`DriveAuthError::Revoked`] when Google rejects the refresh token (400/401),
/// signalling that the user must initiate a new authorization flow.
pub async fn refresh_access_token(refresh_token: &str) -> Result<DriveToken, DriveAuthError> {
    let params = [
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let resp = Client::new()
        .post(TOKEN_ENDPOINT)
        .form(&params)
        .send()
        .await
        .map_err(|e| DriveAuthError::Other(format!("Token refresh request failed: {e}")))?;

    let status = resp.status();
    if status.as_u16() == 400 || status.as_u16() == 401 {
        return Err(DriveAuthError::Revoked);
    }
    if !status.is_success() {
        let body: TokenErrorBody = resp.json().await.unwrap_or_default();
        return Err(DriveAuthError::Other(format!(
            "Token refresh HTTP {}: {} — {}",
            status.as_u16(),
            body.error,
            body.error_description
        )));
    }

    let data: TokenResponse = resp
        .json()
        .await
        .map_err(|e| DriveAuthError::Other(format!("Token refresh response parse error: {e}")))?;

    Ok(make_token(data.access_token, data.expires_in))
}

/// Revokes a refresh token at Google's revocation endpoint.
///
/// Called on disconnect; errors are ignored since local state is cleared regardless.
pub async fn revoke_token(token: &str) {
    let _ = Client::new()
        .post("https://oauth2.googleapis.com/revoke")
        .form(&[("token", token)])
        .send()
        .await;
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn make_token(access_token: String, expires_in: u64) -> DriveToken {
    DriveToken {
        access_token,
        expires_at: Utc::now() + Duration::seconds(expires_in as i64),
    }
}

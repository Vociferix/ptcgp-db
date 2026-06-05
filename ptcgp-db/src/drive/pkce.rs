//! PKCE (RFC 7636) helpers for the OAuth 2.0 Authorization Code flow.
//!
//! Provides code verifier/challenge generation and [`sessionStorage`] persistence across
//! the OAuth redirect round-trip.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

const VERIFIER_KEY: &str = "ptcgp-db-pkce-verifier";
const STATE_KEY: &str = "ptcgp-db-oauth-state";
const RETURN_HASH_KEY: &str = "ptcgp-db-return-hash";

/// Generates a cryptographically random PKCE code verifier.
///
/// The verifier is 64 random bytes encoded as base64url without padding (~86 characters),
/// well within the RFC 7636 length constraints.
pub fn generate_verifier() -> String {
    let mut bytes = [0u8; 64];
    getrandom::getrandom(&mut bytes).expect("WASM crypto RNG unavailable");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Derives the PKCE code challenge from a verifier using the S256 method.
///
/// `challenge = BASE64URL(SHA-256(ASCII(verifier)))`
pub fn derive_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

/// Generates a cryptographically random CSRF state token (16 bytes, base64url).
pub fn generate_state() -> String {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).expect("WASM crypto RNG unavailable");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Saves the PKCE verifier, CSRF state, and return hash to `sessionStorage`.
///
/// All three survive the OAuth redirect and are consumed exactly once by
/// [`take_from_session`].
pub fn save_to_session(verifier: &str, state: &str, return_hash: &str) {
    let Some(session) = session_storage() else {
        return;
    };
    let _ = session.set_item(VERIFIER_KEY, verifier);
    let _ = session.set_item(STATE_KEY, state);
    let _ = session.set_item(RETURN_HASH_KEY, return_hash);
}

/// Retrieves and removes the PKCE verifier, CSRF state, and return hash from `sessionStorage`.
///
/// Returns `None` when the session data is absent (not an OAuth callback, or already consumed).
/// Returns `(verifier, state, return_hash)` on success. `return_hash` defaults to `""`.
pub fn take_from_session() -> Option<(String, String, String)> {
    let session = session_storage()?;
    let verifier = session.get_item(VERIFIER_KEY).ok().flatten()?;
    let state = session.get_item(STATE_KEY).ok().flatten()?;
    let return_hash = session
        .get_item(RETURN_HASH_KEY)
        .ok()
        .flatten()
        .unwrap_or_default();
    let _ = session.remove_item(VERIFIER_KEY);
    let _ = session.remove_item(STATE_KEY);
    let _ = session.remove_item(RETURN_HASH_KEY);
    Some((verifier, state, return_hash))
}

/// Returns `true` when a PKCE session is in progress (an OAuth redirect is pending).
///
/// Used on page load to detect whether this page load is returning from an auth redirect
/// even when the Drive-enabled flag has not been set yet (first-time connect).
pub fn has_session_data() -> bool {
    session_storage()
        .and_then(|s| s.get_item(VERIFIER_KEY).ok().flatten())
        .is_some()
}

fn session_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.session_storage().ok().flatten()
}

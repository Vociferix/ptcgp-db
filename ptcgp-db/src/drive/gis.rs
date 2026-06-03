//! Bindings to the Google Identity Services (GIS) JavaScript library.
//!
//! GIS uses a callback-based API. The functions here bridge it to Rust `async`/`await`
//! using [`futures_channel::oneshot`] channels.

use std::cell::RefCell;
use std::rc::Rc;

use futures_channel::oneshot;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Inline JS: script loader + token-request bridge
// ---------------------------------------------------------------------------

#[wasm_bindgen(inline_js = r#"
// Loads the GIS script if not already present, then calls callback(true) on success
// or callback(false) on failure. If the script is already loaded, calls back immediately.
export function gis_load(callback) {
    if (typeof window !== 'undefined'
        && typeof window.google !== 'undefined'
        && typeof window.google.accounts !== 'undefined') {
        callback(true);
        return;
    }
    if (document.getElementById('ptcgp-gis-script')) {
        const poll = setInterval(() => {
            if (typeof window.google !== 'undefined'
                && typeof window.google.accounts !== 'undefined') {
                clearInterval(poll);
                callback(true);
            }
        }, 50);
        return;
    }
    const s = document.createElement('script');
    s.id = 'ptcgp-gis-script';
    s.src = 'https://accounts.google.com/gsi/client';
    s.onerror = () => callback(false);
    s.onload  = () => callback(true);
    document.head.appendChild(s);
}

// Requests a GIS access token. `prompt` is "" for silent or "select_account" for interactive.
// `callback` is called with the raw TokenResponse or error object from GIS.
export function gis_request_token(client_id, scope, prompt, callback) {
    window.google.accounts.oauth2.initTokenClient({
        client_id: client_id,
        scope: scope,
        callback: callback,
        error_callback: callback,
    }).requestAccessToken({ prompt: prompt });
}
"#)]
extern "C" {
    fn gis_load(callback: &js_sys::Function);
    fn gis_request_token(client_id: &str, scope: &str, prompt: &str, callback: &js_sys::Function);
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parsed fields from a successful GIS token grant.
#[derive(Debug, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    /// Lifetime of the token in seconds (typically 3600).
    pub expires_in: u32,
}

/// Loads the GIS script and waits until `window.google.accounts` is available.
///
/// Returns `Err` if the script fails to download.
pub async fn ensure_gis_loaded() -> Result<(), String> {
    let (tx, rx) = oneshot::channel::<bool>();
    let tx = Rc::new(RefCell::new(Some(tx)));
    let callback = Closure::<dyn FnMut(bool)>::new(move |ok: bool| {
        if let Some(tx) = tx.borrow_mut().take() {
            let _ = tx.send(ok);
        }
    });
    gis_load(callback.as_ref().unchecked_ref());
    callback.forget();
    match rx.await {
        Ok(true) => Ok(()),
        Ok(false) => Err("Failed to load the Google Identity Services script".to_string()),
        Err(_) => Err("GIS script-load channel closed unexpectedly".to_string()),
    }
}

/// Requests a Google OAuth2 access token via GIS.
///
/// `prompt` controls user interaction:
/// - `""` — silent (no popup; fails if no active session or scope not yet granted)
/// - `"select_account"` — shows the account chooser and consent popup
pub async fn request_token(
    client_id: &str,
    scope: &str,
    prompt: &str,
) -> Result<TokenResponse, String> {
    let (tx, rx) = oneshot::channel::<Result<TokenResponse, String>>();
    let tx = Rc::new(RefCell::new(Some(tx)));
    let callback = Closure::<dyn FnMut(JsValue)>::new(move |response: JsValue| {
        if let Some(tx) = tx.borrow_mut().take() {
            let _ = tx.send(parse_token_response(response));
        }
    });
    gis_request_token(client_id, scope, prompt, callback.as_ref().unchecked_ref());
    callback.forget();
    rx.await
        .map_err(|_| "GIS token-request channel closed unexpectedly".to_string())?
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Parses a raw GIS callback value into [`TokenResponse`] or an error string.
///
/// GIS fires the same callback for both success (`access_token` present) and failure
/// (`error` or `type` present), and also for `error_callback` errors.
fn parse_token_response(val: JsValue) -> Result<TokenResponse, String> {
    let obj = js_sys::Object::from(val);
    let get = |key: &str| {
        js_sys::Reflect::get(&obj, &JsValue::from_str(key))
            .ok()
            .and_then(|v| v.as_string())
    };
    if let Some(token) = get("access_token") {
        let expires_in = js_sys::Reflect::get(&obj, &JsValue::from_str("expires_in"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(3600.0) as u32;
        return Ok(TokenResponse { access_token: token, expires_in });
    }
    let error = get("error")
        .or_else(|| get("type"))
        .unwrap_or_else(|| "unknown_token_error".to_string());
    Err(error)
}

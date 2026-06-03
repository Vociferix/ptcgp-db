# Google Drive OAuth Setup

Reference for integrating Google Drive sync into the ptcgp-db app.

## Overview

- Auth flow: Google Identity Services (GIS) JS library, via `wasm-bindgen` interop in the Dioxus web build
- Drive storage: `appDataFolder` scope — a hidden, app-specific folder invisible to the user in their normal Drive view
- Drive API calls: raw `fetch` against the Drive REST API v3 (no Rust crate; `google-drive3` is server-side only)
- Sync strategy: read one JSON file on app load, write on save (debounced, same as the existing auto-save)
- Conflict resolution: last write wins — acceptable for card collection data

## Rust crate landscape (as of 2026-06)

No WASM-compatible Rust crates exist for GIS or the Drive API. `reqwest` (0.12) is used for Drive REST calls (works on WASM via the browser Fetch API). GIS is bridged via `wasm-bindgen` `inline_js` — see `ptcgp-db/src/drive/gis.rs`. Desktop OAuth is a separate, more complex problem (system browser + localhost loopback redirect server); deferred.

## Implementation

Drive sync lives in `ptcgp-db/src/drive/` (WASM-only, gated with `#[cfg(target_arch = "wasm32")] mod drive` in `main.rs`):

| File | Contents |
|---|---|
| `drive/mod.rs` | `DriveState`, `DriveToken`, `DriveSyncData`, localStorage helpers, token acquisition, `save_to_drive`, `DriveSyncSection` UI component |
| `drive/gis.rs` | GIS JS interop via `wasm_bindgen(inline_js)` + `futures_channel::oneshot` bridges |
| `drive/client.rs` | `DriveClient` — find, read, create, update a single `appDataFolder` file via Drive REST v3 |

`DriveState` signal is provided at the app root. The auto-save coroutine in `app.rs` also writes to Drive when connected. On startup, a silent token acquisition attempt runs if `localStorage` has the `ptcgp-db-drive-connected` flag set.

## Google Cloud Console setup

Start at **https://console.cloud.google.com/**

1. Create a new project (or reuse an existing one)
2. **APIs & Services → Library** — search "Google Drive API" and enable it
3. **APIs & Services → OAuth consent screen**
   - Set app name and support email
   - Add scope: `https://www.googleapis.com/auth/drive.appdata`
   - In "Testing" mode, up to 100 manually whitelisted accounts can use the app without verification
   - To allow any Google account (public), submit for Google's verification review (takes a few days; requires a privacy policy URL)
4. **APIs & Services → Credentials → Create Credentials → OAuth 2.0 Client ID**
   - Application type: **Web application**
   - Authorized JavaScript origins: `https://vociferix.github.io` (scheme + host only; no path)
   - Redirect URIs: not needed for the GIS popup/implicit flow

## Authorized JavaScript origin note

The origin is scheme + host only — paths are excluded regardless of where the app is mounted. `https://vociferix.github.io` covers the app whether it's at `/ptcgp-db/` or any other subpath.

## GIS library reference

- Migration guide: https://developers.google.com/identity/oauth2/web/guides/migration-to-gis
- Client ID setup: https://developers.google.com/identity/oauth2/web/guides/get-google-api-clientid
- Drive REST API v3: https://developers.google.com/drive/api/v3/reference

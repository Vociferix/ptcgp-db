# Google Drive OAuth — PKCE Authorization Code Flow

Reference for the Google Drive sync implementation. This document covers the new
Authorization Code + PKCE flow that replaced the original GIS implicit token flow.

---

## Why the implicit flow was replaced

The original implementation used GIS's `initTokenClient` (OAuth 2.0 implicit grant). This caused
three browser-level problems:

1. **Hourly popup.** Access tokens expire after one hour. The implicit flow has no refresh tokens,
   so re-authentication always requires user interaction. GIS's `prompt: ""` ("silent") mode
   attempts a hidden-iframe approach but falls back to a brief visible popup when third-party
   cookies are blocked — which they are in current Chrome and Firefox.

2. **Popup on every page reload.** `startup_drive_sync` called `acquire_token_silent()` on startup,
   triggering a GIS popup before the user had done anything.

3. **Popup blocked on first-time setup.** GIS's `requestAccessToken()` opens a `window.open()`
   popup. Browsers tie popup permission to "user activation" (a direct click). Because our onclick
   handler used `spawn(async move { ... })` followed by awaits, the user gesture was expired by the
   time the popup was opened, so Chrome and Firefox blocked it.

The root cause of all three is that the implicit grant flow is fundamentally popup-based and
provides no refresh mechanism. The Authorization Code + PKCE flow eliminates all three:

- Initial auth: full-page redirect instead of a popup (browsers never block navigation)
- Page reload: exchange the stored refresh token for a new access token via a background fetch —
  no popup, no redirect
- Token expiry during a session: same background fetch — invisible to the user

The GIS JavaScript library is no longer used at all. Auth is initiated by constructing a raw OAuth
URL and navigating to it; token exchange and refresh are plain HTTP requests via `reqwest`.

---

## Overview

| Concern | Mechanism |
|---|---|
| Auth initiation | Redirect to `accounts.google.com/o/oauth2/v2/auth` with PKCE params |
| Code → token exchange | POST to `oauth2.googleapis.com/token` (one-time, on first connect) |
| Token refresh | POST to `oauth2.googleapis.com/token` with `grant_type=refresh_token` |
| Refresh token storage | `localStorage` under `ptcgp-db-drive-refresh-token` |
| Access token storage | In-memory `DriveToken` (not persisted) |
| CSRF protection | `state` parameter: random token stored in `sessionStorage` across redirect |
| PKCE binding | `code_verifier` stored in `sessionStorage` across redirect |

---

## Flows

### Initial connection (user clicks "Connect Google Drive")

```
App                               Browser              Google
 |                                    |                    |
 |-- generate PKCE verifier+challenge |                    |
 |-- generate random state token      |                    |
 |-- sessionStorage.set(verifier, state, return_hash)      |
 |-- location.href = auth URL ------->|                    |
 |                                    |--- navigate ------>|
 |                                    |<-- consent page ---|
 |                                    |--- user consents ->|
 |                                    |<-- redirect -------|
 |<-- App() renders with ?code=... ---|                    |
 |-- read code, state from URL        |                    |
 |-- verify state matches sessionStorage                   |
 |-- read verifier from sessionStorage                     |
 |-- POST /token (code + verifier) ---|--- fetch -------->|
 |<-- { access_token, refresh_token } |<-- JSON ----------|
 |-- localStorage.set(refresh_token)  |                    |
 |-- history.replaceState(clean URL)  |                    |
 |-- load_from_drive()                |                    |
```

### Page load (Drive was previously connected)

```
App                               Browser              Google
 |                                    |                    |
 |-- check localStorage for refresh_token                  |
 |-- POST /token (refresh_token) -----|--- fetch -------->|
 |<-- { access_token, expires_in } ---|<-- JSON ----------|
 |-- load_from_drive()                |                    |
```

No popup. No redirect. One background HTTP request.

### Token expiry during a session

Same as "page load" above: when `save_to_drive` detects an expired access token, it POSTs to
the token endpoint using the stored refresh token and gets a new access token silently.

### Disconnection

1. Remove `ptcgp-db-drive-refresh-token` from `localStorage`
2. Remove `ptcgp-db-drive-connected` from `localStorage`
3. POST to `https://oauth2.googleapis.com/revoke?token=<refresh_token>` (best-effort; ignore errors)
4. Set `DriveState::Disconnected`

---

## PKCE details

PKCE (RFC 7636) prevents authorization code interception. For public clients (no client secret)
it also provides the binding that the token exchange can only be completed by the same party that
initiated the request.

```
code_verifier  = 64 random bytes, base64url-encoded (no padding) → ~86-character string
code_challenge = base64url(SHA-256(ASCII(code_verifier)))         → 43-character string
code_challenge_method = "S256"
```

The verifier is generated in Rust using `getrandom` for randomness and `sha2` for the hash.
It is stored in `sessionStorage` (survives the redirect, not persisted across browser sessions).

---

## Auth URL construction

GIS's `initCodeClient` does not expose `code_challenge` / `code_challenge_method` parameters, so
the auth URL is constructed manually and navigation is initiated via `web_sys::Location::set_href`:

```
https://accounts.google.com/o/oauth2/v2/auth
  ?response_type=code
  &client_id=<CLIENT_ID>
  &redirect_uri=<window.location.origin + window.location.pathname>
  &scope=https://www.googleapis.com/auth/drive.appdata
  &code_challenge=<challenge>
  &code_challenge_method=S256
  &state=<csrf_token>
  &access_type=offline          ← required for refresh_token
  &prompt=consent               ← required on first connect to guarantee refresh_token is returned
```

`access_type=offline` and `prompt=consent` are both required on the initial connection to ensure
Google returns a refresh token. On subsequent silent refreshes these parameters are not sent.

The redirect URI is derived at runtime from `window.location.origin + window.location.pathname`
(no hash, no query string). This correctly resolves to the app's base URL in both production
(`https://vociferix.github.io/ptcgp-db/`) and local dev (`http://localhost:PORT/`).

---

## Token exchange

After Google redirects back with `?code=...&state=...` in the URL:

```
POST https://oauth2.googleapis.com/token
Content-Type: application/x-www-form-urlencoded

code=<authorization_code>
&client_id=<CLIENT_ID>
&redirect_uri=<same URI used during auth request>
&grant_type=authorization_code
&code_verifier=<verifier from sessionStorage>
```

Response:
```json
{
  "access_token": "ya29...",
  "expires_in": 3600,
  "refresh_token": "1//...",
  "scope": "https://www.googleapis.com/auth/drive.appdata",
  "token_type": "Bearer"
}
```

The refresh token is stored in `localStorage`. The access token is held in memory as a `DriveToken`
with a computed `expires_at` timestamp.

### Silent refresh

```
POST https://oauth2.googleapis.com/token
Content-Type: application/x-www-form-urlencoded

client_id=<CLIENT_ID>
&grant_type=refresh_token
&refresh_token=<stored refresh_token>
```

Response is the same shape but without `refresh_token`. Google does not rotate refresh tokens on
this endpoint; the original token remains valid.

---

## Handling the redirect on page load

Google redirects back to the app's base URL, appending `?code=...&scope=...&state=...` as query
params. Because the app uses hash routing, the Dioxus router is unaffected — it only sees the
fragment. The query params are read by the startup Drive sync before the router runs.

Steps in `startup_drive_sync` (in `app.rs`):

1. Check `window.location.search` for a `code` param.
2. If present: verify `state` against `sessionStorage`, retrieve `code_verifier`, exchange code
   for tokens, store refresh token, call `history.replaceState` to strip the params from the URL,
   then restore the pre-redirect hash from `sessionStorage`.
3. If not present and `ptcgp-db-drive-refresh-token` exists in `localStorage`: silent refresh.
4. If not present and no refresh token: Drive was connected but the refresh token is missing
   (cleared manually or revoked) → set `DriveState::NeedsReconnect`.
5. After acquiring a valid access token via either path: call `load_from_drive`.

---

## Return-route preservation

Before redirecting to Google, the current URL hash is saved to `sessionStorage` alongside the
PKCE verifier. After the redirect is processed, `history.replaceState` restores the hash so the
user lands on the same page they were on when they initiated the connection.

If the user initiated connection from the onboarding screen, the hash is typically `#/` or empty;
restoring it leaves them on onboarding, which will exit naturally once `ProfileStore` is loaded
with data from Drive.

---

## DriveState changes

```rust
pub enum DriveState {
    Disconnected,
    Connecting,                                          // startup refresh in progress
    Connected { token: DriveToken, file_id: Option<String> },
    NeedsReconnect,                                      // refresh token rejected (revoked)
    Error(String),                                       // non-auth Drive operation failure
}
```

`NeedsReconnect` is set when a token refresh POST returns 400/401. The settings UI shows
"Reconnect" in this state, which re-initiates the full redirect flow.

---

## Architecture changes

### Files deleted
- `ptcgp-db/src/drive/gis.rs` — GIS is no longer used

### Files added
- `ptcgp-db/src/drive/pkce.rs` — PKCE verifier/challenge generation; `sessionStorage` helpers
- `ptcgp-db/src/drive/token_exchange.rs` — code exchange, silent refresh, revocation

### Files modified
- `ptcgp-db/src/drive/mod.rs`
  - Remove `acquire_token_silent` / `acquire_token_interactive` / `acquire_token`
  - Remove `gis` module reference
  - Add `pkce` and `token_exchange` module references
  - Add `initiate_auth_redirect(return_hash: &str)` — generates PKCE + state, saves to
    `sessionStorage`, sets `window.location.href`
  - Add `handle_auth_callback() -> Result<DriveToken, ...>` — reads code+state from URL,
    verifies, exchanges, stores refresh token, cleans URL
  - Add `acquire_token(drive_state) -> Result<DriveToken, ...>` — uses in-memory token if valid,
    otherwise calls silent refresh; sets `NeedsReconnect` on refresh failure
  - Add `NeedsReconnect` variant to `DriveState` enum
  - Update `save_to_drive` — call new `acquire_token` instead of `acquire_token_silent`
  - Update `onboarding_connect_drive` — call `initiate_auth_redirect` instead of token client
  - Update `DriveSyncSection` — handle `NeedsReconnect` state, update connect button handler
    to call `initiate_auth_redirect` directly (no `spawn` needed; it's synchronous)
  - Update `connect_drive` — call `initiate_auth_redirect`
  - Add `REFRESH_TOKEN_KEY` localStorage constant

- `ptcgp-db/src/app.rs`
  - Update `startup_drive_sync` — check for OAuth callback in URL first, then fall back to
    silent refresh; drop the `acquire_token_silent` call

### New localStorage keys

| Key | Purpose |
|---|---|
| `ptcgp-db-drive-connected` | Exists when Drive is enabled (unchanged) |
| `ptcgp-db-drive-refresh-token` | Stored refresh token (new) |

### New sessionStorage keys (transient, exist only during redirect round-trip)

| Key | Purpose |
|---|---|
| `ptcgp-db-pkce-verifier` | PKCE code verifier |
| `ptcgp-db-oauth-state` | CSRF state token |
| `ptcgp-db-return-hash` | URL hash to restore after redirect |

---

## New dependencies

| Crate | Version constraint | Purpose |
|---|---|---|
| `sha2` | `>=0.10` | SHA-256 for PKCE code challenge |
| `base64` | `>=0.22` | base64url encoding for verifier and challenge |

Both compile to WASM and are well within the project's dependency policy (≥ 100k downloads,
recently updated). `getrandom` is already in the tree (pulled transitively); no new feature flags
are needed for WASM because `getrandom` already uses the `js` backend.

---

## Google Cloud Console changes required

The OAuth client in the Cloud Console (`353554631088-...`) needs **redirect URIs** added. These
were not set before because the popup/implicit flow does not use them.

Go to **APIs & Services → Credentials → [the existing OAuth client] → Edit**:

Under **Authorized redirect URIs**, add:
- `https://vociferix.github.io/ptcgp-db/` (production)
- `http://localhost:8080/` (or whichever port `dx serve` binds; add more ports as needed)

The JavaScript origins entries (`https://vociferix.github.io`) remain. They are no longer required
for the auth flow itself (no GIS script is loaded) but are kept for documentation purposes and in
case GIS is needed again.

---

## Security considerations

**Refresh token in `localStorage`.** The token is sensitive: anyone who can read `localStorage`
for the app's origin can obtain Drive access (scoped to `appDataFolder` only — not the user's
full Drive). Threats:

- *XSS on the app origin*: would expose the token. Mitigation: the app has no user-provided HTML
  rendering paths; the only dynamic content is card collection data.
- *Other scripts on the same origin*: GitHub Pages serves each repo under a unique subdomain path
  (same origin as other Pages repos on `github.io`). This is a known limitation of GitHub Pages;
  it is the same risk as storing any credentials there.
- *Encrypting the token in localStorage*: provides no meaningful protection. Any XSS that can
  read the token can also read or intercept the encryption key. The false sense of security is
  worse than documenting the known risk.

The `drive.appdata` scope limits blast radius: an attacker gets read/write access to a 
`ptcgp-db-sync.json` file only, not the user's full Drive.

**PKCE.** Prevents authorization code interception attacks. Without PKCE, a malicious page on
the same origin could potentially steal the auth code from the redirect URL before our code
processes it. The code verifier in `sessionStorage` ensures only our page can complete the exchange.

**State parameter.** Prevents CSRF: a malicious site cannot initiate a Drive connection on behalf
of the user, because it cannot generate a state token that matches what is in our `sessionStorage`.

---

## Implementation order

1. Add `sha2` and `base64` to `ptcgp-db/Cargo.toml`
2. Write `drive/pkce.rs` — verifier generation, challenge derivation, sessionStorage helpers
3. Write `drive/token_exchange.rs` — `exchange_code`, `refresh_access_token`, `revoke_token`
4. Update `DriveState` in `drive/mod.rs` — add `NeedsReconnect`, update `is_connected`
5. Add `initiate_auth_redirect` and `handle_auth_callback` to `drive/mod.rs`
6. Rewrite `acquire_token` in `drive/mod.rs` to use refresh tokens
7. Update `save_to_drive` and `load_from_drive` call sites
8. Update `startup_drive_sync` in `app.rs` — callback detection first, then silent refresh
9. Update `DriveSyncSection` — new state display for `NeedsReconnect`, synchronous connect button
10. Update `onboarding_connect_drive` and `connect_drive` — replace with redirect initiation
11. Delete `drive/gis.rs`
12. Update Google Cloud Console redirect URIs
13. Manual end-to-end test: fresh connect, page reload, disconnect, reconnect

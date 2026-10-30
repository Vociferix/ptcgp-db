# Image handling and the local image cache (PR #73)

## Critical correction to old knowledge

Images are **NOT bundled via `asset!()`** — DESIGN.md said so and was stale. The only `asset!()`
in the app is `tailwind.css`. All images are absolute **jsDelivr URLs** built at codegen time
(`ptcgp-db-data/codegen/generate.rs`, `JSDELIVR_BASE`, pinned to a `ptcgp-images` tag), returned
as `&'static str` from `CardVersion::image()`, `Set::icon()`, etc. DESIGN.md is now corrected.

Verified CDN headers: `cache-control: public, max-age=31536000, immutable`,
`access-control-allow-origin: *`, card images ~273 KB, 3,289 cards (>1 GB total).
The user's symptom (images never all local) is **HTTP cache eviction**, not bad headers.

## Key API fact

`CacheStorage` is **not** service-worker-only — `caches.open()` works in window scope, so a
durable image cache needs **no JavaScript**. A service worker would only add transparent
interception of `<img>` loads. The user explicitly preferred a Rust refactor over a JS worker.

Deployment is GitHub Pages under base path `/ptcgp-db/`; `ptcgp-db/public/*` is copied verbatim
to the web root (so a service worker at `public/sw.js` → `/ptcgp-db/sw.js` would have the right
scope, if ever needed). Hash routing means the document URL never changes, so relative
registration would work in both dev and prod.

## Implementation

- `ptcgp-db/src/image_cache.rs` — Cache Storage layer + `ImageCache` (Copy handle:
  `ready: Signal<bool>` read reactively, `inner: Signal<CacheInner>` accessed only via
  `peek()`/`write()` so one image resolving does not re-render all the others). Platform layer is
  no-ops off wasm; the `mod image_cache;` decl in main.rs carries
  `#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]`.
- `ptcgp-db/src/components/cached_image.rs` — `CachedImage` replaces **every** `img` in the app
  (37 sites, 7 files). Raw `img` for a ptcgp-images URL is now a bug (bypasses the cache).
  Miss → render CDN URL + background store. Hit → transparent 1×1 GIF placeholder during the
  disk read, then object URL. Decision is made in the `use_signal` initializer, NOT an effect —
  an effect runs after first paint, by which point the browser already started the network fetch.
- Object URLs capped at 192, oldest-first eviction (each pins its blob, ~50 MB ceiling).
- Fetched with `RequestMode::Cors` so responses are non-opaque (opaque ones get ~7 MB quota
  padding).
- Cache name embeds the image-set version parsed at runtime from a sample URL; startup
  `prune_stale()` deletes any `ptcgp-images-*` cache that is not current.
- `cache_images: bool` on `AppSettingsSaveData` with `#[serde(default)]` — no format bump needed.

## Gotchas hit (do not regress)

- Re-enabling after disable must rescan: the scan claim (`begin_scan`) lives in `CacheInner` and
  is cleared by `reset()`. A `use_signal` guard in `App` got this wrong.
- Disabling revokes object URLs, so `CachedImage`'s effect must set `Source::Remote` when the
  setting goes off, or mounted images show broken blobs (render does not subscribe to settings).
- `AppSettingsSaveData` is built as a struct literal in 3 test fixtures
  (`file_storage.rs`, `migration.rs` ×2) — adding a field breaks them.

## Outstanding

User's uncommitted `ptcgp-db-data/codegen/generate.rs:40-41` emits `pub const X: &'static str`,
which fails `cargo clippy --workspace -- -D warnings` (`redundant_static_lifetimes`). Their crate,
so not fixed by Claude. Will break CI when committed. Fix: drop `'static`.

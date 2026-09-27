//! Durable local cache for the app's remote images.
//!
//! Every image the UI renders is served from a version-pinned jsDelivr URL. The CDN already
//! sends `immutable` with a one-year max-age, but the browser's HTTP cache is a shared,
//! size-limited pool: with thousands of card images at ~270 KB each, entries are evicted long
//! before the collection is fully local. This module moves images into the Cache Storage API
//! instead, which is quota-backed rather than LRU-evicted, and asks for persistent storage so
//! the browser will not reclaim it.
//!
//! [`ImageCache`] is the app-wide handle, provided as a context by the root component and read
//! by [`CachedImage`](crate::components::CachedImage). The platform functions below are
//! no-ops off the web, so the same component code compiles for desktop.
//!
//! The cache name embeds the image-set version, so publishing a build that points at a newer
//! jsDelivr tag starts a fresh cache; [`prune_stale`] then deletes the superseded ones.

use std::collections::{HashMap, HashSet, VecDeque};

use dioxus::prelude::*;

/// Upper bound on object URLs held in memory at once.
///
/// Each one pins its image blob in memory, so this caps resident image data at roughly
/// `MAX_RESOLVED × 270 KB` (~50 MB). It only needs to comfortably exceed the number of images
/// on screen at once; eviction is oldest-first, and anything evicted is simply re-read from
/// disk the next time it is displayed.
const MAX_RESOLVED: usize = 192;

/// Prefix shared by every cache this module creates, so stale ones are identifiable.
const CACHE_PREFIX: &str = "ptcgp-images-";

/// A transparent 1×1 GIF, used as the `src` of an image whose bytes are still being read from
/// the cache. Keeping the element an `<img>` with its original classes preserves layout and
/// avoids the broken-image glyph a missing `src` would produce.
pub const PLACEHOLDER_SRC: &str =
    "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

/// Version tag of the image set the data crate currently points at, e.g. `"v0.11.0"`.
///
/// Parsed from a sample image URL rather than hardcoded, so it tracks the data crate without
/// `ptcgp-db-data` needing to expose it. Returns `"unknown"` if the URL shape ever changes,
/// which degrades to a single stable cache name rather than failing.
pub fn image_set_version() -> &'static str {
    ptcgp_db_data::CardVersion::ALL
        .first()
        .and_then(|cv| cv.image().split_once('@'))
        .and_then(|(_, rest)| rest.split('/').next())
        .unwrap_or("unknown")
}

/// Name of the Cache Storage entry holding the current image set.
fn cache_name() -> String {
    format!("{CACHE_PREFIX}{}", image_set_version())
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// Session view of the durable cache. Not read reactively — see [`ImageCache`].
#[derive(Default)]
struct CacheInner {
    /// URLs known to be present in Cache Storage, read once at startup.
    known: HashSet<String>,
    /// Object URLs for images already read back from disk this session.
    resolved: HashMap<String, String>,
    /// Insertion order of `resolved`, for oldest-first eviction.
    order: VecDeque<String>,
    /// Whether the one-time startup scan has been claimed.
    scanning: bool,
}

impl CacheInner {
    /// Records an object URL for `url` and returns the one that should be displayed.
    ///
    /// Each mounted image reads its own bytes back, so a second object URL can arrive for a URL
    /// that already has one. The duplicate is released and the established URL returned —
    /// returning the released one would leave the caller displaying a revoked blob, which never
    /// loads.
    fn insert_resolved(&mut self, url: &str, object_url: String) -> String {
        if let Some(existing) = self.resolved.get(url) {
            revoke(&object_url);
            return existing.clone();
        }
        self.resolved.insert(url.to_string(), object_url.clone());
        self.order.push_back(url.to_string());
        while self.order.len() > MAX_RESOLVED {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(stale) = self.resolved.remove(&oldest) {
                revoke(&stale);
            }
        }
        object_url
    }

    /// Drops any object URL held for `url` and stops treating it as cached.
    fn invalidate(&mut self, url: &str) {
        if let Some(object_url) = self.resolved.remove(url) {
            revoke(&object_url);
        }
        self.order.retain(|held| held != url);
        self.known.remove(url);
    }

    /// Releases every object URL and drops all session state.
    fn clear(&mut self) {
        for (_, object_url) in self.resolved.drain() {
            revoke(&object_url);
        }
        self.order.clear();
        self.known.clear();
        // Cleared so re-enabling the setting scans again rather than staying inert.
        self.scanning = false;
    }
}

/// App-wide handle to the image cache.
///
/// `ready` is the only field read reactively: it flips once at startup when the set of cached
/// URLs has been loaded, and components re-render then. Everything else is accessed without
/// subscribing, so one image resolving does not re-render every other image on screen.
#[derive(Clone, Copy)]
pub struct ImageCache {
    ready: Signal<bool>,
    persistent: Signal<bool>,
    inner: Signal<CacheInner>,
}

impl ImageCache {
    /// Creates the handle. Call once, from `use_context_provider`.
    ///
    /// Both signals are owned by [`ScopeId::ROOT`] rather than the calling component. The
    /// background store task runs in the root scope so it survives the image unmounting, and a
    /// signal owned by a deeper scope cannot legally be written from there — the root scope is
    /// an ancestor of the owner, not a descendant. Owning them at the root also makes the
    /// lifetime genuinely correct: the root outlives every scope that touches them.
    pub fn new() -> Self {
        Self {
            ready: Signal::new_in_scope(false, ScopeId::ROOT),
            persistent: Signal::new_in_scope(false, ScopeId::ROOT),
            inner: Signal::new_in_scope(CacheInner::default(), ScopeId::ROOT),
        }
    }

    /// Whether the browser granted persistent storage, so cached images are protected from
    /// being reclaimed. Reading this subscribes the caller.
    ///
    /// `false` does **not** mean caching is broken: the cache still works, it is just
    /// best-effort and the browser may clear it when disk space runs low.
    pub fn is_persistent(&self) -> bool {
        *self.persistent.read()
    }

    /// Records whether persistent storage was granted.
    pub fn set_persistent(&mut self, granted: bool) {
        self.persistent.set(granted);
    }

    /// Whether the cached-URL set has loaded. Reading this subscribes the caller.
    pub fn ready(&self) -> bool {
        *self.ready.read()
    }

    /// Claims the one-time scan of Cache Storage.
    ///
    /// Returns `false` when a scan is already running, so the caller does nothing.
    pub fn begin_scan(&mut self) -> bool {
        let mut inner = self.inner.write();
        if inner.scanning {
            return false;
        }
        inner.scanning = true;
        true
    }

    /// Records the URLs found in Cache Storage and marks the cache ready.
    pub fn set_known(&mut self, known: HashSet<String>) {
        let mut inner = self.inner.write();
        inner.known = known;
        inner.scanning = false;
        drop(inner);
        self.ready.set(true);
    }

    /// Object URL for `url` if it has already been read back this session.
    pub fn resolved(&self, url: &str) -> Option<String> {
        self.inner.peek().resolved.get(url).cloned()
    }

    /// Whether `url` is present in Cache Storage.
    pub fn is_known(&self, url: &str) -> bool {
        self.inner.peek().known.contains(url)
    }

    /// Notes that `url` is now stored in Cache Storage.
    pub fn mark_known(&mut self, url: &str) {
        self.inner.write().known.insert(url.to_string());
    }

    /// Forgets `url`, after a read found nothing, so it falls back to the network.
    pub fn forget(&mut self, url: &str) {
        self.inner.write().known.remove(url);
    }

    /// Discards everything held for `url` — its object URL and its cached status.
    ///
    /// Used when a displayed object URL turns out to be dead: releasing it stops any other image
    /// reusing it, and `url` falls back to the network.
    pub fn invalidate(&mut self, url: &str) {
        self.inner.write().invalidate(url);
    }

    /// Stores an object URL for `url`, evicting the oldest entries past [`MAX_RESOLVED`].
    ///
    /// Returns the object URL to display, which is not necessarily the one passed in — see
    /// [`CacheInner::insert_resolved`].
    pub fn insert_resolved(&mut self, url: &str, object_url: String) -> String {
        self.inner.write().insert_resolved(url, object_url)
    }

    /// Drops all session state and releases every object URL.
    ///
    /// Does not touch Cache Storage — see [`delete_all`] for that.
    pub fn reset(&mut self) {
        self.inner.write().clear();
        self.ready.set(false);
        self.persistent.set(false);
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Web implementation
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod platform {
    use super::{CACHE_PREFIX, cache_name};
    use std::collections::HashSet;

    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Blob, Cache, CacheStorage, Request, RequestInit, RequestMode, Response, Url};

    fn cache_storage() -> Option<CacheStorage> {
        web_sys::window()?.caches().ok()
    }

    async fn open() -> Option<Cache> {
        let storage = cache_storage()?;
        JsFuture::from(storage.open(&cache_name()))
            .await
            .ok()?
            .dyn_into::<Cache>()
            .ok()
    }

    /// Deletes caches belonging to superseded image-set versions.
    pub async fn prune_stale() {
        let Some(storage) = cache_storage() else {
            return;
        };
        let Ok(keys) = JsFuture::from(storage.keys()).await else {
            return;
        };
        let Ok(names) = keys.dyn_into::<js_sys::Array>() else {
            return;
        };
        let current = cache_name();
        for name in names.iter() {
            if let Some(name) = name.as_string()
                && name.starts_with(CACHE_PREFIX)
                && name != current
            {
                let _ = JsFuture::from(storage.delete(&name)).await;
            }
        }
    }

    /// Every URL currently stored in the cache.
    pub async fn cached_urls() -> HashSet<String> {
        let mut urls = HashSet::new();
        let Some(cache) = open().await else {
            return urls;
        };
        let Ok(keys) = JsFuture::from(cache.keys()).await else {
            return urls;
        };
        let Ok(requests) = keys.dyn_into::<js_sys::Array>() else {
            return urls;
        };
        for request in requests.iter() {
            if let Ok(request) = request.dyn_into::<Request>() {
                urls.insert(request.url());
            }
        }
        urls
    }

    /// Reads `url` back from the cache as an object URL the DOM can display.
    pub async fn object_url(url: &str) -> Option<String> {
        let cache = open().await?;
        let found = JsFuture::from(cache.match_with_str(url)).await.ok()?;
        // A miss resolves to `undefined`, which fails this cast.
        let response = found.dyn_into::<Response>().ok()?;
        let blob = JsFuture::from(response.blob().ok()?)
            .await
            .ok()?
            .dyn_into::<Blob>()
            .ok()?;
        Url::create_object_url_with_blob(&blob).ok()
    }

    /// Fetches `url` and stores it. Returns whether it is now cached.
    ///
    /// Fetched with CORS rather than the default `no-cors` an `<img>` would use: jsDelivr sends
    /// `access-control-allow-origin: *`, so the response is not opaque and counts its real size
    /// against the storage quota instead of a multi-megabyte padding figure.
    pub async fn store(url: &str) -> bool {
        let Some(window) = web_sys::window() else {
            return false;
        };
        let Some(cache) = open().await else {
            return false;
        };
        let init = RequestInit::new();
        init.set_mode(RequestMode::Cors);
        let Ok(response) = JsFuture::from(window.fetch_with_str_and_init(url, &init)).await else {
            return false;
        };
        let Ok(response) = response.dyn_into::<Response>() else {
            return false;
        };
        if !response.ok() {
            return false;
        }
        JsFuture::from(cache.put_with_str(url, &response))
            .await
            .is_ok()
    }

    /// Deletes the current cache and any left by other image-set versions.
    pub async fn delete_all() {
        let Some(storage) = cache_storage() else {
            return;
        };
        let _ = JsFuture::from(storage.delete(&cache_name())).await;
        prune_stale().await;
    }

    /// Asks the browser not to evict this origin's storage.
    ///
    /// Chrome decides from engagement heuristics without prompting; Firefox prompts. A denial
    /// is not an error — the cache still works, it is just evictable.
    pub async fn ensure_persistence() -> bool {
        let Some(window) = web_sys::window() else {
            return false;
        };
        let storage = window.navigator().storage();

        // Check first so an already-granted origin is never prompted again.
        if let Ok(promise) = storage.persisted()
            && let Ok(granted) = JsFuture::from(promise).await
            && granted.is_truthy()
        {
            return true;
        }

        let Ok(promise) = storage.persist() else {
            return false;
        };
        JsFuture::from(promise)
            .await
            .is_ok_and(|granted| granted.is_truthy())
    }

    /// Releases an object URL so its blob can be freed.
    pub fn revoke(object_url: &str) {
        let _ = Url::revoke_object_url(object_url);
    }
}

// ---------------------------------------------------------------------------
// Off-web stubs
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::collections::HashSet;

    /// No-op: Cache Storage is a browser API.
    pub async fn prune_stale() {}

    /// No-op: nothing is ever cached off the web, so no URL is known.
    pub async fn cached_urls() -> HashSet<String> {
        HashSet::new()
    }

    /// No-op: never cached, so never resolvable.
    pub async fn object_url(_url: &str) -> Option<String> {
        None
    }

    /// No-op: reports that nothing was stored, so images load from the network.
    pub async fn store(_url: &str) -> bool {
        false
    }

    /// No-op: there is no cache to delete.
    pub async fn delete_all() {}

    /// No-op: storage persistence is a browser concept. Reports "not persistent".
    pub async fn ensure_persistence() -> bool {
        false
    }

    /// No-op: there are no object URLs to release.
    pub fn revoke(_object_url: &str) {}
}

pub use platform::{
    cached_urls, delete_all, ensure_persistence, object_url, prune_stale, revoke, store,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_set_version_is_parsed_from_the_cdn_url() {
        let version = image_set_version();
        assert!(
            version.starts_with('v'),
            "expected a version tag like v0.11.0, got {version:?}"
        );
        assert!(!version.contains('/'), "version must be a single segment");
    }

    /// Regression: a second object URL arriving for an already-resolved image must not be handed
    /// back to the caller. The duplicate is released, so displaying it left the image
    /// permanently blank.
    #[test]
    fn duplicate_resolve_returns_the_established_url() {
        let mut inner = CacheInner::default();
        let first = inner.insert_resolved("https://cdn/a.png", "blob:first".to_string());
        let second = inner.insert_resolved("https://cdn/a.png", "blob:second".to_string());

        assert_eq!(first, "blob:first");
        assert_eq!(
            second, "blob:first",
            "the duplicate is revoked, so the established URL must be returned instead"
        );
        assert_eq!(inner.resolved.len(), 1);
    }

    #[test]
    fn resolve_returns_the_url_it_stored() {
        let mut inner = CacheInner::default();
        let returned = inner.insert_resolved("https://cdn/a.png", "blob:a".to_string());
        assert_eq!(returned, "blob:a");
        assert_eq!(
            inner.resolved.get("https://cdn/a.png").map(String::as_str),
            Some("blob:a")
        );
    }

    #[test]
    fn oldest_resolved_entries_are_evicted_past_the_cap() {
        let mut inner = CacheInner::default();
        for i in 0..MAX_RESOLVED + 10 {
            inner.insert_resolved(&format!("https://cdn/{i}.png"), format!("blob:{i}"));
        }
        assert_eq!(inner.resolved.len(), MAX_RESOLVED);
        assert!(
            !inner.resolved.contains_key("https://cdn/0.png"),
            "the oldest entry should have been evicted"
        );
        let newest = format!("https://cdn/{}.png", MAX_RESOLVED + 9);
        assert!(inner.resolved.contains_key(&newest));
    }

    /// A dead object URL must be dropped from both maps, so the next render falls back to the
    /// network instead of retrying a URL that will fail again.
    #[test]
    fn invalidate_drops_the_entry_and_its_cached_status() {
        let mut inner = CacheInner::default();
        inner.known.insert("https://cdn/a.png".to_string());
        inner.known.insert("https://cdn/b.png".to_string());
        inner.insert_resolved("https://cdn/a.png", "blob:a".to_string());
        inner.insert_resolved("https://cdn/b.png", "blob:b".to_string());

        inner.invalidate("https://cdn/a.png");

        assert!(!inner.resolved.contains_key("https://cdn/a.png"));
        assert!(!inner.known.contains("https://cdn/a.png"));
        assert!(!inner.order.iter().any(|u| u == "https://cdn/a.png"));
        // Unrelated entries are untouched.
        assert_eq!(
            inner.resolved.get("https://cdn/b.png").map(String::as_str),
            Some("blob:b")
        );
        assert!(inner.known.contains("https://cdn/b.png"));
    }

    #[test]
    fn clear_drops_every_entry_and_lets_a_later_scan_run() {
        let mut inner = CacheInner::default();
        inner.insert_resolved("https://cdn/a.png", "blob:a".to_string());
        inner.known.insert("https://cdn/a.png".to_string());
        inner.scanning = true;

        inner.clear();

        assert!(inner.resolved.is_empty());
        assert!(inner.order.is_empty());
        assert!(inner.known.is_empty());
        assert!(
            !inner.scanning,
            "re-enabling the setting must be able to scan again"
        );
    }

    #[test]
    fn cache_name_is_scoped_to_the_image_set_version() {
        let name = cache_name();
        assert!(name.starts_with(CACHE_PREFIX));
        assert!(name.ends_with(image_set_version()));
    }
}

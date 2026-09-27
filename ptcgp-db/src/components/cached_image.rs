//! The app's image element. Serves from the local image cache when that setting is on.

use dioxus::core::spawn_forever;
use dioxus::prelude::*;
use ptcgp_db_core::AppSettings;

use crate::image_cache::{self, ImageCache, PLACEHOLDER_SRC};

/// Where this render should load the image from.
#[derive(Clone, PartialEq)]
enum Source {
    /// Straight from the CDN, exactly as the app behaves with caching off.
    Remote,
    /// Cached on disk and being read back; show a placeholder rather than hit the network.
    Reading,
    /// Serve the cached bytes from this object URL.
    Local(String),
}

/// Releases a dead object URL so the next render falls back to the CDN.
///
/// A free function rather than an inline closure because `dx fmt` mangles multi-line closures
/// used as RSX props.
fn discard_dead_url(mut cache: ImageCache, mut revision: Signal<u32>, url: &str) {
    cache.invalidate(url);
    *revision.write() += 1;
}

/// An `<img>` backed by the local image cache.
///
/// Used for every image in the app so all of them cache identically. With the setting off this
/// is a plain `<img>` with no extra work.
///
/// On a cache miss the CDN URL is rendered immediately — first load looks exactly as it always
/// has, progressive decode included — and the bytes are stored in the background for next time.
/// A cached image renders a placeholder for the few milliseconds the disk read takes, then swaps
/// in; once read, it is served from memory for the rest of the session with no further delay.
#[component]
pub fn CachedImage(
    /// The image's CDN URL.
    src: String,
    /// Alt text; empty for decorative images.
    #[props(default)]
    alt: String,
    /// Tailwind classes, applied identically in every state so layout never shifts.
    #[props(default)]
    class: String,
    /// Native `loading` attribute, e.g. `"lazy"`.
    #[props(default)]
    loading: Option<&'static str>,
) -> Element {
    let settings = use_context::<Signal<AppSettings>>();
    let cache = use_context::<ImageCache>();
    // Bumped when this image's own cache read finishes, purely to trigger a re-render so the
    // block below re-derives. The cache is read non-reactively, so one image resolving never
    // re-renders all the others.
    let mut revision = use_signal(|| 0u32);

    // `use_reactive!` is required here: `src` is a plain prop, and an effect otherwise re-runs
    // only when a signal it read changes. Without it, re-rendering this component with a
    // different URL — as the catalog's detail panel does on every selection — would never start
    // a read for the new image.
    use_effect(use_reactive!(|src| {
        if !settings.read().cache_images() || !cache.ready() {
            return;
        }
        let mut cache = cache;
        if cache.resolved(&src).is_some() {
            return;
        }
        let url = src.clone();
        if cache.is_known(&url) {
            // Scope-bound: if this image is replaced or unmounted mid-read the result has nobody
            // to go to, and whatever renders next simply reads again.
            spawn(async move {
                match image_cache::object_url(&url).await {
                    Some(object_url) => {
                        cache.insert_resolved(&url, object_url);
                    }
                    None => {
                        // Entry vanished (quota reclaim, manual clear); fall back to the network.
                        cache.forget(&url);
                    }
                }
                *revision.write() += 1;
            });
        } else {
            // Root-scoped so scrolling past an image still finishes storing it. Touches only
            // shared cache state, never this component's signals, so outliving it is safe.
            spawn_forever(async move {
                if image_cache::store(&url).await {
                    cache.mark_known(&url);
                }
            });
        }
    }));

    // Derived from `src` on every render rather than held across renders. A value initialised
    // once per mount kept displaying the previous image whenever this component was re-rendered
    // with a new URL instead of being remounted.
    let _revision = *revision.read();
    let display = if !settings.read().cache_images() {
        Source::Remote
    } else if let Some(object_url) = cache.resolved(&src) {
        Source::Local(object_url)
    } else if cache.is_known(&src) {
        Source::Reading
    } else {
        Source::Remote
    };

    let loading = loading.unwrap_or("eager");
    match display {
        Source::Remote => rsx! {
            img {
                src: "{src}",
                alt: "{alt}",
                class: "{class}",
                loading: "{loading}",
            }
        },
        Source::Reading => rsx! {
            img {
                src: "{PLACEHOLDER_SRC}",
                alt: "{alt}",
                class: "{class}",
                loading: "{loading}",
            }
        },
        Source::Local(object_url) => {
            let dead = src.clone();
            rsx! {
                img {
                    src: "{object_url}",
                    alt: "{alt}",
                    class: "{class}",
                    loading: "{loading}",
                    // Last line of defence: an object URL can go stale (evicted from the
                    // in-memory cap while still displayed, or revoked as the setting is switched
                    // off). Discarding it makes the next render fall back to the CDN, so an
                    // image is never left blank.
                    onerror: move |_| discard_dead_url(cache, revision, &dead),
                }
            }
        }
    }
}

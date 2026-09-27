//! The app's image element. Serves from the local image cache when that setting is on.

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

    // Decided during the first render, not in an effect: an effect runs after the browser has
    // already started fetching whatever the first paint asked for, which would defeat the cache.
    let mut source = use_signal(|| {
        if !settings.peek().cache_images() || !cache.ready() {
            return Source::Remote;
        }
        match cache.resolved(&src) {
            Some(object_url) => Source::Local(object_url),
            None if cache.is_known(&src) => Source::Reading,
            None => Source::Remote,
        }
    });

    // Reads `cache_images` and `ready` reactively so enabling the setting, or the startup scan
    // finishing, kicks off work for images that are already mounted.
    let effect_src = src.clone();
    use_effect(move || {
        if !settings.read().cache_images() {
            // Disabling revokes every object URL, so stop displaying one immediately.
            source.set(Source::Remote);
            return;
        }
        if !cache.ready() {
            return;
        }
        let url = effect_src.clone();
        let mut cache = cache;

        if let Some(object_url) = cache.resolved(&url) {
            source.set(Source::Local(object_url));
            return;
        }
        if cache.is_pending(&url) || !cache.claim(&url) {
            return;
        }

        let known = cache.is_known(&url);
        if known {
            source.set(Source::Reading);
        }
        spawn(async move {
            if known {
                match image_cache::object_url(&url).await {
                    Some(object_url) => {
                        cache.insert_resolved(&url, object_url.clone());
                        source.set(Source::Local(object_url));
                    }
                    None => {
                        // Entry vanished (quota reclaim, manual clear); fall back to the network.
                        cache.forget(&url);
                        source.set(Source::Remote);
                    }
                }
            } else if image_cache::store(&url).await {
                // Stored for next time. The element keeps its CDN URL for this render rather
                // than swapping to a blob, so nothing already painted is disturbed.
                cache.mark_known(&url);
            }
            cache.release(&url);
        });
    });

    let loading = loading.unwrap_or("eager");
    let current = source.read().clone();
    match current {
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
        Source::Local(object_url) => rsx! {
            img {
                src: "{object_url}",
                alt: "{alt}",
                class: "{class}",
                loading: "{loading}",
            }
        },
    }
}

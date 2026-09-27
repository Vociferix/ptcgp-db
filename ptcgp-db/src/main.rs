mod app;
mod components;
#[cfg(target_arch = "wasm32")]
mod drive;
// Off the web the platform layer is a set of no-ops and nothing calls the cache-management
// entry points, so most of this module is deliberately unused there.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]
mod image_cache;
mod pages;
mod routes;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        // Capture window.location.search before Dioxus initializes. HashHistory rewrites the
        // URL during setup (adding #/) which drops any query params, including the OAuth ?code=.
        drive::capture_startup_search();

        use dioxus::web::{Config, HashHistory};
        use std::rc::Rc;
        dioxus::LaunchBuilder::web()
            .with_cfg(Config::new().history(Rc::new(HashHistory::default())))
            .launch(app::App);
    }

    #[cfg(not(target_arch = "wasm32"))]
    dioxus::launch(app::App);
}

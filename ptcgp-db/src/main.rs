mod app;
mod components;
#[cfg(target_arch = "wasm32")]
mod drive;
mod pages;
mod routes;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        use dioxus::web::{Config, HashHistory};
        use std::rc::Rc;
        dioxus::LaunchBuilder::web()
            .with_cfg(Config::new().history(Rc::new(HashHistory::default())))
            .launch(app::App);
    }

    #[cfg(not(target_arch = "wasm32"))]
    dioxus::launch(app::App);
}

mod app;
mod pages;
mod routes;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        use std::rc::Rc;
        use dioxus::web::{Config, HashHistory};
        dioxus::LaunchBuilder::web()
            .with_cfg(Config::new().history(Rc::new(HashHistory::default())))
            .launch(app::App);
    }

    #[cfg(not(target_arch = "wasm32"))]
    dioxus::launch(app::App);
}

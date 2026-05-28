use dioxus::prelude::*;

/// Placeholder settings page. A simple component that exercises light/dark mode
/// classes and the custom element colors defined in tailwind.css, so both modes
/// can be verified before the real Settings UI is built (T15).
#[component]
pub fn SettingsPage() -> Element {
    rsx! {
        div {
            class: "min-h-screen bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 p-8",
            h1 { class: "text-2xl font-bold mb-6", "Settings" }

            p { class: "text-gray-600 dark:text-gray-400 mb-8",
                "Settings page — full implementation coming in T15."
            }

            // Element color swatches — verify custom Tailwind colors from tailwind.css
            h2 { class: "text-lg font-semibold mb-4", "Element colors" }
            div { class: "flex flex-wrap gap-2",
                for (label, cls) in [
                    ("Grass",      "bg-element-grass"),
                    ("Fire",       "bg-element-fire"),
                    ("Water",      "bg-element-water"),
                    ("Lightning",  "bg-element-lightning"),
                    ("Fighting",   "bg-element-fighting"),
                    ("Psychic",    "bg-element-psychic"),
                    ("Darkness",   "bg-element-darkness"),
                    ("Metal",      "bg-element-metal"),
                    ("Colorless",  "bg-element-colorless"),
                    ("Dragon",     "bg-element-dragon"),
                ] {
                    div {
                        class: "flex flex-col items-center gap-1",
                        div { class: "{cls} w-10 h-10 rounded-full border border-gray-300 dark:border-gray-600" }
                        span { class: "text-xs", "{label}" }
                    }
                }
            }
        }
    }
}

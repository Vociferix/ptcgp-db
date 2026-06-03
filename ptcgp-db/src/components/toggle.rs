use crate::components::icons::Check;
use dioxus::prelude::*;

/// Visual checkbox for the multi-select toggle zone in dropdown rows.
///
/// Renders a filled blue box with a checkmark when `checked`, or an outlined square when not.
#[component]
pub fn ToggleCheckbox(checked: bool) -> Element {
    rsx! {
        if checked {
            span { class: "flex items-center justify-center w-5 h-5 rounded bg-blue-500 dark:bg-blue-600",
                Check { class: "w-3.5 h-3.5 text-white" }
            }
        } else {
            span { class: "block w-5 h-5 rounded border-2 border-gray-300 dark:border-gray-500" }
        }
    }
}

/// A styled toggle switch (on/off).
///
/// Renders as a `<button role="switch">` with a sliding thumb. The `checked` prop controls
/// the current state; `on_change` is called with the toggled value on click.
#[component]
pub fn Toggle(checked: bool, on_change: EventHandler<bool>) -> Element {
    let track = if checked {
        "bg-blue-600 shadow-inner"
    } else {
        "bg-gray-300 dark:bg-gray-600 shadow-inner"
    };
    let thumb = if checked {
        "translate-x-5"
    } else {
        "translate-x-0"
    };

    rsx! {
        button {
            r#type: "button",
            role: "switch",
            aria_checked: "{checked}",
            onclick: move |_| on_change.call(!checked),
            class: "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full \
                    border-2 border-transparent transition-colors duration-200 ease-in-out \
                    focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 \
                    dark:focus:ring-offset-gray-800 {track}",
            span { class: "pointer-events-none inline-block h-5 w-5 transform rounded-full \
                        bg-white shadow-md ring-0 transition-transform duration-200 ease-in-out \
                        {thumb}" }
        }
    }
}

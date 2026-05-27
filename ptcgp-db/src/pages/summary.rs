use dioxus::prelude::*;

#[component]
pub fn SummaryPage() -> Element {
    rsx! {
        div { class: "p-4", "Summary — coming soon" }
    }
}

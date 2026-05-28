use dioxus::prelude::*;

/// Placeholder for the first-run onboarding screen (implemented in T17).
#[component]
pub fn OnboardingStub() -> Element {
    rsx! {
        div { class: "flex flex-col items-center justify-center h-screen gap-4",
            h1 { class: "text-2xl font-bold", "Welcome to PTCGP DB" }
            p { "Onboarding — coming soon (T17)" }
        }
    }
}

use dioxus::prelude::*;

use crate::components::profile_selector::ProfileSelector;
use crate::routes::Route;

// ---------------------------------------------------------------------------
// Nav items
// ---------------------------------------------------------------------------

struct NavItem {
    label: &'static str,
    route: Route,
    /// Short label for the narrow bottom bar.
    short: &'static str,
}

fn nav_items() -> impl IntoIterator<Item = NavItem> {
    [
        NavItem {
            label: "Summary",
            route: Route::SummaryPage {},
            short: "Home",
        },
        NavItem {
            label: "Card Catalog",
            route: Route::CatalogPage {},
            short: "Cards",
        },
        NavItem {
            label: "Trade",
            route: Route::TradePage {},
            short: "Trade",
        },
        NavItem {
            label: "Profiles",
            route: Route::ProfileManagerPage {},
            short: "Profiles",
        },
        NavItem {
            label: "Settings",
            route: Route::SettingsPage {},
            short: "Settings",
        },
    ]
}

/// Returns true for pages where the Profile Selector is hidden.
fn hides_profile_selector(route: &Route) -> bool {
    matches!(route, Route::SettingsPage {})
}

// ---------------------------------------------------------------------------
// Layout component
// ---------------------------------------------------------------------------

/// Persistent navigation shell wrapping all routed pages.
///
/// Wide viewports (md+): sidebar always visible on the left.
/// Narrow viewports (<md): top header + bottom navigation bar.
#[component]
pub fn NavLayout() -> Element {
    let current: Route = use_route();
    let show_selector = !hides_profile_selector(&current);

    rsx! {
        div { class: "flex h-screen bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100",

            // ---- Wide sidebar (md+) ----
            nav { class: "hidden md:flex flex-col w-52 shrink-0 border-r \
                        border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800",

                // App title
                div { class: "px-4 py-4 text-lg font-bold tracking-tight border-b \
                              border-gray-200 dark:border-gray-700",
                    "PTCGP DB"
                }

                // Nav links
                ul { class: "flex-1 py-2 overflow-y-auto",
                    for item in nav_items() {
                        li { key: "{item.label}",
                            Link {
                                to: item.route,
                                class: "block px-4 py-2 text-sm rounded-md mx-2 \
                                        hover:bg-gray-200 dark:hover:bg-gray-700 \
                                        text-gray-700 dark:text-gray-300",
                                active_class: "bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-200 font-medium",
                                "{item.label}"
                            }
                        }
                    }
                }

                // Profile selector at the bottom of the sidebar
                if show_selector {
                    div { class: "p-3 border-t border-gray-200 dark:border-gray-700",
                        ProfileSelector { open_upward: true }
                    }
                }
            }

            // ---- Main content area ----
            div { class: "flex flex-col flex-1 min-w-0",

                // ---- Narrow top header (<md) ----
                header { class: "md:hidden flex items-center gap-2 px-3 py-2 border-b \
                            border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 shrink-0",
                    span { class: "font-bold tracking-tight text-base", "PTCGP DB" }
                    div { class: "flex-1",
                        if show_selector {
                            ProfileSelector {}
                        }
                    }
                }

                // Page content
                main { class: "flex-1 overflow-y-auto", Outlet::<Route> {} }

                // ---- Narrow bottom nav (<md) ----
                nav { class: "md:hidden flex border-t border-gray-200 dark:border-gray-700 \
                            bg-gray-50 dark:bg-gray-800 shrink-0",
                    for item in nav_items() {
                        Link {
                            key: "{item.short}",
                            to: item.route,
                            class: "flex-1 flex flex-col items-center py-2 text-xs \
                                    text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400",
                            active_class: "text-blue-600 dark:text-blue-400 font-semibold",
                            "{item.short}"
                        }
                    }
                }
            }
        }
    }
}

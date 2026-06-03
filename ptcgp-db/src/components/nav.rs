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
        div { class: "flex h-screen bg-gray-100 dark:bg-gray-950 text-gray-900 dark:text-gray-100",

            // ---- Wide sidebar (md+) ----
            nav { class: "hidden md:flex flex-col w-52 shrink-0 relative z-10 \
                        border-r border-gray-200/60 dark:border-gray-700/60 \
                        bg-white dark:bg-gray-800 \
                        shadow-[2px_0_12px_rgba(0,0,0,0.08)] dark:shadow-[2px_0_12px_rgba(0,0,0,0.4)]",

                // App title
                div { class: "px-4 py-4 text-lg font-bold tracking-tight border-b \
                              border-gray-200/80 dark:border-gray-700/80",
                    "PTCGP DB"
                }

                // Nav links
                ul { class: "flex-1 py-2 overflow-y-auto",
                    for item in nav_items() {
                        li { key: "{item.label}",
                            Link {
                                to: item.route,
                                class: "block px-4 py-2 text-sm rounded-md mx-2 \
                                        hover:bg-gray-100 dark:hover:bg-gray-700 \
                                        text-gray-700 dark:text-gray-300 transition-colors",
                                active_class: "bg-blue-50 dark:bg-blue-950/60 \
                                              text-blue-700 dark:text-blue-200 font-medium \
                                              ring-1 ring-inset ring-blue-200 dark:ring-blue-800 \
                                              shadow-sm",
                                "{item.label}"
                            }
                        }
                    }
                }

                // Profile selector at the bottom of the sidebar
                if show_selector {
                    div { class: "p-3 border-t border-gray-200/80 dark:border-gray-700/80",
                        ProfileSelector { open_upward: true }
                    }
                }
            }

            // ---- Main content area ----
            div { class: "flex flex-col flex-1 min-w-0",

                // ---- Narrow top header (<md) ----
                header { class: "md:hidden flex items-center gap-2 px-3 py-2 border-b \
                            border-gray-200/80 dark:border-gray-700/80 \
                            bg-white dark:bg-gray-800 shrink-0 \
                            shadow-[0_2px_8px_rgba(0,0,0,0.06)] dark:shadow-[0_2px_8px_rgba(0,0,0,0.3)] \
                            relative z-10",
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
                nav { class: "md:hidden flex border-t border-gray-200/80 dark:border-gray-700/80 \
                            bg-white dark:bg-gray-800 shrink-0 divide-x divide-gray-200/80 dark:divide-gray-700/80 \
                            shadow-[0_-2px_8px_rgba(0,0,0,0.06)] dark:shadow-[0_-2px_8px_rgba(0,0,0,0.3)] \
                            relative z-10",
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

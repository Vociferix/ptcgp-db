use dioxus::prelude::*;
use ptcgp_db_core::save_data::Theme;
use ptcgp_db_core::{AppSettings, ProfileStore};

use crate::app::{AppStorage, schedule_save};
use crate::components::icons::GitHubIcon;
use crate::components::toggle::Toggle;

const APP_VERSION: &str = env!("PTCGP_APP_VERSION");
const GIT_HASH: &str = env!("PTCGP_GIT_HASH");

// ---------------------------------------------------------------------------
// Persistence helper
// ---------------------------------------------------------------------------

fn persist_settings(
    settings: Signal<AppSettings>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
) {
    let current = settings.read().clone();
    let storage = store.read().as_ref().map(|s| s.storage().clone());
    if let Some(storage) = storage {
        spawn(async move {
            if let Err(e) = current.save(&storage).await {
                tracing::error!("settings save failed: {e}");
            }
        });
    }
    // Trigger Drive sync so changed settings are included in the next auto-save bundle.
    schedule_save();
}

// ---------------------------------------------------------------------------
// Setting row (label + description + toggle)
// ---------------------------------------------------------------------------

#[component]
fn SettingToggle(
    label: &'static str,
    description: &'static str,
    checked: bool,
    on_change: EventHandler<bool>,
) -> Element {
    rsx! {
        div { class: "flex items-center justify-between py-4",
            div { class: "flex-1 pr-8",
                p { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                    "{label}"
                }
                p { class: "text-sm text-gray-500 dark:text-gray-400 mt-0.5", "{description}" }
            }
            Toggle { checked, on_change }
        }
    }
}

// ---------------------------------------------------------------------------
// Settings page
// ---------------------------------------------------------------------------

#[component]
pub fn SettingsPage() -> Element {
    let mut settings = use_context::<Signal<AppSettings>>();
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let (theme, ignore_unobtainable, ignore_premium, ignore_gold, merge_dupes) = {
        let s = settings.read();
        (
            s.theme(),
            s.ignore_unobtainable_sets(),
            s.ignore_premium_mission(),
            s.ignore_gold_shop(),
            s.merge_duplicate_printings(),
        )
    };

    #[cfg(target_arch = "wasm32")]
    let drive_section = rsx! {
        crate::drive::DriveSyncSection {}
    };
    #[cfg(not(target_arch = "wasm32"))]
    let drive_section = rsx! {};

    rsx! {
        div { class: "max-w-2xl mx-auto p-6 space-y-6",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Settings" }
            {drive_section}

            // ── Appearance ──────────────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider \
                              text-gray-500 dark:text-gray-400 mb-3",
                    "Appearance"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                              border-gray-200/80 dark:border-gray-700/80 p-4 shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] dark:ring-1 dark:ring-white/[0.06]",
                    div { class: "flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3",
                        div {
                            p { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                                "Theme"
                            }
                            p { class: "text-sm text-gray-500 dark:text-gray-400 mt-0.5",
                                "Dark, light, or follow the system preference"
                            }
                        }
                        // Three-way segmented button
                        div { class: "self-start sm:self-auto flex rounded-md overflow-hidden \
                                      border border-gray-200 dark:border-gray-600",
                            for (label, value) in [("System", Theme::System), ("Light", Theme::Light), ("Dark", Theme::Dark)] {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        settings.write().set_theme(value);
                                        persist_settings(settings, store);
                                    },
                                    class: if theme == value { "px-3 py-1.5 text-sm font-medium bg-blue-600 text-white shadow-inner" } else { "px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-300 \
                                         bg-white dark:bg-gray-800 \
                                         hover:bg-gray-100 dark:hover:bg-gray-700 \
                                         shadow-sm active:shadow-none active:translate-y-px" },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }

            // ── Collection ─────────────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider \
                              text-gray-500 dark:text-gray-400 mb-3",
                    "Collection"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                              border-gray-200/80 dark:border-gray-700/80 px-4 \
                              divide-y divide-gray-100 dark:divide-gray-700 shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] dark:ring-1 dark:ring-white/[0.06]",
                    SettingToggle {
                        label: "Merge duplicate printings",
                        description: "Count reprinted cards as a single logical card; owned counts are summed across all versions.",
                        checked: merge_dupes,
                        on_change: move |v| {
                            settings.write().set_merge_duplicate_printings(v);
                            persist_settings(settings, store);
                        },
                    }
                }
            }

            // ── Filters ────────────────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider \
                              text-gray-500 dark:text-gray-400 mb-3",
                    "Filters"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                              border-gray-200/80 dark:border-gray-700/80 px-4 \
                              divide-y divide-gray-100 dark:divide-gray-700 shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] dark:ring-1 dark:ring-white/[0.06]",
                    SettingToggle {
                        label: "Ignore unobtainable sets",
                        description: "Hide retired sets from the catalog, completion stats, and all probability calculations.",
                        checked: ignore_unobtainable,
                        on_change: move |v| {
                            settings.write().set_ignore_unobtainable_sets(v);
                            persist_settings(settings, store);
                        },
                    }
                    SettingToggle {
                        label: "Ignore Premium Mission cards",
                        description: "Exclude Premium Mission cards from the catalog, completion counts, and analysis.",
                        checked: ignore_premium,
                        on_change: move |v| {
                            settings.write().set_ignore_premium_mission(v);
                            persist_settings(settings, store);
                        },
                    }
                    SettingToggle {
                        label: "Ignore Gold Shop cards",
                        description: "Exclude Gold Shop cards from the catalog, completion counts, and analysis.",
                        checked: ignore_gold,
                        on_change: move |v| {
                            settings.write().set_ignore_gold_shop(v);
                            persist_settings(settings, store);
                        },
                    }
                }
            }

            // ── About ───────────────────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider \
                              text-gray-500 dark:text-gray-400 mb-3",
                    "About"
                }
                div { class: "space-y-3",
                    // App identity card
                    div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                                  border-gray-200/80 dark:border-gray-700/80 p-4 \
                                  shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] \
                                  dark:ring-1 dark:ring-white/[0.06]",
                        div { class: "flex items-start justify-between gap-4",
                            div { class: "min-w-0",
                                p { class: "text-sm font-semibold text-gray-900 dark:text-gray-100",
                                    "ptcgp-db"
                                }
                                p { class: "text-xs text-gray-500 dark:text-gray-400 mt-0.5 font-mono",
                                    "{APP_VERSION} · {GIT_HASH}"
                                }
                                div { class: "flex items-center gap-2 mt-2",
                                    a {
                                        href: "https://github.com/Vociferix/ptcgp-db/blob/master/LICENSE",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        class: "text-xs text-gray-500 dark:text-gray-400 \
                                                hover:text-gray-900 dark:hover:text-gray-100 \
                                                transition-colors",
                                        "Apache 2.0"
                                    }
                                    span { class: "text-gray-300 dark:text-gray-600 select-none",
                                        "·"
                                    }
                                    a {
                                        href: "https://github.com/Vociferix/ptcgp-db/issues",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        class: "text-xs text-gray-500 dark:text-gray-400 \
                                                hover:text-gray-900 dark:hover:text-gray-100 \
                                                transition-colors",
                                        "Report an issue"
                                    }
                                }
                            }
                            a {
                                href: "https://github.com/Vociferix/ptcgp-db",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                class: "flex items-center gap-1.5 flex-shrink-0 \
                                        text-xs text-gray-500 dark:text-gray-400 \
                                        hover:text-gray-900 dark:hover:text-gray-100 \
                                        transition-colors mt-0.5",
                                GitHubIcon { class: "w-4 h-4" }
                                "GitHub"
                            }
                        }
                    }
                    // Disclaimer
                    div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                                  border-gray-200/80 dark:border-gray-700/80 p-4 \
                                  shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] \
                                  dark:ring-1 dark:ring-white/[0.06]",
                        p { class: "text-xs text-gray-500 dark:text-gray-400 leading-relaxed",
                            "The literal and graphical information presented in this application \
                            about Pokémon Trading Card Game Pocket, including card data, text and \
                            images, is copyright The Pokémon Company, DeNA Co., Ltd., and/or \
                            Creatures, Inc. This application is not produced by, endorsed by, \
                            supported by, or affiliated with any of those copyright holders."
                        }
                    }
                }
            }
        }
    }
}

use dioxus::prelude::*;
use ptcgp_db_core::save_data::Theme;
use ptcgp_db_core::{AppSettings, ProfileStore};

use crate::app::AppStorage;

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
}

// ---------------------------------------------------------------------------
// Toggle switch
// ---------------------------------------------------------------------------

#[component]
fn Toggle(checked: bool, on_change: EventHandler<bool>) -> Element {
    let track = if checked {
        "bg-blue-600"
    } else {
        "bg-gray-300 dark:bg-gray-600"
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
                        bg-white shadow ring-0 transition-transform duration-200 ease-in-out \
                        {thumb}" }
        }
    }
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

    let theme = settings.read().theme();
    let ignore_unobtainable = settings.read().ignore_unobtainable_sets();
    let ignore_premium = settings.read().ignore_premium_mission();
    let ignore_gold = settings.read().ignore_gold_shop();
    let merge_dupes = settings.read().merge_duplicate_printings();

    rsx! {
        div { class: "max-w-2xl mx-auto p-6 space-y-6",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Settings" }

            // ── Appearance ──────────────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider \
                              text-gray-500 dark:text-gray-400 mb-3",
                    "Appearance"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                              border-gray-200 dark:border-gray-700 p-4",
                    div { class: "flex items-center justify-between",
                        div {
                            p { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                                "Theme"
                            }
                            p { class: "text-sm text-gray-500 dark:text-gray-400 mt-0.5",
                                "Dark, light, or follow the system preference"
                            }
                        }
                        // Three-way segmented button
                        div { class: "flex rounded-md overflow-hidden \
                                      border border-gray-200 dark:border-gray-600",
                            for (label, value) in [("System", Theme::System), ("Light", Theme::Light), ("Dark", Theme::Dark)] {
                                button {
                                    r#type: "button",
                                    onclick: move |_| {
                                        settings.write().set_theme(value);
                                        persist_settings(settings, store);
                                    },
                                    class: if theme == value { "px-3 py-1.5 text-sm font-medium bg-blue-600 text-white" } else { "px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-300 \
                                         bg-white dark:bg-gray-800 \
                                         hover:bg-gray-100 dark:hover:bg-gray-700" },
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
                              border-gray-200 dark:border-gray-700 px-4 \
                              divide-y divide-gray-100 dark:divide-gray-700",
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
                              border-gray-200 dark:border-gray-700 px-4 \
                              divide-y divide-gray-100 dark:divide-gray-700",
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
        }
    }
}

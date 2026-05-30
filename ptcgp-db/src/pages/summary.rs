use chrono::NaiveDate;
use dioxus::prelude::*;
use ptcgp_db_core::save_data::CardVersionId;
use ptcgp_db_core::{AppSettings, ProfileStore, completion, completion_merged, desired_pull_rate};
use ptcgp_db_data::{CardVersion, Pack, Prob, Set};

use crate::app::AppStorage;
use crate::components::toggle::Toggle;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn today_naive() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

fn set_is_obtainable(set: &Set, today: NaiveDate) -> bool {
    set.retirement_date().is_none_or(|r| r > today)
}

// ---------------------------------------------------------------------------
// Per-set data computed during render
// ---------------------------------------------------------------------------

struct SetRowData {
    set: &'static Set,
    completion_pct: f64,
    owned: usize,
    total: usize,
    obtainable: bool,
    best_pack: Option<&'static Pack>,
    best_rate_pct: f64,
}

// ---------------------------------------------------------------------------
// Set completion row component
// ---------------------------------------------------------------------------

#[component]
fn SetCompletionRow(
    set: &'static Set,
    completion_pct: f64,
    owned: usize,
    total: usize,
    is_obtainable: bool,
    best_pack: Option<&'static Pack>,
    best_rate_pct: f64,
) -> Element {
    let set_name = set.name();
    let is_promo = set.is_promo();

    rsx! {
        div { class: "grid grid-cols-[1fr_auto_auto] gap-x-4 px-4 py-3 items-center border-b border-gray-100 dark:border-gray-700 last:border-0",
            div { class: "flex items-center gap-2 min-w-0",
                img {
                    src: "{set.icon()}",
                    alt: "",
                    class: "h-6 w-6 object-contain shrink-0",
                }
                span { class: "text-sm text-gray-900 dark:text-gray-100 truncate", "{set_name}" }
                if !is_obtainable {
                    span { class: "shrink-0 text-xs px-1.5 py-0.5 rounded-full bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400",
                        "Retired"
                    }
                }
            }
            div { class: "text-right whitespace-nowrap",
                span { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                    "{completion_pct:.1}%"
                }
                span { class: "text-xs text-gray-400 dark:text-gray-500 ml-1.5", "{owned}/{total}" }
            }
            div { class: "text-right w-20 whitespace-nowrap",
                if is_promo || (best_pack.is_none() && completion_pct < 100.0) {
                    span { class: "text-sm text-gray-400 dark:text-gray-500", "—" }
                } else if completion_pct >= 100.0 && !is_promo {
                    span { class: "text-sm text-green-600 dark:text-green-400 font-medium",
                        "Complete"
                    }
                } else {
                    span { class: "text-sm text-gray-900 dark:text-gray-100", "{best_rate_pct:.2}%" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Summary page
// ---------------------------------------------------------------------------

#[component]
pub fn SummaryPage() -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let settings = use_context::<Signal<AppSettings>>();
    let mut include_unobtainable = use_signal(|| false);

    let store_guard = store.read();
    let settings_guard = settings.read();

    let merge_dupes = settings_guard.merge_duplicate_printings();
    let ignore_unobtainable_global = settings_guard.ignore_unobtainable_sets();
    let ignore_premium = settings_guard.ignore_premium_mission();
    let ignore_gold = settings_guard.ignore_gold_shop();

    let include_unob = *include_unobtainable.read();
    let today = today_naive();

    let counts = |id: CardVersionId| -> u32 {
        store_guard
            .as_ref()
            .map(|s| s.aggregate_count(id))
            .unwrap_or(0)
    };

    let cv_included = |cv: &CardVersion| -> bool {
        let src = cv.source().name();
        if ignore_premium && src.as_str() == "Premium Mission" {
            return false;
        }
        if ignore_gold && src.as_str() == "Gold Shop" {
            return false;
        }
        true
    };

    // ── Per-set rows ────────────────────────────────────────────────────────

    let set_rows: Vec<SetRowData> = Set::ALL
        .iter()
        .filter(|set| {
            if ignore_unobtainable_global && !set.is_promo() {
                return set_is_obtainable(set, today);
            }
            true
        })
        .map(|set| {
            let cvs = set.card_versions();
            let included_ids: Vec<CardVersionId> = cvs
                .iter()
                .filter(|cv| cv_included(cv))
                .map(|cv| cv.id())
                .collect();

            let total = included_ids.len();
            let owned = included_ids.iter().filter(|&&id| counts(id) > 0).count();

            #[allow(clippy::redundant_closure)]
            let comp = if merge_dupes {
                completion_merged(|id| counts(id), 1, included_ids.iter().copied())
            } else {
                completion(|id| counts(id), 1, included_ids.iter().copied())
            };

            let obtainable = set_is_obtainable(set, today);

            let (best_pack, best_rate_pct) = if set.is_promo() {
                (None, 0.0)
            } else {
                let result = set
                    .packs()
                    .iter()
                    .filter_map(|p| {
                        let rate = desired_pull_rate(p, |id| {
                            let Some(cv) = CardVersion::from_id(id) else {
                                return false;
                            };
                            cv_included(cv) && counts(id) == 0
                        });
                        if rate == Prob::ZERO {
                            None
                        } else {
                            Some((p, rate))
                        }
                    })
                    .max_by(|(_, a), (_, b)| a.cmp(b));
                match result {
                    Some((pack, rate)) => (Some(pack), rate.as_f64() * 100.0),
                    None => (None, 0.0),
                }
            };

            SetRowData {
                set,
                completion_pct: comp.as_f64() * 100.0,
                owned,
                total,
                obtainable,
                best_pack,
                best_rate_pct,
            }
        })
        .collect();

    // ── Overall totals ───────────────────────────────────────────────────────

    let total_owned: usize = set_rows.iter().map(|r| r.owned).sum();
    let total_cards: usize = set_rows.iter().map(|r| r.total).sum();
    let overall_pct = if total_cards > 0 {
        total_owned as f64 / total_cards as f64 * 100.0
    } else {
        0.0
    };

    // ── Next pack to open ────────────────────────────────────────────────────

    let best_overall = Pack::ALL
        .iter()
        .filter(|p| {
            if p.set().is_promo() {
                return false;
            }
            let ob = set_is_obtainable(p.set(), today);
            if ignore_unobtainable_global {
                return ob;
            }
            include_unob || ob
        })
        .filter_map(|p| {
            let rate = desired_pull_rate(p, |id| {
                let Some(cv) = CardVersion::from_id(id) else {
                    return false;
                };
                cv_included(cv) && counts(id) == 0
            });
            if rate == Prob::ZERO {
                None
            } else {
                Some((p, rate))
            }
        })
        .max_by(|(_, a), (_, b)| a.cmp(b));

    let collection_complete =
        best_overall.is_none() && total_cards > 0 && total_owned == total_cards;

    rsx! {
        div { class: "max-w-4xl mx-auto p-4 sm:p-6 space-y-6",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Summary" }

            // ── Next pack ─────────────────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3",
                    "Next pack to open"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4",
                    if !ignore_unobtainable_global {
                        div { class: "flex items-center justify-between mb-4 pb-4 border-b border-gray-100 dark:border-gray-700",
                            span { class: "text-sm text-gray-600 dark:text-gray-400",
                                "Include unobtainable packs"
                            }
                            Toggle {
                                checked: include_unob,
                                on_change: move |v| include_unobtainable.set(v),
                            }
                        }
                    }
                    if collection_complete {
                        p { class: "text-sm font-medium text-green-600 dark:text-green-400",
                            "Collection complete! You own all available cards."
                        }
                    } else if let Some((pack, rate)) = best_overall {
                        div { class: "flex items-baseline gap-3",
                            span { class: "text-lg font-semibold text-gray-900 dark:text-gray-100",
                                "{pack.title()}"
                            }
                            span { class: "text-sm text-gray-500 dark:text-gray-400",
                                "{rate.as_f64() * 100.0:.2}% chance of a new card"
                            }
                        }
                    } else {
                        p { class: "text-sm text-gray-500 dark:text-gray-400", "No packs available." }
                    }
                }
            }

            // ── Overall totals ────────────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3",
                    "Overall"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4 space-y-3",
                    div { class: "flex items-baseline gap-4",
                        span { class: "text-3xl font-bold text-gray-900 dark:text-gray-100",
                            "{overall_pct:.1}%"
                        }
                        span { class: "text-sm text-gray-500 dark:text-gray-400",
                            "{total_owned} / {total_cards} card versions"
                        }
                    }
                    div { class: "h-2 rounded-full bg-gray-200 dark:bg-gray-700",
                        div {
                            class: "h-2 rounded-full bg-blue-500 transition-all",
                            style: "width: {overall_pct:.4}%",
                        }
                    }
                }
            }

            // ── Set completion table ──────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3",
                    "Set completion"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden",
                    div { class: "grid grid-cols-[1fr_auto_auto] gap-x-4 px-4 py-2 bg-gray-50 dark:bg-gray-900/50 text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider border-b border-gray-200 dark:border-gray-700",
                        span { "Set" }
                        span { "Completion" }
                        span { "Best pull" }
                    }
                    for row in &set_rows {
                        SetCompletionRow {
                            key: "{row.set.id()}",
                            set: row.set,
                            completion_pct: row.completion_pct,
                            owned: row.owned,
                            total: row.total,
                            is_obtainable: row.obtainable,
                            best_pack: row.best_pack,
                            best_rate_pct: row.best_rate_pct,
                        }
                    }
                }
            }
        }
    }
}

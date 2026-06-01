use chrono::NaiveDate;
use dioxus::prelude::*;
use ptcgp_db_core::save_data::{CardKindFilter, FilterConfig};
use ptcgp_db_core::{AppSettings, ProfileStore, SavedQueries, desired_pull_rate};
use ptcgp_db_data::{CardVersion, Pack};

use crate::app::AppStorage;
use crate::components::{FilterMode, FilterToolbar};

// ---------------------------------------------------------------------------
// Persistence helper
// ---------------------------------------------------------------------------

fn persist_queries(
    queries: Signal<SavedQueries>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
) {
    let current = queries.read().clone();
    let storage = store.read().as_ref().map(|s| s.storage().clone());
    if let Some(storage) = storage {
        spawn(async move {
            if let Err(e) = current.save(&storage).await {
                tracing::error!("saved queries save failed: {e}");
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Filter helpers
// ---------------------------------------------------------------------------

fn today_naive() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

/// Returns true when the card version should appear in the Analysis results.
/// Does not apply the goal/desired check — that is handled separately.
fn passes_filter(
    cv: &CardVersion,
    cfg: &FilterConfig,
    settings: &AppSettings,
    today: NaiveDate,
    matched_name_ids: Option<&[usize]>,
) -> bool {
    if settings.ignore_unobtainable_sets() && cv.set().retirement_date().is_some_and(|d| d <= today)
    {
        return false;
    }
    if settings.ignore_premium_mission() && cv.source().name().as_str() == "Premium Mission" {
        return false;
    }
    if settings.ignore_gold_shop() && cv.source().name().as_str() == "Gold Shop" {
        return false;
    }

    if matched_name_ids.is_some_and(|ids| !ids.contains(&cv.card().name().id())) {
        return false;
    }

    if cfg.series.is_some_and(|sid| cv.series().id() != sid) {
        return false;
    }
    if !cfg.sets.is_empty() && !cfg.sets.contains(&cv.set().id()) {
        return false;
    }
    if !cfg.packs.is_empty() && !cv.packs().iter().any(|p| cfg.packs.contains(&p.id())) {
        return false;
    }
    if !cfg.rarities.is_empty() && !cfg.rarities.contains(&cv.rarity().class().id()) {
        return false;
    }

    match cfg.card_kind {
        Some(CardKindFilter::Pokemon) if !cv.card().is_pokemon() => return false,
        Some(CardKindFilter::Trainer) if !cv.card().is_trainer() => return false,
        _ => {}
    }

    let pkmn = cv.card().pokemon();
    if let Some(ex_only) = cfg.ex
        && pkmn.is_none_or(|p| p.is_ex() != ex_only)
    {
        return false;
    }
    if let Some(mega_only) = cfg.mega
        && pkmn.is_none_or(|p| p.is_mega() != mega_only)
    {
        return false;
    }
    if let Some(stage_id) = cfg.stage
        && pkmn.is_none_or(|p| p.stage().id() != stage_id)
    {
        return false;
    }
    if !cfg.elements.is_empty() && pkmn.is_none_or(|p| !cfg.elements.contains(&p.element().id())) {
        return false;
    }
    if cfg.foil.is_some_and(|f| cv.is_foil() != f) {
        return false;
    }
    if !cfg.sources.is_empty() && !cfg.sources.contains(&cv.source().id()) {
        return false;
    }
    if let Some(obtainable) = cfg.obtainable {
        let is_obtainable = cv.set().retirement_date().is_none_or(|d| d > today);
        if is_obtainable != obtainable {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct PackResult {
    pack: &'static Pack,
    rate_pct: f64,
}

#[derive(Clone, PartialEq)]
struct AnalysisResult {
    matching: usize,
    desired: usize,
    completion_pct: f64,
    packs: Vec<PackResult>,
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

fn compute_results(
    cfg: &FilterConfig,
    settings: &AppSettings,
    store: &ProfileStore<AppStorage>,
    today: NaiveDate,
) -> AnalysisResult {
    let goal = cfg.goal.max(1);

    // Resolve name filter to a set of name IDs for fast lookup.
    let name_ids: Option<Vec<usize>> = cfg.name_query.as_deref().and_then(|q| {
        let q = q.trim();
        if q.is_empty() {
            None
        } else {
            let q_lower = q.to_lowercase();
            Some(
                CardVersion::ALL
                    .iter()
                    .filter(|cv| {
                        cv.card().name().as_str().to_lowercase().contains(&q_lower)
                            || format!("{} {}", cv.set().code(), cv.number())
                                .to_lowercase()
                                .contains(&q_lower)
                    })
                    .map(|cv| cv.card().name().id())
                    .collect(),
            )
        }
    });

    let mut matching: usize = 0;
    let mut desired_ids: Vec<usize> = Vec::new();
    let mut numerator: u64 = 0;
    let mut denominator: u64 = 0;

    for cv in CardVersion::ALL.iter() {
        if !passes_filter(cv, cfg, settings, today, name_ids.as_deref()) {
            continue;
        }
        matching += 1;

        let effective_count = if cfg.any_version_owned {
            cv.card()
                .versions()
                .iter()
                .map(|v| store.aggregate_count(v.id()))
                .fold(0u32, u32::saturating_add)
                .min(goal)
        } else {
            store.aggregate_count(cv.id()).min(goal)
        };

        numerator += effective_count as u64;
        denominator += goal as u64;

        if effective_count < goal {
            desired_ids.push(cv.id());
        }
    }

    let completion_pct = if denominator == 0 {
        100.0
    } else {
        (numerator as f64 / denominator as f64) * 100.0
    };

    let desired_count = desired_ids.len();

    // Compute per-pack probabilities for desired cards.
    let packs: Vec<PackResult> = if desired_ids.is_empty() {
        Vec::new()
    } else {
        let mut results: Vec<PackResult> = Pack::ALL
            .iter()
            .filter(|p| !p.set().is_promo())
            .filter_map(|p| {
                let rate = desired_pull_rate(p, |id| desired_ids.contains(&id));
                if rate.as_f64() <= 0.0 {
                    None
                } else {
                    Some(PackResult {
                        pack: p,
                        rate_pct: rate.as_f64() * 100.0,
                    })
                }
            })
            .collect();
        results.sort_by(|a, b| b.rate_pct.partial_cmp(&a.rate_pct).unwrap_or(std::cmp::Ordering::Equal));
        results
    };

    AnalysisResult {
        matching,
        desired: desired_count,
        completion_pct,
        packs,
    }
}

// ---------------------------------------------------------------------------
// Saved queries panel
// ---------------------------------------------------------------------------

#[component]
fn SavedQueriesPanel(config: Signal<FilterConfig>) -> Element {
    let mut queries = use_context::<Signal<SavedQueries>>();
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let mut save_name = use_signal(String::new);
    let mut save_error = use_signal(|| None::<&'static str>);
    let mut panel_open = use_signal(|| false);

    let query_list: Vec<(String, FilterConfig)> = queries
        .read()
        .queries()
        .iter()
        .map(|q| (q.name.clone(), q.config.clone()))
        .collect();

    rsx! {
        div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700",
            // Header / toggle
            button {
                r#type: "button",
                class: "w-full flex items-center justify-between px-4 py-3 text-sm font-medium \
                        text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700/50 \
                        rounded-lg",
                onclick: move |_| panel_open.toggle(),
                span { "Saved Queries" }
                span { class: "text-xs text-gray-400 dark:text-gray-500",
                    if *panel_open.read() {
                        "Hide"
                    } else {
                        "Show"
                    }
                }
            }

            if *panel_open.read() {
                div { class: "border-t border-gray-100 dark:border-gray-700 p-4 space-y-4",
                    // Save current query
                    div { class: "flex gap-2",
                        input {
                            r#type: "text",
                            placeholder: "Query name…",
                            class: "flex-1 min-w-0 rounded-md border border-gray-300 dark:border-gray-600 \
                                    bg-white dark:bg-gray-700 px-2.5 py-1.5 text-sm \
                                    text-gray-900 dark:text-gray-100 \
                                    focus:outline-none focus:ring-2 focus:ring-blue-500",
                            value: "{save_name}",
                            oninput: move |e| {
                                save_name.set(e.value());
                                save_error.set(None);
                            },
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    let name = save_name.read().trim().to_string();
                                    if name.is_empty() {
                                        save_error.set(Some("Name cannot be empty"));
                                        return;
                                    }
                                    let cfg = config.read().clone();
                                    if queries.write().add(name.clone(), cfg) {
                                        persist_queries(queries, store);
                                        save_name.set(String::new());
                                        save_error.set(None);
                                    } else {
                                        save_error.set(Some("A query with that name already exists"));
                                    }
                                }
                            },
                        }
                        button {
                            r#type: "button",
                            class: "shrink-0 px-3 py-1.5 rounded-md text-sm font-medium \
                                    bg-blue-600 text-white hover:bg-blue-700 \
                                    disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: save_name.read().trim().is_empty(),
                            onclick: move |_| {
                                let name = save_name.read().trim().to_string();
                                if name.is_empty() {
                                    save_error.set(Some("Name cannot be empty"));
                                    return;
                                }
                                let cfg = config.read().clone();
                                if queries.write().add(name.clone(), cfg) {
                                    persist_queries(queries, store);
                                    save_name.set(String::new());
                                    save_error.set(None);
                                } else {
                                    save_error.set(Some("A query with that name already exists"));
                                }
                            },
                            "Save"
                        }
                    }
                    if let Some(err) = *save_error.read() {
                        p { class: "text-xs text-red-600 dark:text-red-400", "{err}" }
                    }

                    // Saved query list
                    if query_list.is_empty() {
                        p { class: "text-sm text-gray-400 dark:text-gray-500 italic",
                            "No saved queries yet."
                        }
                    } else {
                        div { class: "space-y-1",
                            for (name, cfg_snapshot) in query_list {
                                SavedQueryRow {
                                    key: "{name}",
                                    name: name.clone(),
                                    on_load: {
                                        let cfg_snapshot = cfg_snapshot.clone();
                                        move |_| config.set(cfg_snapshot.clone())
                                    },
                                    on_delete: move |_| {
                                        queries.write().remove(&name);
                                        persist_queries(queries, store);
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SavedQueryRow(
    name: String,
    on_load: EventHandler<()>,
    on_delete: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-2 py-1",
            span { class: "flex-1 min-w-0 text-sm text-gray-700 dark:text-gray-200 truncate",
                "{name}"
            }
            button {
                r#type: "button",
                class: "shrink-0 px-2 py-1 rounded text-xs font-medium \
                        bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-200 \
                        hover:bg-blue-100 dark:hover:bg-blue-900 hover:text-blue-700 dark:hover:text-blue-300",
                onclick: move |_| on_load.call(()),
                "Load"
            }
            button {
                r#type: "button",
                class: "shrink-0 px-2 py-1 rounded text-xs font-medium \
                        bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 \
                        hover:bg-red-100 dark:hover:bg-red-900/40 hover:text-red-700 dark:hover:text-red-300",
                onclick: move |_| on_delete.call(()),
                "Delete"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pack results table
// ---------------------------------------------------------------------------

#[component]
fn PackResultRow(pack: &'static Pack, rate_pct: f64) -> Element {
    rsx! {
        div { class: "flex items-center gap-3 px-4 py-2.5 border-b \
                      border-gray-100 dark:border-gray-700 last:border-0",
            img {
                src: "{pack.image()}",
                alt: "",
                class: "h-14 w-auto object-contain shrink-0",
            }
            div { class: "flex-1 min-w-0",
                img {
                    src: "{pack.logo()}",
                    alt: "{pack.title()}",
                    class: "h-8 w-auto max-w-40 object-contain",
                }
            }
            span { class: "shrink-0 text-sm font-semibold text-gray-900 dark:text-gray-100 tabular-nums",
                "{rate_pct:.2}%"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Analysis page
// ---------------------------------------------------------------------------

#[component]
pub fn AnalysisPage() -> Element {
    let settings = use_context::<Signal<AppSettings>>();
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();

    // Analysis-specific filter config: default obtainable to Some(true), goal to 1.
    let config: Signal<FilterConfig> = use_signal(|| FilterConfig {
        obtainable: Some(true),
        goal: 1,
        ..FilterConfig::default()
    });

    let today = today_naive();

    let result = {
        let cfg = config.read();
        let s = settings.read();
        let store_guard = store.read();
        let Some(store_ref) = store_guard.as_ref() else {
            return rsx! {
                div { class: "p-4 text-gray-500 dark:text-gray-400", "Loading…" }
            };
        };
        compute_results(&cfg, &s, store_ref, today)
    };

    rsx! {
        div { class: "flex flex-col gap-4 p-4 max-w-3xl mx-auto",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Analysis" }

            // ── Filter toolbar ───────────────────────────────────────────────
            FilterToolbar { config, mode: FilterMode::Analysis }

            // ── Results ──────────────────────────────────────────────────────
            div { class: "bg-white dark:bg-gray-800 rounded-lg border \
                          border-gray-200 dark:border-gray-700",
                // Completion summary header
                div { class: "px-4 py-3 border-b border-gray-100 dark:border-gray-700 \
                              flex flex-wrap items-center justify-between gap-2",
                    div { class: "flex flex-col",
                        span { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                            if result.matching == 0 {
                                "No cards match the current filters."
                            } else if result.desired == 0 {
                                "All matching cards meet the goal."
                            } else {
                                "Completion"
                            }
                        }
                        if result.matching > 0 {
                            span { class: "text-xs text-gray-500 dark:text-gray-400",
                                "{result.desired} of {result.matching} cards still needed"
                            }
                        }
                    }
                    if result.matching > 0 {
                        span { class: "text-lg font-bold text-gray-900 dark:text-gray-100 tabular-nums",
                            "{result.completion_pct:.1}%"
                        }
                    }
                }

                // Pack probability list
                if result.desired == 0 || result.packs.is_empty() {
                    if result.matching > 0 && result.desired == 0 {
                        div { class: "px-4 py-6 text-center",
                            p { class: "text-sm font-medium text-green-600 dark:text-green-400",
                                "Goal fully met — no packs needed."
                            }
                        }
                    } else if result.desired > 0 {
                        div { class: "px-4 py-6 text-center",
                            p { class: "text-sm text-gray-500 dark:text-gray-400",
                                "None of the desired cards appear in non-promo packs."
                            }
                        }
                    }
                } else {
                    div { class: "divide-y divide-gray-100 dark:divide-gray-700",
                        // Column headers
                        div { class: "flex items-center gap-3 px-4 py-2 \
                                      bg-gray-50 dark:bg-gray-700/50",
                            div { class: "flex-1 text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider",
                                "Pack"
                            }
                            span { class: "shrink-0 text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider",
                                "Pull Chance"
                            }
                        }
                        for pr in result.packs {
                            PackResultRow {
                                key: "{pr.pack.id()}",
                                pack: pr.pack,
                                rate_pct: pr.rate_pct,
                            }
                        }
                    }
                }
            }

            // ── Saved Queries ────────────────────────────────────────────────
            SavedQueriesPanel { config }
        }
    }
}

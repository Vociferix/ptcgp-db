use chrono::NaiveDate;
use dioxus::prelude::*;
use ptcgp_db_core::save_data::{CardKindFilter, CardVersionId, FilterConfig};
use ptcgp_db_core::{
    AppSettings, ProfileStore, SavedQueries, completion, completion_merged, desired_pull_rate,
};
use ptcgp_db_data::{CardVersion, Pack, Prob, Set};

use crate::app::AppStorage;
use crate::components::icons::{ChevronDown, ChevronUp};
use crate::components::{FilterMode, FilterToolbar};
use crate::routes::Route;

// ---------------------------------------------------------------------------
// Persistence
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
// Navigation helpers
// ---------------------------------------------------------------------------

fn apply_pack_filter(pack_id: usize, mut filter: Signal<FilterConfig>) {
    *filter.write() = FilterConfig {
        packs: vec![pack_id],
        ..FilterConfig::default()
    };
}

fn apply_set_filter(set_id: usize, mut filter: Signal<FilterConfig>) {
    *filter.write() = FilterConfig {
        sets: vec![set_id],
        ..FilterConfig::default()
    };
}

// ---------------------------------------------------------------------------
// Filter helpers
// ---------------------------------------------------------------------------

fn today_naive() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

fn set_is_obtainable(set: &Set, today: NaiveDate) -> bool {
    set.retirement_date().is_none_or(|r| r > today)
}

/// Returns name IDs matching a text query (searches name and collector number).
fn name_ids_for_query(query: &str) -> Option<Vec<usize>> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }
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

/// Returns true when a card version passes the current filter config.
/// Does not check owned-count or goal — those are handled separately.
fn passes_filter(
    cv: &CardVersion,
    cfg: &FilterConfig,
    settings: &AppSettings,
    today: NaiveDate,
    name_ids: Option<&[usize]>,
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
    if name_ids.is_some_and(|ids| !ids.contains(&cv.card().name().id())) {
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
    if !cfg.elements.is_empty() && pkmn.is_none_or(|p| !cfg.elements.contains(&p.element().id()))
    {
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

/// Returns the effective owned count for a card version under the current config.
/// Handles `any_version_owned`: when set, counts all versions of the same abstract card.
/// Result is clamped to `goal`.
fn effective_count(
    cv_id: CardVersionId,
    cfg: &FilterConfig,
    store: &ProfileStore<AppStorage>,
) -> u32 {
    let goal = cfg.goal.max(1);
    if cfg.any_version_owned {
        let Some(cv) = CardVersion::from_id(cv_id) else {
            return 0;
        };
        cv.card()
            .versions()
            .iter()
            .map(|v| store.aggregate_count(v.id()))
            .fold(0u32, u32::saturating_add)
            .min(goal)
    } else {
        store.aggregate_count(cv_id).min(goal)
    }
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct PackRowData {
    pack: &'static Pack,
    completion_pct: f64,
    owned: usize,
    total: usize,
    rate_pct: f64,
}

struct SetRowData {
    set: &'static Set,
    completion_pct: f64,
    owned: usize,
    total: usize,
    obtainable: bool,
    best_pack: Option<&'static Pack>,
    best_rate_pct: f64,
    pack_rows: Vec<PackRowData>,
}

// ---------------------------------------------------------------------------
// Save Query dialog
// ---------------------------------------------------------------------------

#[component]
fn SaveQueryDialog(
    config: Signal<FilterConfig>,
    on_close: EventHandler<()>,
) -> Element {
    let mut queries = use_context::<Signal<SavedQueries>>();
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let mut name = use_signal(String::new);
    let mut error = use_signal(|| None::<&'static str>);

    rsx! {
        // Backdrop
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/40",
            onclick: move |_| on_close.call(()),
            // Dialog box — stop propagation so clicks inside don't close
            div {
                class: "bg-white dark:bg-gray-800 rounded-xl shadow-xl border \
                        border-gray-200 dark:border-gray-700 p-5 w-80 flex flex-col gap-4",
                onclick: move |e| e.stop_propagation(),
                h3 { class: "text-base font-semibold text-gray-900 dark:text-gray-100",
                    "Save Query"
                }
                div { class: "flex flex-col gap-1",
                    input {
                        r#type: "text",
                        placeholder: "Query name…",
                        autofocus: true,
                        class: "rounded-md border border-gray-300 dark:border-gray-600 \
                                bg-white dark:bg-gray-700 px-3 py-2 text-sm \
                                text-gray-900 dark:text-gray-100 \
                                focus:outline-none focus:ring-2 focus:ring-blue-500",
                        value: "{name}",
                        oninput: move |e| {
                            name.set(e.value());
                            error.set(None);
                        },
                        onkeydown: move |e| {
                            match e.key() {
                                Key::Enter => {
                                    let n = name.read().trim().to_string();
                                    if n.is_empty() {
                                        error.set(Some("Name cannot be empty"));
                                        return;
                                    }
                                    let cfg = config.read().clone();
                                    if queries.write().add(n, cfg) {
                                        persist_queries(queries, store);
                                        on_close.call(());
                                    } else {
                                        error.set(Some("A query with that name already exists"));
                                    }
                                }
                                Key::Escape => on_close.call(()),
                                _ => {}
                            }
                        },
                    }
                    if let Some(err) = *error.read() {
                        p { class: "text-xs text-red-600 dark:text-red-400", "{err}" }
                    }
                }
                div { class: "flex justify-end gap-2",
                    button {
                        r#type: "button",
                        class: "px-3 py-1.5 rounded-md text-sm font-medium \
                                text-gray-700 dark:text-gray-200 \
                                bg-gray-100 dark:bg-gray-700 \
                                hover:bg-gray-200 dark:hover:bg-gray-600",
                        onclick: move |_| on_close.call(()),
                        "Cancel"
                    }
                    button {
                        r#type: "button",
                        class: "px-3 py-1.5 rounded-md text-sm font-medium \
                                bg-blue-600 text-white hover:bg-blue-700 \
                                disabled:opacity-50 disabled:cursor-not-allowed",
                        disabled: name.read().trim().is_empty(),
                        onclick: move |_| {
                            let n = name.read().trim().to_string();
                            if n.is_empty() {
                                error.set(Some("Name cannot be empty"));
                                return;
                            }
                            let cfg = config.read().clone();
                            if queries.write().add(n, cfg) {
                                persist_queries(queries, store);
                                on_close.call(());
                            } else {
                                error.set(Some("A query with that name already exists"));
                            }
                        },
                        "Save"
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Saved queries controls (colocated with filter toolbar)
// ---------------------------------------------------------------------------

#[component]
fn SavedQueriesControls(config: Signal<FilterConfig>) -> Element {
    let queries = use_context::<Signal<SavedQueries>>();
    let mut dialog_open = use_signal(|| false);

    let query_names: Vec<String> = queries
        .read()
        .queries()
        .iter()
        .map(|q| q.name.clone())
        .collect();

    let has_queries = !query_names.is_empty();

    rsx! {
        div { class: "flex items-end gap-1.5 shrink-0",
            // Load query dropdown — only shown when there are saved queries
            if has_queries {
                select {
                    class: "h-8 rounded-md border border-gray-300 dark:border-gray-600 \
                            bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 \
                            text-xs px-2 pr-6 focus:outline-none focus:ring-2 focus:ring-blue-500 \
                            cursor-pointer",
                    value: "",
                    onchange: move |e| {
                        let selected = e.value();
                        if selected.is_empty() {
                            return;
                        }
                        let cfg = queries
                            .read()
                            .queries()
                            .iter()
                            .find(|q| q.name == selected)
                            .map(|q| q.config.clone());
                        if let Some(cfg) = cfg {
                            config.set(cfg);
                        }
                    },
                    option { value: "", disabled: true, "Load query…" }
                    for name in query_names {
                        option { key: "{name}", value: "{name}", "{name}" }
                    }
                }
            }

            // Save Query button
            button {
                r#type: "button",
                class: "h-8 px-2.5 rounded-md text-xs font-medium \
                        border border-gray-300 dark:border-gray-600 \
                        bg-white dark:bg-gray-800 \
                        text-gray-700 dark:text-gray-200 \
                        hover:bg-gray-100 dark:hover:bg-gray-700 \
                        whitespace-nowrap",
                onclick: move |_| dialog_open.set(true),
                "Save Query"
            }

            if *dialog_open.read() {
                SaveQueryDialog { config, on_close: move |_| dialog_open.set(false) }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-pack sub-row
// ---------------------------------------------------------------------------

#[component]
fn PackSubRow(
    pack: &'static Pack,
    completion_pct: f64,
    owned: usize,
    total: usize,
    rate_pct: f64,
) -> Element {
    let nav = use_navigator();
    let catalog_filter = use_context::<Signal<FilterConfig>>();
    let pack_id = pack.id();
    let on_click = move |_| {
        apply_pack_filter(pack_id, catalog_filter);
        drop(nav.push(Route::CatalogPage {}));
    };
    rsx! {
        div {
            class: "flex items-center gap-3 py-2 pl-8 pr-4 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-700/60",
            onclick: on_click,
            img {
                src: "{pack.image()}",
                alt: "",
                class: "h-24 w-auto object-contain shrink-0",
            }
            div { class: "flex-1 min-w-0",
                img {
                    src: "{pack.logo()}",
                    alt: "{pack.title()}",
                    class: "h-12 w-auto max-w-56 object-contain",
                }
            }
            div { class: "text-right whitespace-nowrap shrink-0",
                span { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                    "{completion_pct:.1}%"
                }
                span { class: "text-xs text-gray-400 dark:text-gray-500 ml-1.5", "{owned}/{total}" }
            }
            div { class: "text-right w-20 whitespace-nowrap shrink-0",
                if completion_pct >= 100.0 {
                    span { class: "text-sm text-green-600 dark:text-green-400 font-medium",
                        "Complete"
                    }
                } else if rate_pct <= 0.0 {
                    span { class: "text-sm text-gray-400 dark:text-gray-500", "—" }
                } else {
                    span { class: "text-sm text-gray-900 dark:text-gray-100", "{rate_pct:.2}%" }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Set completion row
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
    pack_rows: Vec<PackRowData>,
) -> Element {
    let mut expanded = use_signal(|| false);
    let nav = use_navigator();
    let catalog_filter = use_context::<Signal<FilterConfig>>();
    let set_name = set.name();
    let is_promo = set.is_promo();
    let is_expandable = !pack_rows.is_empty();
    let set_id = set.id();
    let on_click = move |_| {
        apply_set_filter(set_id, catalog_filter);
        drop(nav.push(Route::CatalogPage {}));
    };

    rsx! {
        div { class: "border-b border-gray-100 dark:border-gray-700 last:border-0",
            div {
                class: "grid grid-cols-[1fr_auto_auto] gap-x-4 px-4 py-3 items-center cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-700/50",
                onclick: on_click,
                div { class: "flex items-center gap-2 min-w-0",
                    if is_expandable {
                        button {
                            class: "shrink-0 w-7 h-7 flex items-center justify-center rounded text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600 hover:text-gray-600 dark:hover:text-gray-200",
                            onclick: move |e| {
                                e.stop_propagation();
                                expanded.set(!expanded());
                            },
                            if expanded() {
                                ChevronUp { class: "w-4 h-4" }
                            } else {
                                ChevronDown { class: "w-4 h-4" }
                            }
                        }
                    }
                    img {
                        src: "{set.icon()}",
                        alt: "",
                        class: "h-5 w-auto max-w-14 object-contain shrink-0",
                    }
                    img {
                        src: "{set.logo()}",
                        alt: "{set_name}",
                        class: "h-10 w-auto max-w-32 object-contain shrink-0",
                    }
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
                    span { class: "text-xs text-gray-400 dark:text-gray-500 ml-1.5",
                        "{owned}/{total}"
                    }
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
            if expanded() && is_expandable {
                div { class: "bg-gray-50 dark:bg-gray-900/30 divide-y divide-gray-100 dark:divide-gray-700",
                    for pack_row in pack_rows {
                        PackSubRow {
                            key: "{pack_row.pack.id()}",
                            pack: pack_row.pack,
                            completion_pct: pack_row.completion_pct,
                            owned: pack_row.owned,
                            total: pack_row.total,
                            rate_pct: pack_row.rate_pct,
                        }
                    }
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
    let catalog_filter = use_context::<Signal<FilterConfig>>();
    let nav = use_navigator();

    // Filter config for this page: goal=1, no other filters by default.
    let config: Signal<FilterConfig> = use_signal(|| FilterConfig {
        goal: 1,
        ..FilterConfig::default()
    });

    let store_guard = store.read();
    let settings_guard = settings.read();
    let cfg = config.read();

    let merge_dupes = settings_guard.merge_duplicate_printings();
    let goal = cfg.goal.max(1);
    let today = today_naive();

    let name_ids = cfg
        .name_query
        .as_deref()
        .and_then(|q| name_ids_for_query(q));

    let Some(store_ref) = store_guard.as_ref() else {
        return rsx! {
            div { class: "p-4 text-gray-500 dark:text-gray-400", "Loading…" }
        };
    };

    let counts = |id: CardVersionId| store_ref.aggregate_count(id);

    // ── Per-set rows ────────────────────────────────────────────────────────

    let set_rows: Vec<SetRowData> = Set::ALL
        .iter()
        .filter_map(|set| {
            // Collect card versions in this set that pass the filter.
            let matching_cvs: Vec<&CardVersion> = set
                .card_versions()
                .iter()
                .filter(|cv| passes_filter(cv, &cfg, &settings_guard, today, name_ids.as_deref()))
                .collect();

            if matching_cvs.is_empty() {
                return None;
            }

            let total = matching_cvs.len();
            let owned = matching_cvs
                .iter()
                .filter(|cv| effective_count(cv.id(), &cfg, store_ref) >= goal)
                .count();

            let comp = if merge_dupes {
                completion_merged(
                    counts,
                    goal,
                    matching_cvs.iter().map(|cv| cv.id()),
                )
            } else {
                completion(counts, goal, matching_cvs.iter().map(|cv| cv.id()))
            };

            let obtainable = set_is_obtainable(set, today);

            let desired_ids_for_set: Vec<CardVersionId> = matching_cvs
                .iter()
                .filter(|cv| effective_count(cv.id(), &cfg, store_ref) < goal)
                .map(|cv| cv.id())
                .collect();

            let (best_pack, best_rate_pct) = if set.is_promo() || desired_ids_for_set.is_empty() {
                (None, 0.0)
            } else {
                let result = set
                    .packs()
                    .iter()
                    .filter_map(|p| {
                        let rate = desired_pull_rate(p, |id| desired_ids_for_set.contains(&id));
                        if rate == Prob::ZERO { None } else { Some((p, rate)) }
                    })
                    .max_by(|(_, a), (_, b)| a.cmp(b));
                match result {
                    Some((pack, rate)) => (Some(pack), rate.as_f64() * 100.0),
                    None => (None, 0.0),
                }
            };

            let pack_rows: Vec<PackRowData> = set
                .packs()
                .iter()
                .map(|p| {
                    let p_matching: Vec<&CardVersion> = p
                        .card_versions()
                        .iter()
                        .filter(|cv| {
                            passes_filter(cv, &cfg, &settings_guard, today, name_ids.as_deref())
                        })
                        .collect();
                    let p_total = p_matching.len();
                    let p_owned = p_matching
                        .iter()
                        .filter(|cv| effective_count(cv.id(), &cfg, store_ref) >= goal)
                        .count();

                    let p_comp = if merge_dupes {
                        completion_merged(counts, goal, p_matching.iter().map(|cv| cv.id()))
                    } else {
                        completion(counts, goal, p_matching.iter().map(|cv| cv.id()))
                    };

                    let p_desired: Vec<CardVersionId> = p_matching
                        .iter()
                        .filter(|cv| effective_count(cv.id(), &cfg, store_ref) < goal)
                        .map(|cv| cv.id())
                        .collect();
                    let p_rate = desired_pull_rate(p, |id| p_desired.contains(&id));

                    PackRowData {
                        pack: p,
                        completion_pct: p_comp.as_f64() * 100.0,
                        owned: p_owned,
                        total: p_total,
                        rate_pct: p_rate.as_f64() * 100.0,
                    }
                })
                .collect();

            Some(SetRowData {
                set,
                completion_pct: comp.as_f64() * 100.0,
                owned,
                total,
                obtainable,
                best_pack,
                best_rate_pct,
                pack_rows,
            })
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

    // ── Best pack overall ────────────────────────────────────────────────────

    let all_desired_ids: Vec<CardVersionId> = set_rows
        .iter()
        .flat_map(|row| {
            row.set
                .card_versions()
                .iter()
                .filter(|cv| {
                    passes_filter(cv, &cfg, &settings_guard, today, name_ids.as_deref())
                        && effective_count(cv.id(), &cfg, store_ref) < goal
                })
                .map(|cv| cv.id())
        })
        .collect();

    let best_rate = Pack::ALL
        .iter()
        .filter(|p| !p.set().is_promo())
        .filter_map(|p| {
            let rate = desired_pull_rate(p, |id| all_desired_ids.contains(&id));
            if rate == Prob::ZERO { None } else { Some((p, rate)) }
        })
        .max_by(|(_, a), (_, b)| a.cmp(b));

    let best_packs: Vec<(&'static Pack, Prob)> = if let Some((_, best)) = best_rate {
        Pack::ALL
            .iter()
            .filter(|p| !p.set().is_promo())
            .filter_map(|p| {
                let rate = desired_pull_rate(p, |id| all_desired_ids.contains(&id));
                if rate == best { Some((p, rate)) } else { None }
            })
            .collect()
    } else {
        vec![]
    };

    let collection_complete =
        best_packs.is_empty() && total_cards > 0 && total_owned == total_cards;

    let next_pack_cls = if best_packs.len() > 2 {
        "divide-y divide-gray-200 dark:divide-gray-700 max-h-96 overflow-y-auto"
    } else {
        "divide-y divide-gray-200 dark:divide-gray-700"
    };

    // Drop read guards before rendering to avoid holding them across RSX.
    drop(cfg);
    drop(settings_guard);
    drop(store_guard);

    rsx! {
        div { class: "max-w-4xl mx-auto p-4 sm:p-6 space-y-6",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Summary" }

            // ── Filter toolbar + saved queries controls ───────────────────────
            div { class: "flex flex-col gap-1.5",
                FilterToolbar { config, mode: FilterMode::Analysis }
                SavedQueriesControls { config }
            }

            // ── Overall totals ────────────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3",
                    "Overall"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4 space-y-3",
                    if total_cards == 0 {
                        p { class: "text-sm text-gray-500 dark:text-gray-400",
                            "No cards match the current filters."
                        }
                    } else {
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
            }

            // ── Next pack ─────────────────────────────────────────────────────
            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3",
                    "Next pack to open"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4",
                    if collection_complete {
                        p { class: "text-sm font-medium text-green-600 dark:text-green-400",
                            "Goal met for all matching cards!"
                        }
                    } else if best_packs.is_empty() && total_cards > 0 {
                        p { class: "text-sm text-gray-500 dark:text-gray-400",
                            "No packs can yield the desired cards."
                        }
                    } else if best_packs.is_empty() {
                        p { class: "text-sm text-gray-500 dark:text-gray-400",
                            "No cards match the current filters."
                        }
                    } else {
                        div { class: "{next_pack_cls}",
                            for (pack, rate) in best_packs.iter().copied() {
                                div {
                                    key: "{pack.id()}",
                                    class: "flex items-start gap-4 py-4 cursor-pointer rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700/60",
                                    onclick: move |_| {
                                        apply_pack_filter(pack.id(), catalog_filter);
                                        drop(nav.push(Route::CatalogPage {}));
                                    },
                                    img {
                                        src: "{pack.image()}",
                                        alt: "{pack.title()}",
                                        class: "h-40 w-auto object-contain shrink-0",
                                    }
                                    div { class: "flex flex-col gap-1",
                                        div { class: "flex items-center gap-2",
                                            img {
                                                src: "{pack.set().icon()}",
                                                alt: "",
                                                class: "h-5 w-auto max-w-14 object-contain shrink-0",
                                            }
                                            span { class: "text-lg font-semibold text-gray-900 dark:text-gray-100",
                                                "{pack.title()}"
                                            }
                                        }
                                        span { class: "text-sm text-gray-500 dark:text-gray-400",
                                            "{rate.as_f64() * 100.0:.2}% chance of a desired card"
                                        }
                                    }
                                }
                            }
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
                    if set_rows.is_empty() {
                        p { class: "px-4 py-6 text-sm text-gray-500 dark:text-gray-400",
                            "No sets match the current filters."
                        }
                    } else {
                        for row in set_rows {
                            SetCompletionRow {
                                key: "{row.set.id()}",
                                set: row.set,
                                completion_pct: row.completion_pct,
                                owned: row.owned,
                                total: row.total,
                                is_obtainable: row.obtainable,
                                best_pack: row.best_pack,
                                best_rate_pct: row.best_rate_pct,
                                pack_rows: row.pack_rows,
                            }
                        }
                    }
                }
            }
        }
    }
}

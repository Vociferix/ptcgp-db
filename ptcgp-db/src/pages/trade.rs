use std::cmp::Reverse;
use std::collections::HashSet;

use chrono::NaiveDate;
use dioxus::prelude::*;
use ptcgp_db_core::{
    AppSettings, ProfileStore, max_card_pull_rate,
    save_data::{CardKindFilter, FilterConfig},
};
use ptcgp_db_data::{Card, CardVersion, Prob};

use crate::app::{AppStorage, CardDetailOrigin, TradePageState, schedule_save};
use crate::components::toggle::Toggle;
use crate::components::{FilterMode, FilterToolbar};
use crate::routes::Route;

// ---------------------------------------------------------------------------
// Filter + count helpers
// ---------------------------------------------------------------------------

fn today_naive() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

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
    if !cfg.elements.is_empty()
        && pkmn.is_none_or(|p| !cfg.elements.contains(&p.element().id()))
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

/// Raw aggregate count for destination (active profiles), with optional merge/any-version.
fn raw_dest_count(
    cv: &CardVersion,
    store: &ProfileStore<AppStorage>,
    merge_dupes: bool,
    any_version: bool,
) -> u32 {
    if any_version {
        cv.card()
            .versions()
            .iter()
            .map(|v| store.aggregate_count(v.id()))
            .fold(0u32, u32::saturating_add)
    } else if merge_dupes {
        let mut total = store.aggregate_count(cv.id());
        for dup in cv.duplicates() {
            total = total.saturating_add(store.aggregate_count(dup.id()));
        }
        total
    } else {
        store.aggregate_count(cv.id())
    }
}

/// Raw count for a specific (source) profile, with optional merge_dupes.
fn raw_source_count(
    cv: &CardVersion,
    store: &ProfileStore<AppStorage>,
    profile_name: &str,
    merge_dupes: bool,
) -> u32 {
    if merge_dupes {
        let mut total = store.owned_count(profile_name, cv.id());
        for dup in cv.duplicates() {
            total = total.saturating_add(store.owned_count(profile_name, dup.id()));
        }
        total
    } else {
        store.owned_count(profile_name, cv.id())
    }
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct SourceInfo {
    name: String,
    count: u32,
}

/// A recommended share: one card the destination needs, with a source profile.
#[derive(Clone, PartialEq)]
struct ShareRec {
    cv: &'static CardVersion,
    dest_count: u32,
    needed: u32,
    max_rate: Prob,
    /// True when max_pull_rate == 0 (top-priority tier).
    is_zero_rate: bool,
    best_source: SourceInfo,
    alt_sources: Vec<SourceInfo>,
}

/// A recommended trade: two-sided exchange between source profile and destination.
#[derive(Clone, PartialEq)]
struct TradeRec {
    source_name: String,
    /// Card the destination receives from the source.
    card_b: &'static CardVersion,
    card_b_dest_count: u32,
    card_b_source_count: u32,
    card_b_max_rate: Prob,
    card_b_receive_value: f64,
    /// Card the destination gives to the source.
    card_a: &'static CardVersion,
    card_a_dest_count: u32,
    card_a_source_count: u32,
    card_a_max_rate: Prob,
}

/// A card from the destination collection that is a good candidate to give in trades.
#[derive(Clone, PartialEq)]
struct CandidateRec {
    cv: &'static CardVersion,
    dest_count: u32,
    excess: u32,
    max_rate: Prob,
    is_unobtainable: bool,
}

// ---------------------------------------------------------------------------
// Computation
// ---------------------------------------------------------------------------

fn build_shares(
    store: &ProfileStore<AppStorage>,
    settings: &AppSettings,
    cfg: &FilterConfig,
    today: NaiveDate,
    inactive_names: &[String],
    matched_name_ids: Option<&[usize]>,
) -> Vec<ShareRec> {
    let goal = cfg.goal.max(1);
    let merge_dupes = settings.merge_duplicate_printings();
    let any_version = cfg.any_version_owned;
    let mut recs: Vec<ShareRec> = Vec::new();

    for cv in CardVersion::ALL {
        if merge_dupes && !cv.is_original() && !cv.duplicates().is_empty() {
            continue;
        }
        if !cv.is_tradable() {
            continue;
        }
        if cv.rarity().group().name().as_str() != "Diamond" {
            continue;
        }
        if !passes_filter(cv, cfg, settings, today, matched_name_ids) {
            continue;
        }

        let raw = raw_dest_count(cv, store, merge_dupes, any_version);
        if raw >= goal {
            continue;
        }
        let needed = goal - raw;

        let mut sources: Vec<SourceInfo> = inactive_names
            .iter()
            .filter_map(|name| {
                let cnt = raw_source_count(cv, store, name, merge_dupes);
                if cnt > 0 { Some(SourceInfo { name: name.clone(), count: cnt }) } else { None }
            })
            .collect();

        if sources.is_empty() {
            continue;
        }

        sources.sort_by_key(|s| Reverse(s.count));
        let best_source = sources.remove(0);
        let alt_sources = sources;

        let max_rate = max_card_pull_rate(cv.id());
        let is_zero_rate = max_rate == Prob::ZERO;

        recs.push(ShareRec { cv, dest_count: raw, needed, max_rate, is_zero_rate, best_source, alt_sources });
    }

    recs.sort_by(|a, b| match (a.is_zero_rate, b.is_zero_rate) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (true, true) => a.needed.cmp(&b.needed),
        (false, false) => {
            let va = 1.0 / (a.max_rate.as_f64() * a.needed as f64);
            let vb = 1.0 / (b.max_rate.as_f64() * b.needed as f64);
            vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    recs
}

fn build_trades(
    store: &ProfileStore<AppStorage>,
    settings: &AppSettings,
    cfg: &FilterConfig,
    today: NaiveDate,
    inactive_names: &[String],
    matched_name_ids: Option<&[usize]>,
) -> Vec<TradeRec> {
    let goal = cfg.goal.max(1);
    let merge_dupes = settings.merge_duplicate_printings();
    let any_version = cfg.any_version_owned;

    struct CardData {
        cv: &'static CardVersion,
        dest_raw: u32,
        rarity_class_id: usize,
        max_rate: Prob,
    }

    let card_data: Vec<CardData> = CardVersion::ALL
        .iter()
        .filter(|cv| {
            if merge_dupes && !cv.is_original() && !cv.duplicates().is_empty() {
                return false;
            }
            cv.is_tradable() && passes_filter(cv, cfg, settings, today, matched_name_ids)
        })
        .map(|cv| CardData {
            cv,
            dest_raw: raw_dest_count(cv, store, merge_dupes, any_version),
            rarity_class_id: cv.rarity().class().id(),
            max_rate: max_card_pull_rate(cv.id()),
        })
        .collect();

    let mut recs: Vec<TradeRec> = Vec::new();

    for source_name in inactive_names {
        let src_counts: Vec<u32> = card_data
            .iter()
            .map(|d| raw_source_count(d.cv, store, source_name, merge_dupes))
            .collect();

        let rarity_class_ids: Vec<usize> = {
            let mut seen: HashSet<usize> = HashSet::new();
            card_data.iter().map(|d| d.rarity_class_id).filter(|&id| seen.insert(id)).collect()
        };

        for rarity_class_id in rarity_class_ids {
            let best_b = card_data
                .iter()
                .zip(src_counts.iter())
                .filter(|(d, src_cnt)| {
                    d.rarity_class_id == rarity_class_id && d.dest_raw < goal && **src_cnt > 0
                })
                .max_by(|(da, _), (db, _)| {
                    match (da.max_rate == Prob::ZERO, db.max_rate == Prob::ZERO) {
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        _ => {
                            let va = if da.max_rate == Prob::ZERO {
                                f64::INFINITY
                            } else {
                                1.0 / (da.max_rate.as_f64() * (goal - da.dest_raw) as f64)
                            };
                            let vb = if db.max_rate == Prob::ZERO {
                                f64::INFINITY
                            } else {
                                1.0 / (db.max_rate.as_f64() * (goal - db.dest_raw) as f64)
                            };
                            va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                        }
                    }
                });

            let Some((b_data, b_src_count_ref)) = best_b else { continue };
            let b_src_count = *b_src_count_ref;

            let b_receive_value = if b_data.max_rate == Prob::ZERO {
                f64::INFINITY
            } else {
                1.0 / (b_data.max_rate.as_f64() * (goal - b_data.dest_raw) as f64)
            };

            let best_a = card_data
                .iter()
                .zip(src_counts.iter())
                .filter(|(d, src_cnt)| {
                    d.rarity_class_id == rarity_class_id
                        && d.dest_raw > goal
                        && **src_cnt < goal
                        && d.max_rate != Prob::ZERO
                })
                .min_by(|(da, _), (db, _)| {
                    let va = 1.0 / (da.max_rate.as_f64() * (da.dest_raw - goal) as f64);
                    let vb = 1.0 / (db.max_rate.as_f64() * (db.dest_raw - goal) as f64);
                    va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
                });

            let Some((a_data, a_src_count_ref)) = best_a else { continue };
            let a_src_count = *a_src_count_ref;

            recs.push(TradeRec {
                source_name: source_name.clone(),
                card_b: b_data.cv,
                card_b_dest_count: b_data.dest_raw,
                card_b_source_count: b_src_count,
                card_b_max_rate: b_data.max_rate,
                card_b_receive_value: b_receive_value,
                card_a: a_data.cv,
                card_a_dest_count: a_data.dest_raw,
                card_a_source_count: a_src_count,
                card_a_max_rate: a_data.max_rate,
            });
        }
    }

    recs.sort_by(|a, b| {
        b.card_b_receive_value
            .partial_cmp(&a.card_b_receive_value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    recs
}

fn build_candidates(
    store: &ProfileStore<AppStorage>,
    settings: &AppSettings,
    cfg: &FilterConfig,
    today: NaiveDate,
    matched_name_ids: Option<&[usize]>,
    show_unobtainable: bool,
) -> Vec<CandidateRec> {
    let goal = cfg.goal.max(1);
    let merge_dupes = settings.merge_duplicate_printings();
    let any_version = cfg.any_version_owned;
    let mut recs: Vec<CandidateRec> = Vec::new();

    for cv in CardVersion::ALL {
        if merge_dupes && !cv.is_original() && !cv.duplicates().is_empty() {
            continue;
        }
        if !passes_filter(cv, cfg, settings, today, matched_name_ids) {
            continue;
        }

        let raw = raw_dest_count(cv, store, merge_dupes, any_version);
        if raw <= goal {
            continue;
        }
        let excess = raw - goal;

        let max_rate = max_card_pull_rate(cv.id());
        if max_rate == Prob::ZERO {
            continue;
        }

        let is_unobtainable = cv.set().retirement_date().is_some_and(|d| d <= today);
        if is_unobtainable && !show_unobtainable {
            continue;
        }

        recs.push(CandidateRec { cv, dest_count: raw, excess, max_rate, is_unobtainable });
    }

    recs.sort_by(|a, b| {
        let va = 1.0 / (a.max_rate.as_f64() * a.excess as f64);
        let vb = 1.0 / (b.max_rate.as_f64() * b.excess as f64);
        va.partial_cmp(&vb).unwrap_or(std::cmp::Ordering::Equal)
    });
    recs
}

// ---------------------------------------------------------------------------
// Tab state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Shares,
    Trades,
    Candidates,
}

#[component]
fn TabBtn(label: &'static str, tab: Tab, active_tab: Signal<Tab>) -> Element {
    let is_active = *active_tab.read() == tab;
    let cls = if is_active {
        "px-4 py-2.5 text-sm font-medium border-b-2 border-blue-600 text-blue-600 \
         dark:text-blue-400 dark:border-blue-400 whitespace-nowrap"
    } else {
        "px-4 py-2.5 text-sm font-medium border-b-2 border-transparent \
         text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 \
         whitespace-nowrap"
    };
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            onclick: move |_| active_tab.set(tab),
            "{label}"
        }
    }
}

// ---------------------------------------------------------------------------
// Shared card panel
// ---------------------------------------------------------------------------

/// Card image + name + code + set/rarity icons, sized to match the catalog list.
#[component]
fn CardPanel(cv_id: usize) -> Element {
    let Some(cv) = CardVersion::from_id(cv_id) else {
        return rsx! {};
    };
    let card_name = cv.card().name();
    let set_code = cv.set().code();
    let number = cv.number().get();
    let rarity_icon = cv.rarity().class().icon();
    let set_icon = cv.set().icon();
    let card_image = cv.image();
    rsx! {
        div { class: "flex items-center gap-3 min-w-0",
            img {
                src: "{card_image}",
                alt: "{card_name}",
                class: "w-14 h-20 object-cover rounded flex-shrink-0",
            }
            div { class: "min-w-0",
                p { class: "text-sm font-semibold text-gray-900 dark:text-gray-100 truncate",
                    "{card_name}"
                }
                p { class: "text-xs text-gray-500 dark:text-gray-400 mt-0.5",
                    "{set_code} {number:03}"
                }
                div { class: "flex items-center gap-1.5 mt-1",
                    img {
                        src: "{set_icon}",
                        alt: "",
                        class: "h-5 w-auto max-w-14 object-contain flex-shrink-0",
                    }
                    img {
                        src: "{rarity_icon}",
                        alt: "",
                        class: "h-5 w-auto object-contain flex-shrink-0",
                    }
                }
            }
        }
    }
}

fn pull_rate_label(rate: Prob) -> String {
    if rate == Prob::ZERO { "—".to_string() } else { format!("{:.3}%", rate.as_f64() * 100.0) }
}

// ---------------------------------------------------------------------------
// Shares tab
// ---------------------------------------------------------------------------

#[component]
fn ShareRow(rank: usize, rec: ShareRec, dest_name: String, disabled: bool) -> Element {
    let mut store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let mut back_origin = use_context::<Signal<CardDetailOrigin>>();
    let nav = use_navigator();
    let cv_id = rec.cv.id();
    let source_name = rec.best_source.name.clone();
    let dest_for_xfer = dest_name.clone();
    let on_transfer = move |e: Event<MouseData>| {
        e.stop_propagation();
        let mut s = store.write();
        if let Some(st) = s.as_mut() {
            let src_c = st.owned_count(&source_name, cv_id);
            let _ = st.set_owned_count(&source_name, cv_id, src_c.saturating_sub(1));
            let dst_c = st.owned_count(&dest_for_xfer, cv_id);
            let _ = st.set_owned_count(&dest_for_xfer, cv_id, dst_c + 1);
        }
        schedule_save();
    };

    rsx! {
        div {
            class: "flex gap-3 p-4 border-b border-gray-100 dark:border-gray-700 last:border-0 \
                    cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700/50",
            onclick: move |_| {
                back_origin.set(CardDetailOrigin::Trade);
                drop(
                    nav
                        .push(Route::CardDetailPage {
                            card_id: cv_id,
                        }),
                );
            },
            // Rank badge
            span { class: "shrink-0 w-8 h-8 flex items-center justify-center rounded-full text-xs font-bold bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 mt-6",
                "#{rank}"
            }
            // Card panel
            div { class: "flex-1 min-w-0",
                CardPanel { cv_id: rec.cv.id() }
                // Priority badge
                if rec.is_zero_rate {
                    span { class: "inline-flex items-center mt-1.5 px-1.5 py-0.5 rounded text-xs font-medium bg-amber-100 dark:bg-amber-900/40 text-amber-800 dark:text-amber-200",
                        "Priority — not obtainable from packs"
                    }
                }
            }
            // Stats + transfer
            div { class: "shrink-0 flex flex-col items-end gap-1.5 min-w-[11rem]",
                button {
                    r#type: "button",
                    class: "px-3 py-1.5 text-xs font-medium rounded-md bg-blue-600 text-white \
                            hover:bg-blue-700 disabled:opacity-40 disabled:cursor-not-allowed",
                    disabled,
                    onclick: on_transfer,
                    "Transfer"
                }
                // Source profile
                div { class: "text-xs text-right",
                    span { class: "text-gray-500 dark:text-gray-400", "Source: " }
                    span { class: "font-medium text-gray-800 dark:text-gray-200",
                        "{rec.best_source.name}"
                    }
                    span { class: "text-gray-500 dark:text-gray-400", " ({rec.best_source.count} owned)" }
                }
                // Dest profile
                div { class: "text-xs text-right",
                    span { class: "text-gray-500 dark:text-gray-400", "Dest: " }
                    span { class: "font-medium text-gray-800 dark:text-gray-200", "{dest_name}" }
                    span { class: "text-gray-500 dark:text-gray-400", " ({rec.dest_count} owned)" }
                }
                // Pull rate
                div { class: "text-xs text-right text-gray-500 dark:text-gray-400",
                    "Pull rate: {pull_rate_label(rec.max_rate)}"
                }
                // Alt sources
                if !rec.alt_sources.is_empty() {
                    div { class: "text-xs text-right text-gray-400 dark:text-gray-500",
                        "Also: "
                        for (i, alt) in rec.alt_sources.iter().enumerate() {
                            if i > 0 {
                                ", "
                            }
                            "{alt.name} ({alt.count})"
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Trades tab
// ---------------------------------------------------------------------------

/// Compact card half-panel for use inside the two-column trade layout.
#[component]
fn TradeCardHalf(
    cv_id: usize,
    you_label: String,
    you_count: u32,
    other_label: String,
    other_count: u32,
    max_rate: Prob,
) -> Element {
    let Some(cv) = CardVersion::from_id(cv_id) else {
        return rsx! {};
    };
    let card_name = cv.card().name();
    let set_code = cv.set().code();
    let number = cv.number().get();
    let rarity_icon = cv.rarity().class().icon();
    let set_icon = cv.set().icon();
    let card_image = cv.image();
    rsx! {
        div { class: "flex gap-2",
            img {
                src: "{card_image}",
                alt: "{card_name}",
                class: "w-14 h-20 object-cover rounded flex-shrink-0",
            }
            div { class: "min-w-0 flex flex-col gap-0.5",
                p { class: "text-sm font-semibold text-gray-900 dark:text-gray-100 truncate",
                    "{card_name}"
                }
                p { class: "text-xs text-gray-500 dark:text-gray-400", "{set_code} {number:03}" }
                div { class: "flex items-center gap-1",
                    img {
                        src: "{set_icon}",
                        alt: "",
                        class: "h-5 w-auto max-w-14 object-contain flex-shrink-0",
                    }
                    img {
                        src: "{rarity_icon}",
                        alt: "",
                        class: "h-5 w-auto object-contain flex-shrink-0",
                    }
                }
                p { class: "text-xs text-gray-600 dark:text-gray-300",
                    "{you_label}: {you_count} owned"
                }
                p { class: "text-xs text-gray-600 dark:text-gray-300",
                    "{other_label}: {other_count} owned"
                }
                p { class: "text-xs text-gray-400 dark:text-gray-500",
                    "Pull: {pull_rate_label(max_rate)}"
                }
            }
        }
    }
}

#[component]
fn TradeRow(rank: usize, rec: TradeRec, dest_name: String, disabled: bool) -> Element {
    let mut store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let mut back_origin = use_context::<Signal<CardDetailOrigin>>();
    let nav = use_navigator();
    let cv_b_id = rec.card_b.id();
    let cv_a_id = rec.card_a.id();
    let source = rec.source_name.clone();
    let dest_for_xfer = dest_name.clone();

    let on_transfer = move |e: Event<MouseData>| {
        e.stop_propagation();
        let mut s = store.write();
        if let Some(st) = s.as_mut() {
            let b_src = st.owned_count(&source, cv_b_id);
            let _ = st.set_owned_count(&source, cv_b_id, b_src.saturating_sub(1));
            let b_dst = st.owned_count(&dest_for_xfer, cv_b_id);
            let _ = st.set_owned_count(&dest_for_xfer, cv_b_id, b_dst + 1);
            let a_dst = st.owned_count(&dest_for_xfer, cv_a_id);
            let _ = st.set_owned_count(&dest_for_xfer, cv_a_id, a_dst.saturating_sub(1));
            let a_src = st.owned_count(&source, cv_a_id);
            let _ = st.set_owned_count(&source, cv_a_id, a_src + 1);
        }
        schedule_save();
    };

    let rarity_icon = rec.card_b.rarity().class().icon();

    rsx! {
        div { class: "p-4 border-b border-gray-100 dark:border-gray-700 last:border-0",
            // Header row: rank + profiles + rarity icon + Transfer button
            div { class: "flex items-center gap-2 mb-3",
                span { class: "shrink-0 w-8 h-8 flex items-center justify-center rounded-full text-xs font-bold bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300",
                    "#{rank}"
                }
                div { class: "flex-1 min-w-0 flex items-center gap-1.5 flex-wrap",
                    span { class: "text-xs font-medium text-gray-800 dark:text-gray-200",
                        "{rec.source_name}"
                    }
                    span { class: "text-xs text-gray-400 dark:text-gray-500", "→" }
                    span { class: "text-xs font-medium text-gray-800 dark:text-gray-200",
                        "{dest_name}"
                    }
                    img {
                        src: "{rarity_icon}",
                        alt: "",
                        class: "h-5 w-auto object-contain flex-shrink-0 ml-1",
                    }
                }
                button {
                    r#type: "button",
                    class: "shrink-0 px-3 py-1.5 text-xs font-medium rounded-md bg-blue-600 \
                            text-white hover:bg-blue-700 disabled:opacity-40 \
                            disabled:cursor-not-allowed",
                    disabled,
                    onclick: on_transfer,
                    "Transfer"
                }
            }
            // Two-column card layout — each box navigates to that card's detail page
            div { class: "grid grid-cols-2 gap-3",
                div {
                    class: "bg-green-50 dark:bg-green-950/20 rounded-md p-2 cursor-pointer \
                            hover:bg-green-100 dark:hover:bg-green-900/50",
                    onclick: move |_| {
                        back_origin.set(CardDetailOrigin::Trade);
                        drop(
                            nav
                                .push(Route::CardDetailPage {
                                    card_id: cv_b_id,
                                }),
                        );
                    },
                    p { class: "text-xs font-semibold text-green-700 dark:text-green-400 mb-2",
                        "You receive"
                    }
                    TradeCardHalf {
                        cv_id: rec.card_b.id(),
                        you_label: dest_name.clone(),
                        you_count: rec.card_b_dest_count,
                        other_label: rec.source_name.clone(),
                        other_count: rec.card_b_source_count,
                        max_rate: rec.card_b_max_rate,
                    }
                }
                div {
                    class: "bg-red-50 dark:bg-red-950/20 rounded-md p-2 cursor-pointer \
                            hover:bg-red-100 dark:hover:bg-red-900/50",
                    onclick: move |_| {
                        back_origin.set(CardDetailOrigin::Trade);
                        drop(
                            nav
                                .push(Route::CardDetailPage {
                                    card_id: cv_a_id,
                                }),
                        );
                    },
                    p { class: "text-xs font-semibold text-red-700 dark:text-red-400 mb-2",
                        "You give"
                    }
                    TradeCardHalf {
                        cv_id: rec.card_a.id(),
                        you_label: dest_name.clone(),
                        you_count: rec.card_a_dest_count,
                        other_label: rec.source_name.clone(),
                        other_count: rec.card_a_source_count,
                        max_rate: rec.card_a_max_rate,
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Candidates tab
// ---------------------------------------------------------------------------

#[component]
fn CandidateRow(rank: usize, rec: CandidateRec, dest_name: String) -> Element {
    let mut back_origin = use_context::<Signal<CardDetailOrigin>>();
    let nav = use_navigator();
    let cv_id = rec.cv.id();
    rsx! {
        div {
            class: "flex gap-3 p-4 border-b border-gray-100 dark:border-gray-700 last:border-0 \
                    cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700/50",
            onclick: move |_| {
                back_origin.set(CardDetailOrigin::Trade);
                drop(
                    nav
                        .push(Route::CardDetailPage {
                            card_id: cv_id,
                        }),
                );
            },
            span { class: "shrink-0 w-8 h-8 flex items-center justify-center rounded-full text-xs font-bold bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 mt-6",
                "#{rank}"
            }
            div { class: "flex-1 min-w-0",
                CardPanel { cv_id: rec.cv.id() }
                if rec.is_unobtainable {
                    span { class: "inline-flex items-center mt-1.5 px-1.5 py-0.5 rounded text-xs font-medium bg-orange-100 dark:bg-orange-900/40 text-orange-800 dark:text-orange-200",
                        "Retired set — cannot be re-obtained from packs"
                    }
                }
            }
            div { class: "shrink-0 flex flex-col items-end gap-1.5",
                div { class: "text-xs text-right",
                    span { class: "text-gray-500 dark:text-gray-400", "{dest_name}: " }
                    span { class: "font-medium text-gray-800 dark:text-gray-200",
                        "{rec.dest_count} owned"
                    }
                    span { class: "text-gray-500 dark:text-gray-400", " ({rec.excess} excess)" }
                }
                div { class: "text-xs text-right text-gray-500 dark:text-gray-400",
                    "Pull rate: {pull_rate_label(rec.max_rate)}"
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Empty state
// ---------------------------------------------------------------------------

fn empty_state_sources(single_profile: bool) -> Element {
    rsx! {
        div { class: "p-6 text-sm text-gray-500 dark:text-gray-400",
            if single_profile {
                "Create a second profile to see recommendations. Shares and trades work between an inactive profile (source) and your active profiles (destination)."
            } else {
                "Deselect at least one profile to use it as a source. Active profiles are the destination; inactive profiles are sources."
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Trade page
// ---------------------------------------------------------------------------

#[component]
pub fn TradePage() -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let settings = use_context::<Signal<AppSettings>>();

    let mut trade_state_ctx = use_context::<Signal<TradePageState>>();
    let init = trade_state_ctx.read();
    let config: Signal<FilterConfig> = use_signal(|| init.config.clone());
    let mut show_unobtainable = use_signal(|| init.show_unobtainable);
    let active_tab: Signal<Tab> = use_signal(|| match init.active_tab {
        1 => Tab::Trades,
        2 => Tab::Candidates,
        _ => Tab::Shares,
    });
    drop(init);

    use_drop(move || {
        let mut state = trade_state_ctx.write();
        state.config = config.read().clone();
        state.show_unobtainable = *show_unobtainable.read();
        state.active_tab = match *active_tab.read() {
            Tab::Shares => 0,
            Tab::Trades => 1,
            Tab::Candidates => 2,
        };
    });

    let store_guard = store.read();
    let settings_guard = settings.read();
    let cfg = config.read();

    let Some(store_ref) = store_guard.as_ref() else {
        return rsx! {
            div { class: "p-4 text-gray-500 dark:text-gray-400", "Loading…" }
        };
    };

    let today = today_naive();

    let active_set: HashSet<&str> = store_ref
        .active_profile_names()
        .iter()
        .map(|s| s.as_str())
        .collect();
    let inactive_names: Vec<String> = store_ref
        .profiles()
        .iter()
        .filter(|p| !active_set.contains(p.name.as_str()))
        .map(|p| p.name.clone())
        .collect();

    let has_sources = !inactive_names.is_empty();
    let single_profile = store_ref.profiles().len() == 1;
    let multi_active = store_ref.active_profile_names().len() > 1;

    // Destination display + transfer target: single active profile name, or "Active profiles".
    let dest_name = match store_ref.active_profile_names() {
        [name] => name.clone(),
        _ => "Active profiles".to_string(),
    };

    let matched_name_ids: Option<Vec<usize>> = cfg
        .name_query
        .as_deref()
        .filter(|q| !q.trim().is_empty())
        .map(|q| Card::NAMES.search(q).map(|e| e.id()).collect());

    let shares = if has_sources {
        build_shares(store_ref, &settings_guard, &cfg, today, &inactive_names, matched_name_ids.as_deref())
    } else {
        Vec::new()
    };

    let trades = if has_sources {
        build_trades(store_ref, &settings_guard, &cfg, today, &inactive_names, matched_name_ids.as_deref())
    } else {
        Vec::new()
    };

    let candidates = build_candidates(
        store_ref,
        &settings_guard,
        &cfg,
        today,
        matched_name_ids.as_deref(),
        *show_unobtainable.read(),
    );

    drop(cfg);
    drop(settings_guard);
    drop(store_guard);

    let card_cls =
        "bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700";

    rsx! {
        div { class: "max-w-4xl mx-auto p-4 sm:p-6 space-y-4",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Trade" }

            FilterToolbar { config, mode: FilterMode::Trade }

            // ── Tab bar ───────────────────────────────────────────────────────
            div { class: "border-b border-gray-200 dark:border-gray-700 overflow-x-auto",
                div { class: "flex min-w-max",
                    TabBtn {
                        label: "Recommended Shares",
                        tab: Tab::Shares,
                        active_tab,
                    }
                    TabBtn {
                        label: "Recommended Trades",
                        tab: Tab::Trades,
                        active_tab,
                    }
                    TabBtn {
                        label: "Trade Candidates",
                        tab: Tab::Candidates,
                        active_tab,
                    }
                }
            }

            // ── Tab content ───────────────────────────────────────────────────
            match *active_tab.read() {
                Tab::Shares => rsx! {
                    div { class: "{card_cls}",
                        if !has_sources {
                            {empty_state_sources(single_profile)}
                        } else if shares.is_empty() {
                            p { class: "p-6 text-sm text-gray-500 dark:text-gray-400",
                                "No sharing recommendations match the current filters."
                            }
                        } else {
                            for (rank, rec) in shares.into_iter().enumerate() {
                                ShareRow {
                                    key: "{rec.cv.id()}",
                                    rank: rank + 1,
                                    rec,
                                    dest_name: dest_name.clone(),
                                    disabled: multi_active,
                                }
                            }
                        }
                    }
                },
                Tab::Trades => rsx! {
                    div { class: "{card_cls}",
                        if !has_sources {
                            {empty_state_sources(single_profile)}
                        } else if trades.is_empty() {
                            p { class: "p-6 text-sm text-gray-500 dark:text-gray-400",
                                "No trading recommendations match the current filters."
                            }
                        } else {
                            for (rank, rec) in trades.into_iter().enumerate() {
                                TradeRow {
                                    key: "{rec.source_name}-{rec.card_b.id()}-{rec.card_a.id()}",
                                    rank: rank + 1,
                                    rec,
                                    dest_name: dest_name.clone(),
                                    disabled: multi_active,
                                }
                            }
                        }
                    }
                },
                Tab::Candidates => rsx! {
                    div {
                        // Unobtainable toggle
                        div { class: "flex items-center gap-2 mb-3",
                            Toggle {
                                checked: *show_unobtainable.read(),
                                on_change: move |v| show_unobtainable.set(v),
                            }
                            span { class: "text-sm text-gray-700 dark:text-gray-300", "Show retired-set cards" }
                        }
                        div { class: "{card_cls}",
                            if candidates.is_empty() {
                                p { class: "p-6 text-sm text-gray-500 dark:text-gray-400",
                                    "No trade candidates match the current filters."
                                }
                            } else {
                                for (rank, rec) in candidates.into_iter().enumerate() {
                                    CandidateRow {
                                        key: "{rec.cv.id()}",
                                        rank: rank + 1,
                                        rec,
                                        dest_name: dest_name.clone(),
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

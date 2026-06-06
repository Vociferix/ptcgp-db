//! Trade page: share/trade recommendations and candidate card lists.

mod history;
mod rows;

use std::collections::HashSet;

use dioxus::prelude::*;
use ptcgp_db_core::save_data::FilterConfig;
use ptcgp_db_core::{AppSettings, ProfileStore, build_candidates, build_shares, build_trades};
use ptcgp_db_data::Card;

use crate::app::{AppStorage, CompletedTransfer, TradePageState};
use crate::components::icons::{ChevronDown, ChevronUp};
use crate::components::toggle::{Toggle, ToggleCheckbox};
use crate::components::{FilterMode, FilterToolbar};

use history::{CompletedShareSection, CompletedTradeSection};
use rows::{CandidateRow, ShareRow, TradeRow};

/// Card container class shared between active lists and completed-transfer section bodies.
pub(super) const CARD_CLS: &str = "bg-white dark:bg-gray-800 rounded-lg border border-gray-200/80 \
    dark:border-gray-700/80 shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] \
    dark:ring-1 dark:ring-white/[0.06]";

const DROPDOWN_TRIGGER_CLS: &str = "flex items-center gap-1 px-2 h-8 rounded-md text-sm font-medium \
     bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 \
     text-gray-800 dark:text-gray-100 shadow-sm active:shadow-none active:translate-y-px";

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
// Source profile dropdown
// ---------------------------------------------------------------------------

/// Multi-select dropdown for choosing which inactive profiles act as sources.
///
/// An empty selection means "all inactive profiles". Selecting specific profiles
/// restricts share/trade results to those sources only.
#[component]
fn SourceProfileDropdown(selected: Signal<Vec<String>>) -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let mut open = use_signal(|| false);

    // Hold the guard across RSX so profile names can be borrowed as &str
    // rather than cloned into a Vec<String>.
    let guard = store.read();
    let Some(ref s) = *guard else {
        return rsx! {};
    };
    let active_names = s.active_profile_names();
    let profiles = s.profiles();

    let sel = selected.read();
    let count = sel
        .iter()
        .filter(|n| profiles.iter().any(|p| &p.name == *n) && !active_names.contains(*n))
        .count();
    drop(sel);

    let open_now = *open.read();

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: DROPDOWN_TRIGGER_CLS,
                onclick: move |_| open.toggle(),
                "Sources"
                if count > 0 {
                    span { class: "px-1.5 py-0.5 text-xs rounded-full bg-blue-600 text-white",
                        "{count}"
                    }
                }
                if open_now {
                    ChevronUp { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                } else {
                    ChevronDown { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                }
            }

            if open_now {
                div {
                    class: "fixed inset-0 z-10",
                    onclick: move |_| open.set(false),
                }
                div { class: "absolute left-0 top-full mt-1 z-20 min-w-48 \
                              max-h-80 overflow-y-auto overflow-x-hidden \
                              rounded-md border border-gray-200/60 dark:border-gray-600/60 \
                              bg-white dark:bg-gray-700 \
                              shadow-xl dark:shadow-[0_4px_28px_rgba(0,0,0,0.7)] \
                              ring-1 ring-black/5 dark:ring-white/[0.09] py-1",
                    if count > 0 {
                        button {
                            r#type: "button",
                            class: "w-full px-3 py-1.5 text-xs text-center \
                                    text-gray-400 dark:text-gray-500 \
                                    hover:text-gray-600 dark:hover:text-gray-300 \
                                    hover:bg-gray-50 dark:hover:bg-gray-600/40 \
                                    active:bg-gray-100 dark:active:bg-gray-500/40 \
                                    border-b border-gray-100 dark:border-gray-600",
                            onclick: move |_| selected.write().clear(),
                            "Clear"
                        }
                    }
                    for profile in profiles.iter().filter(|p| !active_names.contains(&p.name)) {
                        SourceProfileItem {
                            key: "{profile.name}",
                            name: profile.name.clone(),
                            selected,
                            open,
                        }
                    }
                }
            }
        }
    }
}

/// One row in the source profile dropdown.
///
/// Clicking the row body selects only this profile and closes the dropdown
/// (clicking again when already the only selection clears it back to "all").
/// Clicking the checkbox on the right toggles without closing.
#[component]
fn SourceProfileItem(
    name: String,
    selected: Signal<Vec<String>>,
    mut open: Signal<bool>,
) -> Element {
    let checked = selected.read().contains(&name);
    let row_cls = if checked {
        "flex items-center gap-2 px-3 py-2 text-sm cursor-pointer select-none \
         bg-blue-50 dark:bg-blue-950/80 hover:bg-blue-100 dark:hover:bg-blue-900/60"
    } else {
        "flex items-center gap-2 px-3 py-2 text-sm cursor-pointer select-none \
         hover:bg-gray-50 dark:hover:bg-gray-600"
    };
    let on_select = {
        let name = name.clone();
        move |_| {
            let mut sel = selected.write();
            let already_only = sel.len() == 1 && sel[0] == name;
            if already_only {
                sel.clear();
            } else {
                *sel = vec![name.clone()];
            }
            drop(sel);
            open.set(false);
        }
    };
    let on_toggle = {
        let name = name.clone();
        move |e: MouseEvent| {
            e.stop_propagation();
            let mut sel = selected.write();
            if checked {
                sel.retain(|n| n != &name);
            } else {
                sel.push(name.clone());
            }
        }
    };

    rsx! {
        div { class: "{row_cls}", onclick: on_select,
            span { class: "flex-1 truncate text-gray-800 dark:text-gray-100", "{name}" }
            button {
                r#type: "button",
                class: "shrink-0 p-2 -mr-1 rounded \
                        hover:bg-gray-200/60 dark:hover:bg-gray-500/40",
                onclick: on_toggle,
                ToggleCheckbox { checked }
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
    let source_profiles: Signal<Vec<String>> = use_signal(|| init.source_profiles.clone());
    let completed_transfers: Signal<Vec<CompletedTransfer>> =
        use_signal(|| init.completed_transfers.clone());
    let next_id: Signal<u64> = use_signal(|| init.next_transfer_id);
    drop(init);

    let mut shares_limit = use_signal(|| 10usize);
    let mut trades_limit = use_signal(|| 10usize);
    let mut candidates_limit = use_signal(|| 10usize);

    use_drop(move || {
        let mut state = trade_state_ctx.write();
        state.config = config.read().clone();
        state.show_unobtainable = *show_unobtainable.read();
        state.active_tab = match *active_tab.read() {
            Tab::Shares => 0,
            Tab::Trades => 1,
            Tab::Candidates => 2,
        };
        state.source_profiles = source_profiles.read().clone();
        state.completed_transfers = completed_transfers.read().clone();
        state.next_transfer_id = *next_id.read();
    });

    let store_guard = store.read();
    let settings_guard = settings.read();
    let cfg = config.read();

    let Some(store_ref) = store_guard.as_ref() else {
        return rsx! {
            div { class: "p-4 text-gray-500 dark:text-gray-400", "Loading…" }
        };
    };

    let today = chrono::Utc::now().date_naive();

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

    // Restrict to explicitly selected sources when the user has filtered the dropdown;
    // an empty selection means "all inactive profiles".
    // Declare `filtered` here so its lifetime covers the build calls below.
    let sel = source_profiles.read();
    let filtered: Vec<String>;
    let effective_inactive: &[String] = if sel.is_empty() {
        &inactive_names
    } else {
        filtered = inactive_names
            .iter()
            .filter(|n| sel.contains(n))
            .cloned()
            .collect();
        &filtered
    };
    drop(sel);

    let has_sources = !inactive_names.is_empty();
    let has_multiple_sources = inactive_names.len() > 1;
    let single_profile = store_ref.profiles().len() == 1;
    let multi_active = store_ref.active_profile_names().len() > 1;

    let dest_name = match store_ref.active_profile_names() {
        [name] => name.clone(),
        _ => "Active profiles".to_string(),
    };

    let matched_name_ids: Option<Vec<usize>> = cfg
        .name_query
        .as_deref()
        .filter(|q| !q.trim().is_empty())
        .map(|q| Card::NAMES.search(q).map(|e| e.id()).collect());

    let mut shares = if has_sources {
        build_shares(
            store_ref,
            &settings_guard,
            &cfg,
            today,
            effective_inactive,
            matched_name_ids.as_deref(),
        )
    } else {
        Vec::new()
    };

    let mut trades = if has_sources {
        build_trades(
            store_ref,
            &settings_guard,
            &cfg,
            today,
            effective_inactive,
            matched_name_ids.as_deref(),
        )
    } else {
        Vec::new()
    };

    let mut candidates = build_candidates(
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

    let shares_total = shares.len();
    shares.truncate(*shares_limit.read());
    let shares_remaining = shares_total - shares.len();

    let trades_total = trades.len();
    trades.truncate(*trades_limit.read());
    let trades_remaining = trades_total - trades.len();

    let candidates_total = candidates.len();
    candidates.truncate(*candidates_limit.read());
    let candidates_remaining = candidates_total - candidates.len();

    let show_more_cls = "w-full px-4 py-3 text-center text-sm text-blue-600 dark:text-blue-400 \
        hover:bg-gray-50 dark:hover:bg-gray-700/50 border-t border-gray-100 dark:border-gray-700";

    rsx! {
        div { class: "max-w-4xl mx-auto p-4 sm:p-6 space-y-4",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Trade" }

            FilterToolbar { config, mode: FilterMode::Trade }

            div { class: "border-b border-gray-200 dark:border-gray-700 overflow-x-auto",
                div { class: "flex min-w-max",
                    TabBtn { label: "Shares", tab: Tab::Shares, active_tab }
                    TabBtn { label: "Trades", tab: Tab::Trades, active_tab }
                    TabBtn {
                        label: "Candidates",
                        tab: Tab::Candidates,
                        active_tab,
                    }
                }
            }

            match *active_tab.read() {
                Tab::Shares => rsx! {
                    div {
                        if has_multiple_sources {
                            div { class: "flex items-center gap-2 mb-3",
                                SourceProfileDropdown { selected: source_profiles }
                            }
                        }
                        CompletedShareSection { history: completed_transfers, dest_name: dest_name.clone() }
                        div { class: "{CARD_CLS}",
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
                                        history: completed_transfers,
                                        next_id,
                                    }
                                }
                                if shares_remaining > 0 {
                                    button {
                                        r#type: "button",
                                        class: "{show_more_cls}",
                                        onclick: move |_| *shares_limit.write() += 10,
                                        "Show more ({shares_remaining} remaining)"
                                    }
                                }
                            }
                        }
                    }
                },
                Tab::Trades => rsx! {
                    div {
                        if has_multiple_sources {
                            div { class: "flex items-center gap-2 mb-3",
                                SourceProfileDropdown { selected: source_profiles }
                            }
                        }
                        CompletedTradeSection { history: completed_transfers, dest_name: dest_name.clone() }
                        div { class: "{CARD_CLS}",
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
                                        history: completed_transfers,
                                        next_id,
                                    }
                                }
                                if trades_remaining > 0 {
                                    button {
                                        r#type: "button",
                                        class: "{show_more_cls}",
                                        onclick: move |_| *trades_limit.write() += 10,
                                        "Show more ({trades_remaining} remaining)"
                                    }
                                }
                            }
                        }
                    }
                },
                Tab::Candidates => rsx! {
                    div {
                        div { class: "flex items-center gap-2 mb-3",
                            Toggle {
                                checked: *show_unobtainable.read(),
                                on_change: move |v| show_unobtainable.set(v),
                            }
                            span { class: "text-sm text-gray-700 dark:text-gray-300", "Show retired-set cards" }
                        }
                        div { class: "{CARD_CLS}",
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
                                if candidates_remaining > 0 {
                                    button {
                                        r#type: "button",
                                        class: "{show_more_cls}",
                                        onclick: move |_| *candidates_limit.write() += 10,
                                        "Show more ({candidates_remaining} remaining)"
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

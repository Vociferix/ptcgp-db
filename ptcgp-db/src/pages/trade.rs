use std::collections::HashSet;

use dioxus::prelude::*;
use ptcgp_db_core::{
    AppSettings, CandidateRec, ProfileStore, ShareRec, TradeRec, build_candidates, build_shares,
    build_trades,
    save_data::{CardVersionId, FilterConfig},
};
use ptcgp_db_data::{Card, CardVersion, Prob};

use crate::app::{AppStorage, CardDetailOrigin, TradePageState, schedule_save};
use crate::components::icons::{ChevronDown, ChevronUp};
use crate::components::toggle::{Toggle, ToggleCheckbox};
use crate::components::{FilterMode, FilterToolbar};
use crate::routes::Route;

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
// Shared card panel
// ---------------------------------------------------------------------------

/// Card image + name + code + set/rarity icons.
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
                p { class: "text-sm font-semibold text-gray-900 dark:text-gray-100 line-clamp-2",
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
    if rate == Prob::ZERO {
        "—".to_string()
    } else {
        format!("{:.3}%", rate.as_f64() * 100.0)
    }
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
    let rate_label = pull_rate_label(rec.max_rate);
    let source_name = rec.best_source.name.clone();
    let dest_for_xfer = dest_name.clone();
    let on_transfer = use_callback(move |e: Event<MouseData>| {
        e.stop_propagation();
        let mut s = store.write();
        if let Some(st) = s.as_mut() {
            let cv_id = CardVersionId(cv_id);
            let src_c = st.owned_count(&source_name, cv_id);
            let _ = st.set_owned_count(&source_name, cv_id, src_c.saturating_sub(1));
            let dst_c = st.owned_count(&dest_for_xfer, cv_id);
            let _ = st.set_owned_count(&dest_for_xfer, cv_id, dst_c + 1);
        }
        schedule_save();
    });
    let btn_cls = "px-3 py-1.5 text-xs font-medium rounded-md bg-blue-600 text-white \
                   hover:bg-blue-700 disabled:opacity-40 disabled:cursor-not-allowed \
                   shadow-md shadow-blue-500/30 dark:shadow-blue-900/70 active:shadow-sm active:translate-y-px";

    rsx! {
        div {
            class: "p-4 border-b border-gray-100 dark:border-gray-700 \
                    last:border-0 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700/50",
            onclick: move |_| {
                back_origin.set(CardDetailOrigin::Trade);
                drop(
                    nav
                        .push(Route::CardDetailPage {
                            card_id: cv_id,
                        }),
                );
            },
            // Mobile header (hidden sm+): rank + source→dest + Transfer
            div { class: "sm:hidden flex items-center gap-2 mb-3",
                span { class: "shrink-0 w-8 h-8 flex items-center justify-center rounded-full text-xs font-bold bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300",
                    "#{rank}"
                }
                div { class: "flex-1 min-w-0 text-xs",
                    span { class: "font-medium text-gray-800 dark:text-gray-200",
                        "{rec.best_source.name}"
                    }
                    span { class: "text-gray-400 dark:text-gray-500", " → " }
                    span { class: "font-medium text-gray-800 dark:text-gray-200", "{dest_name}" }
                }
                button {
                    r#type: "button",
                    class: "{btn_cls}",
                    disabled,
                    onclick: move |e| on_transfer.call(e),
                    "Transfer"
                }
            }
            // Body: desktop rank badge + card panel + desktop stats sidebar
            div { class: "flex items-start gap-3",
                span { class: "hidden sm:flex shrink-0 w-8 h-8 items-center justify-center rounded-full text-xs font-bold bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300",
                    "#{rank}"
                }
                div { class: "flex-1 min-w-0",
                    CardPanel { cv_id }
                    div { class: "sm:hidden mt-1 text-xs text-gray-500 dark:text-gray-400",
                        "Pull rate: {rate_label}"
                    }
                    if !rec.alt_sources.is_empty() {
                        div { class: "sm:hidden mt-0.5 text-xs text-gray-400 dark:text-gray-500 break-words",
                            "Also: "
                            for (i, alt) in rec.alt_sources.iter().enumerate() {
                                if i > 0 {
                                    ", "
                                }
                                "{alt.name} ({alt.count})"
                            }
                        }
                    }
                    if rec.is_zero_rate {
                        span { class: "inline-flex items-center mt-1.5 px-1.5 py-0.5 rounded text-xs font-medium bg-amber-100 dark:bg-amber-900/40 text-amber-800 dark:text-amber-200",
                            "Priority — not obtainable from packs"
                        }
                    }
                }
                div { class: "hidden sm:flex flex-col items-end gap-1.5 shrink-0 min-w-[11rem]",
                    button {
                        r#type: "button",
                        class: "{btn_cls}",
                        disabled,
                        onclick: move |e| on_transfer.call(e),
                        "Transfer"
                    }
                    div { class: "text-xs text-right",
                        span { class: "text-gray-500 dark:text-gray-400", "Source: " }
                        span { class: "font-medium text-gray-800 dark:text-gray-200",
                            "{rec.best_source.name}"
                        }
                        span { class: "text-gray-500 dark:text-gray-400",
                            " ({rec.best_source.count} owned)"
                        }
                    }
                    div { class: "text-xs text-right",
                        span { class: "text-gray-500 dark:text-gray-400", "Dest: " }
                        span { class: "font-medium text-gray-800 dark:text-gray-200",
                            "{dest_name}"
                        }
                        span { class: "text-gray-500 dark:text-gray-400", " ({rec.dest_count} owned)" }
                    }
                    div { class: "text-xs text-right text-gray-500 dark:text-gray-400",
                        "Pull rate: {rate_label}"
                    }
                    if !rec.alt_sources.is_empty() {
                        div { class: "text-xs text-right text-gray-400 dark:text-gray-500 break-words",
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
}

// ---------------------------------------------------------------------------
// Trades tab
// ---------------------------------------------------------------------------

/// Compact card half-panel for the two-column trade layout.
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
                p { class: "text-sm font-semibold text-gray-900 dark:text-gray-100 line-clamp-2",
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
            let cv_b_id = CardVersionId(cv_b_id);
            let cv_a_id = CardVersionId(cv_a_id);
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
                            disabled:cursor-not-allowed \
                            shadow-md shadow-blue-500/30 dark:shadow-blue-900/70 active:shadow-sm active:translate-y-px",
                    disabled,
                    onclick: on_transfer,
                    "Transfer"
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
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
    let rate_label = pull_rate_label(rec.max_rate);
    rsx! {
        div {
            class: "flex flex-col p-4 border-b border-gray-100 dark:border-gray-700 \
                    last:border-0 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700/50",
            onclick: move |_| {
                back_origin.set(CardDetailOrigin::Trade);
                drop(
                    nav
                        .push(Route::CardDetailPage {
                            card_id: cv_id,
                        }),
                );
            },
            // Mobile header (hidden sm+)
            div { class: "sm:hidden flex items-start justify-between gap-2 mb-3",
                span { class: "shrink-0 w-8 h-8 flex items-center justify-center rounded-full text-xs font-bold bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300",
                    "#{rank}"
                }
                div { class: "flex flex-col items-end gap-1",
                    div { class: "text-xs text-right",
                        span { class: "text-gray-500 dark:text-gray-400", "{dest_name}: " }
                        span { class: "font-medium text-gray-800 dark:text-gray-200",
                            "{rec.dest_count} owned"
                        }
                        span { class: "text-gray-500 dark:text-gray-400", " ({rec.excess} excess)" }
                    }
                    div { class: "text-xs text-right text-gray-500 dark:text-gray-400",
                        "Pull rate: {rate_label}"
                    }
                }
            }
            div { class: "flex items-start gap-3",
                span { class: "hidden sm:flex shrink-0 w-8 h-8 items-center justify-center rounded-full text-xs font-bold bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300",
                    "#{rank}"
                }
                div { class: "flex-1 min-w-0",
                    CardPanel { cv_id }
                    if rec.is_unobtainable {
                        span { class: "inline-flex items-center mt-1.5 px-1.5 py-0.5 rounded text-xs font-medium bg-orange-100 dark:bg-orange-900/40 text-orange-800 dark:text-orange-200",
                            "Retired set — cannot be re-obtained from packs"
                        }
                    }
                }
                div { class: "hidden sm:flex flex-col items-end gap-1.5 shrink-0",
                    div { class: "text-xs text-right",
                        span { class: "text-gray-500 dark:text-gray-400", "{dest_name}: " }
                        span { class: "font-medium text-gray-800 dark:text-gray-200",
                            "{rec.dest_count} owned"
                        }
                        span { class: "text-gray-500 dark:text-gray-400", " ({rec.excess} excess)" }
                    }
                    div { class: "text-xs text-right text-gray-500 dark:text-gray-400",
                        "Pull rate: {rate_label}"
                    }
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

    let inactive_names: Vec<String> = {
        let guard = store.read();
        let Some(ref s) = *guard else {
            return rsx! {};
        };
        let active_set: HashSet<&str> = s
            .active_profile_names()
            .iter()
            .map(|n| n.as_str())
            .collect();
        s.profiles()
            .iter()
            .filter(|p| !active_set.contains(p.name.as_str()))
            .map(|p| p.name.clone())
            .collect()
    };

    let sel = selected.read();
    let count = sel.iter().filter(|n| inactive_names.contains(n)).count();
    drop(sel);

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
                if *open.read() {
                    ChevronUp { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                } else {
                    ChevronDown { class: "w-4 h-4 text-gray-500 dark:text-gray-400" }
                }
            }

            if *open.read() {
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
                    for name in inactive_names {
                        SourceProfileItem {
                            key: "{name}",
                            name: name.clone(),
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
    let effective_inactive: Vec<String> = {
        let sel = source_profiles.read();
        if sel.is_empty() {
            inactive_names.clone()
        } else {
            inactive_names
                .iter()
                .filter(|n| sel.contains(n))
                .cloned()
                .collect()
        }
    };

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
            &effective_inactive,
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
            &effective_inactive,
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

    let card_cls = "bg-white dark:bg-gray-800 rounded-lg border border-gray-200/80 dark:border-gray-700/80 shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] dark:ring-1 dark:ring-white/[0.06]";
    let show_more_cls = "w-full px-4 py-3 text-center text-sm text-blue-600 dark:text-blue-400 hover:bg-gray-50 dark:hover:bg-gray-700/50 border-t border-gray-100 dark:border-gray-700";

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

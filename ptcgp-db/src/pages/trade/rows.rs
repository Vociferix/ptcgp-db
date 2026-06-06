//! Row components for the Trade page: share, trade, and candidate recommendation rows.

use dioxus::prelude::*;
use ptcgp_db_core::save_data::CardVersionId;
use ptcgp_db_core::{CandidateRec, ProfileStore, ShareRec, TradeRec};
use ptcgp_db_data::{CardVersion, Prob};

use crate::app::{AppStorage, CardDetailOrigin, CompletedTransfer, schedule_save};
use crate::routes::Route;

// ---------------------------------------------------------------------------
// Card display helpers
// ---------------------------------------------------------------------------

/// Card image + name + set code + set/rarity icons.
#[component]
pub(super) fn CardPanel(cv_id: usize) -> Element {
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

pub(super) fn pull_rate_label(rate: Prob) -> String {
    if rate == Prob::ZERO {
        "—".to_string()
    } else {
        format!("{:.3}%", rate.as_f64() * 100.0)
    }
}

/// Compact card half-panel for the two-column trade layout.
#[component]
pub(super) fn TradeCardHalf(
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

// ---------------------------------------------------------------------------
// Shares tab
// ---------------------------------------------------------------------------

#[component]
pub(super) fn ShareRow(
    rank: usize,
    rec: ShareRec,
    dest_name: String,
    disabled: bool,
    history: Signal<Vec<CompletedTransfer>>,
    next_id: Signal<u64>,
) -> Element {
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
            let cv_key = CardVersionId(cv_id);
            let src_c = st.owned_count(&source_name, cv_key);
            let _ = st.set_owned_count(&source_name, cv_key, src_c.saturating_sub(1));
            let dst_c = st.owned_count(&dest_for_xfer, cv_key);
            let _ = st.set_owned_count(&dest_for_xfer, cv_key, dst_c + 1);
        }
        drop(s);
        schedule_save();
        let id = {
            let mut guard = next_id.write();
            let id = *guard;
            *guard = id + 1;
            id
        };
        history.write().push(CompletedTransfer::Share {
            id,
            cv_id,
            source_name: source_name.clone(),
            dest_name: dest_for_xfer.clone(),
        });
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

#[component]
pub(super) fn TradeRow(
    rank: usize,
    rec: TradeRec,
    dest_name: String,
    disabled: bool,
    history: Signal<Vec<CompletedTransfer>>,
    next_id: Signal<u64>,
) -> Element {
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
            let cv_b_key = CardVersionId(cv_b_id);
            let cv_a_key = CardVersionId(cv_a_id);
            let b_src = st.owned_count(&source, cv_b_key);
            let _ = st.set_owned_count(&source, cv_b_key, b_src.saturating_sub(1));
            let b_dst = st.owned_count(&dest_for_xfer, cv_b_key);
            let _ = st.set_owned_count(&dest_for_xfer, cv_b_key, b_dst + 1);
            let a_dst = st.owned_count(&dest_for_xfer, cv_a_key);
            let _ = st.set_owned_count(&dest_for_xfer, cv_a_key, a_dst.saturating_sub(1));
            let a_src = st.owned_count(&source, cv_a_key);
            let _ = st.set_owned_count(&source, cv_a_key, a_src + 1);
        }
        drop(s);
        schedule_save();
        let id = {
            let mut guard = next_id.write();
            let id = *guard;
            *guard = id + 1;
            id
        };
        history.write().push(CompletedTransfer::Trade {
            id,
            cv_b_id,
            cv_a_id,
            source_name: source.clone(),
            dest_name: dest_for_xfer.clone(),
        });
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
pub(super) fn CandidateRow(rank: usize, rec: CandidateRec, dest_name: String) -> Element {
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

//! Completed-transfer history section components for the Trade page.
//!
//! Each tab (Shares, Trades) shows a collapsible "Completed (N)" section above the active list.
//! Every entry can be undone (counts reversed) or dismissed (removed without reversal).

use dioxus::prelude::*;
use ptcgp_db_core::ProfileStore;
use ptcgp_db_core::save_data::CardVersionId;

use crate::app::{AppStorage, CompletedTransfer, schedule_save};
use crate::components::icons::{Check, ChevronDown, ChevronUp, XMark};

use super::CARD_CLS;
use super::rows::CardPanel;

// ---------------------------------------------------------------------------
// Shared styling constants
// ---------------------------------------------------------------------------

const UNDO_BTN_CLS: &str = "px-3 py-1.5 text-xs font-medium rounded-md \
    border border-blue-600 text-blue-600 \
    hover:bg-blue-50 dark:border-blue-400 dark:text-blue-400 \
    dark:hover:bg-blue-950/30 shadow-sm active:shadow-none active:translate-y-px";

const DISMISS_BTN_CLS: &str = "p-1.5 rounded text-gray-400 hover:text-gray-600 dark:text-gray-500 \
    dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700";

const COMPLETED_ROW_CLS: &str = "p-4 border-b border-gray-100 dark:border-gray-700 \
    last:border-0 bg-green-50/60 dark:bg-green-950/20";

const CHECK_BADGE_CLS: &str = "shrink-0 w-8 h-8 flex items-center justify-center \
    rounded-full bg-green-100 dark:bg-green-900/40";

// ---------------------------------------------------------------------------
// Section header
// ---------------------------------------------------------------------------

fn section_header(count: usize, mut expanded: Signal<bool>) -> Element {
    let exp = *expanded.read();
    rsx! {
        button {
            r#type: "button",
            class: "w-full flex items-center justify-between px-4 py-2.5 rounded-lg \
                    bg-green-50 dark:bg-green-950/30 border border-green-200/60 \
                    dark:border-green-800/40 text-sm font-medium \
                    text-green-800 dark:text-green-300 \
                    hover:bg-green-100 dark:hover:bg-green-900/40",
            onclick: move |_| expanded.toggle(),
            span { "Completed ({count})" }
            if exp {
                ChevronUp { class: "w-4 h-4 text-green-600 dark:text-green-400" }
            } else {
                ChevronDown { class: "w-4 h-4 text-green-600 dark:text-green-400" }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Completed share section
// ---------------------------------------------------------------------------

/// Collapsible section listing completed share transfers for the current session.
///
/// Renders nothing when no share transfers exist in `history`.
#[component]
pub(super) fn CompletedShareSection(
    history: Signal<Vec<CompletedTransfer>>,
    dest_name: String,
) -> Element {
    let expanded = use_signal(|| true);

    let h = history.read();
    let shares: Vec<(u64, usize, String)> = h
        .iter()
        .rev()
        .filter_map(|t| {
            if let CompletedTransfer::Share {
                id,
                cv_id,
                source_name,
                ..
            } = t
            {
                Some((*id, *cv_id, source_name.clone()))
            } else {
                None
            }
        })
        .collect();
    drop(h);

    if shares.is_empty() {
        return rsx! {};
    }

    let count = shares.len();

    rsx! {
        div { class: "mb-3",
            {section_header(count, expanded)}
            if *expanded.read() {
                div { class: "{CARD_CLS} mt-1",
                    for (id, cv_id, source_name) in shares {
                        CompletedShareRow {
                            key: "{id}",
                            transfer_id: id,
                            cv_id,
                            source_name,
                            dest_name: dest_name.clone(),
                            history,
                        }
                    }
                }
            }
        }
    }
}

/// One row in the completed share history. Shows the card panel, source/dest, and undo/dismiss.
#[component]
fn CompletedShareRow(
    transfer_id: u64,
    cv_id: usize,
    source_name: String,
    dest_name: String,
    mut history: Signal<Vec<CompletedTransfer>>,
) -> Element {
    let mut store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let source_for_undo = source_name.clone();
    let dest_for_undo = dest_name.clone();

    let on_undo = use_callback(move |e: Event<MouseData>| {
        e.stop_propagation();
        let mut s = store.write();
        if let Some(st) = s.as_mut() {
            let cv_key = CardVersionId(cv_id);
            let src_c = st.owned_count(&source_for_undo, cv_key);
            let _ = st.set_owned_count(&source_for_undo, cv_key, src_c + 1);
            let dst_c = st.owned_count(&dest_for_undo, cv_key);
            let _ = st.set_owned_count(&dest_for_undo, cv_key, dst_c.saturating_sub(1));
        }
        drop(s);
        schedule_save();
        history.write().retain(|t| t.id() != transfer_id);
    });

    let on_dismiss = use_callback(move |e: Event<MouseData>| {
        e.stop_propagation();
        history.write().retain(|t| t.id() != transfer_id);
    });

    rsx! {
        div { class: "{COMPLETED_ROW_CLS}",
            // Mobile header (hidden sm+)
            div { class: "sm:hidden flex items-center gap-2 mb-3",
                span { class: "{CHECK_BADGE_CLS}",
                    Check { class: "w-4 h-4 text-green-600 dark:text-green-400" }
                }
                div { class: "flex-1 min-w-0 text-xs",
                    span { class: "font-medium text-gray-800 dark:text-gray-200", "{source_name}" }
                    span { class: "text-gray-400 dark:text-gray-500", " → " }
                    span { class: "font-medium text-gray-800 dark:text-gray-200", "{dest_name}" }
                }
                div { class: "flex items-center gap-1.5 shrink-0",
                    button {
                        r#type: "button",
                        class: "{UNDO_BTN_CLS}",
                        onclick: move |e| on_undo.call(e),
                        "Undo"
                    }
                    button {
                        r#type: "button",
                        class: "{DISMISS_BTN_CLS}",
                        onclick: move |e| on_dismiss.call(e),
                        XMark { class: "w-4 h-4" }
                    }
                }
            }
            // Body: check badge + card panel + desktop sidebar
            div { class: "flex items-start gap-3",
                span { class: "hidden sm:flex {CHECK_BADGE_CLS}",
                    Check { class: "w-4 h-4 text-green-600 dark:text-green-400" }
                }
                div { class: "flex-1 min-w-0",
                    CardPanel { cv_id }
                }
                div { class: "hidden sm:flex flex-col items-end gap-1.5 shrink-0 min-w-[11rem]",
                    div { class: "flex items-center gap-1.5",
                        button {
                            r#type: "button",
                            class: "{UNDO_BTN_CLS}",
                            onclick: move |e| on_undo.call(e),
                            "Undo"
                        }
                        button {
                            r#type: "button",
                            class: "{DISMISS_BTN_CLS}",
                            onclick: move |e| on_dismiss.call(e),
                            XMark { class: "w-4 h-4" }
                        }
                    }
                    div { class: "text-xs text-right",
                        span { class: "text-gray-500 dark:text-gray-400", "Source: " }
                        span { class: "font-medium text-gray-800 dark:text-gray-200",
                            "{source_name}"
                        }
                    }
                    div { class: "text-xs text-right",
                        span { class: "text-gray-500 dark:text-gray-400", "Dest: " }
                        span { class: "font-medium text-gray-800 dark:text-gray-200",
                            "{dest_name}"
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Completed trade section
// ---------------------------------------------------------------------------

/// Collapsible section listing completed trade transfers for the current session.
///
/// Renders nothing when no trade transfers exist in `history`.
#[component]
pub(super) fn CompletedTradeSection(
    history: Signal<Vec<CompletedTransfer>>,
    dest_name: String,
) -> Element {
    let expanded = use_signal(|| true);

    let h = history.read();
    let trades: Vec<(u64, usize, usize, String)> = h
        .iter()
        .rev()
        .filter_map(|t| {
            if let CompletedTransfer::Trade {
                id,
                cv_b_id,
                cv_a_id,
                source_name,
                ..
            } = t
            {
                Some((*id, *cv_b_id, *cv_a_id, source_name.clone()))
            } else {
                None
            }
        })
        .collect();
    drop(h);

    if trades.is_empty() {
        return rsx! {};
    }

    let count = trades.len();

    rsx! {
        div { class: "mb-3",
            {section_header(count, expanded)}
            if *expanded.read() {
                div { class: "{CARD_CLS} mt-1",
                    for (id, cv_b_id, cv_a_id, source_name) in trades {
                        CompletedTradeRow {
                            key: "{id}",
                            transfer_id: id,
                            cv_b_id,
                            cv_a_id,
                            source_name,
                            dest_name: dest_name.clone(),
                            history,
                        }
                    }
                }
            }
        }
    }
}

/// One row in the completed trade history. Shows both card panels and undo/dismiss.
#[component]
fn CompletedTradeRow(
    transfer_id: u64,
    cv_b_id: usize,
    cv_a_id: usize,
    source_name: String,
    dest_name: String,
    mut history: Signal<Vec<CompletedTransfer>>,
) -> Element {
    let mut store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let source_for_undo = source_name.clone();
    let dest_for_undo = dest_name.clone();

    let on_undo = use_callback(move |e: Event<MouseData>| {
        e.stop_propagation();
        let mut s = store.write();
        if let Some(st) = s.as_mut() {
            let cv_b_key = CardVersionId(cv_b_id);
            let cv_a_key = CardVersionId(cv_a_id);
            // Reverse: source takes back card_b, dest returns card_b
            let b_src = st.owned_count(&source_for_undo, cv_b_key);
            let _ = st.set_owned_count(&source_for_undo, cv_b_key, b_src + 1);
            let b_dst = st.owned_count(&dest_for_undo, cv_b_key);
            let _ = st.set_owned_count(&dest_for_undo, cv_b_key, b_dst.saturating_sub(1));
            // Reverse: dest takes back card_a, source returns card_a
            let a_dst = st.owned_count(&dest_for_undo, cv_a_key);
            let _ = st.set_owned_count(&dest_for_undo, cv_a_key, a_dst + 1);
            let a_src = st.owned_count(&source_for_undo, cv_a_key);
            let _ = st.set_owned_count(&source_for_undo, cv_a_key, a_src.saturating_sub(1));
        }
        drop(s);
        schedule_save();
        history.write().retain(|t| t.id() != transfer_id);
    });

    let on_dismiss = use_callback(move |e: Event<MouseData>| {
        e.stop_propagation();
        history.write().retain(|t| t.id() != transfer_id);
    });

    rsx! {
        div { class: "{COMPLETED_ROW_CLS}",
            // Header row: check badge + source→dest + undo/dismiss
            div { class: "flex items-center gap-2 mb-3",
                span { class: "{CHECK_BADGE_CLS}",
                    Check { class: "w-4 h-4 text-green-600 dark:text-green-400" }
                }
                div { class: "flex-1 min-w-0 flex items-center gap-1.5 flex-wrap text-xs",
                    span { class: "font-medium text-gray-800 dark:text-gray-200", "{source_name}" }
                    span { class: "text-gray-400 dark:text-gray-500", "→" }
                    span { class: "font-medium text-gray-800 dark:text-gray-200", "{dest_name}" }
                }
                div { class: "flex items-center gap-1.5 shrink-0",
                    button {
                        r#type: "button",
                        class: "{UNDO_BTN_CLS}",
                        onclick: move |e| on_undo.call(e),
                        "Undo"
                    }
                    button {
                        r#type: "button",
                        class: "{DISMISS_BTN_CLS}",
                        onclick: move |e| on_dismiss.call(e),
                        XMark { class: "w-4 h-4" }
                    }
                }
            }
            // Two-panel card display (no counts — they've changed post-transfer)
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                div { class: "bg-green-50 dark:bg-green-950/20 rounded-md p-2",
                    p { class: "text-xs font-semibold text-green-700 dark:text-green-400 mb-2",
                        "You received"
                    }
                    CardPanel { cv_id: cv_b_id }
                }
                div { class: "bg-red-50 dark:bg-red-950/20 rounded-md p-2",
                    p { class: "text-xs font-semibold text-red-700 dark:text-red-400 mb-2",
                        "You gave"
                    }
                    CardPanel { cv_id: cv_a_id }
                }
            }
        }
    }
}

//! Card detail view: full-screen narrow-viewport page and wide-viewport panel.
//!
//! `CardDetailBody` is the shared renderer; `DetailPanel` wraps it for the catalog sidebar
//! and `CardDetailPage` wraps it for the routed full-screen view.

use dioxus::prelude::*;
use ptcgp_db_core::save_data::CardVersionId;
use ptcgp_db_core::{AppSettings, CARD_PULL_RATES, ProfileStore};
use ptcgp_db_data::CardVersion;

use crate::app::{AppStorage, CardDetailOrigin, set_card_count};
use crate::components::count_spinner::CountSpinner;
use crate::components::effect_text::EffectText;
use crate::components::icons::ArrowLeft;
use crate::routes::Route;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn focus_on_mount(data: Event<MountedData>) {
    let _ = spawn(async move {
        let _ = data.set_focus(true).await;
    });
}

fn ordinal_suffix(n: usize) -> &'static str {
    match n {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

// ---------------------------------------------------------------------------
// Pull rate hierarchy: PullRateSection → PackPullBlock
// ---------------------------------------------------------------------------

#[component]
fn PullRateSection(cv_id: usize) -> Element {
    let pd = &CARD_PULL_RATES[cv_id];
    rsx! {
        div { class: "flex flex-col gap-3",
            p { class: "text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide",
                "Pull Rates"
            }
            for pr in pd.pack_pull_rates.iter() {
                PackPullBlock {
                    pack_id: pr.pack.id(),
                    overall_pct: pr.percent,
                    cv_id,
                }
            }
        }
    }
}

struct VariantPullRow {
    name: ptcgp_db_data::str_table::StrEntry,
    variant_pct: f64,
    card_pct: f64,
    slots: Vec<(usize, f64)>,
}

#[component]
fn PackPullBlock(pack_id: usize, overall_pct: f64, cv_id: usize) -> Element {
    let Some(pack) = ptcgp_db_data::Pack::from_id(pack_id) else {
        return rsx! {};
    };
    let logo = pack.logo();
    let title = pack.title();

    let variant_rows: Vec<VariantPullRow> = pack
        .variants()
        .iter()
        .filter_map(|variant| {
            let slots: Vec<(usize, f64)> = variant
                .slots()
                .iter()
                .filter_map(|slot| {
                    slot.card_versions()
                        .iter()
                        .find(|cvpr| cvpr.card_version().id() == cv_id)
                        .map(|cvpr| (slot.pull_number(), cvpr.pull_rate().as_f64() * 100.0))
                        .filter(|(_, r)| *r > 0.0)
                })
                .collect();
            if slots.is_empty() {
                return None;
            }
            let not_prob = slots
                .iter()
                .fold(1.0f64, |acc, (_, r)| acc * (1.0 - r / 100.0));
            Some(VariantPullRow {
                name: variant.name(),
                variant_pct: variant.pull_rate().as_f64() * 100.0,
                card_pct: (1.0 - not_prob) * 100.0,
                slots,
            })
        })
        .collect();

    rsx! {
        div { class: "flex flex-col gap-1.5",
            div { class: "flex items-center gap-3 rounded px-1 hover:bg-gray-100 dark:hover:bg-gray-700/50",
                img {
                    src: "{logo}",
                    alt: "",
                    class: "h-12 w-24 object-contain flex-shrink-0",
                }
                span { class: "flex-1 text-sm text-gray-700 dark:text-gray-300", "{title}" }
                span { class: "text-sm tabular-nums font-medium text-gray-900 dark:text-gray-100",
                    "{overall_pct:.3}%"
                }
            }
            for row in variant_rows {
                div { class: "ml-6 flex flex-col gap-0.5",
                    div { class: "flex items-center gap-2 rounded px-1 hover:bg-gray-100 dark:hover:bg-gray-700/50",
                        span { class: "flex-1 text-xs text-gray-600 dark:text-gray-400",
                            "{row.name} · {row.variant_pct:.3}% of packs"
                        }
                        span { class: "text-xs tabular-nums text-gray-700 dark:text-gray-300",
                            "{row.card_pct:.3}%"
                        }
                    }
                    for (pull_num, slot_pct) in row.slots {
                        div { class: "ml-4 flex items-center gap-2 rounded px-1 hover:bg-gray-100 dark:hover:bg-gray-700/50",
                            span { class: "flex-1 text-xs text-gray-400 dark:text-gray-500",
                                "{pull_num + 1}{ordinal_suffix(pull_num + 1)} Card"
                            }
                            span { class: "text-xs tabular-nums text-gray-500 dark:text-gray-400",
                                "{slot_pct:.3}%"
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AttackRow
// ---------------------------------------------------------------------------

#[component]
fn AttackRow(attack_id: usize) -> Element {
    let attack = &ptcgp_db_data::Attack::ALL[attack_id];
    let cost = attack.cost();
    let name = attack.name();
    let damage = attack.damage();
    let effect = attack.effect();
    rsx! {
        div { class: "flex flex-col gap-1 p-2 rounded bg-gray-50 dark:bg-gray-800",
            div { class: "flex items-center gap-2",
                div { class: "flex items-center gap-1 shrink-0",
                    if cost.is_empty() {
                        img {
                            src: "{ptcgp_db_data::Element::NO_COST}",
                            alt: "",
                            class: "h-5 w-5 object-contain",
                        }
                    } else {
                        for elem in cost.iter() {
                            img {
                                src: "{elem.icon()}",
                                alt: "",
                                class: "h-5 w-5 object-contain",
                            }
                        }
                    }
                }
                span { class: "flex-1 text-sm font-medium text-gray-900 dark:text-gray-100",
                    "{name}"
                }
                if attack.base_damage() > 0 || attack.damage_suffix().is_some() {
                    span { class: "text-sm font-bold tabular-nums text-gray-900 dark:text-gray-100",
                        "{damage}"
                    }
                }
            }
            if let Some(eff) = effect {
                p { class: "text-xs text-gray-600 dark:text-gray-400",
                    EffectText { text: eff.to_string() }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Version navigation chips and cards
// ---------------------------------------------------------------------------

#[component]
fn VersionChip(cv_id: usize, current_id: usize, on_click: EventHandler<usize>) -> Element {
    let cv = &CardVersion::ALL[cv_id];
    let set_code = cv.set().code();
    let number = cv.number().get();
    let rarity = cv.rarity().name();
    let is_current = cv_id == current_id;
    let cls = if is_current {
        "text-xs px-2 py-0.5 rounded-full border font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 border-blue-300 dark:border-blue-700 cursor-default"
    } else {
        "text-xs px-2 py-0.5 rounded-full border bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 border-gray-200 dark:border-gray-700 hover:border-blue-400 dark:hover:border-blue-500 cursor-pointer"
    };
    rsx! {
        button {
            r#type: "button",
            class: "{cls}",
            title: "{rarity}",
            onclick: move |_| on_click.call(cv_id),
            "{set_code} {number:03}"
        }
    }
}

#[component]
fn VersionCard(cv_id: usize, current_id: usize, on_click: EventHandler<usize>) -> Element {
    let cv = &CardVersion::ALL[cv_id];
    let card_image = cv.image();
    let set_code = cv.set().code();
    let number = cv.number().get();
    let is_current = cv_id == current_id;
    let ring_cls = if is_current {
        "ring-2 ring-blue-500 dark:ring-blue-400"
    } else {
        "ring-1 ring-gray-200 dark:ring-gray-700 hover:ring-blue-400 dark:hover:ring-blue-500"
    };
    rsx! {
        button {
            r#type: "button",
            class: "flex flex-col items-center gap-1 cursor-pointer",
            onclick: move |_| on_click.call(cv_id),
            img {
                src: "{card_image}",
                alt: "",
                class: "w-14 h-20 object-cover rounded {ring_cls}",
            }
            span { class: "text-xs text-gray-500 dark:text-gray-400 tabular-nums",
                "{set_code} {number:03}"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CardDetailBody — shared by DetailPanel and CardDetailPage
// ---------------------------------------------------------------------------

#[component]
pub(super) fn CardDetailBody(cv_id: usize, on_navigate: EventHandler<usize>) -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let settings = use_context::<Signal<AppSettings>>();

    let cv = &CardVersion::ALL[cv_id];
    let merge = settings.read().merge_duplicate_printings();
    let (multi_active, value, stored_count) = {
        let s = store.read();
        let multi_active = s
            .as_ref()
            .is_some_and(|s| s.active_profile_names().len() > 1);
        let s = s.as_ref();
        let agg = s.map_or(0, |s| s.aggregate_count(CardVersionId(cv_id)));
        let merged = if merge {
            cv.duplicates().iter().fold(agg, |acc, d| {
                acc.saturating_add(s.map_or(0, |s| s.aggregate_count(CardVersionId(d.id()))))
            })
        } else {
            agg
        };
        let stored = if multi_active {
            agg
        } else {
            s.and_then(|s| {
                s.active_profile_names()
                    .first()
                    .map(|n| s.owned_count(n, CardVersionId(cv_id)))
            })
            .unwrap_or(0)
        };
        (multi_active, merged, stored)
    };

    let set_code = cv.set().code();
    let number = cv.number().get();
    let name = cv.card().name();
    let illustrator = cv.illustrator();
    let rarity_icon = cv.rarity().class().icon();
    let rarity_name = cv.rarity().name();
    let card_image = cv.image();
    let source_name = cv.source().name();
    let source_desc = cv.source().description();
    let is_pack_source = source_name.as_str() == "Pack";
    let is_pokemon = cv.card().is_pokemon();
    let is_trainer = cv.card().is_trainer();
    let pd = &CARD_PULL_RATES[cv_id];
    let pkmn = cv.card().pokemon();
    let trainer = cv.card().trainer();
    let duplicates = cv.duplicates();
    let all_versions = cv.card().versions();
    let mut lightbox_open: Signal<bool> = use_signal(|| false);

    rsx! {
        div { class: "flex flex-col h-full overflow-y-auto",
            div { class: "flex justify-center p-4 bg-gray-50 dark:bg-gray-800/50 shrink-0",
                img {
                    src: "{card_image}",
                    alt: "{name}",
                    class: "h-64 w-auto object-contain rounded shadow-md cursor-zoom-in",
                    onclick: move |_| lightbox_open.set(true),
                }
            }

            div { class: "flex flex-col gap-4 p-4",
                div { class: "flex items-start justify-between gap-2",
                    div {
                        p { class: "text-lg font-bold text-gray-900 dark:text-gray-100 leading-tight",
                            "{name}"
                        }
                        p { class: "text-xs text-gray-400 dark:text-gray-500 tabular-nums",
                            "{set_code} {number:03}"
                        }
                    }
                    div { class: "flex flex-col items-end gap-1 shrink-0",
                        if is_pokemon {
                            span { class: "text-xs px-2 py-0.5 rounded-full bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300",
                                "Pokémon"
                            }
                        } else if is_trainer {
                            span { class: "text-xs px-2 py-0.5 rounded-full bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300",
                                "Trainer"
                            }
                        }
                        span { class: "text-xs italic text-gray-400 dark:text-gray-500",
                            "Illus. {illustrator}"
                        }
                    }
                }

                div { class: "flex items-center gap-2",
                    span { class: "text-sm text-gray-600 dark:text-gray-400", "Owned" }
                    CountSpinner {
                        value,
                        stored_count,
                        disabled: multi_active,
                        on_change: move |n| set_card_count(cv_id, n, store),
                    }
                    div { class: "flex items-center gap-2 ml-auto",
                        img {
                            src: "{rarity_icon}",
                            alt: "",
                            class: "h-6 w-auto object-contain",
                        }
                        span { class: "text-xs text-gray-500 dark:text-gray-400", "{rarity_name}" }
                    }
                }

                if !is_pack_source {
                    div { class: "text-xs text-amber-700 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 rounded p-2",
                        "{source_desc}"
                    }
                }

                if let Some(p) = pkmn {
                    div { class: "flex flex-col gap-3",
                        if let Some(ft) = p.flavor_text() {
                            p { class: "text-xs italic text-gray-400 dark:text-gray-500 leading-relaxed",
                                "{ft}"
                            }
                        }

                        div { class: "grid grid-cols-3 gap-2",
                            div { class: "flex flex-col items-center p-2 rounded bg-gray-50 dark:bg-gray-800",
                                span { class: "text-xs text-gray-400 dark:text-gray-500",
                                    "Pokédex"
                                }
                                span { class: "flex items-baseline gap-1",
                                    span { class: "text-sm font-bold tabular-nums text-gray-900 dark:text-gray-100",
                                        "#{p.base_pokemon().natdex_number()}"
                                    }
                                    span { class: "text-xs text-gray-600 dark:text-gray-400",
                                        "{p.base_pokemon().name()}"
                                    }
                                }
                            }
                            div { class: "flex flex-col items-center p-2 rounded bg-gray-50 dark:bg-gray-800",
                                span { class: "text-xs text-gray-400 dark:text-gray-500",
                                    "HP"
                                }
                                span { class: "text-sm font-bold tabular-nums text-gray-900 dark:text-gray-100",
                                    "{p.hp()}"
                                }
                            }
                            div { class: "flex flex-col items-center p-2 rounded bg-gray-50 dark:bg-gray-800",
                                span { class: "text-xs text-gray-400 dark:text-gray-500",
                                    "Stage"
                                }
                                span { class: "text-sm font-bold text-gray-900 dark:text-gray-100",
                                    "{p.stage().name()}"
                                }
                            }
                        }

                        div { class: "flex items-center gap-2",
                            img {
                                src: "{p.element().icon()}",
                                alt: "",
                                class: "h-5 w-5 object-contain",
                            }
                            span { class: "text-sm text-gray-700 dark:text-gray-300",
                                "{p.element().name()}"
                            }
                        }

                        div { class: "flex items-center gap-2",
                            span { class: "text-xs text-gray-400 dark:text-gray-500 w-20 shrink-0",
                                "Retreat"
                            }
                            if p.retreat_cost() == 0 {
                                span { class: "text-xs text-gray-500 dark:text-gray-400",
                                    "Free"
                                }
                            } else {
                                div { class: "flex items-center gap-1",
                                    if let Some(colorless) = ptcgp_db_data::Element::ALL
                                        .iter()
                                        .find(|e| e.code() == Some('C'))
                                    {
                                        for _ in 0..p.retreat_cost() {
                                            img {
                                                src: "{colorless.icon()}",
                                                alt: "",
                                                class: "h-5 w-5 object-contain",
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(w) = p.weakness() {
                            div { class: "flex items-center gap-2",
                                span { class: "text-xs text-gray-400 dark:text-gray-500 w-20 shrink-0",
                                    "Weakness"
                                }
                                div { class: "flex items-center gap-1",
                                    img {
                                        src: "{w.icon()}",
                                        alt: "",
                                        class: "h-5 w-5 object-contain",
                                    }
                                    span { class: "text-sm text-gray-700 dark:text-gray-300",
                                        "{w.name()}"
                                    }
                                }
                            }
                        }

                        if p.is_ex() || p.is_mega() || cv.is_foil() || !cv.is_tradable() {
                            div { class: "flex flex-wrap gap-2",
                                if p.is_ex() {
                                    span { class: "text-xs px-2 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300",
                                        "Pokémon ex"
                                    }
                                }
                                if p.is_mega() {
                                    span { class: "text-xs px-2 py-0.5 rounded-full bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300",
                                        "Mega Pokémon ex"
                                    }
                                }
                                if cv.is_foil() {
                                    span { class: "text-xs px-2 py-0.5 rounded-full bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300",
                                        "Foil"
                                    }
                                }
                                if !cv.is_tradable() {
                                    span { class: "text-xs px-2 py-0.5 rounded-full bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400",
                                        "Not Tradable"
                                    }
                                }
                            }
                        }

                        if let Some(evo) = p.evolves_from() {
                            p { class: "text-xs text-gray-500 dark:text-gray-400",
                                "Evolves from "
                                span { class: "font-medium text-gray-700 dark:text-gray-300",
                                    "{evo}"
                                }
                            }
                        }

                        if !p.attacks().is_empty() {
                            div { class: "flex flex-col gap-2",
                                p { class: "text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide",
                                    "Attacks"
                                }
                                for attack in p.attacks().iter() {
                                    AttackRow { attack_id: attack.id() }
                                }
                            }
                        }

                        if let Some(ab) = p.ability() {
                            div { class: "flex flex-col gap-1 p-2 rounded bg-blue-50 dark:bg-blue-900/20",
                                p { class: "text-xs font-semibold text-blue-700 dark:text-blue-300",
                                    "Ability — {ab.name()}"
                                }
                                p { class: "text-xs text-gray-600 dark:text-gray-400",
                                    EffectText { text: ab.effect().to_string() }
                                }
                            }
                        }
                    }
                }

                if let Some(t) = trainer {
                    div { class: "flex flex-col gap-3",
                        div { class: "flex flex-wrap gap-2",
                            span { class: "text-xs px-2 py-0.5 rounded-full bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300",
                                "{t.kind().name()}"
                            }
                            if cv.is_foil() {
                                span { class: "text-xs px-2 py-0.5 rounded-full bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300",
                                    "Foil"
                                }
                            }
                            if !cv.is_tradable() {
                                span { class: "text-xs px-2 py-0.5 rounded-full bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400",
                                    "Not Tradable"
                                }
                            }
                        }
                        div { class: "p-2 rounded bg-gray-50 dark:bg-gray-800",
                            p { class: "text-xs text-gray-700 dark:text-gray-300",
                                EffectText { text: t.effect().to_string() }
                            }
                        }
                    }
                }

                if !duplicates.is_empty() {
                    div { class: "flex flex-col gap-2",
                        p { class: "text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide",
                            "Duplicate Printings"
                        }
                        div { class: "flex flex-wrap gap-1",
                            for d in duplicates.iter() {
                                VersionChip {
                                    cv_id: d.id(),
                                    current_id: cv_id,
                                    on_click: on_navigate,
                                }
                            }
                        }
                    }
                }

                if all_versions.len() > 1 {
                    div { class: "flex flex-col gap-2",
                        p { class: "text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide",
                            "All Versions"
                        }
                        div { class: "flex flex-wrap gap-3",
                            for v in all_versions.iter() {
                                VersionCard {
                                    cv_id: v.id(),
                                    current_id: cv_id,
                                    on_click: on_navigate,
                                }
                            }
                        }
                    }
                }

                if is_pack_source && !pd.pack_pull_rates.is_empty() {
                    PullRateSection { cv_id }
                }
            }
        }

        if *lightbox_open.read() {
            div {
                class: "fixed inset-0 z-50 flex items-center justify-center bg-black/80 cursor-zoom-out",
                tabindex: "-1",
                onmounted: focus_on_mount,
                onkeydown: move |evt| {
                    if evt.key() == Key::Escape {
                        lightbox_open.set(false);
                    }
                },
                onclick: move |_| lightbox_open.set(false),
                img {
                    src: "{card_image}",
                    alt: "{name}",
                    class: "max-w-[calc(100vw-2rem)] max-h-[calc(100vh-2rem)] object-contain rounded shadow-2xl",
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DetailPanel — wide-viewport sidebar beside the catalog list
// ---------------------------------------------------------------------------

/// Shows card details in the fixed sidebar shown at xl+ viewports.
///
/// Renders a placeholder prompt when no card is selected.
#[component]
pub(super) fn DetailPanel(cv_id: Signal<Option<usize>>) -> Element {
    let Some(id) = *cv_id.read() else {
        return rsx! {
            div { class: "flex flex-col items-center justify-center h-full text-sm text-gray-400 dark:text-gray-600 p-6 text-center",
                "Select a card to view details."
            }
        };
    };
    rsx! {
        CardDetailBody { cv_id: id, on_navigate: move |new_id| cv_id.set(Some(new_id)) }
    }
}

// ---------------------------------------------------------------------------
// CardDetailPage — full-screen routed page for narrow viewports
// ---------------------------------------------------------------------------

/// Full-screen card detail page, used for narrow viewports where the sidebar is hidden.
#[component]
pub fn CardDetailPage(card_id: usize) -> Element {
    let nav = use_navigator();
    let back_origin = use_context::<Signal<CardDetailOrigin>>();
    if card_id >= CardVersion::ALL.len() {
        return rsx! {
            div { class: "flex flex-col items-center justify-center h-full text-sm text-gray-400 dark:text-gray-600 p-6",
                "Card not found."
            }
        };
    }
    let (back_label, back_route) = match *back_origin.read() {
        CardDetailOrigin::Trade => ("Trade", Route::TradePage {}),
        CardDetailOrigin::Catalog => ("Catalog", Route::CatalogPage {}),
    };
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex items-center shrink-0 px-3 py-2 border-b border-gray-200 dark:border-gray-700",
                button {
                    r#type: "button",
                    class: "flex items-center gap-1 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100",
                    onclick: move |_| {
                        drop(nav.push(back_route.clone()));
                    },
                    ArrowLeft { class: "w-4 h-4".to_string() }
                    "{back_label}"
                }
            }
            div { class: "flex-1 min-h-0",
                CardDetailBody {
                    cv_id: card_id,
                    on_navigate: move |id| drop(
                        nav
                            .push(Route::CardDetailPage {
                                card_id: id,
                            }),
                    ),
                }
            }
        }
    }
}

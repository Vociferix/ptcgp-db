use dioxus::document;
use dioxus::prelude::*;
use ptcgp_db_core::save_data::FilterConfig;
use ptcgp_db_core::{AppSettings, CARD_PULL_RATES, ProfileStore, filter_card};
use ptcgp_db_data::{Card, CardVersion};

use crate::app::{AppStorage, CardDetailOrigin, set_card_count};
use crate::components::count_spinner::CountSpinner;
use crate::components::icons::{ChevronDown, ChevronUp};
use crate::components::{FilterMode, FilterToolbar};
use crate::routes::Route;

use super::card_details::DetailPanel;

// ---------------------------------------------------------------------------
// Sort state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Default)]
enum SortColumn {
    #[default]
    Default,
    Name,
    OwnedCount,
    Rarity,
    Element,
    PullRate,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum SortDir {
    #[default]
    Asc,
    Desc,
}

impl SortDir {
    fn toggle(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
struct SortConfig {
    column: SortColumn,
    dir: SortDir,
}

// ---------------------------------------------------------------------------
// Row visual style helpers
// ---------------------------------------------------------------------------

fn element_tint_class(cv: &CardVersion) -> &'static str {
    let Some(pkmn) = cv.card().pokemon() else {
        return "";
    };
    let name = pkmn.element().name();
    match name.as_str() {
        "Grass" => "bg-element-grass/10",
        "Fire" => "bg-element-fire/10",
        "Water" => "bg-element-water/10",
        "Lightning" => "bg-element-lightning/10",
        "Fighting" => "bg-element-fighting/10",
        "Psychic" => "bg-element-psychic/10",
        "Darkness" => "bg-element-darkness/10",
        "Metal" => "bg-element-metal/10",
        "Dragon" => "bg-element-dragon/10",
        "Colorless" => "bg-element-colorless/10",
        _ => "",
    }
}

fn element_color_hex(cv: &CardVersion) -> &'static str {
    let Some(pkmn) = cv.card().pokemon() else {
        return "#6b7280";
    };
    let name = pkmn.element().name();
    match name.as_str() {
        "Grass" => "#4ade80",
        "Fire" => "#ef4444",
        "Water" => "#38bdf8",
        "Lightning" => "#fde047",
        "Fighting" => "#c2762c",
        "Psychic" => "#a855f7",
        "Darkness" => "#0d6977",
        "Metal" => "#94a3b8",
        "Dragon" => "#ca8a04",
        "Colorless" => "#d1d5db",
        _ => "#6b7280",
    }
}

fn row_border_style(cv: &CardVersion) -> String {
    let foil = if cv.is_foil() {
        "border-image: linear-gradient(135deg, #ff0000, #ff7700, #ffff00, #00aa00, #0000ff, #8b00ff) 1; border-width: 1px; border-style: solid;"
    } else {
        ""
    };
    let group_name = cv.rarity().group().name();
    let rarity_border = match group_name.as_str() {
        "Star" => {
            let hex = element_color_hex(cv);
            format!("outline: 1px solid {hex}80; outline-offset: -1px;")
        }
        "Shiny" => "outline: 1px solid #c0c0c080; outline-offset: -1px;".to_string(),
        "Crown" => "outline: 2px solid #f59e0b; outline-offset: -2px;".to_string(),
        _ => String::new(),
    };
    format!("{foil}{rarity_border}")
}

fn premium_tint_class(cv: &CardVersion) -> &'static str {
    let src_name = cv.source().name();
    let src = src_name.as_str();
    if src == "Premium Mission" || src == "Gold Shop" {
        "bg-amber-100/30 dark:bg-amber-900/20"
    } else {
        ""
    }
}

// ---------------------------------------------------------------------------
// Sort helpers
// ---------------------------------------------------------------------------

/// Sorts `ids` using a comparison function, applying direction and a stable tiebreak on index.
///
/// `cmp(a, b)` should return the natural `Ordering` for the sort key (i.e. as if ascending).
/// `dir` flips the primary ordering; ties always break ascending by index for determinism.
fn sort_with_dir(
    ids: &mut [usize],
    dir: SortDir,
    cmp: impl Fn(usize, usize) -> std::cmp::Ordering,
) {
    ids.sort_by(|&a, &b| {
        let n = cmp(a, b);
        if dir == SortDir::Asc {
            n.then(a.cmp(&b))
        } else {
            n.reverse().then(a.cmp(&b))
        }
    });
}

// ---------------------------------------------------------------------------
// Virtual-list + scroll helpers
// ---------------------------------------------------------------------------

const ITEM_HEIGHT: f64 = 88.0;
const SCROLL_BUFFER: usize = 8;
const SCROLL_CONTAINER_ID: &str = "ptcgp-catalog-vlist";

fn handle_scroll(mut scroll_top: Signal<f64>, mut scroll_pending: Signal<bool>) {
    if *scroll_pending.read() {
        return;
    }
    scroll_pending.set(true);
    let script = format!("dioxus.send(document.getElementById('{SCROLL_CONTAINER_ID}').scrollTop)");
    let _ = spawn(async move {
        let mut e = document::eval(&script);
        if let Ok(v) = e.recv::<f64>().await {
            scroll_top.set(v);
        }
        scroll_pending.set(false);
    });
}

fn handle_container_mounted(data: Event<MountedData>, mut container_height: Signal<f64>) {
    let _ = spawn(async move {
        if let Ok(rect) = data.get_client_rect().await {
            container_height.set(rect.size.height);
        }
    });
}

fn handle_sort_click(col: SortColumn, mut sort_cfg: Signal<SortConfig>) {
    let mut sc = sort_cfg.write();
    if sc.column == col {
        if sc.dir == SortDir::Desc {
            *sc = SortConfig::default();
        } else {
            sc.dir = sc.dir.toggle();
        }
    } else {
        *sc = SortConfig {
            column: col,
            dir: SortDir::Asc,
        };
    }
}

// ---------------------------------------------------------------------------
// CatalogPage
// ---------------------------------------------------------------------------

#[component]
pub fn CatalogPage() -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let settings = use_context::<Signal<AppSettings>>();

    let config = use_context::<Signal<FilterConfig>>();
    let sort_cfg: Signal<SortConfig> = use_signal(SortConfig::default);
    let selected: Signal<Option<usize>> = use_signal(|| None);
    let mut scroll_top: Signal<f64> = use_signal(|| 0.0);
    let container_height: Signal<f64> = use_signal(|| 600.0);
    let scroll_pending: Signal<bool> = use_signal(|| false);

    let today = chrono::Utc::now().date_naive();

    let filtered_ids = use_memo(move || {
        let s = store.read();
        let Some(s) = s.as_ref() else {
            return Vec::new();
        };
        let cfg = config.read();
        let sett = settings.read();
        let sc = sort_cfg.read();

        let matched_name_ids: Option<Vec<usize>> = cfg
            .name_query
            .as_deref()
            .filter(|q| !q.trim().is_empty())
            .map(|q| Card::NAMES.search(q).map(|e| e.id()).collect());

        let mut ids: Vec<usize> = CardVersion::ALL
            .iter()
            .filter(|cv| {
                let owned_count = cfg.owned_count.map(|_| s.aggregate_count(cv.id()));
                filter_card(
                    cv,
                    &cfg,
                    &sett,
                    today,
                    matched_name_ids.as_deref(),
                    owned_count,
                )
            })
            .map(|cv| cv.id())
            .collect();

        match sc.column {
            SortColumn::Default => {
                if sc.dir == SortDir::Desc {
                    ids.reverse();
                }
            }
            SortColumn::Name => {
                sort_with_dir(&mut ids, sc.dir, |a, b| {
                    CardVersion::ALL[a]
                        .card()
                        .name()
                        .as_str()
                        .cmp(CardVersion::ALL[b].card().name().as_str())
                });
            }
            SortColumn::OwnedCount => {
                sort_with_dir(&mut ids, sc.dir, |a, b| {
                    s.aggregate_count(a).cmp(&s.aggregate_count(b))
                });
            }
            SortColumn::Rarity => {
                sort_with_dir(&mut ids, sc.dir, |a, b| {
                    CardVersion::ALL[a]
                        .rarity()
                        .class()
                        .id()
                        .cmp(&CardVersion::ALL[b].rarity().class().id())
                });
            }
            SortColumn::Element => {
                sort_with_dir(&mut ids, sc.dir, |a, b| {
                    let ea = CardVersion::ALL[a]
                        .card()
                        .pokemon()
                        .map(|p| p.element().id())
                        .unwrap_or(usize::MAX);
                    let eb = CardVersion::ALL[b]
                        .card()
                        .pokemon()
                        .map(|p| p.element().id())
                        .unwrap_or(usize::MAX);
                    ea.cmp(&eb)
                });
            }
            SortColumn::PullRate => {
                ids.sort_by(|&a, &b| {
                    let ra = CARD_PULL_RATES[a].max_pull_rate_pct;
                    let rb = CARD_PULL_RATES[b].max_pull_rate_pct;
                    // Zero-rate (no pack) cards always sort last regardless of direction.
                    match (ra == 0.0, rb == 0.0) {
                        (true, false) => std::cmp::Ordering::Greater,
                        (false, true) => std::cmp::Ordering::Less,
                        _ => {
                            let n = ra.total_cmp(&rb);
                            if sc.dir == SortDir::Asc {
                                n.then(a.cmp(&b))
                            } else {
                                n.reverse().then(a.cmp(&b))
                            }
                        }
                    }
                });
            }
        }
        ids
    });

    // Reset scroll to top when the filter or sort changes.
    use_effect(move || {
        let _ = config.read();
        let _ = sort_cfg.read();
        scroll_top.set(0.0);
        let _ = spawn(async move {
            let _ = document::eval(&format!(
                "let e=document.getElementById('{SCROLL_CONTAINER_ID}');if(e)e.scrollTop=0;"
            ))
            .await;
        });
    });

    let total = filtered_ids.read().len();
    let st = *scroll_top.read();
    let ch = *container_height.read();

    let start_idx = ((st / ITEM_HEIGHT) as usize).saturating_sub(SCROLL_BUFFER);
    let end_idx = (((st + ch) / ITEM_HEIGHT) as usize + SCROLL_BUFFER + 1).min(total);
    let offset_px = start_idx as f64 * ITEM_HEIGHT;

    let multi_active = store
        .read()
        .as_ref()
        .is_some_and(|s| s.active_profile_names().len() > 1);

    rsx! {
        div { class: "flex h-full",

            // ── List column ──────────────────────────────────────────────────
            div { class: "flex flex-col flex-1 xl:flex-none xl:w-[840px] min-w-0",

                div { class: "p-4 pb-2 shrink-0",
                    FilterToolbar { config, mode: FilterMode::Catalog }
                }

                SortHeader { sort_cfg }

                div {
                    id: SCROLL_CONTAINER_ID,
                    class: "flex-1 min-h-0 overflow-y-auto",
                    onmounted: move |data| handle_container_mounted(data, container_height),
                    onscroll: move |_| handle_scroll(scroll_top, scroll_pending),

                    if total == 0 {
                        div { class: "flex items-center justify-center h-32 text-sm text-gray-500 dark:text-gray-400",
                            "No cards match the current filters."
                        }
                    } else {
                        div { style: "height: {total as f64 * ITEM_HEIGHT}px; position: relative;",
                            div { style: "position: absolute; top: {offset_px}px; left: 0; right: 0;",
                                for idx in start_idx..end_idx {
                                    CatalogRow {
                                        key: "{filtered_ids.read()[idx]}",
                                        cv_id: filtered_ids.read()[idx],
                                        selected,
                                        multi_active,
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Detail panel (xl+ only) ──────────────────────────────────────
            div { class: "hidden xl:flex flex-col flex-1 min-w-52 border-l border-gray-200 dark:border-gray-700",
                DetailPanel { cv_id: selected }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SortHeader
// ---------------------------------------------------------------------------

#[component]
fn SortHeader(sort_cfg: Signal<SortConfig>) -> Element {
    rsx! {
        div { class: "flex items-center shrink-0 px-3 py-1 text-xs font-medium text-gray-500 dark:text-gray-400 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 gap-2",
            div { class: "w-14 shrink-0" }
            SortBtn {
                col: SortColumn::Name,
                label: "Name",
                sort_cfg,
                flex_class: "flex-1 min-w-0 text-left",
            }
            div { class: "hidden lg:block w-12 shrink-0" }
            div { class: "w-28 shrink-0" }
            SortBtn {
                col: SortColumn::Rarity,
                label: "Rarity",
                sort_cfg,
                flex_class: "hidden lg:block w-20 text-center",
            }
            SortBtn {
                col: SortColumn::Element,
                label: "Element",
                sort_cfg,
                flex_class: "hidden lg:block w-12 text-center",
            }
            SortBtn {
                col: SortColumn::PullRate,
                label: "Pull %",
                sort_cfg,
                flex_class: "hidden lg:block w-16 text-right",
            }
            SortBtn {
                col: SortColumn::OwnedCount,
                label: "Owned",
                sort_cfg,
                flex_class: "w-28 text-right",
            }
        }
    }
}

#[component]
fn SortBtn(
    col: SortColumn,
    label: &'static str,
    flex_class: &'static str,
    sort_cfg: Signal<SortConfig>,
) -> Element {
    let sc = sort_cfg.read();
    let active = sc.column == col;
    let dir = if active { Some(sc.dir) } else { None };
    drop(sc);
    let cls = if active {
        "cursor-pointer select-none text-blue-600 dark:text-blue-400"
    } else {
        "cursor-pointer select-none hover:text-gray-700 dark:hover:text-gray-300"
    };
    rsx! {
        button {
            r#type: "button",
            class: "{flex_class} {cls}",
            onclick: move |_| handle_sort_click(col, sort_cfg),
            span { class: "inline-flex items-center gap-0.5",
                "{label}"
                if let Some(d) = dir {
                    if d == SortDir::Asc {
                        ChevronUp { class: "w-3 h-3".to_string() }
                    } else {
                        ChevronDown { class: "w-3 h-3".to_string() }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CatalogRow
// ---------------------------------------------------------------------------

#[component]
fn CatalogRow(cv_id: usize, selected: Signal<Option<usize>>, multi_active: bool) -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let settings = use_context::<Signal<AppSettings>>();
    let mut back_origin = use_context::<Signal<CardDetailOrigin>>();
    let nav = use_navigator();

    let cv = &CardVersion::ALL[cv_id];
    let pd = &CARD_PULL_RATES[cv_id];

    let tint = element_tint_class(cv);
    let premium = premium_tint_class(cv);
    let border_style = row_border_style(cv);

    let merge = settings.read().merge_duplicate_printings();
    let (value, stored_count) = {
        let s = store.read();
        let s = s.as_ref();
        let agg = s.map_or(0, |s| s.aggregate_count(cv_id));
        let merged = if merge {
            cv.duplicates().iter().fold(agg, |acc, d| {
                acc.saturating_add(s.map_or(0, |s| s.aggregate_count(d.id())))
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
                    .map(|n| s.owned_count(n, cv_id))
            })
            .unwrap_or(0)
        };
        (merged, stored)
    };

    let is_selected = *selected.read() == Some(cv_id);
    let selected_class = if is_selected {
        "bg-blue-50 dark:bg-blue-900/20"
    } else {
        "hover:bg-gray-50 dark:hover:bg-gray-700/50"
    };

    let pull_label = if pd.max_pull_rate_pct > 0.0 {
        format!("{:.3}%", pd.max_pull_rate_pct)
    } else {
        "N/A".to_string()
    };
    let pull_title = pd
        .best_pack
        .map(|p| p.title().to_string())
        .unwrap_or_default();

    let set_code = cv.set().code();
    let number = cv.number().get();
    let name = cv.card().name();
    let rarity_icon = cv.rarity().class().icon();
    let card_image = cv.image();
    let element_icon = cv.card().pokemon().map(|p| p.element().icon());
    let set_icon = cv.set().icon();
    let (pack_logo, is_source_icon) = {
        let mut packs = cv.packs().iter();
        match (packs.next(), packs.next()) {
            (None, _) => (cv.source().icon(), true),
            (Some(p), None) => (p.logo(), false),
            _ => (cv.set().logo(), false),
        }
    };
    let logo_img_class = if is_source_icon {
        "h-10 w-auto object-contain"
    } else {
        "max-h-full w-full object-contain"
    };

    rsx! {
        div {
            class: "relative flex items-center gap-2 px-3 cursor-pointer {tint} {premium} {selected_class}",
            style: "height: {ITEM_HEIGHT}px; {border_style}",
            onclick: move |_| selected.set(Some(cv_id)),

            // Narrow-viewport: overlay that navigates to the full detail page.
            // Hidden at xl+ where the inline detail panel handles selection.
            div {
                class: "absolute inset-0 xl:hidden z-10",
                onclick: move |e| {
                    e.stop_propagation();
                    back_origin.set(CardDetailOrigin::Catalog);
                    drop(
                        nav
                            .push(Route::CardDetailPage {
                                card_id: cv_id,
                            }),
                    );
                },
            }

            img {
                src: "{card_image}",
                alt: "",
                loading: "lazy",
                class: "w-14 h-20 object-cover rounded flex-shrink-0",
            }

            div { class: "flex flex-col flex-1 min-w-0",
                span { class: "text-xs text-gray-400 dark:text-gray-500 tabular-nums leading-none",
                    "{set_code} {number:03}"
                }
                span { class: "text-sm font-medium text-gray-900 dark:text-gray-100 leading-snug",
                    "{name}"
                }
            }

            div { class: "hidden lg:flex w-12 h-full justify-center items-center flex-shrink-0",
                img {
                    src: "{set_icon}",
                    alt: "",
                    class: "h-full w-full object-contain",
                }
            }

            div { class: "flex w-28 h-full py-2 justify-center items-center flex-shrink-0",
                img { src: "{pack_logo}", alt: "", class: "{logo_img_class}" }
            }

            div { class: "hidden lg:flex w-20 justify-center flex-shrink-0",
                img {
                    src: "{rarity_icon}",
                    alt: "",
                    class: "h-6 max-w-full object-contain",
                }
            }

            div { class: "hidden lg:flex w-12 justify-center flex-shrink-0",
                if let Some(icon) = element_icon {
                    img {
                        src: "{icon}",
                        alt: "",
                        class: "h-5 w-5 object-contain",
                    }
                }
            }

            div { class: "hidden lg:block w-16 text-right flex-shrink-0",
                span {
                    class: "text-xs text-gray-600 dark:text-gray-400",
                    title: "{pull_title}",
                    "{pull_label}"
                }
            }

            // z-20 keeps spinner above the xl:hidden nav overlay beneath it.
            div { class: "w-28 flex justify-end flex-shrink-0 relative z-20",
                CountSpinner {
                    value,
                    stored_count,
                    disabled: multi_active,
                    on_change: move |n| set_card_count(cv_id, n, store),
                }
            }
        }
    }
}

use chrono::NaiveDate;
use dioxus::document;
use dioxus::prelude::*;
use ptcgp_db_core::save_data::{CardKindFilter, CountThreshold, FilterConfig};
use ptcgp_db_core::{AppSettings, CARD_PULL_RATES, ProfileStore};
use ptcgp_db_data::{Card, CardVersion};

use crate::app::{AppStorage, schedule_save};
use crate::components::count_spinner::CountSpinner;
use crate::components::effect_text::EffectText;
use crate::components::icons::{ArrowLeft, ChevronDown, ChevronUp};
use crate::components::{FilterMode, FilterToolbar};
use crate::routes::Route;

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
// Filter application
// ---------------------------------------------------------------------------

fn passes_filter(
    cv: &CardVersion,
    cfg: &FilterConfig,
    settings: &AppSettings,
    store: &ProfileStore<AppStorage>,
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
    if let Some(thresh) = cfg.owned_count {
        let count = store.aggregate_count(cv.id());
        let ok = match thresh {
            CountThreshold::Equal(n) => count == n,
            CountThreshold::LessThan(n) => count < n,
            CountThreshold::AtLeast(n) => count >= n,
        };
        if !ok {
            return false;
        }
    }
    true
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
// Count mutation helper
// ---------------------------------------------------------------------------

fn do_set_count(cv_id: usize, new_count: u32, mut store: Signal<Option<ProfileStore<AppStorage>>>) {
    let name = {
        let s = store.read();
        let Some(s) = s.as_ref() else { return };
        s.active_profile_names().first().cloned()
    };
    let Some(name) = name else { return };
    {
        let mut s = store.write();
        let Some(s) = s.as_mut() else { return };
        let _ = s.set_owned_count(&name, cv_id, new_count);
    }
    schedule_save();
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

    // Filtered + sorted indices. Recomputes on config, sort, store, or settings change.
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
            .filter(|cv| passes_filter(cv, &cfg, &sett, s, today, matched_name_ids.as_deref()))
            .map(|cv| cv.id())
            .collect();

        match sc.column {
            SortColumn::Default => {
                if sc.dir == SortDir::Desc {
                    ids.reverse();
                }
            }
            SortColumn::Name => ids.sort_by(|&a, &b| {
                let n = CardVersion::ALL[a]
                    .card()
                    .name()
                    .as_str()
                    .cmp(CardVersion::ALL[b].card().name().as_str());
                if sc.dir == SortDir::Asc {
                    n.then(a.cmp(&b))
                } else {
                    n.reverse().then(a.cmp(&b))
                }
            }),
            SortColumn::OwnedCount => ids.sort_by(|&a, &b| {
                let n = s.aggregate_count(a).cmp(&s.aggregate_count(b));
                if sc.dir == SortDir::Asc {
                    n.then(a.cmp(&b))
                } else {
                    n.reverse().then(a.cmp(&b))
                }
            }),
            SortColumn::Rarity => ids.sort_by(|&a, &b| {
                let n = CardVersion::ALL[a]
                    .rarity()
                    .class()
                    .id()
                    .cmp(&CardVersion::ALL[b].rarity().class().id());
                if sc.dir == SortDir::Asc {
                    n.then(a.cmp(&b))
                } else {
                    n.reverse().then(a.cmp(&b))
                }
            }),
            SortColumn::Element => ids.sort_by(|&a, &b| {
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
                let n = ea.cmp(&eb);
                if sc.dir == SortDir::Asc {
                    n.then(a.cmp(&b))
                } else {
                    n.reverse().then(a.cmp(&b))
                }
            }),
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
            // Fixed at 840px once the detail panel is visible (xl+); full-width below xl.
            // At xl the 208px sidebar leaves 1072px content; 840px list + 232px detail.
            div { class: "flex flex-col flex-1 xl:flex-none xl:w-[840px] min-w-0",

                // Filter toolbar
                div { class: "p-4 pb-2 shrink-0",
                    FilterToolbar { config, mode: FilterMode::Catalog }
                }

                // Sort header
                SortHeader { sort_cfg }

                // Virtual list scroll container
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
                        // Spacer that creates correct scrollbar height
                        div { style: "height: {total as f64 * ITEM_HEIGHT}px; position: relative;",
                            // Rendered window — only visible rows
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
            // Thumbnail placeholder column
            div { class: "w-14 shrink-0" }
            // Code + Name
            SortBtn {
                col: SortColumn::Name,
                label: "Name",
                sort_cfg,
                flex_class: "flex-1 min-w-0 text-left",
            }
            // Set icon placeholder (hidden below lg)
            div { class: "hidden lg:block w-12 shrink-0" }
            // Pack / set logo placeholder (always visible)
            div { class: "w-28 shrink-0" }
            // Rarity (hidden below lg)
            SortBtn {
                col: SortColumn::Rarity,
                label: "Rarity",
                sort_cfg,
                flex_class: "hidden lg:block w-20 text-center",
            }
            // Element (hidden below lg)
            SortBtn {
                col: SortColumn::Element,
                label: "Element",
                sort_cfg,
                flex_class: "hidden lg:block w-12 text-center",
            }
            // Pull rate (hidden below lg)
            SortBtn {
                col: SortColumn::PullRate,
                label: "Pull %",
                sort_cfg,
                flex_class: "hidden lg:block w-16 text-right",
            }
            // Owned count
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

            // Narrow-viewport: transparent overlay that navigates to the full detail page.
            // Hidden at xl+ where the inline detail panel handles selection instead.
            Link {
                class: "absolute inset-0 xl:hidden z-10",
                to: Route::CardDetailPage {
                    card_id: cv_id,
                },
            }

            // Thumbnail
            img {
                src: "{card_image}",
                alt: "",
                loading: "lazy",
                class: "w-14 h-20 object-cover rounded flex-shrink-0",
            }

            // Code + Name
            div { class: "flex flex-col flex-1 min-w-0",
                span { class: "text-xs text-gray-400 dark:text-gray-500 tabular-nums leading-none",
                    "{set_code} {number:03}"
                }
                span { class: "text-sm font-medium text-gray-900 dark:text-gray-100 leading-snug",
                    "{name}"
                }
            }

            // Set icon (hidden below lg — sidebar consumes 208px at md+, leaving too little room)
            div { class: "hidden lg:flex w-12 h-full justify-center items-center flex-shrink-0",
                img {
                    src: "{set_icon}",
                    alt: "",
                    class: "h-full w-full object-contain",
                }
            }

            // Pack / set logo (always visible)
            div { class: "flex w-28 h-full py-2 justify-center items-center flex-shrink-0",
                img { src: "{pack_logo}", alt: "", class: "{logo_img_class}" }
            }

            // Rarity icon (hidden below lg)
            div { class: "hidden lg:flex w-20 justify-center flex-shrink-0",
                img {
                    src: "{rarity_icon}",
                    alt: "",
                    class: "h-6 max-w-full object-contain",
                }
            }

            // Element icon (hidden below lg)
            div { class: "hidden lg:flex w-12 justify-center flex-shrink-0",
                if let Some(icon) = element_icon {
                    img {
                        src: "{icon}",
                        alt: "",
                        class: "h-5 w-5 object-contain",
                    }
                }
            }

            // Pull rate (hidden below lg)
            div { class: "hidden lg:block w-16 text-right flex-shrink-0",
                span {
                    class: "text-xs text-gray-600 dark:text-gray-400",
                    title: "{pull_title}",
                    "{pull_label}"
                }
            }

            // Count spinner — hidden below xl; at narrow widths editing is done on the
            // full-screen detail page to avoid overlap with the navigation overlay.
            div { class: "w-28 hidden xl:flex justify-end flex-shrink-0 relative z-20",
                CountSpinner {
                    value,
                    stored_count,
                    disabled: multi_active,
                    on_change: move |n| do_set_count(cv_id, n, store),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PackRateRow
// ---------------------------------------------------------------------------

#[component]
fn PackRateRow(pack_id: usize, percent: f64) -> Element {
    let Some(pack) = ptcgp_db_data::Pack::from_id(pack_id) else {
        return rsx! {};
    };
    let logo = pack.logo();
    let title = pack.title();
    rsx! {
        div { class: "flex items-center gap-2",
            img {
                src: "{logo}",
                alt: "",
                class: "h-8 w-16 object-contain flex-shrink-0",
            }
            span { class: "flex-1 text-sm text-gray-700 dark:text-gray-300", "{title}" }
            span { class: "text-sm tabular-nums font-medium text-gray-900 dark:text-gray-100",
                "{percent:.3}%"
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
                div { class: "flex items-center gap-0.5 shrink-0",
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
// VersionChip
// ---------------------------------------------------------------------------

#[component]
fn VersionChip(cv_id: usize, current_id: usize) -> Element {
    let cv = &CardVersion::ALL[cv_id];
    let set_code = cv.set().code();
    let number = cv.number().get();
    let rarity = cv.rarity().name();
    let is_current = cv_id == current_id;
    let cls = if is_current {
        "text-xs px-2 py-0.5 rounded-full border font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 border-blue-300 dark:border-blue-700"
    } else {
        "text-xs px-2 py-0.5 rounded-full border bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400 border-gray-200 dark:border-gray-700"
    };
    rsx! {
        span { class: "{cls}", title: "{rarity}", "{set_code} {number:03}" }
    }
}

// ---------------------------------------------------------------------------
// CardDetailBody — shared by DetailPanel (wide) and CardDetailPage (narrow)
// ---------------------------------------------------------------------------

#[component]
fn CardDetailBody(cv_id: usize) -> Element {
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let settings = use_context::<Signal<AppSettings>>();

    let cv = &CardVersion::ALL[cv_id];
    let merge = settings.read().merge_duplicate_printings();
    let (multi_active, value, stored_count) = {
        let s = store.read();
        let multi_active = s.as_ref().is_some_and(|s| s.active_profile_names().len() > 1);
        let s = s.as_ref();
        let agg = s.map_or(0, |s| s.aggregate_count(cv_id));
        let merged = if merge {
            cv.duplicates()
                .iter()
                .fold(agg, |acc, d| acc.saturating_add(s.map_or(0, |s| s.aggregate_count(d.id()))))
        } else {
            agg
        };
        let stored = if multi_active {
            agg
        } else {
            s.and_then(|s| s.active_profile_names().first().map(|n| s.owned_count(n, cv_id)))
                .unwrap_or(0)
        };
        (multi_active, merged, stored)
    };

    let set_code = cv.set().code();
    let number = cv.number().get();
    let name = cv.card().name();
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

    rsx! {
        div { class: "flex flex-col h-full overflow-y-auto",
            div { class: "flex justify-center p-4 bg-gray-50 dark:bg-gray-800/50 shrink-0",
                img {
                    src: "{card_image}",
                    alt: "{name}",
                    class: "h-64 w-auto object-contain rounded shadow-md",
                }
            }

            div { class: "flex flex-col gap-4 p-4",
                // Name + code + type badge
                div { class: "flex items-start justify-between gap-2",
                    div {
                        p { class: "text-lg font-bold text-gray-900 dark:text-gray-100 leading-tight",
                            "{name}"
                        }
                        p { class: "text-xs text-gray-400 dark:text-gray-500 tabular-nums",
                            "{set_code} {number:03}"
                        }
                    }
                    if is_pokemon {
                        span { class: "text-xs px-2 py-0.5 rounded-full bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 shrink-0 mt-0.5",
                            "Pokémon"
                        }
                    } else if is_trainer {
                        span { class: "text-xs px-2 py-0.5 rounded-full bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 shrink-0 mt-0.5",
                            "Trainer"
                        }
                    }
                }

                // Rarity
                div { class: "flex items-center gap-2",
                    img {
                        src: "{rarity_icon}",
                        alt: "",
                        class: "h-5 w-auto object-contain",
                    }
                    span { class: "text-xs text-gray-500 dark:text-gray-400", "{rarity_name}" }
                }

                // Owned count
                div { class: "flex items-center gap-2",
                    span { class: "text-sm text-gray-600 dark:text-gray-400", "Owned" }
                    CountSpinner {
                        value,
                        stored_count,
                        disabled: multi_active,
                        on_change: move |n| do_set_count(cv_id, n, store),
                    }
                }

                // Pack pull rates
                if is_pack_source && !pd.pack_pull_rates.is_empty() {
                    div { class: "flex flex-col gap-2",
                        p { class: "text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide",
                            "Pull Rates"
                        }
                        for pr in pd.pack_pull_rates.iter() {
                            PackRateRow { pack_id: pr.pack.id(), percent: pr.percent }
                        }
                    }
                }

                // Source description (non-pack)
                if !is_pack_source {
                    div { class: "text-xs text-amber-700 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 rounded p-2",
                        "{source_desc}"
                    }
                }

                // Pokémon-only fields
                if let Some(p) = pkmn {
                    div { class: "flex flex-col gap-3",
                        // Stats: Pokédex · HP · Stage
                        div { class: "grid grid-cols-3 gap-2",
                            div { class: "flex flex-col items-center p-2 rounded bg-gray-50 dark:bg-gray-800",
                                span { class: "text-xs text-gray-400 dark:text-gray-500",
                                    "Pokédex"
                                }
                                span { class: "text-sm font-bold tabular-nums text-gray-900 dark:text-gray-100",
                                    "#{p.base_pokemon().natdex_number()}"
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

                        // Element
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

                        // Retreat cost
                        div { class: "flex items-center gap-1.5",
                            span { class: "text-xs text-gray-400 dark:text-gray-500 w-14 shrink-0",
                                "Retreat"
                            }
                            if p.retreat_cost() == 0 {
                                span { class: "text-xs text-gray-500 dark:text-gray-400",
                                    "Free"
                                }
                            } else {
                                div { class: "flex items-center gap-0.5",
                                    for _ in 0..p.retreat_cost() {
                                        if let Some(colorless) = ptcgp_db_data::Element::ALL
                                            .iter()
                                            .find(|e| e.code() == Some('C'))
                                        {
                                            img {
                                                src: "{colorless.icon()}",
                                                alt: "",
                                                class: "h-4 w-4 object-contain",
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Weakness
                        if let Some(w) = p.weakness() {
                            div { class: "flex items-center gap-1.5",
                                span { class: "text-xs text-gray-400 dark:text-gray-500 w-14 shrink-0",
                                    "Weakness"
                                }
                                img {
                                    src: "{w.icon()}",
                                    alt: "",
                                    class: "h-4 w-4 object-contain",
                                }
                                span { class: "text-sm text-gray-700 dark:text-gray-300",
                                    "{w.name()}"
                                }
                            }
                        }

                        // Flags
                        if p.is_ex() || p.is_mega() {
                            div { class: "flex gap-2",
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
                            }
                        }

                        // Evolves from
                        if let Some(evo) = p.evolves_from() {
                            p { class: "text-xs text-gray-500 dark:text-gray-400",
                                "Evolves from "
                                span { class: "font-medium text-gray-700 dark:text-gray-300",
                                    "{evo}"
                                }
                            }
                        }

                        // Attacks
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

                        // Ability
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

                        // Flavor text
                        if let Some(ft) = p.flavor_text() {
                            p { class: "text-xs italic text-gray-400 dark:text-gray-500 leading-relaxed",
                                "{ft}"
                            }
                        }
                    }
                }

                // Trainer-only fields
                if let Some(t) = trainer {
                    div { class: "flex flex-col gap-3",
                        span { class: "self-start text-xs px-2 py-0.5 rounded-full bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300",
                            "{t.kind().name()}"
                        }
                        div { class: "p-2 rounded bg-gray-50 dark:bg-gray-800",
                            p { class: "text-xs text-gray-700 dark:text-gray-300",
                                EffectText { text: t.effect().to_string() }
                            }
                        }
                    }
                }

                // Duplicate printings
                if !duplicates.is_empty() {
                    div { class: "flex flex-col gap-2",
                        p { class: "text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide",
                            "Duplicate Printings"
                        }
                        div { class: "flex flex-wrap gap-1",
                            for d in duplicates.iter() {
                                VersionChip { cv_id: d.id(), current_id: cv_id }
                            }
                        }
                    }
                }

                // All versions of this abstract card
                if all_versions.len() > 1 {
                    div { class: "flex flex-col gap-2",
                        p { class: "text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide",
                            "All Versions"
                        }
                        div { class: "flex flex-wrap gap-1",
                            for v in all_versions.iter() {
                                VersionChip { cv_id: v.id(), current_id: cv_id }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DetailPanel — wide-viewport panel beside the catalog list
// ---------------------------------------------------------------------------

#[component]
fn DetailPanel(cv_id: Signal<Option<usize>>) -> Element {
    let Some(id) = *cv_id.read() else {
        return rsx! {
            div { class: "flex flex-col items-center justify-center h-full text-sm text-gray-400 dark:text-gray-600 p-6 text-center",
                "Select a card to view details."
            }
        };
    };
    rsx! {
        CardDetailBody { cv_id: id }
    }
}

// ---------------------------------------------------------------------------
// CardDetailPage — full-screen narrow-viewport detail page
// ---------------------------------------------------------------------------

#[component]
pub fn CardDetailPage(card_id: usize) -> Element {
    let nav = use_navigator();
    if card_id >= CardVersion::ALL.len() {
        return rsx! {
            div { class: "flex flex-col items-center justify-center h-full text-sm text-gray-400 dark:text-gray-600 p-6",
                "Card not found."
            }
        };
    }
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex items-center shrink-0 px-3 py-2 border-b border-gray-200 dark:border-gray-700",
                button {
                    r#type: "button",
                    class: "flex items-center gap-1 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100",
                    onclick: move |_| {
                        drop(nav.push(Route::CatalogPage {}));
                    },
                    ArrowLeft { class: "w-4 h-4".to_string() }
                    "Catalog"
                }
            }
            div { class: "flex-1 min-h-0",
                CardDetailBody { cv_id: card_id }
            }
        }
    }
}

use dioxus::prelude::*;
use ptcgp_db_core::save_data::{FilterConfig, SavedQueriesSaveData};
use ptcgp_db_core::{
    AppSettings, PackRowData, ProfileStore, SavedQueries, SummaryData, compute_summary,
};
use ptcgp_db_data::{Pack, Set};

#[cfg(target_arch = "wasm32")]
use ptcgp_db_core::storage::Storage as _;

use crate::app::{AppStorage, SummaryPageState};
use crate::components::icons::{ChevronDown, ChevronUp, XMark};
use crate::components::{FilterMode, FilterToolbar};
use crate::routes::Route;

// ---------------------------------------------------------------------------
// Dropdown trigger style — matches Set/Pack/Source dropdowns in the toolbar
// ---------------------------------------------------------------------------

const TRIGGER_CLS: &str = "flex items-center gap-1 px-2 h-8 rounded-md text-sm font-medium \
    bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 \
    text-gray-800 dark:text-gray-100 shadow-sm active:shadow-none active:translate-y-px";

// ---------------------------------------------------------------------------
// Navigation helpers
// ---------------------------------------------------------------------------

enum CatalogNav {
    Pack(usize),
    Set(usize),
}

/// Navigate to the catalog with a pack or set pre-selected, preserving the summary's active
/// filters (series, rarities, elements, etc.) but dropping goal, any-version, owned-count,
/// and name-query (which are summary-only).
fn apply_catalog_filter(
    nav: CatalogNav,
    summary_config: Signal<FilterConfig>,
    mut catalog_filter: Signal<FilterConfig>,
) {
    let summary = summary_config.read();
    let (packs, sets) = match nav {
        CatalogNav::Pack(id) => (vec![id], vec![]),
        CatalogNav::Set(id) => (vec![], vec![id]),
    };
    *catalog_filter.write() = FilterConfig {
        packs,
        sets,
        goal: 1,
        any_version_owned: false,
        owned_count: None,
        name_query: None,
        ..summary.clone()
    };
}

// ---------------------------------------------------------------------------
// Saved-query persistence
// ---------------------------------------------------------------------------

fn default_filter_config() -> FilterConfig {
    FilterConfig {
        goal: 1,
        obtainable: Some(true),
        ..FilterConfig::default()
    }
}

/// Persist saved queries after any mutation.
#[cfg(not(target_arch = "wasm32"))]
fn save_queries_data(data: SavedQueriesSaveData, store: Signal<Option<ProfileStore<AppStorage>>>) {
    let storage = store.read().as_ref().map(|s| s.storage().clone());
    let Some(storage) = storage else { return };
    if let Err(e) = storage.save_saved_queries_sync(&data) {
        tracing::error!("saved queries save failed: {e}");
    }
}

#[cfg(target_arch = "wasm32")]
fn save_queries_data(data: SavedQueriesSaveData, store: Signal<Option<ProfileStore<AppStorage>>>) {
    let storage = store.read().as_ref().map(|s| s.storage().clone());
    let Some(storage) = storage else { return };
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = storage.save_saved_queries(&data).await {
            tracing::error!("saved queries save failed: {e}");
        }
    });
}

// ---------------------------------------------------------------------------
// Saved-queries dropdown
// ---------------------------------------------------------------------------

#[component]
fn SavedQueryItem(
    name: String,
    cfg_snapshot: FilterConfig,
    config: Signal<FilterConfig>,
    mut queries: Signal<SavedQueries>,
    store: Signal<Option<ProfileStore<AppStorage>>>,
    mut open: Signal<bool>,
    mut active_query: Signal<Option<String>>,
) -> Element {
    let name_for_load = name.clone();
    rsx! {
        div { class: "flex items-center gap-1 px-3 py-2 select-none hover:bg-gray-50 dark:hover:bg-gray-600",
            div {
                class: "flex-1 min-w-0 cursor-pointer",
                onclick: move |_| {
                    config.set(cfg_snapshot.clone());
                    active_query.set(Some(name_for_load.clone()));
                    open.set(false);
                },
                span { class: "text-sm text-gray-700 dark:text-gray-300 truncate block",
                    "{name}"
                }
            }
            button {
                r#type: "button",
                class: "shrink-0 w-5 h-5 flex items-center justify-center rounded \
                        text-gray-300 dark:text-gray-600 \
                        hover:text-red-500 dark:hover:text-red-400 \
                        hover:bg-red-50 dark:hover:bg-red-950/30",
                title: "Delete",
                onclick: move |e| {
                    e.stop_propagation();
                    let was_active = active_query.read().as_deref() == Some(name.as_str());
                    let data = {
                        let mut q = queries.write();
                        q.remove(&name);
                        q.as_save_data().clone()
                    };
                    save_queries_data(data, store);
                    if was_active {
                        active_query.set(None);
                    }
                },
                XMark { class: "w-3.5 h-3.5" }
            }
        }
    }
}

#[component]
fn SavedQueriesDropdown(
    config: Signal<FilterConfig>,
    mut active_query: Signal<Option<String>>,
) -> Element {
    let queries = use_context::<Signal<SavedQueries>>();
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let mut open = use_signal(|| false);

    let query_list: Vec<(String, FilterConfig)> = queries
        .read()
        .queries()
        .iter()
        .map(|q| (q.name.clone(), q.config.clone()))
        .collect();
    let count = query_list.len();

    let active_name = active_query.read().clone();
    let (label, is_modified) = match &active_name {
        None => ("Queries".to_string(), false),
        Some(name) => {
            let cfg = config.read();
            let modified = queries
                .read()
                .queries()
                .iter()
                .find(|q| &q.name == name)
                .map(|q| q.config != *cfg)
                .unwrap_or(false);
            (name.clone(), modified)
        }
    };
    let is_named = active_name.is_some();

    rsx! {
        div { class: "relative",
            button {
                r#type: "button",
                class: "{TRIGGER_CLS}",
                onclick: move |_| open.toggle(),
                "{label}"
                if is_modified {
                    span { class: "text-amber-500 font-medium", " *" }
                }
                if !is_named && count > 0 {
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
                div { class: "absolute left-0 top-full mt-1 z-20 max-h-80 overflow-y-auto \
                              overflow-x-hidden rounded-md border border-gray-200/60 \
                              dark:border-gray-600/60 bg-white dark:bg-gray-700 \
                              shadow-xl dark:shadow-[0_4px_28px_rgba(0,0,0,0.7)] ring-1 ring-black/5 dark:ring-white/[0.09] py-1 \
                              min-w-48",
                    div {
                        class: "flex items-center px-3 py-2 cursor-pointer select-none \
                                hover:bg-gray-50 dark:hover:bg-gray-600",
                        onclick: move |_| {
                            config.set(default_filter_config());
                            active_query.set(None);
                            open.set(false);
                        },
                        span { class: "text-sm text-gray-500 dark:text-gray-400 italic",
                            "Default"
                        }
                    }

                    if !query_list.is_empty() {
                        div { class: "border-t border-gray-100 dark:border-gray-600 mt-1 pt-1",
                            for (name, cfg_snapshot) in query_list {
                                SavedQueryItem {
                                    key: "{name}",
                                    name,
                                    cfg_snapshot,
                                    config,
                                    queries,
                                    store,
                                    open,
                                    active_query,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Save Query dialog
// ---------------------------------------------------------------------------

#[component]
fn SaveQueryDialog(
    config: Signal<FilterConfig>,
    mut active_query: Signal<Option<String>>,
    on_close: EventHandler<()>,
) -> Element {
    let mut queries = use_context::<Signal<SavedQueries>>();
    let store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
    let mut name = use_signal(String::new);
    let mut error = use_signal(|| None::<&'static str>);

    let mut try_save = move || {
        let n = name.read().trim().to_string();
        if n.is_empty() {
            error.set(Some("Name cannot be empty"));
            return;
        }
        let n_saved = n.clone();
        let cfg = config.read().clone();
        let result = {
            let mut q = queries.write();
            if q.add(n, cfg) {
                Some(q.as_save_data().clone())
            } else {
                None
            }
        };
        if let Some(data) = result {
            active_query.set(Some(n_saved));
            save_queries_data(data, store);
            on_close.call(());
        } else {
            error.set(Some("A query with that name already exists"));
        }
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/40",
            onclick: move |_| on_close.call(()),
            div {
                class: "bg-white dark:bg-gray-800 rounded-xl shadow-2xl dark:shadow-[0_16px_48px_rgba(0,0,0,0.7)] \
                        ring-1 ring-black/10 dark:ring-white/10 border \
                        border-gray-200/60 dark:border-gray-700/60 p-5 w-80 flex flex-col gap-4",
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
                        onkeydown: move |e| match e.key() {
                            Key::Enter => try_save(),
                            Key::Escape => on_close.call(()),
                            _ => {}
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
                                hover:bg-gray-200 dark:hover:bg-gray-600 \
                                shadow-sm active:shadow-none active:translate-y-px",
                        onclick: move |_| on_close.call(()),
                        "Cancel"
                    }
                    button {
                        r#type: "button",
                        class: "px-3 py-1.5 rounded-md text-sm font-medium \
                                bg-blue-600 text-white hover:bg-blue-700 \
                                disabled:opacity-50 disabled:cursor-not-allowed \
                                shadow-md shadow-blue-500/30 dark:shadow-blue-900/70 \
                                active:shadow-sm active:translate-y-px",
                        disabled: name.read().trim().is_empty(),
                        onclick: move |_| try_save(),
                        "Save"
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sort state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Default)]
enum SortColumn {
    #[default]
    Default,
    Completion,
    BestPull,
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
// Column sort button
// ---------------------------------------------------------------------------

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
// Per-pack sub-row
// ---------------------------------------------------------------------------

#[component]
fn PackSubRow(
    pack: &'static Pack,
    completion_pct: f64,
    owned: usize,
    total: usize,
    rate_pct: f64,
    summary_config: Signal<FilterConfig>,
) -> Element {
    let nav = use_navigator();
    let catalog_filter = use_context::<Signal<FilterConfig>>();
    let pack_id = pack.id();
    let on_click = move |_| {
        apply_catalog_filter(CatalogNav::Pack(pack_id), summary_config, catalog_filter);
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
            div { class: "hidden sm:block flex-1 min-w-0",
                img {
                    src: "{pack.logo()}",
                    alt: "{pack.title()}",
                    class: "h-12 w-auto max-w-56 object-contain",
                }
            }
            div { class: "text-right whitespace-nowrap shrink-0",
                span { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                    "{completion_pct:.3}%"
                }
                span { class: "block sm:inline text-xs text-gray-400 dark:text-gray-500 sm:ml-1.5",
                    "{owned}/{total}"
                }
            }
            div { class: "text-right w-20 whitespace-nowrap shrink-0",
                if completion_pct >= 100.0 {
                    span { class: "text-sm text-green-600 dark:text-green-400 font-medium",
                        "Complete"
                    }
                } else if rate_pct <= 0.0 {
                    span { class: "text-sm text-gray-400 dark:text-gray-500", "—" }
                } else {
                    span { class: "text-sm text-gray-900 dark:text-gray-100", "{rate_pct:.3}%" }
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
    summary_config: Signal<FilterConfig>,
) -> Element {
    let mut expanded = use_signal(|| false);
    let nav = use_navigator();
    let catalog_filter = use_context::<Signal<FilterConfig>>();
    let set_name = set.name();
    let is_promo = set.is_promo();
    let is_expandable = !pack_rows.is_empty();
    let set_id = set.id();
    let on_click = move |_| {
        apply_catalog_filter(CatalogNav::Set(set_id), summary_config, catalog_filter);
        drop(nav.push(Route::CatalogPage {}));
    };

    rsx! {
        div { class: "border-b border-gray-100 dark:border-gray-700 last:border-0",
            div {
                class: "grid grid-cols-[1fr_auto_auto] gap-x-4 px-4 py-3 items-center cursor-pointer select-none hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors",
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
                    div { class: "flex flex-col sm:flex-row items-center gap-1 sm:gap-2",
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
                    }
                    if !is_obtainable {
                        span { class: "shrink-0 text-xs px-1.5 py-0.5 rounded-full bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400",
                            "Retired"
                        }
                    }
                }
                div { class: "text-right whitespace-nowrap",
                    span { class: "text-sm font-medium text-gray-900 dark:text-gray-100",
                        "{completion_pct:.3}%"
                    }
                    span { class: "block sm:inline text-xs text-gray-400 dark:text-gray-500 sm:ml-1.5",
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
                        span { class: "text-sm text-gray-900 dark:text-gray-100", "{best_rate_pct:.3}%" }
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
                            summary_config,
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
    let mut dialog_open = use_signal(|| false);
    let active_query: Signal<Option<String>> = use_signal(|| None);

    let mut summary_state_ctx = use_context::<Signal<SummaryPageState>>();
    let config: Signal<FilterConfig> = use_signal(|| summary_state_ctx.read().config.clone());
    let sort_cfg: Signal<SortConfig> = use_signal(SortConfig::default);

    use_drop(move || {
        summary_state_ctx.write().config = config.read().clone();
    });

    let store_guard = store.read();
    let settings_guard = settings.read();
    let cfg = config.read();

    let today = chrono::Utc::now().date_naive();

    let Some(store_ref) = store_guard.as_ref() else {
        return rsx! {
            div { class: "p-4 text-gray-500 dark:text-gray-400", "Loading…" }
        };
    };

    let SummaryData {
        mut set_rows,
        best_packs,
        total_owned,
        total_denom,
    } = compute_summary(store_ref, &cfg, &settings_guard, today);

    {
        let sc = sort_cfg.read();
        match sc.column {
            SortColumn::Default => {}
            SortColumn::Completion => {
                for row in set_rows.iter_mut() {
                    row.pack_rows.sort_by(|a, b| {
                        let cmp = a
                            .completion_pct
                            .partial_cmp(&b.completion_pct)
                            .unwrap_or(std::cmp::Ordering::Equal);
                        let cmp = if sc.dir == SortDir::Asc { cmp } else { cmp.reverse() };
                        cmp.then(a.pack.id().cmp(&b.pack.id()))
                    });
                }
                set_rows.sort_by(|a, b| {
                    let cmp = a
                        .completion_pct
                        .partial_cmp(&b.completion_pct)
                        .unwrap_or(std::cmp::Ordering::Equal);
                    let cmp = if sc.dir == SortDir::Asc { cmp } else { cmp.reverse() };
                    cmp.then(a.set.id().cmp(&b.set.id()))
                });
            }
            SortColumn::BestPull => {
                for row in set_rows.iter_mut() {
                    row.pack_rows.sort_by(|a, b| {
                        match (a.rate_pct > 0.0, b.rate_pct > 0.0) {
                            (true, false) => std::cmp::Ordering::Less,
                            (false, true) => std::cmp::Ordering::Greater,
                            (false, false) => a.pack.id().cmp(&b.pack.id()),
                            (true, true) => {
                                let cmp = a
                                    .rate_pct
                                    .partial_cmp(&b.rate_pct)
                                    .unwrap_or(std::cmp::Ordering::Equal);
                                let cmp = if sc.dir == SortDir::Asc { cmp } else { cmp.reverse() };
                                cmp.then(a.pack.id().cmp(&b.pack.id()))
                            }
                        }
                    });
                }
                set_rows.sort_by(|a, b| {
                    match (a.best_pack.is_some(), b.best_pack.is_some()) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        (false, false) => a.set.id().cmp(&b.set.id()),
                        (true, true) => {
                            let cmp = a
                                .best_rate_pct
                                .partial_cmp(&b.best_rate_pct)
                                .unwrap_or(std::cmp::Ordering::Equal);
                            let cmp = if sc.dir == SortDir::Asc { cmp } else { cmp.reverse() };
                            cmp.then(a.set.id().cmp(&b.set.id()))
                        }
                    }
                });
            }
        }
    }

    let overall_pct = if total_denom > 0 {
        total_owned as f64 / total_denom as f64 * 100.0
    } else {
        0.0
    };
    let collection_complete =
        best_packs.is_empty() && total_denom > 0 && total_owned == total_denom;

    let next_pack_cls = if best_packs.len() > 2 {
        "divide-y divide-gray-200 dark:divide-gray-700 max-h-96 overflow-y-auto"
    } else {
        "divide-y divide-gray-200 dark:divide-gray-700"
    };

    drop(cfg);
    drop(settings_guard);
    drop(store_guard);

    rsx! {
        div { class: "max-w-4xl mx-auto p-4 sm:p-6 space-y-6",
            h1 { class: "text-2xl font-bold text-gray-900 dark:text-gray-100", "Summary" }

            div { class: "flex flex-col gap-1.5",
                FilterToolbar { config, mode: FilterMode::Summary }
                div { class: "flex items-center gap-1.5",
                    SavedQueriesDropdown { config, active_query }
                    button {
                        r#type: "button",
                        class: "{TRIGGER_CLS}",
                        onclick: move |_| dialog_open.set(true),
                        "Save"
                    }
                    if *dialog_open.read() {
                        SaveQueryDialog {
                            config,
                            active_query,
                            on_close: move |_| dialog_open.set(false),
                        }
                    }
                }
            }

            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3",
                    "Overall"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200/80 dark:border-gray-700/80 p-4 space-y-3 shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] dark:ring-1 dark:ring-white/[0.06]",
                    if total_denom == 0 {
                        p { class: "text-sm text-gray-500 dark:text-gray-400",
                            "No cards match the current filters."
                        }
                    } else {
                        div { class: "flex items-baseline gap-4",
                            span { class: "text-3xl font-bold text-gray-900 dark:text-gray-100",
                                "{overall_pct:.3}%"
                            }
                            span { class: "text-sm text-gray-500 dark:text-gray-400",
                                "{total_owned} / {total_denom}"
                            }
                        }
                        div { class: "h-2 rounded-full bg-gray-200 dark:bg-gray-700 shadow-inner",
                            div {
                                class: "h-2 rounded-full bg-blue-500 transition-all shadow-sm",
                                style: "width: {overall_pct:.4}%",
                            }
                        }
                    }
                }
            }

            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3",
                    "Next pack to open"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200/80 dark:border-gray-700/80 p-4 shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] dark:ring-1 dark:ring-white/[0.06]",
                    if collection_complete {
                        p { class: "text-sm font-medium text-green-600 dark:text-green-400",
                            "Goal met for all matching cards!"
                        }
                    } else if best_packs.is_empty() && total_denom > 0 {
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
                                    class: "flex items-start gap-4 py-4 cursor-pointer rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/60 hover:shadow-sm transition-shadow",
                                    onclick: move |_| {
                                        apply_catalog_filter(CatalogNav::Pack(pack.id()), config, catalog_filter);
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
                                            "{rate.as_f64() * 100.0:.3}% chance of a desired card"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            section {
                h2 { class: "text-xs font-semibold uppercase tracking-wider text-gray-500 dark:text-gray-400 mb-3",
                    "Set completion"
                }
                div { class: "bg-white dark:bg-gray-800 rounded-lg border border-gray-200/80 dark:border-gray-700/80 overflow-hidden shadow-md dark:shadow-[0_4px_20px_rgba(0,0,0,0.55)] dark:ring-1 dark:ring-white/[0.06]",
                    div { class: "grid grid-cols-[1fr_auto_auto] gap-x-4 px-4 py-2 bg-gray-50 dark:bg-gray-800/80 text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider border-b border-gray-200/80 dark:border-gray-700/80 shadow-sm",
                        span { "Set" }
                        SortBtn {
                            col: SortColumn::Completion,
                            label: "Completion",
                            flex_class: "text-right",
                            sort_cfg,
                        }
                        SortBtn {
                            col: SortColumn::BestPull,
                            label: "Best pull",
                            flex_class: "w-20 text-right",
                            sort_cfg,
                        }
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
                                summary_config: config,
                            }
                        }
                    }
                }
            }
        }
    }
}

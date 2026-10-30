# Pack Point Purchases (Trade page 4th tab)

Branch `feature-pack-point-suggestions`. Adds "which cards should I buy with pack points?".

## Data API (ptcgp-db-data)
`Rarity::pack_point_cost()` and `Rarity::shine_dust()` — renamed by the user from
`craft_cost()` / `dupe_dust()`. Per-rarity, not per-card. Distinct cost tiers:
35, 70, 150, 400, 500, 1000, 1250, 1350, 1500, 2500.

## Core (`ptcgp-db-core/src/purchase_data.rs`)
- `PurchaseRec { cv, dest_count, needed, max_rate, cost, value }`
- `build_purchases(store, settings, cfg, today, matched_name_ids, max_cost: Option<u32>)`
  - eligible: aggregate count < goal, `max_pull_rate > 0`, set not retired
  - **sorted ascending by `cost * max_rate`** (lower = better buy). User explicitly
    corrected this direction; do not "fix" it to descending.
  - reuses `trade_data::raw_dest_count` and `filter::filter_card`
- `pack_point_cost_tiers()` — distinct sorted costs, used to build the max-cost filter options

## UI
- `Tab::Points` in `pages/trade/mod.rs`; `TradePageState.active_tab == 3`,
  `TradePageState.max_pack_points: Option<u32>`
- `MaxPointsDropdown` / `MaxPointsItem` — single-select, options are the cost tiers plus "Any".
  Always visible above the list (tab-scoped, like the Candidates "Show retired-set cards" toggle),
  NOT in the FilterToolbar.
- `PurchaseRow` in `pages/trade/rows.rs` — modelled on `CandidateRow`; cost pill replaces the
  Source/Dest lines of `ShareRow`.
- Nav sidebar label is now "Trade & Buy"; bottom bar short label stays "Trade"; page h1 "Trade & Buy".
- Card details: 2-tile `grid grid-cols-2` for Pack Points / Shine Dust, after the Owned+rarity row.

## Notes / open items
- Three separate `MemStorage` test stubs now exist in ptcgp-db-core (trade_data, summary_data,
  profile_store/tests) plus a 4th in purchase_data. Extracting a shared `#[cfg(test)] test_support`
  module is a worthwhile future cleanup; not done to avoid scope creep.
- Playwright is NOT installed in this environment (no python module, no browser cache), so the
  responsive screenshot check from `mem:feedback_playwright_testing` could not be run.

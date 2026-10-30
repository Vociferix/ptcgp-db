# Trade page fixes and debugging notes (PRs #67–#69)

## Bugs found and fixed

1. **#67 pack points vs retired sets** — `build_purchases` unconditionally excluded retired-set
   cards. Wrong: pack points are banked per set and stay spendable after retirement. Removed the
   special case; `filter_card` already applies the Obtainable filter + `ignore_unobtainable_sets`.

2. **#68 trade Card A suppression (the 1-star bug)** — `build_trades` required the source NOT to
   already own Card A (`src_cnt < goal`). Card A and Card B are chosen per (source, rarity class)
   and both must exist, so one overlapping copy on the source killed the ENTIRE class. Bit hardest
   at rare classes where the dest has excess of only one card; invisible at Diamond where there
   are dozens of candidates. Fix: two-tier ranking — prefer `src_cnt < goal`, fall back to
   already-owned instead of excluding. `TradeRec::card_a_source_wants` flags the fallback; the
   "You give" panel shows an amber "<source> already has this" badge.

3. **#69 Sources selector staleness** — `TradePageState.source_profiles` stores profile NAMES with
   no reconciliation. Activating/renaming/deleting a selected profile left a stale name, so
   `effective_inactive` filtered to empty and the page showed "No recommendations match the current
   filters" (blaming filters). Badge counts only valid names, so a fully stale selection looks
   identical to no selection; selector is hidden below 2 inactive profiles, so there may be no way
   to clear it. Fix: `use_effect` prunes unselectable names (empty ⇒ "all"), rename path carries
   the name over, shared `is_selectable_source` helper.

## Debugging technique that worked

For "recommendations are silently missing" bugs, write a **property test with an independent
oracle**: brute-force whether a valid pair exists, compare against what the builder returned,
sweep over goal / keep-threshold / merge-dupes / any-version. Use a deterministic xorshift PRNG
(no dev-dependency). `every_class_with_an_available_pair_is_listed` in trade_data.rs. Always
verify a new regression test FAILS against the old code before trusting it.

## Data facts (verified by diagnostics, 2026-09)

- Rarity class ids: 0-3 Diamond/1-4, 4 Star/1, 5 Star/2, 6 Star/3, 7 Shiny/1, 8 Shiny/2, 9 Crown/1.
- Star/3 and Crown/1 have ZERO tradable cards. All other classes are mostly tradable.
- The ONLY non-tradable Star/1 cards are promos (P-A: 16, P-B: 12). Every pack-set 1-star is tradable.
- No tradable card has `max_card_pull_rate == Prob::ZERO`, so the `max_rate != ZERO` guard on
  Card A is effectively dead code (and is NOT in DESIGN.md).
- **A4b (Deluxe Pack: ex) is the only set with a retirement date.** A1 is not retired.
- A1 227 = full art Bulbasaur, Art Rare, Star/1, Pack source, original, no duplicate printings.
- No card group has a non-tradable original with a tradable duplicate, so merge-dupes cannot make
  a group vanish.

## Dioxus API fact

`consume_context` / `try_consume_context` are plain functions (dioxus-core `global_context`), NOT
hooks — safe to call from event handlers and non-component fns. `use_coroutine_handle` wraps
`use_hook(consume_context)` and IS a hook. Use `try_consume_context` to reach page state (e.g.
`Signal<TradePageState>`) from another page's handler instead of prop-drilling.

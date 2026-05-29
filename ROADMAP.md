# PTCGP DB — Implementation Roadmap

Tasks are written at Agile story granularity. Each is self-contained enough for a single implementation session. Descriptions reference DESIGN.md rather than duplicating its content — read the relevant section there before starting each task.

**Status values**: `[ ]` Not started · `[~]` In progress · `[x]` Done

---

## Foundation

### T01 — Add ptcgp-db-core and ptcgp-db crate skeletons to workspace

**Depends on**: —

Create the `ptcgp-db-core` and `ptcgp-db` Cargo crates and register them in the workspace `Cargo.toml` `members` list.

- `ptcgp-db-core`: a `lib` crate. Add `ptcgp-db-data` as a path dependency. Minimal `src/lib.rs` placeholder for now.
- `ptcgp-db`: a `bin` crate for the Dioxus app. Add `dioxus` with the appropriate feature flags for web (WASM) and desktop. Add `ptcgp-db-core` and `ptcgp-db-data` as path dependencies. Create a `Dioxus.toml` configured for multi-platform builds.

See DESIGN.md §Workspace Structure for crate responsibilities. Verify that `cargo check --workspace` passes before closing this task.

- [x] **T01**

---

## ptcgp-db-core

### T02 — Storage trait and save data type definitions

**Depends on**: T01

In `ptcgp-db-core`, define the persistence interfaces — the trait and the types it operates on. Do not implement any backends yet.

- Define an async `Storage` trait with methods sufficient for `ProfileStore`, `AppSettings`, and `SavedQueries` to load and save their data.
- Define the save data structs (the types that are serialized). Every save data struct must include a `format_version` field. Use `serde` for derive macros. The internal format may vary per backend; the user-facing export format must be JSON.
- See DESIGN.md §What Is Stored and §Data Format and Versioning.

- [x] **T02**

---

### T03 — Web storage backend (IndexedDB)

**Depends on**: T02

In `ptcgp-db-core`, implement the `Storage` trait for web using IndexedDB. IndexedDB is required over LocalStorage per DESIGN.md §Storage Backends (asynchronous, more robust for structured data).

- Use `web-sys` directly or a lightweight wrapper such as `rexie`.
- Gate the implementation on `#[cfg(target_arch = "wasm32")]` (or a `web` feature flag) so it does not compile on non-web targets.

- [x] **T03**

---

### T04 — Desktop/mobile storage backend (file-based JSON)

**Depends on**: T02

In `ptcgp-db-core`, implement the `Storage` trait for desktop and mobile using a JSON file in the platform-appropriate user data directory.

- Use the `dirs` crate to locate the data directory. See DESIGN.md §Storage Backends for the per-platform paths (`~/.local/share/ptcgp-db/` on Linux, `%APPDATA%\ptcgp-db\` on Windows, etc.).
- Gate the implementation on non-wasm targets.

- [x] **T04**

---

### T05 — Data format versioning and migration

**Depends on**: T02

In `ptcgp-db-core`, implement the migration layer so the app can load save data from any older format version.

- On load, inspect `format_version` and apply any necessary migrations in sequence to bring the data to the current version.
- Migrations must be lossless where possible per DESIGN.md §Data Format and Versioning.
- Start with format version 1 (no historical data to migrate from). Structure the code so future migration steps are easy to add.
- Write tests that round-trip data through each defined version boundary.

- [x] **T05**

---

### T06 — Collection model and ProfileStore

**Depends on**: T02, T05

In `ptcgp-db-core`, implement the `ProfileStore` type. This is the central state object for all collection data. Per DESIGN.md §Profiles and §State Management:

- Owns all profiles and their owned-count data: a map from `(profile_name, CardVersionId)` to `u32`.
- Tracks which profiles are currently active (one or more) and which is the primary profile.
- Exposes methods to:
  - Read and write owned counts for a specific profile and card version.
  - Create, rename, and delete profiles.
  - Change which profile is primary.
  - Activate and deactivate profiles.
- Owns the `Storage` backend instance and triggers auto-saves: debounced 2 seconds after the last write. See DESIGN.md §Auto-Save.
- Enforces the deletion policy from DESIGN.md §Profile Manager: when the primary profile is deleted, auto-promote the profile with the largest total owned count; when the only active profile is deleted, activate the primary profile instead.

This type will be provided as a Dioxus context at the app root (T09).

- [x] **T06**

---

### T07 — AppSettings and SavedQueries types

**Depends on**: T02

In `ptcgp-db-core`, implement the `AppSettings` and `SavedQueries` types. Both will be provided as Dioxus context at the app root (T09). Per DESIGN.md §Settings and §Saved Queries:

`AppSettings`:
- Theme preference: Dark, Light, or System (default: System).
- `ignore_unobtainable_sets: bool` (default: `false`)
- `ignore_premium_mission: bool` (default: `false`)
- `ignore_gold_shop: bool` (default: `false`)
- `merge_duplicate_printings: bool` (default: `false`)
- Persisted via the `Storage` backend.

`SavedQueries`:
- A list of named filter configurations. Not profile-specific — shared across all profiles.
- The serializable filter configuration struct can be defined here or in `ptcgp-db` and passed in; coordinate with T14 (Shared Filter Toolbar).
- Persisted via the `Storage` backend.

- [x] **T07**

---

### T08 — Probability calculation engine

**Depends on**: T01 (for ptcgp-db-data types)

In `ptcgp-db-core`, implement the probability calculation functions used throughout the app. Per DESIGN.md §Probability Calculations:

1. **Per-card pull rate for a pack** — given a non-promo `Pack` and a `CardVersionId`, compute the aggregate `Prob` across all variants and slots using the formula in §Per-Card Pull Rate for a Pack.
2. **Probability of pulling any desired card** — given a non-promo `Pack` and a set of desired `CardVersionId`s, compute the union probability using the formula in §Probability of Pulling Any "Desired" Card.
3. **max_pull_rate for a card** — the highest per-card pull rate across all non-promo packs for a given `CardVersionId`. Used in Card Catalog rows, Summary page, and Trade page.
4. **Completion percentage** — given a collection, a target `T`, and a query set of card versions, compute `Σ min(count(c), T) / (|query| × T)` per §Completion Formula. Implement the "Merge duplicate printings" variant that collapses duplicate groups.

All intermediate arithmetic must use `Prob` (exact rational arithmetic). Only convert to `f64` or a percentage string for final display. Promo packs are always excluded from all calculations.

Write tests for the core formulas, including edge cases (all owned, none owned, T=2, merge enabled).

- [x] **T08**

---

## ptcgp-db — App Shell

### T09 — Dioxus app scaffolding

**Depends on**: T01, T06, T07

In `ptcgp-db`, build the full app skeleton:

- `App` root component with context providers for `ProfileStore`, `AppSettings`, and `SavedQueries`. See DESIGN.md §State Management.
- Hash-based routing for web using Dioxus's `#[derive(Routable)]`. Routes: Summary (`/`), Card Catalog (`/catalog`), Analysis (`/analysis`), Trade (`/trade`), Profile Manager (`/profiles`), Import/Export (`/import-export`), Settings (`/settings`). See DESIGN.md §Routing and §Pages. Desktop/mobile use native routing without hash prefix.
- First-run detection: on startup, if no save data exists, render the onboarding screen (T17) instead of routing to Summary.
- Stub `fn ComponentName() -> Element` for each page route so the app compiles and navigation works end-to-end before any page is built.

- [x] **T09**

---

### T10 — Tailwind CSS configuration

**Depends on**: T09

Wire Tailwind CSS into the `dx` build pipeline and configure the theme.

- Install Tailwind (v3 or v4 as appropriate for the `dx` version in use) and verify it runs during `dx build` and `dx serve`.
- In `tailwind.config.js`, define custom element theme colors under `theme.extend.colors.element` for all ten Pokémon element types. See DESIGN.md §Element Theme Colors for color character guidance. Derive specific hex values by visually inspecting the element icon images in `ptcgp-images/elements/icons/`. The colors will be used for row tints, Full Art borders, and other element-coded UI.
- Enable dark mode support with Tailwind's `dark:` variant. Use class-based dark mode (`.dark` class on `<html>`) so the app can control it programmatically from `AppSettings`.
- Verify a simple test component renders correctly in both light and dark mode before closing.

- [x] **T10**

---

## ptcgp-db — Shared Components

### T11 — Navigation layout

**Depends on**: T09, T10, T12

Implement the persistent navigation shell that wraps all pages. Per DESIGN.md §App Structure and Navigation and §Responsive Design:

- **Wide viewports**: a sidebar listing all page links, always visible.
- **Narrow viewports**: collapse to a bottom navigation bar, hamburger menu, or equivalent. The exact pattern is left to implementation judgment, but it must remain fully usable on a narrow phone screen.
- Use Dioxus router `<Link>` components for navigation. Highlight the active route.
- Embed the Profile Selector (T12) in the nav. On the Settings and Import/Export pages, hide or visually disable it since the active profile does not affect those pages (per DESIGN.md §Profile Selector).

- [x] **T11**

---

### T12 — Profile Selector component

**Depends on**: T09, T10

A reusable component embedded in the navigation for switching active profiles. Per DESIGN.md §Profile Selector:

- Displays the name(s) of currently active profiles.
- Opens a dropdown or popover listing all profiles with checkboxes (multi-select).
- Single-profile selection is the primary path; multi-select is an explicit secondary action — the UI must make selecting one profile the path of least resistance.
- Writes to `ProfileStore` context when the selection changes.
- Must be usable at any viewport width.

- [x] **T12**

---

### T13 — Count Spinner component

**Depends on**: T09, T10

A reusable owned-count editor used in Card Catalog rows and Card Details. Per DESIGN.md §Count Spinner and §Custom Widgets:

- Decrement (`−`) and increment (`+`) buttons plus a direct text input field.
- **Do not use `<input type="number">`** — browser-native up/down arrows cannot be styled consistently. Use a custom `<input type="text">` with numeric validation.
- Minimum: 0. Decrement at 0 is a no-op. When "Merge duplicate printings" is on, the no-op check is against the individual version's stored count, not the merged sum.
- Maximum: `u32::MAX`. Clamp; do not overflow or panic.
- Text input: clamp to `[0, MAX]` on blur or Enter. Reject (reset to previous) on non-numeric input.
- **Disabled state**: when multiple profiles are active, render the component as read-only and visually distinct; display the aggregate sum.

- [x] **T13**

---

### T14 — Shared Filter Toolbar component

**Depends on**: T09, T10

A configurable filter toolbar used by the Card Catalog, Analysis, and Trade pages. Per DESIGN.md §Card Catalog Page — Filters:

Implement all filter dimensions: Name/Number text (case-insensitive substring match on name and collector number), Series (single-select), Set (multi-select), Pack (multi-select), Rarity (multi-select, rendered as rarity class icon images), Card Kind (Pokémon/Trainer single-select), Ex (toggle), Mega (toggle), Stage (single-select), Element (multi-select), Foil (toggle), Card Source (multi-select), Obtainable (toggle), Owned count (threshold expression: `= 0`, `< N`, `>= N`).

The component must accept a configuration struct controlling:
- Which filter dimensions are visible (so Analysis and Trade can suppress Owned count and show a goal input instead — see DESIGN.md §Analysis Page).
- Default values for each filter.

All filters default to unset (no effect) unless the configuration overrides them. At narrow viewports, collapse filters behind a "Filters" toggle button that reveals a drawer or modal (DESIGN.md §Responsive Design).

The `SavedQueries` type in `ptcgp-db-core` (T07) must store a serializable representation of a filter configuration — coordinate the data shape between this component and T07.

- [x] **T14**

---

## ptcgp-db — Pages

### T15 — Settings page

**Depends on**: T09, T10, T11

Implement the Settings page at the `/settings` route. Per DESIGN.md §Settings:

- **Dark / Light / System theme toggle**: three-way selector. Changes take effect immediately by toggling the `.dark` class on `<html>`. Persisted in `AppSettings`.
- **Ignore unobtainable sets** toggle (default: off). Persisted in `AppSettings`.
- **Ignore Premium Mission cards** toggle (default: off). Persisted in `AppSettings`.
- **Ignore Gold Shop cards** toggle (default: off). Persisted in `AppSettings`.
- **Merge duplicate printings** toggle (default: off). Persisted in `AppSettings`.

All toggles read from and write to the `AppSettings` context. The Profile Selector is hidden on this page.

- [x] **T15**

---

### T16 — Profile Manager page

**Depends on**: T09, T10, T11

Implement the Profile Manager at its route (page, modal, or collapsible panel — implementation's choice). Per DESIGN.md §Profile Manager:

- **Create** a new profile: name input, required. Show an inline error if the name is already taken.
- **Rename** a profile: inline edit or dialog. Same uniqueness constraint.
- **Delete** a profile: show a confirmation dialog before proceeding. The deletion policy (auto-promote primary, auto-activate if active) is enforced in `ProfileStore` (T06), not here — just call the method.
- **Change the primary** profile: radio-style or explicit button selection.

- [x] **T16**

---

### T17 — First Run / Onboarding screen

**Depends on**: T09, T10, T18

Implement the first-run onboarding flow. Per DESIGN.md §First Run:

- Shown on startup when no save data exists (detected in T09's app root).
- Display a full-page screen or modal. Must contain:
  1. A profile name input (required to submit). The named profile becomes the primary profile.
  2. A prominent import option that triggers the import flow from T18. If the user completes an import, the imported profiles satisfy first-run requirements — skip profile creation and go directly to Summary.
- If the user dismisses (closes the modal, navigates away, or otherwise exits without submitting): silently create a profile named `"Main"` as the primary profile and proceed to Summary.
- Once completed (any path), store a flag so the screen is never shown again.

This task depends on T18 for the import action. It is acceptable to stub the import action in T17 as a placeholder and wire it up once T18 is done.

- [x] **T17**

---

### T18 — Import / Export page

**Depends on**: T09, T10, T11, T05, T06

Implement the Import/Export page at its route. Per DESIGN.md §Import / Export:

**Export**:
- A button that serializes all profiles and settings as a JSON file (using the canonical export format with `format_version`) and triggers a browser download or native file save dialog.
- Nice-to-have: XLSX export. CSV is an acceptable fallback. This is not a blocking requirement.

**Import**:
- A file picker accepting JSON files previously exported by the app.
- On name collision between an imported profile and an existing profile, prompt the user to choose overwrite or skip for each conflict.
- Apply migration logic (T05) if the imported file's `format_version` is older than the current version.
- After a successful import, reload `ProfileStore` context with the merged data.

The export format must include `format_version` per DESIGN.md §Data Format and Versioning. The Profile Selector is hidden on this page.

- [x] **T18**

---

### T19 — Summary page

**Depends on**: T09, T10, T11, T08

Implement the Summary page at the `/` route (the default home). Per DESIGN.md §Summary Page:

**Next pack to open**: using T08's probability engine, find the non-promo pack with the highest probability of yielding a card with aggregate owned count = 0 across active profiles. Display the pack name (using `Pack::title()` per DESIGN.md §Pack Display Name) and probability as a percentage. Exclude unobtainable packs by default. When the "Ignore unobtainable sets" setting is off, show a toggle to optionally include unobtainable packs; hide the toggle when the setting is on. Show a collection-complete message when all cards are owned.

**Set completion table**: for each set (and an overall total row), show:
- Completion % = owned card versions / total card versions in set (T = 1 formula)
- Probability of pulling a new card from the best pack for that set
- Promo sets: completion % only, no probability data
- Identify each set by its logo (`sets/logos/`) where space permits, icon (`sets/icons/`) otherwise

**Overall totals**: total owned card versions and total card versions across all sets, plus an overall completion %.

All displayed data reacts to the active profiles via `ProfileStore` context. The "Merge duplicate printings" setting (T15) affects completion counts per DESIGN.md §Completion Formula.

- [ ] **T19**

---

### T20 — Card Catalog page

**Depends on**: T09, T10, T11, T13, T14, T08

Implement the Card Catalog at the `/catalog` route. Per DESIGN.md §Card Catalog Page:

**Virtual list**: use the `dioxus-primitives` virtual list component. This is a hard requirement — see DESIGN.md §Virtual List for the rationale (3,289+ cards, >1 GB of images). Only visible rows plus a scroll buffer are rendered in the DOM. Rows are styled `div` elements acting as columns; do not use `<table>`.

**Each row** (DESIGN.md §Each Row):
- Card thumbnail (lazy-loaded via `asset!()` macro; unload when off-screen)
- Card code (e.g., `A2b 025`)
- Card name
- Rarity class icon image
- Max pull rate % across all non-promo packs (N/A for promo/non-Pack-source cards). Hovering shows a tooltip with the source pack's display name (per `Pack::title()` convention).
- Count Spinner (T13), disabled when multiple profiles are active

**Row visual styling** (DESIGN.md §Row Visual Styling) — four additive static CSS layers:
1. Element tint: ~10–15% opacity background in the card's element color (Trainers: no tint). Use the custom Tailwind element colors from T10.
2. Rarity group effect: Diamond = none; Star = subtle gradient border in element color; Shiny = sparkle/glitter overlay; Crown = gold border.
3. Foil effect: rainbow/iridescent gradient on the row border or background when `CardVersion::is_foil()`.
4. Premium source tint: distinct gold/amber overlay for "Premium Mission" and "Gold Shop" source cards.
No animations — all effects are static CSS only.

**Sorting** (DESIGN.md §Sorting):
- Default: canonical set order (series alphabetically → within series by release date ascending, promo sets last → collector number ascending). See §Sorting for the full canonical ordering — do not sort sets alphabetically by code.
- Additional sortable dimensions: Name, Owned count, Rarity, Element, Pull rate. Clickable column headers.

**Filters**: integrate the Shared Filter Toolbar (T14). Update results on every keystroke. Show a "no results" message when the filtered list is empty.

**Responsive**: at narrow widths, tapping a row navigates to Card Details (T21) as a full-screen page. At wide widths, selecting a row updates the detail panel without navigation.

- [ ] **T20**

---

### T21 — Card Details view

**Depends on**: T09, T10, T13, T20

Implement the card details display. Per DESIGN.md §Card Details View:

**Layout**:
- Wide viewports: a panel beside the catalog list. Updates in place as the user selects rows — no navigation.
- Narrow viewports: a full-screen page at a `/catalog/:card_id` route (or equivalent). A back button returns to the catalog.

**Common fields** (Pokémon and Trainer): large card image, card name, card code, rarity class icon + specific rarity name as supplemental text, card type, Count Spinner (T13), packs with per-pack pull rate (omit entirely when `CardVersion::source().name() != "Pack"`), card source description when source is not `"Pack"`, duplicate versions list (`CardVersion::duplicates()`), all versions of the same abstract card (`CardVersion::card().versions()`).

**Pokémon-only fields**: Pokédex number, element icon, stage, HP, retreat cost (Colorless energy count), weakness element, flavor text, ex/Mega flags, evolves-from name, attacks (name, energy cost as icons using `elements/icons/` with `no_cost.png` for 0 energy, damage with suffix, effect text), ability (name, effect text). In all effect and ability text, replace element placeholders (e.g., `[R]` for Fire, `[G]` for Grass) with inline `elements/symbols/` images.

**Trainer-only fields**: trainer kind (Item/Supporter/Stadium/Tool), effect text (with element placeholder substitution).

The layout of fields within the panel is left to implementation judgment.

- [ ] **T21**

---

### T22 — Analysis page

**Depends on**: T09, T10, T11, T14, T08

Implement the Analysis page at the `/analysis` route. Per DESIGN.md §Analysis Page.

**Filter toolbar**: reuse T14 configured for Analysis mode:
- Replace the owned-count threshold filter with a **goal number input** (T, default 1). Cards where `count < T` are "desired" and drive pack probabilities; all matching cards count toward the completion denominator regardless.
- Add an **"Any version owned" toggle**: when on, a card version is treated as owned if any version of the same abstract card has aggregate count > 0.
- Default the **Obtainable** filter to "obtainable only". Hide it when the global "Ignore unobtainable sets" setting is on.

**Results**: for each non-promo pack that can yield at least one desired card, show the pack name and probability (descending). Use T08's union probability formula.

**Completion display**: show completion % for the current query using the formula in DESIGN.md §Completion Display. If no desired cards exist (all matching cards already satisfy T), show a "fully met" message instead of pack probabilities.

**Saved queries**: a "Save" button prompts for a name and stores the current filter configuration in `SavedQueries` context (T07). A "Saved" dropdown or list allows loading or deleting previously saved queries.

- [ ] **T22**

---

### T23 — Trade page

**Depends on**: T09, T10, T11, T14, T08

Implement the Trade page at the `/trade` route. Per DESIGN.md §Trade Page.

**Filter toolbar + goal input**: same configuration as the Analysis page (T22) — defines what cards the destination "wants" and the target T. Default T = 1.

Active profiles = the destination. Inactive profiles = sources. All sections respect the "Merge duplicate printings", "Ignore unobtainable sets", "Ignore Premium Mission", and "Ignore Gold Shop" settings.

**Recommended Shares**: requires at least one inactive profile. For each eligible card (Diamond rarity, `is_tradable()`, aggregate destination count < T), rank by receive-value formula (`1.0 / (max_pull_rate × needed)`). Cards where `max_pull_rate = 0` are shown as a separate top-priority tier. Each recommendation lists the source profile and a "Record transfer" button that subtracts 1 from the source profile and adds 1 to the destination profile via `ProfileStore`. Show an empty state when no inactive profiles exist; in the single-profile case, prompt the user to create a second profile. See DESIGN.md §Recommended Shares for the full formula and tie-breaking rules.

**Recommended Trades**: requires at least one inactive profile. For each (source profile, rarity class) pair, find Card B (highest receive-value for destination) and Card A (lowest trade-candidate value for destination) per DESIGN.md §Recommended Trades. Rank recommendations by Card B's receive-value. Show both sides of each trade. Same empty state as Shares.

**Trade Candidates**: always shown regardless of profile count. Rank the destination's excess cards by trade-candidate value formula (`1.0 / (max_pull_rate × excess)`) ascending. Exclude unobtainable-set cards by default with an opt-in toggle; visually flag them when shown. See DESIGN.md §Trade Candidates.

- [ ] **T23**

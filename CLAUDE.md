# Claude Code Instructions

## General approach

DESIGN.md is the primary reference for all requirements and design decisions. Read the relevant
section before starting any task. When a requirement is unclear or a significant design decision is
not covered there, stop and ask rather than making assumptions.

After completing any task, update CLAUDE.md and/or memory with new patterns, API facts, or
constraints discovered during the work — before opening the PR. Since Claude authors most of the
code in this project, there is rarely a need to re-read files for orientation at the start of a
session; rely on memory instead and only read files when about to edit them.

## Crate ownership

`ptcgp-db-data` was written by the user. Claude's role in that crate is limited to doc comments and
tests. Do not restructure, rename, or otherwise modify its implementation without explicit
discussion and user approval.

## Hard constraints

These override any general defaults:

- **Styling**: Tailwind CSS only. No custom CSS except Tailwind theme configuration.
- **UI components**: Do not add third-party UI component crates. `dioxus-primitives` is the sole
  approved exception.
- **Dependencies**: Only crates with ≥ 100,000 downloads and updated within the past year. Ask for
  an exception if needed — some "done" crates qualify.
- **Unsafe**: No `unsafe` blocks without prior user approval. Approved blocks must be minimal in
  scope and include a `// SAFETY:` comment.
- **Panics**: No `.unwrap()` outside tests. Avoid `.expect()` unless the panic is genuinely
  impossible and self-evident from context.
- **Native form elements**: Never use `<input type="number">` or other browser-native widgets with
  styled decorations that can't be overridden. Implement custom Dioxus components instead.

## Code quality

- **File size**: Keep Rust files under 800 lines. Each type and its implementation typically gets
  its own file. Complex types may split across multiple files. Multiple small related types may
  share a file.
- **Documentation**: All public functions, types, traits, and constants must have doc comments.
  Most modules should have a module-level comment. Inline comments explain non-obvious logic only.
- **Code duplication**: Outside tests, avoid duplicating logic. Refactor with generics and traits.
- **Tests**: Test non-obvious behavior; omit tests for trivial logic. Place tests in a
  `#[cfg(test)] mod tests { ... }` block at the end of the file under test, or in a separate file
  (e.g., `src/foo/tests.rs`). Never interleave test code with production code.
- **Performance vs. build time**: Runtime performance takes priority. Long build times isolated to
  their own crate (like `ptcgp-db-data`) are acceptable.
- **Consistency**: Avoid two components that look or behave slightly differently but serve the same
  purpose. Build shared components and use them everywhere.

## Running the app

Run from the workspace root using `-p ptcgp-db`:

```
dx serve -p ptcgp-db --platform web
dx serve -p ptcgp-db --platform desktop
```

The `--platform` flag is required; `dx serve` without it cannot detect the target and will exit with
an error.

## Dioxus app patterns

### Contexts

The three root contexts are provided in `ptcgp-db/src/app.rs`:

| Type | What it holds |
|------|--------------|
| `Signal<Option<ProfileStore<AppStorage>>>` | All profile/collection data. `None` while storage is loading. |
| `Signal<AppSettings>` | Theme and filter settings. |
| `Signal<SavedQueries>` | Named Analysis page filter configs. |

Consume them with `use_context::<Signal<…>>()`. The `Option` on `ProfileStore` is only `None`
during the initial load; by the time any page component renders, it is always `Some`.

### Mutating ProfileStore

After any write to `ProfileStore`, call `schedule_save()` (defined in `app.rs`) to trigger the
2-second debounced auto-save coroutine:

```rust
use crate::app::schedule_save;

let mut store = use_context::<Signal<Option<ProfileStore<AppStorage>>>>();
store.write().as_mut().unwrap().set_owned_count(...)?;
schedule_save();
```

### Signal read guards — consolidate and hold across RSX

When reading several values from the same signal in one render, use a single `.read()` guard.
More importantly, hold the guard across the RSX block to borrow `Vec` fields as slices instead
of cloning them:

```rust
// Good — one guard, no heap allocations
let cfg = config.read();
let sets = cfg.sets.as_slice();   // &[usize], zero allocation
let series = cfg.series;          // Copy
// sets and series are valid through the entire rsx! call below

// Avoid — clones allocate new Vecs just to read them
let (sets, series) = {
    let cfg = config.read();
    (cfg.sets.clone(), cfg.series)
};
```

This is safe because:
- `Signal::read()` borrows from the signal's internal arena, not from the `Signal<T>` handle.
  The `Signal` variable can be freely copied into closures while a read guard is held.
- RSX construction is synchronous — the guard lives through the entire `rsx!` call.
- Props computed from the slice (e.g. `checked: sets.contains(&id)`) are scalar values copied
  into child component props before the function returns.
- Event handler closures capture `config: Signal<FilterConfig>` (Copy), not the slice.

When a child component already receives `Signal<FilterConfig>`, do not also pass a `Vec<usize>`
snapshot of one of its fields as a separate prop. Let the child read from the signal directly
and hold its own guard. This eliminates the parent-side `field.clone()` entirely.

### Signal write guards across await points

**Never hold a `Signal::write()` guard across an `.await`.** The guard is backed by a `RefCell`;
if another component renders while the guard is live it will panic. The pattern used in the
auto-save coroutine is:

1. Acquire a read guard, clone the data needed, drop the guard.
2. Perform the async I/O.
3. Acquire a write guard briefly to update flags (e.g. `mark_clean()`).

### `use_coroutine` is auto-provided as context

`use_coroutine` in Dioxus 0.7 automatically inserts the coroutine handle into the context tree.
Child components retrieve it with `use_coroutine_handle::<MessageType>()`. The `ScheduleSave`
ZST and `schedule_save()` helper in `app.rs` wrap this so callers don't need to know the type.

### Avoiding allocations in render

Render functions run on every state change. Avoid unnecessary heap allocations:

- **`Asset` is `Copy`** — do not call `.to_string()` on an `Asset` value. Use it directly or
  format it in RSX: `src: "{asset}"`.
- **RSX string formatting over `.to_string()`** — when a prop type is `String` and the value
  comes from a static method returning `&str`, prefer `"{s.code()}"` over `s.code().to_string()`.
  Both allocate the same way but the former is shorter.
- **Consuming RSX for loops** — when the items in a `Vec` are about to be moved into child
  component props anyway, iterate by value to avoid a clone per item:
  ```rust
  for (key, items) in owned_vec {   // moves items, no clone
      ChildComponent { key: "{key}", items, config }
  }
  ```
- **Vec vs HashSet for small collections** — for collections with n < ~30 elements (filter
  selections, rarity lists, element lists, etc.), `Vec` with `contains()` is faster than
  `HashSet` or `BTreeSet` due to cache locality. Do not reach for a set type unless the
  collection can grow unbounded or the lookup is in a hot inner loop.

## Component placement

Reusable, generic UI components (toggles, spinners, dropdowns, etc.) belong in
`ptcgp-db/src/components/`. Page-specific compositions of those components can live in the page
file. When in doubt: if a component could plausibly be used on more than one page, it goes in
`components/`.

## Tailwind patterns

### Element colors

Ten custom colors are defined in `ptcgp-db/tailwind.css` under `@theme`. Use them as standard
Tailwind utility classes:

| Element    | Class prefix example       |
|------------|---------------------------|
| Grass      | `bg-element-grass`        |
| Fire       | `bg-element-fire`         |
| Water      | `bg-element-water`        |
| Lightning  | `bg-element-lightning`    |
| Fighting   | `bg-element-fighting`     |
| Psychic    | `bg-element-psychic`      |
| Darkness   | `bg-element-darkness`     |
| Metal      | `bg-element-metal`        |
| Colorless  | `bg-element-colorless`    |
| Dragon     | `bg-element-dragon`       |

The same names work with any utility prefix: `text-element-fire`, `border-element-grass`, etc.

### Dark mode

Class-based dark mode is enabled. Use `dark:` variant classes normally; no extra setup needed
in components. The `.dark` class on `<html>` is managed by `app.rs` based on `AppSettings.theme`.

### Class string safety

Every Tailwind class name must appear as a complete, unbroken string in the source so the
scanner can detect it. Never build class strings with `format!()` or string interpolation that
splits a class name across fragments. Conditional classes are fine as branch literals:

```rust
// Good — each branch is a complete literal; scanner sees every class
let cls = if active { "bg-blue-600 text-white" } else { "bg-gray-200 text-gray-800" };

// Bad — "bg-{color}-600" is never a known class in source
let cls = format!("bg-{color}-600");
```

## Keyboard events

`Key` is re-exported in `dioxus::prelude` (already in scope via the glob import). Match on
`Key::Enter`, `Key::Escape`, etc. directly — do not call `.to_string()` and compare strings:

```rust
onkeydown: move |evt| {
    match evt.key() {
        Key::Enter => { /* commit */ }
        Key::Escape => { /* cancel */ }
        _ => {}
    }
},
```

## Icons

All icons use the SVG components in `components/icons.rs`, sourced from
[Heroicons v2](https://heroicons.com/) outline set. Never use Unicode glyphs (▲, ▼, ☰, ✓, +, −,
etc.) for UI icons — always use or add to the icon components.

Currently available:

| Component | Heroicons name | Used for |
|-----------|---------------|---------|
| `ChevronUp` | `chevron-up` | Dropdown open state |
| `ChevronDown` | `chevron-down` | Dropdown closed state |
| `Bars3` | `bars-3` | Filter panel toggle (hamburger) |
| `Check` | `check` | Selected-item indicator |
| `Plus` | `plus` | Count increment |
| `Minus` | `minus` | Count decrement |
| `ArrowLeft` | `arrow-left` | Back navigation |
| `XMark` | `x-mark` | Delete / dismiss |

Each component takes a `class: String` prop for Tailwind sizing and color (e.g.
`class: "w-4 h-4 text-gray-500 dark:text-gray-400"`). Icons render at `currentColor` so
text color classes control stroke color.

To add a new icon: copy the SVG `path` `d` attribute from heroicons.com, add a component following
the existing pattern in `icons.rs`, and add a row to this table.

## Shared components

### CountSpinner (`components/count_spinner.rs`)

Owned-card count editor. Props:

| Prop | Type | Purpose |
|------|------|---------|
| `value` | `u32` | Displayed value (may be merged sum when "Merge duplicate printings" is on) |
| `stored_count` | `u32` | Individual version's stored count; guards the decrement button |
| `disabled` | `bool` | `true` when multiple profiles are active → read-only |
| `on_change` | `EventHandler<u32>` | Called with the new *individual* stored count |

Increment/decrement call `on_change(stored_count ± 1)`. Text input commits on blur or Enter,
reverts on Escape or non-numeric input.

## PR process

Each task gets its own branch and pull request.

1. **Branch** — use a short kebab-case name (e.g. `fix-filter-crash`, `feature-history-graphs`)
2. **Work** — implement the task, commit
3. **Format** — run `dx fmt -p ptcgp-db` before pushing. CI runs `dx fmt --check` and will fail
   if RSX macros are not formatted. `cargo fmt` does not cover RSX.
4. **Push and open a PR** — `git push -u origin t07`, then open a PR against `master`
5. **CI** — all checks must pass before review
6. **Review** — address any comments left on the PR; every comment gets a response, code changes
   only where the comment calls for them
7. **Merge** — the user merges when satisfied; never merge yourself

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

## Running the app

Run from the crate directory that contains `Dioxus.toml`:

```
cd ptcgp-db/ptcgp-db
dx serve --platform web
dx serve --platform desktop
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

## PR process

Each task from the roadmap gets its own branch and pull request.

1. **Branch** — name after the task ID in lowercase (e.g. T07 → `t07`)
2. **Work** — implement the task, commit
3. **Format** — run `dx fmt -p ptcgp-db` before pushing. CI runs `dx fmt --check` and will fail
   if RSX macros are not formatted. `cargo fmt` does not cover RSX.
4. **Push and open a PR** — `git push -u origin t07`, then open a PR against `master`
5. **CI** — all checks must pass before review
6. **Review** — address any comments left on the PR; every comment gets a response, code changes
   only where the comment calls for them
7. **Merge** — the user merges when satisfied; never merge yourself

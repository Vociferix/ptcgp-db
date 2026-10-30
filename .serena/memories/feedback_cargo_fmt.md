# Pre-push checklist (mirrors CI exactly)

CI has **7** jobs (`.github/workflows/ci.yml`). Run all of these before every push:

1. `cargo fmt --all -- --check`
2. `dx fmt --check -p ptcgp-db`   (RSX; `cargo fmt` does NOT cover rsx! macros)
3. `cargo clippy --workspace -- -D warnings`   (note: CI does **not** pass `--all-targets`)
4. `cargo clippy -p ptcgp-db --target wasm32-unknown-unknown --features web -- -D warnings`
5. `cargo test --workspace`
6. **WASM tests** — `wasm-pack test --headless --chrome ptcgp-db-core`, or without Chrome:
   `cd ptcgp-db-core && CARGO_BUILD_TARGET=wasm32-unknown-unknown RUSTFLAGS="-D warnings" cargo build --tests`
7. `cargo audit`   (cargo-audit is usually NOT installed locally)
   Plus a **Web Build** job: `dx build -p ptcgp-db --platform web`

## Why step 6 matters (learned the hard way)

Adding a field to `AppSettingsSaveData` broke a struct literal in a test helper inside
`ptcgp-db-core/src/web_storage.rs`'s `#[cfg(target_arch = "wasm32")]` module. Native
`cargo check --all-targets` never compiles it, so **only** the WASM Tests job caught it.
`AppSettingsSaveData` is built as a literal in **4** places: `file_storage.rs`,
`migration.rs` (×2), and `web_storage.rs` (wasm-only).

Rule: any change to a type constructed in `#[cfg(target_arch = "wasm32")]` test code needs step 6.

## cargo audit notes

`cargo audit` fails only on **vulnerabilities**; unmaintained/unsound findings are already
allowlisted ("N allowed warnings found") and do not fail. Read the log for
`error: N vulnerability found` rather than assuming the warnings are the cause.

Transitive-only vulnerabilities are often fixable with `cargo update -p <crate>` when the fix is
within the existing semver range — no dependency's requirement has to change. Did exactly that for
`rustls` 0.23.43 → 0.23.45 (RUSTSEC-2026-0285). `rustls` is a lockfile-only entry, not in any
built graph.

Gotcha: a `cargo update -p X` can get silently reverted if a background cargo (rust-analyzer)
re-resolves before the new version is in the local registry cache. **Commit the lockfile
immediately after updating, then re-verify** `git show HEAD:Cargo.lock`.

## Local blocker to be aware of

The user's `ptcgp-db-data/codegen/generate.rs` emits `pub const X: &'static str`, which trips
`clippy::redundant_static_lifetimes` under `-D warnings`. While that is uncommitted in their
working tree, local clippy runs need `-A clippy::redundant_static_lifetimes` to get past it;
CI is unaffected because it builds committed code only.

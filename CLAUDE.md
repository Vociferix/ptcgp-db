# Claude Code Instructions

## General approach

DESIGN.md is the primary reference for all requirements and design decisions. Read the relevant
section before starting any task. When a requirement is unclear or a significant design decision is
not covered there, stop and ask rather than making assumptions.

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

## PR process

Each task from the roadmap gets its own branch and pull request.

1. **Branch** — name after the task ID in lowercase (e.g. T07 → `t07`)
2. **Work** — implement the task, commit
3. **Push and open a PR** — `git push -u origin t07`, then open a PR against `master`
4. **CI** — all checks must pass before review
5. **Review** — address any comments left on the PR; every comment gets a response, code changes
   only where the comment calls for them
6. **Merge** — the user merges when satisfied; never merge yourself

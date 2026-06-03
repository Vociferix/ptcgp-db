> **Disclaimer:** The literal and graphical information presented in this application about Pokémon
> Trading Card Game Pocket, including card data, text and images, is copyright The Pokémon Company,
> DeNA Co., Ltd., and/or Creatures, Inc. This application is not produced by, endorsed by, supported
> by, or affiliated with any of those copyright holders.

---

# PTCGP DB

A web app (and desktop app) for tracking your Pokémon TCG Pocket card collection and browsing the
full database of cards, packs, and sets.

**Live app:** https://vociferix.github.io/ptcgp-db  
**License:** [Apache-2.0](LICENSE) *(software only — see license for details)*

This project was largely (though not entirely) written by [Claude](https://claude.ai).

## Features

- Browse all cards, sets, and booster packs
- Track owned card counts across one or more profiles
- Filter and search the card catalog by set, element, rarity, and more
- Pull-rate analysis and pack-opening statistics
- Light and dark themes
- Fully serverless — runs from static files with no backend

## Technology

- **Language:** Rust
- **UI framework:** [Dioxus](https://dioxuslabs.com/) — single codebase targeting web (WASM) and desktop
- **Styling:** Tailwind CSS

The workspace contains three crates:

| Crate | Purpose |
|-------|---------|
| `ptcgp-db` | Dioxus UI — pages, components, routing |
| `ptcgp-db-core` | Business logic — collection model, probability calculations, storage |
| `ptcgp-db-data` | Build-time generated card/pack/set data (includes git submodules) |

## Building

### Prerequisites

- Rust toolchain (stable)
- [`dioxus-cli`](https://github.com/DioxusLabs/dioxus): `cargo install dioxus-cli`
- Node.js (for Tailwind CSS)

Card data and images live in git submodules. Initialize them before building:

```sh
git submodule update --init --recursive
```

> **Note:** The `ptcgp-db-data` crate generates all card data at compile time. The first build
> can take up to 10 minutes; subsequent incremental builds are much faster.

### Web

```sh
dx serve -p ptcgp-db --platform web
```

### Desktop

```sh
dx serve -p ptcgp-db --platform desktop
```

### Release build

```sh
dx build -p ptcgp-db --platform web --release
```

Output is written to `ptcgp-db/dist/`.

# Development guide

Umran has a Rust simulation engine and a browser workbench. All linguistic
logic runs in Rust. The front end presents the engine's results, and there
is no backend.

## Run locally

Install [rustup](https://rustup.rs/) and [Bun](https://bun.sh/). The Rust
version is pinned in `rust-toolchain.toml` and installs through rustup.
Bun manages the front-end dependencies, including `wasm-pack`.

```sh
bun install
bun run dev
```

This builds the WebAssembly bindings and serves the workbench at
`http://127.0.0.1:5173/`. Generated bindings live in `web/src/wasm` and are
ignored in Git.

## Build and check

```sh
bun run build           # WASM, TypeScript check, and a static site in dist/
bun run test:web        # browser-side unit and rendering tests
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

`bun run check` checks TypeScript without rebuilding WASM. Browser tests
also import the generated bindings, so run `bun run build:wasm` first when
starting from a fresh checkout or after changing the facade.

Prose checks use Vale 3.25.1:

```sh
vale sync --no-global
bun run test:prose
bun run lint:prose
```

[Prose and linting](prose.md) describes the rules, audit findings, and
coverage limits.

## Test offline support

Run `bun run build`, then `bun run preview` and open
`http://127.0.0.1:4173/`. Offline support is enabled only in production
builds. Deployment requires HTTPS, with localhost also supported.

The manifest, icons, and service worker support subpaths such as `/umran/`.
Fonts fetched while the worker is active are cached too. Fonts not yet
cached fall back to local serif faces when offline.

For installation, saved worlds, and update behaviour, see
[offline use and saving](guide.md#offline-use-and-saving).

## Architecture

- **Rust engine (`crates/umran-sim`).** Holds all the linguistics, has no
  knowledge of the browser, and is tested natively.
- **WebAssembly facade (`crates/umran-web`).** Uses `wasm-bindgen` to keep
  the history and serve ready-to-show JSON views to the browser.
- **React and TypeScript front end (`web/`).** Built with Vite. It handles
  presentation and never makes or changes a word. Worlds run in an engine
  worker so simulation does not block the page.

### Deterministic histories

Every random draw comes from a ChaCha8 stream keyed by its purpose.
Adding a new process leaves existing streams' draws unchanged. The same
seed gives the same history natively and in WebAssembly.

A saved world is a seed plus the actions taken. Past years are recovered
by replaying, with cached checkpoints. The [history guide](history.md)
covers exact reading points, branching, save revisions, and recovery.

### Linguistic evidence

The comparative method reconstructs relationships without reading the
true family tree. Its results are evaluated against that record
separately.

Sound frequencies come from [PHOIBLE](https://phoible.org/), the basic
vocabulary is the Leipzig–Jakarta list, and borrowing is checked against
the [World Loanword Database](https://wold.clld.org/). Statistical tests
check behaviour in bands over many seeds rather than exact outputs.

The [engine model](engine.md) describes the implementation and calibration
module by module. Its [command-line examples](engine.md#studying-one-mechanism)
study individual mechanisms, such as word erosion or family divergence.
The [product frame](book.md) describes the book, four screens, and the
found → run → read → revise loop.

## Project layout

```text
crates/umran-sim   the engine: world, map, sounds, words, names, history
crates/umran-web   the WebAssembly facade and its JSON views
web/src           the React workbench and engine worker
scripts/          parity checks, prose linting, and font tooling
tests/            browser-side unit and rendering tests
docs/             user guides, engine model, product frame, and save format
brand/            the astrolabe mark and logo generator
AGENTS.md         architecture notes and rules every change keeps
```

Edit `brand/build.js` and run `bun run brand` to redraw the icons and logos.
[AGENTS.md](../AGENTS.md) records the engine invariants and component map.

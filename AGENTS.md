# Umran

A seeded language-change simulator. Communities with sound preferences grow,
split, meet, and rule one another, and their languages change in response:
regular sound change, borrowing, word competition, and language shift. Rust
engine, browser workbench. The name is Ibn Khaldun's *ʿumrān*, settled
social life, the subject of his *Muqaddimah*. It was called langgen until
October 2026; saved data keeps that name (see `docs/history.md`).

Read further when the task touches it:

- `docs/book.md`: the product frame: the book of a world as the product,
  the found → run → read → revise loop, the book's chapters, the four
  screens, the six-word vocabulary, and the cause contract on annals.
  Any UI work starts here.
- `docs/engine.md`: the engine model, module by module, the mechanism
  examples, and what is not yet modelled.
- `docs/history.md`: actions, replay, branching, recipes, and autosave.
- `README.md`: the public face of the project, with screenshots in
  `docs/images/`. Refresh it when the workbench changes visibly.
- `brand/build.js`: draws the astrolabe mark and writes the favicon, app
  icons, the brand lettering's CSS mask, and the README logos. Edit it,
  not its outputs, and run `bun run brand`.

## Architecture

- `crates/umran-sim`: the engine, independent of the browser.
- `crates/umran-web`: `wasm-bindgen` facade. `Bench` holds the logic and
  is tested natively; `Workbench` is a thin wrapper, since `JsValue` panics
  off WASM.
- `web/src`: React, TypeScript, and Vite. `model.ts` mirrors the facade's
  JSON views; `engine.ts` adapts the WASM; `shelf.ts` stores the saved
  worlds (still keyed as books, so old saves open) and names a world
  after its earliest-named continent unless the author titled it;
  `takeout.ts` formats exports; `lore.ts` holds shared names, the
  mechanism phrases for recorded causes, and the glossary of linguistic
  terms; `App.tsx` holds view state and actions. `styles.css` and
  `chart.css` hold the one look, an explorer's chart of strange lands,
  by day or by lamplight (`LightSwitch`). `components/` holds the four
  screens of `docs/book.md`: the chart room (`ChartRoom`: the author's
  worlds, the last opened leading, each a miniature of its map drawn
  from its seed with the peoples saved in the shelf index); founding
  (`Founding`: the chart with its title box and the book of accounts,
  one line per people when closed, five stages when open, each choice a
  `Phrase` that opens a menu; `ReachOverlay` draws who can meet whom);
  the workshop (`Stage`: `MapView` with its layers, label policy by
  zoom, and the `lens` that draws the chart as one people knew it; the
  world's cartouche and menu top-left, Find and the layers top-right,
  the chronicle's latest line as a caption over the foot, and a time bar
  whose primary verb runs to a year in chunked frames, with play and
  step in a study drawer, and whose `Timeline` clusters turning points
  over a quiet density of sound changes; the cards in `Pedia`, the
  frontispiece as the home card, each subject card opening in
  derivation order with a `Decide…` menu, a card's long sections
  (`Leaf`) opening wide over the map; `LanguageChapter` holds the
  language sections the card and the book share); and the book (`Book`:
  the world read in chapter order, laid over the map like a wide card).
  Every menu is a `Popover`: one open at a time, closed by an outside
  click or Escape. Annals carry the peoples, lands, and laws they tell
  of, so every entry can link to their cards, and an optional `cause`
  naming the entry that triggered them where the engine recorded it.
  Every language view carries a specimen, a few basic words chosen by
  `SPECIMEN` in the facade (`Specimen.tsx` shows them), and sound-change
  annals carry it with each word's form before.

All linguistic logic runs in Rust. Presentation code never mints or changes
words, and there is no backend or second implementation of linguistic
rules. Bun manages frontend dependencies; Rust is pinned in
`rust-toolchain.toml`; generated WASM bindings are ignored in Git.

## Rules that must hold

- Every random draw comes from a ChaCha8 stream keyed by purpose
  (`rng.rs`); a new process gets its own stream or draws after existing
  ones, so it never shifts earlier draws.
- Draw indices with `rng::index`, never `gen_range` over `usize`: `usize`
  ranges sample 64 bits natively but 32 in WASM, so histories would differ
  between tests and the browser.
- Use `math.rs`'s `libm` functions for transcendental math, never std float
  methods, so native and WASM histories match.
- Sound laws apply regularly to every living word and name, never to
  obsolete words, never delete a word's last vowel, and never wear a word
  below its language's minimal word.
- Borrowability is per concept, never per semantic field. WOLD figures
  (`wold.rs`) are for validation only.
- The comparative method (`compare.rs`) never reads lineage.
- Catalog segments are appended, never inserted.
- Bump `ENGINE_REVISION` (`chronicle.rs`) whenever an existing recipe would
  replay differently.
- Never delete a user's saved data; unreadable saves open recovery.

## Commands

```sh
bun install
bun run dev                 # builds WASM, serves http://127.0.0.1:5173
bun run build               # WASM, TypeScript check, static dist/ (all deployment needs)
bun run brand               # redraws icons and logos from brand/build.js
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
```

## Testing

- Tests that study one mechanism use `Params::static_society()`.
- Statistical tests check bands over many seeds, not exact values. Tune
  `Params` against the `calibrate` example, then confirm the tests hold.

## Working with Sol

Sol is learning through the design, wants architectural choices and
linguistic terms explained, and permits clean redesign rather than
protecting incidental implementation. The framing is loosely Toynbee
rather than Spengler: inherent sound and worldview biases plus challenge
and response, not fixed civilizational life cycles.

## Advisor

"Astra" is `modelRoles.advisor` (`openai-codex/gpt-6-astra:high`). Spawn
agent name `advisor`, model `@advisor`, user agent
`~/.omp/agent/agents/advisor.md`. Advise only unless Sol asks for edits.

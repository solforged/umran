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
- `docs/prose.md`: prose guardrails, audit findings, and lint coverage.
  Use it when editing documentation or interface copy.
- `README.md`: the short introduction, usage, and development quickstart.
  Keep detailed feature descriptions in `docs/guide.md`, with screenshots
  in `docs/images/`. Refresh both when the workbench changes visibly.
- `docs/development.md`: developer setup, checks, architecture, and layout.
- `docs/about.md`: the name and mark.
- `brand/build.js`: draws the astrolabe mark and writes the favicon, app
  icons, the brand lettering's CSS mask, and the README logos. Edit it,
  not its outputs, and run `bun run brand`.

## Architecture

- `crates/umran-sim`: the engine, independent of the browser.
- `crates/umran-web`: `wasm-bindgen` facade. `Bench` holds the logic and
  is tested natively; `Workbench` is a thin wrapper, since `JsValue` panics
  off WASM.
- `web/src`: the browser workbench, built with React, TypeScript, and Vite.

### Browser data and presentation

- `model.ts` mirrors the facade's JSON views. `engine.ts` adapts the WASM
  and runs every world in `engine.worker.ts`, with messages typed in
  `engineProtocol.ts`, so the page stays live while years pass.
- Reads return a `Read`. Components take it with React's `use` inside
  `<Pending>` or with `useAnswer` (`components/Pending.tsx`). A write starts
  a new reading of the tip while old readings keep their answers.
- `shelf.ts` stores saved worlds (still keyed as books, so old saves open),
  founding drafts, put-away worlds, and save revisions for cross-tab safety.
  Before another engine revision overwrites a save, it keeps that save as
  it stood. It names a world after its earliest-named continent unless the
  author titled it.
- `route.ts` names hash routes for the chart room, founding drafts, worlds,
  and book chapters. `App.tsx` holds view state and actions.
- `edition.ts` and `print.css` render editions from the book's own chapters.
  `takeout.ts` downloads files. The facade's `frozen.rs` writes the frozen
  world: a save with the world it resolved to beside the recipe as data.
- `archive.ts` keeps each world's frozen record per engine revision in
  IndexedDB and asks the browser to keep the site's data. The facade's
  `drift.rs` compares a record with this engine's replay, and the
  revision warning shows the result (`DriftNotice`).
- `lore.ts` holds shared names, mechanism phrases for recorded causes, and
  the linguistic glossary.
- `styles.css` and `chart.css` hold the one look: an explorer's chart of
  strange lands, by day or lamplight (`LightSwitch`).

### The four screens

`components/` holds the screens described in `docs/book.md`.

- **Chart room (`ChartRoom`).** The author's worlds, with the last opened
  leading. Each has a miniature drawn from its seed, with the peoples saved
  in the shelf index. With no saved worlds, a title page shows the sample's
  chart.
- **Founding (`Founding`).** Five steps edit one engine draft along a rail
  over the chart: the world, the homeland, the peoples, their speech, and
  the title page. `FormingChart` plays the engine's plate-drift frames.
  Voice cards preview each preset. A people's portrait sets way of life,
  the six leanings against the engine's `foundingTraits` draw, standing,
  openness, and year-0 crafts, realm, and faith. Each choice has a `lore.ts`
  line on what it drives. The sixth step is the workshop's first run,
  `Stage`'s `runOnOpen`.
- **Workshop (`Stage`).** The chart, cards, chronicle caption, and time bar.
  Its components are listed below.
- **Book (`Book`).** A full-window reading in chapter order, with a contents
  rail, saved bookmark, and the author named on the frontispiece.

### Workshop components

- `MapView` has chart and globe projections, reach, layers, zoom, and the
  `lens` that draws the chart as one people knew it. Globe rotation runs
  imperatively outside React rendering. The world's cartouche and menu sit
  top-left, with Find and layers top-right.
- `gazetteer.ts` arranges the facade's native names and exonyms.
  `MapInspector` reads a place and pins it before opening its card.
- `FortunesTour` follows a people's recorded entries and journeys, restoring
  the prior reading and camera when done.
- The chronicle's latest line is a caption over the foot. The time bar runs
  to a year in chunked frames, with play and step in a study drawer.
  `Timeline` clusters turning points over a quiet density of sound changes.
- `Pedia` holds the cards, with the frontispiece as home. Each subject opens
  in derivation order with a `Decide…` menu. Long sections (`Leaf`) open wide
  over the map, and the author's notes sit in the margin (`Margin`).
- `eras.ts` reads the chronicle in eras, opened by a state's rise or fall,
  a conquest, a faith, or a classical language. Each year's bookkeeping
  folds into one quiet line.
- `LanguageChapter` holds the language sections shared by cards and the
  book: `LifeRibbon`, law ledgers, loan roads, grammar stories, and
  word-building examples from the facade's `evidence.rs` reads.
  `SoundChart` lays the inventory on the catalog's grid, weighed by the
  facade's `soundChart` read. `FamilyTree` ticks each branch with its own
  laws, and `Dictionary` opens with a map of the vocabulary by field.
- `Desk` reads blind correspondences from `desk.rs`. Recorded truth stays
  a separate read.

### Shared view rules

- Every menu is a `Popover`: one open at a time, closed by an outside click
  or Escape.
- Layout follows Android's window size classes only: compact below 600 px,
  medium to 840, expanded beyond, and short below 480 px tall. On compact
  screens, cards and the founding panel sit in a `Sheet` with peek, half,
  and full detents over the map.
- Apparatus (facts boxes, rosters, ledgers) uses the dense scale set by the
  `--dense-*` properties. Prose keeps its reading measure.
- Annals carry the peoples, lands, and laws they tell of, so every entry
  can link to their cards. An optional `cause` names the triggering entry
  where the engine recorded it. The `decision` that produced an annal is
  recorded as the action runs, never inferred from prose.
- The save carries the world's title and author beside its tellings and
  notes.
- Every language view carries a specimen: a few basic words chosen by
  `SPECIMEN` in the facade and shown by `Specimen.tsx`. Sound-change annals
  carry it with each word's form before.

All linguistic logic runs in Rust. Presentation code never mints or changes
words, and there is no backend or second implementation of linguistic
rules. Bun manages frontend dependencies. Rust is pinned in
`rust-toolchain.toml`, and generated WASM bindings are ignored in Git.

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
- Worlds live in the engine worker. The main thread keeps only the
  stateless WASM helpers (catalog, meanings, voice designs, root checks,
  previews) and never blocks on a world.

## Commands

```sh
bun install
bun run dev                 # builds WASM, serves http://127.0.0.1:5173
bun run build               # WASM, TypeScript check, static dist/ (all deployment needs)
bun run brand               # redraws icons and logos from brand/build.js
vale sync --no-global       # installs the pinned prose rules
bun run test:prose           # tests enabled rules and their exceptions
bun run lint:prose           # checks all Git-visible Markdown, failing on warnings
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

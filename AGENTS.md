# langgen

A seeded language-change simulator. Communities with sound preferences grow,
split, meet, and rule one another, and their languages change in response:
regular sound change, borrowing, word competition, and language shift. Rust
engine, browser workbench. Started for Sol's game civilizations; it is now
pursued for its own sake, and names are a later, downstream feature.

The design doc holds the plan, milestone status, and calibration results:
https://claude.ai/code/artifact/965cab5c-8222-4277-8b9d-804394941e3c
Record milestone progress there, not here; this file is for lasting rules.

## Architecture

- `crates/langgen-sim`: the engine, independent of the browser.
- `crates/langgen-web`: `wasm-bindgen` facade. `Bench` holds the logic and
  is tested natively; `Workbench` is a thin wrapper, since `JsValue` panics
  off WASM. It returns presentation-ready JSON views of any generation.
- `web/src/engine.ts`: WASM initialization and the typed adapter.
- `web/src/model.ts`: the TypeScript presentation contract.
- `web/src/App.tsx`: view state, actions, branching, and persistence.
- `web/src/components/`: `Timeline` (scrubber), `Communities`, `Lexicon`,
  `Inspector` (word history and sound system), `ActionDialog`, `Modal`.

React, TypeScript, and Vite provide the interface; all linguistic logic runs
in Rust/WASM. No backend, network language service, component suite, or
second implementation of linguistic rules. Presentation code never mints or
changes words. Bun manages frontend dependencies; Rust is pinned in
`rust-toolchain.toml`. Generated WASM bindings are ignored in Git and rebuilt
before development or production builds.

## Engine model

- `World` (`world.rs`) steps communities, varieties, and contacts through
  25-year generations. `Variety` holds a `SoundProfile` and a `Lexicon` of
  `Slot`s, where words compete for each concept with usage weights. Words
  keep a log of every sound law, borrowing, extension, and loss.
- `SoundProfile`s are numbers; the four packs (`elvish`, `kuo-toa`,
  `illithid`, `neutral`) are only examples, and `neutral` has flat tastes so
  world frequencies decide. A `Flavor` is stored adjustments written from a
  plain brief (`familiar` targets English readers), which the engine never
  reads. One preference score, taste plus cross-linguistic frequency from
  PHOIBLE (`typology.rs`), drives inventories, how often sounds are used,
  sound-law odds, and acceptance of foreign sounds.
- Concepts (`concepts.rs`): the Leipzig–Jakarta 100 in rank order plus
  cultural concepts with a `Tier`. Word length follows `length_bias`
  (Zipf's law of abbreviation: basic meanings short, specialist long, one
  to three syllables). Parent words usually follow the nursery pattern
  (mama, papa). Only `expressive` meanings (small things, insects and
  birds, cries and sounds, baby talk) reduplicate or repeat consonants;
  others avoid it. Roots are unique within a semantic field, with a weak
  sound-symbolic bias. Minting skips spellings that read as English
  vulgarities, a courtesy rather than a linguistic claim.
- Word families (`FAMILIES`, `morphology.rs`): each language mints its own
  affixes, or vowel patterns over consonant roots for root-and-pattern
  (`MorphologyKind::RootPattern`), and builds some family members from
  their bases (`Origin::Derived`). Junctions get a link vowel or a glide as
  the language needs. Root-and-pattern derivations use each word's true
  root skeleton, never a surface reading that includes a pattern's prefix.
  New words come from curated semantic shifts (`RELATED`) or fresh roots.
- Sound laws (`laws.rs`) apply simultaneously and regularly to every living
  word, never to obsolete ones, and never delete a word's last vowel.
  "No change" competes with them, so a culture is never forced into a law.
- Borrowability is set per concept (Leipzig–Jakarta rank or `Tier`), never
  per semantic field, so field patterns must emerge. `wold.rs` holds WOLD
  figures for validation only; the model never reads them. Loans adapt to
  the recipient's established sounds and then undergo only later laws.
- Varieties fork on splits and shifts and keep their lineage (`Fork`);
  `World::cognate` and `root_of` give true descent. The comparative method
  (`compare.rs`) must never read lineage; it is only graded against it.
- Communities grow within their founders' territory, split when large, and
  take prestige from authored `power` plus relative size. A community shifts
  language only to another family's, keeping its own sound preferences and
  some old words as a substrate. Unspoken varieties are extinct and frozen.
- Social identity, territory, ancestry, and language are independent.
- Every random draw comes from a ChaCha8 stream keyed by purpose
  (`rng.rs`), so adding a process never shifts existing draws.

## History and saving

A history is a seed plus ordered `Action`s (`chronicle.rs`): found, connect,
split, shift, run. Consecutive runs merge, so playing stays one action and
undo removes the whole stretch. Any past generation is recovered by
replaying, with checkpoints every 10 generations; the timeline needs no
separate data. Play and "next event" work only at the present.
Acting while viewing the past discards the later history after confirmation.

Saves are recipes (`Recipe`: format, `ENGINE_REVISION`, seed, actions), not
resolved states. Bump `ENGINE_REVISION` whenever a change would make an
existing recipe replay differently; loading a recipe from another revision
still works but the UI warns that its words may differ. Resolved-state saves
remain possible later work if exact preservation across versions matters.

The browser autosaves under localStorage key `langgen.sim.v1`. The previous
workbench's `langgen.workbench.v2` data is left untouched, never deleted.
Unreadable saves open recovery without being overwritten; save failures stay
visible and export still works. Storage is local to the browser, not synced.

## Commands

```sh
bun install
bun run dev                 # builds WASM, serves http://127.0.0.1:5173
bun run build               # WASM, TypeScript check, production dist/
bun run preview             # serve the production build
bun run check               # TypeScript only; requires generated bindings
cargo test --workspace
cargo fmt --all
cargo run --release -p langgen-sim --example found -- <seed> <profile>
cargo run --release -p langgen-sim --example drift -- <seed> <profile> <generations> [flavor...]
cargo run --release -p langgen-sim --example contact -- <seed> <donor> <recipient> <kind> <generations> <seeds>
cargo run --release -p langgen-sim --example family -- <seed> <proto> <outsider> <generations>
cargo run --release -p langgen-sim --example history -- <seed> <generations>
cargo run --release -p langgen-sim --example calibrate -- <seeds> <generations>
```

The pinned toolchain includes `wasm32-unknown-unknown`; wasm-pack is a local
development dependency. Deployment needs only `dist/`, not a server.

## Testing

- Tests and examples that study one mechanism use
  `Params::static_society()`, so growth, splits, and shifts cannot interfere.
- Statistical tests check bands over many seeds, not exact values. Tune
  `Params` against the calibration examples, then confirm the tests still
  hold; the simulator crate is optimized even in dev builds for this.

## Not yet modelled

Places and migration (territories stand in for a map), compounding and
inflection, derivation as a source of new words after founding, stress,
syntax and alignment, dialect levelling, and names.

## Working with Sol

Sol is learning through the design, wants architectural choices explained,
and permits clean redesign rather than protecting incidental implementation.
The framing is loosely Toynbee rather than Spengler: inherent sound and
worldview biases plus challenge and response, not fixed civilizational life
cycles.

## Advisor

"Astra" is `modelRoles.advisor` (`openai-codex/gpt-6-astra:high`). Spawn
agent name `advisor`, model `@advisor`, user agent
`~/.omp/agent/agents/advisor.md`. Advise only unless Sol asks for edits.

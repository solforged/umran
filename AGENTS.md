# langgen

Seeded conlang engine for Sol's game civilizations. Rust engine, browser
workbench. The TUI and CLI have been removed; do not maintain compatibility
unless Sol explicitly asks for it.

## Goal

Build languages, not name lists. Speech communities use language varieties;
neither identity determines the other. A variety has phonology, a lexicon,
sense-licensed grammar constructions, and authored sound, semantic, and
morphological history. Aesthetics are sound preferences, not races or places.

Reproducible: seed and aesthetic determine the initial language; history
also depends on ordered authored events. Save resolved states, not just a
recipe that might produce different words after an engine update.

## Architecture

- `crates/langgen-core`: Rust engine, independent of browser/UI types.
- `crates/langgen-web`: `wasm-bindgen` Workbench facade and presentation DTOs.
  It exposes snapshots, formation options, preview, commit, save, and load.
- `web/src/engine.ts`: asynchronous WASM initialization and a typed adapter.
- `web/src/model.ts`: the TypeScript presentation contract.
- `web/src/App.tsx`: communities, varieties, senses, events, and persistence.
- `web/src/components/Inspector.tsx`: senses, grammar, sound, and provenance.
- `web/src/components/Modal.tsx`: controlled native dialogs.

## Simulator rebuild

`crates/langgen-sim` is replacing `langgen-core`: a generation-stepped
simulator where communities, their sound preferences, and their contacts
drive sound change, borrowing, and word competition. The design doc holds
the plan, milestone status, and calibration results:
https://claude.ai/code/artifact/965cab5c-8222-4277-8b9d-804394941e3c
Old crates and the workbench stay until the UI moves to the new engine; do
not add features to `langgen-core`.

- `World` (`world.rs`) steps communities, varieties, and contacts through
  25-year generations. `Variety` holds a `SoundProfile` and a `Lexicon` of
  `Slot`s, where words compete for each concept with usage weights.
- `SoundProfile`s are numbers; the four packs are only examples. A `Flavor`
  is stored adjustments written from a plain brief, which the engine never
  reads. One preference score drives inventories, sound-law odds, and
  acceptance of foreign sounds.
- Sound laws (`laws.rs`) apply simultaneously and regularly to every living
  word, never to obsolete ones, and never delete a word's last vowel.
- Borrowability is set per concept (Leipzig–Jakarta rank or cultural
  `Tier`), never per semantic field, so field patterns must emerge.
  `wold.rs` holds WOLD figures for validation only; the model never reads
  them.
- Every random draw comes from a ChaCha8 stream keyed by purpose
  (`rng.rs`), so adding a process never shifts existing draws.
- Varieties fork on splits and shifts and keep their lineage (`Fork`);
  `World::cognate` and `root_of` give true descent. The comparative method
  (`compare.rs`) must never read lineage; it is only graded against it.
- Communities grow within their founders' territory, split when large,
  and take prestige from authored `power` plus relative size. A community
  shifts language only to another family's, and keeps its own sound
  preferences as a substrate. Unspoken varieties are extinct and frozen.
- Tests and examples that study one mechanism use
  `Params::static_society()`, so growth, splits, and shifts cannot
  interfere.
- Tune `Params` against the calibration tools rather than the tests; the
  statistical tests check bands, not exact values.

```sh
cargo run --release -p langgen-sim --example found -- <seed> <profile>
cargo run --release -p langgen-sim --example drift -- <seed> <profile> <generations> [flavor...]
cargo run --release -p langgen-sim --example contact -- <seed> <donor> <recipient> <kind> <generations> <seeds>
cargo run --release -p langgen-sim --example family -- <seed> <proto> <outsider> <generations>
cargo run --release -p langgen-sim --example history -- <seed> <generations>
cargo run --release -p langgen-sim --example calibrate -- <seeds> <generations>
```

React, TypeScript, and Vite provide the interface; all linguistic generation
and history mutation run in Rust/WASM. No backend, network language service,
component suite, or second implementation of linguistic rules. Bun manages the
locked frontend dependencies and scripts; Rust is pinned in
`rust-toolchain.toml`. Generated WASM bindings are ignored in Git and rebuilt
before development or production builds.

## Product loop

Choose a community and one of its language varieties, then inspect a word
and a particular sense. Formation cards come from that variety's grammar;
existing formations open their attested entry rather than being coined again.
Filtering does not change the selected word. Following a relative clears
filters; archived words remain inspectable. Historical source links open
the recorded checkpoint, not today's source word. Rows support arrows and j/k.

The header holds a compact checkpoint/history control. Past checkpoints are
read-only; return to latest before authoring. Found creates a community with
authored group, location, and ancestry; it can share an existing variety or
found a new one. UseLanguage adds home, trade, or ritual use. Separate creates
a daughter variety and adds its use without replacing other languages.
Develop and Contact target varieties, not community identities. Development
chooses applicable sound laws with seeded aesthetic bias; contact nativizes
general or maritime loans, with an additional loan budget per event.
Every event previews before acceptance. Escape returns an event preview to
its composer, then closes the composer on a second press.

Derive takes a base, sense, and construction ID. ExtendSense adds a meaning;
ShiftSense changes one while preserving already-derived meanings. Replace
retires a word and transfers its senses to a compatible active word.
Lexicalize removes current analysis, not origin or meaning. Remodel changes
form or analysis by analogy while preserving the word's meanings and origin.

New workshop starts an empty history or a coastal example with shared and
multilingual use, descent, development, and contact. Community and variety
counts are unrestricted by the UI.

Accepted work stays under localStorage key `langgen.workbench.v2` so existing
data is discovered and rejected explicitly rather than silently disappearing;
the history format itself is version 3, with no migration.
Import validates JSON before replacing work; export downloads
`langgen-history-{seed}.json`. Invalid saved data opens recovery without
silently overwriting it. Save failures remain visible and export still works.
Storage is local to the browser and origin, not cloud sync.

## Engine model

`History` in `history.rs` owns communities, varieties, ordered events, and
full immutable checkpoints. `Community.uses` is many-to-many: one community
can use several varieties, and several communities can share one variety.

`linguistics.rs` separates `Lexeme.senses`, current `Analysis`, and immutable
`Origin`. Origins reference a variety, lexeme, and historical checkpoint;
formed origins also record the source sense and construction. Meanings are
copied expressions, never live references into a base's changing semantics.
Traces include sound change, borrowing, inheritance, semantic changes,
replacement, lexicalization, and analogy. History-facing traces use the
variety's orthography consistently.

`Grammar` holds language-local classes and seeded construction IDs, not a
universal suffix menu. Eligibility uses class and sense frames: participants
need the named event role; collectives need a countable entity sense.
Founding preserves the original concept coverage, adds fishing, dying, and
wood, and seeds a few shared-sense patterns and licensed formations.
`DERIVE` and lexical reuse of aesthetic name endings are gone. There is no
global anti-homophone or word-distance penalty.

Bound prefixes/suffixes are phoneme sequences with contextual allomorphs,
not independently evolving words. Compose full words, preserve nested
boundaries and vowel length, and assign grammatical stress. Sound changes
track segment ancestry to retain boundaries and surviving stress. Bound
forms evolve in host contexts and use attested witnesses; retired words
neither evolve nor supply productive witnesses. Never recompose an attested
derivative from today's base and marker.

History JSON version 3 stores resolved grammar, senses, words, inventories,
rules, and traces at every checkpoint. Phonemes serialize as IPA symbols,
not catalog offsets. Load validates graph, chronology, semantic references,
stress, and boundaries without replaying generation. Older versions are
rejected, not migrated. Browser seeds and numeric IDs are unsigned 32-bit.

Packs in `aesthetic.rs`: `elvish`, `kuo-toa`, `illithid`, `neutral`. Each pack
is data: inventory priors, cluster policy, word shape, orthography, name
endings, and signatures. `Language::new` samples an inventory once;
`mint_roots` independently samples short stems for the original 36 glosses.
Language-specific relationships come from senses and licensed constructions,
not a universal derivation graph. Older Session/Family library experiments
are not browser screens or alternate history persistence formats.

## Commands

```sh
bun install
bun run dev                 # builds WASM, serves http://127.0.0.1:5173
bun run build               # WASM, TypeScript check, production dist/
bun run preview             # serve the production build
bun run check               # TypeScript only; requires generated bindings
cargo test --workspace
cargo fmt --all
```

The pinned Rust toolchain includes `wasm32-unknown-unknown`; wasm-pack is a
local development dependency. This is a static client build; deployment
needs only the contents of `dist/`, not a Rust server.

## Invariants

- Proto stems are one-syllable CV/CVC grunts, not 2–4 syllable name output.
- `apply_changes` never deletes a word's last remaining vowel.
- Preview is pure; accepted checkpoints remain unchanged by later events.
- Separate inherits a language rather than minting another inventory.
- Contact is distinct from descent; loans retain their source and history.
- Existing derivatives are attested words, not recomposed previews.
- Meaning, current analysis, and historical origin are distinct.
- Sense drift never silently rewrites an existing derivative's meaning.
- Social identity, location, ancestry, and language use are independent.
- Morphology is realized in whole-word context, including bound markers.
- Historical conditions bias outcomes; trade does not mandate a sound law.
- Do not invent Early/Middle/Late dates. Checkpoints are authored states.
- Engine rules stay in Rust. Presentation code must not mint words.

Human sound-law labels live in `rule_label` / `rule_detail` in
`change_packs.rs`. Inventory currently retains unused phones and adds novel
outputs. Classes currently distinguish entities and events, with count/mass
senses and agent, experiencer, and patient roles. Morphology is concatenative;
contextual witnesses are not a full phonological cycle or complete grammar.
Stress-conditioned laws, inflection, automatic semantic change, and a full
social simulation are not implemented. The concept stock and sound laws
remain deliberately small.

## Design history

Generator dumps, independently sampled names, and long proto words were
rejected. The session TUI made individual sound changes walkable; the family
pane added comparison/contact; History then added authorable communities and
immutable checkpoints. The browser replaces those screens with a
community-and-word workbench, keeping chronology available but compact.

Sol is learning through the design, wants architectural choices explained,
and permits clean redesign rather than protecting incidental implementation.
Names should become downstream cultural constructions, not the primary UI.
The lexical-model cutover adds authored semantic and morphological history,
not an automatic simulation of language evolution. Custom aesthetics, richer
grammar, and emergent cultural dynamics remain separate work.

## Advisor

"Astra" is `modelRoles.advisor` (`openai-codex/gpt-6-astra:high`). Spawn
agent name `advisor`, model `@advisor`, user agent
`~/.omp/agent/agents/advisor.md`. Advise only unless Sol asks for edits.

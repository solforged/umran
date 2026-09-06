# langgen

Seeded conlang engine for Sol's game civilizations. Rust engine, browser
workbench. The TUI and CLI have been removed; do not maintain compatibility
unless Sol explicitly asks for it.

## Goal

Build languages, not name lists. A community gets a phonology, a small
lexicon, productive word formations, and an authored history of sound change
and contact. Aesthetics are vibes (elvish, kuo-toa, illithid, plus originals),
not Tolkien clones or hardcoded name lists.

Reproducible: seed and aesthetic determine the initial language; history
also depends on ordered authored events. Save resolved states, not just a
recipe that might produce different words after an engine update.

## Architecture

- `crates/langgen-core`: Rust engine, independent of browser/UI types.
- `crates/langgen-web`: `wasm-bindgen` Workbench facade and presentation DTOs.
  It exposes snapshots, formation options, preview, commit, save, and load.
- `web/src/engine.ts`: asynchronous WASM initialization and a typed adapter.
- `web/src/model.ts`: the TypeScript presentation contract.
- `web/src/App.tsx`: community selection, lexicon, event authoring, persistence.
- `web/src/components/Inspector.tsx`: relatives, formations, sound, provenance.
- `web/src/components/Modal.tsx`: controlled native dialogs.

React, TypeScript, and Vite provide the interface; all linguistic generation
and history mutation run in Rust/WASM. No backend, network language service,
component suite, or second implementation of linguistic rules. Bun manages the
locked frontend dependencies and scripts; Rust is pinned in
`rust-toolchain.toml`. Generated WASM bindings are ignored in Git and rebuilt
before development or production builds.

## Product loop

Choose a community, search/select a word, inspect its relatives and possible
formations, preview one, then accept it. Acceptance focuses the new word;
existing formations open their attested entry instead of being coined again.
Search and origin filters keep the inspector on a visible word; following a
relative clears filters that could hide it. The lexicon also supports arrow
keys and j/k when its rows have keyboard focus.

The header holds a compact checkpoint/history control. Past checkpoints are
read-only; return to latest before authoring. Add event provides Found,
Separate, Develop, and Contact. Found creates an independent community;
Separate inherits current words and suffixes. Development chooses applicable
sound laws with seeded aesthetic bias. Contact prefers general or maritime
vocabulary and nativizes loans; its count is an additional loan budget.
Every event previews before acceptance. Escape returns an event preview to
its composer, then closes the composer on a second press.

New workshop starts a coastal example or an empty history. The coastal
example independently founds Inland and Coast, separates Colony from Inland,
develops Inland, loans coastal words into Colony, then develops Colony.
There is no fixed community count.

Accepted work is saved to localStorage under `langgen.workbench.v2`.
Import validates JSON before replacing work; export downloads
`langgen-history-{seed}.json`. Invalid saved data opens recovery without
silently overwriting it. Save failures remain visible and export still works.
Storage is local to the browser and origin, not cloud sync.

## Engine model

`History` in `history.rs` owns speech communities, ordered events, and full
immutable checkpoints. Lexemes have local IDs, ultimate source identities,
local derivational bases, and traces for founding, inheritance, borrowing,
derivation, and sound change. Previews do not mutate accepted states.

Founding gives each community three seeded productive suffixes: agent,
place, and collective. These yield generic compositional meanings, not a
complete grammar or an automatic semantic simulation. Founding relatives
reuse the existing lexical derivation table; productive formations use the
current base and current suffix. Already-coined words evolve independently:
never reconstruct an existing derivative from today's base and suffix.
Suffixes participate in sound development and are inherited at separation.
Loans retain source identity but do not invent a local morphological analysis.

History JSON version 2 stores the seed, full aesthetic data, all checkpoint
states, resolved rules, suffixes, and traces. Phonemes serialize as IPA
symbols rather than catalog offsets. Load validates and restores without
replaying generation; older history versions are rejected, not migrated.
The browser contract uses unsigned 32-bit seeds and IDs.

Packs in `aesthetic.rs`: `elvish`, `kuo-toa`, `illithid`, `neutral`. Each pack
is data: inventory priors, cluster policy, word shape, orthography, name
endings, and signatures. `Language::new` samples an inventory once;
`mint_roots` mints 36 glosses from short CV/CVC stems. Related glosses derive
from those stems, not independent name samples. Older Session/Family library
experiments are not browser screens or alternate persistence formats.

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
- Historical conditions bias outcomes; trade does not mandate a sound law.
- Do not invent Early/Middle/Late dates. Checkpoints are authored states.
- Engine rules stay in Rust. Presentation code must not mint words.

Human sound-law labels live in `rule_label` / `rule_detail` in
`change_packs.rs`. Inventory currently retains unused phones and adds novel
outputs; phonemic mergers, stress evolution, inflection, and rich senses are
not implemented. The 36-gloss starting lexicon and seven sound laws remain
limited; this is not a complete linguistic or social simulation.

## Design history

Generator dumps, independently sampled names, and long proto words were
rejected. The session TUI made individual sound changes walkable; the family
pane added comparison/contact; History then added authorable communities and
immutable checkpoints. The browser replaces those screens with a
community-and-word workbench, keeping chronology available but compact.

Sol is learning through the design, wants architectural choices explained,
and permits clean redesign rather than protecting incidental implementation.
Names should become downstream cultural constructions, not the primary UI.
Custom aesthetics, richer semantics, coherent phonological systems, and more
historical events are later work, not implicit additions to a UI task.

## Advisor

"Astra" is `modelRoles.advisor` (`openai-codex/gpt-6-astra:high`). Spawn
agent name `advisor`, model `@advisor`, user agent
`~/.omp/agent/agents/advisor.md`. Advise only unless Sol asks for edits.

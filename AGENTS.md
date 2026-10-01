# langgen

A seeded language-change simulator. Communities with sound preferences grow,
split, meet, and rule one another, and their languages change in response:
regular sound change, borrowing, word competition, and language shift. Rust
engine, browser workbench.

Read further when the task touches it:

- `docs/engine.md`: the engine model, module by module, the mechanism
  examples, and what is not yet modelled.
- `docs/history.md`: actions, replay, branching, recipes, and autosave.
- Design doc (plan, milestone status, calibration results):
  https://claude.ai/code/artifact/965cab5c-8222-4277-8b9d-804394941e3c.
  Record milestone progress there, not here.

## Architecture

- `crates/langgen-sim`: the engine, independent of the browser.
- `crates/langgen-web`: `wasm-bindgen` facade. `Bench` holds the logic and
  is tested natively; `Workbench` is a thin wrapper, since `JsValue` panics
  off WASM.
- `web/src`: React, TypeScript, and Vite. `model.ts` mirrors the facade's
  JSON views; `engine.ts` adapts the WASM; `App.tsx` holds view state,
  actions, and persistence; `components/` holds the panes and dialogs.

All linguistic logic runs in Rust. Presentation code never mints or changes
words, and there is no backend or second implementation of linguistic
rules. Bun manages frontend dependencies; Rust is pinned in
`rust-toolchain.toml`; generated WASM bindings are ignored in Git.

## Rules that must hold

- Every random draw comes from a ChaCha8 stream keyed by purpose
  (`rng.rs`); a new process gets its own stream or draws after existing
  ones, so it never shifts earlier draws.
- Sound laws apply regularly to every living word and name, never to
  obsolete words, and never delete a word's last vowel.
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

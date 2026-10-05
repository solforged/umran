---
name: astra-contract-lanes
description: "Use when running Astra/Sol subagents on Umran engine logic against written contracts (main session keeps design/UI)."
---

# Astra contract lanes (Umran)

Astra (`astra-engineer` agent, model `@advisor`) and Sol (default `task` agents) are good at logic, tests, verification, and computer use, but poor at design. They implement against contracts. The main session owns names users read, layout, and the data shapes the UI consumes.

## Setup
- Check `~/.omp/agent/agents/astra-engineer.md` exists: full tools; prompt says follow the contract, touch only allowed files, no `web/src`, user-facing text is placeholder, report with pasted proof. The `advisor` agent stays read-only (it cannot write `local://`; ask for its report as the final answer).
- `xd://intercom` `list` to find other umran sessions; ask which engine areas they own and send a quiet note naming the files each lane touches.
- Map the slice first with read-only `scout` tasks: types, file:line refs, the view JSON in `crates/umran-web/src/lib.rs` and `web/src/model.ts`, annal shapes in `annals.rs`, RNG keys, tick order in `world.rs`.

## Brief (`local://brief-<slice>.md`)
1. Real-world linguistic or historical models the mechanism should reproduce.
2. Engine numbered requirements: new module where possible, small hooks in `world.rs`/`lib.rs`/`annals.rs`. Draws on new purpose-keyed streams only; catalog entries appended; `Params::static_society()` keeps the feature off.
3. Floats: arithmetic and `sqrt` only, or `crate::math` (libm). Std `exp`/`ln`/`powf` differ by an ULP between native and WASM.
4. "Do not bump ENGINE_REVISION; the integrator does at merge."
5. Facade contract as exact TS field shapes with enum ids. No label strings.
6. "Not yours": always `web/src`, plus other lanes' areas.
7. Acceptance: fmt, clippy, tests once at the end; tests for plausible bugs; a band over 40 seeds × 4,000 years with stated numbers, including the sample recipe (`web/src/sample.ts`); an `examples/<slice>.rs` report with pasted excerpt; `docs/engine.md` update.

## Launch
- `task` with `agent: "astra-engineer"`, `isolated: true`, one lane per slice ("Implement the brief at local://brief-x.md"). Default pace: at most 3–4 engine lanes at once (Sol asked to be gentle with quota).
- Generic `task` (Sol) for no-UI-cost lanes: native-vs-WASM parity harness (two-tier digest, discrete and float), invariant property tests in `crates/umran-sim/tests/`, test-oracle adaptation.
- Astra computer-use audit for UI defects: detached worktree, own port (not 5173 or 5180), fresh Chrome profile, screenshots outside the worktree; reports only, no redesigns.

## While lanes run
- Answer lane IRC quickly; approve sensible scoping; steer when a lane reports a missing hook (e.g. the conquest hazard in `make_contacts`).
- Keep `local://integration-notes.md`: oracle notes, bugs found, factors chosen.
- Fix small out-of-scope bugs lanes report (e.g. an examples panic) yourself.

## Merge, one lane at a time
1. If not auto-applied: `git apply --3way <session-dir>/<Agent>.patch`; resolve via `conflict://N` (new catalog laws append after existing ones; doc example lists keep both). For shared-checkout details see `umran-merge-parked-lane-patches`.
2. Review the diff; fix cross-lane compile errors (new struct fields in literals); adapt `tests/invariants.rs`.
3. Rewrite placeholder annal wording in the annalist's voice (`annals.rs`).
4. Bump `ENGINE_REVISION` in `chronicle.rs` by one per merge.
5. `cargo fmt --all`, `cargo clippy --workspace --all-targets --release`, `cargo test --workspace --release`, the lane's example and band test, `bun run parity` ("Parity passed").
6. Commit the engine change (Conventional Commits, body wrapped at 72; the hook rejects longer lines), then build the UI yourself (`model.ts`, `lore.ts` EVENT_KIND and TERMS, Pedia cards, MapView) and smoke it on a free vite port after `bun run build:wasm`.

## Perf patches
Prove outcome-identical: `git worktree add --detach /tmp/umran-base HEAD`, run the `audit`/`history`/`cities` examples in both trees with separate `CARGO_TARGET_DIR`s, `cmp` outputs, then parity.

## Useful recipes
- Dense wide audit world: `/tmp/umran-audit-shots/world-3-recipe.json` (shelf file input).
- Family tree is `DescentChart` with `Lineage[]` in `FamilyTree.tsx`; reuse for any lineage.

## Emergency burn
Only when Sol explicitly invokes it before a quota reset: follow the global `emergency-quota-burn` skill, using this skill's brief and merge steps for each lane.

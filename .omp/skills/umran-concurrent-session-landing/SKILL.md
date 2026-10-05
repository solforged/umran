---
name: umran-concurrent-session-landing
description: "Use for landing Umran feature branches from isolated worktrees onto a main that other omp sessions keep moving, with a dirty shared checkout."
---

# Landing an Umran branch beside concurrent omp sessions

Use when your work lives in `/tmp/umran-*` worktrees (see `umran-ui-lanes-worktrees`) and `xd://intercom list` shows other sessions in `~/src/umran`, often with a dirty checkout.

## Before building
1. Check for duplicates: `git log --oneline -15 main` and docs/engine.md. If another session built the feature, port onto its API rather than landing a rival.
2. Branch an integration worktree off main (`git worktree add -b <x> /tmp/umran-<x> main`), per-lane worktrees off that, each with its own `CARGO_TARGET_DIR=/tmp/umran-<x>-target`. Symlink `node_modules` (or `bun install` if it needs WASM), copy `web/src/wasm`. Tell lanes never to edit, stage, stash, or commit in `/Users/admin/src/umran`.
3. Agree over intercom (`send`, `delivery: "aside"`) which session owns which functions/files and what is in flight; pass lanes the other lanes' /tmp status notes. Expect main to move 5–10 commits during a 2–3 h lane run.

## Integrating
4. WIP lane commits may skip hooks/signing since they get squashed: `git -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -m "wip: <lane>"`. Cherry-pick onto the integration branch, `bun run check && bun run test:web`.
5. Rebase onto current main early. If a peer already took your `GeographyVersion`/feature name, rename yours (V4 → V5); share capabilities via a predicate method (`has_lakes()`), never duplicate condition lists. Keep every landed feature when resolving and migrate new callers (e.g. `World::connect` returns a Result; callers handle refusals).
6. Heavy conflicts in `MapView.tsx`/`cartography.ts`: hand the rebase to an `astra-engineer` lane in a fresh worktree with "both sides' behaviour survives, docs additive"; it surfaces real integration bugs (hover gated on a stale drag latch, rivers ending in lakes).
7. New geography breaks seed-dependent fixtures and the sample world: pin fixtures to the old version with `World::with_geography`; re-author the sample on the new geography.
8. `ENGINE_REVISION` (crates/umran-sim/src/chronicle.rs) is contended: set it one above main's, asking the peer for the next number each time main moves; fold it into the commit body and `tests/shelf.test.ts`.

## Landing
9. Squash with `git reset --soft main`, commit logical Conventional Commits (`-F -` heredoc, subject ≤72, 72-col body). Final commits are SSH-signed through 1Password: on `failed to fill whole buffer` ask Sol to unlock and retry; never disable signing or use `--no-verify` for these. If signing fails from the shell entirely, tell Sol to `git rebase --gpg-sign`.
10. Full suite on the final tip only: `bun run build:wasm`, `bun run check`, `bun run test:web`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --release -- -D warnings`, `cargo test --workspace --release` (~7 min, async), `bun run parity`. For statistical/engine changes also a 30-seed probe and native step timing vs main. Browser smoke: drag timing, gazetteer hover/pin/Escape, sample world.
11. Ask peers to commit their hunks first and pause `crates/` commits for the window. Then in `~/src/umran`: `git stash push -- <third-party dirty files>`, `git merge --ff-only <branch>` (if main moved, rebase again), `git stash pop`, resolve README/doc conflicts additively via `conflict://N`, keep committed screenshots (`git checkout --ours`), `git reset` so third-party prose stays unstaged, `git stash drop`.
12. Run `bun run build:wasm` in the main tree so the dev server (127.0.0.1:5173) picks up the change.
13. UI handoff: subagents never edit web/src. Collect each lane's JSON contract into one /tmp note for the UI session; flag renamed fields that silently break the UI (`overseas` became `bySea`).
14. Send peers the new hash and next free revision; remove merged worktrees and lane branches (keep unmerged or dirty ones and say so), stop vite services, close tabs. Pushing to origin is Sol's call.

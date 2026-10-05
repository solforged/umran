---
name: umran-merge-parked-lane-patches
description: "Use when landing isolated Umran lane patches (session *.patch files or auto-applied) one at a time onto a shared main checkout with peer sessions' dirty files."
---

# Merging parked Umran lane patches onto shared main

Isolated lanes (`task` with `isolated: true`) end with `~/.local/share/omp/sessions/-src-umran/<session>/<Lane>.patch`, or with `Applied patches: yes` straight into the working tree. Land them one at a time, each verified on its own.

## Before each patch
- `git status --short` and `git diff --cached --stat`. Files you did not touch belong to peers: never stage or revert them. Peer dirty files (e.g. AGENTS.md, README.md, docs/book.md, chart.css) can end up staged after a peer stash/pop; unstage with `git restore --staged <f>`.
- Pre-check queued patches: `git apply --check --cached <patch>`.
- Patch touching a peer's dirty file: apply with `--exclude=<file>`, then that file's hunks separately with `git apply --cached --include=<file> <patch>` plus `git apply --include=<file> <patch>` (index stays clean of peer edits).

## Apply
`git -c rerere.enabled=false apply --3way <patch>`, then `git diff --name-only --diff-filter=U`. Resolve with `conflict://` writes:
- Two lanes extending the same list (pronoun CELLS, concept tables, kind unions): keep both, append at the end so catalog ids stay stable.
- Filters and matches: combine both conditions.
- Count asserts in band tests: include both lanes' additions.
Lane logs often show the lane's base, not the real merge state; rerun checks yourself.

Abandoning a patch: restore its files from HEAD (`git checkout HEAD -- f`, `rm` files it created). Never `git apply -R` its clean files: two lanes can add an identical line and reversing one deletes the other's. Afterwards compare `git show --stat HEAD` with the committed lane's file list.

## Integrator duties per lane
- Rewrite placeholder annal and law-label wording in the house voice: past tense, peoples as "the X" not italicised, only language words in `*thus*`.
- New annal kinds: `model.ts` (Annal kind union), `lore.ts` EVENT_KIND (with a lucide icon), `history.ts` GROUP, `eras.ts` QUIET_KINDS when minor. Kinds `quietLine` doesn't describe fall back to their EVENT_KIND name.
- New `Mechanism`: `MECHANISM_NAME` entry in lore.ts.
- Facade structs serialized to the web need `#[serde(rename_all = "camelCase")]`; check field names match `model.ts` (a `bySea`/`overseas` mismatch once went unnoticed).
- Lanes leave docs/engine.md blank: write the section (mechanism, sources, band numbers) and update "Not yet modelled".
- Bump `ENGINE_REVISION` in `chronicle.rs` with a `/// Revision N ...` line when histories change. Display text and outcome-identical perf work need none; prove perf identical instead (detached worktree, `cargo run --example history -- 7 160` in both trees, `cmp`).
- Behaviour fixes need a regression test that fails without them: `git stash push <src file>`, run the test, `git stash pop`.
- Delete lane tests that pin wording; keep tests of branches and boundaries.

## Verify (script at /tmp/umran-verify.sh; recreate if missing)
```sh
cargo fmt --all && cargo clippy --workspace --all-targets --release -q -- -D warnings
cargo test --workspace --release -q --no-fail-fast   # without it an umran-sim failure hides umran-web; count ok suites
cargo test --release -p umran-sim --test <band> -- --ignored --nocapture
bun run build:wasm && bun run check                  # check reads the generated bindings
bun test && bun run parity > log                     # look for "Parity passed"
```
About 10–13 min; run async with timeout ≥ 1800 s.
- Band out of range (e.g. after a geography change): measure, widen limits, record old and new numbers in the test comment and `docs/engine.md`. Never retune silently.
- New `GeographyVersion::CURRENT`: `scripts/parity.ts` must require it for new worlds and add the previous version to the legacy surveys.

## Commit
- Conventional Commit with the revision in the body, hard-wrapped at 72 (commit-msg hook). Leave peer files unstaged (`git reset -q <file>` after `git add`).
- 1Password signing locks often ("failed to fill whole buffer" or "agent returned an error"): ask Sol to unlock, keep the index, retry the same commit.
- After each commit send the new hash and revision to peer sessions over intercom; agree which revision numbers each side takes, and hold their files while they rebase.

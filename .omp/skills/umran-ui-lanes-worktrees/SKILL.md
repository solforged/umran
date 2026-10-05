---
name: umran-ui-lanes-worktrees
description: "Delegate Umran web/src UI work to Astra/Sol subagents in per-lane git worktrees off a feature branch, prove it in headless Chromium, and cherry-pick onto the branch."
---

# Umran UI lanes in worktrees

Use when the parent works on a branch inside `.claude/worktrees/<x>` and wants Astra (`astra-engineer`) or Sol (`task`) subagents to build frontend pieces in parallel. The parent owns design: names users read, layout, data shapes.

## Why worktrees, not `isolated: true`
`isolated: true` branches from and applies patches to the session's repo root (the main checkout), not the parent's worktree. Patches then land in the wrong tree; recover from `<session-dir>/<Lane>.patch` with `git add -A && git apply --3way` in the right worktree.

## Setup per lane
```sh
cd <branch worktree>
git worktree add -q -b lane/<name> /tmp/umran-<name>-wt <branch>
ln -s $PWD/node_modules /tmp/umran-<name>-wt/node_modules
cp -R web/src/wasm /tmp/umran-<name>-wt/web/src/wasm
```
Shared `node_modules` shares vite's deps cache, which can duplicate React (blank page, `reading 'useState'`). Tell lanes to start vite with a throwaway config outside the worktree (`cacheDir: "/tmp/vite-cache-<lane>"`, `resolve.dedupe: ["react","react-dom"]`) on a free port in 5200-5299, never `vite --force`.

## Brief (`local://brief-<slice>.md`)
Worktree path; exact allow list of files/functions; the contract as TS signatures and class names; CSS tokens to use; what is "not yours"; proof: `bun run check` once at the end, headless Chromium numbers (bounding boxes, counts, computed styles) and screenshots in day and lamplight (`data-theme` on `<html>`), `tab.errors()` clean beyond requestFailed. When two lanes touch one file (e.g. `Stage.tsx`), make both add the identical shared lines so the cherry-pick conflict is trivial.

## Launch
`task` with `tasks[]`, `agent: "astra-engineer"` for logic-heavy lanes, default for the rest, NOT isolated, "Your worktree: /tmp/umran-<name>-wt" in the task text. Answer lane IRC questions fast; approve sensible deviations (e.g. inline `<span role="button">` because `<button>` cannot wrap prose).

## Merge
```sh
cd /tmp/umran-<name>-wt && git add -A && git commit -q -m "wip: <name> lane"
cd <branch worktree> && git cherry-pick lane/<name>
```
Fix duplicates from shared lines, `bun run check`, smoke on the branch's own vite port, then `git reset --soft <base>` and recommit the lanes as one Conventional Commit (body wrapped at 72). Update README/docs/AGENTS.md and `docs/images/stage.jpg` yourself. Remove worktrees: `git worktree remove --force /tmp/umran-<name>-wt && git branch -D lane/<name>`.

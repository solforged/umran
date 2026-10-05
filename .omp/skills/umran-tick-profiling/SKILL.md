---
name: umran-tick-profiling
description: "Use when Umran (formerly langgen) playback is slow: profile per-generation engine step and overview natively and in browser."
---

# Profiling Umran ticks

1. Write a throwaway probe `crates/umran-web/examples/tick_scratch.rs`: `Bench::new(7, "medium")`. Found 3 presets via `umran_sim::LanguageDesign::preset(p, i)` and a JSON `found` action. Run N generations, then loop: time `b.act(r#"{"kind":"run","generations":1}"#)` and `b.overview(gen)` and print the overview size.
2. Time it: `zsh -lc 'cargo run --release -q -p umran-web --example tick_scratch -- 360 3'`. Native timings are the fair before/after comparison; the overview size should stay identical.
3. Sample the hot path: build the probe, run it in the background with many loops, then `sample $PID 8 -file /tmp/x.sample`. Parse the call tree in Python eval: rows are `(indent, count, symbol)`. Find the children of `Bench3act`, `Bench8overview`, `World4step`, and the parents of ChaCha `generate_and_set`. rustfilt is not installed, so read the mangled names.
4. Browser: rebuild WASM with `bun run build:wasm` (Vite hot-reloads it). Set the stop-for select to `nothing` (otherwise play halts on events and the measure is wrong) and the pace select to `16`. Record the year from `.stage-year`, click `.timebar button.play`, wait 8 s in the eval host (not in a page promise; long page promises get collected), then compute ms per generation as dt / (Δyear / 25). Headless worlds keep growing between runs, so compare at a similar size.
5. Delete the probe example dir before committing. Run `cargo clippy`, `cargo test --workspace --release`, and `bun run build`.

Known costs: Adapter::new per call (reads the whole lexicon; build one ear per variety with `World::ear`); borrow() stream per contact×direction×concept (fixing it changes replay, so bump ENGINE_REVISION); crowded worlds (100+ peoples) because splits continue on full land.

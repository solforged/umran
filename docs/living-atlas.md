# Living atlas implementation

Worktree: `codex/living-atlas`. This work carries the full direction in
`backlog.md`: spatial decisions, connected subject histories and alternate
tellings, guided founding, believable geography, and material motion.

## Delivery sequence

1. **Settlement and divergence.** Rust owns exact history points, atomic
   interventions, connected territory partitions, resident allocations and
   journeys. The chart previews all three intents and opens a result linking
   parent, daughter, place and language. A refused or stale choice changes nothing.
2. **Histories and tellings.** Follow subjects across names and years; mark
   discoveries; return to the point before an intervention. Give tellings stable
   identities, resolve their views independently and compare outcomes at the
   same year without joining unrelated IDs.
3. **Guided founding.** Compose homeland, livelihood, temper, speech and identity
   one people at a time, with a map, specimen and an editable founding charter.
4. **Geography and scale.** Coasts, terrain and routes explain contact and
   isolation at regional and continental scales; detail changes with zoom.
5. **Material motion.** Paired, interruptible openings and closings preserve
   focus and reading position, with immediate reduced-motion behavior.

Each delivery needs a rendered clickthrough and appropriate focused checks.
No compatibility layer is required. Do not trade the intended product model
for an easier implementation or an easier test.

## Progress

- The earlier workbench pass is the starting commit, `cdbc093`.
- Settlement design: one planner with partition, settlers and whole migration;
  travel uses the existing physical graph. History transactions must include
  the position within a year, and previews must detect same-year mutations.
- Urban residents and state capitals must be included in territorial changes.
- Delivered the settlement planner and map-side account for all three intents.
  Recorded decisions carry population allocations, actual travel paths, affected
  states, and parent/daughter links. Exact history points distinguish decisions
  made in one year; applying an invalid or stale preview leaves history intact.
- Verified desktop and 390px phone clickthroughs, exact return, alternate choice,
  reload and restore. The full Rust workspace suite passes (244 tests), Clippy
  passes, and native/WASM parity matches the planner and all three choices plus
  saved replay (13 history checks). Production web build and five web checks pass.
- Next: stable named tellings and reading-scoped views. Do not use array positions
  as telling identities, discard equal histories, or join entities born after
  divergence just because their numeric IDs match.

# History and saving

How the workbench records, replays, and stores a world.

A history is a seed plus ordered `Action`s (`chronicle.rs`): found, connect,
split, shift, run. Found records the naming, the full design, and the
language's own seed, the one its preview used, so founding gives exactly
the previewed words and names. A split's naming is optional; without one
the new people chooses. Consecutive runs merge, so playing stays one action and
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

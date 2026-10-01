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

The browser keeps a shelf of books (`web/src/shelf.ts`): each book's
history autosaves under localStorage key `langgen.book.<id>`, and
`langgen.shelf.v1` lists the books with their titles and the one last open.
The single-world save `langgen.sim.v1` is copied onto the shelf once as "An
earlier history" and left in place, as is the older `langgen.workbench.v2`.
Books leave the shelf only when the reader removes one and confirms.
Unreadable books or an unreadable shelf open recovery without being
overwritten; save failures stay visible and export still works. Storage is
local to the browser, not synced. The sample chronicle (`web/src/sample.ts`)
is a fixed recipe; opening it puts a fresh copy on the shelf.

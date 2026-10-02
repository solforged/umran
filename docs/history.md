# History and saving

How the workbench records, replays, and stores a world.

A history is a seed, a map size, and ordered `Action`s (`chronicle.rs`):
found, connect, settle, shift, state, religion, craft, temper, run. The
seed draws the map as well as the history. Found records the naming, the
full design, the language's own seed, the one its preview used, so
founding gives exactly the previewed words and names, and optionally the
land the people settles (without one the world chooses) and any leanings
of its temper (the rest are drawn). A settlement records its intent,
destination, departing share, optional daughter naming, and contact intensity.
Partition divides held territory around two hearts; settlers draw a share from
every inhabited land; migration moves the whole community without replacing
its language. Without a naming choice the new people chooses. State makes a people a
state, with its court at one of its lands (its heart if none is given); a
rule contact makes the more prestigious side rule the other's state.
Religion raises a founder among a people, and a faith with him; craft has
a people come upon a craft it lacks; temper nudges one leaning of a
people's temper, which then drifts and answers events as before.
Consecutive runs merge up to the per-action limit, so playback stays a compact
account. Any past generation is recovered by replaying, with checkpoints every
10 generations; the timeline needs no separate data. Play runs at the present;
advance and "next event" can continue a past reading as a new telling.

An exact `HistoryPoint` names an action position and an offset into a run, so
two choices in the same year remain distinguishable. A run's endpoint uses its
full offset and stays fixed when playback extends that run. `act_at` validates
a rewind and intervention on a candidate history and publishes it only on
success; an invalid choice cannot set aside the current future. The facade
adds a session mutation counter to every preview. Even another same-year
action invalidates a pending preview.

Settlement previews use the Rust census, food budgets and physical travel
graph. A territorial boundary propagates through held lands only, with each
side connected to its own heart. Journeys require routes from every inhabited
source, including residents in external cities. The recorded account keeps
the actual before/after populations, holdings and paths, and identifies a
capital loss before the choice is applied. Its chronicle card can return to
the point immediately before the decision. Community ancestry is independent
of later language shifts.

Nothing written is thrown away. Each `Telling` has a permanent ID, editable
name, complete action log and optional parent `ReadingRef` (a telling ID plus
an exact `HistoryPoint`). The founding account is a telling too. Returning
before an action only moves the reading; it neither edits history nor creates
another account. The first successful action from that earlier reading creates
a child telling, preserving the parent's whole future. Even equal continuations
keep distinct identities. Reading a telling never activates or alters it;
continuing at its tip extends that account, and continuing earlier forks it.

The facade gives each reading its own read-only view of the world. Overview,
lexicon, word history, family and map views all use that telling and exact point.
The browser's world owns a small cache of these views; cards do not own or free
the underlying WASM allocations. Failed reads leave the account listed and its
recipe available for recovery.

Comparison reads both tellings at one year, bounded by the shorter recorded
history. It never silently advances either. Ancestry identifies the last proven
shared reading, normalizing run endpoints before comparing prefixes. Only
entities already present at that reading can be followed across both accounts;
equal numeric IDs born after divergence do not establish identity. The two
charts share a camera, while their populations, holdings, language families and
word forms come from their own views. Following a link opens that telling;
continuing either panel is an explicit intervention.

Saves are recipes (`Recipe`: format, `ENGINE_REVISION`, seed, map size,
active telling ID and all telling records), not resolved states.
The import validates identities, parent references and shared action prefixes.
An unreadable inactive telling stays retained; it does not prevent reading a
valid active account. Bump `ENGINE_REVISION` whenever a change would make an
existing recipe replay differently; loading a recipe from another revision
still works but the UI warns that its words may differ. Resolved-state saves
remain possible later work if exact preservation across versions matters.

The browser keeps a shelf of saved worlds (`web/src/shelf.ts`): each
world's history autosaves under localStorage key `langgen.book.<id>`, and
`langgen.shelf.v1` lists them with their titles and the one last open.
These keys, the look key `langgen.look`, and the recipe format
`langgen-sim-recipe` keep the project's first name and the shelf's first
metaphor. The named-telling schema replaces the former active-log/set-aside-log
shape; incompatible recipes open recovery with their original text intact. The
single-world save `langgen.sim.v1` is copied onto the shelf once as "An
earlier history" and left in place, as is the older `langgen.workbench.v2`.
Worlds leave the shelf only when the reader removes one and confirms.
Unreadable saves or an unreadable shelf open recovery without being
overwritten; save failures stay visible and export still works. Storage is
local to the browser, not synced. The sample world (`web/src/sample.ts`)
is a fixed recipe of four thousand years, offered on the shelf and at the
top of world setup; opening it puts a fresh copy on the shelf.

Founding setup keeps one account open beside the chart, with independent
homeland, livelihood, temper, speech, and identity choices. The optional
charter compares the founders and their possible first journeys. On Begin,
three to five questions are kept by default through the existing notebook
save path, each with a `question` kind and a destination at the exact
year-zero reading. Their people, land, language, and word references open
from the field notebook and travel with the saved world. Unchecking
"Keep these questions in the notebook" skips them; the charter itself is
never required to begin.

The first visit opens the shelf, even when it is empty; setup and the
sample are explicit choices there. Later visits resume the world left
open, including one reopened without making changes. Returning to the
shelf clears that resume choice, so reloading stays on the shelf. The
top-left astrolabe and name lead to the shelf from setup, the stage, and
Export; Export also has a separate link back to the map. Leaving the stage
during playback saves the latest generation before recording the shelf as
the next launch destination.

The production workbench is also an installable offline app. Its service
worker caches the app shell and WebAssembly engine, not worlds; saved
recipes keep the same localStorage keys. Worker cache cleanup never
touches the shelf. Worlds created or advanced offline save normally.
Updates wait for "Save and reload", which saves even an actively playing
world before reloading. A failed save blocks the reload and leaves export
available. Accepting an update in one window does not force other open
windows to reload. Installing does not sync worlds between browsers or
devices; where an installed app has separate storage, move worlds by
exporting and importing recipes.

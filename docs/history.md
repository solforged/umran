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
10 generations; the timeline needs no separate data. Running to a year
applies one `run` action per animation frame at the present; a step or
"until something happens" can continue a past reading as a new telling.

An exact `HistoryPoint` names an action position and an offset into a run, so
two choices in the same year remain distinguishable. A run's endpoint uses its
full offset and stays fixed when playback extends that run. `act_at` validates
a rewind and intervention on a candidate history and publishes it only on
success; an invalid choice cannot set aside the current future. The facade
adds a session mutation counter to every preview. Even another same-year
action invalidates a pending preview.

Every world-event annal names its author's `decision` by action index when
one produced it, and carries `before`, the exact `HistoryPoint` immediately
before that action in its telling. Simulation events carry neither. Grouped
annals take the first member's decision; each member keeps its own attribution.
Settlement evidence remains separate from authorship. The facade's
`decisions()` lists every non-run action through the current reading in index
order, with its generation, kind, sentence, people, resulting language for
founding/settlement/shift, and original annal IDs (including grouped members).
A decision with no emitted events stays in the list with an empty `annals`.

The book and cards mark authorship from that engine record, not by matching
entry text or parsing the save. Each decision has a row before the entries
it produced, including decisions that produced no entries. Event cards can
return to `before` for any decision; settlements reopen their account,
while other decisions return the reading to that exact point.

The world's title and author are metadata in the engine save, independent
of its tellings. Founding sets both; the frontispiece names the author and
the chart room keeps that byline with the title. Exporting and importing a
save preserves both. The browser also remembers the last author name in
`umran.author` as a convenience for the next founding, not as the save's
source of authorship.

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

The workbench shows one overlaid sheet at a time. Opening Find or a
decision lifts the current wide card before the dialog settles, then
restores its section when the dialog closes. Comparison returns to the
chronicle card's "Other tellings" section, or the matching tab on a phone.
Escape dismisses only the top sheet. A wide card's filters and reading
position survive the temporary cover, as they do a visit to a word's
history.

The author's notes are written and edited in the margin of each card,
dated at the year being read and tied to its exact telling, point, and
subject. The world card can also name a year. The chronicle card collects
every note in year order, with links back to those exact cards; the book's
Notes chapter collects their text with links to its subject sections.
Keeping, editing, or removing a note saves it with the world without
changing the simulation or invalidating a settlement preview. Notes from
older saves keep their kind and archived flag and remain visible.

Saves are recipes (`Recipe`: format, `ENGINE_REVISION`, seed, map size,
active telling ID and all telling records), not resolved states.
The import validates identities, parent references and shared action prefixes.
An unreadable inactive telling stays retained; it does not prevent reading a
valid active account. Bump `ENGINE_REVISION` whenever a change would make an
existing recipe replay differently; loading a recipe from another revision
still works but the UI warns that its words may differ. Resolved-state saves
remain possible later work if exact preservation across versions matters.

The save document flattens that recipe alongside the notebook and optional
`title` and `author` strings. Old saves without these fields load with neither
set. `title()` and `author()` read this document metadata; `setTitle` and
`setAuthor` update it, with an empty string clearing the field. Export and
import preserve both without changing actions, simulation state, or revision.

The browser keeps a shelf of saved worlds (`web/src/shelf.ts`): each
world's history autosaves under localStorage key `langgen.book.<id>`, and
`langgen.shelf.v1` lists them with their titles and the one last open.
`langgen.place.<id>` keeps where the reader was in that world: the
telling, exact reading, year, and open card. Reopening the world returns
there; a place that no longer resolves (another engine revision, a
removed telling) is ignored and the world opens at its latest year.
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
is a fixed recipe of four thousand years, offered in the chart room and
at the top of founding; opening it puts a fresh copy on the shelf.

A world is titled by the author at founding, or else named after its
earliest-named continent (`worldName` in `shelf.ts`); a continent's name
is fixed once given, so the title never drifts as a people's name wears.
An authored title is marked `named` on the entry and kept on every save.

Founding keeps one account open beside the chart, with independent
homeland, livelihood, temper, speech, grammar, and identity choices; the others
are one inked line each. The chosen people's reach, from the engine's
founding preview, is drawn on the chart rather than listed.

The first visit opens the chart room, even when it is empty; founding
and the sample are explicit choices there. Later visits resume the world
left open, including one reopened without making changes. Returning to
the chart room clears that resume choice, so reloading stays there. The
world's cartouche on the stage opens a menu with the way back; the brand
mark in founding is the same link. The book is a sheet laid over the
map, so the stage keeps its cards, layers, and pace while it is open;
its last chapter holds the save file and the text export. Leaving the
stage during a run saves the latest generation before recording the
chart room as the next launch destination.

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

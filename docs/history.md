# History and saving

How the workbench records, replays, and stores a world.

A history is a seed, a map size, and ordered `Action`s (`chronicle.rs`):
found, connect, settle, shift, state, religion, craft, law, temper, meaning,
word, run. The seed draws the map as well as the history.

Found records the naming, full design, and language's own seed (the one
its preview used), so founding gives exactly the previewed words and
names. It also records an optional homeland and chosen temper leanings.
The world chooses a land if none is given and draws the remaining leanings.

A settlement records its intent, destination, departing share, optional
daughter naming, and contact intensity. Partition divides held territory
around two hearts. Settlers draw a share from every inhabited land.
Migration moves the whole community without replacing its language.
Without a naming choice the new people chooses its name.

State makes a people a state, with its court at one of its lands (its
heart if none is given). A rule contact makes the more prestigious side
rule the other's state. Religion establishes a faith and its founder
among a people. Craft lets a people discover a craft it lacks. Temper
nudges one leaning of a people's temper, which then drifts and responds
to events as before.

`Meaning` appends a world-local meaning (id, gloss, field, class,
countability, tier, and optional derivation base and relation). Before
founding it receives a root, and later living languages coin words for it.
`Word` records a language index, meaning id, and catalog IPA. It makes
the author's form dominant at that reading while retaining competing forms.
Both are authored decisions with an annal, and follow their telling's replay.

A law records the living language's index and a catalog id
(`{"kind":"law","variety":0,"law":"spirantization"}`), applying it at the
reading year. `lawChoices(point, variety)` previews eligible laws without
altering that reading, ordered by changed living words, then catalog order.
Its `recent` flag reports the usual sixty-generation quiet span, but does
not prevent an author's repeat application. `Authored` records a law the
author wrote in notation (`title`, `source`, the parsed `rules`, an
optional `stress`). Replay applies the stored rules and never reparses
the source. The law then spreads by wave like a catalog law.
`lawPreview(point, variety, law)` applies a catalog or authored law to a
clone of the reading and lists every word and name it changes. The
stateless `checkLaw(title, source)` reads the notation line by line.

Consecutive runs merge up to the per-action limit, so playback stays a compact
account. Any past generation is recovered by replaying, with checkpoints every
10 generations. The timeline needs no separate data. Running to a year
applies one `run` action per animation frame at the present. A step or
"until something happens" can continue a past reading as a new telling.
The past-reading panel **Continue in another telling** shows its last
recorded decision and a year to write through. Confirmation uses `act_at`
at the exact reading point for the first generation, preserving the
original telling atomically, then the workshop runs the new telling to
that year through the same chunked run loop. The new telling and its
unfinished target are saved as soon as the first chunk succeeds.
The panel explains that without a new decision the years repeat as told.
Deciding something in the past also begins a new telling.

`Chronicle::retell(index, replacement)` tells the active telling again as
a child telling, with the decision at `index` replaced, or left out when
there is no replacement. Every later run and decision is applied again in
order. A later decision is left out, and returned with its reason
(`LeftOut`), when the world refuses it or when it names a people or a
language that is not the same one in the retelling. Peoples and languages
go by number, so sameness needs care. Until a run follows the change, the
original is applied beside the retelling and whatever each decision made
is paired exactly; dropping a founding renumbers the later founders. After
a run, a people or language is the same only if it holds the same place
among those born since, arose in the same year, and has the same parents.
A refused replacement changes nothing. The facade's `retell(telling,
mutation, decision, action)` answers with the new telling and the
decisions left out, in the words of the telling it came from. A decision's
card offers it as *Tell it again…*, and the dialog that replaces a
decision opens on the reading before it. A written law opens with its own
rules (`DecisionView.law`), and a founding opens the sound designer on its
recorded design (`DecisionView.founding`). The replacement keeps the
founding's homeland, livelihood, temper, grammar, roots, and affixes.

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

Sound-law annals are synthesized from the language's law ledger rather
than world events. Authored applications therefore keep a separate
language/year/law-to-decision record, rebuilt when the recipe replays.
Both `Law.decision` and the existing yearly sound-change annal read that
record. Several authored laws in one language/year keep one annal, marked
with the latest decision; every relevant decision links to it. Repeating
the same law in that year gives its ledger entries the latest mark too.
If the same law also arose naturally that year, the author's mark wins.

The book and cards mark authorship from that engine record, not by matching
entry text or parsing the save. Each decision has a row before the entries
it produced, including decisions that produced no entries. Event cards can
return to `before` for any decision; settlements reopen their account,
while other decisions return the reading to that exact point.

The world's title and author are save metadata, independent of its
tellings; their document fields and setters are described below. Founding
sets both, and the chart room keeps the byline with the title. The browser
also remembers the last author name in `umran.author` for the next founding,
not as the save's source of authorship.

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
before an action only moves the reading. It neither edits history nor creates
another account. The first successful action from that earlier reading creates
a child telling, preserving the parent's whole future. Even equal continuations
keep distinct identities. Reading a telling never activates or alters it.
Continuing at its tip extends that account, and continuing earlier forks it.

The facade gives each reading its own read-only view of the world. Overview,
lexicon, word history, family and map views all use that telling and exact point.
The browser's world owns a small cache of these views; cards do not own or free
the underlying WASM allocations. Failed reads leave the account listed and its
recipe available for recovery.

The JavaScript workbench and scoped read views reject non-finite, negative,
fractional, or greater-than-`u32::MAX` community, language and river IDs before
conversion; valid integer IDs still have to exist in the requested reading.

Comparison reads both tellings at one year, bounded by the shorter recorded
history. It never silently advances either. Ancestry identifies the last proven
shared reading, normalizing run endpoints before comparing prefixes. Only
entities already present at that reading can be followed across both accounts.
Equal numeric IDs born after divergence do not establish identity. The two
charts share a camera, while their populations, holdings, language families and
word forms come from their own views. Following a link opens that telling.
Continuing either panel is an explicit intervention.
The comparison opens with its divergence year and recorded decision.
Its speech reading lists laws unique to either side, changed specimen
forms, loans by donor, and descendant-language counts for languages
shared before divergence. Each law, word, and donor link opens that
side's evidence at the comparison year. Speech reads wait independently
of the comparison's charts and accounts.
Only shared languages with differing evidence are shown. Equal loan
counts by donor do not make a difference. An empty comparison keeps only
the line “The tellings speak alike so far.”

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
An unreadable inactive telling is retained and does not prevent reading a
valid active account. Bump `ENGINE_REVISION` whenever a change would make an
existing recipe replay differently. Revision 33 changes flat geography to
a sphere, replacing region identities: the engine refuses earlier recipes
and the shelf opens recovery with the original data still downloadable.
An imported older file is refused without changing saved worlds. Its
region IDs are never guessed onto the new map.

A recipe also records its `geography` version (`spherical-v1`, `continental-v2`, `continental-v3`,
`continental-v4`, `continental-v5`, `continental-v6`),
so a change to how the land is drawn adds a version rather than redrawing
saved worlds. New worlds use `GeographyVersion::CURRENT`. V3 fixes the
geographic direction of zonal wind. V4 adds lakes, endorheic drainage, and
river channels. V5 adds unequal continental crust while retaining V4's lake
pipeline. V6 moves continental crust through a short deterministic
plate-drift history before generating the same lakes and rivers. V1–V5
retain their recorded generators. V1–V3 keep their earlier lake-free maps,
drainage, rivers, and climate. Missing geography still means `spherical-v1`,
never the current default. A save from an earlier spherical revision still
replays, with a warning that says what reads differently. A save stamped
with a revision newer than the running engine is refused by name
(`Chronicle::from_recipe`): an older build could drop actions or members it
does not know and then save over them. The save stays as it is and opens
recovery, which asks the reader to update Umran.

A save keeps the stamp of the engine revision that wrote its history. A
world loaded from another revision keeps that stamp while its action logs
stand as loaded, so a new title, a note, or a renamed telling does not
restamp it. Once this engine writes history into it (a run or a decision),
`save` stamps the current revision and adds the old one to the document's
`earlierRevisions`, oldest first. The overview's `savedRevision` is the
stamp as loaded when it differs, for the warning in that session.
`firstRevision` is the earliest revision known to have written the
history, and the book's frontispiece keeps showing it afterwards.

Before an engine of another revision overwrites a stored save, the shelf
keeps that save as it stood under `langgen.kept.<id>.<revision>`
(`saveBook` in `shelf.ts`). The first copy kept for a revision is never
replaced. If it cannot be kept, the save fails visibly and the stored
world is left alone. The warning offers the kept save for download, the
book's last page lists every one, and recovery's *Download all* includes
them. They leave the browser only when their world is removed for good.
A kept save replays exactly only on its own revision, which no deployed
build can yet supply.
Shelf miniatures use the saved geography version for compatible spherical
recipes (revision 33 through the running revision). Earlier revisions and
unrecognized geography are not guessed onto a new map.

A frozen file is a save with the world one reading resolved written beside
the recipe as data (`Bench::frozen`, `frozen.rs`). Its `frozen` member
carries the record's `version`, the engine `revision` that resolved it,
the `telling`, `point`, and `generation` read, and `yearsPerGeneration`.
The world follows under the names the workbench reads it by:
`communities`, `varieties`, `contacts`, `states`, `cities`, `religions`,
`continents`, `crafts`, `places`, `rivers`, `lakes`, `moves`, and
`annals`. Each variety, spoken or silent, also carries its whole
`lexicon`, its `nameQuarry`, and its `phrasebook`. Lands are numbered by
region of the recipe's geography version, which does not change with the
engine. The record's `revision` can be newer than the recipe's stamp,
since a reread save keeps its stamp until history is written into it.

Loading a frozen file replays the recipe and passes over the record, so
it opens as the save it carries and takes no longer to load. A fixed
reading freezes its own point, keeps the whole recipe, and opens on its
telling. The book's last page writes one for the year it reads.

The archive (`web/src/archive.ts`) keeps frozen records in IndexedDB,
database `umran`, store `frozen`, keyed by world id and engine revision
and gzipped where the browser can. The shelf's saves stay in
`localStorage`: they are small, and saving them is synchronous so that a
closing page can still save. Five seconds after the last save, the app
freezes the present telling at its latest year and keeps it as this
revision's record, replacing only this revision's earlier one. It does so
only while the save's stamp is the running revision, so a record is
always an engine's reading of history it wrote itself. A record missed
because the tab closed is taken when the world next opens under the same
engine. A frozen file opened from the chart room brings its record with
it. Removing a world for good removes its records.

`Bench::drift` (`drift.rs`) compares a frozen record with this engine's
replay to the record's telling and point. It reports whether the two read
the same, the year the chronicles part ways, the peoples and languages
with other names, and counts and examples of the names and words that
differ. Peoples and languages are matched by number, and numbers by
value, so a file rewritten by another JSON tool still compares. When a
world opens under another revision, the warning carries that comparison
against the latest record an earlier revision left, and offers the record
for download. The book's last page lists every earlier revision's record.
Opening a record without replaying is later work.

The app asks the browser to keep the site's data
(`navigator.storage.persist`) after the first save of a session. The
chart room's colophon says whether it agreed and offers to ask again.

The save document flattens that recipe alongside the notebook and optional
`title` and `author` strings, `earlierRevisions` when it has any, and
`frozen` in a frozen file. Old
saves without these fields load with neither set. `title()` and `author()` read this document metadata; `setTitle` and
`setAuthor` update it, with an empty string clearing the field. Export and
import preserve both without changing actions, simulation state, or revision.

The browser keeps a shelf of saved worlds (`web/src/shelf.ts`): each
world's history autosaves under localStorage key `langgen.book.<id>`, and
`langgen.shelf.v1` lists them with their titles and the one last open.
`langgen.place.<id>` keeps where the reader was in that world: the
telling, exact reading, year, and open card. Reopening the world returns
there. If a place no longer resolves (another engine revision, a removed
telling), it is ignored and the world opens at its latest year.

These keys, the look key `langgen.look`, and the recipe format
`langgen-sim-recipe` keep the project's first name and the shelf's first
metaphor. The named-telling schema replaces the former active-log/set-aside-log
shape. Incompatible recipes open recovery with their original text intact. The
single-world save `langgen.sim.v1` is copied onto the shelf once as "An
earlier history" and left in place, as is the older `langgen.workbench.v2`.

*Put away* keeps a world's save and reading place off the main shelf.
*Bring back* restores it. Permanent removal requires putting it away first
and confirming.

Unreadable saves or an unreadable shelf open recovery without being
overwritten. Save failures stay visible and export still works. Storage is
local to the browser, not synced. The sample world (`web/src/sample.ts`)
is a fixed recipe of four thousand years, offered in the chart room and
at the top of founding. Opening it puts a fresh copy on the shelf.

A world is titled by the author at founding, or else named after its
earliest-named continent (`worldName` in `shelf.ts`); a continent's name
is fixed once given, so the title never drifts as a people's name wears.
An authored title is marked `named` on the entry and kept on every save.

The shelf also keeps founding drafts under `langgen.draft.<id>`, with their
step and choices for *Resume*. Beginning the chronicle saves the world
before removing its draft. Each world save advances its shelf revision;
a tab with an older base revision cannot overwrite a newer save and must
reload or keep its work as another world. Storage events notify open tabs.
Saving an open world that another tab put away does not bring it back.

Founding keeps one account open beside the chart, with independent
homeland, livelihood, temper, speech, grammar, and identity choices; the others
are one inked line each. The chosen people's reach, from the engine's
founding preview, is drawn on the chart rather than listed.

Chart and Globe are two projections of the same spherical world. The
`umran.projection` browser preference defaults to Chart; it is not part of
the recipe. Turning, zooming, or switching views makes no authored decision
and does not change replay.

The first visit opens the chart room, even when it is empty. Founding
and the sample are explicit choices there. Later visits resume the world
left open, including one reopened without making changes. Returning to
the chart room clears that resume choice, so reloading stays there. Hash
routes (`web/src/route.ts`) name the chart room, a founding draft, a world,
and a book chapter. An explicit route takes precedence over resume.
The world's cartouche on the stage opens the way back. The brand mark in
founding is the same link. The book is a full-window screen with a contents
rail. Its bookmark keeps the chapter, anchor, scroll offset, and open
sections in the reading place, without replacing the workshop's cards,
layers, or camera. Editions read the book's own chapters.
Runs save their unfinished target and latest completed chunks. Leaving
the stage saves the latest generation before recording the chart room as
the next launch destination. A resumed unfinished run offers continuation.

The production workbench is also an installable offline app. Its service
worker caches the app shell and WebAssembly engine, not worlds. Saved
recipes keep the same localStorage keys. Worker cache cleanup never
touches the shelf. Worlds created or advanced offline save normally.

Updates wait for "Save and reload", which saves even an actively playing
world before reloading. A failed save blocks the reload and leaves export
available. Accepting an update in one window does not force other open
windows to reload. Installing does not sync worlds between browsers or
devices. Where an installed app has separate storage, move worlds by
exporting and importing recipes.

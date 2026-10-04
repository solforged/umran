# The book

Product frame, October 2026. This replaces the direction in `backlog.md`;
the pieces that document delivered there (settlement planner, tellings,
comparison, guided founding, motion) survive as workshop tools inside the
shape described here.

## The sentence

**Umran writes the book of a world: its peoples, their fortunes, and the
languages that grew out of both. The simulation is how the book gets
written; the book is the product.**

The person using it is a philologist of a world that does not exist yet,
in the way Tolkien was. They set the founding conditions, let regular
derivation run, and read what came of it as if discovering it. The
discovery is honest because the derivation is: every sound law is
regular, every borrowing has a contact behind it, every migration a
pressure. Nothing is decreed; everything is caused, by land, by accident,
or by a dated decision of the author's.

This is the Toynbee frame the project started with. A people has a land,
and the land and their way of living on it make their temper. History
puts challenges to them (famine, a drying climate, a stronger neighbour,
a new faith) and they respond: they move, they part, they found a state,
they convert, they take up another tongue. Their language is the residue
of all of it. The book's job is to make that chain legible: land, people,
fortunes, speech, in that order.

## The loop

Not a game loop. A drafting loop.

```
found  →  run  →  read  →  revise  →  run again
```

- **Found.** Author the constraints: the coasts, who lives where, how they
  live, what sounds they favour, how they name themselves. This is where
  authorship is thickest, so it is the richest surface.
- **Run.** Let the years run to a year, in one stroke. Watching year by
  year is a study tool, kept, not the default.
- **Read.** The chart, the chronicle, and the cards: the workshop. Every
  name links to its card; every card shows its causes.
- **Revise.** A dated decision (a people parts, a faith is founded, a
  craft is taught) opens another telling from that year. Tellings are
  drafts of the same world. Compare them at the same year.
- **Run again.** Until a telling is one you want to keep. Then read it
  as a book.

Authorship stays at the level of constraints and dated decisions. The
author never edits a derived form; if *fees* is wrong, the law is wrong,
and the law is what changes. The engine's regularity is the point.

## The book's table of contents

What a finished world reads as. Each chapter is drawn from a facade view
that exists today (`web/src/model.ts`), so the book page is a reading of
the engine, not a second source. Blank where the engine has nothing yet.

1. **Frontispiece.** The world's name (the landmass the engine names,
   `Hūhupam`, never a people, so it does not drift when a people's name
   erodes), the author, the seed and engine revision, the chart, and the
   year the book is read at. Which telling, if there are several.
2. **The lands.** The chart as drawn, its coasts, terrain, rivers, lakes,
   and climate; every named land with the chronology of its names and who
   calls it what (`Place`, `PlaceName`, `climate`, `river`, `lake`).
3. **The peoples.** One section per people, living or ended, ordered by
   founding. Each opens the way the engine derives it: *where they live*
   and what the land is like; *how they live* (livelihood); *what they
   are like* (temper, with the causes that turned it); *their fortunes*
   (the chronicle filtered to them, challenge beside response); *what
   they speak*, with the specimen. Then kin, dealings, faith, crafts,
   names others call them (`Community`, `story`).
4. **The languages.** The centre of the book and the reason for it. One
   chapter per language, living or silent, grouped by family:
   - *Position*: parent, daughters, the family tree, kin by shared core
     words (`Variety.parent/fork/family`, `kin`).
   - *Sounds*: the inventories, the minimal word, stress, geminates.
   - *Sound laws*: a dated ledger, each law with the words it changed,
     before and after, and where it came from if it spread as a wave
     (`Variety.laws`, `Annal.specimen`, `word` history).
   - *Word building*: the builders (affixes or vowel patterns).
   - *Grammar*: the sketch (word order, how the object is marked, where
     the possessor goes), a sample sentence and a possession phrase
     rendered from the lexicon, the categories the engine models
     (plural, past, object: `grammar.markers`), and a heading for what
     it does not model yet, stated as such.
   - *Lexicon*: every word with its origin (inherited, borrowed with its
     source and the contact behind it, coined, stretched), competitors,
     and cognates across the family (`lexicon`, `word`).
   - *Names*: given names and their style; the lands and peoples named
     in this tongue (`givenNames`, `PlaceName.coiner`).
   - *Standing*: standard, classical, sacred, written, vernacular, and
     which state, faith, or city made it so.
5. **The states, faiths, crafts, cities.** Short sections; their effect on
   speech is the point (court speech, sacred language, koine, the words a
   craft brings) (`StateView`, `ReligionView`, `crafts`, `cities`).
6. **The chronicle.** Every entry, by year, with causes where recorded,
   and the author's decisions marked as such. It is read in eras, not
   as a log. An era opens when a state rises or falls, a people
   conquers another, a faith is founded, or a language becomes
   classical, and runs at least two hundred years: an opener sooner
   than that stays a headline inside the era. The era's heading is a
   few words from the opener's subjects ("Iffi rises"); its full entry
   is the title. Within an era the headline entries stand at full
   weight (a people appears, parts, moves, or changes tongue; a
   conquest; a state; a faith and its
   schisms, conversions, and holy lands; a craft; a city and its
   speech; a language written, standardised, or made classical; a
   people ends; a way of life or hard times). The bookkeeping of a
   year (neighbours, spreads, crowding, the weather, a river, a
   meaning or spelling, a temper, a sound change, a grammar change)
   folds into one quiet line under the year, "also that year", which
   opens to its entries. A filter to one group or a search unfolds
   what it matches. The chronicle's card is the table of contents:
   each era with its span, its heading, and the peoples, states, and
   languages it concerns, at the dense scale.
7. **Notes.** The author's own, dated, with the subjects they refer to.
8. **Other tellings.** Each draft's name, the year and decision it
   diverged on, and what it kept.

## A spherical world, two views

The chart remains the first view, with a rotatable inked globe beside it
as a choice in the cartouche. Founding, lands, journeys, names, and cards
refer to the same world in both views. A projection is a way of reading,
not a decision in the world's history.

The world is spherical from its founding. A people's known-world lens can
show how its geographic knowledge grows on either view; it does not change
the surface. A literal flat world made round would be a separate authored
historical event, not an automatic stage of this simulator.

## Screens

Four, each a chapter of the book or a tool for writing one. Anything
that is neither is residue of the live-game phase and goes.

### The chart room (the shelf)

An index of the author's worlds, not a menu for strangers. The last
opened world leads, large: its miniature, name, year, the last chronicle
line, and *Continue*. Other worlds below it in a row. Then, smaller:
*Found a new world*, *Read a chronicle already written* (the sample),
*Open a save file*. Tile text is the world's name and the last line of
its chronicle, never a comma list of language names.

### Founding

The chart, with the title box, and the book of accounts beside it. The
accounts are the navigation: each people is one inked line when closed
("Hū · the people · farmers of Nūhupam · soft-spoken, rich in f, th,
kh") and opens into its six stages when chosen (homeland, livelihood,
temper, speech, grammar, identity). This scales to twelve
peoples without a tab strip. Reach is drawn on the chart when an account
is open (a ring, neighbours it can meet shaded), with effort on hover,
not listed as lines of effort-km. The sound chart and naming pattern are
one click from the account, since that is where this author spends their
time. The world is named here, defaulting to the landmass. The footer
verb is *Begin*; nothing else is in the footer.

### The workshop (the stage)

The chart, the cards, the chronicle, and the time bar. What changes:

- The time bar's primary verb is **Run to year N**, defaulting to a long
  span (4000). Play, step, and stop-for move into a study drawer on the
  bar for whoever wants to watch one law spread. The timeline track
  bins by century at wide spans and draws turning points over a quiet
  density of sound changes; a cluster opens on hover.
- At year 0 the open card is the frontispiece: the name, the peoples,
  *Run*. No tutorial, no stat box. The one-line caption at the foot of
  the chart does the teaching: "Year 0 · eight peoples settle Hūhupam".
- When the years move, the open card is the frontispiece or something
  that exists at the current year, and the breadcrumb says the current
  year. The folio is a card opened wide; it never has its own title.
  No banner about previews going stale, because running and deciding
  are never simultaneous.
- The language card is the primary card; the people card is "who speaks
  it and what happened to them". Both open with the derivation order:
  land, people, fortunes, speech.
- A decision is taken from a card (*Decide…*), dated, and marked in the
  chronicle as the author's. It opens another telling if it is not at
  the tip.
- Labels on the chart follow a policy by zoom and layer: people names
  always; tongue names only when they differ from the people's and the
  layer is families; land names when zoomed in; state outlines only on
  that layer or when a state card is open. Eight peoples is where
  cartography starts to matter.
- Reading as a people: *as the Kakimi knew it* is a lens on the chart
  (their known lands, their names for them, their neighbours as they
  called them), chosen from a people's card. That is the explorer frame,
  kept as a way of reading rather than a mechanic.
- Two scales on a card. Prose (the head, a section's sentence, an
  entry's text) keeps the reading measure. Apparatus (a facts box, a
  roster, a ledger, a leaf's title row, the chronicle's contents) sits
  at the dense scale the layers menu set: smaller type, tight rows,
  little padding, so a card holds detail without scrolling.

### The book

A first-class page, reached from the workshop's cartouche, laid out to
be read in the order above, with the chart as frontispiece. Reuses the
card sections rather than restating them. *As a save file* and *As text*
are its last page, not its reason for existing.

## Vocabulary

Six words. Everything the reader sees uses them.

| Word | Means |
|---|---|
| chart | the map |
| chronicle | the record of years, and an entry in it |
| card | any subject opened for reading; opened wide it is still a card |
| telling | a draft of the world from a decision onward |
| decision | the author's dated intervention |
| note | the author's own writing |

Retired: moment, discovery, notebook, field notebook, folio, leaf,
history, reading, shape history, keep this discovery, stop for. "Account"
stays only in founding, for a people's founding account. `lore.ts` holds
the names; nothing else mints one.

## Honest causes

The book states challenge and response only where the engine recorded
the trigger. Today it does for temper (`temper.cause`), climate, and
rivers, and for loans (`LoanCause.event`). It does not for migration,
split, displacement, livelihood, state rise and fall, standard, koine,
conversion, or conquest, which are exactly the responses the book is
about.

Engine contract: every response annal carries an optional `cause` with
the triggering annal's id and a mechanism label, set where the engine
knows the trigger at the decision point and absent otherwise. The
mechanisms, from `docs/engine.md`:

| Response | Trigger it can name |
|---|---|
| migration, split, displaced | hardship, crowding, a stronger co-resident people, a drying climate |
| livelihood | climate change, a craft learned |
| rose | hard times, crowding, a stronger neighbouring state |
| fell | hard times, a conquest |
| standard, koine | a city standing long enough |
| conversion | a contact, a pilgrimage road |
| conquest | an unfaithful holder of a holy land |

The chronicle shows a cause as "after the famine of 1250" linking to its
entry; a card's fortunes section shows challenge beside response. Where
`cause` is absent the entry says what happened and nothing more. This
bumps `ENGINE_REVISION`, since annals change shape.

## Delivery

Branch `book` off `main`. New screen components beside the old ones;
`model.ts`, `engine.ts`, `shelf.ts`, `takeout.ts`, `lore.ts`, `MapView`,
`Timeline`, `Popover`, `styles.css`, and `chart.css` are kept and
extended. Lanes in worktrees per `umran-ui-lanes-worktrees`; the engine
cause contract is one Astra lane. Cut over when found → run → read →
book works end to end on the branch; the old shell is deleted in the
same commit. The README and `docs/images/` follow the cutover.

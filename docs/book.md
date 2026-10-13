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
   and climate; every named land in a line, with the peoples who hold it,
   and the chronology of its names and who calls it what folded beneath
   (`Place`, `PlaceName`, `climate`, `river`, `lake`).
3. **The peoples.** One section per people, living or ended, ordered by
   founding. Each opens the way the engine derives it: *where they live*
   and what the land is like; *how they live* (livelihood); *what they
   are like* (temper, with the causes that turned it); *their fortunes*
   (the chronicle filtered to them and read in the world's eras, its
   headlines in full and each era's quieter entries folded into a line);
   *what they speak*, with the specimen. Then kin, dealings, faith,
   crafts, and names others call them, each name once with every people
   that says it (`Community`, `story`). A people that has ended keeps a
   paragraph (its last land, way of life, speech, and what became of
   it), with the rest folded.
4. **The languages.** The centre of the book and the reason for it. One
   chapter per language, living or silent, grouped by family; a silent
   language keeps its specimen with its chapter folded:
   - *Position*: parent, daughters, the family tree, kin by core-word
     sound similarity, chance-corrected and not a count of shared roots
     (`Variety.parent/fork/family`, `kin`).
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
   The sound laws, grammar, lexicon, and names, each hundreds of rows
   for a long-lived language, open beneath a one-line summary.
   The six-meaning ribbon labels only spelling changes; sound-only ticks
   keep their IPA before and after on hover and keyboard focus. Each
   recorded holder has a separate band ending at its recorded end, with
   unknown onsets kept visibly uncertain. The language card opens with a
   compact ribbon and current forms, linking to the labelled Sound laws
   reading; those labels fit within their lanes without overlapping.
5. **The states, faiths, crafts, cities.** Short sections; their effect on
   speech is the point (court speech, sacred language, koine, the words a
   craft brings) (`StateView`, `ReligionView`, `crafts`, `cities`).
6. **The chronicle.** The headline entries, by year, with causes where
   recorded, and the author's decisions marked on the entries they
   produced. It is read in eras, not as a log. An era opens when a state
   rises or falls, a people conquers another, a faith is founded, or a
   language becomes classical, and runs at least two hundred years: an
   opener sooner than that stays a headline inside the era. The era's
   heading is a few words from the opener's subjects ("Iffi rises"); its
   full entry is the title. Under the heading, one line tells what
   happened to speech in the era: the languages that arose, how many
   sound changes fell and on which languages most, new words, and
   grammar changes, since eras are named for states and faiths and the
   languages' own history would otherwise sit unseen. Within an era the
   headline entries stand at full weight (a people appears, parts,
   moves, or changes tongue; a conquest; a state; a faith and its
   schisms, conversions, and holy lands; a craft; a city and its
   speech; a language written, standardised, or made classical; a
   people ends; a way of life or hard times). The bookkeeping of a
   year (neighbours, spreads, crowding, the weather, a river, a
   meaning or spelling, a temper, a sound change, a grammar change)
   folds into one quiet line under the year, "also that year", which
   opens to its entries; consecutive quiet years fold into one line
   for the run. A filter to one group or a search unfolds what it
   matches. Every reading of a chronicle (the chronicle's wide card, a
   subject's fortunes, and the book) uses the same eras and reads from
   the beginning; latest first is a choice. The chronicle's card is the
   table of contents: each era with its span, its heading, the peoples,
   states, and languages it concerns, and its speech in counts, at the
   dense scale. The Markdown export keeps every entry.
7. **Notes.** The author's own, dated, with the subjects they refer to.
8. **Other tellings.** Each draft's name, the year and decision it
   diverged on, and what it kept.
   Comparing two drafts opens with the year and recorded decision they
   diverged on. **What changed in speech** reads each shared language:
   sound laws only in one telling, changed specimen words from left to
   right, loans counted by donor, and descendant-language counts. Law,
   word, and donor links open the evidence in that telling at the
   comparison year; descendants born after divergence are never paired
   by their numbers. Only languages whose evidence differs are shown;
   when none differ, the section says the tellings speak alike so far.

Namesakes keep their recorded names. Place links add terrain when needed,
then a compass direction among namesakes on like terrain. A local ordinal
appears only on labels that remain identical. Compare names shown in the
same reading, not unrelated historical spellings or exonyms. Cities use
their land; rivers use their source; lakes use a shore region. Peoples use
their heart land ("Kin of Opuw"), and a number only when that land is
unnamed or shared.
Successive realms of one people carry their founding year, including in
era headings. The breadcrumb is a path: revisiting a card truncates the
path to it. Chronicle pages keep each year together, and the contents'
remaining-subject count opens that era.

## A spherical world, two views

The chart remains the first view on wide screens, with a rotatable inked
globe beside it as a choice next to the map's navigation controls; a
tall, narrow map opens on the globe instead, since a whole sphere fits a
portrait screen better than a strip of chart. Once the author picks
one, that choice holds. Founding, lands, journeys, names, and cards refer
to the same world in both views.
A projection is a way of reading, not a decision in the world's history.
Each world opens padded around all living peoples' lands on either view,
unless its reading place already keeps a camera. The chart room remembers
each world's chart camera, globe camera and turn, and chosen projection.
Home always returns to the whole chart or globe. A recorded passage can
temporarily frame other lands and draw its dated route; ending that reading
returns to the reader's camera rather than replacing it.

The world is spherical from its founding. A people's known-world lens can
show how its geographic knowledge grows on either view; it does not change
the surface. A literal flat world made round would be a separate authored
historical event, not an automatic stage of this simulator.

## Screens

Four, each a chapter of the book or a tool for writing one. Anything
that is neither is residue of the live-game phase and goes.

### The chart room (the shelf)

An index of the author's worlds. The last opened world leads, large: its
miniature, name, year, the last chronicle line, and *Continue*. Other
worlds below it in a row. Then, smaller: *Found a new world*, *Read a
chronicle already written* (the sample), *Open a save file*. Tile text is
the world's name and the last line of its chronicle, never a comma list
of language names.

With no worlds yet, the room is a title page for a stranger: the mark,
"The book of a world", one sentence on what Umran writes, *Found a new
world* with the founding steps listed beneath it, and the sample's chart
as a frontispiece that opens the sample. While the engine loads, the mark
turns above the same sentence.

### Founding

Founding is a sequence of steps, always opened at the first. A rail
across the top names them, and any step whose prerequisites hold can be
reopened; the chart stays behind every step. All steps edit one draft on
the real engine.

1. **The world.** For worlds drawn from moving plates, the coasts form
   first: the supercontinent, the engine's ten drift steps, then the
   drawn coasts (`forming()`), each frame the generator's own state.
   *Skip* ends it; reduced motion shows the coasts at once. The plate
   offers the sizes as cards, *Draw other coasts*, and the seed. Changing
   either after founding requires an explicit choice to relocate the
   peoples; their speech and kinship survive. Selecting a land opens the
   next step.
2. **The homeland.** Selecting a land reads its terrain, area, rivers,
   and inhabitants; it never moves a people silently. Once peoples exist,
   *Found another people here* and *Move the … here* act on that land.
3. **The peoples.** Three ways to begin: *One people*; *Kin who parted*,
   two to six peoples speaking branches of one ancestral tongue; or
   *Neighbours*, two to six peoples with languages of their own. Groups
   take distinct, reachable homelands, nearest first but with two lands
   between homelands where the land allows. Related speech has real
   ancestry in the engine; matching sound presets alone do not make a
   family. Then a compact roster above a portrait in derivation order:
   - *How they live*: as the land suggests, or farmers, herders, or
     foragers, each card saying how many souls the homeland would feed
     that way (`foundingTraits`), with a line on what the way of life does.
   - *What they are like*: all six leanings, each a ruled line between its
     ends. A lozenge marks where land and life put them; four stops let
     the author lean them somewhat or strongly, replacing that axis only.
     Beneath, one line per notable leaning says what it drives in the
     engine.
   - *Among other peoples*: standing (the floor of prestige, which decides
     conquest and whose words are borrowed) and openness to foreign words,
     three stops each. Kin share their ancestor's.
   - *What they begin with*: crafts, a realm, a faith, each a dated year-0
     decision. One the engine declines shows as not possible here.
   - *What they call their children*: the given names their language
     draws, which move with their way of life and temper.

   *Move* on the portrait, or *Another people…*, takes the next land
   clicked; Escape or *Cancel* leaves things as they were. *Start again*
   clears the peoples after asking. Reach is drawn on the chart for the
   selected people, with effort on hover.
4. **Their speech.** Every preset as a voice card showing the words this
   people would have under it: the engine's own preview with the seed
   founding uses, so the chosen card's words are the founded words.
   *Draw other words* draws another seed; *Shape the sounds by hand…*
   opens the full sound chart. Grammar follows as phrases with sample
   sentences. Related peoples share their ancestral sound and grammar
   choices.
5. **The title page.** Set like a printed book's opening over the
   dimmed chart: the world's name, the author, the peoples, the seed, and
   the year the chronicle runs to (4000 by default). Its verb is *Write
   the chronicle*.
6. **The years.** The workshop opens and runs to that year at once. While
   it runs, the frontispiece tells the chronicle being written: the year,
   a progress rule, the latest turning points as they happen, and *Stop
   here*.

One starting region is enough; there is no requirement to populate the
globe.

### The workshop (the stage)

The chart, the cards, the chronicle, and the time bar. What changes:

- The time bar's primary verb is **Run to year N**, defaulting to a long
  span (4000). Play, step, and stop-for move into a study drawer on the
  bar for whoever wants to watch one law spread. The timeline track
  bins by century at wide spans and draws turning points over a quiet
  density of sound changes; a cluster opens on hover.
- Reading the past keeps **Run** disabled. Beside it, **Continue in
  another telling** opens a small panel with the exact reading year's
  last decision and “Your telling is kept”. Choose the year to write
  through, defaulting to the present's year (or five hundred years beyond
  a same-year reading). Confirming creates a telling from that reading
  point and runs it in the usual stoppable chunks; Escape or an outside
  click closes the panel without changing either telling.
  The panel explains that without a new decision the years repeat as
  told, and that deciding something in the past also begins a new telling.
- At year 0 the open card is the frontispiece: the name, the peoples,
  *Run*. No tutorial, no stat box. The one-line caption at the foot of
  the chart does the teaching: "Year 0 · eight peoples settle Hūhupam".
  While the years run, the frontispiece shows the chronicle being written
  in place of *Run*.
- When the years move, the open card is the frontispiece or something
  that exists at the current year, and the breadcrumb says the current
  year. The folio is a card opened wide; it never has its own title.
  No banner about previews going stale, because running and deciding
  are never simultaneous.
- The language card is the primary card; the people card is "who speaks
  it and what happened to them". Both open with the derivation order:
  land, people, fortunes, speech.
- The philologist's desk opens wide from *Compare with another language…*
  on a language card, or *Open in the desk* beside a pair in the world's
  *Words compared*. Choose a living language and a meaning set. It begins
  with the record covered: neutral names, aligned sounds in two rows,
  gaps shown as a dash, and recurring sound pairs with their word support.
  Selecting pairs highlights their segments; *Note a hypothesis* carries
  the selected pairs into an editable, dated note kept in the first
  language's margin. *Reveal the record* separately reads each word's
  shared root and borrowing, with the contact's chronicle link, and the
  languages' common ancestor and parting year. Differences between
  recurring support and root history are explained, never graded.
  *Words compared* chooses its meaning set, living-language subset, and
  reference language; its shading is labelled *Recorded roots*.
- A decision is taken from a card (*Decide…*), dated, and marked in the
  chronicle as the author's. It opens another telling if it is not at
  the tip.
- Labels and layer detail follow one physical scale in kilometres per
  pixel, not a fraction of the world's drawing. At world scale only
  continents and the six largest living peoples are lettered. Below
  6 km per pixel, land and water names begin at 13 screen pixels and grow
  to 20 as the reader comes closer; tongue names join below 1.5 km per
  pixel when they differ from the people's and the layer is families.
  Terrain glyphs have screen-size caps, while borders keep their fine
  strokes. Names sit at their places, never pushed aside on leader lines;
  where two collide, states and the larger ruling peoples win, and the
  rest keep their dot until a closer zoom makes room.
- A compact ruled key stays on the chart, naming the active colouring
  and the overlays that are on. At world scale it explains that smaller
  lettering, roads, dealings and marks appear in closer views.
- Hover or focus a place on either map to read its gazetteer. A land
  shows its name and pronunciation, at most two other names and how many
  remain, its holders, and one recorded chronicle entry. Clicking a
  printed name or land pins the inspector; only Open in its fixed footer
  navigates to the full card.
- *Follow their fortunes* on a people card reads their recorded journeys
  and headline entries, from the beginning through the telling's retained
  tip. Previous and Next scrub to each entry's year and frame the involved
  lands on either projection; dated routes use recorded travel legs only.
  The people card stays visible beside the chart. The chart's key folds away
  during the tour, and the panel keeps clear of the time bar and compact compass.
  Entries without named lands use their participants' holdings in that
  recorded year, never their present-day holdings. Done or Escape restores
  the exact prior reading point, card, and camera. No journey or annal is
  invented when the record has none.
- Lakes are drawn over the lands they cover, with a shore only where
  water meets land. The chart's top and bottom are a ruled frame, the
  edge of the drawing rather than a coast.
- The viewpoint control beside Chart and Globe starts at *The author's
  names*. Choose *As the … knew it* and a people, then independently
  choose *Their names* and *Only lands they knew*. Opening a different
  card never changes this choice. The known-lands action on a language
  card explicitly keeps the author's names when it narrows the lands.
  This is the explorer's lens, a way of reading rather than a mechanic.
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

The seal, contents, and frontispiece appear before the engine finishes
reading the chapters. Lands, each people, and each language section wait
independently; a route or bookmark keeps its anchor as later sections
arrive. Boxed initials do not decorate the book's headings.

Print, the self-contained web page, and Markdown are editions of this same
book, with every disclosure open. Editions keep their own read promises
until completion, beyond the workshop's bounded card cache, and open
languages in small batches so the progress counts completed chapters.
Full word histories span the lexicon ledger's measure in an edition;
printed evidence can continue across pages instead of trapping a whole
history in a narrow, unbroken origin cell.

### At every size

The same four screens on a phone, a folding phone, a tablet, and a
desktop, arranged by Android's window size classes and no other
breakpoints: compact below 600 px wide, medium to 840, expanded beyond,
and a short class below 480 px tall for phones on their side.

On compact screens the map fills the window and the card is a sheet over
it, with three detents: a peek that shows the title and the time bar, a
half that shares the screen with the chart, and the full card. Opening a
subject raises the sheet to half; a decision on the chart lowers it.
Founding keeps its panel in the same sheet and folds the rail into one
line, "2 of 6 · The homeland", that opens a menu of the steps. A medium
screen held upright stacks the chart above the card; expanded screens
keep the card as a column beside the map.
Touch targets grow to 44 px on coarse pointers, and the frame respects
the notch and gesture bar.

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
the trigger. A response annal carries an optional `cause`: the
triggering annal's id and a mechanism, set where the engine knows the
trigger at the decision point (`World::causes`, `causes.rs`) and absent
otherwise. No facade search or adjacency rule supplies a missing cause.
`docs/engine.md` lists what each response records; in short:

| Response | Triggers it can name |
|---|---|
| migration, displacement, a sea colony | a stronger neighbour, climate |
| livelihood | climate |
| a people takes to the sea | the craft of seafaring learned |
| rose, fell | hard times, climate, a stronger neighbour, a conquest |
| standard, koine | a great city |
| conversion, a holy-land conquest, doctrine | a contact, a pilgrimage road, an unfaithful holder, a schism |
| coinage, pronouns, purism, tone | a word needed, a court, foreign prestige, a revival, a purist norm, contact |

Territorial splits, crowding, and ordinary conquest have no recorded
trigger and say none.

The chronicle shows a cause beneath its entry ("after a conquest in
3425 ↗"), linking to the triggering entry; `lore.ts` names each
mechanism. Where `cause` is absent the entry says what happened and
nothing more.

## Delivery

Branch `book` off `main`. New screen components beside the old ones;
`model.ts`, `engine.ts`, `shelf.ts`, `takeout.ts`, `lore.ts`, `MapView`,
`Timeline`, `Popover`, `styles.css`, and `chart.css` are kept and
extended. Lanes in worktrees per `umran-ui-lanes-worktrees`; the engine
cause contract is one Astra lane. Cut over when found → run → read →
book works end to end on the branch; the old shell is deleted in the
same commit. The README and `docs/images/` follow the cutover.

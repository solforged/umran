# The living atlas

Design backlog, October 2026. Superseded as direction by `book.md`, which
frames the product as the book of a world and keeps the pieces delivered
here as workshop tools. The record of what was built stays below.

The direction was an observation-first world
simulator: a lone historian charts unfamiliar peoples, follows their words,
and gradually learns how their histories connect. A playthrough should become
more interesting to read as it grows. More engine detail is useful when the
reader can discover it, understand it, and make a different history from it.

The working loop is **observe → notice → investigate → intervene → compare**.
The chart supplies place; the encyclopedia supplies context; the chronicle
supplies time. Names and events should lead between all three without losing
the reader's place. Preserve the explorer's chart, the account book, and the
folio laid over the map.

Choose the strongest long-term design for this vision. Backward compatibility
is not a requirement: replace incidental interfaces and data formats when a
coherent model calls for it. Keep validation focused on consequential behavior,
with clickthroughs for interaction design and targeted tests for engine
invariants and tricky state transitions.

## This workbench pass

- Searchable atlas index for peoples, languages, named lands, continents,
  states, faiths, and crafts; names can be typed without their accents and
  older names remain useful search terms.
- Chronicle search, subject filters, chronological order, explicit sound-law
  scope, bounded pages, and other tellings kept separately from the current
  history. Following an event and returning keeps the chosen reading.
- Chart/Reading modes on small screens, connected keyboard folio tabs,
  intentional card scroll positions, and specimen words high on language cards.
- Dictionary filters and the selected word survive a visit to a word's history;
  keyboard navigation selects a row and Enter opens it.
- Visible map coloring, a known-world reminder, fit-to-selection, keyboard
  navigation, touch pinch handling, and reduced-motion support.
- The engine's map-size catalog reaches setup, including the vast theatre.
  This exposes the current geography; it does not make the geography more realistic.
- Chart-based settlement accounts now replace the split dialog: connected
  territorial division, settlers from a chosen population share, and whole
  migration, with real routes, a census and an exact pre-choice return.
- Grammar-change events from the new engine revision join the chronicle and
  language-event filters, with their explanation and participants on the card.

## 1. Let an intervention have a place

**Observed problem:** “Some go their own way” does not let the reader say
*where*, or distinguish a departing group from a territorial division.

The current engine has three related behaviors (`world.rs`):

- `split_large` only splits peoples with more than one holding.
- `migrate` moves a one-land people as a whole, retaining its language.
- `leavers` automatically partitions multiple holdings around a far holding.
  On one holding, it moves half the population to a better neighboring land
  or an available sea colony, falling back to the same home.

The former `Split` action had no destination, partition, or population-share
field. Revision 30 replaces it with `Settle`: an explicit intent, destination,
share and naming/contact choices. “Half leave” was misleading, especially
for a territorial split.

Build a chart-based preview with three explicit intents:

1. **Part along their lands.** Choose the new heart; shade the proposed
   holdings of each people and show how many souls belong to each.
2. **Send settlers.** Choose a reachable destination and a share of the
   population. Found a daughter people whose speech can diverge.
3. **Move the people.** Choose a reachable destination while retaining the
   community and its language.

Rust must return the eligible destinations, travel effort, voyage or land
journey, proposed holdings/populations, and explanations of unavailable
choices. The browser selects and draws this information. Preview must not
consume random draws; apply revalidates the choice in the viewed generation
and records it in the recipe. Use the travel graph for connected territory,
not screen distance. Both sides of a partition need viable lands and people.

Acceptance: from a people's card, preview a nearby daughter settlement, inspect
what happens to both groups, apply it, follow both histories, rewind, and make
a different choice. Later years become another telling that remains readable.
Check non-seafarers, occupied lands, disconnected holdings, and a rejected
preview that has become stale. Replace the old action shape if the spatial
model calls for it, and bump the engine revision when replay changes.

Delivered in the living-atlas worktree: all three intents, city residents and
capital loss, connected authored partitions, route evidence, community ancestry,
stale-preview rejection and atomic exact-point intervention. Desktop and phone
clickthroughs cover applying, returning before the choice, making a different
choice, reloading and restoring the preserved telling. Native/WASM parity covers
the planner and all three resulting histories. Named, addressable tellings and
side-by-side comparison now complete the alternate-choice loop.

Separate engine question: should a one-land people spontaneously send a
splinter group? That changes histories and needs calibration, rather than a
UI-only fix.

## 2. Turn accumulated events into an investigation

The new chronicle is a useful index. The next step is continuity across time.

- Follow a people, language, place, faith, or word through one contextual
  timeline. Let a name change without breaking that thread.
- Give an event a clear account of *who, where, when, and what changed*, with
  before/after words or territories where the engine can provide them.
- Offer an overview by era, with quiet summaries of repeated minor events and
  every original entry still available. The reader chooses when to zoom in.
- Keep a small notebook of bookmarked discoveries, named years, and questions
  to return to. A long playthrough should collect the reader's observations,
  not only the simulator's output.
- Let the reader follow a developing story during playback and stop for that
  subject, instead of choosing only a broad event type.
- Put related entries within reach: from a conquest to its rulers, subjected
  peoples, changed court speech, and relevant vocabulary.

Do not invent causes from adjacency in a log. A true cause-and-consequence
view needs structured evidence from Rust: a triggering event, mechanism, and
participants stable within that telling. Colony → new faith → conquest →
borrowed sacred vocabulary is an investigation to support, not a story to
assert when the engine has not recorded the links.

Acceptance: start with one unfamiliar word, discover the faith or encounter
that brought it, see where its speakers lived at that time, follow its later
forms, and return to the original word and reading position.

## 3. Make other tellings tangible

The recipe and recovery model already keep alternate histories. Build on it.

- Make the year where the reader is about to intervene unmistakable. Preview
  which later years will be set aside before applying the intervention.
- Name a telling and show its divergence year and the choice that created it.
- Compare two tellings at the same year: holdings, peoples, family trees,
  survival of a word, and changes to its form.
- Return to a prior telling without losing the one just explored.
- Allow a few hypotheses or notes to travel with the saved world and exports.

Delivered: permanent named telling records with exact parent readings,
non-mutating return before an action, independent reading views, and same-year
comparison with linked map cameras, populations, holdings, family trees and
word forms. Each panel's links open its own account. Ancestry limits shared
identity to subjects already present before divergence; later descendants stay
within their own telling. Naming and switching accounts survive saved replay.
Reader hypotheses and notes are the next piece.

Acceptance: rewind before a conquest, avoid it, run forward, and compare the
languages and sacred vocabulary in both histories without manually exporting
or juggling separate browser saves.

## 4. Found civilizations as an unfolding account

Keep the book of accounts, but let the reader compose a founding people in
meaningful stages: homeland, livelihood, temper, sound, identity, and first
encounters. It should feel like assembling an expedition's first reports.

- Give one people the page at a time, with a clear overview of all founders
  and an easy way to return to any account.
- Keep a real map and real specimen visible while making choices. Explain
  what a choice changes using the engine's preview, not promised outcomes.
- Show likely separation and possible contact between homelands using actual
  travel rules; distance on the drawing alone is not enough.
- Offer strong starting characters while allowing a reader to open the full
  sound chart or tune a particular leaning.
- End with an intelligible founding charter: who lives where, how their
  speech differs, and the first questions this world might answer.
- Keep fast random setup available for repeat visits. Guided setup should
  add understanding, not become mandatory ceremony.

Acceptance: create three recognizably different peoples, understand why they
may or may not meet, revise the second one without losing the others, and
begin with a short set of people/place/word trails worth following.

Delivered: one open account with five visible stages, a pinned keyboard
contents line, and a founding charter comparing peoples, specimen words,
and first journeys. The year-zero facade preview reuses actual neighbour
and merchant reach rules without draws or mutation. Three to five optional
questions become exact people/place/word/language references in the field
notebook on Begin; immediate random founding remains available. Desktop
and phone clickthroughs in both lights cover revising the second account
without changing the other two, and following a saved question to its card.

## 5. Give geography a believable scale and shape

The vast theatre has 3,600 regions at the existing physical scale. Larger is
not the same as geographically convincing.

- Study the shape of coastlines, islands, bays, straits, connected inland
  areas, and routes through mountain ranges.
- Introduce coherent terrain and drainage, then climate and its history,
  using a common physical scale. Coordinate this with the engine's planned
  rivers/climate work rather than creating a second geography model.
- Name regional theatres honestly and keep km, travel effort, and map size
  visible where they affect a choice. Distinguish a regional sea, small
  continents, and a globe; do not merely stretch existing cells.
- Make zoom disclose appropriate detail: continental relationships at a
  distance, routes and holdings closer in, local names when readable.
- Benchmark both setup and long histories at the largest size. Rendering,
  layout, and replay all need budgets; sparse maps should still invite
  exploration before they fill with peoples.

Acceptance: two seeds create recognizably different opportunities for contact
and isolation, while coasts and terrain explain those opportunities at every
zoom level.

## 6. Animate the material, preserve the reader's place

Opening and closing should feel like handling charts and leaves: a folio
settles onto the desk, an account turns, an entry receives a brief ink emphasis.
Use short, interruptible transitions with a clear source and destination.

- Pair opening and closing motion; the current pass supplies only restrained
  entry motion and reduced-motion handling.
- Keep text stable while reading. Avoid ongoing parchment effects and
  unrelated movement during playback.
- Animate meaningful change on the map—arrival, departure, a new boundary—
  without implying travel paths or causation the engine does not supply.
- Preserve keyboard focus through transitions, support rapid repeated actions,
  and offer immediate motion-free changes with reduced motion enabled.

Acceptance: open and dismiss cards, folios, and dialogs repeatedly on desktop
and phone without delayed clicks, stolen focus, or movement after dismissal.

Delivered: interruptible paired opening and closing for dialogs, folios and
settlement accounts; directional card turns and keyboard-only heading focus;
brief ink emphasis when following a moment or returning through the trail.
Only one sheet is visible at a time. A dialog temporarily lifts the folio,
then restores its chosen leaf and filters; comparison returns to Tellings.
On a phone, a reopened folio's selected tab is the visible focus destination.
Single-step advances and interventions fade changed holdings and draw new
state borders, without motion during playback or multi-step scrubs. Reduced
motion makes every sheet and map change immediate.

## Suggested order

First finish spatial intervention previews and subject timelines. They make
the existing simulation more expressive and readable. Then build comparisons
between tellings and guided founding. Geography and its performance work can
advance alongside those interfaces, with the same Rust-owned preview contract.

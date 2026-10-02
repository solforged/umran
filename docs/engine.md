# Engine model

How `crates/umran-sim` models language change. The rules that must
always hold are summarized in `AGENTS.md`; this is the fuller picture.

Histories replay identically natively and in WASM on the same engine
revision. Transcendental math goes through `math.rs`, using `libm` rather
than std float methods whose last bits can differ between targets.
Clippy bans those methods; basic arithmetic and square roots stay native,
and integer squares use explicit multiplication.

`Chronicle` records each successful authored action in `World::decisions`:
its telling-local action index and the half-open range of `World::events`
positions it produced, including an empty range when no event was emitted.
These records clone with the world and are rebuilt on every replay path,
including exact readings and checkpoints. `Run` advances the simulation; it
is not an authored decision, and its events have no decision attribution.
This bookkeeping makes no random draws and changes no simulation events,
so the engine revision remains 31.

- `World` (`world.rs`) steps communities, varieties, and contacts through
  25-year generations. `Variety` holds a `SoundProfile` and a `Lexicon` of
  `Slot`s, where words compete for each concept with usage weights. Words
  keep a log of every sound law, borrowing, extension, and loss.
- The map (`geography.rs`) is drawn from the world seed at a chosen size
  (`MapSize`). Regions are Voronoi cells around jittered hex-grid points,
  so each borders about six others. Terrain is sea, plains, forest,
  steppe, hills, mountains, or desert. About half the regions are land, split
  among separated continental bodies and small islands. Each land region
  has one `landmass`; sea has none. The existing sizes have one to three
  continents with size-specific budgets. Vast has two, each at least
  800 regions. Budgets are checked rather than left to a favourable seed.
  Founders settle continents before islands.
  Drawing coordinates stay in map units. One unit means 100 km.
  Shared-border midpoints give geometric centre-to-centre routes.
  Terrain multiplies their physical length to give effort-km, equivalent
  plain kilometres: a 100 km mountain crossing costs 400 effort-km.
  Polygon area sets regional food capacity. Map geometry uses arithmetic
  and square roots, so native and WASM draw the same map.
  Elevation and moisture are retained rather than discarded after terrain
  choice. Static regional drainage supplies named river courses, and connected
  climate zones supply weather histories without redrawing the coast.
- Languages are founded from a `LanguageDesign` (`design.rs`): the exact
  sounds, each used or favoured, plus knobs (word length, final consonants,
  inner clusters, repetition, long vowels, geminates, stress, affixes or
  root-and-pattern, suffixing, derivation), grammar choices, and spelling. It resolves to an internal
  `SoundProfile`: chosen sounds exactly, favoured ones preferred, absent
  ones discouraged but reachable by sound change. There are no named
  culture packs; `SoundProfile::base()` weights sounds by world frequency
  for "draw by world frequency", and presets (the base plus a `Flavor` from
  `flavor.rs` or `palettes.rs`) only produce starting designs to edit.
- One preference score, taste plus cross-linguistic frequency from PHOIBLE
  (`typology.rs`), drives inventories, how often sounds are used, sound-law
  odds, and acceptance of foreign sounds. Sampled inventories favour
  feature economy (series like b d g), and a forbidden sound beats the
  "voiced implies voiceless" repair (Arabic has b without p).
- The catalog (`phoneme.rs`) describes consonants by place, manner,
  voicing, and a `Secondary` articulation (aspirated, breathy,
  labialized); new segments are appended, never inserted. Length is a
  per-segment flag: long vowels and geminate consonants. Founding geminates
  are off unless the design allows them; later laws can create them.
- Concepts (`concepts.rs`): the Leipzig–Jakarta 100 in rank order plus
  cultural concepts with a `Tier`. Word length follows `length_bias`
  (Zipf's law of abbreviation: basic meanings short, specialist long, one
  to three syllables). Parent words usually follow the nursery pattern
  (mama, papa). Only `expressive` meanings (small things, insects and
  birds, cries and sounds, baby talk) reduplicate or repeat consonants;
  others avoid it. Roots are unique within a semantic field, with a weak
  sound-symbolic bias. Minting skips spellings that read as English
  vulgarities, a courtesy rather than a linguistic claim.
- Word families (`FAMILIES`, `morphology.rs`): each language mints its own
  affixes, or vowel patterns over consonant roots for root-and-pattern
  (`MorphologyKind::RootPattern`), and builds some family members from
  their bases (`Origin::Derived`). Junctions get a link vowel or a glide as
  the language needs. Root-and-pattern derivations use each word's true
  root skeleton, never a surface reading that includes a pattern's prefix.
  New words come from curated semantic shifts (`RELATED`) or fresh roots.
- Grammar (`grammar.rs`) begins with count noun plural and object marking,
  and verb past. The object is the thing acted upon: an accusative-like
  contrast, not a complete case system. Count nouns name separate things,
  so water, fire, and collective people take neither plural nor object
  forms. Eligibility is explicit for each concept and follows a word's
  current senses, not only its first meaning. Designs save each category
  as suffix, prefix, particle, or none. A suffix follows its stem; a prefix
  precedes it; a particle is a separate grammatical word. Missing plural
  and past choices in older designs remain suffixes; missing object, word
  order, and possessor choices are drawn at founding. Affix-building
  profiles draw plural and past bound marking, particles, and no marking at
  weights 0.6/0.3/0.1; root-and-pattern profiles use 0.5/0.4/0.1. Object
  marking uses 0.45/0.15/0.40 in both. The suffixing preference divides
  bound choices between suffixes and prefixes.
- Word order and possessor order are fixed at founding and inherited by
  daughters; neither drifts. SOV means subject–object–verb, SVO means
  subject–verb–object, and VSO means verb–subject–object. Fully suffixing
  designs draw SOV/SVO/VSO at 0.55/0.35/0.10; fully prefixing designs draw
  0.15/0.50/0.35, with linear interpolation for intermediate preferences.
  The possessor precedes its noun with probability 0.80 for SOV, 0.50 for
  SVO, and 0.15 for VSO; otherwise it follows. Each setting has its own
  purpose-keyed stream, after the existing founding choices. Presets fix
  these as a sketch, not a reconstruction: pie-like, Indic, Iranian, and
  Caucasian are SOV; Germanic, Bantu, Finnic, and familiar are SVO; Semitic,
  Polynesian, and Nahuatl are VSO. Possessors follow in Semitic, Polynesian,
  Bantu, and Nahuatl, and precede in the other named presets.
- Eligible words store their complete attached forms and their histories.
  Particles share one marker form instead of copying it across all nouns.
  At most three active forms compete for each word and category. A word
  gains its current native marking when coined, borrowed, or used for a
  new eligible sense. Existing forms are never rebuilt every generation.
  Daughters inherit grammar with words. Productive endings for new words
  follow the usage-weighted dominant surviving edge, including empty edges,
  rather than repeatedly restoring the founding affix.
- Sound laws assess and change grammatical forms as well as base words.
  Alternative forms contribute by usage share, and shared particles count
  once. An attached ending has no size floor of its own, but its complete
  word retains last-vowel and minimal-word protection. Separate particles
  have their own stress and size floor. Umlaut fronts a vowel before a
  front vowel in the immediately following syllable: a becomes e, o becomes
  ø, and u becomes y. These sounds already exist in the catalog. Consonants
  may intervene, but a separate word cannot trigger the change. Ending loss
  can leave an audible stem alternation, so absence of an ending does not
  itself mean grammatical contrast has disappeared.
- Contrast retention compares sounds, length, stress, and word breaks, not
  spelling or internal morpheme boundaries. Category-wide loss is estimated
  only with at least five eligible living stems. Increasing loss raises a
  rare rebuilding chance but never forces recovery. Languages founded with
  no marking retain only a low baseline chance. Grammaticalization is a
  lexical word taking a grammatical job: many, all, or people can supply a
  plural particle; finish or have can supply a past particle; take, give,
  or hand can supply an object adposition. The source keeps its ordinary
  lexical use, and its copied particle evolves separately.
  When object contrast retention crosses from above one half to at most
  one half, the chronicle says that the object's mark wore away and word
  order now says who did what to whom. This does not change the already
  fixed order. The same rebuilding path may later supply a new adposition.
- New particles enter at a small share. Usage drifts with a modest advantage
  for forms that preserve audible contrast. After eight consecutive
  generations of majority use, a particle becomes eligible for rare fusion
  into a bound competitor. Fusion joins current sounds without restoring
  lost ones and derives one word's stress. Rare analogy levels one irregular
  form toward the dominant productive ending and records both forms. Retired
  realizations retain histories and stop undergoing sound laws.
- Loans normally receive native inflection. Strong contact between bilingual
  speakers can rarely import a current donor base and its marked form,
  including a shared separate particle, beside a native competitor. Neither
  acquisition nor fusion makes the foreign marker productive. Productive
  bound transfer requires at least three imported pairs, eight consecutive
  strong-contact generations,
  and a much rarer draw. Language shift mildly and probabilistically favours
  one productive pattern, while retained substrate words receive native
  inflection. It does not erase categories.
- The comparative method supplements lexical evidence with current
  same-category contrasts, including particles and surviving stem changes.
  Repetition of one marker pair contributes at most one correspondence
  support per category, not one clue for every noun. Marker ancestry,
  grammatical sources, histories, and lineage are never evidence.
- Language views report contrast retention separately from how synthetic
  they are: the usage-weighted share of eligible forms with an audible
  grammatical contrast inside one word. Separate particles score zero for
  synthesis. Word views carry paradigm shares and histories; grammatical
  spelling freezes at attestation just as base spelling does. Grammar
  founding, rebuilding, competition, fusion, analogy, and contact each use
  independent purpose-keyed random streams. Revision 32 adds the object
  category and changes the replay of older recipes; originals and recovery
  remain available.
- Each language view carries `grammar.order` (SOV, SVO, or VSO),
  `grammar.possessor` (before or after), and `grammar.marking`: case while
  there is a productive object marker and object contrast retention exceeds
  one half, otherwise order. Here "case" includes a separate adposition.
  The sample sentence is "the child saw the dog": bare child, the current
  past form of see, and the current object form of dog, in the language's
  order. Each category uses its most-used living realization, including
  surviving stem changes. Bound forms gloss as see-PAST and dog-OBJ;
  particles have their own PAST or OBJ gloss item, on their recorded side.
  Text and stressed IPA separate spoken words with spaces. The possession
  sample is "the child's fish", child and fish in possessor order, without
  a marker: no genitive is modelled. The facade omits the sample if a
  required word or form is missing. The design JSON keeps plural and past
  choices and adds nullable object, order, and possessor settings; null or
  omission draws that setting at founding, while a resolved design saves
  each drawn value.
- Calibration follows the founding language in each solo run, like the
  lexical statistics, rather than counting its cloned daughters as new
  independent observations. With the default 200 seeds and profiles in
  rotation, case marking starts at 55.0%. At 1,000 years it is 53.0%:
  4 of 110 founding case languages have lost contrast (3.6%), mean first
  loss year 350, with 15 object rebuilds. At 4,000 years it is 45.5%:
  21 of 110 have lost contrast (19.1%), mean first loss year 2170.2, with
  46 rebuilds. Rebuilds count new adpositions, not founding markers or
  fusion; a rebuilt language can regain case marking. Only suffixes lost
  object contrast in this sample; prefixes and particles did not.
  The target of one third to one half losing it over 4,000 years is not
  yet reached. Prior-only trials with bound/particle/none weights
  0.60/0/0.40 and 1/0/0 gave 28.2% and 28.5% loss, while 0.30/0/0.70 gave
  26.9%. The original 0.45/0.15/0.40 prior is retained rather than removing
  adpositions or forcing case marking on every language to chase the band.
- Sound laws (`laws.rs`) apply simultaneously and regularly to every living
  word, never to obsolete ones, and never delete a word's last vowel.
  "No change" competes with them, so a culture is never forced into a law.
  A catalog law may recur after sixty generations (1,500 years) if it
  would change living words again. Intervening changes and new words can
  restore its input sounds, as related consonant shifts happened in
  Germanic and later High German. The quiet span applies to inherited
  laws and incoming waves too; the full history keeps every occurrence.
  Stress-conditioned laws reduce unstressed vowels to schwa, syncopate
  medial vowels between single consonants, delete unstressed final vowels,
  lengthen stressed open-syllable vowels, and voice fricatives after an
  unstressed vowel (Verner's law). Cluster assimilation and gemination
  before j create long consonants; degemination removes length. The
  Western Romance chain voices single intervocalic stops before shortening
  geminates, so the two inputs remain distinct. Loss of a preconsonantal
  nasal can lengthen the preceding vowel. Length alone has neutral segment
  preference; assimilation is scored toward the surviving consonant.
  The author can also apply a catalog law to a living language as a dated
  decision. The catalog previews its effect on six basic words and counts
  the living lexemes (individual vocabulary entries) whose current forms
  would change. Eligibility also considers grammatical forms, such as a
  plural with an ending; the same minimal-word and last-vowel guards apply.
  A recent law may be chosen again if it still changes something. Authored
  laws enter the ordinary ledger, change living names and grammar, and
  spread to neighbours by the same wave mechanism. Koiné levelling is not
  offered: its mergers depend on the particular city's speakers, so it is
  not a portable rule. Revision means returning before the decision, not
  editing a derived word or removing a law from the ledger.
- Sound laws also spread as waves (`spread_waves`), as the wave model of
  language change describes: a law that took hold in a variety in the
  last ten generations may pass to a variety in contact with it. The
  chance grows with the contact's intensity and kind (neighbours and
  intermarriage carry sounds most, trade and religion least), how close
  their land is, the source's prestige, the law's freshness, and the
  receivers' taste, and above all with kinship: dialects that have just
  parted take up each other's changes readily, half as readily after 20
  generations apart, and unrelated languages seldom (`KIN_STRANGERS`).
  A variety takes at most one wave per generation and records each
  arrival's generation and source (`Variety::waves`). Waves make related
  languages share changes their common ancestor never had, leaving
  isoglosses (the lines where a change stopped) across the family tree.
  Stress shifts to the first syllable or penult are candidates only when
  the rule differs and a living word's accent would move. They change no
  segments, but record word and name histories and spread as waves.
- Each language has a minimal word (`prosody.rs`), drawn at founding: any
  syllable, a heavy one (two moras), or two syllables, likelier the more
  disyllabic its roots. A rule that would wear a word below it passes that
  word by, as apocope spares short words. Words below the minimum, or
  sounding like another word in use, draw competitors more often and lose
  usage; about half of those competitors are the old word renewed, with
  the language's renewing affix (Latin auris > auricula) or compounded
  with a related concept's word (Mandarin ěr > ěrduo). Renewed words keep
  their root, so they count as retained. `examples/length.rs` measures
  word length, homophony, and renewal over time.
  A language also has a `StressRule`: initial, penult, final, Latin-like
  weight (heavy penult, otherwise antepenult), or free. Predictable stress
  is derived from each current form, never cached. Free-stress founding
  words draw a lexical syllable on the separate founding-prosody stream;
  newly coined unaccented forms default to initial stress. Deletion keeps
  that accent on its surviving vowel, or the next vowel if it is lost,
  falling back to the last vowel. Loan epenthesis moves the index with its
  vowel. Compounds keep the leftmost explicit lexical accent; otherwise
  they carry the right accent across the junction, with hiatus coalescence
  retaining the preceding vowel. Clipped names retain their accent or use
  their last surviving syllable. Stress rules and their history are
  inherited on splits; adopting a language inherits its rule, not just
  its segments. Germanic and Finnic palettes start with initial stress,
  Polynesian with penult; other unspecified profiles draw independently
  of earlier founding processes.
  A geminate is one segment shared by the preceding coda and following
  onset when intervocalic, so it closes that syllable and contributes a
  mora. Initial consonant length does not create a mora. The facade's IPA
  display marks stress before the accented syllable in polysyllables and
  writes length with `ː`; `Form::ipa()` and its parser remain stress-free.
  Spelling doubles a geminate's entire letter or multigraph, never marks
  stress, and retains the existing long-vowel convention. Language views
  report their stress rule and whether living words contain geminates;
  specimens include a zero-based stress index and `wasIpa` when IPA changes
  even if spelling does not. `examples/stress.rs` traces these changes.
- Borrowability is set per concept (Leipzig–Jakarta rank or `Tier`), never
  per semantic field, so field patterns must emerge. `wold.rs` holds WOLD
  figures for validation only; the model never reads them. Loans adapt to
  the recipient's established sounds and then undergo only later laws.
  Every lexical loan keeps the mechanism that brought it (`provenance.rs`)
  in both its origin and borrowing log: a specific contact, rule, sacred
  language or conversion, classical learning, substrate retention, city
  levelling, or finding a word for a newly held idea. This is bookkeeping,
  not a new borrowing decision; loan identity still depends only on donor
  variety and word.
  The facade links these records to annal ids by recorded participants and
  dates. A parent–daughter contact links to its split or settlement when
  that event created it; classical learning links to the fixing of that
  language. Contacts without a recorded beginning, inherited faith without
  a conversion, and idea word-finding can have no event link. Religion
  contacts without a named faith remain contacts. Reading-scoped
  `story` returns original annal ids for a subject, expanding grouped entries
  and limiting word stories to the selected meaning's words and their
  recorded causes, meaning changes, and sound changes.
- Names (`names.rs`) are words: a people's name is coined from its own
  lexicon by a `Naming` (the people, those who speak, people of a place,
  an epithet on an older name, or, for a people that moves off, the land
  it settles, with the belonging affix: "of Opuw", as Northumbrians are
  of Northumbria), and its language is named after it with the language's
  belonging affix or a compound with "tongue" or "word" (`NameRules` in
  `morphology.rs`, drawn after all other founding draws). Names are
  clipped to three syllables for a people and four for a language, as
  names said every day are, and epithets go only on short names, so they
  do not stack over many splits. A split-off people avoids meanings other
  peoples have; when all are had, it is named for its land, or failing
  that for a place another people is also named for. It never takes a
  name in use: when its clipped name is another's, its land, a place, or
  an epithet on the old name said short ("the far Goths") tells it apart,
  or else whichever of those needs the fewest syllables more. A new
  language whose usual name is taken is named the other way ("the X
  tongue" for "of the X") or in full. Names then undergo the same sound
  laws as their variety's words, so two may still come to sound alike
  over time. Split-off peoples name themselves; a people that shifts
  keeps its name and names its new speech after itself. Exonyms are the
  name adapted to a contact's sounds, computed from current forms.
- Lands are named too (`World::places`). The largest people on a land
  names it on first holding it, from its own words: for what the land is
  ("the hill"), what it is like ("the black hill", "the horse field"), as
  the place of something, or after the people ("the field of the Angles",
  only when that fits in four syllables). The name follows its holders'
  sound laws and stays frozen while no one lives there. It passes to
  another people only once that people outnumbers the namers twice over.
  A people speaking a daughter of the namers' language inherits it, a
  people that shifts language keeps it, and newcomers otherwise borrow it,
  fitted to their sounds (85% when they dealt with the namers, 40% when
  the namers were gone and unknown), or coin their own. Each land keeps
  every name it has had, with where each came from. Peoples living on or
  beside a land that speakers of another language hold hear its name
  once, fitted to their sounds (or as it is, if their language descends
  from the namers'), and keep it as a word of their own
  (`Variety::exonyms`): their sound laws change it and their daughter
  languages inherit it, as German *Mailand* came from *Mediolanum*.
- A language's known world (`World::known_lands`) is the lands it knows
  by name: those its speakers hold or held, those they border (unpeopled
  land they see gets a name of their own coining), the lands their
  contacts hold, and one more land each contact passes on per generation
  in each direction, from what the teller knew at the start of it. The
  first name heard is kept, even when it goes stale. A daughter language
  starts with what its parent knew when they parted, a people that
  shifts language keeps its old map of names, and a language that loses
  a land still remembers it. Nothing new is stored: the known world is
  read from `Variety::exonyms` and the land-name histories.
- Each continent gets one name on the chart (`World::continent_names`),
  from the speech of the first living people to know any of it: "the
  land of" a people or place, a quality such as "the wide land", or,
  for a continent known only from across the sea, "the far land". The
  name is fixed once written, as Herodotus kept *Asia* and *Libya*,
  whatever later becomes of the language that gave it.
- Varieties fork on splits and shifts and keep their lineage (`Fork`);
  `World::cognate` and `root_of` give true descent. The comparative method
  (`compare.rs`) must never read lineage; it is only graded against it.
- A people holds many lands (`Community::lands`, its heart first) and
  lives by a way of life (`livelihood.rs`): foraging, herding, or
  farming. Each sets how many a land feeds them (farmers on open plain
  most, about thirty times what foragers get there), how fast they grow,
  how readily they move, and how large they hold together. A founded
  people settles unpeopled land, likelier the more it feeds and the
  further it lies from other peoples, and lives as that land suits unless
  told otherwise. Plains normally farm, steppe and desert herd, and the rest
  forage. A river valley defaults to farming when its current yield is
  substantially better. Its number is spread over its lands by how much
  each feeds it, and it grows logistically toward that food, counting everyone
  living there. `World::feeds` reads one generation's cached climate and
  river yields, including `area_km2 / 8660.254`. The default capacity, 40,000,
  describes farmers on a reference-area plain. Unequal cells keep equal
  density, and larger boundary cells are not normalized away. When its lands
  hold more than they feed it, it declines, so foragers dwindle among
  farmers. A people using most of its lands' food spreads into bordering
  land with real room left (`spread_rate`). One too large for its way of
  life (`cohesion_size`) or spread too far from its heart
  (`cohesion_reach`) splits along its lands: the leavers take the
  furthest land and every land nearer it than the heart. A people on one
  land sends half its number to the roomiest bordering land, or, when
  that is full and it lives by the sea, along or over the sea, as Greek
  cities sent out colonies. A people on one land may also migrate whole
  (`migration_rate`): likelier the more crowded home is, the more mobile
  its land and way of life make it, and when a stronger people shares
  it. Migrants go to better land within reach, counting half of a weaker
  people's land as free, so strong peoples push into good land others
  hold. A stronger people that outnumbers another on one of its several
  lands crowds it off. Peoples learn a way of life that feeds them half
  again as well from those they deal with (`adoption_rate`); farmers on
  drying land may turn to herding. Foragers and herders on suitable plains
  or river valleys rarely begin to farm of their own accord.
- Authored settlement (`settlement.rs`) has three explicit intents. A
  territorial partition grows two fronts from the old and chosen hearts
  through the graph of held lands, assigning each land by travel effort.
  Both resulting territories stay connected to their heart; a disconnected
  component without either heart cannot silently change hands. People remain
  where they live, including residents of cities. Sending settlers takes a
  chosen share from every inhabited land into one reachable destination;
  moving whole retains the people's identity and language. Every source
  needs an actual walking route or a voyage their own seafaring permits.
  Destinations account for the people already living there, including the
  parent's residents who will stay. Preview is read-only; application repeats
  the same validation and records the census, routes, affected capitals and
  daughter identity. Shared division logic conserves city residents, updates
  their speech composition without advancing time, and records community
  ancestry independently of language descent. This authored planner is
  distinct from the automatic `leavers` behavior described above.
- Peoples suffer and end. Famine or plague strikes a peopled land now and
  then (`hardship_rate`), killing a share of everyone there, so a people on
  one land suffers worst. Drought instead lowers food until recovery; it
  does not also impose a random drought death toll. A people below
  a hundred dies out; one far smaller than a kindred people on its heart
  land may merge into it (`merge_rate`), and one swamped by a people of
  another family shifts to its language first. An ended people keeps its
  record and last lands but does nothing more, and its dealings end with
  it; when no people speaks a family's languages, the family is lost.
  Prestige comes from authored `power` plus relative size. A community
  shifts language only to another family's, keeping its own sound
  preferences and some old words as a substrate. Unspoken varieties are
  extinct and frozen.
- Contacts come and go (`end_contacts`, `make_contacts`). Each kind has a
  typical lifespan (trade 12 generations, rule 16, intermarriage 20,
  religion 30, distant neighbours 40) and cannot end in its first third.
  Rule lasts longer the further its ruler stands above the ruled.
  Ended rule and intermarriage leave neighbours only on shared or
  bordering land. Peoples on the same land
  stay neighbours. Two peoples have one contact at most: a new one between
  them replaces the old. The world makes contacts of its own: peoples on
  the same or bordering land become neighbours, more readily and more
  closely the easier the border is to cross; peoples open trade with
  reachable partners, nearer partners likelier; and a people far above
  one it deals with may conquer it when its own people can get there.
  Physical access is checked before stochastic turnover and after
  territorial change. An unreachable relation ends even with turnover
  disabled. Each beginning, ending, and conquest is a `WorldEvent`, so
  it is told in the history and stops "until something happens".
  `Params::static_society()` disables autonomous hazards, not physical gates.
- Response annals carry an optional `cause: { event, mechanism }` (revision
  31). `event` is the original position in this telling's `World::events`;
  its facade identity is `world:<event>`, including entries inside grouped
  neighbour accounts. It always precedes the response in event order and
  is no later in generation. `World::causes` stores only recorded links;
  `causes.rs` retains trigger identities where the corresponding state is
  changed. No facade search or adjacency rule supplies missing causes.
  This adds bookkeeping only: no draws, hazards, words, or event order change.
  - Migration records `stronger-neighbour` for a recorded meeting with a
    stronger, larger co-resident, or `climate` for a recorded climate
    transition still reducing its old land's food. Displacement records
    the meeting with the stronger people actually displacing it.
  - Automatic splits depend on size and territorial reach, not on a
    hardship or climate event, so have no cause. Crowding without an event
    also remains unlinked. Famine and plague reduce population, not food
    capacity; attaching them to migration or splitting would claim a
    decision the engine does not make.
  - Livelihood change records `climate` only when recorded weather reduces
    the old livelihood's yield and the new livelihood would not meet the
    adoption gain at baseline. Learning a craft does not change these
    yields or the adoption decision, so no `craft` link is emitted.
    Learning a livelihood from a neighbour or finding it independently
    without that climate trigger remains unlinked.
  - State rise and collapse record `hardship` for recent famine or plague,
    or `climate` for ongoing or remembered climate pressure. These use the
    same six-generation challenge window as the decisions. Rise under
    `stronger-neighbour` names that state's recorded rise or latest
    conquest. State rise or fall caused by conquest records `conquest`,
    including an authored rule-contact annal that imposed that conquest.
    Comfort, unrecorded crowding, rulers ending, and capital loss do not
    acquire an inferred cause.
  - Standards and koinés record `city` with that city's threshold annal.
    A court-only standard without a recorded city has no city cause.
  - Conversion records `contact` from the relationship actually selected
    at the decision point, including a rule relationship established by
    conquest. A pilgrim-created relationship records `pilgrimage` only if
    that exact route has a recorded pilgrimage; otherwise its recorded
    meeting remains the cause. Implicit links, direct authored conversion,
    and a schism's reformer or statewide conversion do not gain inferred
    meeting links. Joining a schism through a selected contact keeps that
    contact's cause when its beginning was recorded.
  - A conquest whose hazard includes holy-land pressure records
    `unfaithful-holder` only when the shrine's recorded allegiance change
    names the actual target. An unrecorded change between unfaithful
    holders clears that link. Ordinary conquest stays unlinked.
  The shared mechanism vocabulary reserves `crowding` and `craft`; neither
  currently has a recorded trigger for these responses. Empty causes are
  omitted from JSON rather than serialized as null.
- States (`polity.rs`) sit above peoples: a ruling people, the peoples it
  rules, a capital land, and a name coined from the rulers' name with
  their belonging affix ("the realm of the Ivo"), which changes with
  their speech while the state stands. Every subject has a rule contact
  with its rulers, so borrowing, waves, and shift work through rule as
  before; when that contact ends, the subject has thrown off the rule. A
  conquest, by the world or an author's rule contact, brings the ruled
  into the rulers' state, raising one if they have none, and conquered
  rulers lose their old state. Each former subject joins the conqueror
  only if the new ruler can reach it; the rest become independent.
  Subjects make no conquests. A split-off people stays in its parent's
  state only when the actual ruler can reach the daughter. A large
  farming people under no state may organize itself into one
  (`state_rate`): readily in answer to a challenge (bad times on its
  lands, being crowded off land, a stronger state beside it), a tenth as
  readily in comfort. This is Toynbee's challenge and response. Ruling
  provides a food budget: a tenth of the rulers' own lands' yield and a
  tenth of their subjects' number in tribute. The court uses that budget
  until townsfolk have their own community; their food then comes out of
  the same budget, not a second addition, and townsfolk pay no circular
  tribute to themselves. A state falls when its rulers end or lose the
  capital, or collapses (`collapse_rate`), three times as readily after
  bad times on its rulers' lands; states last about 750 years. Rule holds
  half as long after such bad times. A state with a city of 10,000 that
  has stood 200 years may take its townsfolk's speech as its standard,
  or the court's if no koiné has formed. The selected people is recorded
  separately from the rulers, so the standard's prestige, borrowing,
  waves, shifts, spelling, and classical form all follow the selected
  speech. A standard takes up sound laws, waves, and new words at half
  the pace; its words carry extra prestige; its sound changes and words
  reach the kindred speech of its subjects three times as readily, even
  basic words (dialect levelling); its subjects of other families shift
  to it twice as readily; and rule under it holds half again as long.
  Each state draws a purism at its rising, most near none: a purist standard borrows
  less, its loans lose ground in use, and concepts held by loans gain
  native words. When a state falls the pull stops, and its dialects
  drift apart again.
- Cities (`cities.rs`) first enter the record at 10,000 people, once per
  state. Migrants come from rulers and subjects, weighted by their number
  divided by one plus travel distance to the capital; `city_rate` closes
  a quarter of the gap to that makeup per generation. Before townsfolk
  become their own people, residents remain explicit subsets of their
  source communities' census, capped at half of each source. `presence`
  subtracts them from the source's ordinary lands and places them at the
  capital: showing a city never adds a second population. These subsets
  share their source people's births, deaths, and language shifts.
  Four consecutive generations with two speech varieties each at 15%
  make the townsfolk a normal community on the capital land. Formation
  debits the reserved people once from their sources; later immigration
  also debits its sources. The people names itself for the capital using
  `Naming::Land`. Ordinary contacts, waves, borrowing, demographic change,
  and shifts then work without a separate urban language engine.
- A koiné descends from its largest contributor and records all its
  formation shares (`Variety::koine_of`). For each core concept,
  contributor share times word usage votes for a word, cognates voting
  together; the strongest contributing form in the winning group wins.
  Contributors' inventories determine regular minority-sound mergers:
  sounds present among less than half merge into the nearest sound
  present among at least half, within consonants or vowels. If no such
  target exists the segment stays; vowel and minimal-word guards still
  apply. Every living word and name undergoes the same `koine-levelling`
  law, recorded with before-forms and the variety's exact merger pairs.
  This makeup-dependent law is not a portable catalog wave; subsequent
  ordinary laws are. The parent's morphology stays, except optional
  derivational affixes/patterns used by fewer than three living words
  and supported by less than half the contributors are dropped.
  Basic patterns, name builders, and renewal remain available.
- Urban diffusion keeps rural contact weights unchanged and multiplies
  waves involving townsfolk by `1 + min(3, (A/10000)(B/10000)/(1+d)²)`,
  where A and B are the two populations and d is travel effort.
  Great cities also open direct trade contacts with a bounded gravity
  probability, so a wave can jump between cities without passing through
  every intervening rural variety. Routes use a purpose-keyed `city`
  stream; koiné levelling itself has no random draws. When a state falls,
  its food budget ends. A city's remaining townsfolk disperse a fifth of
  their excess above 2,500 each generation into surviving former members
  of the state, conserving population; ordinary land capacity and
  hardships still determine survival. If no receiving people remains,
  ordinary demography alone governs the town. A community that has left
  the capital is no longer counted as that city's residents.
  `Params::static_society()` disables city formation and migration.
  In the 40-seed, 4,000-year band (`cities -- --band 40 160`, the audit's
  three 1,000-person founders with default rates), 8/40 worlds formed a
  koiné (20%); the median was zero koinés per world, with ten overall.
  Two of those ten became standards (20%), in two worlds.
- People have given names (`names.rs`). Each language keeps a stock of
  eight in fashion, built from its own words in its own style: one word
  ("Wolf") or two joined ("Wulf-stan", as Germanic, Slavic, and Greek
  names were). The meanings are weighted by way of life and ethos:
  herders name children for horses, cattle, and spears, farmers for grain
  and fields; martial peoples favour weapons and fighting, pious peoples
  their gods. Names go out of fashion and new ones come (`name_turnover`);
  sound laws change them like any word. A people of a founded faith names for
  its god four times as often (theophoric names, as Theodore), and
  converts take names from the sacred language. Founders of states and
  faiths are drawn from their people's stock.
- A way of life shows in words (`ideas::LIVING_RELATED`). Herders stretch
  "cattle" to cover wealth (Latin *pecunia* from *pecus*) and "herd" to
  cover priest (Latin *pastor*); farmers stretch "seed" to cover child
  and "till" to cover worship (Latin *colere*, whence *cult*). Such an
  apt stretch takes half the meaning's use at once.
- Ideas (`ideas.rs`) are crafts and founded faiths. Each brings meanings
  a language has no word for until its people holds the idea: iron,
  bronze, and smith with metalworking; ride and saddle with riding; sail
  and ship with seafaring; write, book, and letter with writing; holy,
  sin, temple, prophet, heaven, demon, and sorcerer with a faith; till
  with farming and herd with herding. Gods, priests, chiefs, and markets
  every language has. When the idea arrives the language finds a word:
  it borrows the teachers' word, stretches one it has ("write" from
  "scratch", "book" from "tree", as English and Latin did), builds one
  from its own parts, or coins one. Open peoples borrow; purist
  standards build.
- Crafts pass along contacts (`idea_rate`), most readily with traders
  and rulers, and writing with priests. A people may also come upon one
  itself (`craft_rate`, scaled per craft), but only where it could begin:
  metalworking among farmers or herders with hills or mountains on or
  beside their lands, riding among herders on or beside the steppe,
  seafaring on the coast, and writing at the court of a state whose city
  holds 10,000. Over many seeds, seafaring begins near year 1,400,
  metalworking near 2,900, and writing a few centuries after the first
  great city; riding begins only in worlds with steppe herders.
  Metalworking, and riding for herders, add prestige in war; riding
  carries a people further and holds it together over more land; only
  seafarers migrate or send colonies over the sea. An inland holder
  cannot borrow another people's port, and a subject's ships do not
  supply its ruler's navy.
- Writing fixes spelling. From the generation a language is first
  written its words keep the spelling they had, while sound laws go on
  changing speech, so spelling falls behind (as English *knight*). A
  written standard changes more slowly, and after six laws it may reform
  its spelling to fit speech again.
- Founded religions (`ideas::Religion`) begin among subjects of 5,000 or
  more in a state that has stood eight generations, readily in a time of
  troubles (bad times on their lands or their rulers', or being crowded
  off land) and a tenth as readily in quiet (`religion_rate`): Toynbee's
  universal church. An author can found one among any people. The
  founder's speech, frozen as it stood, becomes the
  faith's sacred language, a variety of its own that keeps its prestige.
  Each faith reveres one place (`Religion::shrine`), chosen from lands its
  founders knew: their home (weight 3), a mountain (3), an island (2), or
  a shore on another continent (2), its name frozen in the sacred
  language. Converts learn every shrine's name.
  Most faiths seek converts and spread along contacts
  (`conversion_rate`); a third translate their words, the rest keep the
  sacred language and lend from it, so converts gain learned doublets
  beside their inherited words (as English *fragile* beside *frail*). A
  faith whose teaching is written brings writing to half its converts.
  Conversion changes old words: converts to a faith that keeps its
  sacred language may turn their old god into a demon and their old
  priest into a sorcerer (pejoration, as Greek *daimōn* and Persian
  *magus*), and a founder's reform may do so to his own people, leaving
  god and demon as cognates across languages (Sanskrit *deva* "god",
  Avestan *daēva* "demon"). Iconoclasm is not yet modelled.
  Faiths divide (`schisms.rs`, `schism_rate`): across landmasses or states
  separated for sixteen generations (`Distance`), when rulers take a
  church centred in another state (`Rule`), when core-vocabulary sharing
  with the sacred language falls below 0.55 (`Reform`, the same
  chance-corrected phonetic measure as `keptFromHigh`), or in the first
  six generations after a founder (`Succession`). Succession has 0.12
  of the usual opportunity; it cannot recur among a branch's followers.
  A branch is a faith with a parent and a split generation, inheriting
  the founder, sacred speech, shrines, and doctrine. Reform translates;
  distance gives a 70% chance of revering another known place on the
  schismatics' own lands, chosen with the founding shrine weights.
  Branch names are coined in their own speech, after a given name, a
  homeland, or an old/new epithet on the parent's name. Rule takes every
  follower of the parent in the rulers' state; others join with a chance
  proportional to contact intensity and linguistic kinship, capped at
  0.85. A faith family rests twelve generations between splits, and can
  have at most six descendants; each existing descendant reduces the
  per-faith hazard by a divisor of `1 + 0.4 × descendants`. Related
  branches remain rivals, but conversion is twice as likely between
  them as between unrelated founded faiths.
  Pilgrim roads begin with probability `pilgrimage_rate` (0.02) per
  generation for every follower and shrine within 1,200 effort-km.
  They follow the cheapest permitted directed journey from a held land
  to the exact site. A boat journey requires the pilgrim's own Seafaring
  and coastal endpoints, with no borrowed port or inland leg.
  Roads persist only while their departure land remains held, their
  people follow the faith, and their recorded mode stays reachable.
  Pilgrims open Religion at intensity 0.3 with a shrine holder, weighted
  by population at that site. Existing contacts stay unchanged.
  Contact creation uses the pilgrimage hazard even on an existing road.
  Ordinary contact turnover and physical reconciliation still apply.
  Borrowing and waves use those contacts without a separate mechanism.
  Each faith records its first overseas pilgrims from each landmass.
  A holy land's holder is its largest local population (lowest community
  id breaks ties), not its namers or political overlord. Changes between
  holders of the shrine's exact faith and other holders, including no
  holder, are recorded in both directions. The conquest comparison in
  `make_contacts` has twice its usual hazard, capped at 1, against an
  unfaithful holder of an attacker's shrine or an ancestral faith's shrine;
  co-faithful targets never receive this bonus. This scales the existing
  comparison without adding draws. Schisms and pilgrimages use independent
  purpose-keyed streams. `static_society` disables both and holy-land events.
- Diglossia (`diglossia.rs`): a state's written standard is fixed as a
  classical form, a frozen copy pushed as a variety of its own, either by
  grammarians once it has been written twelve generations (a tenth
  likely each generation after) or when its state falls after six
  generations of writing. Its speakers and the speakers of every language
  descended from it then write the classical form, not their speech:
  their speech counts as unwritten, changes at the ordinary pace again
  (the standard's brake moves to the frozen form), and borrows learned
  words from the classical form, which stands above any people that
  learns from it (doublets, as French *fragile* beside *frêle*). Readers
  of other languages that deal with them borrow from it too, half as
  closely. The classical form outlives its state. A language is written
  in its own right again (`Variety::vernacular`) when a state takes it as
  its standard, when a faith that translates its written teaching reaches
  its speakers, or when a founder's written teaching is in it; the
  classical form then lends to it half as closely. Over many seeds most
  worlds fix a classical form, the first near year 4,450.
- Social identity, territory, ancestry, and language are independent.
- Every random draw comes from a ChaCha8 stream keyed by purpose
  (`rng.rs`), so adding a process never shifts existing draws.

## Worldview

`ethos.rs` gives each people six heritable axes in [-1, 1]: `martial`
(peaceable to martial), `open` (insular to open), `pious` (worldly to
pious), `hierarchical` (egalitarian to hierarchical), `roving` (rooted to
roving), and `seaward` (landward to seaward). These are biases, not fixed
civilizational stages. The author's existing `power` and `openness` remain
fixed bases; ethos multiplies rates on top, never replaces those bases
or derives its priors from them.

Founding priors, before independent uniform noise in [-0.3, 0.3):

| Way of life | Martial | Hierarchical | Roving |
| --- | ---: | ---: | ---: |
| Foraging | -0.1 | -0.3 | +0.15 |
| Herding | +0.3 | 0 | +0.4 |
| Farming | 0 | +0.3 | -0.35 |

Open and pious start at zero before noise; seaward starts at +0.4 on a
coast, -0.25 inland. Founding actions may supply `ethos`, a partial object
of any of the six axes; omitted axes retain their draws. Explicit values
must be finite and in [-1, 1]. Draws are keyed by `ethos`, community, and
axis, independently of language generation. Given-name weights read ethos
at founding as well as when new names enter fashion. Existing martial
name meanings are spear, shield, war, fight, and bow; there is no wolf
concept in the lexicon.

Every generation, contacts pull each axis simultaneously toward their
partners, weighted by intensity times `(0.5 + partner prestige)` times
kind: intermarriage 1, rule 0.9, religion 0.6, neighbours 0.45, trade 0.2.
The update closes at most 2% of the weighted gap (a maximum absolute
change of 0.04), scaling down when total contact weight is below one.
Separate `temper` streams add independent uniform drift in [-0.012, 0.012)
per axis per generation. A split copies its parent's ethos with uniform
drift in [-0.04, 0.04); leavers settling elsewhere add +0.06 roving.
Language shift keeps ethos. Townsfolk inherit the population-weighted
mean of the actual residents contributing to the koiné, before those
residents are debited from their sources.

Challenges and responses add small, clamped steps:

- Hardship: +0.06 pious; -0.04 roving, or +0.06 roving when the people's
  lands were at least 90% full before that generation's hardship.
- Throwing off a rule contact: +0.12 martial for the former subjects.
- Ruling at least one subject for twelve generations: +0.004 hierarchical
  per generation thereafter.
- Twelve generations without a challenge: -0.003 martial and -0.003 pious
  per generation. Hardship, displacement, overcrowding, and ongoing rule
  restart the quiet-span clock.
- Learning seafaring: +0.12 seaward.
- Founding a faith: +0.12 pious; conversion: +0.08 pious.

A leaning becomes notable when an axis reaches ±0.5 and stays so until it
falls back inside ±0.35, so drift around the line is not retold. Each
change records `WorldEvent::Temper` with axis, pole, whether the people
entered or left it, and cause; a jump from one pole to the other records
the leaving first. Founding leanings are marked without an event, and a
daughter starts from its own marks before its inheritance drift. Causes
are `drift`, `inheritance`, `contact`, `hardship`, `freed`, `long-rule`,
`comfort`, `seafaring`, `faith`, and `fate`. The `Temper { community,
axis, amount }` recipe action nudges rather than overrides; its signed
amount must be finite and in [-1, 1], and the people must still live.
Annal kind `temper` exposes `{ axis, pole: "high" | "low", entered,
cause }`, and other annals expose `temper: null`. End-of-generation ethos
snapshots use at most one 28-byte entry per changed generation per
people; `Community::ethos_at` supports historical views and ended peoples
keep their final ethos.

Every effect uses one arithmetic multiplier `clamp(1 + k * axis, 0.25, 2)`:

| Use | Axis | k |
| --- | --- | ---: |
| Conquest hazard | martial | 0.5 |
| Trade formation, including city trade | open | 0.5 |
| Borrowing, retention of foreign sounds, craft-word borrowing | open | 0.5 |
| Purism drawn when a state rises (result capped at 1) | open | -0.5 |
| Conversion, faith founding, pilgrimage | pious | 0.5 |
| Schism hazard (population-weighted mean among followers) | pious | 0.5 |
| State formation, standard adoption (rulers' ethos) | hierarchical | 0.5 |
| Migration, territorial spread | roving | 0.5 |
| Cohesion reach | roving | 0.35 |
| Seafaring invention, overseas colony attraction | seaward | 0.75 |
| Given-name meaning weights | relevant axis | 0.75 |

Intermarriage remains author-created: there is no automatic formation
rate to scale; its borrowing and acculturation use ethos normally.
Colonies likewise have no probabilistic formation rate. Seaward multiplies
only overseas destinations' existing room/distance score and their room
comparison against home; land-reached alternatives and travel stay unchanged.
`Params::static_society()` freezes ethos, including inheritance drift,
challenge responses, and authored nudges. `Params { ethos_enabled: false,
..Params::default() }` draws only zero ethos and disables changes, for a
strict baseline. With `ethos_shifts: false` separately, authored zero
axes exercise every multiplier as exact floating-point 1.

In the 40-seed, 4,000-year band (`ethos -- --band 40 160`), all worlds
completed. Each entry below is pooled standard deviation across living
peoples / mean of each world's within-world standard deviation:

| Setup | Martial | Open | Pious | Hierarchical | Roving | Seaward |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Three-founder | .1623 / .0929 | .1531 / .0802 | .1951 / .1125 | .2297 / .1122 | .2183 / .1229 | .2388 / .1199 |
| Sample recipe | .1805 / .1035 | .1608 / .0798 | .1781 / .1151 | .2626 / .1424 | .2274 / .1494 | .2416 / .1262 |

Temper events per world: three-founder mean 3.6, median 3, range 0–15
(144 total); sample mean 4.9, median 5, range 0–15 (196).
Event means per world, launch HEAD baseline → ethos:

| Setup | Conquests | Migrations | Schisms |
| --- | ---: | ---: | ---: |
| Three-founder | 2.350 → 2.025 | .625 → .775 | .025 → .025 |
| Sample recipe | 1.275 → 1.300 | .275 → .400 | .225 → .250 |

These measurements precede the physical-travel cutover. The baseline was
built separately at `616cc3c`, without ethos. Neutral `audit 3 160`,
`history 42 160`, and `cities 42 160` reports were byte-identical to that
HEAD, including names and event ordering (11,044, 19,072, and 3,369 bytes).
The examples now place authored-contact founders within physical reach.
Their `--neutral` switch still disables ethos, not the newer geography.

## Physical travel and map scale

Walking traverses land only. Sea endpoints are unreachable even for
identity. A voyage starts at held coastal land, ends at a coast, and
has at least one sea cell. It never passes through another coastal port. Each
embarkation or landing adds 100 effort-km. Same-continent voyages are
valid. There are no rented ports, mixed inland-and-sea legs, fleets,
travel durations, or globe wrapping.

`World::journey_to` and `journey_between` are directed: only the
traveller's own Seafaring enables a boat journey. Walking wins a tie.
`apart` is symmetric, so either participant may provide transport for
trade, intermarriage, or religious contact. Rule always requires the
actual ruler's directed access. `World::connect` returns a refusal before
changing contacts, states, or history when a requested relation is
physically impossible. Authors bypass chance, not reachability.

Default effort-km parameters are founding separation saturation 800,
migration 600, colony 1,200, trade 1,800, conquest 800, pilgrimage 1,200,
and cohesion reach 300 before mobility. Pilgrimage rate is 0.02.
Founding is independent settlement and needs no ships. Spread remains
land-adjacent and uncapped. Cohesion and territorial partition use exact
walking distances without a cache ceiling. Distance preferences use
`1 + effort / 100`, so changing units does not change reference-grid odds.
The colony ceiling intentionally rises from the old eight steps to
twelve, while embarkation adds overhead. On reference plain shores,
three intervening sea cells cost 1,200 effort-km rather than the old 1,000.

The facade's pure year-zero `founding_preview` reports each pair's exact
walking and coast-to-coast voyage effort, retaining routes beyond first
contact range. `neighbours` means `World::nearness > 0`, the shared/bordering
land predicate used by `make_contacts`. Otherwise, `walking` means a land
journey within `Params::trade_reach` (1,800 effort-km by default); `sea`
means a voyage within that same merchant range when walking is not within
it; `apart` means neither first journey qualifies. A voyage remains only
a possibility until someone learns Seafaring. Coastal status, homeland
landmass, and the nearest other founder accompany the pairs; nearest means
the smallest finite walking or voyage effort, breaking ties by community ID.
This preview does not predict later territorial spread or consume draws.

The immutable map stores sparse walking and voyage rows out to 1,800
effort-km. Missing cached destinations are outside a bounded query,
not necessarily unreachable. Exact pair queries use Dijkstra beyond
the cache, and uncapped consumers reuse full source rows. Authored
larger reaches use phase-local rows without mutating the shared map.
Spatial and contact indexes are derived per phase. Successful spread
redistributes all old holding presence; later migrants see earlier
migrants' consumed room. These indexes are not additional replay state.
Urban residence adds population at a city, not a territorial holding or
borrowable port. The phase indexes distinguish residents from land holders.

The fixed drawing scale gives these rectangular extents:

| Size | Regions | Land regions | Approximate extent |
| --- | ---: | ---: | --- |
| small | 63 | 31 | 950 × 620 km |
| middling (`medium`) | 130 | 65 | 1,350 × 879 km |
| wide (`large`) | 252 | 126 | 1,850 × 1,226 km |
| vast | 3,600 | 1,800 | 6,050 × 5,210 km |

Vast has two continental bodies of at least 800 regions each and
exactly three one- or two-region islands. It is a large flat theatre,
not a planet. The facade exposes `catalog.mapSizes`, `map.kmPerUnit`,
`Region.areaKm2`, and physical landmass metadata while retaining drawing
units. Movement views use the event's recorded `bySea`, not the shape
of the present coast.

## Rivers and regional climate

`rivers.rs` builds a coast-inward priority flood. Each land drains through
one shared-border neighbour toward the sea, staying within its landmass.
Enclosed hollows receive a lowest-saddle spill route without changing their
visible elevation or terrain. Region ids break ties, and the upstream order
cannot loop. Runoff accumulates physical wet catchment area, with a separate
seeded local variation. A reach becomes a river at about 25,981 wet km²,
not at a fraction of the map. Vast therefore has more catchments and rivers,
not smaller cells or a river in every cell.

Every river has a stable id, an ordered main course, an ultimate sea mouth,
and its upstream catchment, the land supplying its water. The strongest
branch continues the main course;
other branches keep their own ids and join it. Shared downstream reaches are
stored once. Tributary and main-river catchments may overlap because water
from the tributary supplies both. Short tributaries remain short, and great
courses can continue for many regions where a landmass permits.

Water gives an immediate, bounded, additive floodplain farming benefit,
including in steppe and desert. Foragers gain fishing and gathering food,
and herders gain less. Hills receive a smaller farming benefit, and mountain
streams do not become grain plains. Flow depends on upstream rain, so a
locally dry valley can be a refuge until its wider catchment dries. Cold
still limits farming beside a flowing river. There is no irrigation craft.

Drainage-connected, usable river reaches multiply the canonical walking
border effort by 0.65. Adjacent unrelated rivers receive no discount.
The same costs feed distance, border closeness, migration, cohesion, trade,
and sound waves. Flow crossing about 17,321 wet km² changes usability;
only such crossings rebuild sparse walking rows. Newly inaccessible contacts
end before the generation's loans or sound waves can use them. Sea journeys
retain their own costs and permissions. No river grants ships or seafaring.

`climate.rs` gives connected land zones, usually 6–20 regions, a shared
seeded history. Islands and narrow leftover pieces can be smaller.
Wetness and warmth targets are anomalies from local baselines. Epochs last
several generations; conditions move one quarter of the remaining gap
toward a target each generation. Short droughts, cold spells, successive
long drying, and recovery occur on occupied and unoccupied lands alike.
Local baseline rain and warmth keep dry interiors and highlands distinct.
Courses, sea, relief, and landmasses remain fixed.

Climate advances before growth. It caches flow, effective vegetation, and
all three livelihood yields once for the generation. Growth, presence,
settlement, adoption, spreading, and migration use this same food. Farming
responds more strongly to lost rain and cold than foraging; water adds food
rather than multiplying the desert's small farming baseline. Vegetation
also determines whether riding has a steppe opportunity. Metalworking
still needs relief, and writing still needs a large court city.

Climate onset, worsening, recovery, and river flow changes record causes,
affected lands, and exposed peoples. Food loss is not a claimed casualty
share. Severe ongoing stress and six generations of remembered exposure
count as `HardTimes`; migrants and daughters keep that exposure. Affected
rulers also face the existing weaker-rule and collapse paths. State size,
challenge chances, city thresholds, tribute, and split rules are unchanged.
An automatic capital is the ruler's best-fed held land, with usable river
access breaking a food tie; an authored capital is respected.
`Params::static_society()` freezes climate through `climate_enabled = false`.
Authors cannot currently impose climate targets as replay actions.

A hydronym is a river name. `river_names.rs` reuses `Name`, `PlaceName`,
and their origin records. The first settled speakers coin it from river,
water, and descriptive words. Speakers along connected courses hear
local forms without merging tributary identities. They inherit, borrow,
and retain those forms through language shift. Regular sound laws change
spoken local names and remembered forms, while deserted attestations
freeze. Nearby land and departing peoples can be named for a real river.
The facade exposes static river topology and zone ids in `map()`,
historical conditions and literal feeding capacities in `climate(generation)`,
and name histories with local alternatives in `river(generation, id)`.

The workbench reads those views within the active telling and year. Rivers
are inked above terrain, with three catchment-width tiers and dashed failed
flows. Tributaries meet their parent course; main rivers meet the shared
coast of their sea mouth. The read-only `joinAt` field names the exact
confluence region; a tributary can border several reaches of its parent.
Close zoom adds attested names in the selected
people's speech, falling back to the mouth's speakers. River cards retain
name records, exonyms, and flow annals. Weather colouring uses zone departures,
and climate annals lead to zone cards. Far, middle, and close chart views
disclose progressively finer detail; a kilometre scale follows the camera.

## Studying one mechanism

Each example prints a readable report; profiles are preset ids
(a flavor such as `familiar`, `germanic`, `semitic`, `polynesian`).

```sh
cargo run --release -p umran-sim --example found -- <seed> <profile>
cargo run --release -p umran-sim --example drift -- <seed> <profile> <generations> [flavor...]
cargo run --release -p umran-sim --example contact -- <seed> <donor> <recipient> <kind> <generations> <seeds>
cargo run --release -p umran-sim --example family -- <seed> <proto> <outsider> <generations>
cargo run --release -p umran-sim --example history -- <seed> <generations>
cargo run --release -p umran-sim --example calibrate -- <seeds> <generations> [profile]
cargo run --release -p umran-sim --example calibrate -- geography [seeds] [generations] [size] [founders]
cargo run --release -p umran-sim --example rivers -- [seeds] [size] [generations]
cargo run --release -p umran-sim --example length -- [seeds] [generations]
cargo run --release -p umran-sim --example audit -- [seeds] [generations]
cargo run --release -p umran-sim --example faiths -- [seeds] [years] [first-seed] [seeded|natural|sample|sample-unseeded]
cargo run --release -p umran-sim --example stress -- [seed] [generations]
cargo run --release -p umran-sim --example cities -- [seed] [generations]
cargo run --release -p umran-sim --example cities -- --band 40 160
cargo run --release -p umran-sim --example ethos -- [seed] [generations]
cargo run --release -p umran-sim --example ethos -- --band 40 160
cargo run --release -p umran-sim --example endings -- [seeds]
cargo run --release -p umran-web --example provenance
```

The faith report's `seeded` setup begins with three peoples sharing one
authored faith; everything thereafter uses default parameters. Over seeds
0–39 for 4,000 years it gives 69 schisms (median 2, maximum 4 per world
and per faith family), none by succession, and holy-land changes in 32
worlds (80%). The 2× holy-war bonus applies to 824 eligible conquest
comparisons. These measurements precede physical travel and the revised
workbench sample. `natural` leaves that starting faith out.
The earlier `sample` band had 9 schisms (median 0, maximum 1), six
unseeded faiths, holy-land changes in 19 worlds, and 46 holy-war checks.
Its `sample-unseeded` control had eight unseeded faiths and two schisms.
The current `sample` follows the workbench's fixed founding lands, early
sea realm and writing, and authored conquest, faith, and shift at year
2,500. `sample-unseeded` omits only that authored faith. Other map seeds
can refuse this fixed recipe; the report counts those refusals separately.

## Not yet modelled

Fleets, rented ports, mixed inland-and-sea itineraries, exact river channels
and lakes, seasonal weather, resolved travel times, globe wrapping,
purism within a classical form,
compounding and derivation after founding beyond renewal and new meanings,
inflection beyond count noun plural and object marking and verb past,
productive root-and-pattern inflection, agreement, grammatical gender,
tense beyond past, pronoun paradigms, genitive marking, tone, vowel harmony
beyond next-syllable umlaut, prenasalized stops, syntax beyond fixed word
and possessor order, alignment beyond this object contrast, and doctrinal
detail beyond the causes of a schism.

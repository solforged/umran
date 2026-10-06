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
This bookkeeping makes no random draws and does not itself change replay.

- `World` (`world.rs`) steps communities, varieties, and contacts through
  25-year generations. `Variety` holds a `SoundProfile` and a `Lexicon` of
  `Slot`s, where words compete for each concept with usage weights. Words
  keep a log of every sound law, borrowing, extension, and loss.
- The map (`geography.rs`, `sphere.rs`) is a closed spherical surface drawn
  from the world seed at a chosen physical size (`MapSize`). Regions are
  dual cells of a subdivided icosahedron, with shared borders across the
  chart seam and around both poles. Canonical unit-vector centres and
  boundaries determine spherical area, drainage, landmass anchors, and
  great-circle travel. Terrain is sea, plains, forest, steppe, hills,
  mountains, or desert. `ContinentalV6` starts with one supercontinent and
  moves its crust through ten rigid-plate drift steps on a fixed 10,242-point
  icosphere. Rifting, subduction, continental collision and mantle hotspots
  leave the coasts, ranges, shelves and islands.
  Latitude and elevation affect warmth; moisture varies with latitude,
  relief, and proximity to sea, with rain shadows downwind of high ground
  under easterlies below 30° and westerlies above. Climate remains a static
  analogue, while the new surface records a short moving-plate history.
  Each connected land body has one `landmass`; sea has none. Bodies of at
  least 500,000 km² are continents. V5 and later small worlds use 125,000 km²
  because their sphere has one quarter the area of medium worlds. Founders
  prefer continents to islands. V1–V5 retain their original surfaces,
  streams, wind fields, and physical-area classifications.
  Shared-border great-circle midpoints give centre-to-centre routes.
  Terrain multiplies their physical length to give effort-km, equivalent
  plain kilometres: a 100 km mountain crossing costs 400 effort-km.
  Spherical area sets regional food capacity. Drawing coordinates remain
  derived equirectangular map units, at 100 km per unit along the equator;
  their lengths and areas never measure the simulation. Portable `libm`
  trigonometry keeps native and WASM geography identical. Static drainage
  supplies named rivers, and connected climate zones supply weather
  histories without redrawing the coast.
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
  and verb past, future, and progressive. The object is the thing acted upon:
  an accusative-like contrast, not a complete case system. Count nouns name separate things,
  so water, fire, and collective people take neither plural nor object
  forms. Eligibility is explicit for each concept and follows a word's
  current senses, not only its first meaning. Designs save each category
  as suffix, prefix, particle, or none. A suffix follows its stem; a prefix
  precedes it; a particle is a separate grammatical word. Missing plural
  and past choices in older designs remain suffixes; missing object, future,
  progressive, word order, and possessor choices are drawn at founding.
  Affix-building profiles draw plural and past bound marking, particles, and no marking at
  weights 0.6/0.3/0.1; root-and-pattern profiles use 0.5/0.4/0.1. Object
  marking uses 0.45/0.15/0.40 in both. The suffixing preference divides
  bound choices between suffixes and prefixes.
- Future places an event after the present; progressive presents an event
  as ongoing rather than completed. Both use the existing verb eligibility,
  including verbs whose ongoing reading depends on context. Future particles
  come from go, come, or have (motion and obligation); progressive particles
  come from stand (posture). These are existing meanings, so no lexical
  founding draws move. The source word keeps its ordinary meaning and the
  auxiliary has a separate sound history. This follows the pathways described
  by Bybee, Perkins, and Pagliuca, *The Evolution of Grammar* (1994), and the
  grammaticalization cycle in Hopper and Traugott, *Grammaticalization*.
  Founding bound/particle/none weights are 0.30/0.30/0.40 for future and
  0.25/0.35/0.40 for progressive, in either morphology family. These are model
  priors, not measured language frequencies. A language may have no dedicated
  mark and use context instead.
- Future and progressive auxiliaries enter, compete, fuse, erode, and renew
  through the same machinery as past. Loss raises the per-generation
  rebuilding chance from 0.0007 toward 0.0087. A new particle takes an initial
  share of 0.12 beside the old form; fusion takes 0.18. The old ending can
  survive on some verbs while the new auxiliary spreads. These are usage
  shares, not distinct modal meanings: narrower future senses, separate clitic
  stages, and combined tense–aspect forms are not modelled. The new categories
  have their own category keys within each grammar purpose stream; earlier
  categories' draws are not consumed by them. `Params.tense_aspect` is true by
  default and false in `static_society()`: turning it off stops their renewal,
  competition, fusion, and analogy, not founding choices or regular sound laws.
- The genitive marks a countable possessor, as English -'s does. It is a
  category like the object, drawn at founding as bound, particle, or none
  at 0.45/0.35/0.20, and it grammaticalizes from have or hand and erodes
  through the same marker machinery (Heine and Kuteva, *World Lexicon of
  Grammaticalization*).
- Pronouns (`pronouns.rs`) are lexicon words: first, second, and third
  person, singular and plural; respectful address may add a polite
  singular. A separate founding stream gives 31.5% of
  systems an inclusive/exclusive first-person plural distinction, replacing
  unmarked "we" with "we including you" and "we excluding you". This follows
  the 63 of 200 languages in [Cysouw, WALS 39A](https://wals.info/feature/39A);
  the five inclusive-only systems are not counted as the full opposition.
  The choice is inherited, not redrawn when a people splits. Founding gives
  each cell a short syllable of common segments from the language's own
  phonotactics, distinct from the others. They then change by regular sound
  law like every word, and both plural cells take part in renewal. Contact
  copies only cells that both languages express. A pronoun
  worn below two segments or the minimal word, or merged in sound with
  another, can be renewed from a noun (person, head, heart, child; people
  for plurals), as Malay saya, "I", came from a word for "servant"
  (`pronoun_rate`, 0.003 a generation). A people ruled by a state for eight
  generations can extend plural address to a separate respectful singular
  cell, six times as often, with the state's rise as its cause, provided
  its plural "you" still sounds unlike its familiar singular. Familiar
  singular remains alive alongside it: the T–V distinction of French
  tu/vous (Brown and Gilman, 1960;
  [Helmbrecht, WALS 45](https://wals.info/chapter/45)). The respectful cell
  initially shares the plural lexeme, including its regular sound changes;
  later renewal can distinguish it. Only after at least 24 generations
  (600 years) of coexistence can respectful address generalise, with a
  separate per-generation chance of half `pronoun_rate` (0.0015 by default).
  This removes familiar address and retires the separate polite cell,
  recording a distinct notice, as English you displaced thou. The waiting
  period and transition rate are model assumptions, not WALS estimates.
  Generalisation has its own `pronoun address generalisation` stream and
  remains possible after the court falls. Under rule or intermarriage
  at intensity 0.9 or more, after eight generations of contact, a very open
  people may borrow a pronoun, third-person plural most readily, as English
  they came from Norse; this is twenty times rarer than renewal. Each
  process has its own purpose-keyed stream, and `static_society()` sets
  `pronoun_rate` to zero.
  In the isolated 40-seed, 4,000-year court cohort, all 40 languages per
  profile acquired respectful address; 37 kept familiar and respectful
  singulars together and three generalised the respectful form. Across
  five profiles this is 185 coexisting systems and 15 generalisations out
  of 200 (7.5%); the same seeds share social draws across profiles, so these
  are 40 independent social histories, not 200. The unchanged workbench
  sample had 96 living cells and no address changes. The ignored address
  and stability bands together ran in 31.59 seconds in release mode.
  In the family-balanced class-and-pronoun band (40 seeds per profile,
  five profiles, 200 independently seeded founders, 4,000 years), 55/200
  systems, 27.5%, have the distinction; all 55 still have distinct forms.
  Related daughters and the separately reported workbench sample are not
  pooled into that percentage. The founding choice is profile-neutral;
  case, dual number, and inclusive-only systems are not yet represented.
- Noun classes (`gender.rs`) sort nouns into groups that other words must
  agree with, following Corbett, *Gender* (1991): agreement is how a class
  becomes visible. Founding draws presence and semantic basis separately
  from class count: sex, animacy, and shape-and-kind systems can have two,
  three, four, or five to eight classes. Binary sex systems keep masculine
  and feminine cores; binary animacy groups people and animals together
  against inanimates. Three-class systems add a remainder or separate
  people from other animates. Larger systems add shape and material cores.
  [Corbett, WALS 30A](https://wals.info/feature/30A) reports none/two/three/four/
  five-plus at 56.4/19.5/10.1/4.7/9.3%; these modern frequencies constrain
  the four-millennium outcome, not a claim about prehistoric frequencies.
  Assignment is semantic at the core, by each meaning's
  noun semantics; loans and nouns outside the core are assigned by their
  final sound, as formal gender assigns them. The agreeing word is the
  determiner "this", built with a class affix on the morphology's side and
  rendered in the sample sentence and possession phrase (glossed DEM.CL).
  Determiners change by regular sound law; when two come to sound alike,
  their classes merge, and when one class is left, the system is lost,
  with the sound law recorded as the trigger. A language without classes
  can recruit them, rarely (`class_emergence_rate`, 0.00075 a generation,
  no sooner than 32 generations after a loss), from nouns such as person,
  dog, tree, or stone shortened to their first syllable, the classifier
  route of Greenberg (1978). `static_society()` turns emergence off.
  Recruitment draws size on its own purpose-keyed stream with the same
  range as founding; neither process is locked to three or eight classes.
  Founding chooses no classes with probability 0.60, then sex/animacy/
  shape-and-kind with probabilities 0.20/0.15/0.05. Conditional on a system,
  two/three/four/five-plus have weights 0.34/0.23/0.11/0.32, with five
  through eight equally likely in the last group. These priors leave room
  for later merging and recruitment. The same 40-seed-per-profile band
  ends at 106/42/24/10/18 of 200 independent families: 53/21/12/5/9%.
  Each bucket must stay within ten percentage points of WALS 30A and remain
  represented. The isolated societies keep sound change and default
  recruitment and pronoun renewal, without pooling family splits; the
  full workbench sample is run separately. The release band takes 29s.
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
- Root-and-pattern languages can also inflect internally (`inflection.rs`).
  A founding bound plural or past keeps its chosen affix and adds a
  three-consonant template with a drawn vowel melody, such as `C1uC2uC3`.
  Particles and unmarked categories are not overridden. This follows the
  root tier, consonant–vowel skeleton, and vocalic melody distinguished by
  McCarthy, “A Prosodic Theory of Nonconcatenative Morphology” (1981),
  with Arabic broken plurals beside sound plurals as the main model;
  Hebrew and Amharic provide related internal-inflection systems. It is
  a small productive class, not a reconstruction of any of those grammars.
  Germanic ablaut (sing/sang) is not added by this mechanism.
- Internal inflection requires three recoverable root consonants. Known
  derivational prefixes are excluded from that root; other words with too
  few or too many consonants take ordinary native marking rather than a
  truncated or invented root. Eligible founding words draw the pattern
  with probability 0.65–0.85; new words use its current usage share, scaled
  by 0.8 for coinings and 0.4 for loans. A form identical to the base is
  not acquired as an internal contrast. These are modelling priors, not
  measured frequencies. Pattern forms enter the ordinary stored paradigm
  with an 8% affix competitor, whose usage can grow or disappear.
- Sound laws change the complete internal forms, with the same size and
  last-vowel protections as attached forms. When all living forms of a
  pattern agree on a changed melody or vowel length, its productive
  template changes too, with a marker `SoundLaw` history. Contextual
  differences remain stored allomorphs (different shapes of the same
  grammatical mark); a majority never silently overwrites the other
  forms. Retired forms remain frozen. Rare analogy replaces one internal
  form with the available native affix and records the old and new forms,
  as irregular classes can shrink while new words favour affixing.
  `pattern_analogy_rate` defaults to 0.04 per category per generation
  and is zero in `Params::static_society()`. Founding, acquisition, and
  levelling use the separate `pattern founding`, `pattern acquisition`,
  and `pattern levelling` streams. Language views expose marker kind
  `pattern` and an optional `template` string; ordinary form histories,
  contrast retention, and synthesis include internal forms.
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
  A future sample, "the child will see the dog", uses the same word order and
  object form with the most-used future realization of see; its gloss is FUT,
  or bare see when no dedicated mark survives. It is `grammar.sample.future`,
  beside `sentence` and `possession`, with the same text/IPA/gloss shape.
  Text and stressed IPA separate spoken words with spaces. The possession
  sample is "the child's fish", child and fish in possessor order, with the
  child's most-used genitive realization, glossed GEN. The facade omits the sample if a
  required word or form is missing. The design JSON keeps plural and past
  choices and adds nullable object, future, progressive, genitive, order, and possessor
  settings; null or omission draws that setting at founding, while a resolved
  design saves each drawn value.
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
  alternate eligible medial vowels, delete unstressed final vowels,
  lengthen stressed open-syllable vowels, and voice fricatives after an
  unstressed vowel (Verner's law). Syncope counts outward from the input
  stress on each side, deleting the first, third, and later odd eligible
  nuclei. It still requires a single, non-geminate consonant immediately
  on either side, and skips a nucleus whose loss would join more than
  three consonant segments. Skipped nuclei do not advance the rhythm.
  The three-consonant bound is local to the word, not a founding-language
  limit: existing longer clusters elsewhere survive, and other processes
  may create them. This conservative limit stands in for the syllabic
  consonants and cluster repairs not yet modelled. The rule remains a
  deterministic function of the input, with no new random stream or
  law-id exception; assessment, grammatical edges, names, and ordinary
  application share the same matches.
  [Bowers and Hao (2020), §§1 and 3](https://repository.upenn.edu/bitstreams/ad88b760-c451-47e6-84d6-aac391f61ac9/download)
  describe alternating syncope, including Old Irish and East Slavic
  precedents, while warning that its long-term productivity is disputed.
  This is not a claim that all historical syncope was rhythmic: Latin
  and Romance also show loss confined to individual weak positions
  ([Adams 2013, ch. V](https://doi.org/10.1017/CBO9780511843433.008)).
  The `syncope` example follows the same founding varieties as the
  plausibility probe: seeds 0–39 in each of base, Germanic, Semitic,
  Finnic, and Polynesian, with default parameters for 4,000 years.
  Before this change, 6 of 36,709 living words had a consonant run of
  at least five (respectively 0, 0, 2, 3, and 1 by profile); afterward,
  none of 36,749 did. The distribution of each founder's largest run
  changed from `{1: 5, 2: 102, 3: 67, 4: 21, 5: 2, 6: 2, 7: 1}`
  to `{1: 8, 2: 127, 3: 65}`. These are segment counts, with an
  affricate or geminate represented as one segment, not IPA character
  counts. The ignored `syncope_band` test passed in 408.1 seconds;
  its band permits fewer than five such words across the 200 founders,
  rather than demanding that every possible history have none.
  The workbench sample separately had none among 3,245 living words
  before and after; its 16 related living varieties are not independent
  observations. Neither cohort is a world-frequency target.
  Cluster assimilation and gemination
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
- Vowel harmony (`harmony.rs`) makes the vowels of a whole word agree in
  one feature: front with back (backness, as in Finnish and Turkish),
  rounding (as in Turkish and Mongolian), or tongue-root position (ATR, as
  in Akan). It grows out of assimilation between neighbouring syllables,
  as is thought for Finnic and Turkic: within 24 generations of umlaut,
  rounding assimilation, or ATR assimilation, and while the language keeps
  the vowel pairs the feature needs, it can become a rule for the word
  (`harmony_rate`, 0.004 a generation). From then on the first paired stem
  vowel decides; every living word, name, and grammatical form agrees, and
  bound endings take agreeing alternants, as Turkish -lar/-ler. About two
  thirds of disharmonic loans after the gain stay disharmonic, as Turkish
  kitap. Loss is checked against the vowels in living lexical and grammatical
  forms, not the two-percent `established()` threshold used for adapting loans:
  a rare harmonic class still counts, without requiring an accidental minimal
  pair in the small generated dictionary. A sound law ends harmony if it removes
  the last paired contrast or collapses all attested productive affix alternation.
  The check runs before agreement could restore the erased alternants. Only a
  law that actually changed a relevant vowel is recorded as a contrast merger;
  a contrast already absent before that law is recorded as lexical attrition,
  without assigning the next consonant law as its cause. Long contact remains
  a separate loss route, after twelve or more generations of strong contact
  with a language without harmony, as urban Uzbek under Persian. The distinction
  between exceptional stems and productive endings follows Kiparsky,
  [*Domains of Vowel Harmony*, §§17.1–17.4](https://web.stanford.edu/~kiparsky/Papers/vowelharmony2.pdf);
  the categorical loss rule is a model, not an estimated historical loss rate.
  Two catalog laws carry
  the rounding and ATR assimilations; the word-level rule is recorded in
  word histories under its own law id. Each decision has its own stream,
  and `static_society()` sets `harmony_rate` to zero. Over 40 seeds and
  4,000 years, the corrected loss checks leave 60 of 318 varieties active
  (18.87%), with 75 inherited-inclusive gains and 15 losses, all from sound
  laws; lexical attrition and contact account for none in this cohort.
  Before the correction these figures were 61 active (19.18%), 70 gains,
  and nine losses, all labelled mergers. The workbench sample has no gains.
  These are related varieties, not independent typological observations
  (`tests/harmony_band.rs`, about 77 seconds in release). In the separate
  12-seed, four-profile example, losses change from seven to ten and active
  harmony from 103/432 (23.84%) to 98/432 (22.69%): genuine affix collapse
  now ends some systems even while rare stem contrasts preserve others.
  The audit's Finnic seed 2 has no losses after degemination or nasal
  assimilation, and keeps all 16 varieties active instead of 12.
- Tone (`tone.rs`) is pitch that tells words apart, carried on each vowel
  (`Seg::tone`). IPA keeps Chao letters: ˥ high, ˩ low, ˧˥ rising,
  ˥˩ falling, ˩˧ low-rising, and ˧˩ low-falling. Spelling instead adds
  acute, grave, caron, circumflex, hook above, and dot below respectively
  (á à ǎ â ả ạ), normalized to Unicode NFC. The mark goes on the vowel's
  first letter, stacking with its quality and length marks; the low
  contours do not reuse the palettes' macrons or vowel-quality marks.
  A profile using acute for length keeps it while atonal; in a tonal
  language length uses a macron instead, including on untoned words, so
  acute can distinguish high tone. Serialized profiles keep their original
  length setting. Names, specimens, historical spellings, and exports use
  the same renderer; segment histories and IPA are unchanged.
  Tone arises through regular sound laws by
  the two best-attested routes. Final glottal stops and h or s falling
  silent leave rising or falling tone, as Haudricourt (1954) showed for
  Vietnamese (`coda-tonogenesis`). Voiced and voiceless onsets merging
  leave a low and a high register on the next vowel, as in Thai and the
  Chinese languages (Matisoff, 1973; `register-tonogenesis`). Later laws
  merge contours into level tones or lose tone altogether, as Swahili lost
  the tones of its Bantu ancestors. Tone laws are drawn on their own
  purpose-keyed stream (`tone_rate`), compete with no change, and spread
  as waves like other laws. A language is tonal while any living word or
  grammatical form carries a tone, and gaining or losing tonality is
  recorded with the law that caused it. Loanwords take or drop tone by the
  borrower's adaptation, and the comparative method compares tones with
  vowels. `static_society()` sets `tone_rate` to zero. Over 40 seeds and
  4,000 years, 22 founding languages are tonal at the end, against about
  two fifths of languages worldwide (WALS 13A); the register split needs
  voiced obstruents to merge, so a language without them cannot take it.
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
  that for a place another people is also named for. When its clipped
  name is another's, its land, a place, or an epithet on the old name said
  short ("the far Goths") tells it apart, or else whichever of those needs
  the fewest syllables more. If every whole name is taken, it qualifies
  the first with "new". Where vowel fusion swallows the "new" and leaves
  the name unchanged, it tries the other whole names in order, and keeps
  the first if none can grow. A new language whose
  usual name is taken is named the other way ("the X tongue" for "of the
  X"), then either way a syllable longer, and only then in full. Names
  then undergo the same sound
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
  furthest land and every land nearer it than the heart. On a single
  crowded land, a people with access to ships can send half its number
  overseas when a sea destination offers more room per journey effort
  than nearby land. The chance is `fission_rate` times
  the occupied share above the same 0.6 food threshold used by spreading;
  the destination must feed at least half the arriving group. A people
  on one land may also migrate whole (`migration_rate`): likelier the
  more crowded home is, the more mobile its land and way of life make it,
  and when a stronger people shares it. Migrants go to better land within
  `migration_reach` on foot. V5 and later sea migration adds a 100-effort-km
  provisioning allowance, capped by `colony_reach`; earlier geographies
  use `colony_reach` for sea migration. They count half of
  a weaker people's land as free, so strong peoples push into good land
  others hold. A stronger people that outnumbers another on one of its
  several lands crowds it off. Peoples learn a way of life that feeds them half
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
  needs a permitted itinerary with enough owned or rented fleet capacity.
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
  - Territorial splits depend on size and reach, not on a hardship or
    climate event, so have no cause. Pressure-driven sea colonies record
    `climate` when recorded weather still reduces their old land's food.
    Crowding without an event remains unlinked. Famine and plague reduce
    population, not food capacity; neither is invented as a movement cause.
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
- A written high form can also be reformed by purists (`purism.rs`).
  Sacred registers are exempt: a revival can restore scripture, but it
  cannot replace words in its frozen language.
  Two recorded challenges start a movement: a neighbouring state of
  another family and greater prestige (Mechanism `foreign-prestige`), or
  a reform schism in a faith with scripture (`religious-revival`). The
  pressure lasts 24 generations while its keepers live and keep their
  speech; each generation it reforms with chance `purism_rate` (0.08),
  at most once in 12 generations per high form. A reform strikes out
  about two fifths of the borrowed words that hold their meaning in the
  written form. Each is replaced by a surviving native competitor, then
  an attested archaism, then a word derived from a native base, and only
  then a fresh root. If the standard was not yet fixed, the reform fixes
  it as a classical form (`Fixing::Purism`), so the vernacular keeps its
  loans and goes on changing. Icelandic, Hungarian language reform, and
  the Turkish reform of the 1930s are the models; the high form changes,
  speech follows only through the standard's ordinary pull. Over 20
  default six-people worlds of 4,000 years, one reform replaced four
  words among 96 written standards: rare, as in history. The ignored
  `tests/purism.rs` band runs 40 authored revival courts, where 35
  reform and 1,011 of 1,251 replacements revive older native words.
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
- Native coinage continues after founding (`coinage.rs`). New crafts and
  faiths invite compounds or derivatives beside loans, fresh roots, and
  stretched meanings. The choice uses openness, purism, the teacher's
  prestige, and a faith's practice of translation. Existing meanings may
  also gain a native competitor (`coinage_rate`, 0.001 per meaning per
  generation, multiplied by `1 + purism`); ordinary competition decides
  whether it survives. `static_society` disables this new process while
  retaining the older ways of finding a word for an idea.
  The semantic recipes are curated: fire-stone for iron, pray-house for
  temple, and write-word for letter, not arbitrary pairings. Compounds use
  the language's established head order and junction rules. Derivatives
  use its existing affixes or root patterns, following derivational bases
  rather than reading a pattern prefix as part of the root. A missing
  concept gets no word before its people holds the requisite idea.
- Under contact a transparent foreign compound can supply a recipe:
  speakers translate each part into their own current words and put them
  in their own compound order. This is a calque, not a loan of foreign
  sounds; absent native parts or an opaque donor prevent it. These
  choices follow the distinctions and social pressures in Haspelmath,
  [“Lexical borrowing: Concepts and issues” (2009), §§3, 7](https://elearn.univ-tlemcen.dz/pluginfile.php/124991/mod_resource/content/1/lexis.pdf).
  Rates and individual semantic recipes are modelling choices, not
  measured cross-language frequencies.
- Each coinage keeps its parts as they were said when it was made, its
  generation, and, for a calque, its donor language. Thereafter it changes
  as one word, never reconstructed from the parts. After sound laws and
  word competition the engine records the first generation in which its
  form no longer contains every part's current dominant form. The record
  remains even if later mergers restore a match. This is literal segment
  transparency, not a judgement about what speakers understand: junction
  coalescence and root patterns can be opaque by this measure at creation.
  Obsolete words and unspoken languages keep their records unchanged.
  This separation of construction from a word's later independent life
  follows Brinton and Traugott, *Lexicalization and Language Change*
  (2005). Coinage and calque annals point to the recorded craft, conversion,
  revelation, or change of livelihood when that need is known; otherwise
  a calque may point to its recorded contact.
  `examples/coinage.rs` reports three profiles in trade contact over
  4,000 years and the workbench sample. The ignored `coinage_band` test
  runs forty seeds per profile and the unchanged sample recipe.
  In that forty-seed band, the two-peoples worlds produced 21.7–21.8
  native compounds and derivatives on average, with 0.8 calques. Across
  the three profiles, 1,927 of 2,706 coinages became opaque by the literal
  measure; the workbench sample produced 18 coinages. The band took
  59 seconds of execution (79 seconds including the build).
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
  carries a people further and holds it together over more land. Sea
  migrants and settlers need their own fleet or rented berths from a
  contacted carrier. A subject's ships do not supply its ruler's navy.
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
  Each faith holds a position on six tenets (`doctrine.rs`): scripture
  in the sacred tongue or in its people's speech, images, an order of
  priests, rules of purity, pilgrimage, and withdrawal from the world.
  Positions in [-1, 1] are drawn at revelation from the founders'
  worldview plus noise; a pious, insular people leans toward sacred
  speech and purity, a hierarchical one toward priests. They drift
  slowly toward the followers' population-weighted leanings and are
  recorded only when one moves by 0.25. A schism disputes the tenet
  where the reformers differ most, moving it by at least 0.6; a reform
  schism always rejects the sacred tongue, as Wycliffe and Luther did.
  The sacred-language position decides whether followers translate or
  borrow the faith's words and, for literate followers of a scriptural
  faith, whether they write the sacred language or their own. A faith
  with strong purity rules can forbid one concept's word (blood, the
  divine, or cattle), and its followers renew it, as speakers avoided the
  old Indo-European word for the bear; other senses of the word survive.
  `static_society` turns doctrine off. Disputes come only from schisms,
  which are rare: V5 seeds 0–39, six peoples over 4,000 years each, gave
  26 faiths and 3 disputes, too few to test reliably. Over V6 seeds 0–159
  (`tests/doctrine_band.rs`), 179 faiths held their founding positions
  without recorded drift, 13 schisms disputed a tenet, and 99 words were
  forbidden. The band asks for 1–20 disputes and 80–240 faiths.
  Pilgrim roads begin with probability `pilgrimage_rate` (0.02) per
  generation for every follower and shrine within 1,200 effort-km.
  They follow the cheapest permitted directed journey from a held land
  to the exact site. A boat journey needs an owned or contacted carrier's
  port, and can include inland and river legs before or after the crossing.
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
identity. Seafaring learned by a coastal people sustains a fleet at its
held ports; the craft alone supplies no ships inland. A fleet begins at
strength 0.2 and, with automatic crafts enabled, gains up to 0.3 through
eight generations of sustained coastal shipbuilding. It also gains 0.1
per additional port and up to 0.4 from cities at those ports, capped at 1.
Losing every port loses the fleet and its accumulated experience.
Each fleet's passenger capacity is its owner's current population
multiplied by `0.5 + strength`. This is transport over a generation, not
a fixed count of hulls: even a modest fleet can take a half-population
colony, while moving the whole people requires strength at least 0.5.
Population scaling lets a seafaring people's boats grow with its emigrant
groups, as in Austronesian and Norse settlement, instead of making large
peoples permanently unable to colonize. A contacted carrier offers one
quarter of its capacity in rented berths, leaving most transport to its
own people; contact intensity must be positive. Urban residence alone
is not a held port.

Journeys may walk inland to a port, sail, and continue overland, including
usable river-valley edges. Each embarkation and landing adds 100
effort-km. Every sea leg must embark at an owned or contacted carrier's
port; a landed traveller may continue walking but cannot embark again
without access to another port. A passenger party must fit every fleet
it uses. Migration and travelling settlements retain their actual
land, river, and sea legs rather than reconstructing them from today's
coasts or river flow. Fleet formation, fleet loss, and the first use of
each directed sea corridor are recorded in the annals. Rented journeys
strengthen the passenger people's contact with their carriers.

The old automatic sea-movement failure had three gates, not a lack of
Seafaring. The 40-world cohort uses seeds 0–39, ten per map size, and six
founders with rotating profiles: three share a coastal homeland and three
are freely founded. Only founding is authored; all parameters are defaults.
Over 4,000 years the `e12e917` engine recorded 46,107 seafaring
people-generations but 389 land migrations, no sea migrations, and 2,427
territorial splits with no travelling settlements. The earlier `fecd8e0`
cohort recorded 363 land migrations and 2,388 splits, also with no sea
movements. Sea terrain cost three times walking on a plain, plus the
200-effort-km embarkation and landing overhead. Only 23 of 11,474 coastal
lands had any boat-shortest destination within the 600-effort-km migration
radius; none were on Large or Vast maps. Extending sea migration to 1,200
without changing that cost still produced no sea migrations.

Automatic splitting also skipped every one-land people, so its sea-colony
selector was never reached. That selector gave any roomier neighbour an
absolute veto over every overseas destination. Automatic pressure-driven
colonies now reach the selector; land and sea destinations compete by
room per journey effort, with the existing seaward preference. Sea effort
is 0.5 rather than 3. The migration hazard, walking costs, 600-effort-km
walking radius, and squared walking distance penalty are unchanged.
The original fleet calibration gave ships the existing 1,200-effort-km
colony radius, linear distance penalty, and seaward preference; V5's
whole-people sea radius is now separated from colony expeditions below.
The shortest transport route determines its mode before the applicable
radius is checked; a pointless boat detour cannot turn a distant walk
into a sea migration. Climate still supplies pressure through the same
cached food capacity, rather than a separate sea bonus. Finally, a
single-port fleet can mature enough to carry its people without first
acquiring a city. These are illustrative colonisation rates, not estimates
fitted to a historical census.

The separate authored maritime controls explicitly found coastal seafarers,
send half-population colonies, and rent berths for small groups. Their
expedition radius is a selected finite sea corridor's effort, not a change
to the natural cohort's defaults. The ignored `fleets_band` test keeps
their capacity checks separate from spontaneous movements.

With the fix and fleets, the default cohort records 368 land and 53 sea
migrations (12.59% by sea), 2,465 splits including 13 travelling sea
settlements, 30 rented passenger journeys, and 22 mixed itineraries.
Land migrations are 5.4% below the current-tree baseline and 1.4% above
the earlier cohort. It builds 1,319 fleets, loses 251, and opens 152 sea
corridors. The 40 authored controls make 160 sea settlements (40 rented),
open 80 corridors, and refuse 40 larger rented groups for insufficient
capacity. The full sample recipe has 11 fleets built, two land migrations,
and no sea movements; it is included without inventing a per-world floor.

Those figures were measured on continental-v4 maps. On continental-v5,
with its broken coasts and more islands, the same cohort at revision 53
recorded 330 land and 175 sea migrations (34.65% by sea), 143 travelling
sea settlements, 152 rented journeys, 62 mixed itineraries, 1,694 fleets
built and 257 lost, and 448 sea corridors. Land migration stays within
its earlier band; sea movement adds to it rather than replacing it. Small
worlds go by sea most often (54% of migrations), medium ones least (17%).
The full sample recipe builds 27 fleets and makes one sea settlement.

The matched-map diagnostic (`sea_migration -- 40`, seeds 0–39 with the
same size rotation) separates physical opportunity from fleet, contact,
capacity, and demographic gates:

| Size | Coastal land cells, V4 → V5 | Islands, V4 → V5 | Walk destinations per shore at 600, V4 → V5 | Boat-shortest destinations per shore at 1,200, V4 → V5 |
| --- | ---: | ---: | ---: | ---: |
| Small | 40.3% → 76.2% | 39 → 54 | 23.09 → 13.95 | 32.55 → 53.42 |
| Medium | 27.2% → 45.3% | 72 → 87 | 25.94 → 20.96 | 31.61 → 37.43 |
| Large | 26.6% → 46.8% | 56 → 141 | 8.45 → 7.70 | 6.00 → 7.67 |
| Vast | 14.8% → 27.5% | 34 → 489 | 9.94 → 7.79 | 4.64 → 8.67 |

These are coast-to-coast routes, not predictions of complete mixed
itineraries. On V5, respectively 77.8%, 64.7%, 51.9%, and 62.2% of those
boat-shortest destinations cross between landmasses; on V4 the shares
were 53.0%, 39.8%, 27.2%, and 12.0%. Neither the 100-effort-km charge at
each shore nor sea effort of 0.5 changed with V5. Minimum boat-shortest
costs barely changed: 334/340/472/484 became 334/339/468/470 effort-km.
The new geometry provides more ports and overseas targets while taking
away walkable alternatives.

Small is also physically small: its 800-km radius makes the old
1,200-effort-km sea-migration reach 1.5 radii, versus 0.75, 0.375, and
0.188 on the other sizes. All 1,463 sampled Small shores can reach a
boat-shortest destination at 1,200; they average 53.42 such destinations
against only 13.95 walking destinations at 600. At 600, sea destinations
fall to 3.85 per shore. Linear sea weighting against squared walking
weighting further favours those numerous destinations, even though
boats already spend less effort per physical kilometre.

Rented berths amplify access rather than bypassing the capacity gate:
only a quarter of a contacted carrier's transport is offered. V5 builds
1,694 fleets against V4's 1,319, and 152 of its 318 sea migrations and
settlements use rented transport (47.8%, versus V4's 30 of 66, 45.5%).
Small's 18 rented journeys among 21 sea movements make shared access
especially important there. These event counts describe realised use,
not an additive causal attribution: geography also changes settlement,
population, and subsequent contact histories.

Revision 54 distinguishes provisioning a whole-people move from sending
settlers. On V5, automatic migration and authored settlement planning
share `min(colony_reach, migration_reach + SEA_MIGRATION_PROVISION_KM)`
for sea migration: 700 effort-km by default, against 600 on foot and
1,200 for settlers. The named 100-effort-km provisioning allowance is
calibrated, not a new physical speed or a historical estimate. Sea
effort, embarkation costs, linear sea weighting, migration hazards,
fleet capacities, rented-berth shares, and colony journeys are unchanged.
All earlier geography versions retain their original reach and replay
behaviour; no random stream changes.

The reach experiments explain why the allowance is nonzero. A 600 sea
cap gave 450 land / 41 sea migrations (8.35%) but no Medium sea migration.
That is not an absence of Medium sea routes: 70.8% of its V5 shores have
a boat-shortest destination at 600, rising to 90.1% at 700 and 97.3% at
800; candidate destinations per shore rise from 3.13 to 6.24 to 10.37.
The cohort's realised Medium sea migrations are respectively 0, 1, and
2. At 800, however, the overall sea share rises to 21.41% and Small to
33.33%. At 700, every size has sea migration and the overall share is
13.58%. Changing only sea weighting from linear to squared left 26.24%
by sea, so the existing weighting was retained.

| Size | Revision 53 land | Revision 53 sea | Sea share | Revision 54 land | Revision 54 sea | Sea share |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Small | 13 | 15 | 53.57% | 20 | 3 | 13.04% |
| Medium | 35 | 7 | 16.67% | 37 | 1 | 2.63% |
| Large | 116 | 62 | 34.83% | 170 | 35 | 17.07% |
| Vast | 166 | 91 | 35.41% | 193 | 27 | 12.27% |
| Total | 330 | 175 | 34.65% | 420 | 66 | 13.58% |

Revision 54 also records 220 travelling sea settlements, 149 rented
journeys, 40 mixed itineraries, 1,753 fleets built and 314 lost, and 384
sea corridors. Travelling settlements increase rather than disappearing
as whole-people sea moves fall. The
authored controls remain 160 sea settlements, 40 rented journeys,
80 corridors, and 40 capacity refusals. That V5 sample records two land
migrations, one rented mixed sea settlement, and 27 fleets built.

V6 keeps those reach caps and movement rules. Its compact continents
produce 485 land and 21 sea migrations, a 4.15% sea share. The Small,
Medium, Large, and Vast shares are 10.00%, 13.79%, 3.72%, and 2.44%,
with 3, 4, 9, and 5 sea migrations. None is zero. The cohort also has
34 sea settlements, 17 rented journeys, 9 mixed itineraries, and 207
sea corridors. Including settlements, 10.19% of movement is by sea.
Before one-region inland seas were filled, V6 gave 5.60% and 75 sea
settlements: those pockets made the land around them coastal, so peoples
launched fleets from water with no way out. The V6 sample has four land
migrations, no sea migration or settlement, and 16 fleets built. The
migration-share floor changes from V5's 8% to 3%, below the measured
4.15%, rather than changing geography or travel reach to force more
voyages. The band retains the 20% overall and 30% per-size ceilings,
292–500 land migrations, and positive sea settlements, rented journeys,
and mixed itineraries. The land ceiling has headroom over V5's 420
(and the 600-cap experiment's 450); the sea-settlement ceiling stays 300.

The original V4 band ran in 159.96 seconds in release. For the same three-founder
Vast timing recipe (`calibrate -- geography 1 160 vast 3`), pre-change
median/p95 step times were 34.71/73.78 ms at 632.76 years/s; with fleets
and sea movement they are 30.69/66.89 ms at 706.45 years/s. Startup changes
from 50.46 to 68.93 ms as cheaper sea travel enlarges cached voyage rows.
These are local measurements, not a portable timing guarantee.

`World::journey_to` and `journey_between` are directed, including rented
transport where contact already exists. Walking wins a tie. `apart` is
symmetric, so either participant may provide transport for trade,
intermarriage, or religious contact. Fleet strength limits the frequency
and intensity of overseas trade. Rule always requires the actual ruler's
directed access using its own fleet, never rented berths.
`World::connect` returns a refusal before changing contacts, states, or
history when a requested relation is physically impossible. Authors
bypass chance, not reachability. Resolved travel durations and hull-by-hull
shipbuilding are not modelled.

Default effort-km parameters are founding separation saturation 800,
migration 600, colony 1,200, trade 1,800, conquest 800, pilgrimage 1,200,
and cohesion reach 300 before mobility. Pilgrimage rate is 0.02.
Founding is independent settlement and needs no ships. Spread remains
land-adjacent and uncapped. Cohesion and territorial partition use exact
walking distances without a cache ceiling. Distance preferences use
`1 + effort / 100`, so changing units does not change reference-grid odds.
Ships spend 0.5 effort-km per sea kilometre, against 1 on an open plain;
access and the fixed embarkation costs still limit journeys. On reference
plain shores, three intervening 100-km sea cells cost 450 effort-km.

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

The map stores sparse walking and voyage rows out to 1,800 effort-km.
Walking rows retain separate allocations so a changed river edge rebuilds
only rows whose old reachable set includes either endpoint; untouched rows
are neither searched again nor copied. This also covers newly shorter paths:
their first changed edge must be reachable through an unchanged prefix.
Missing cached destinations are outside a bounded query, not necessarily
unreachable. Exact pair queries use Dijkstra beyond the cache, and uncapped
consumers reuse full source rows. Land-only directed access retains these
cached rows, and rule checks them before searching for a sea route. Mixed
routes use a traveller's currently held and contacted ports in Dijkstra,
with reusable scratch space for migration and outward contact searches.
Incoming carrier rows are reused until a contact changes, rather than
searched again for every prospective partner. Settlement planning searches
once per inhabited source, not once per destination. Authored larger
reaches use phase-local rows without mutating the shared map.
Spatial and contact indexes are derived per phase. Successful spread
redistributes all old holding presence; later migrants see earlier
migrants' consumed room. These indexes are not additional replay state.
Urban residence adds population at a city, not a territorial holding or
borrowable port. The phase indexes distinguish residents from land holders.

Facade overview, annal, word-history, and grammar-history views borrow the
world's existing sound-law catalog rather than rebuilding it for each view.
These cache changes preserve all costs, histories, and serialized views;
they do not change the engine revision.

Physical radius and regional resolution are separate:

| Size | Regions | Radius | Approximate circumference |
| --- | ---: | ---: | ---: |
| small | 642 | 800 km | 5,030 km |
| middling (`medium`) | 2,562 | 1,600 km | 10,050 km |
| wide (`large`) | 2,562 | 3,200 km | 20,110 km |
| vast | 10,242 | 6,371 km | 40,030 km |

Wide and vast use coarser regions, rather than letting planet size demand
unbounded mesh and route storage.

### ContinentalV6

The new generator runs ten semi-Lagrangian drift steps on a fixed level-five
icosphere with 10,242 points, at every world size. Each point carries its
plate, continental or oceanic crust, thickness, oceanic age, accumulated
uplift and last uplift step. An unequal warped spherical Voronoi partition
has nine to twelve plates. Its additive size weights fall with the inverse
square of plate rank. Continental loading slows a plate's Euler rotation.
Each destination inverse-rotates into every plate's previous frame, then
finds the nearest source by deterministic ascent through the icosphere's
Delaunay graph. The regional meshes use exact subsets of these points. A
sea one region wide with land all round is a basin below the map's
resolution; it rises to its lowest neighbour, and rivers and lakes carry
inland water at that scale. Seas of two or more regions remain.

One connected, domain-warped supercontinent initially covers 38–42% of
the sphere including shelves. Two inherited sutures mark eroded ranges;
one guides the dominant rift. Radial spreading opens new ocean, and one
marginal plate returns partway through the history. Divergence stretches
and thins continental shelves. Oceanic overlap consistently subducts the
older lithosphere, rather than allowing individual volcanic vents to
change which entire plate overrides. Convergence raises a coastal range
and lowers an offshore trench. Oceanic convergence builds separate
volcanic vents. Continental overlap thickens crust and welds the plate
groups, which share one Euler velocity thereafter. Three fixed mantle
hotspots leave volcanic material behind as plates move over them.

The analogues are Pangaea's breakup and Africa and South America's
conjugate passive margins, young rift seas such as the Red Sea, Andean
subduction ranges, Japanese island arcs, Himalayan collision plateaus,
eroded Appalachian and Ural sutures, and Hawaiian hotspot chains. Thin
rifted provinces can become microcontinents, as with Madagascar, while
near-sea-level shelves can leave islands like the British Isles. These
are consequences of the crust history, not separately drawn outlines.

Isostasy turns crust thickness into height. Old ocean floor cools and
deepens; zero-age gaps are shallow ridges. Uplift erodes with elapsed
steps. Coast-scale noise has amplitude 0.012, far below the continental
and oceanic contrast. A seeded spherical-area sea level selects land;
the bounded signed elevation field also drives `sphere::refine_coasts`.
The top 10% of land regions by height are mountains and the next 17% are
hills. Moisture keeps V5's latitude, coastal distance, relief and
geographic-wind law. Lakes, channels, rivers and climate use the existing
downstream pipeline. All new random streams begin with `continental-v6`.
No generated world is rejected or retried.

Forty seeds per size measured the following physical bands after coast
refinement. Mountains are near a margin when a simulated convergent
contact or advected suture lies within two regional borders.

| Size | Land share | Continents | Largest share of land | Islands | Mountains near a margin |
| --- | ---: | ---: | ---: | ---: | ---: |
| small | 29.14–30.51% | 2–5 | 34.57–68.08% | 3–18 | 89.47–100% |
| medium | 29.39–30.84% | 3–5 | 34.36–62.27% | 11–33 | 92–100% |
| large | 29.39–30.84% | 4–7 | 34.36–62.27% | 9–29 | 92–100% |
| vast | 29.50–30.66% | 4–13 | 34.37–78.57% | 18–59 | 93.79–99.37% |

Spherical convexity measures the share of deterministic pairs of land
regions whose minor great-circle arc stays on that same landmass.
There are at most 512 evenly spaced pairs per body, sampled at less than
one quarter of a regional spacing and tested against the refined cell
polygons. This replaces centroid-gnomonic solidity, which is undefined
for the largest V5 trefoils crossing their centroid's horizon. No body is
excluded. Median V5/V6 convexity is 0.683/0.893 at small, 0.730/0.871 at
medium, 0.730/0.833 at large, and 0.715/0.779 at vast. The median over all
continents is 0.719/0.838. Compact Africa and Eurasia's more indented
coasts are qualitative orientation, not measurements on this generator.

A sea or gulf counts when it is a connected water patch inside one
landmass's convex hull, has at least two regional centres, and at least
70% of its rim borders that body. Its open mouth counts against the rim
share. The survey finds one or more in 28 of 40 medium seeds, above the
10-seed band. It records four to nine active or inherited sutures.
Hotspot islands are identified by advected volcanic material, not by
their present position beside a mantle source.

`continents -- svg <seed> <size> <out.svg>` draws two orthographic
hemispheres with terrain, final plate boundaries and the initial
supercontinent coast. `continents -- timing` measures complete
`Map::generate` calls, including hydrology and travel caches.


### Legacy ContinentalV5

`ContinentalV5` draws three to six cratons,
usually four or five. Small worlds use three or four so that their coarse
mesh resolves each body's gulfs. Continental area shares descend sharply:
the largest two usually hold about half and one quarter of the land.
Lambert equal-area coordinates and determinant-one elliptic deformation
keep body size independent of latitude. The largest two bodies have aspect
factors of 2.6–3.0 and 2.0–2.8; broad second- and third-harmonic outlines
form gulfs and peninsulas. Their squared mean normalizes continuous area.
Finer shore detail falls faster than frequency to the power -1.2.
The second craton lies near the first, while the rest occupy remaining
ocean; occasional intervening sea cells form continental straits.

Crust grows continuously from each body's thickest cell, with a priority
frontier and cumulative spherical-area budgets targeting 28.5–31.5% land
before coast refinement. Flooded province boundaries preserve straits
without joining differently sized continents. This replaces the old
branching assembly, rather than retrying unfavorable generated maps.
Divergence erodes margins; convergent continental domains raise interior
sutures, and oceanic neighbours supply
coastal belts. A separate 6.5% emergent-crust budget grows islands along
oceanic convergence and on shelves one sea cell off the mainland. A seeded,
low-frequency shelf field places groups, and flooded straits keep them
separate. Their characteristic area stays below the physical continent
cutoff; larger worlds consequently have more islands. No uniform
microfragment field is added. The measured island band is 3–8% of land,
with at least two islands at small, six at medium and large, and 15 at vast.
`sphere::refine_coasts` still supplies shared coastal detail without changing
region adjacency. V5 retains V3's moisture
law and geographic wind, and runs the same lake basins, river channels,
lake naming, and downstream climate buffering as V4 through the shared
`GeographyVersion::has_lakes` capability. The new crust generator uses only
`continental-v5` purpose streams; V4 remains the V3 surface with lakes.

`cargo run --release -p umran-sim --example continents -- 40 all` now prints
V5 and V6 side by side for every map size.
It reports physical land share, continent counts, five largest shares,
island area and count, compactness, mountain placement, spherical
convexity, enclosed seas and gulfs, sutures and hotspot-supported islands.
Compactness is `4πA/P²` with both area and coastal perimeter measured on the
pre-refinement cell polygons, not the decorative coast. V5's legacy bands are
0.07–0.38 per continent, 0.12–0.30 for each world's median, and at most
0.25 for the largest continent. The island-support diagnostic checks one
intervening sea cell to a continent or one cell to a convergent band;
mountain diagnostics compare mean stress with other land and count coastal
mountains. The legacy surface-only band tests sample 12 medium, four vast, and four
each small and large worlds without building travel caches, including
continent-count variation and continental straits.
`continents -- dynamics 12 160` compares V5 and V6 default-parameter medium worlds
with six independent founders, following `calibrate geography`'s profile
and standing choices. Counts are final living peoples and contacts by kind,
cumulative states founded and language shifts, and `Met` events by kind.
Conquests have their own events, so `Met` is not a cumulative rule counter.
The report does not force founders together or author any contact.
The authored showcase remains seed 21, medium, now on V6: its river plain is
region 448, the middle people begin in 1722, and the sea people in coastal
plain 33. The lake-free plains path `448 → 1722 → 33` connects the three.
Its native provenance test checks the first realm and writing, the land
conquest at generation 100, and the subsequent faith and preserved sacred
language through generation 160.
`continents -- sites 21` lists lake-free river/coast plains connected by
at most four land borders when a new geography needs a new showcase.

The facade exposes `radiusKm`, geographic `center` and `boundary`
coordinates, `Region.areaKm2`, and authoritative `River.lengthKm`.
`site` is a derived chart coordinate only. River length
follows great-circle course segments through the actual confluence or
shared coastal midpoint. The browser's `cartography.ts` projects this one
geography as an equirectangular chart or an orthographic globe; d3-geo clips
the seam and horizon. Switching or rotating views changes no history.
Movement views use the event's recorded `bySea`, not the present coast.

Revision 33 replaces the old flat geography and its region identities.
Earlier recipes open recovery with their originals intact rather than
replaying their decisions on unrelated lands. Native parity parsing uses
`serde_json`'s `float_roundtrip` feature so geographic f64 values retain
their exact bits through JSON, as they do with the browser's `JSON.parse`.

## Rivers and regional climate

`rivers.rs` builds a coast-inward priority flood. Each land drains through
one shared-border neighbour toward the sea, staying within its landmass.
In `spherical-v1`, `continental-v2`, and `continental-v3`, enclosed hollows
receive a lowest-saddle spill route without changing their visible elevation
or terrain. Region ids break ties, and the upstream order
cannot loop. Runoff accumulates physical wet catchment area, with a separate
seeded local variation. A reach becomes a river at about 25,981 wet km²,
not at a fraction of the map. At the coarser wide and vast resolutions,
one wet region can already supply a named headwater reach.

Every river has a stable id, an ordered main course, an ultimate sea or lake mouth,
and its upstream catchment, the land supplying its water. The strongest
branch continues the main course;
other branches keep their own ids and join it. Shared downstream reaches are
stored once. Tributary and main-river catchments may overlap because water
from the tributary supplies both. Short tributaries remain short, and great
courses can continue for many regions where a landmass permits.

`continental-v4` introduced lakes on V3's unchanged land and geographic
wind. V5 and V6 use their own crust with the same `lakes.rs` pipeline:
connected depressions below a common spill surface.
Their baseline catchment supply is compared with warmth-dependent evaporation.
A wet basin overflows its lowest saddle. An endorheic basin has no outlet;
its rivers end at its lowest land. Its lower water surface covers only part
of the depression. Lakes occupy parts of land regions, preserving every
region id, terrain, and landmass. Their extent and outlets do not vary with
later climate. V1–V3 retain their original drainage order, runoff, rivers,
and climate, with no lakes or channels.

V4 and later river channels follow unit-vector arcs from each region centre through
the shared border to the next centre. Tributaries end at their confluence,
open rivers at the coast, and closed rivers in a lake. They do not depend
on a chart projection. This is a regional channel model, not river meanders.
The depression model follows the priority-flood approach of Barnes, Lehman,
and Mulla (2014); the water-balance distinction follows the open and closed
lake basins described in Wetzel's *Limnology* (2001).

Water gives an immediate, bounded, additive floodplain farming benefit,
including in steppe and desert. Foragers gain fishing and gathering food,
and herders gain less. Hills receive a smaller farming benefit, and mountain
streams do not become grain plains. Flow depends on upstream rain, so a
locally dry valley can be a refuge until its wider catchment dries. Cold
still limits farming beside a flowing river. There is no irrigation craft.
Lake shores receive an additive food benefit capped with the river benefit.
Their warmth and wetness anomalies are reduced to 65% of the zone departure.
This coarse buffering does not simulate lake currents, salinity, or seasons.

Drainage-connected, usable river reaches multiply the canonical walking
border effort by 0.65. Adjacent unrelated rivers receive no discount.
The same costs feed distance, border closeness, migration, cohesion, trade,
and sound waves. Flow crossing about 17,321 wet km² changes usability;
only such crossings refresh affected sparse walking rows. Newly inaccessible contacts
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
`lake_names.rs` applies the same inheritance, borrowing, shift, and sound-law
rules to lake names. The first settled shore speakers coin a water-name;
nearby speakers remember their own forms. Lakes have separate identities
from rivers even when a river carries their overflow. Dedicated lake-name,
succession, exonym, and memory-shift streams leave earlier draws untouched.
The facade exposes static lakes, river topology, V4/V5 channels, and zone ids in `map()`,
historical conditions and literal feeding capacities in `climate(generation)`,
and name histories with local alternatives in `river(generation, id)` and
`lake(generation, id)`.

The workbench reads those views within the active telling and year. Rivers
are inked above terrain, with three catchment-width tiers and dashed failed
flows. Tributaries meet their parent course; open main rivers meet the shared
coast of their sea mouth, while closed rivers stop within their terminal
lake cell. The read-only `joinAt` field names the exact
confluence region; a tributary can border several reaches of its parent.
Close zoom adds attested names in the selected
people's speech, falling back to the mouth's speakers. River cards retain
name records, exonyms, and flow annals. Weather colouring uses zone departures,
and climate annals lead to zone cards. Far, middle, and close chart views
disclose progressively finer detail; a kilometre scale follows the camera.
Lakes appear on both chart and globe as sea-filled, coast-outlined insets
within their regions. Their area is not modelled: every outline is drawn
at a fixed 0.6 scale toward its region centre, not from water elevation.
Close zoom adds lake names in the selected speech, falling back to shore
speakers. Lake cards preserve naming dates and coiners, other peoples'
names, shore lands, and the overflow river or closed basin. Their chronicle
collects events in those shore lands. Find, notes, and the lands chapter
include lakes alongside rivers.

### Seasonal weather

`seasons.rs` derives each land's seasonal profile from spherical latitude,
distance inland, elevation, and terrain. Thermal amplitude increases toward
the poles and continental interiors. Summer falls half a year apart in the
two hemispheres. Low-latitude large landmasses can have monsoon-like summer
rains; subtropical coasts can have winter rains. The profile exposes a
normalized thermal range, wet-season id, and river-flood susceptibility.

Each generation samples one representative seasonal year, not all twenty-five
individual harvests. A new `seasonal weather` stream, keyed by generation
and climate zone, draws after the existing climate history. Neighbouring
lands share anomalies but differ in susceptibility. Drought is likelier in
dry climates and where a monsoon can fail. Harsh winters require a cold
winter baseline. Damaging floods require a low-relief river reach; upstream
seasonal runoff contributes to their likelihood, including in dry valleys.
These are fictional risk bands, not calibrated Earth return periods.

Only the strongest food hazard in a land's sampled year is retained.
Its bottleneck reduces the current generation's feeding capacity, more for
crops than for mixed gathering. It does not change annual-mean climate,
river navigability, or the ordinary benefit of floodplain water. The next
sample restores yields before applying new losses, so shocks do not compound.
Severe shocks feed the existing hardship and remembered-exposure paths.
Migration records the active hazard when it reduces food. Livelihood change
records it only when removing the shock removes the adoption advantage.
Only severe shocks on inhabited land enter the record, as drought, hard
winter, or flood annals naming the land and its peoples; milder or
unwitnessed ones still cost food but leave no entry, so a response to them
has no recorded cause. `Seasons::struck` tallies every sampled hazard for
the band. Over 40 worlds of 4,000 years, 5.5% of land-years saw drought,
3.6% a hard winter, and 0.8% a flood; 1,461 severe shocks on inhabited land
were recorded, and 35 migrations cite one. No harvest loss is reported as
deaths.
`Params::static_society()` disables hazards through `seasons_enabled = false`.

Seasonal hardship can shorten rule and leave fewer subjects shifting to a
ruler's language. In the 40-seed subjugation band, 13 of 40 shifted on
lake-free maps without seasons, 9 with seasons, and 6 with seasons and
lakes together; default worlds keep both.

The model follows the latitude and land–sea contrast described in Dennis
Hartmann's *Global Physical Climatology* and the monsoon seasonality in
Colin Ramage's *Monsoon Meteorology*. The Nile and Yellow River motivate
the distinction between useful seasonal water and damaging floods.
Historical comparisons include the debated drought contribution to the Late
Bronze Age collapse and the exceptional cold of 536 described by Procopius;
the engine does not claim a single climatic cause for either history.

## Studying one mechanism

Each example prints a readable report; profiles are preset ids
(a flavor such as `familiar`, `germanic`, `semitic`, `polynesian`).

```sh
cargo run --release -p umran-sim --example found -- <seed> <profile>
cargo run --release -p umran-sim --example drift -- <seed> <profile> <generations> [flavor...]
cargo run --release -p umran-sim --example contact -- <seed> <donor> <recipient> <kind> <generations> <seeds>
cargo run --release -p umran-sim --example family -- <seed> <proto> <outsider> <generations>
cargo run --release -p umran-sim --example history -- <seed> <generations> [size]
cargo run --release -p umran-sim --example calibrate -- <seeds> <generations> [profile]
cargo run --release -p umran-sim --example calibrate -- geography [seeds] [generations] [size] [founders]
cargo run --release -p umran-sim --example rivers -- [seeds] [size] [generations]
cargo run --release -p umran-sim --example seasons
cargo run --release -p umran-sim --example fleets -- [seeds] [generations]
cargo run --release -p umran-sim --example sea_migration -- [seeds]
cargo run --release -p umran-sim --example length -- [seeds] [generations]
cargo run --release -p umran-sim --example audit -- [seeds] [generations]
cargo run --release -p umran-sim --example faiths -- [seeds] [years] [first-seed] [seeded|natural|sample|sample-unseeded]
cargo run --release -p umran-sim --example stress -- [seed] [generations]
cargo run --release -p umran-sim --example cities -- [seed] [generations]
cargo run --release -p umran-sim --example cities -- --band 40 160
cargo run --release -p umran-sim --example ethos -- [seed] [generations]
cargo run --release -p umran-sim --example ethos -- --band 40 160
cargo run --release -p umran-sim --example endings -- [seeds]
cargo run --release -p umran-sim --example tense -- [seeds] [years]
cargo run --release -p umran-sim --example pronouns -- [seeds]
cargo run --release -p umran-sim --example gender -- [seeds-per-profile]
cargo run --release -p umran-sim --example harmony -- [seeds] [generations]
cargo run --release -p umran-sim --example pattern -- [seeds] [generations]
cargo run --release -p umran-sim --example tone -- [seeds]
cargo run --release -p umran-sim --example syncope -- [seeds]
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

Subregional river meanders, changing lake extents, lake salinity, resolved travel times,
globe wrapping,
purism in speech rather than writing, spelling reform as a purist act,
inflection beyond count noun plural and object marking, the genitive, and verb
past, future, and progressive, agreement beyond the determiner, combined
tense–aspect forms, modal readings and clitic stages, pronoun case forms,
tone beyond the word (sandhi), vowel harmony beyond the word, syllabic
consonants and productive cluster repair, prenasalized stops, syntax beyond fixed word
and possessor order, alignment beyond this object contrast, and doctrine
beyond six tenets, with images, pilgrimage, and monasticism recorded but not
yet acting on the world.

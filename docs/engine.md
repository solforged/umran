# Engine model

How `crates/umran-sim` models language change. The rules that must
always hold are summarized in `AGENTS.md`; this is the fuller picture.

- `World` (`world.rs`) steps communities, varieties, and contacts through
  25-year generations. `Variety` holds a `SoundProfile` and a `Lexicon` of
  `Slot`s, where words compete for each concept with usage weights. Words
  keep a log of every sound law, borrowing, extension, and loss.
- The map (`geography.rs`) is drawn from the world seed at a chosen size
  (`MapSize`): regions are Voronoi cells around jittered hex-grid points,
  so each borders about six others. Terrain (sea, plains, forest, steppe,
  hills, mountains, desert) comes from smooth noise ranked into fixed
  shares, half of it sea, and sets how many a region feeds (`fertility`,
  where peoples are founded; how many it feeds each way of life is
  `Livelihood::feeds`), how hard it is to cross (`travel`), and how
  readily its people move (`mobility`: steppe and desert most, mountain
  folk least). Land
  falls toward the map's edges but is rough enough that larger maps
  usually have islands; each land region knows its `landmass`, so
  crossing between landmasses means crossing the sea. Travel distances
  between all regions are precomputed. Map generation uses only arithmetic
  and square roots, so native and WASM draw the same map.
- Languages are founded from a `LanguageDesign` (`design.rs`): the exact
  sounds, each used or favoured, plus knobs (word length, final consonants,
  inner clusters, repetition, long vowels, affixes or root-and-pattern,
  suffixing, derivation) and spelling. It resolves to an internal
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
  labialized); new segments are appended, never inserted. Vowel length is
  a per-segment flag that profiles can make contrastive.
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
- Sound laws (`laws.rs`) apply simultaneously and regularly to every living
  word, never to obsolete ones, and never delete a word's last vowel.
  "No change" competes with them, so a culture is never forced into a law.
- Sound laws also spread as waves (`spread_waves`), as the wave model of
  language change describes: a law that took hold in a variety in the
  last ten generations may pass to a variety in contact with it. The
  chance grows with the contact's intensity and kind (neighbours and
  intermarriage carry sounds most, trade and religion least), how close
  their land is, the source's prestige, the law's freshness, and the
  receivers' taste, and above all with kinship: dialects that have just
  parted take up each other's changes readily, half as readily after 20
  generations apart, and unrelated languages seldom (`KIN_STRANGERS`).
  A variety takes at most one wave per generation and records where each
  came from (`Variety::waves`). Waves make related languages share
  changes their common ancestor never had, leaving isoglosses (the lines
  where a change stopped) that cut across the family tree.
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
- Borrowability is set per concept (Leipzig–Jakarta rank or `Tier`), never
  per semantic field, so field patterns must emerge. `wold.rs` holds WOLD
  figures for validation only; the model never reads them. Loans adapt to
  the recipient's established sounds and then undergo only later laws.
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
  told otherwise: plains farm, steppe and desert herd, the rest forage.
  Its number is spread over its lands by how much each feeds it, and it
  grows logistically toward what they feed it (`capacity` times
  `Livelihood::feeds`), counting everyone living there; when its lands
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
  steppe turn to herding, and foragers on plain rarely begin to farm of
  their own accord.
- Peoples suffer and end. Famine, plague, or drought (on dry land) strikes
  a peopled land now and then (`hardship_rate`), killing a share of
  everyone there, so a people on one land suffers worst. A people below
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
  Rule lasts longer the further its ruler stands above the ruled, and rule
  and intermarriage leave the peoples neighbours. Peoples on the same land
  stay neighbours. Two peoples have one contact at most: a new one between
  them replaces the old. The world makes contacts of its own: peoples on
  the same or bordering land become neighbours, more readily and more
  closely the easier the border is to cross; peoples open trade, nearer
  partners likelier; and a people far above one it deals with may conquer
  it. Each beginning, ending, and conquest is a `WorldEvent`, so it is told
  in the history and stops "until something happens".
  `Params::static_society()` turns all of this off.
- States (`polity.rs`) sit above peoples: a ruling people, the peoples it
  rules, a capital land, and a name coined from the rulers' name with
  their belonging affix ("the realm of the Ivo"), which changes with
  their speech while the state stands. Every subject has a rule contact
  with its rulers, so borrowing, waves, and shift work through rule as
  before; when that contact ends, the subject has thrown off the rule. A
  conquest, by the world or an author's rule contact, brings the ruled
  into the rulers' state, raising one if they have none, and conquered
  rulers bring their whole state, which falls. Subjects make no
  conquests, and a people split off within a state stays in it. A large
  farming people under no state may organize itself into one
  (`state_rate`): readily in answer to a challenge (bad times on its
  lands, being crowded off land, a stronger state beside it), a tenth as
  readily in comfort. This is Toynbee's challenge and response. Ruling
  feeds the rulers more: a tenth of their own lands' yield and a tenth of
  their subjects' number in tribute. Those the tribute feeds live in the
  capital's city. A state falls when its rulers end or lose the capital,
  or collapses (`collapse_rate`), three times as readily after bad times
  on its rulers' lands; states last about 750 years. Rule holds half as
  long after such bad times. A state with a city of 10,000 that has stood
  200 years may take its court speech as its standard. A standard takes
  up sound laws, waves, and new words at half the pace; its words carry
  extra prestige; its sound changes and words reach the kindred speech of
  its subjects three times as readily, even basic words (dialect
  levelling); its subjects of other families shift to it twice as
  readily; and rule under it holds half again as long. Each state draws
  a purism at its rising, most near none: a purist standard borrows
  less, its loans lose ground in use, and concepts held by loans gain
  native words. When a state falls the pull stops, and its dialects
  drift apart again.
- People have given names (`names.rs`). Each language keeps a stock of
  eight in fashion, built from its own words in its own style: one word
  ("Wolf") or two joined ("Wulf-stan", as Germanic, Slavic, and Greek
  names were). The meanings are weighted by way of life: herders name
  children for horses, cattle, and spears, farmers for grain and fields.
  Names go out of fashion and new ones come (`name_turnover`); sound
  laws change them like any word. A people of a founded faith names for
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
  seafarers migrate or send colonies over the sea.
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
  Avestan *daēva* "demon"). Rival doctrines and iconoclasm are not yet
  modelled.
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
cargo run --release -p umran-sim --example length -- [seeds] [generations]
cargo run --release -p umran-sim --example audit -- [seeds] [generations]
```

## Not yet modelled

Rivers, climates, cities with speech of their own (a city's speech is
its court's; no koiné), purism within a classical form, compounding and
derivation after founding beyond renewal and new meanings, inflection,
stress, tone, vowel harmony, consonant length, prenasalized stops,
syntax and alignment, rival doctrines within a faith. Each catalog law
happens at most once in a lineage, so over long spans languages run
short of changes they have not had, and waves grow rarer with them; real
sound change goes on recycling the same kinds of change.

// Presentation types mirroring `crates/umran-web`. The engine owns all
// linguistic state; these are read-only views of one generation.

export type ContactKind = "neighbours" | "trade" | "rule" | "religion" | "intermarriage";

/// How a people feeds itself, which shapes how many its lands support.
export type Livelihood = "foraging" | "herding" | "farming";

export type Craft = "metalworking" | "riding" | "seafaring" | "writing";

export type LongVowelStyle = "Acute" | "Double" | "Macron" | "Unmarked";

export interface Spelling {
  overrides: [string, string][];
  kw_as_qu: boolean;
  long_vowels: LongVowelStyle;
  mark_hiatus: boolean;
  boundary_mark: string | null;
}

/// A language as designed before founding; mirrors `LanguageDesign`.
export interface LanguageDesign {
  sounds: { ipa: string; favoured: boolean }[];
  wordLength: number;
  finalConsonants: number;
  innerClusters: boolean;
  repetition: number;
  longVowels: number;
  building: "concatenative" | "root-pattern";
  suffixing: number;
  derivation: number;
  grammar?: GrammarDesign;
  spelling: Spelling;
}

export interface SoundInfo {
  ipa: string;
  roman: string;
  vowel: boolean;
  column: string;
  row: string;
  voiced: boolean;
  secondary: "plain" | "aspirated" | "breathy" | "labialized" | "rounded";
  share: number;
}

export interface PreviewWord {
  gloss: string;
  spelled: string;
  ipa: string;
  from: string | null;
}

/// What a people's name means, which decides how it is built from their words.
export type Naming =
  | { kind: "people" }
  | { kind: "speakers" }
  | { kind: "place"; place: string }
  | { kind: "epithet"; epithet: string };

export interface NameView {
  name: string;
  ipa: string;
  meaning: string;
}

export interface Preview {
  people: NameView;
  language: NameView;
  words: PreviewWord[];
  families: PreviewWord[];
  homophones: number;
  syllables: number;
}

export type Action =
  | ({ kind: "settle" } & SettlementChoice)
  /// Without `region`, the world chooses where the people settles.
  | {
      kind: "found";
      naming: Naming;
      design: LanguageDesign;
      seed: number;
      power: number;
      openness: number;
      region?: number;
      /// Without a way of life, the land chooses how the people feeds itself.
      livelihood?: Livelihood;
      /// Leanings the author sets; the rest its land and life choose.
      ethos?: Partial<Ethos>;
    }
  | {
      kind: "found-related";
      source: number;
      region: number;
      naming: Naming;
      livelihood?: Livelihood;
      ethos?: Partial<Ethos>;
    }
  | { kind: "connect"; a: number; b: number; intensity: number; contact: ContactKind }
  | { kind: "shift"; community: number; toward: number }
  | { kind: "state"; community: number; capital?: number }
  | { kind: "religion"; community: number }
  | { kind: "craft"; community: number; craft: Craft }
  | { kind: "law"; variety: number; law: string }
  | { kind: "run"; generations: number }
  /// Fate's hand on a people's temper: nudges one axis, by -1 to 1.
  | { kind: "temper"; community: number; axis: EthosAxis; amount: number };

/// One eligible catalog law and its effect at the exact reading point.
export interface LawChoice {
  id: string;
  label: string;
  words: number;
  specimen: SpecimenWord[];
  recent: boolean;
}

/// A people's temper: six leanings, each from -1 to 1, toward the second
/// word in each name (peaceable to martial, insular to open, and so on).
export interface Ethos {
  martial: number;
  open: number;
  pious: number;
  hierarchical: number;
  roving: number;
  seaward: number;
}

export type EthosAxis = keyof Ethos;

export interface Choice {
  id: string;
  name: string;
  description: string;
}

export interface Catalog {
  mapSizes: Choice[];
  sounds: SoundInfo[];
  places: string[];
  manners: string[];
  heights: string[];
  presets: Choice[];
  contacts: Choice[];
  crafts: Choice[];
  /// Most meanings any language can have words for.
  meanings: number;
  namePlaces: string[];
  nameEpithets: string[];
  /// The engine revision, for the colophon.
  revision: number;
}

export interface Marker {
  generation: number;
  kind: "action" | "run" | "event";
  label: string;
}

export interface Community {
  parents: number[];
  id: number;
  /// What it calls itself, in its own language.
  name: string;
  meaning: string;
  ipa: string;
  coined: number;
  /// The name as first spelled, if sound change has altered it.
  once: string | null;
  /// What each contact calls it.
  exonyms: { by: number; name: string }[];
  variety: number;
  size: number;
  prestige: number;
  power: number;
  openness: number;
  /// Its heart land, where its name appears on the map.
  region: number;
  /// Every land it holds, heart first; its last lands if it has ended.
  lands: number[];
  /// How it feeds itself, which shapes how many its lands support.
  livelihood: Livelihood;
  faith: number | null;
  crafts: Craft[];
  fleet: { since: number; strength: number; ports: number[] } | null;
  /// Its temper now, or as it was when it ended.
  ethos: Ethos;
  /// The generation it ended, or null while it lives.
  ended: number | null;
  /// The people it merged into, or null if it died out.
  endedInto: number | null;
}

export interface MemberView {
  community: number;
  joined: number;
  left: number | null;
}

/// A great city: a state's capital once its people passed the threshold.
export interface CityView {
  id: number;
  state: number;
  region: number;
  name: NameView;
  size: number;
  /// The generation it became a great city.
  since: number;
  /// The people its townsfolk became, once their speech was their own.
  townsfolk: number | null;
  /// What its people speak now, largest share first.
  makeup: Share[];
}

export interface Share {
  variety: number;
  share: number;
}

export interface StateView {
  id: number;
  name: string;
  meaning: string;
  ipa: string;
  once: string | null;
  rulers: number;
  founder: NameView;
  members: MemberView[];
  capital: number;
  rose: number;
  rise: "conquest" | "proclaimed" | "hard-times" | "crowded" | "neighbour" | "comfort";
  fell: number | null;
  fall: "rulers-ended" | "capital-lost" | "conquered" | "collapsed" | null;
  fallenTo: number | null;
  standard: number | null;
  purism: number;
  /// Its standard frozen as a classical form, once fixed: by grammarians
  /// ("age") or by the state's fall ("fall").
  classical: { variety: number; fixed: number; how: "age" | "fall" | "purism" } | null;
  city: number;
  lands: number[];
}

export interface ReligionView {
  id: number;
  name: string;
  meaning: string;
  ipa: string;
  founder: NameView;
  people: number;
  land: number;
  founded: number;
  how: "troubles" | "quiet" | "proclaimed";
  sacred: number;
  converts: boolean;
  translates: boolean;
  doctrine: { tenet: Tenet; stance: number }[];
  disputed: Tenet | null;
  scripture: boolean;
  followers: number[];
  words: RenderingRow[];
  /// The one place the faith reveres, chosen from the lands its founders
  /// knew, its name frozen in the sacred language.
  shrine: Shrine;
  /// The faith it broke from, when, and why; null for a founded faith.
  parent: number | null;
  split: number | null;
  cause: SchismCause | null;
  /// What its name was drawn from: a leader, a land, or an epithet on its
  /// parent's name.
  named: "leader" | "land" | "epithet" | null;
  /// The faiths that broke from it, oldest first.
  branches: number[];
  /// Every holy place it reveres; the first is `shrine`.
  shrines: Shrine[];
  /// Who holds each holy place now, and whether they keep this faith.
  holyLand: HolyLand[];
  /// The roads its pilgrims walk now.
  pilgrims: Pilgrimage[];
}

export type SchismCause = "distance" | "rule" | "reform" | "succession";

export interface HolyLand {
  region: number;
  heldBy: number | null;
  faithful: boolean;
}

export interface Pilgrimage {
  /// The pilgrims' people, and their home land.
  people: number;
  from: number;
  /// The holy place they go to.
  to: number;
  /// Lands along the road, from `from` to `to`.
  path: number[];
  legs?: TravelLeg[];
  since: number;
}

export type StressRule = "initial" | "penult" | "final" | "weight" | "free";

export type ShrineKind = "home" | "mountain" | "island" | "far-shore";

export interface Shrine {
  region: number;
  /// Why it was revered: the founders' home, a mountain, an island, or a
  /// shore on another continent.
  kind: ShrineKind;
  name: NameView;
}

export interface CraftView {
  id: Craft;
  name: string;
  first: number | null;
  inventors: number[];
  holders: number[];
  words: RenderingRow[];
}

export interface RenderingRow {
  concept: string;
  gloss: string;
  renderings: Rendering[];
}

export interface Rendering {
  variety: number;
  spelled: string;
  ipa: string;
  how: "borrowed" | "kept" | "stretched" | "built" | "coined" | "inherited";
  from: string | null;
}

export interface GivenView {
  name: string;
  ipa: string;
  meaning: string;
  from: number | null;
}

export interface OwnWords {
  meanings: number;
  own: number;
  loans: number;
  shared: number;
}

/// A sound law a variety underwent; `from` is the variety it spread from,
/// if it came as a wave from a neighbour rather than arising there.
export interface Law {
  generation: number;
  id: string;
  label: string;
  from: number | null;
  decision: number | null;
}

export interface Variety {
  id: number;
  /// What its speakers call it.
  name: string;
  meaning: string;
  parent: number | null;
  forkedAt: number | null;
  /// The languages a city's townsfolk levelled into this one, and each
  /// one's share of the city then, largest first; null if it is no koiné.
  koineOf: Share[] | null;
  family: number;
  spoken: boolean;
  /// The generation it arose: founded, parted from its parent, or taken up
  /// in a shift.
  born: number;
  /// When its last speakers took up another language or ended, if they have.
  silentSince: number | null;
  consonants: string[];
  vowels: string[];
  laws: Law[];
  words: number;
  wordBuilding: string;
  builders: { relation: string; shape: string }[];
  /// The smallest word sound change leaves, such as "two syllables".
  minimalWord: string;
  /// Where the language puts its stress.
  stress: StressRule;
  /// Whether its living words have long consonants, as Italian *fatto*.
  geminates: boolean;
  /// Distinct marked lexical tones in living words.
  tones: number;
  tonal: { since: number; lost: number | null } | null;
  /// A few basic words, to know the language by.
  specimen: SpecimenWord[];
  pronouns: {
    cell: string;
    clusivity: "inclusive" | "exclusive" | null;
    person: 1 | 2 | 3;
    number: "sg" | "pl";
    polite: boolean;
    spelled: string;
    ipa: string;
    since: number;
    origin: "founding" | "renewed" | "borrowed";
    source: string | null;
    /// The dominant object form at this reading, or null without an audible object contrast.
    object: { spelled: string; ipa: string; kind: "bound" | "particle" | "pattern"; since: number } | null;
  }[];
  harmony: { feature: "backness" | "rounding" | "atr"; since: number; lost: number | null } | null;
  grammar: {
    order: WordOrder;
    possessor: PossessorOrder;
    marking: "case" | "order";
    sample?: { sentence: GrammarRendering; possession: GrammarRendering; future: GrammarRendering; pronoun: { first: GrammarRendering; second: GrammarRendering } | null };
    /// Where the pronouns stand against the nouns on object marking.
    pronounCase: PronounCase;
    markers: GrammarMarker[];
    classes: NounClass[];
    categories: {
      category: GrammarCategory;
      label: string;
      description: string;
      eligible: number;
      howSynthetic: number;
      contrastRetention: number;
    }[];
  };
  standardOf: number | null;
  ownWords: OwnWords;
  names: GivenView[];
  nameStyle: "single" | "double";
  written: number | null;
  purism: { since: number; cause: Cause | null; replaced: { concept: string; loan: string; native: string }[] }[];
  sacredOf: number | null;
  /// The state whose classical form it is, if it is one.
  classicalOf: number | null;
  /// The high written register (sacred, classical, or court standard) its
  /// speakers write, or wrote before writing their own speech.
  high: number | null;
  /// When its speakers began to write their own speech in place of `high`.
  vernacular: number | null;
  /// 0–1: how much of the core vocabulary its speech still shares with
  /// `high`, while it is spoken.
  keptFromHigh: number | null;
  /// Lands its speakers know by name, as they say them: lands they hold or
  /// held, border, or have heard of from others. Ordered by region.
  knownLands: KnownLand[];
}

export interface KnownLand {
  region: number;
  spelled: string;
  ipa: string;
}

/// One of the few basic words shown wherever a language appears.
export interface SpecimenWord {
  concept: string;
  gloss: string;
  spelled: string;
  ipa: string;
  /// In a sound change's entry, how it was spelled before, if it changed.
  was: string | null;
  /// The stressed syllable, counted from 0; null for a word of one syllable.
  stress: number | null;
  /// In a sound change's entry, how it sounded before, if the sound or the
  /// stress changed, even when the spelling did not.
  wasIpa: string | null;
}

/// Another spoken language and its chance-corrected sound similarity for
/// current forms expressing the same core meanings, not a count of roots.
export interface Kin {
  other: number;
  score: number;
}

export interface Contact {
  a: number;
  b: number;
  intensity: number;
  kind: ContactKind;
}

export interface HistoryPoint {
  action: number;
  offset: number;
}

export interface Overview {
  telling: number;
  atTip: boolean;
  point: HistoryPoint;
  mutation: number;
  seed: number;
  generation: number;
  latest: number;
  revision: number;
  savedRevision: number | null;
  timeline: Marker[];
  communities: Community[];
  varieties: Variety[];
  states: StateView[];
  /// Every great city that has grown, standing or fallen.
  cities: CityView[];
  religions: ReligionView[];
  crafts: CraftView[];
  contacts: Contact[];
  /// What each land that has been held is called, through history.
  places: Place[];
  /// Peoples going to new land: migrations, and split-offs settling away.
  moves: Move[];
  annals: Annal[];
  /// Histories set aside, with what they told that this one does not.
  tellings: TellingView[];
  /// Each continent (not island), with its name if anyone has given one.
  /// Not indexed by landmass: find by `landmass`.
  continents: ContinentView[];
}

/// A continent as the chart knows it in the year in view.
export interface ContinentView {
  landmass: number;
  /// Entered on the chart from the speech of the first people to know it;
  /// fixed once given. Null while no living people knows any of it.
  name: ContinentName | null;
  /// Living peoples holding land on it.
  peoples: number[];
  /// Faiths whose shrine is on it.
  religions: number[];
}

export interface ContinentName extends NameView {
  variety: number;
  people: number;
  /// The region of it that people knew when they named it.
  witness: number;
  since: number;
}

export interface Seasons {
  amplitude: number;
  wet: "none" | "summer" | "winter" | "monsoon";
  floods: boolean;
}

/// A land's names, oldest first; the last is its name now.
export interface Place {
  region: number;
  seasons: Seasons;
  names: PlaceName[];
  /// What speakers of other living languages call it now, each heard once
  /// from its holders and changed since by their own sound laws.
  exonyms: PlaceExonym[];
}

export interface PlaceExonym {
  variety: number;
  language: string;
  spelled: string;
  ipa: string;
  /// The generation its speakers first heard of the land.
  heard: number;
  /// How it was spelled when they heard it, if it has changed.
  once: string | null;
}

export interface PlaceName {
  /// The generation its speakers came to hold the land.
  since: number;
  variety: number;
  /// The language it is a name in, as that language was called then.
  language: string;
  spelled: string;
  ipa: string;
  meaning: string;
  origin: "coined" | "inherited" | "kept" | "borrowed";
  /// Who coined it, for a coined name.
  by: number | null;
  /// How it was spelled when its speakers took it up, if it has changed.
  once: string | null;
}

export interface TravelLeg {
  from: number;
  to: number;
  by: "land" | "sea" | "river";
  km: number;
}

export interface Move {
  path: number[];
  legs: TravelLeg[];
  generation: number;
  community: number;
  from: number;
  to: number;
  kind: "migration" | "split";
  /// Whether they crossed the sea.
  bySea: boolean;
}

/// A history set aside by undoing, or by writing on from an earlier year.
export interface ReadingRef { telling: number; point: HistoryPoint }

/// A subject is resolved within a telling, never against another account's IDs.
export type Subject =
  | { kind: "world" } | { kind: "history" }
  | { kind: "people"; id: number } | { kind: "state"; id: number }
  | { kind: "religion"; id: number } | { kind: "craft"; id: Craft }
  | { kind: "language"; variety: number }
  | { kind: "word"; variety: number; concept: string }
  | { kind: "law"; id: string; variety?: number; generation?: number; index?: number } | { kind: "land"; region: number }
  | { kind: "river"; id: number } | { kind: "lake"; id: number } | { kind: "zone"; id: number }
  | { kind: "continent"; landmass: number } | { kind: "event"; id: string };
export interface Destination { reading: ReadingRef; subject: Subject }
export interface NotebookNote {
  id: string;
  title: string;
  body: string;
  kind: "observation" | "question" | "year";
  target: Destination | null;
  label: string;
  generation: number;
  revision: number;
  archived: boolean;
}
export interface TellingView {
  id: number;
  name: string;
  parent: ReadingRef | null;
  from: number | null;
  latest: number;
  tip: HistoryPoint;
  actions: number;
}
export interface Comparison {
  generation: number;
  common: ReadingRef;
  diverged: number;
  sharedPeoples: number[];
  sharedLanguages: number[];
  left: Overview;
  right: Overview;
}

/// Why a zone's weather or a river's flow changed, as the engine records it.
export type ClimateCause = "drought" | "cold-spell" | "recovery" | "long-drying";

/// One chronicle entry; `variety` is set for a sound law.
export interface Annal {
  id: string;
  members: Annal[];
  languages: number[];
  settlement?: SettlementRecord;
  /// Set on every entry a decision produced: the point just before it.
  before?: HistoryPoint;
  /// The index of the decision that produced this entry, if one did.
  decision?: number;
  generation: number;
  kind: "found" | "split" | "settlement" | "migration" | "shift" | "contact" | "parted" | "neighbours" | "conquest" | "law" | "spread" | "displaced" | "hardship" | "climate" | "river-flow" | "livelihood" | "ended" | "rose" | "fell" | "standard" | "classical" | "vernacular" | "craft" | "faith" | "conversion" | "meaning" | "respelling" | "schism" | "pilgrimage" | "holy-land" | "city" | "koine" | "temper" | "grammar" | "pronoun-renewed" | "pronoun-polite" | "pronoun-generalised" | "pronoun-borrowed" | "class-emerged" | "class-merged" | "class-lost" | "harmony-gained" | "harmony-lost" | "tone-gained" | "tone-lost" | "coinage" | "calque" | "purist-reform" | "purist-replacement" | "tenet-adopted" | "tenet-disputed" | "taboo-replacement" | "drought" | "hard-winter" | "flood" | "fleet-built" | "fleet-lost" | "sea-route-opened";
  /// The annalist's words; words of the language are marked *thus*.
  text: string;
  /// Recorded names in appearance order; start is a UTF-16 offset in text.
  names: { text: string; subject: Subject; start: number }[];
  /// The apparatus: what a linguist would note, such as sound laws.
  notes: string[];
  cause?: Cause;
  variety: number | null;
  /// The peoples it tells of.
  peoples: number[];
  /// The lands it tells of: where peoples went, and where from.
  lands: number[];
  /// The states it tells of.
  states: number[];
  religions: number[];
  crafts: Craft[];
  /// The sound laws it tells of, by id.
  laws: string[];
  /// For a sound change, the language's specimen words after it.
  specimen: SpecimenWord[];
  /// For a change of temper, the leaning, which end of it, whether the
  /// people came to lean that way or ceased to, and the engine's id for
  /// what caused it.
  temper: { axis: EthosAxis; pole: "high" | "low"; entered: boolean; cause: string } | null;
  doctrine?: { tenet: Tenet; previous: number | null; stance: number };
  taboo?: { concept: string; old: number; word: number };
  /// Climate zones and rivers it tells of.
  zones: number[];
  rivers: number[];
  /// For a change of climate: the zone, the engine's cause, and how far
  /// rain and warmth departed from the zone's usual weather.
  climate: { zone: number; cause: ClimateCause; change: "onset" | "worsening" | "recovery"; severity: number; wetness: number; warmth: number } | null;
  /// For a river's flow weakening or recovering.
  riverFlow: { river: number; cause: ClimateCause; flowing: boolean } | null;
}

export type LoanCause =
  | { kind: "contact"; contact: ContactKind; donor: number; recipient: number; since: number; event: string | null }
  | { kind: "rule"; ruler: number; ruled: number; state: number | null; event: string | null }
  | { kind: "faith"; religion: number; teacher: number | null; recipient: number; event: string | null }
  | { kind: "shift"; community: number; fromVariety: number; event: string | null }
  | { kind: "city"; city: number; community: number; event: string | null }
  | { kind: "coinage"; peoples: number[]; event: string | null }
  | { kind: "classical"; classical: number; recipient: number; event: string | null }
  | { kind: "unrecorded" };

export interface Story { annals: string[] }

export interface Origin {
  kind: "inherited" | "coined" | "borrowed" | "kept" | "derived";
  from: string | null;
  generation: number;
  fromVariety: number | null;
  cause: LoanCause | null;
}

export interface Coined {
  parts: { concept: string; spelled: string; gloss: string }[];
  kind: "compound" | "derived" | "calque";
  generation: number;
  from?: number;
  opaqueSince: number | null;
}

export interface FormView { spelled: string; ipa: string }

export interface LawOccurrenceRef {
  variety: number; law: string; generation: number; index: number;
}

export type ChangeCause =
  | { kind: "law"; law: string; label: string; occurrence: LawOccurrenceRef | null }
  | { kind: "loan"; cause: LoanCause }
  | { kind: "coined"; coined: Coined }
  | { kind: "inherited"; from: number }
  | { kind: "founding" }
  | { kind: "kept"; from: number }
  /// `from` is the recorded first meaning; intermediate semantic steps may be unknown.
  | { kind: "extended"; from: string }
  | { kind: "expressive" }
  | { kind: "marker"; origin: GrammarOrigin };

export interface LawOccurrence {
  ref: LawOccurrenceRef;
  label: string;
  arrival: { kind: "own" } | { kind: "wave"; from: number } | { kind: "inherited"; ancestor: number }
    | { kind: "authored"; decision: number } | { kind: "substrate" } | { kind: "unrecorded" };
  annal: string | null;
  words: { concept: string; gloss: string; before: FormView; after: FormView; dominant: boolean }[];
  markers: { category: string; marker: string; before: FormView; after: FormView }[];
  names: { subject: Subject; before: string; after: string }[];
}

export interface WordLife {
  concept: string; gloss: string;
  /// Recorded holders only, with the current holder last. Competition weights
  /// are not a historical series: an unknown dominance onset stays null.
  holders: {
    forms: { generation: number; form: FormView; cause: ChangeCause }[];
    entered: number;
    dominantFrom: number | null;
    until: number | null;
    share: number;
  }[];
}

export interface LoanTrail {
  concept: string;
  donor: { variety: number; form: FormView };
  cause: LoanCause;
  heard: FormView;
  adapted: FormView;
  since: number;
  after: { generation: number; form: FormView; cause: ChangeCause }[];
  sameMeeting: { concept: string; gloss: string; form: FormView }[];
}

export interface GrammarStory {
  category: string; label: string;
  markers: {
    marker: string; kind: string;
    source: { concept: string; gloss: string; form: FormView } | null;
    born: number; retired: number | null;
    /// Only recorded observations; no interpolation of past competition.
    shares: { generation: number; share: number }[];
    forms: { generation: number; form: FormView; cause: ChangeCause | { kind: "analogy" } }[];
  }[];
}

export interface BuilderExample {
  builder: string;
  base: { concept: string; gloss: string; form: FormView };
  built: { concept: string; gloss: string; form: FormView };
}

/// One aligned position: an IPA segment on each side, or null for a gap.
export interface Segment { a: string | null; b: string | null }

/// The comparative method on two languages' current forms (compare.rs). It never reads lineage.
export interface Correspondences {
  a: number;
  b: number;
  generation: number;
  /// The meaning set used: "core", "specimen", or a semantic field id from `meaningSets`.
  meanings: string;
  rows: {
    concept: string;
    gloss: string;
    a: FormView;
    b: FormView;
    alignment: Segment[];
    /// Share of this row's aligned pairs that are recurring correspondences, 0 to 1.
    regularity: number;
    /// The method's verdict: the pair looks cognate.
    cognate: boolean;
  }[];
  /// Recurring correspondences, strongest first; `support` counts the rows that show the pair.
  regular: { pair: Segment; support: number }[];
  /// Grammatical observations the method used.
  grammar: { category: string; a: string; b: string; regular: boolean }[];
}

/// The answer key for the same rows, kept apart from `Correspondences`.
export interface Truth {
  rows: { concept: string; sameRoot: boolean; borrowed: { into: number; from: number; cause: LoanCause } | null }[];
  /// Nearest common ancestor variety, when the two are related.
  ancestor: number | null;
  /// The generation they parted.
  splitAt: number | null;
}

export interface MeaningSet { id: string; label: string }

/// One side of a shared language in `SpeechDelta`.
export interface SpeechSide {
  alive: boolean;
  /// Languages descended from it on this side after the divergence.
  descendants: number;
  /// Laws applied after the divergence on this side.
  laws: { id: string; label: string; generation: number }[];
  specimen: SpecimenWord[];
}

/// What diverged in speech between two tellings at one year, for the languages they share
/// from before the divergence. Branch-local descendants are counted, never paired by id.
export interface SpeechDelta {
  generation: number;
  /// The generation the two tellings parted.
  diverged: number;
  /// The first decision after the divergence in the right telling.
  decision: DecisionView | null;
  languages: {
    /// A variety id both tellings share from before the divergence.
    variety: number;
    left: SpeechSide;
    right: SpeechSide;
    /// Law ids applied after the divergence on one side only.
    onlyLeft: string[];
    onlyRight: string[];
    /// Specimen meanings whose current forms differ.
    changedWords: { concept: string; gloss: string; left: FormView | null; right: FormView | null }[];
    /// Loans after the divergence, counted by donor variety.
    loansLeft: { donor: number; count: number }[];
    loansRight: { donor: number; count: number }[];
  }[];
}

export interface LexiconForm {
  spelled: string;
  said: string | null;
  ipa: string;
  origin: Origin;
  coined?: Coined;
  changes: number;
}

export interface LexiconRow extends LexiconForm {
  concept: string;
  gloss: string;
  field: string;
  rank: number | null;
  class?: number;
  /// Current competing forms other than the dominant word above.
  competingForms: LexiconForm[];
  competitors: number;
}

export interface HistoryLine {
  generation: number;
  text: string;
  cause?: LoanCause;
}

export interface Variant {
  spelled: string;
  said: string | null;
  ipa: string;
  share: number;
  class?: number;
  origin: Origin;
  coined?: Coined;
  senses: string[];
  history: HistoryLine[];
}

export interface WordDetail {
  concept: string;
  gloss: string;
  field: string;
  rank: number | null;
  related: string[];
  variants: Variant[];
  cognates: { variety: number; name: string; spelled: string; ipa: string }[];
}

export type MapSize = "small" | "medium" | "large" | "vast";

export type GeographyVersion = "spherical-v1" | "continental-v2" | "continental-v3" | "continental-v4" | "continental-v5" | "continental-v6";

export type Terrain = "sea" | "plains" | "forest" | "steppe" | "hills" | "mountains" | "desert";

export interface Region {
  id: number;
  terrain: Terrain;
  areaKm2: number;
  elevation: number;
  moisture: number;
  warmth: number;
  climateZone: number | null;
  /// Canonical longitude east and latitude north, in degrees.
  center: [number, number];
  /// Clockwise unclosed small-polygon ring in canonical geographic degrees.
  boundary: [number, number][];
  /// Derived equirectangular drawing coordinates, not a physical distance metric.
  site: [number, number];
  coastal: boolean;
  /// Land on an island rather than a continent.
  island: boolean;
  /// Regions sharing a border with it.
  neighbours: number[];
  /// Its body of land, an index into `WorldMap.landmasses`; null for sea.
  landmass: number | null;
}

/// One connected body of land.
export interface Landmass {
  id: number;
  kind: "continent" | "island";
  /// Its regions, in increasing order.
  regions: number[];
  /// The member region nearest its centre, where its name is written.
  anchor: number;
}

export interface Lake {
  id: number;
  regions: number[];
  /// River carrying overflow; null for a closed basin.
  outlet: number | null;
}

/// The land a book's history plays out on; it never changes.
export interface WorldMap {
  size: MapSize;
  geography: GeographyVersion;
  radiusKm: number;
  width: number;
  height: number;
  /// Equatorial equirectangular drawing scale; use engine metrics for distances.
  kmPerUnit: number;
  regions: Region[];
  landmasses: Landmass[];
  rivers: River[];
  lakes: Lake[];
  climateZones: { id: number; regions: number[] }[];
}

export interface River {
  id: number;
  course: number[];
  channel: [number, number, number][];
  mouth: number;
  catchment: number[];
  joins: number | null;
  /// Exact downstream region at the confluence; null for a sea or closed lake outlet.
  joinAt: number | null;
  /// Geodesic length along the canonical channel (legacy versions use region centers).
  lengthKm: number;
}

export interface ClimateView {
  generation: number;
  zones: {
    id: number;
    epoch: number;
    remaining: number;
    wetness: number;
    warmth: number;
    targetWetness: number;
    targetWarmth: number;
    cause: ClimateCause;
    severity: number;
  }[];
  regions: {
    id: number;
    zone: number | null;
    wetness: number;
    warmth: number;
    vegetation: Terrain;
    riverFlow: number;
    feeding: Record<Livelihood, number>;
    severe: boolean;
    seasons: Seasons;
  }[];
  rivers: { id: number; flow: number; flowing: boolean }[];
}

export interface RiverNamesView {
  river: number;
  names: PlaceName[];
  exonyms: PlaceExonym[];
}

export interface LakeNamesView {
  lake: number;
  names: PlaceName[];
  exonyms: PlaceExonym[];
}

/// Every people's word for one meaning, as a dialect atlas shows it.
export interface WordMap {
  concept: string;
  gloss: string;
  words: {
    community: number;
    spelled: string;
    ipa: string;
    /// Words sharing a group descend from one root.
    group: number;
    origin: Origin;
  }[];
}

/// Pure year-zero travel evidence for the founding accounts.
export interface FoundingPreview {
  pairs: {
    a: number;
    b: number;
    walk: number | null;
    voyage: number | null;
    reach: "neighbours" | "walking" | "sea" | "apart";
    sameLandmass: boolean;
  }[];
  peoples: {
    community: number;
    coastal: boolean;
    landmass: number;
    nearestOther: number | null;
  }[];
}

/// An engine answer from its thread. React's `use` reads one synchronously
/// once it has settled; until then the reading component suspends.
export type Read<T> = Promise<T> & ({ status: "pending" } | { status: "fulfilled"; value: T } | { status: "rejected"; reason: unknown });

/// One reading of a telling, fixed at the history it was opened on. Its
/// answers are kept, so a component re-rendered with the same reading and
/// arguments gets the same promise back.
export interface ReadEngine {
  overviewAt(point: HistoryPoint): Read<Overview>;
  settlement(point: HistoryPoint, community: number, intent: SettlementIntent, share: number, destination: number | null): Read<SettlementPreview>;
  lawChoices(point: HistoryPoint, variety: number): Read<LawChoice[]>;
  contactReach(point: HistoryPoint, a: number, b: number, contact: ContactKind): Read<ContactReach>;
  latest(): Read<number>;
  overview(generation: number): Read<Overview>;
  lexicon(generation: number, variety: number): Read<LexiconRow[]>;
  kin(generation: number, variety: number): Read<Kin[]>;
  word(generation: number, variety: number, concept: string): Read<WordDetail>;
  lawOccurrences(generation: number, variety: number, law: string): Read<LawOccurrence[]>;
  specimenLives(generation: number, variety: number): Read<WordLife[]>;
  wordLife(generation: number, variety: number, concept: string): Read<WordLife>;
  loanTrail(generation: number, variety: number, concept: string): Read<LoanTrail | null>;
  grammarStory(generation: number, variety: number, category: string): Read<GrammarStory>;
  builderExamples(generation: number, variety: number): Read<BuilderExample[]>;
  /// The comparative method on current forms; never reads lineage.
  correspondences(generation: number, a: number, b: number, meanings: string): Read<Correspondences>;
  /// What the record knows about the same rows.
  truth(generation: number, a: number, b: number, meanings: string): Read<Truth>;
  meaningSets(): Read<MeaningSet[]>;
  story(generation: number, subject: Subject): Read<Story>;
  climate(generation: number): Read<ClimateView>;
  river(generation: number, id: number): Read<RiverNamesView>;
  lake(generation: number, id: number): Read<LakeNamesView>;
  /// Every river's and every lake's names, in map order, for the chart.
  rivers(generation: number): Read<RiverNamesView[]>;
  lakes(generation: number): Read<LakeNamesView[]>;
  map(): Read<WorldMap>;
  wordMap(generation: number, concept: string): Read<WordMap>;
  save(): string;
  /// The author's decisions in this reading's telling, up to its point.
  decisions(): Read<DecisionView[]>;
}
/// How the coasts formed: the supercontinent, ten plate-drift steps, then
/// the drawn coasts. Each string has one char per map region.
export interface Forming {
  frames: { land: string; plate: string }[];
}
/// What land and life give a founding people before the author leans on
/// them, and what each way of life would mean on their homeland.
export interface FoundingTraits {
  drawn: Ethos;
  suggested: Livelihood;
  /// Souls their homeland would feed under each way of life.
  feeds: Record<Livelihood, number>;
}
/// An open world on the engine's thread. What a reload must save, and what
/// the page shows of the world itself, is kept here synchronously and
/// refreshed by every change; everything else is read through a `ReadEngine`.
export interface Engine {
  latest(): number;
  save(): string;
  /// The author's title and name, kept in the save; empty clears.
  title(): string | null;
  author(): string | null;
  notebook(): NotebookNote[];
  /// The present telling as it stands; a new reading after every change.
  tip(): ReadEngine;
  read(telling: number, point?: HistoryPoint | null): ReadEngine;
  map(): Read<WorldMap>;
  /// Copy an untouched year-zero base, sharing its generated geography.
  foundingDraft(): Promise<Engine>;
  foundingPreview(): Read<FoundingPreview>;
  /// Up to 12 nearby unoccupied year-zero lands, in land-travel order.
  /// An unoccupied anchor comes first; an occupied anchor is only the origin.
  foundingSites(region: number, count: number): Read<number[]>;
  /// Year 0 only: a founding people's drawn temper and their land's yields.
  foundingTraits(community: number): Read<FoundingTraits>;
  /// The coasts as they formed; null for worlds not drawn from moving plates.
  forming(): Read<Forming | null>;
  saveNote(note: NotebookNote): Promise<void>;
  resolveNote(id: string): Promise<Destination | null>;
  removeNote(id: string): Promise<void>;
  setTitle(title: string): Promise<void>;
  setAuthor(author: string): Promise<void>;
  previous(reading: ReadingRef): Promise<ReadingRef>;
  compare(left: number, right: number, generation: number): Read<Comparison>;
  speechDelta(left: number, right: number, generation: number): Read<SpeechDelta>;
  rename(telling: number, name: string): Promise<void>;
  actAt(reading: ReadingRef, mutation: number, action: Action): Promise<void>;
  untilAt(reading: ReadingRef, mutation: number, limit: number): Promise<number>;
  act(action: Action): Promise<void>;
  runUntilEvent(limit: number): Promise<number>;
  branch(generation: number): Promise<void>;
  restore(telling: number): Promise<void>;
  dispose(): void;
}

/// The engine's physical-access check, shared with the contact decision.
export interface ContactReach {
  ok: boolean;
  reason: string | null;
}

/// Years per generation, for display only.
export const YEARS = 25;

export type SettlementIntent = "partition" | "settlers" | "migration";
export interface SettlementChoice {
  community: number;
  intent: SettlementIntent;
  destination: number;
  share: number;
  naming: Naming | null;
  intensity: number;
}
export interface Allocation {
  lands: number[];
  population: number;
  presence: [number, number][];
}
export interface SettlementRoute {
  from: number;
  to: number;
  population: number;
  effort: number;
  by_sea: boolean;
  path: number[];
  legs?: TravelLeg[];
}
export interface SettlementPlan {
  choice: SettlementChoice;
  before: Allocation;
  remaining: Allocation;
  arriving: Allocation;
  routes: SettlementRoute[];
  inhabitants: [number, number][];
  capacity: number;
  room: number;
  falling_states: number[];
  notes: string[];
}
export interface SettlementRecord { plan: SettlementPlan; daughter: number | null }
export interface SettlementPreview {
  telling: number;
  point: HistoryPoint;
  mutation: number;
  options: { region: number; reason: string | null; effort: number | null }[];
  plan: SettlementPlan | null;
  reason: string | null;
}

/// Why a response happened, where the engine knew its trigger when it
/// decided the response. Absent when it did not; never inferred.
export type Mechanism =
  | "hardship"
  | "crowding"
  | "stronger-neighbour"
  | "word-need"
  | "climate"
  | "craft"
  | "conquest"
  | "city"
  | "court"
  | "contact"
  | "pilgrimage"
  | "unfaithful-holder"
  | "foreign-prestige"
  | "religious-revival"
  | "purist-norm"
  | "doctrine";

export type Tenet = "sacred-language" | "images" | "hierarchy" | "purity" | "pilgrimage" | "monasticism";

export interface Cause {
  /// The annal that triggered this one.
  event: number;
  mechanism: Mechanism;
}

/// An author's decision in the current telling, as the engine recorded it.
export interface DecisionView {
  /// The action's index in its telling.
  index: number;
  generation: number;
  kind: "settle" | "found" | "found-related" | "connect" | "shift" | "state" | "religion" | "craft" | "temper" | "law";
  /// One sentence in the chronicle's voice.
  text: string;
  people: number[];
  /// The resulting language for founding, settlement, or shift.
  variety: number | null;
  /// Original annal ids, including members of grouped entries.
  annals: string[];
}

export type GrammarCategory = "plural" | "past" | "object" | "future" | "progressive" | "genitive";
export type GrammarChoice = "suffix" | "prefix" | "particle" | "none";
export type WordOrder = "SOV" | "SVO" | "VSO";
export type PossessorOrder = "before" | "after";
/// none: no pronoun has an audible object form; like-nouns: pronouns mark the
/// object as nouns do; ahead: pronouns carry a bound form while nouns still
/// use a separate word; relic: pronouns keep a contrast the nouns have lost.
export type PronounCase = "none" | "like-nouns" | "ahead" | "relic";

export interface GrammarDesign {
  plural: GrammarChoice;
  past: GrammarChoice;
  /// Null or omitted means draw at founding.
  object?: GrammarChoice | null;
  genitive?: GrammarChoice | null;
  order?: WordOrder | null;
  possessor?: PossessorOrder | null;
  future?: GrammarChoice | null;
  progressive?: GrammarChoice | null;
  classes?: "none" | "sex" | "animacy" | "many" | null;
}

export interface NounClass {
  id: number;
  basis: "animacy" | "sex" | "shape" | "formal";
  marker: { spelled: string; ipa: string } | null;
  members: number;
  born: number;
  mergedInto: number | null;
}

export interface GrammarRendering {
  text: string;
  ipa: string;
  gloss: string[];
  translation: string;
}

export type GrammarOrigin =
  | { kind: "founding" }
  | { kind: "grammaticalized"; source: number; concept: string; gloss: string; sourceForm: { form: string; ipa: string } }
  | { kind: "fused"; particle: number }
  | { kind: "imported"; from: number; language: string; marker: number; source: { form: string; ipa: string } };

export interface GrammarMarker {
  id: number;
  category: GrammarCategory;
  kind: "bound" | "particle" | "none" | "pattern";
  template?: string;
  side: "prefix" | "suffix";
  form: string;
  spelled: string;
  said: string | null;
  ipa: string;
  alternants?: { spelled: string; ipa: string }[];
  share: number;
  origin: GrammarOrigin;
  born: number;
  retired: number | null;
  productive: boolean;
  /// A marker on pronouns only never spreads to nouns.
  scope: "all" | "pronouns";
  history: HistoryLine[];
}

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
    }
  | { kind: "connect"; a: number; b: number; intensity: number; contact: ContactKind }
  /// Without `naming`, the new community chooses its own name.
  | { kind: "split"; community: number; naming?: Naming; intensity: number }
  | { kind: "shift"; community: number; toward: number }
  | { kind: "state"; community: number; capital?: number }
  | { kind: "religion"; community: number }
  | { kind: "craft"; community: number; craft: Craft }
  | { kind: "run"; generations: number };

export interface Choice {
  id: string;
  name: string;
  description: string;
}

export interface Catalog {
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
  scripture: boolean;
  followers: number[];
  words: RenderingRow[];
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
}

export interface Variety {
  id: number;
  /// What its speakers call it.
  name: string;
  meaning: string;
  parent: number | null;
  forkedAt: number | null;
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
  /// A few basic words, to know the language by.
  specimen: SpecimenWord[];
  standardOf: number | null;
  ownWords: OwnWords;
  names: GivenView[];
  nameStyle: "single" | "double";
  written: number | null;
  sacredOf: number | null;
}

/// One of the few basic words shown wherever a language appears.
export interface SpecimenWord {
  concept: string;
  gloss: string;
  spelled: string;
  ipa: string;
  /// In a sound change's entry, how it was spelled before, if it changed.
  was: string | null;
}

/// Another spoken language and the share of core words it shares with one.
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

export interface Overview {
  seed: number;
  generation: number;
  latest: number;
  revision: number;
  savedRevision: number | null;
  timeline: Marker[];
  communities: Community[];
  varieties: Variety[];
  states: StateView[];
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
}

/// A land's names, oldest first; the last is its name now.
export interface Place {
  region: number;
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

export interface Move {
  generation: number;
  community: number;
  from: number;
  to: number;
  kind: "migration" | "split";
  overseas: boolean;
}

/// A history set aside by undoing, or by writing on from an earlier year.
export interface TellingView {
  index: number;
  why: "undone" | "rewritten";
  /// The generation from which it tells otherwise.
  from: number;
  struck: Annal[];
}

/// One chronicle entry; `variety` is set for a sound law.
export interface Annal {
  generation: number;
  kind: "found" | "split" | "migration" | "shift" | "contact" | "parted" | "neighbours" | "conquest" | "law" | "spread" | "displaced" | "hardship" | "livelihood" | "ended" | "rose" | "fell" | "standard" | "craft" | "faith" | "conversion" | "meaning" | "respelling";
  /// The annalist's words; words of the language are marked *thus*.
  text: string;
  /// The apparatus: what a linguist would note, such as sound laws.
  notes: string[];
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
}

export interface Origin {
  kind: "inherited" | "coined" | "borrowed" | "kept" | "derived";
  from: string | null;
  generation: number;
}

export interface LexiconRow {
  concept: string;
  gloss: string;
  field: string;
  rank: number | null;
  spelled: string;
  said: string | null;
  ipa: string;
  origin: Origin;
  changes: number;
  competitors: number;
}

export interface HistoryLine {
  generation: number;
  text: string;
}

export interface Variant {
  spelled: string;
  said: string | null;
  ipa: string;
  share: number;
  origin: Origin;
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

export type MapSize = "small" | "medium" | "large";

export type Terrain = "sea" | "plains" | "forest" | "steppe" | "hills" | "mountains" | "desert";

export interface Region {
  id: number;
  terrain: Terrain;
  /// The point the region was drawn around, in map units.
  site: [number, number];
  outline: [number, number][];
  coastal: boolean;
  /// Land on a body of land of at most two regions.
  island: boolean;
  /// Regions sharing a border with it.
  neighbours: number[];
}

/// The land a book's history plays out on; it never changes.
export interface WorldMap {
  size: MapSize;
  width: number;
  height: number;
  regions: Region[];
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

export interface Engine {
  act(action: Action): void;
  undo(): boolean;
  runUntilEvent(limit: number): number;
  branch(generation: number): void;
  /// Takes up a telling set aside, setting the present one aside.
  restore(index: number): void;
  latest(): number;
  overview(generation: number): Overview;
  lexicon(generation: number, variety: number): LexiconRow[];
  /// The share of core words `variety` shares with each other spoken
  /// language, the closest first.
  kin(generation: number, variety: number): Kin[];
  word(generation: number, variety: number, concept: string): WordDetail;
  map(): WorldMap;
  wordMap(generation: number, concept: string): WordMap;
  save(): string;
  dispose(): void;
}

/// Years per generation, for display only.
export const YEARS = 25;

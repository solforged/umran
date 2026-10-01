// Presentation types mirroring `crates/umran-web`. The engine owns all
// linguistic state; these are read-only views of one generation.

export type ContactKind = "neighbours" | "trade" | "rule" | "religion" | "intermarriage";

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
    }
  | { kind: "connect"; a: number; b: number; intensity: number; contact: ContactKind }
  /// Without `naming`, the new community chooses its own name.
  | { kind: "split"; community: number; naming?: Naming; intensity: number }
  | { kind: "shift"; community: number; toward: number }
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
  /// The map region it lives on.
  region: number;
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
  /// When its last speakers took up another language, if they have.
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
  kind: "found" | "split" | "migration" | "shift" | "contact" | "parted" | "neighbours" | "conquest" | "law";
  /// The annalist's words; words of the language are marked *thus*.
  text: string;
  /// The apparatus: what a linguist would note, such as sound laws.
  notes: string[];
  variety: number | null;
  /// The peoples it tells of.
  peoples: number[];
  /// The lands it tells of: where peoples went, and where from.
  lands: number[];
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

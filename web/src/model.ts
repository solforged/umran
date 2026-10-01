// Presentation types mirroring `crates/langgen-web`. The engine owns all
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
  | { kind: "found"; naming: Naming; design: LanguageDesign; seed: number; power: number; openness: number }
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
  profile: string;
  consonants: string[];
  vowels: string[];
  laws: { generation: number; label: string }[];
  words: number;
  wordBuilding: string;
  builders: { relation: string; shape: string }[];
  /// The smallest word sound change leaves, such as "two syllables".
  minimalWord: string;
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
  intelligibility: { a: number; b: number; score: number }[];
  annals: Annal[];
  /// Histories set aside, with what they told that this one does not.
  tellings: TellingView[];
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
  kind: "found" | "split" | "shift" | "contact" | "parted" | "conquest" | "law";
  /// The annalist's words; words of the language are marked *thus*.
  text: string;
  /// The apparatus: what a linguist would note, such as sound laws.
  notes: string[];
  variety: number | null;
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
  word(generation: number, variety: number, concept: string): WordDetail;
  save(): string;
  dispose(): void;
}

/// Years per generation, for display only.
export const YEARS = 25;

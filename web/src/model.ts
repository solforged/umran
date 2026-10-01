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

export interface Preview {
  words: PreviewWord[];
  families: PreviewWord[];
  homophones: number;
  syllables: number;
}

export type Action =
  | { kind: "found"; name: string; design: LanguageDesign; seed: number; power: number; openness: number }
  | { kind: "connect"; a: number; b: number; intensity: number; contact: ContactKind }
  | { kind: "split"; community: number; name: string; intensity: number }
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
}

export interface Marker {
  generation: number;
  kind: "action" | "run" | "event";
  label: string;
}

export interface Community {
  id: number;
  name: string;
  variety: number;
  size: number;
  prestige: number;
  power: number;
  openness: number;
}

export interface Variety {
  id: number;
  name: string;
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
  latest(): number;
  overview(generation: number): Overview;
  lexicon(generation: number, variety: number): LexiconRow[];
  word(generation: number, variety: number, concept: string): WordDetail;
  save(): string;
  dispose(): void;
}

/// Years per generation, for display only.
export const YEARS = 25;

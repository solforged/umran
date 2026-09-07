export type ContactDomain = "General" | "Maritime";
export type UseDomain = "Home" | "Trade" | "Ritual";
export type Role = "Agent" | "Experiencer" | "Patient";
export type SemanticFrame = { Entity: { countable: boolean } } | { Event: { roles: Role[] } };
export type FoundLanguage = { New: { name: string; aesthetic: string } } | { Existing: { variety: number } };

export type HistoryEvent =
  | { Found: { name: string; group: string; location: string; ancestry: string; language: FoundLanguage } }
  | { UseLanguage: { community: number; variety: number; domain: UseDomain } }
  | { Separate: { parent: number; name: string; community: number } }
  | { Develop: { variety: number; steps: number } }
  | { Contact: { donor: number; recipient: number; domain: ContactDomain; count: number } }
  | { Derive: { variety: number; base: string; sense: number; construction: string } }
  | { ExtendSense: { variety: number; lexeme: string; gloss: string; frame: SemanticFrame } }
  | { ShiftSense: { variety: number; lexeme: string; sense: number; gloss: string; frame: SemanticFrame } }
  | { Replace: { variety: number; lexeme: string; replacement: string } }
  | { Lexicalize: { variety: number; lexeme: string } }
  | { Remodel: { variety: number; lexeme: string; base: string; sense: number; construction: string } };

export interface Trace {
  checkpoint: number;
  variety: number;
  before: string | null;
  after: string;
  explanation: string;
}

export interface LexemeRef {
  variety: number;
  checkpoint: number;
  lexeme: string;
}

export interface Sense {
  id: number;
  gloss: string;
  frame: SemanticFrame;
}

export interface Lexeme {
  id: string;
  classId: string;
  classLabel: string;
  senses: Sense[];
  form: string;
  ipa: string;
  stress: number;
  boundaries: number[];
  analysis: { base: string; sense: number; construction: string; label: string } | null;
  origin: {
    kind: "Unrecorded" | "Formed" | "Borrowed" | "Inherited";
    label: string;
    source: LexemeRef | null;
    construction: string | null;
    sense: number | null;
    checkpoint: number;
  };
  retired: boolean;
  traces: Trace[];
}

export interface Community {
  id: number;
  name: string;
  group: string;
  location: string;
  ancestry: string;
  uses: { variety: number; domain: UseDomain }[];
}

export interface Variety {
  id: number;
  name: string;
  parent: number | null;
  foundedAt: number;
  aesthetic: { id: string; name: string; description: string };
  inventory: { consonants: string[]; vowels: string[] };
  classes: { id: string; label: string; kind: "Entity" | "Event" }[];
  stress: "Initial" | "Penultimate" | "Final";
  constructions: {
    id: string;
    label: string;
    inputClass: string;
    outputClass: string;
    operation: string;
    exponents: string[];
  }[];
  lexicon: Lexeme[];
}

export interface Effect {
  variety: number;
  lexeme: string;
  gloss: string;
  before: string | null;
  after: string;
  explanation: string;
}

export interface Snapshot {
  seed: number;
  checkpoint: number;
  latest: number;
  summary: string;
  event: HistoryEvent | null;
  checkpoints: { id: number; summary: string; event: HistoryEvent | null }[];
  communities: Community[];
  varieties: Variety[];
  effects: Effect[];
  rules: { id: string; label: string; detail: string }[];
}

export interface FormationOption {
  construction: string;
  label: string;
  gloss: string;
  form: string;
  ipa: string;
  base: string;
  sense: number;
  exponent: string;
  existing: string | null;
}

export interface Engine {
  snapshot(checkpoint?: number): Snapshot;
  options(checkpoint: number, variety: number, base: string, sense: number): FormationOption[];
  preview(event: HistoryEvent): Snapshot;
  commit(event: HistoryEvent): Snapshot;
  save(): string;
  load(json: string): Snapshot;
  dispose(): void;
}

export interface NewHistory {
  seed: number;
  aesthetic: string;
  empty: boolean;
}

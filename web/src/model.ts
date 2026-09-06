export type FormationKind = "Agent" | "Place" | "Collective";
export type ContactDomain = "General" | "Maritime";

export type HistoryEvent =
  | { Found: { name: string; aesthetic: string } }
  | { Separate: { parent: number; name: string } }
  | { Develop: { community: number; steps: number } }
  | { Contact: { donor: number; recipient: number; domain: ContactDomain; count: number } }
  | { Derive: { community: number; base: string; kind: FormationKind } };

export interface Trace {
  checkpoint: number;
  community: number;
  before: string | null;
  after: string;
  explanation: string;
}

export interface Lexeme {
  id: string;
  gloss: string;
  form: string;
  ipa: string;
  originCommunity: number;
  originLexeme: string;
  baseLexeme: string | null;
  formation: FormationKind | null;
  traces: Trace[];
}

export interface Community {
  id: number;
  name: string;
  parent: number | null;
  foundedAt: number;
  aesthetic: { id: string; name: string; description: string };
  inventory: { consonants: string[]; vowels: string[] };
  lexicon: Lexeme[];
  formations: { kind: FormationKind; label: string; form: string; ipa: string }[];
}

export interface Effect {
  community: number;
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
  effects: Effect[];
  rules: { id: string; label: string; detail: string }[];
}

export interface FormationOption {
  kind: FormationKind;
  label: string;
  gloss: string;
  form: string;
  ipa: string;
  base: string;
  exponent: string;
  existing: string | null;
}

export interface Engine {
  snapshot(checkpoint?: number): Snapshot;
  options(checkpoint: number, community: number, base: string): FormationOption[];
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

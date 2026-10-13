// Messages between the page (engine.ts) and the engine's thread
// (engine.worker.ts). Engine values cross as the facade's JSON strings; the
// page parses them.
import type { GeographyVersion, MapSize } from "./model";

/// Workbench and ReadView methods that only read.
export const READ_METHODS = {
  latest: true, overview: true, overviewAt: true, settlement: true, contactReach: true, lawChoices: true,
  lexicon: true, kin: true, word: true, story: true, climate: true, river: true,
  lake: true, map: true, wordMap: true, decisions: true, foundingPreview: true,
  foundingSites: true, foundingTraits: true, forming: true, resolveNote: true,
  previous: true, compare: true,
} as const satisfies Record<string, true>;
/// Workbench methods that change the world; each answers with its new status.
export const WRITE_METHODS = {
  act: true, actAt: true, untilAt: true, runUntilEvent: true, branch: true,
  restore: true, rename: true, saveNote: true, removeNote: true, setTitle: true,
  setAuthor: true,
} as const satisfies Record<string, true>;
export type ReadMethod = keyof typeof READ_METHODS;
export type WriteMethod = keyof typeof WRITE_METHODS;

/// What the page keeps synchronously about an open world.
export interface EngineStatus {
  latest: number;
  /// The world's save file; kept current so a reload can always save.
  save: string;
  title: string | null;
  author: string | null;
  /// The notebook as the facade's JSON.
  notebook: string;
}

export type EngineRequest = { id: number } & (
  | { op: "create"; seed: number; size: MapSize }
  | { op: "load"; json: string }
  | { op: "draft"; world: number }
  | { op: "geography"; seed: number; size: MapSize; geography: GeographyVersion }
  | { op: "free"; world: number }
  | { op: "read"; world: number; scope: { telling: number; point: string } | null; method: ReadMethod; args: unknown[] }
  /// One reading per argument list, answered together.
  | { op: "readEach"; world: number; scope: { telling: number; point: string } | null; method: ReadMethod; calls: unknown[][] }
  | { op: "write"; world: number; method: WriteMethod; args: unknown[] }
);

export type EngineReply = { id: number } & ({ ok: true; value: unknown } | { ok: false; error: string });

/// A world as `create`, `load`, and `draft` open it.
export interface Opened { world: number; status: EngineStatus }
/// A change's own result and the world after it.
export interface Written { value: unknown; status: EngineStatus }

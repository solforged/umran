import init, { Workbench, ReadView } from "./wasm/umran_web";
import type {
  Naming,
  Action,
  Catalog,
  Engine,
  ReadEngine,
  Comparison,
  ReadingRef,
  NotebookNote,
  Destination,
  Kin,
  LanguageDesign,
  LexiconRow,
  MapSize,
  Overview,
  Preview,
  WordDetail,
  WordMap,
  WorldMap,
  SettlementPreview,
} from "./model";

// One WASM module; the Workbench owns the history and React only holds view
// selections. Errors from the engine arrive as strings.
let initialized: Promise<unknown> | undefined;

async function ready(): Promise<void> {
  initialized ??= init().catch((error: unknown) => {
    initialized = undefined;
    throw error;
  });
  await initialized;
}

function reader(bench: Workbench | ReadView, save: () => string) {
  let cached: { generation: number; view: Overview } | undefined;
  const clear = () => { cached = undefined; };
  const engine: ReadEngine = {
    overviewAt: (point) => JSON.parse(bench.overviewAt(JSON.stringify(point))) as Overview,
    settlement: (point, community, intent, share, destination) =>
      JSON.parse(bench.settlement(JSON.stringify(point), community, intent, share, destination ?? -1)) as SettlementPreview,
    latest: () => bench.latest(),
    overview: (generation) => {
      if (cached?.generation !== generation) cached = { generation, view: JSON.parse(bench.overview(generation)) as Overview };
      return cached.view;
    },
    lexicon: (generation, variety) => JSON.parse(bench.lexicon(generation, variety)) as LexiconRow[],
    kin: (generation, variety) => JSON.parse(bench.kin(generation, variety)) as Kin[],
    word: (generation, variety, concept) => JSON.parse(bench.word(generation, variety, concept)) as WordDetail,
    map: () => JSON.parse(bench.map()) as WorldMap,
    wordMap: (generation, concept) => JSON.parse(bench.wordMap(generation, concept)) as WordMap,
    save,
  };
  return { engine, clear, dispose: () => { clear(); bench.free(); } };
}

function wrap(bench: Workbench): Engine {
  const root = reader(bench, () => bench.save());
  // The document owns WASM views. Components hold lightweight reading
  // capabilities, so rendering twice or leaving a card never frees a peer's
  // world. Three cached scopes cover the active reading and a comparison.
  const scopes = new Map<string, ReturnType<typeof reader>>();
  let comparison: { key: string; value: Comparison } | undefined;
  let disposed = false;
  const clear = () => {
    root.clear(); comparison = undefined;
    scopes.forEach((scope) => scope.dispose()); scopes.clear();
  };
  const scope = (telling: number, point: string) => {
    if (disposed) throw new Error("This world has been closed.");
    const key = `${telling}:${point}`;
    let entry = scopes.get(key);
    if (!entry) {
      entry = reader(bench.read(telling, point), () => bench.save());
      if (scopes.size >= 3) {
        const oldest = scopes.keys().next().value!;
        scopes.get(oldest)!.dispose(); scopes.delete(oldest);
      }
    }
    scopes.delete(key); scopes.set(key, entry);
    return entry.engine;
  };
  return {
    ...root.engine,
    notebook: () => JSON.parse(bench.notebook()) as NotebookNote[],
    saveNote: (note) => { bench.saveNote(JSON.stringify(note)); },
    resolveNote: (id) => JSON.parse(bench.resolveNote(id)) as Destination | null,
    read: (telling, point) => {
      const encoded = point ? JSON.stringify(point) : "";
      const get = () => scope(telling, encoded);
      return {
        latest: () => get().latest(), overview: (g) => get().overview(g), overviewAt: (p) => get().overviewAt(p),
        settlement: (p, c, intent, share, destination) => get().settlement(p, c, intent, share, destination),
        lexicon: (g, v) => get().lexicon(g, v), kin: (g, v) => get().kin(g, v), word: (g, v, c) => get().word(g, v, c),
        wordMap: (g, c) => get().wordMap(g, c), map: () => get().map(), save: () => bench.save(),
      };
    },
    previous: (reading) => JSON.parse(bench.previous(JSON.stringify(reading))) as ReadingRef,
    compare: (left, right, generation) => {
      const key = `${left}:${right}:${generation}`;
      if (comparison?.key !== key) comparison = { key, value: JSON.parse(bench.compare(left, right, generation)) as Comparison };
      return comparison.value;
    },
    rename: (telling, name) => { bench.rename(telling, name); clear(); },
    actAt: (reading, mutation, action) => { bench.actAt(JSON.stringify(reading), mutation, JSON.stringify(action)); clear(); },
    untilAt: (reading, mutation, limit) => { const ran = bench.untilAt(JSON.stringify(reading), mutation, limit); clear(); return ran; },
    act: (action: Action) => { bench.act(JSON.stringify(action)); clear(); },
    runUntilEvent: (limit) => { const ran = bench.runUntilEvent(limit); clear(); return ran; },
    branch: (generation) => { bench.branch(generation); clear(); },
    restore: (telling) => { bench.restore(telling); clear(); },
    dispose: () => { if (!disposed) { clear(); root.dispose(); disposed = true; } },
  };
}

export async function createEngine(seed: number, size: MapSize): Promise<Engine> {
  if (!Number.isInteger(seed) || seed < 0 || seed > 0xffff_ffff) {
    throw new Error("Choose a whole-number seed between 0 and 4294967295.");
  }
  await ready();
  return wrap(new Workbench(seed, size));
}

export async function loadEngine(json: string): Promise<Engine> {
  await ready();
  return wrap(Workbench.load(json));
}

// A map depends on its seed and size alone, so the shelf can draw a saved
// world without replaying its history. Kept once drawn.
const maps = new Map<string, Promise<WorldMap>>();

export function landMap(seed: number, size: MapSize): Promise<WorldMap> {
  const key = `${seed}:${size}`;
  let map = maps.get(key);
  if (!map) {
    map = ready().then(() => {
      const bench = new Workbench(seed, size);
      try {
        return JSON.parse(bench.map()) as WorldMap;
      } finally {
        bench.free();
      }
    });
    maps.set(key, map);
    map.catch(() => maps.delete(key));
  }
  return map;
}

export async function loadCatalog(): Promise<Catalog> {
  await ready();
  return JSON.parse(Workbench.catalog()) as Catalog;
}

// Design helpers; call only after `loadCatalog` has resolved, which
// initializes the module.
export function presetDesign(preset: string, seed: number): LanguageDesign {
  return JSON.parse(Workbench.design(preset, seed)) as LanguageDesign;
}

/// Sounds drawn by how common they are worldwide, in the numbers given.
export function frequencyDesign(seed: number, consonants: number, vowels: number): LanguageDesign {
  return JSON.parse(Workbench.frequencyDesign(seed, consonants, vowels)) as LanguageDesign;
}

/// Sample words for a design, or the reason it cannot found a language.
export function preview(design: LanguageDesign, seed: number, naming: Naming): Preview | string {
  try {
    return JSON.parse(Workbench.preview(JSON.stringify(design), seed, JSON.stringify(naming))) as Preview;
  } catch (error) {
    return message(error);
  }
}

export function message(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}

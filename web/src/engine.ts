import init, { Workbench } from "./wasm/umran_web";
import type {
  Naming,
  Action,
  Catalog,
  Engine,
  Kin,
  LanguageDesign,
  LexiconRow,
  MapSize,
  Overview,
  Preview,
  WordDetail,
  WordMap,
  WorldMap,
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

function wrap(bench: Workbench): Engine {
  return {
    act: (action: Action) => bench.act(JSON.stringify(action)),
    undo: () => bench.undo(),
    runUntilEvent: (limit) => bench.runUntilEvent(limit),
    branch: (generation) => bench.branch(generation),
    restore: (index) => bench.restore(index),
    latest: () => bench.latest(),
    overview: (generation) => JSON.parse(bench.overview(generation)) as Overview,
    lexicon: (generation, variety) => JSON.parse(bench.lexicon(generation, variety)) as LexiconRow[],
    kin: (generation, variety) => JSON.parse(bench.kin(generation, variety)) as Kin[],
    word: (generation, variety, concept) =>
      JSON.parse(bench.word(generation, variety, concept)) as WordDetail,
    map: () => JSON.parse(bench.map()) as WorldMap,
    wordMap: (generation, concept) => JSON.parse(bench.wordMap(generation, concept)) as WordMap,
    save: () => bench.save(),
    dispose: () => bench.free(),
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

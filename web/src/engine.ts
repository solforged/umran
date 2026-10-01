import init, { Workbench } from "./wasm/langgen_web";
import type {
  Naming,
  Action,
  Catalog,
  Engine,
  LanguageDesign,
  LexiconRow,
  Overview,
  Preview,
  WordDetail,
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
    word: (generation, variety, concept) =>
      JSON.parse(bench.word(generation, variety, concept)) as WordDetail,
    save: () => bench.save(),
    dispose: () => bench.free(),
  };
}

export async function createEngine(seed: number): Promise<Engine> {
  if (!Number.isInteger(seed) || seed < 0 || seed > 0xffff_ffff) {
    throw new Error("Choose a whole-number seed between 0 and 4294967295.");
  }
  await ready();
  return wrap(new Workbench(seed));
}

export async function loadEngine(json: string): Promise<Engine> {
  await ready();
  return wrap(Workbench.load(json));
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

export function typicalDesign(seed: number, consonants: number, vowels: number): LanguageDesign {
  return JSON.parse(Workbench.typicalDesign(seed, consonants, vowels)) as LanguageDesign;
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

import init, { Workbench } from "./wasm/langgen_web";
import type { Action, Catalog, Engine, LexiconRow, Overview, WordDetail } from "./model";

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

export function message(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}

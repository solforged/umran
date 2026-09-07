import init, { Workbench } from "./wasm/langgen_web";
import type { Engine, FormationOption, NewHistory, Snapshot } from "./model";

// One WASM module, one owner of accepted state; React only holds view selections.
let initialized: Promise<unknown> | undefined;

export async function createEngine(
  config: NewHistory = { seed: 42, aesthetic: "elvish", empty: false },
  saved?: string,
): Promise<Engine> {
  if (!Number.isInteger(config.seed) || config.seed < 0 || config.seed > 0xffff_ffff) {
    throw new Error("Choose a whole-number seed between 0 and 4294967295.");
  }
  initialized ??= init().catch((error: unknown) => {
    initialized = undefined;
    throw error;
  });
  await initialized;
  const workbench = new Workbench(config.seed, config.aesthetic, saved ? true : config.empty);
  try {
    if (saved !== undefined) workbench.load(saved);
  } catch (error) {
    workbench.free();
    throw error;
  }
  return {
    snapshot: (checkpoint = workbench.latest()) =>
      JSON.parse(workbench.snapshot(checkpoint)) as Snapshot,
    options: (checkpoint, variety, base, sense) =>
      JSON.parse(workbench.options(checkpoint, variety, base, sense)) as FormationOption[],
    preview: (event) => JSON.parse(workbench.preview(JSON.stringify(event))) as Snapshot,
    commit: (event) => JSON.parse(workbench.commit(JSON.stringify(event))) as Snapshot,
    save: () => workbench.save(),
    load: (json) => {
      workbench.load(json);
      return JSON.parse(workbench.snapshot(workbench.latest())) as Snapshot;
    },
    dispose: () => workbench.free(),
  };
}

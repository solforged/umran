// The sample world: a fixed recipe, so every visitor starts from the same
// four thousand years. Three peoples of three families settle nearby lands
// on one continent and spread. The sea people make a realm and write first.
// After 2,500 years the river people conquer them by land. A teacher among
// the conquered founds a faith just before they take their rulers' tongue;
// their old speech lives on as the faith's sacred language.
// Other happenings are the engine's to tell and can change with its revision.
// Opening it puts a copy on the shelf to continue.

import { createEngine, presetDesign } from "./engine";
import type { Action, Engine, MapSize, Naming } from "./model";

/// The land the sample plays out on, for drawing it before it is opened.
export const SAMPLE_LAND: { seed: number; size: MapSize } = { seed: 21, size: "medium" };

export async function sampleWorld(): Promise<Engine> {
  const engine = await createEngine(SAMPLE_LAND.seed, SAMPLE_LAND.size);
  const found = (
    preset: string, seed: number, naming: Naming, power: number, region: number,
  ): Action => ({
    kind: "found",
    naming,
    design: presetDesign(preset, seed),
    seed,
    power,
    openness: 0.5,
    region,
  });
  // These plains are a few land borders apart on the same continent.
  const actions: Action[] = [
    found("germanic", 31, { kind: "place", place: "river" }, 0.7, 448),
    found("semitic", 52, { kind: "people" }, 0.5, 1722),
    found("polynesian", 73, { kind: "place", place: "sea" }, 0.4, 33),
    { kind: "state", community: 2 },
    { kind: "craft", community: 2, craft: "writing" },
    { kind: "run", generations: 100 },
    { kind: "connect", a: 0, b: 2, intensity: 0.8, contact: "rule" },
    { kind: "religion", community: 2 },
    { kind: "shift", community: 2, toward: 0 },
    { kind: "run", generations: 60 },
  ];
  try {
    for (const action of actions) engine.act(action);
  } catch (error) {
    engine.dispose();
    throw error;
  }
  return engine;
}

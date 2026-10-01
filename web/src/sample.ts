// The sample chronicle: a fixed recipe, so every reader starts from the
// same few centuries. Opening it puts a copy on the shelf to continue.

import { createEngine, presetDesign } from "./engine";
import type { Action, Engine } from "./model";

const WORLD = 1407;

export async function sampleBook(): Promise<Engine> {
  const engine = await createEngine(WORLD);
  const actions: Action[] = [
    {
      kind: "found",
      naming: { kind: "place", place: "river" },
      design: presetDesign("germanic", 31),
      seed: 31,
      power: 0.6,
      openness: 0.4,
    },
    { kind: "run", generations: 8 },
    { kind: "split", community: 0, intensity: 0.3 },
    { kind: "run", generations: 10 },
    {
      kind: "found",
      naming: { kind: "people" },
      design: presetDesign("finnic", 52),
      seed: 52,
      power: 0.4,
      openness: 0.6,
    },
    { kind: "connect", a: 1, b: 2, intensity: 0.6, contact: "trade" },
    { kind: "run", generations: 12 },
  ];
  try {
    for (const action of actions) engine.act(action);
  } catch (error) {
    engine.dispose();
    throw error;
  }
  return engine;
}

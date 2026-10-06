// Run with `bun run parity` (or `bun run parity --seed 42 --years 4000`).
// Add `--every-year` to inspect every generation, including transient drift.
// Two SHA-256 digests of key-sorted JSON distinguish discrete history from exact
// float state. Together they cover the complete overview except present-history
// metadata, map, every dominant lexicon, and all competing living forms with
// shares/senses/history. Names, laws, contacts, polities, faiths, crafts, places,
// membership, and the ordered annal stream remain in the discrete tier.
// Arrays retain order. Float mismatches still fail: no rounding or tolerance.
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, rm } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import type { ChildProcess } from "node:child_process";
import type { Workbench as WasmWorkbench } from "../web/src/wasm/umran_web";
import { YEARS } from "../web/src/model";

type Json = null | boolean | number | string | Json[] | JsonObject;
interface JsonObject { [key: string]: Json }

function json(value: unknown): Json {
  if (value === null || typeof value === "boolean" || typeof value === "string") return value;
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (Array.isArray(value)) return value.map(json);
  if (value && typeof value === "object") {
    const result: JsonObject = {};
    for (const [key, child] of Object.entries(value)) result[key] = json(child);
    return result;
  }
  throw new Error("Expected finite JSON data from the facade");
}
function object(value: Json | undefined): JsonObject {
  if (value && typeof value === "object" && !Array.isArray(value)) return value;
  throw new Error("Expected a JSON object from the facade");
}
function array(value: Json | undefined): Json[] {
  if (Array.isArray(value)) return value;
  throw new Error("Expected a JSON array from the facade");
}
function number(value: Json | undefined): number {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  throw new Error("Expected a number from the facade");
}
function string(value: Json | undefined): string {
  if (typeof value === "string") return value;
  throw new Error("Expected a string from the facade");
}

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const started = performance.now();
const seeds = [0, 1, 2, 3, 4, 5, 7, 11, 42, 73, 2026, 0xffff_ffff];
const args = process.argv.slice(2);
let onlySeed: number | undefined;
let years = [0, 250, 1000, 4000];
let everyYear = false;
for (let i = 0; i < args.length; i++) {
  const option = args[i];
  if (option === "--every-year") { everyYear = true; continue; }
  const value = Number(args[++i]);
  if (!Number.isSafeInteger(value) || value < 0) throw new Error(`Invalid ${option}: ${args[i]}`);
  if (option === "--seed" && value <= 0xffff_ffff) onlySeed = value;
  else if (option === "--years" && value % YEARS === 0 && value <= 2000 * YEARS) years = [value];
  else throw new Error("Usage: bun run parity [--seed <u32>] [--years <multiple of 25, at most 50000>] [--every-year]");
}

// Persistent, root-specific caches keep warm runs quick without sharing the
// web build's Cargo lock or changing any generated browser bindings.
const cacheKey = createHash("sha256").update(root).digest("hex").slice(0, 12);
const cache = join("/tmp", `umran-parity-${cacheKey}`);
const scratch = await mkdtemp(join(tmpdir(), "umran-parity-bindings-"));
await mkdir(cache, { recursive: true });

async function command(command: string[], target: string): Promise<void> {
  const child = Bun.spawn(command, {
    cwd: root,
    env: { ...process.env, CARGO_TARGET_DIR: target },
    stdout: "inherit",
    stderr: "inherit",
  });
  const code = await child.exited;
  if (code !== 0) throw new Error(`${command.join(" ")} exited ${code}`);
}

function canonical(value: Json): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical(value[key])}`).join(",")}}`;
}
function digest(value: Json): string {
  return createHash("sha256").update(canonical(value)).digest("hex");
}
// These are the facade's floating fields, including integral-valued floats. Geometry
// arrays inherit their field's classification. A new fractional numeric field
// is also float state; a new integral field remains discrete until classified.
const FLOAT_FIELDS: Record<string, true | undefined> = {
  size: true, prestige: true, power: true, openness: true, intensity: true,
  purism: true, city: true, keptFromHigh: true, share: true,
  width: true, height: true, site: true,
  radiusKm: true, center: true, boundary: true, lengthKm: true,
  elevation: true, moisture: true, warmth: true, wetness: true,
  targetWetness: true, targetWarmth: true, riverFlow: true, flow: true,
  feeding: true, areaKm2: true, kmPerUnit: true,
};
function tiers(state: Json): { discrete: Json; floats: JsonObject } {
  const floats: JsonObject = {};
  function visit(value: Json, path: string, floatField: boolean): Json {
    if (typeof value === "number" && (floatField || !Number.isInteger(value))) {
      floats[path] = value;
      return "<float>";
    }
    if (value === null || typeof value !== "object") return value;
    if (Array.isArray(value)) return value.map((child, i) => visit(child, `${path}.${i}`, floatField));
    const discrete: JsonObject = {};
    for (const [key, child] of Object.entries(value)) {
      discrete[key] = visit(child, `${path}.${key}`, floatField || FLOAT_FIELDS[key] === true);
    }
    return discrete;
  }
  return { discrete: visit(state, "$", false), floats };
}

function firstDifference(a: Json | undefined, b: Json | undefined, path = "$"): string {
  if (Object.is(a, b)) return "";
  if (a === null || b === null || typeof a !== "object" || typeof b !== "object") {
    return `${path}: native=${JSON.stringify(a)} wasm=${JSON.stringify(b)}`;
  }
  if (Array.isArray(a) !== Array.isArray(b)) return `${path}: different JSON types`;
  const left = Array.isArray(a) ? Object.fromEntries(a.map((value, i) => [i, value])) : a;
  const right = Array.isArray(b) ? Object.fromEntries(b.map((value, i) => [i, value])) : b;
  const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
  for (const key of keys) {
    const difference = firstDifference(left[key], right[key], key.startsWith("$") ? key : `${path}.${key}`);
    if (difference) return difference;
  }
  return "";
}

function snapshot(bench: WasmWorkbench, generation: number): JsonObject {
  const overview = object(json(JSON.parse(bench.overview(generation))));
  delete overview.latest;
  delete overview.timeline;
  delete overview.mutation;
  delete overview.point;
  delete overview.atTip;
  delete overview.tellings;
  const competitors: JsonObject[] = [];
  const lexicons = array(overview.varieties).map(value => {
    const variety = object(value);
    const id = number(variety.id);
    const rows = array(json(JSON.parse(bench.lexicon(generation, id))));
    for (const value of rows) {
      const row = object(value);
      if (number(row.competitors) > 0) {
        const concept = string(row.concept);
        const detail = object(json(JSON.parse(bench.word(generation, id, concept))));
        competitors.push({ variety: id, concept, variants: array(detail.variants) });
      }
    }
    return { variety: id, rows };
  });
  const map = object(json(JSON.parse(bench.map())));
  const climate = json(JSON.parse(bench.climate(generation)));
  const rivers = array(map.rivers).map(value =>
    json(JSON.parse(bench.river(generation, number(object(value).id)))));
  return { overview, map, climate, rivers, lexicons, competitors };
}
function summary(state: Json): string {
  const o = object(object(state).overview);
  const communities = array(o.communities).map(object);
  const varieties = array(o.varieties).map(object);
  const living = communities.filter(c => c.ended === null).length;
  const spoken = varieties.filter(v => v.spoken === true).length;
  const words = varieties.reduce((n, v) => n + number(v.words), 0);
  const laws = varieties.reduce((n, v) => n + array(v.laws).length, 0);
  return `peoples=${living}/${communities.length} languages=${spoken}/${varieties.length} words=${words} laws=${laws} contacts=${array(o.contacts).length} polities=${array(o.states).length} faiths=${array(o.religions).length} annals=${array(o.annals).length}`;
}

let native: ChildProcess | undefined;
let rpc: (input: JsonObject) => Promise<Json>;
let Workbench: typeof WasmWorkbench;
let checks = 0;
let worlds = 0;
let mismatches = 0;
interface Drift { discrete: number | null; floats: number | null; through: number }
const drift = new Map<string, Drift>();

async function compare(bench: WasmWorkbench, label: string, seed: number, year: number): Promise<void> {
  const generation = year / YEARS;
  const nativeState = await rpc({ kind: "snapshot", generation });
  const nativeTiers = tiers(nativeState);
  const wasmTiers = tiers(snapshot(bench, generation));
  const key = `${label.replace(/\/replay$/, "")} seed=${seed}`;
  let record = drift.get(key);
  if (!record) {
    record = { discrete: null, floats: null, through: year };
    drift.set(key, record);
  }
  record.through = Math.max(record.through, year);
  const equal = {
    discrete: digest(nativeTiers.discrete) === digest(wasmTiers.discrete),
    floats: digest(nativeTiers.floats) === digest(wasmTiers.floats),
  };
  checks++;
  if (!equal.discrete || !equal.floats) mismatches++;
  if (!everyYear || years.includes(year) || label.endsWith("/replay")) {
    console.log(`${equal.discrete && equal.floats ? "PASS" : "DRIFT"} ${label} seed=${seed} year=${year} discrete=${equal.discrete ? "MATCH" : "DIFFER"}:${digest(nativeTiers.discrete)} floats=${equal.floats ? "MATCH" : "DIFFER"}:${digest(nativeTiers.floats)} ${summary(nativeState)}`);
  }
  for (const tier of ["discrete", "floats"] as const) {
    if (equal[tier] || record[tier] !== null) continue;
    if (everyYear) {
      record[tier] = year;
      console.error(`FIRST ${tier} ${key} year=${year}: ${firstDifference(nativeTiers[tier], wasmTiers[tier])}`);
      console.error(`  Repro: bun run parity --seed ${seed} --years ${year}`);
      continue;
    }
    // Bisect with checkpoint-backed past views, then inspect the earlier
    // generations chronologically. The scan makes "first" exact even if a
    // transient float difference reconverges, rather than assuming monotonicity.
    let low = -1;
    let high = generation;
    while (high - low > 1) {
      const middle = Math.floor((low + high) / 2);
      const n = tiers(await rpc({ kind: "snapshot", generation: middle }));
      const w = tiers(snapshot(bench, middle));
      if (digest(n[tier]) === digest(w[tier])) low = middle;
      else high = middle;
    }
    for (let first = 0; first <= high; first++) {
      const n = tiers(await rpc({ kind: "snapshot", generation: first }));
      const w = tiers(snapshot(bench, first));
      if (digest(n[tier]) !== digest(w[tier])) {
        record[tier] = first * YEARS;
        console.error(`FIRST ${tier} ${key} year=${record[tier]}: ${firstDifference(n[tier], w[tier])}`);
        console.error(`  Repro: bun run parity --seed ${seed} --years ${year}`);
        break;
      }
    }
  }
}

function assertExact(nativeValue: Json, wasmValue: Json, label: string): void {
  if (digest(nativeValue) !== digest(wasmValue)) {
    throw new Error(`${label}: ${firstDifference(nativeValue, wasmValue)}`);
  }
  checks++;
}

async function refused(
  bench: WasmWorkbench, request: JsonObject, apply: () => unknown, label: string,
): Promise<void> {
  const nativeSave = await rpc({ kind: "save" });
  const wasmSave = bench.save();
  let nativeError = false;
  let wasmError = false;
  try { await rpc(request); } catch { nativeError = true; }
  try { apply(); } catch { wasmError = true; }
  if (!nativeError || !wasmError) throw new Error(`${label} must be refused by both facades`);
  assertExact(await rpc({ kind: "save" }), nativeSave, `${label} changed native history`);
  assertExact(bench.save(), wasmSave, `${label} changed WASM history`);
}

const presets = ["germanic", "semitic", "polynesian"];
function actions(seed: number, region: number): JsonObject[] {
  const found: JsonObject[] = presets.map((preset, index) => ({
    kind: "found", preset, seed: (seed + 31 + 21 * index) >>> 0,
    naming: index === 0 ? { kind: "place", place: "river" } : index === 1 ? { kind: "people" } : { kind: "place", place: "sea" },
    power: [0.7, 0.5, 0.4][index], openness: 0.5,
    region,
  }));
  return [
    ...found,
    { kind: "state", community: 0 },
    { kind: "religion", community: 1 },
    { kind: "craft", community: 0, craft: "writing" },
    { kind: "connect", a: 0, b: 1, intensity: 0.6, contact: "trade" },
    { kind: "connect", a: 1, b: 2, intensity: 0.4, contact: "intermarriage" },
  ];
}
function wasmAct(bench: WasmWorkbench, descriptor: JsonObject): void {
  const action = { ...descriptor };
  if (action.preset) {
    action.design = json(JSON.parse(Workbench.design(string(action.preset), number(action.seed))));
    delete action.preset;
  }
  bench.act(JSON.stringify(action));
}

async function generated(seed: number): Promise<void> {
  const map = ["small", "medium", "large", "vast"][seed % 4];
  const label = `generated/${map}`;
  await rpc({ kind: "new", seed, map });
  const bench = new Workbench(seed, map);
  try {
    const regions = array(object(json(JSON.parse(bench.map()))).regions).map(object);
    const home = regions.find(region => region.terrain === "plains");
    if (!home) throw new Error("The authored-contact fixture needs a farming plain");
    for (const action of actions(seed, number(home.id))) {
      await rpc({ kind: "act", action });
      wasmAct(bench, action);
    }
    let generation = 0;
    const checkpoints = everyYear
      ? Array.from({ length: Math.max(...years) / YEARS + 1 }, (_, i) => i * YEARS)
      : years;
    for (const year of checkpoints) {
      const more = year / YEARS - generation;
      if (more > 0) {
        const action = { kind: "run", generations: more };
        await rpc({ kind: "act", action });
        wasmAct(bench, action);
        generation += more;
      }
      await compare(bench, label, seed, year);
    }
    // Recipes, not resolved worlds, are what users save. Exercise both
    // architectures' real load/replay path as well as their live action path.
    const nativeRecipe = string(await rpc({ kind: "save" }));
    const wasmRecipe = bench.save();
    if (digest(json(JSON.parse(nativeRecipe))) !== digest(json(JSON.parse(wasmRecipe)))) {
      throw new Error(`Saved recipes differ: seed=${seed}`);
    }
    const live = digest(snapshot(bench, generation));
    await rpc({ kind: "load", recipe: nativeRecipe });
    const replay = Workbench.load(wasmRecipe);
    try {
      await compare(replay, `${label}/replay`, seed, generation * YEARS);
      if (digest(snapshot(replay, generation)) !== live) throw new Error(`Live/replayed WASM world differs: seed=${seed}`);
    } finally { replay.free(); }
    worlds++;
  } finally { bench.free(); }
}

async function relatedFamily(seed: number): Promise<void> {
  const size = ["small", "medium", "large", "vast"][seed % 4];
  const label = `related/${size}`;
  await rpc({ kind: "new", seed, map: size });
  const bench = new Workbench(seed, size);
  try {
    const map = object(json(JSON.parse(bench.map())));
    if (map.geography !== "continental-v6") throw new Error("New founding worlds must use continental-v6");
    const nativeBefore = await rpc({ kind: "save" });
    const wasmBefore = bench.save();
    let sites: number[] | undefined;
    for (const region of array(map.regions).map(object).filter(r => r.terrain !== "sea")) {
      const candidates = array(json(JSON.parse(bench.foundingSites(number(region.id), 3)))).map(number);
      if (candidates.length === 3) { sites = candidates; break; }
    }
    if (!sites) throw new Error(`Related-family fixture needs three nearby land sites: seed=${seed}`);
    const anchor = sites[0];
    assertExact(
      await rpc({ kind: "founding-sites", region: anchor, count: 3 }), sites,
      `Founding sites differ: seed=${seed}`,
    );
    assertExact(await rpc({ kind: "save" }), nativeBefore, "Founding-site query changed native recipe");
    assertExact(bench.save(), wasmBefore, "Founding-site query changed WASM recipe");
    const sea = array(map.regions).map(object).find(r => r.terrain === "sea");
    if (!sea) throw new Error("Founding-site refusal fixture needs sea");
    for (const [region, count] of [[number(sea.id), 3], [anchor, 0], [anchor, 13], [array(map.regions).length, 3]]) {
      await refused(bench, { kind: "founding-sites", region, count },
        () => bench.foundingSites(region, count), "Invalid founding-site query");
    }
    // wasm-bindgen must not silently truncate invalid JavaScript numbers.
    for (const [region, count] of [[anchor + 0.5, 3], [-1, 3], [anchor, 1.5]]) {
      let rejected = false;
      try { bench.foundingSites(region, count); } catch { rejected = true; }
      if (!rejected) throw new Error("Fractional/negative founding-site input was silently coerced");
    }
    assertExact(bench.save(), wasmBefore, "Invalid WASM query changed history");
    const founder: JsonObject = {
      kind: "found", preset: "germanic", seed: (seed + 31) >>> 0,
      naming: { kind: "people" }, region: anchor, livelihood: "farming",
      power: 0.5, openness: 0.5,
    };
    await rpc({ kind: "act", action: founder });
    wasmAct(bench, founder);
    const occupiedSites = json(JSON.parse(bench.foundingSites(anchor, 3)));
    assertExact(await rpc({ kind: "founding-sites", region: anchor, count: 3 }), occupiedSites,
      `Occupied-anchor sites differ: seed=${seed}`);
    if (array(occupiedSites).includes(anchor) || number(array(occupiedSites)[0]) !== sites[1]) {
      throw new Error("An occupied anchor must be skipped while preserving nearby order");
    }
    for (const [source, region] of [[99, sites[1]], [0, number(sea.id)]]) {
      const invalid: JsonObject = { kind: "found-related", source, region, naming: { kind: "people" } };
      await refused(bench, { kind: "act", action: invalid }, () => wasmAct(bench, invalid),
        "Invalid related-founding reference");
    }
    for (const region of sites.slice(1)) {
      const action: JsonObject = {
        kind: "found-related", source: 0, region, naming: { kind: "people" },
        livelihood: "farming", ethos: { open: 0.4 },
      };
      await rpc({ kind: "act", action });
      wasmAct(bench, action);
    }
    const overview = object(json(JSON.parse(bench.overview(0))));
    const communities = array(overview.communities).map(object);
    const varieties = array(overview.varieties).map(object);
    const source = varieties[number(communities[0].variety)];
    const sourceRows = json(JSON.parse(bench.lexicon(0, number(source.id))));
    const descendants = communities.slice(1).map(c => varieties[number(c.variety)]);
    for (const [index, child] of descendants.entries()) {
      assertExact(communities[index + 1].parents!, [0], "Related people's ancestry is missing");
      if (communities[index + 1].size !== communities[0].size || child.parent !== source.id
        || child.forkedAt !== 0 || child.family !== source.family) {
        throw new Error("Related founding must be year-zero ancestry, not unrelated speech or migration");
      }
      assertExact(child.grammar!, source.grammar!, "Related founding lost its inherited grammar");
      assertExact(json(JSON.parse(bench.lexicon(0, number(child.id)))), sourceRows,
        "Related founding regenerated words or lost lexical origins");
    }
    const decisions = json(JSON.parse(bench.decisions()));
    assertExact(await rpc({ kind: "decisions" }), decisions, "Related authored decisions differ");
    for (const decision of array(decisions).map(object).slice(1)) {
      if (decision.kind !== "found-related" || array(decision.people)[0] !== 0 || decision.variety === null) {
        throw new Error("Related authored decisions must retain source, child and resulting language");
      }
    }
    await compare(bench, label, seed, 0);
    // A real law on just the founder demonstrates that forks evolve separately.
    const point = overview.point!;
    const choices = json(JSON.parse(bench.lawChoices(JSON.stringify(point), number(source.id))));
    assertExact(await rpc({ kind: "law-choices", point, variety: source.id! }), choices,
      "Related family's independent law choices differ");
    const law = array(choices).map(object).find(choice => number(choice.words) > 0);
    if (!law) throw new Error("Related-family fixture needs an effective sound law");
    const childRows = descendants.map(child => json(JSON.parse(bench.lexicon(0, number(child.id)))));
    const action: JsonObject = { kind: "law", variety: source.id!, law: law.id! };
    await rpc({ kind: "act", action });
    wasmAct(bench, action);
    if (digest(json(JSON.parse(bench.lexicon(0, number(source.id))))) === digest(sourceRows)) {
      throw new Error("Authored law did not change the founder's living words");
    }
    for (const [index, child] of descendants.entries()) {
      assertExact(json(JSON.parse(bench.lexicon(0, number(child.id)))), childRows[index],
        "A law on the parent mutated its independent founding fork");
    }
    assertExact(await rpc({ kind: "decisions" }), json(JSON.parse(bench.decisions())),
      "Related family's exhaustive law decision export differs");
    const run: JsonObject = { kind: "run", generations: 4 };
    await rpc({ kind: "act", action: run });
    wasmAct(bench, run);
    await compare(bench, label, seed, 4 * YEARS);
    const nonzero: JsonObject = {
      kind: "found-related", source: 0, region: sites[1], naming: { kind: "people" },
    };
    await refused(bench, { kind: "act", action: nonzero }, () => wasmAct(bench, nonzero),
      "Nonzero-generation related founding");
    const nativeRecipe = string(await rpc({ kind: "save" }));
    const wasmRecipe = bench.save();
    assertExact(json(JSON.parse(nativeRecipe)), json(JSON.parse(wasmRecipe)), "Related-family recipes differ");
    const live = snapshot(bench, bench.latest());
    await rpc({ kind: "load", recipe: nativeRecipe });
    const restored = Workbench.load(wasmRecipe);
    try {
      await compare(restored, `${label}/replay`, seed, restored.latest() * YEARS);
      assertExact(snapshot(restored, restored.latest()), live, "Related family changed after export/reload");
      assertExact(await rpc({ kind: "decisions" }), json(JSON.parse(restored.decisions())),
        "Related decisions changed on replay");
    } finally { restored.free(); }
    worlds++;
  } finally { bench.free(); }
}

async function legacyGeography(seed: number, geography: "spherical-v1" | "continental-v2" | "continental-v3" | "continental-v4" | "continental-v5"): Promise<void> {
  const size = ["small", "medium", "large", "vast"][seed % 4];
  const label = `legacy/${geography}/${size}`;
  await rpc({ kind: "new", seed, map: size, geography });
  const original = Workbench.withGeography(seed, size, geography);
  try {
    const map = json(JSON.parse(original.map()));
    const hydrology = array(object(map).lakes).length !== 0
      || array(object(map).rivers).some(r => array(object(r).channel).length !== 0);
    if (geography !== "continental-v4" && geography !== "continental-v5" && hydrology) {
      throw new Error("Legacy geography acquired lakes or river channels");
    }
    const anchor = number(object(array(object(map).landmasses)[0]).anchor);
    const founder: JsonObject = {
      kind: "found", preset: "germanic", seed: (seed + 31) >>> 0,
      naming: { kind: "people" }, region: anchor, power: 0.5, openness: 0.5,
    };
    await rpc({ kind: "act", action: founder });
    wasmAct(original, founder);
    const yearZero = snapshot(original, 0);
    const run: JsonObject = { kind: "run", generations: 2 };
    await rpc({ kind: "act", action: run });
    wasmAct(original, run);
    const nativeRecipe = object(json(JSON.parse(string(await rpc({ kind: "save" })))));
    const wasmRecipe = object(json(JSON.parse(original.save())));
    assertExact(nativeRecipe, wasmRecipe, `Explicit ${geography} recipes differ`);
    if (wasmRecipe.geography !== geography) throw new Error("Old geography must remain explicit on export");
    if (geography === "spherical-v1") {
      // Recreate the actual revision-33 spelling: old maps had no version field.
      nativeRecipe.revision = 33;
      wasmRecipe.revision = 33;
      delete nativeRecipe.geography;
      delete wasmRecipe.geography;
    }
    await rpc({ kind: "load", recipe: JSON.stringify(nativeRecipe) });
    const loaded = Workbench.load(JSON.stringify(wasmRecipe));
    try {
      await compare(loaded, label, seed, 2 * YEARS);
      assertExact(json(JSON.parse(loaded.map())), map, "Legacy load redrew the saved map");
      const resaved = loaded.save();
      const savedRecipe = object(json(JSON.parse(resaved)));
      if (savedRecipe.geography !== geography) throw new Error("Re-saving lost the old geography");
      assertExact(json(JSON.parse(string(await rpc({ kind: "save" })))), savedRecipe,
        "Migrated legacy recipes differ");
      await rpc({ kind: "load", recipe: resaved });
      const replay = Workbench.load(resaved);
      try {
        await compare(replay, `${label}/replay`, seed, 2 * YEARS);
        assertExact(json(JSON.parse(replay.map())), map, "Re-saved old geography changed on replay");
        await rpc({ kind: "branch", generation: 0 });
        replay.branch(0);
        await compare(replay, `${label}/branch`, seed, 0);
        const branched = snapshot(replay, 0);
        // Branching intentionally assigns a new telling ID, not a new world.
        const expectedYearZero = {
          ...yearZero,
          overview: { ...object(yearZero.overview), telling: object(branched.overview).telling! },
        };
        assertExact(branched, expectedYearZero, "An old-geography branch changed its original year-zero world");
        const branchRecipe = replay.save();
        if (object(json(JSON.parse(branchRecipe))).geography !== geography) {
          throw new Error("Branching changed the old geography version");
        }
        await rpc({ kind: "load", recipe: branchRecipe });
        const branchReplay = Workbench.load(branchRecipe);
        try { await compare(branchReplay, `${label}/branch/replay`, seed, 0); }
        finally { branchReplay.free(); }
      } finally { replay.free(); }
      worlds++;
    } finally { loaded.free(); }
  } finally { original.free(); }
}

async function sample(): Promise<void> {
  // Execute the actual first-sight sample, not a hand-copied recipe. Its only
  // dependency is replaced at bundle time with the same Node-target facade;
  // sampleWorld still applies all its real actions through Workbench.act.
  const wasmPath = join(scratch, "pkg", "umran_web.js");
  const build = await Bun.build({
    entrypoints: [join(root, "web/src/sample.ts")], target: "bun",
    external: [wasmPath],
    plugins: [{ name: "sample-node-engine", setup(build) {
      build.onResolve({ filter: /^\.\/engine$/ }, () => ({ path: "engine", namespace: "parity" }));
      build.onLoad({ filter: /.*/, namespace: "parity" }, () => ({
        loader: "js",
        contents: `import { Workbench } from ${JSON.stringify(wasmPath)};
          export async function createEngine(seed, size) {
            const bench = new Workbench(seed, size);
            return { bench, act: action => bench.act(JSON.stringify(action)), dispose: () => bench.free() };
          }
          export function presetDesign(preset, seed) { return JSON.parse(Workbench.design(preset, seed)); }`,
      }));
    } }],
  });
  if (!build.success) throw new AggregateError(build.logs, "Could not bundle the actual sample");
  const source = await build.outputs[0].text();
  // The module is generated at runtime; a data URL keeps the bundled source in
  // memory while its external Node-target bindings remain on disk.
  // The bundled entry and adapter are our own code, with this exact interface.
  const sampleModule = await import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`) as {
    sampleWorld(): Promise<{ bench: WasmWorkbench; dispose(): void }>;
    SAMPLE_LAND: { seed: number };
  };
  const engine = await sampleModule.sampleWorld();
  const bench = engine.bench;
  try {
    const recipe = bench.save();
    await rpc({ kind: "load", recipe });
    const checkpoints = everyYear
      ? Array.from({ length: bench.latest() + 1 }, (_, i) => i * YEARS)
      : [0, 250, 1000, bench.latest() * YEARS];
    for (const year of checkpoints) {
      await compare(bench, "sample", sampleModule.SAMPLE_LAND.seed, year);
    }
    const replay = Workbench.load(recipe);
    try {
      await compare(replay, "sample/replay", sampleModule.SAMPLE_LAND.seed, replay.latest() * YEARS);
      if (digest(snapshot(replay, replay.latest())) !== digest(snapshot(bench, bench.latest()))) {
        throw new Error("Live/replayed sample differs in WASM");
      }
    } finally { replay.free(); }
    // The planner's full census, paths and refusals must agree before an
    // authored decision is applied. Reload the same starting history for each
    // intent so this exercises three alternatives, not an accidental chain.
    for (const intent of ["partition", "settlers", "migration"]) {
      await rpc({ kind: "load", recipe });
      const alternative = Workbench.load(recipe);
      try {
        const overview = object(json(JSON.parse(alternative.overview(alternative.latest()))));
        const point = overview.point;
        let chosen: JsonObject | undefined;
        for (const community of array(overview.communities).map(object).filter(c => c.ended === null)) {
          const request = { kind: "settlement", point, community: community.id, intent, share: 0.5, destination: -1 };
          const preview = object(json(JSON.parse(alternative.settlement(JSON.stringify(point), number(community.id), intent, 0.5, -1))));
          const nativePreview = await rpc(request);
          if (digest(preview) !== digest(nativePreview)) throw new Error(`Settlement options differ: ${intent}: ${firstDifference(nativePreview, preview)}`);
          const destination = array(preview.options).map(object).find(o => o.reason === null);
          if (!destination) continue;
          request.destination = number(destination.region);
          const account = object(json(JSON.parse(alternative.settlement(JSON.stringify(point), number(community.id), intent, 0.5, request.destination))));
          const nativeAccount = await rpc(request);
          if (digest(account) !== digest(nativeAccount)) throw new Error(`Settlement account differs: ${intent}: ${firstDifference(nativeAccount, account)}`);
          chosen = { kind: "settle", ...object(object(account.plan).choice) };
          break;
        }
        if (!chosen) throw new Error(`Sample needs a valid ${intent} for parity`);
        await rpc({ kind: "act", action: chosen });
        wasmAct(alternative, chosen);
        await compare(alternative, `sample/${intent}`, sampleModule.SAMPLE_LAND.seed, alternative.latest() * YEARS);
        const saved = alternative.save();
        await rpc({ kind: "load", recipe: saved });
        const restored = Workbench.load(saved);
        try { await compare(restored, `sample/${intent}/replay`, sampleModule.SAMPLE_LAND.seed, restored.latest() * YEARS); }
        finally { restored.free(); }
      } finally { alternative.free(); }
    }
    worlds++;
  } finally { engine.dispose(); }
}

try {
  const buildStarted = performance.now();
  await Promise.all([
    command(["cargo", "build", "--release", "-p", "umran-web", "--example", "parity"], join(cache, "native")),
    command([join(root, "node_modules/.bin/wasm-pack"), "build", "crates/umran-web", "--target", "nodejs", "--out-dir", join(scratch, "pkg"), "--out-name", "umran_web"], join(cache, "wasm")),
  ]);
  console.log(`Build wall time: ${((performance.now() - buildStarted) / 1000).toFixed(2)}s`);
  // Both targets' bindings are generated from the same wasm-bindgen class.
  const wasmModule = createRequire(import.meta.url)(join(scratch, "pkg", "umran_web.js")) as { Workbench: typeof WasmWorkbench };
  Workbench = wasmModule.Workbench;
  native = spawn(join(cache, "native/release/examples/parity"), [], { stdio: ["pipe", "pipe", "inherit"] });
  const lines = createInterface({ input: native.stdout! });
  const iterator = lines[Symbol.asyncIterator]();
  rpc = async (input: JsonObject) => {
    native!.stdin!.write(`${JSON.stringify(input)}\n`);
    const line = await iterator.next();
    if (line.done) throw new Error("Native parity driver ended before responding");
    const response = object(json(JSON.parse(line.value)));
    if ("error" in response) throw new Error(`Native: ${string(response.error)}`);
    if (!("ok" in response)) throw new Error("Native driver returned no result");
    return response.ok;
  };
  const nativeCatalog = await rpc({ kind: "catalog" });
  const wasmCatalog = json(JSON.parse(Workbench.catalog()));
  if (digest(nativeCatalog) !== digest(wasmCatalog)) {
    throw new Error(`Catalog differs: ${firstDifference(nativeCatalog, wasmCatalog)}`);
  }
  const runStarted = performance.now();
  for (const seed of onlySeed === undefined ? seeds : [onlySeed]) {
    await generated(seed);
    await relatedFamily(seed);
    for (const geography of ["spherical-v1", "continental-v2", "continental-v3", "continental-v4", "continental-v5"] as const) {
      await legacyGeography(seed, geography);
    }
  }
  if (onlySeed === undefined || onlySeed === 21) await sample();
  for (const [key, record] of drift) {
    console.log(`SURVEY ${key} through=${record.through} checked=${everyYear ? "all-generations" : "checkpoints"} discreteFirst=${record.discrete ?? "none"} floatFirst=${record.floats ?? "none"}`);
  }
  console.log(`Parity ${mismatches === 0 ? "passed" : "FAILED"}: ${worlds} worlds, ${checks} state/replay checks, ${mismatches} mismatching states; simulation wall time=${((performance.now() - runStarted) / 1000).toFixed(2)}s`);
  if (mismatches > 0) process.exitCode = 1;
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
} finally {
  native?.stdin?.end();
  native?.kill();
  await rm(scratch, { recursive: true, force: true });
  console.log(`Total wall time: ${((performance.now() - started) / 1000).toFixed(2)}s`);
}

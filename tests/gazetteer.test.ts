import { describe, expect, test } from "bun:test";
import { featureKey, gazette, type MapFeature } from "../web/src/gazetteer";
import type { ClimateView, Community, LakeNamesView, Overview, PlaceExonym, PlaceName, Region, RiverNamesView, Variety, WorldMap } from "../web/src/model";

const name = (variety: number, since: number, spelled: string): PlaceName => ({
  variety, since, spelled, language: `Speech ${variety}`, ipa: `${spelled}a`, meaning: "the valley", origin: "coined", by: variety, once: null,
});
const exonym = (variety: number, language: string, spelled: string): PlaceExonym => ({ variety, language, spelled, ipa: `${spelled}i`, heard: 2, once: null });
const map = {
  regions: [{ id: 0, terrain: "plains", areaKm2: 1234.6, climateZone: 0 }, { id: 1, terrain: "sea", areaKm2: 900, climateZone: null }, { id: 2, terrain: "forest", areaKm2: 500, climateZone: 1 }] as Region[],
  rivers: [{ id: 0, course: [2, 0], mouth: 0, catchment: [0, 2], lengthKm: 246.8, joins: null, joinAt: null }],
  lakes: [{ id: 2, regions: [0, 2], outlet: 0 }, { id: 3, regions: [2], outlet: null }],
  landmasses: [{ id: 0, kind: "continent", regions: [0, 2], anchor: 0 }],
} as WorldMap;
const overview = {
  varieties: Array.from({ length: 8 }, (_, id) => ({ id, name: `Speech ${id}`, spoken: id !== 7 })) as Variety[],
  communities: [
    { id: 0, name: "Nara", ipa: "nara", meaning: "people", coined: 0, variety: 0, size: 4321, ended: null, lands: [0], region: 0, exonyms: [{ by: 1, name: "Nali" }] },
    { id: 1, name: "Luma", ipa: "luma", meaning: "river folk", coined: 1, variety: 1, size: 1000, ended: null, lands: [2], region: 2, exonyms: [] },
    { id: 2, name: "Past", ended: 5, variety: 2, lands: [0] },
  ] as Community[],
  places: [{ region: 0, names: [name(0, 0, "A"), name(1, 3, "B"), name(2, 5, "C"), name(3, 7, "D"), name(4, 9, "E")],
    exonyms: [exonym(5, "Zeta", "Z"), exonym(6, "Alpha", "Y"), exonym(7, "Silent", "X")] }],
  cities: [{ id: 3, state: 2, region: 0, name: { name: "Narum", ipa: "narum", meaning: "great home" }, since: 4, size: 8000, townsfolk: null, makeup: [{ variety: 0, share: 0.6 }, { variety: 1, share: 0.4 }] }],
  states: [{ id: 2, name: "Realm", ipa: "relm", meaning: "valley realm", rulers: 0, standard: 1, capital: 0, rose: 3, fell: null }],
  continents: [{ landmass: 0, name: { name: "Maina", ipa: "maina", meaning: "great land", variety: 0, people: 0, witness: 0, since: 2 }, peoples: [0, 1], religions: [] }],
  religions: [{ id: 1, name: "Light", ipa: "lit", meaning: "the light", sacred: 0, land: 0, founded: 1, shrines: [{ region: 2, name: { name: "Summit", ipa: "sumit", meaning: "high light" }, kind: "mountain" }] }],
} as Overview;
const land: MapFeature = { kind: "land", region: 0 };

describe("map gazetteer", () => {
  test("selected exonym, current holders, alphabetic living exonyms, then three newest former holders", () => {
    const card = gazette(land, map, overview, 10, { selectedVariety: 5 });
    expect(card.title).toBe("Z");
    expect(card.sayings.map((s) => [s.spelled, s.relation])).toEqual([["Z", "selected"], ["E", "holders"], ["Y", "exonym"], ["D", "former"], ["C", "former"], ["B", "former"]]);
    expect(card.sayings[0].ipa).toBe("Zi");
    expect(card.open?.subject).toEqual(land);
  });
  test("a selected holder is not repeated and an unattested selection adds nothing", () => {
    expect(gazette(land, map, overview, 10, { selectedVariety: 4 }).sayings.filter((s) => s.spelled === "E").map((s) => s.relation)).toEqual(["selected"]);
    expect(gazette(land, map, overview, 10, { selectedVariety: 99 }).sayings[0].relation).toBe("holders");
    expect(gazette(land, map, overview, 10, {}).sayings.filter((s) => s.relation === "exonym").map((s) => s.language)).toEqual(["Alpha", "Zeta"]);
  });
  test("a selected former holder is first, not repeated in the former ledger", () => {
    const card = gazette(land, map, overview, 10, { selectedVariety: 3 });
    expect(card.sayings[0]).toMatchObject({ spelled: "D", relation: "selected", since: 7 });
    expect(card.sayings.filter((s) => s.relation === "former").map((s) => s.spelled)).toEqual(["C", "B", "A"]);
  });
  test("rewinding excludes future names and unheard exonyms", () => {
    const card = gazette(land, map, overview, 1, { selectedVariety: 5 });
    expect(card.sayings.map((s) => s.spelled)).toEqual(["A"]);
  });
  test("input ledgers stay unchanged", () => {
    const before = JSON.stringify(overview);
    gazette(land, map, overview, 10, { selectedVariety: 6 });
    expect(JSON.stringify(overview)).toBe(before);
  });
  test("unnamed land, sea, and veiled land have honest fallbacks", () => {
    expect(gazette({ kind: "land", region: 2 }, map, overview, 10, {}).title).toBe("Unnamed forest");
    expect(gazette({ kind: "land", region: 1 }, map, overview, 10, { known: new Set() }).title).toBe("Sea");
    expect(gazette(land, map, overview, 10, { known: new Set([2]) })).toMatchObject({ title: "Unknown land", sayings: [], facts: [], open: null });
  });
  test("land facts use facade area, living peoples and the zone's weather", () => {
    const climate = { regions: [{ id: 0, zone: 0 }], zones: [{ id: 0, wetness: -0.1, warmth: 0, severity: 3 }] } as unknown as ClimateView;
    expect(gazette(land, map, overview, 10, { climate }).facts).toEqual([
      { label: "Terrain", value: "Plains" }, { label: "Area", value: "1,235 km²" }, { label: "Peoples", value: "Nara" }, { label: "Weather", value: "much drier than usual" },
    ]);
  });
  test("city title keeps its own facade name and exposes the land, makeup and state card", () => {
    const card = gazette({ kind: "city", city: 3, region: 0 }, map, overview, 10, { selectedVariety: 5 });
    expect(card.title).toBe("Narum");
    expect(card.subtitle).toBe("stands on Z");
    expect(card.sayings[0]).toMatchObject({ spelled: "Narum", ipa: "narum", variety: 1 });
    expect(card.sayings[1].relation).toBe("selected");
    expect(card.facts).toContainEqual({ label: "Since", value: "year 100" });
    expect(card.facts).toContainEqual({ label: "Speech", value: "Speech 0 60%, Speech 1 40%" });
    expect(card.open?.subject).toEqual({ kind: "state", id: 2 });
    expect(gazette({ kind: "city", city: 3, region: 0 }, map, overview, 3, {}).sayings).toEqual([]);
  });
  test("river names use their own ledger, length and catchment", () => {
    const riverNames: RiverNamesView[] = [{ river: 0, names: [name(0, 0, "Flow")], exonyms: [exonym(1, "Other", "Flew")] }];
    const card = gazette({ kind: "river", river: 0, region: 0 }, map, overview, 10, { selectedVariety: 1, riverNames });
    expect(card.title).toBe("Flew");
    expect(card.sayings.map((s) => s.relation)).toEqual(["selected", "holders"]);
    expect(card.facts).toEqual([{ label: "Length", value: "247 km" }, { label: "Catchment", value: "2 lands" }]);
    expect(card.open?.subject).toEqual({ kind: "river", id: 0 });
  });
  test("lake names share the river ordering and open the lake with shore and outlet facts", () => {
    const lakeNames: LakeNamesView[] = [{ lake: 2, names: [name(0, 0, "Old lake"), name(1, 3, "Lake")],
      exonyms: [exonym(5, "Zeta", "Tarn"), exonym(6, "Alpha", "Pool")] }];
    const riverNames: RiverNamesView[] = [{ river: 0, names: [name(0, 0, "Flow")], exonyms: [exonym(5, "Zeta", "Flew")] }];
    const feature: MapFeature = { kind: "lake", lake: 2, region: 0 };
    const card = gazette(feature, map, overview, 10, { selectedVariety: 5, lakeNames, riverNames });
    expect(card.title).toBe("Tarn");
    expect(card.sayings.map((s) => [s.spelled, s.relation])).toEqual([
      ["Tarn", "selected"], ["Lake", "holders"], ["Pool", "exonym"], ["Old lake", "former"],
    ]);
    expect(card.facts).toEqual([{ label: "Shore lands", value: "2" }, { label: "Outlet", value: "Flew" }]);
    expect(card.open?.subject).toEqual({ kind: "lake", id: 2 });
    expect(featureKey({ ...feature, region: 2 })).toBe("lake:2");
    expect(gazette(feature, map, overview, 1, { lakeNames }).sayings.map((s) => s.spelled)).toEqual(["Old lake"]);
    expect(gazette(feature, map, overview, 10, { lakeNames, known: new Set([2]) })).toMatchObject({ sayings: [], facts: [], open: null });
    expect(gazette({ kind: "lake", lake: 3, region: 2 }, map, overview, 10, {}).facts).toEqual([
      { label: "Shore lands", value: "1" }, { label: "Outlet", value: "Closed basin" },
    ]);
  });
  test("continent has one recorded holders' name and its meaning", () => {
    const card = gazette({ kind: "continent", landmass: 0, region: 0 }, map, overview, 10, { selectedVariety: 5 });
    expect(card.title).toBe("Maina");
    expect(card.sayings).toHaveLength(1);
    expect(card.sayings[0]).toMatchObject({ relation: "holders", meaning: "great land", ipa: "maina" });
    expect(card.open?.subject).toEqual({ kind: "continent", landmass: 0 });
  });
  test("people arrange real exonyms without inventing their absent IPA", () => {
    const card = gazette({ kind: "people", community: 0, region: 0 }, map, overview, 10, { selectedVariety: 1 });
    expect(card.sayings.map((s) => [s.spelled, s.relation, s.ipa])).toEqual([["Nali", "selected", ""], ["Nara", "holders", "nara"]]);
    expect(card.facts).toContainEqual({ label: "Speech", value: "Speech 0" });
    expect(card.open?.subject).toEqual({ kind: "people", id: 0 });
  });
  test("shrine and state route to existing faith and state cards", () => {
    expect(gazette({ kind: "shrine", religion: 1, region: 2 }, map, overview, 10, {})).toMatchObject({ title: "Summit", subtitle: "holy to Light", open: { subject: { kind: "religion", id: 1 } } });
    const state = gazette({ kind: "state", state: 2, region: 0 }, map, overview, 10, {});
    expect(state.facts).toContainEqual({ label: "Capital", value: "Narum" });
    expect(state.facts).toContainEqual({ label: "Standing since", value: "year 75" });
  });
  test("feature identity ignores the pointer's region except for distinct holy places", () => {
    const features: MapFeature[] = [land, { kind: "city", city: 3, region: 0 }, { kind: "river", river: 0, region: 0 }, { kind: "continent", landmass: 0, region: 0 }, { kind: "shrine", religion: 1, region: 2 }, { kind: "people", community: 0, region: 0 }, { kind: "state", state: 2, region: 0 }];
    expect(new Set(features.map(featureKey)).size).toBe(features.length);
    expect(featureKey({ kind: "river", river: 0, region: 2 })).toBe(featureKey(features[2]));
    expect(featureKey({ kind: "shrine", religion: 1, region: 0 })).not.toBe(featureKey(features[4]));
  });
});

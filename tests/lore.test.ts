import { describe, expect, test } from "bun:test";
import { causePhrase, cityLabel, faithTeaching, landCardLabel, otherNames, peopleLabel, seasonalPhrase, stateLabel } from "../web/src/lore";
import type { Annal, Overview, PlaceExonym, ReligionView, Seasons, WorldMap } from "../web/src/model";

const entry = (id: string, kind: Annal["kind"], generation: number, fields: Partial<Annal> = {}) => ({
  id, kind, generation, members: [], languages: [], variety: null,
  text: "Recorded event.", notes: [], peoples: [], lands: [], states: [],
  religions: [], crafts: [], laws: [], specimen: [], temper: null, ...fields,
}) as unknown as Annal;

const overview = (fields: Partial<Overview>) => fields as Overview;

describe("recorded causes", () => {
  test("resolve the individual trigger inside a grouped year and give its year", () => {
    const trigger = entry("world:12", "contact", 50);
    const response = entry("world:13", "conversion", 51, { cause: { event: 12, mechanism: "contact" } });
    const phrase = causePhrase(response, overview({ annals: [entry("group:50", "neighbours", 50, { members: [trigger] } as Partial<Annal>)], places: [] }));
    expect(phrase?.trigger).toBe(trigger);
    expect(phrase?.text).toContain("1250");
  });

  test("name the trigger's land as it was called in the trigger's year", () => {
    const trigger = entry("world:12", "hardship", 50, { lands: [0] });
    const response = entry("world:13", "faith", 51, { cause: { event: 12, mechanism: "hardship" } });
    const phrase = causePhrase(response, overview({
      annals: [trigger],
      places: [{ region: 0, names: [{ spelled: "Nūhupam", since: 0 }, { spelled: "Later name", since: 60 }] }] as unknown as Overview["places"],
    }));
    expect(phrase?.text).toContain("Nūhupam");
    expect(phrase?.text).not.toContain("Later name");
  });

  test("add nothing when no cause was recorded or its trigger is missing", () => {
    const empty = overview({ annals: [], places: [] });
    expect(causePhrase(entry("world:1", "found", 0), empty)).toBeNull();
    expect(causePhrase(entry("world:2", "temper", 1, { temper: { cause: "hardship" } as Annal["temper"] }), empty)).toBeNull();
    expect(causePhrase(entry("world:3", "faith", 1, { cause: { event: 999, mechanism: "hardship" } }), empty)).toBeNull();
  });
});

describe("seasonal profiles", () => {
  test("choose the cold-season phrase at both amplitude boundaries", () => {
    const profile: Seasons = { amplitude: 0, wet: "none", floods: false };
    expect(seasonalPhrase({ ...profile, amplitude: 0.249 })).toContain("mild");
    expect(seasonalPhrase({ ...profile, amplitude: 0.25 })).toContain("marked");
    expect(seasonalPhrase({ ...profile, amplitude: 0.499 })).toContain("marked");
    expect(seasonalPhrase({ ...profile, amplitude: 0.5 })).toContain("winter");
  });

  test("choose each rain regime and include floods only when recorded", () => {
    const profile: Seasons = { amplitude: 0, wet: "none", floods: false };
    expect(seasonalPhrase(profile)).toContain("any time");
    expect(seasonalPhrase({ ...profile, wet: "summer" })).toContain("summer");
    expect(seasonalPhrase({ ...profile, wet: "winter" })).toContain("winter");
    expect(seasonalPhrase({ ...profile, wet: "monsoon" })).toContain("monsoon");
    expect(seasonalPhrase(profile)).not.toContain("floods");
    expect(seasonalPhrase({ ...profile, floods: true })).toContain("floods");
  });
});

describe("faith teaching", () => {
  const faith = (doctrine: ReligionView["doctrine"]) => ({ doctrine }) as ReligionView;

  test("omit neutral tenets and include held and rejected stances at the threshold", () => {
    expect(faithTeaching(faith([]))).toBeNull();
    expect(faithTeaching(faith([{ tenet: "images", stance: 0.149 }, { tenet: "hierarchy", stance: -0.149 }]))).toBeNull();
    const teaching = faithTeaching(faith([{ tenet: "images", stance: 0.15 }, { tenet: "hierarchy", stance: -0.15 }]));
    expect(teaching).toContain("venerated");
    expect(teaching).toContain("no order");
    expect(teaching).toContain("; ");
  });
});

describe("land-name apparatus", () => {
  test("group shared exonyms once with distinct languages and the earliest hearing", () => {
    const names = [
      { spelled: "Oro", ipa: "oro", variety: 1, heard: 8, once: null },
      { spelled: "Oro", ipa: "oro", variety: 2, heard: 4, once: "Ora" },
      { spelled: "Oro", ipa: "oro", variety: 2, heard: 6, once: null },
      { spelled: "Home", ipa: "hom", variety: 3, heard: 0, once: null },
    ] as PlaceExonym[];
    expect(otherNames(names, "Home")).toEqual([
      { spelled: "Oro", ipa: "oro", varieties: [1, 2], heard: 4, once: "Ora", same: false },
      { spelled: "Home", ipa: "hom", varieties: [3], heard: 0, once: null, same: true },
    ]);
    expect(names[0].heard).toBe(8);
  });
});

describe("reading names", () => {
  test("namesake lands and peoples remain distinct without changing engine names", () => {
    const world = overview({
      varieties: [],
      places: [{ region: 0, names: [{ spelled: "Poghtz" }], exonyms: [] }, { region: 1, names: [{ spelled: "Poghtz" }], exonyms: [] }, { region: 2, names: [{ spelled: "Pōpn" }], exonyms: [] }] as Overview["places"],
      communities: [{ id: 0, name: "Kin" }, { id: 1, name: "Kin" }] as Overview["communities"],
    });
    const map = { regions: [{ id: 0, terrain: "forest", center: [0, 10] }, { id: 1, terrain: "forest", center: [0, -10] }, { id: 2, terrain: "forest", center: [0, 0] }] } as WorldMap;
    expect(new Set([0, 1].map((id) => landCardLabel(world, map, id))).size).toBe(2);
    expect(landCardLabel(world, map, 2)).toBe("Pōpn");
    expect(new Set([0, 1].map((id) => peopleLabel(world, id))).size).toBe(2);
    expect(world.places[0].names[0].spelled).toBe("Poghtz");
    expect(landCardLabel(world, map, 0)).toContain("northern forest");
    expect(landCardLabel(world, map, 1)).toContain("southern forest");
  });

  test("successive realms of one people keep distinct labels even when their names diverge", () => {
    const world = overview({ states: [
      { id: 0, name: "Iffi", rulers: 0, rose: 88 },
      { id: 1, name: "Iffi", rulers: 0, rose: 100 },
      { id: 2, name: "Iff", rulers: 0, rose: 106 },
      { id: 3, name: "Iffi", rulers: 1, rose: 100 },
    ] as Overview["states"] });
    expect(new Set(world.states.map((state) => stateLabel(world, state.id))).size).toBe(4);
    expect(stateLabel(world, 0)).toContain(String(88 * 25));
  });

  test("cities in different lands are not identified by spelling alone", () => {
    const world = overview({ cities: [
      { id: 0, region: 0, name: { name: "Oro" } },
      { id: 1, region: 1, name: { name: "Oro" } },
    ] as Overview["cities"] });
    const map = { regions: [{ terrain: "hills", center: [0, 0] }, { terrain: "plains", center: [0, 0] }] } as WorldMap;
    expect(cityLabel(world, map, world.cities[0])).toContain("hills");
    expect(cityLabel(world, map, world.cities[1])).toContain("plains");
  });

  test("terrain resolves namesakes before direction, with local ordinals only for geographic ties", () => {
    const world = overview({ varieties: [], places: [0, 1, 2].map((region) => ({ region, names: [{ spelled: "Oro" }], exonyms: [] })) as Overview["places"] });
    const map = { regions: [{ terrain: "hills", center: [0, 0] }, { terrain: "plains", center: [0, 0] }, { terrain: "plains", center: [0, 0] }] } as WorldMap;
    expect(landCardLabel(world, map, 0)).toBe("Oro (hills)");
    expect(landCardLabel(world, map, 1)).toBe("Oro (central plains, 1)");
    expect(landCardLabel(world, map, 2)).toBe("Oro (central plains, 2)");
  });

  test("only the two directionally tied namesakes receive ordinals", () => {
    const world = overview({ places: [0, 1, 2].map((region) => ({ region, names: [{ spelled: "Oro" }] })) as Overview["places"] });
    const map = { regions: [
      { terrain: "plains", center: [-3, 0] },
      { terrain: "plains", center: [1, 0] },
      { terrain: "plains", center: [2, 0] },
    ] } as WorldMap;
    expect(landCardLabel(world, map, 0)).toBe("Oro (western plains)");
    expect(landCardLabel(world, map, 1)).toBe("Oro (eastern plains, 1)");
    expect(landCardLabel(world, map, 2)).toBe("Oro (eastern plains, 2)");
  });

  test("historical spellings do not introduce invisible current-name ties", () => {
    const world = overview({ places: [
      { region: 0, names: [{ spelled: "Oro" }] },
      { region: 1, names: [{ spelled: "Oro" }] },
      { region: 2, names: [{ spelled: "Oro" }, { spelled: "New" }] },
    ] as Overview["places"] });
    const map = { regions: [
      { terrain: "plains", center: [-3, 0] },
      { terrain: "plains", center: [1, 0] },
      { terrain: "plains", center: [2, 0] },
    ] } as WorldMap;
    expect(landCardLabel(world, map, 0)).toBe("Oro (western plains)");
    expect(landCardLabel(world, map, 1)).toBe("Oro (eastern plains)");
    expect(landCardLabel(world, map, 1, "Oro", [0, 1, 2])).toBe("Oro (eastern plains, 1)");
  });

  test("east and west follow the short direction across the date line", () => {
    const world = overview({ varieties: [], places: [0, 1].map((region) => ({ region, names: [{ spelled: "Oro" }], exonyms: [] })) as Overview["places"] });
    const map = { regions: [{ terrain: "hills", center: [179, 0] }, { terrain: "hills", center: [-179, 0] }] } as WorldMap;
    expect(landCardLabel(world, map, 0)).toContain("western");
    expect(landCardLabel(world, map, 1)).toContain("eastern");
  });
});

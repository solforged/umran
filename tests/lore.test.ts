import { describe, expect, test } from "bun:test";
import { causePhrase, faithTeaching, otherNames, seasonalPhrase } from "../web/src/lore";
import type { Annal, Overview, PlaceExonym, ReligionView, Seasons } from "../web/src/model";

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

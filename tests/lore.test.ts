import { describe, expect, test } from "bun:test";
import { causePhrase } from "../web/src/lore";
import type { Annal, Overview } from "../web/src/model";

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

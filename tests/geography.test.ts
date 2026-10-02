import { describe, expect, test } from "bun:test";
import { riverLength, riverPoints } from "../web/src/components/MapView";
import type { River, WorldMap } from "../web/src/model";

describe("river courses on the chart", () => {
  test("an ambiguous tributary ends at its actual confluence, not the first adjacent parent reach", () => {
    const tributary: River = { id: 0, course: [0], mouth: 1, catchment: [0], joins: 1, joinAt: 3 };
    const parent: River = { id: 1, course: [2, 3], mouth: 1, catchment: [0, 2, 3], joins: null, joinAt: null };
    const map = {
      kmPerUnit: 100,
      regions: [
        { site: [0, 0], neighbours: [2, 3] },
        { site: [999, 999] },
        { site: [10, 0] },
        { site: [3, 4] },
      ],
      rivers: [tributary, parent],
    } as WorldMap;
    expect(riverPoints(map, tributary)).toEqual([[0, 0], [3, 4]]);
    expect(riverLength(map, tributary)).toBe(500);
  });

  test("a sea outlet reaches the shared coast, not the centre of the sea region", () => {
    const river: River = { id: 0, course: [0], mouth: 1, catchment: [0], joins: null, joinAt: null };
    const map = {
      kmPerUnit: 100,
      regions: [
        { site: [1, 1], outline: [[0, 0], [2, 0], [2, 2], [0, 2]] },
        { site: [4, 1], outline: [[2, 0], [4, 0], [4, 2], [2, 2]] },
      ],
      rivers: [river],
    } as WorldMap;
    expect(riverPoints(map, river).at(-1)).toEqual([2, 1]);
    expect(riverLength(map, river)).toBe(100);
  });
});

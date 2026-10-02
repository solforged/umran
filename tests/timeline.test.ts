import { describe, expect, test } from "bun:test";
import { marksFromAnnals, rulerTicks } from "../web/src/components/Timeline";
import type { Annal } from "../web/src/model";
import { YEARS } from "../web/src/model";

const annal = (generation: number, kind: Annal["kind"]): Annal => ({
  generation, kind, text: "", notes: [], variety: null, peoples: [], lands: [], states: [], religions: [], crafts: [], laws: [], specimen: [], temper: null,
});

describe("timeline marks", () => {
  test("keeps turning points in generation and precedence order, dropping routine events", () => {
    const source = [
      annal(8, "craft"), annal(2, "shift"), annal(1, "split"), annal(4, "rose"),
      annal(8, "found"), annal(6, "faith"), annal(5, "fell"), annal(3, "conquest"),
      annal(0, "neighbours"), annal(7, "migration"), annal(9, "meaning"),
    ];
    const { marks } = marksFromAnnals(source);
    expect(marks.map(({ generation, kind }) => [generation, kind])).toEqual([
      [1, "split"], [2, "shift"], [3, "conquest"], [4, "rose"],
      [5, "fell"], [6, "faith"], [8, "found"], [8, "craft"],
    ]);
    expect(marks[6].annal).toBe(source[4]);
    expect(source[0].kind).toBe("craft");
  });

  test("folds sound changes within a generation without merging different generations", () => {
    const { marks, sounds } = marksFromAnnals([
      annal(8, "law"), annal(3, "law"), annal(8, "law"), annal(8, "law"),
    ]);
    expect(marks).toEqual([]);
    expect(sounds.map(({ generation, count }) => [generation, count])).toEqual([[3, 1], [8, 3]]);
  });
});

describe("timeline ruler", () => {
  test("chooses the smallest readable year step for the measured width", () => {
    expect(rulerTicks(200, 600).step).toBe(500);
    expect(rulerTicks(200, 1400).step).toBe(200);
    expect(rulerTicks(0, 600)).toEqual({ years: [0], step: 100 });
    const { years, step } = rulerTicks(193, 600);
    expect(years).toEqual(Array.from({ length: Math.floor(193 * YEARS / step) + 1 }, (_, i) => i * step));
    expect(years.at(-1)).toBeLessThanOrEqual(193 * YEARS);
  });
});

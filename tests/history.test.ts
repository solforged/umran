import { describe, expect, test } from "bun:test";
import { filterHistory, INITIAL_HISTORY, searchText, subjectHistory } from "../web/src/history";
import type { Annal, Overview, Variety, WorldMap } from "../web/src/model";

const annal = (generation: number, kind: Annal["kind"], variety: number | null, text = "A moment."): Annal => ({
  id: `${kind}:${variety}:${generation}`, members: [], languages: variety === null ? [] : [variety],
  generation, kind, variety, text, notes: [], peoples: [], lands: [], states: [], religions: [], crafts: [], laws: [], specimen: [], temper: null,
});
const family = [
  { id: 0, parent: null, forkedAt: null },
  { id: 1, parent: 0, forkedAt: 4 },
  { id: 2, parent: 1, forkedAt: 8 },
] as Variety[];

describe("chronicle readings", () => {
  test("a subject's story includes its own encounter rather than unrelated neighbours in the year's summary", () => {
    const a = { ...annal(2, "neighbours", null, "A meets B"), id: "world:3", peoples: [0, 1] };
    const b = { ...annal(2, "neighbours", null, "C meets D"), id: "world:4", peoples: [2, 3] };
    const aggregate = { ...annal(2, "neighbours", null), peoples: [0, 1, 2, 3], members: [a, b] };
    const overview = { annals: [aggregate] } as Overview;
    expect(subjectHistory({ kind: "people", id: 0 }, overview, {} as WorldMap)).toEqual([a]);
  });
  test("an unfiltered reading includes sound changes and reads latest first without mutating history", () => {
    const source = [annal(1, "found", 0), annal(2, "law", 0)];
    expect(filterHistory(source, family, INITIAL_HISTORY).map((a) => a.kind)).toEqual(["law", "found"]);
    expect(source.map((a) => a.kind)).toEqual(["found", "law"]);
  });
  test("follows ancestors only until their descendant forked", () => {
    const source = [annal(3, "law", 0), annal(5, "law", 0), annal(7, "law", 1), annal(9, "law", 1), annal(10, "law", 2)];
    expect(filterHistory(source, family, { ...INITIAL_HISTORY, sounds: 2, order: "oldest" }).map((a) => a.generation)).toEqual([3, 7, 10]);
  });
  test("rewinding before the selected language exists still renders non-sound events", () => {
    const source = [annal(0, "found", 0), annal(1, "law", 0)];
    expect(filterHistory(source, family.slice(0, 1), { ...INITIAL_HISTORY, sounds: 2 })).toEqual([source[0]]);
  });
  test("search ignores accents and requires every term, including notes", () => {
    const source = [annal(1, "faith", 0, "The Miwfafȳwa began."), annal(2, "faith", 1, "Another faith began.")];
    source[0].notes = ["A mountain shrine."];
    expect(filterHistory(source, family, { ...INITIAL_HISTORY, query: "miwfafywa shrine" })).toEqual([source[0]]);
    expect(searchText("Cīif Ātewi SHȲFI")).toBe("ciif atewi shyfi");
  });
  test("categories and omitted sound changes compose without hiding other language events", () => {
    const source = [annal(1, "law", 0), annal(2, "shift", 0), annal(3, "migration", 0), annal(4, "grammar", 0)];
    expect(filterHistory(source, family, { ...INITIAL_HISTORY, group: "Languages & words", sounds: "none" })).toEqual([source[3], source[1]]);
  });
});

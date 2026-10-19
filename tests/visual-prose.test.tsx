import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { GrammarSketch, type ChapterContext } from "../web/src/components/LanguageChapter";
import { quietLine, quietRuns, quietSummary, speechLine, type EraYear } from "../web/src/eras";
import type { Annal, Overview, Variety } from "../web/src/model";

const quiet = (kind: Annal["kind"], members: Annal[] = []) => ({
  kind, members, text: "Recorded activity.", languages: [], variety: null,
}) as Annal;

describe("quiet-year disclosure", () => {
  test("keep the collapsed account short, counting grouped entries individually", () => {
    const year: EraYear = { generation: 160, headlines: [], quiet: [quiet("law"), quiet("neighbours", [quiet("contact"), quiet("contact")]), quiet("climate")] };
    expect(quietSummary(year)).toMatch(/\b4 quieter entries\b/);
    const detail = quietLine(year, { varieties: [] } as unknown as Overview);
    expect(detail).toContain(";");
    expect(detail.length).toBeGreaterThan(quietSummary(year).length);
    expect(quietSummary({ ...year, quiet: [quiet("law")] })).toMatch(/\b1 quieter entry\b/);
  });
  test("read consecutive quiet years as one run, broken by a headline year or a year kept apart", () => {
    const year = (generation: number, headline = false): EraYear => ({ generation, headlines: headline ? [quiet("found")] : [], quiet: [quiet("law")] });
    const runs = quietRuns([year(1), year(2), year(3), year(4, true), year(5), year(6), year(7)], (y) => y.generation === 6);
    expect(runs.map((r) => [r.generation, r.through ?? null, r.quiet.length])).toEqual([[1, 3, 3], [4, null, 1], [5, null, 1], [6, null, 1], [7, null, 1]]);
    expect(quietSummary(runs[0])).toMatch(/^In these years: 3 quieter entries/);
  });
});

describe("an era's speech", () => {
  const law = (variety: number) => ({ ...quiet("law"), variety });
  const world = { varieties: [{ id: 0, name: "Tal" }, { id: 1, name: "Iffi" }, { id: 2, name: "Opuw" }] } as unknown as Overview;
  test("counts grouped entries one by one and names the languages that changed most", () => {
    const annals = [law(1), law(0), { ...quiet("law"), languages: [1] }, quiet("grammar", [quiet("grammar"), quiet("tone-gained")]), quiet("coinage"), quiet("neighbours")];
    expect(speechLine(annals, world, [2])).toBe("In speech: Opuw arose; three sound changes, most in Iffi and Tal; one new word; two grammar changes.");
    expect(speechLine(annals, world, [2], true)).toBe("one language arose · three sound changes · one new word · two grammar changes");
  });
  test("says nothing when speech was quiet", () => {
    expect(speechLine([quiet("neighbours"), quiet("climate")], world)).toBeNull();
  });
});

describe("harmony marker presentation", () => {
  const render = (spellings: string[]) => {
    const variety = {
      grammar: {
        order: "SOV", marking: "case", possessor: "before", sample: null, classes: [], pronounCase: "none",
        agreement: { stage: "none", proDrop: false, side: "prefix", since: null, cells: [], history: [] },
        categories: [{ category: "past", label: "Past", description: "Past time.", eligible: 1, howSynthetic: 1, contrastRetention: 1 }],
        markers: [{ id: 0, category: "past", spelled: "ry", ipa: "ry", said: null, kind: "bound", side: "suffix", share: 1, productive: true, born: 0, retired: null, history: [], origin: { kind: "founding" }, alternants: spellings.map((spelled) => ({ spelled, ipa: spelled })) }],
      }, pronouns: [],
    } as unknown as Variety;
    return renderToStaticMarkup(<GrammarSketch variety={variety} ctx={{} as ChapterContext} />);
  };
  test("only additional unique spellings follow also", () => {
    const html = render(["ri", "ry", "ri"]);
    expect(html).toContain("by vowel harmony also ri");
    expect(html).not.toContain("also ri, ry");
    expect(html).not.toContain("also ri, ri");
  });
  test("omit also when no additional form remains", () => {
    expect(render(["ry", "ry"])).not.toContain("by vowel harmony also");
    expect(render([])).not.toContain("by vowel harmony also");
  });
});

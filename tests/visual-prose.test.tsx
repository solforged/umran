import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { GrammarSketch, type ChapterContext } from "../web/src/components/LanguageChapter";
import { quietLine, quietSummary, type EraYear } from "../web/src/eras";
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
});

describe("harmony marker presentation", () => {
  const render = (spellings: string[]) => {
    const variety = {
      grammar: {
        order: "SOV", marking: "case", possessor: "before", sample: null, classes: [],
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

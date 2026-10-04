import type { Annal, Overview } from "./model";
import { YEARS } from "./model";
import { CHRONICLE, countWord } from "./lore";

export type Era = {
  opening: Annal | null;
  start: number;
  end: number;
  /// The opening entry's full text with its year; the folio's header.
  title: string;
  /// A few words for the contents and the book: "Iffi rises".
  heading: string;
  years: EraYear[];
};
export type EraYear = {
  generation: number;
  headlines: Annal[];
  quiet: Annal[];
};
export const ERA_OPENERS: ReadonlySet<Annal["kind"]> = new Set(["rose", "fell", "conquest", "faith", "classical"]);
export const QUIET_KINDS: ReadonlySet<Annal["kind"]> = new Set(["neighbours", "spread", "displaced", "parted", "contact", "climate", "river-flow", "meaning", "respelling", "temper", "law", "grammar", "pronoun-renewed", "pronoun-borrowed", "class-merged", "harmony-gained", "harmony-lost", "tone-gained", "tone-lost", "coinage", "calque"]);
/// An opener within this many generations of an era's start stays a
/// headline inside it; two hundred years is the shortest era.
export const MIN_ERA = 8;

/// A short heading for an opener, from the subjects it names, in their
/// current names; the full entry stays the era's title.
export function heading(annal: Annal, overview: Overview): string {
  const state = overview.states[annal.states[0] ?? -1];
  switch (annal.kind) {
    case "rose": return state ? `${state.name} rises` : CHRONICLE.rose;
    case "fell": return state ? `${state.name} falls` : CHRONICLE.fell;
    case "conquest": {
      const [ruler, ruled] = annal.peoples.map((id) => overview.communities[id]?.name);
      return ruler && ruled ? `The ${ruler} conquer the ${ruled}` : CHRONICLE.conquest;
    }
    case "faith": { const faith = overview.religions[annal.religions[0] ?? -1]; return faith ? `${faith.name} is founded` : CHRONICLE.faith; }
    case "classical": { const tongue = state?.classical ? overview.varieties[state.classical.variety]?.name : undefined; return tongue ?? CHRONICLE.classical; }
    default: return annal.text.replace(/\*/g, "");
  }
}

/// Boundaries come from the unfiltered telling, never from a reading filter.
export function eras(annals: Annal[], latest: number, overview: Overview): Era[] {
  const result: Era[] = [{ opening: null, start: 0, end: latest, title: CHRONICLE.first, heading: CHRONICLE.first, years: [] }];
  for (const annal of annals) {
    if (!ERA_OPENERS.has(annal.kind) || annal.generation > latest) continue;
    const previous = result[result.length - 1];
    if (annal.generation < previous.start + MIN_ERA) continue;
    previous.end = annal.generation - 1;
    result.push({ opening: annal, start: annal.generation, end: latest,
      title: `Year ${annal.generation * YEARS} · ${annal.text.replace(/\*/g, "")}`, heading: heading(annal, overview), years: [] });
  }
  if (result.length < 2 && latest > 40) {
    result.length = 0;
    for (let start = 0; start < latest; start += 20) {
      const boundary = Math.min(start + 20, latest);
      const title = `Years ${start * YEARS}–${boundary * YEARS}`;
      result.push({ opening: null, start, end: boundary === latest ? latest : boundary - 1, title, heading: title, years: [] });
    }
  }
  let index = 0;
  for (const annal of annals) {
    if (annal.generation > latest) continue;
    while (index + 1 < result.length && annal.generation >= result[index + 1].start) index++;
    const era = result[index];
    let year = era.years.at(-1);
    if (!year || year.generation !== annal.generation) {
      year = { generation: annal.generation, headlines: [], quiet: [] };
      era.years.push(year);
    }
    (QUIET_KINDS.has(annal.kind) ? year.quiet : year.headlines).push(annal);
  }
  return result;
}

/// A folded year describes recorded activity; names remain the engine's.
export function quietLine(year: EraYear, overview: Overview): string {
  const byKind = new Map<Annal["kind"], Annal[]>();
  for (const annal of year.quiet) {
    const entries = byKind.get(annal.kind) ?? [];
    entries.push(annal);
    byKind.set(annal.kind, entries);
  }
  const parts: string[] = [];
  const neighbours = byKind.get("neighbours");
  if (neighbours) {
    const count = neighbours.reduce((sum, a) => sum + (a.members.length || 1), 0);
    parts.push(`${countWord(count)} ${CHRONICLE.neighbours[count === 1 ? 0 : 1]}`);
  }
  const spread = byKind.get("spread");
  if (spread) {
    const count = spread.reduce((sum, a) => sum + (a.members.length || 1), 0);
    parts.push(`${count === 1 ? "a people" : "peoples"} spread into ${countWord(count)} ${CHRONICLE.lands[count === 1 ? 0 : 1]}`);
  }
  // Crowding is not interchangeable bookkeeping: retain who crowded whom.
  for (const annal of byKind.get("displaced") ?? []) parts.push(annal.text.replace(/\*/g, "").replace(/[.!?]$/, "").replace(/^The\b/, "the"));
  for (const kind of ["parted", "contact"] as const) {
    const entries = byKind.get(kind);
    if (entries) parts.push(`${countWord(entries.length)} ${CHRONICLE[kind][entries.length === 1 ? 0 : 1]}`);
  }
  if (byKind.has("climate")) parts.push(CHRONICLE.weather);
  for (const [kind, noun] of [["river-flow", "rivers"], ["temper", "temper"], ["law", "law"], ["grammar", "grammar"], ["meaning", "meaning"], ["respelling", "respelling"]] as const) {
    const entries = byKind.get(kind);
    if (!entries) continue;
    let phrase = `${entries.length === 1 ? "a" : countWord(entries.length)} ${CHRONICLE[noun][entries.length === 1 ? 0 : 1]}`;
    if (kind === "law" || kind === "grammar") {
      const ids = [...new Set(entries.flatMap((a) => a.variety === null ? a.languages : [a.variety]))];
      const names = ids.map((id) => overview.varieties[id]?.name).filter((name): name is string => name !== undefined);
      const first = names.slice(0, 2).map((name) => `the ${name}`);
      if (names.length > 2) first.push(`${countWord(names.length - 2)} more`);
      if (first.length) phrase += ` among ${first.length === 1 ? first[0] : first.length === 2 ? first.join(" and ") : `${first.slice(0, -1).join(", ")}, and ${first.at(-1)}`}`;
    }
    parts.push(phrase);
  }
  return `${CHRONICLE.also}: ${parts.join("; ")}.`;
}

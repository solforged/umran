import type { Annal, Overview } from "./model";
import { YEARS } from "./model";
import { CHRONICLE, countWord, EVENT_KIND, peopleLabel, stateLabel } from "./lore";
import { individualAnnals } from "./history";

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
  /// The last year of a run of quiet years read as one, if it is a run.
  through?: number;
  headlines: Annal[];
  quiet: Annal[];
};
export const ERA_OPENERS: ReadonlySet<Annal["kind"]> = new Set(["rose", "fell", "conquest", "faith", "classical"]);
export const QUIET_KINDS: ReadonlySet<Annal["kind"]> = new Set(["neighbours", "spread", "displaced", "parted", "contact", "climate", "river-flow", "meaning", "respelling", "temper", "law", "grammar", "pronoun-renewed", "pronoun-borrowed", "class-merged", "harmony-gained", "harmony-lost", "tone-gained", "tone-lost", "coinage", "calque", "purist-replacement", "tenet-adopted", "drought", "hard-winter", "flood", "sea-route-opened"]);

/// A fleet put to sea by learning Seafaring is the craft's follow-on, not
/// a second headline for the same people in the same year.
export function isQuiet(annal: Annal, annals: readonly Annal[]): boolean {
  if (QUIET_KINDS.has(annal.kind)) return true;
  if (annal.kind !== "fleet-built" || annal.cause?.mechanism !== "craft") return false;
  const craft = annals.find((entry) => entry.id === `world:${annal.cause!.event}`);
  return craft?.kind === "craft" && craft.generation === annal.generation &&
    craft.crafts.includes("seafaring") && craft.peoples[0] === annal.peoples[0];
}

/// The entry a caption tells as the world's latest: the latest year's last
/// headline, or its last entry when that year held only bookkeeping.
export function latestTold(annals: readonly Annal[]): Annal | undefined {
  const latest = annals.at(-1);
  if (!latest || !isQuiet(latest, annals)) return latest;
  return annals.findLast((annal) => annal.generation === latest.generation && !isQuiet(annal, annals)) ?? latest;
}

/// Quiet kinds `quietLine` describes in its own words; the rest are named
/// by their chronicle heading.
const DESCRIBED: ReadonlySet<Annal["kind"]> = new Set(["neighbours", "spread", "displaced", "parted", "contact", "climate", "river-flow", "temper", "law", "grammar", "meaning", "respelling"]);
/// An opener within this many generations of an era's start stays a
/// headline inside it; two hundred years is the shortest era.
export const MIN_ERA = 8;
/// Changes to a language's grammar, each a quiet or headline entry.
const GRAMMAR_KINDS: Partial<Record<Annal["kind"], true>> = {
  grammar: true, "pronoun-renewed": true, "pronoun-polite": true, "pronoun-generalised": true, "pronoun-borrowed": true,
  "class-emerged": true, "class-merged": true, "class-lost": true, "harmony-gained": true, "harmony-lost": true, "tone-gained": true, "tone-lost": true,
};
/// Entries that give a language a word it did not have.
const WORD_KINDS: Partial<Record<Annal["kind"], true>> = { coinage: true, calque: true, "purist-replacement": true, "taboo-replacement": true, "word-authored": true };

/// "A", "A and B", "A, B, and C", or "A, B, and two more" past `max`.
export function listed(names: string[], max = 3): string {
  const shown = names.length > max ? [...names.slice(0, max - 1), `${countWord(names.length - max + 1)} more`] : names;
  if (shown.length < 3) return shown.join(" and ");
  return `${shown.slice(0, -1).join(", ")}, and ${shown.at(-1)}`;
}

/// The languages that arose in an era: daughters parting, and peoples
/// founded after the first year.
export function eraBirths(era: Era, overview: Overview): number[] {
  return overview.varieties.filter((v) => v.born >= era.start && v.born <= era.end && (v.born > 0 || v.parent !== null)).map((v) => v.id);
}

/// What happened to speech in a span of the chronicle: the languages that
/// arose, how many sound changes fell and on which languages most, new
/// words, and grammar. Eras are opened by states, conquests, and faiths;
/// this line keeps the languages' own history beside them. `brief` gives
/// counts without names, for the contents. Null when speech was quiet.
export function speechLine(annals: Annal[], overview: Overview, born: number[] = [], brief = false): string | null {
  const entries = individualAnnals(annals);
  const parts: string[] = [];
  const named = (ids: number[]) => ids.map((id) => overview.varieties[id]?.name).filter((n): n is string => n !== undefined);
  if (born.length) {
    parts.push(brief || !named(born).length
      ? `${countWord(born.length)} ${born.length === 1 ? "language" : "languages"} arose`
      : `${listed(named(born))} arose`);
  }
  const laws = entries.filter((a) => a.kind === "law");
  if (laws.length) {
    const counts = new Map<number, number>();
    for (const law of laws) for (const id of law.variety === null ? law.languages : [law.variety]) counts.set(id, (counts.get(id) ?? 0) + 1);
    const most = named([...counts].sort((a, b) => b[1] - a[1] || a[0] - b[0]).map(([id]) => id));
    let phrase = `${countWord(laws.length)} ${CHRONICLE.law[laws.length === 1 ? 0 : 1]}`;
    if (!brief && most.length) phrase += most.length === 1 ? ` in ${most[0]}` : `, most in ${listed(most.slice(0, 2), 2)}`;
    parts.push(phrase);
  }
  const words = entries.filter((a) => WORD_KINDS[a.kind]).length;
  if (words) parts.push(`${countWord(words)} ${CHRONICLE.words[words === 1 ? 0 : 1]}`);
  const grammar = entries.filter((a) => GRAMMAR_KINDS[a.kind]).length;
  if (grammar) parts.push(`${countWord(grammar)} ${CHRONICLE.grammar[grammar === 1 ? 0 : 1]}`);
  if (!parts.length) return null;
  return brief ? parts.join(" · ") : `${CHRONICLE.speech}: ${parts.join("; ")}.`;
}

/** Political eras remain the calendar; this chooses this language's recorded turning point. */
export function languageEraChange(era: Pick<Era, "start" | "end">, annals: readonly Annal[], variety: number): Annal | null {
  let chosen: Annal | null = null;
  let importance = -1;
  for (const annal of individualAnnals([...annals])) {
    if (annal.variety !== variety || annal.generation < era.start || annal.generation > era.end) continue;
    let score = -1;
    switch (annal.kind) {
      case "shift": case "koine": score = 60; break;
      case "tone-gained": case "tone-lost": case "harmony-gained": case "harmony-lost": score = 50; break;
      case "class-merged": score = 40; break;
      case "pronoun-renewed": score = 30; break;
      case "grammar": {
        // The facade records grammar events structurally; never recognise them from prose.
        const grammar = (annal as Annal & { grammar?: { event: string } }).grammar;
        if (grammar?.event === "new-marker" || grammar?.event === "fusion" || grammar?.event === "pronoun-fusion") score = 30;
        if (grammar?.event === "pronoun-relic") score = 35;
        break;
      }
      case "law": score = annal.specimen.filter((word) => word.was !== null || word.wasIpa !== null).length; break;
    }
    if (score > importance) { chosen = annal; importance = score; }
  }
  return chosen;
}

/// A short heading for an opener, from the subjects it names, in their
/// current names; the full entry stays the era's title.
export function heading(annal: Annal, overview: Overview): string {
  const state = overview.states[annal.states[0] ?? -1];
  switch (annal.kind) {
    case "rose": return state ? `${stateLabel(overview, state.id)} rises` : CHRONICLE.rose;
    case "fell": return state ? `${stateLabel(overview, state.id)} falls` : CHRONICLE.fell;
    case "conquest": {
      const [ruler, ruled] = annal.peoples.map((id) => overview.communities[id] ? peopleLabel(overview, id) : undefined);
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
      const end = boundary === latest ? latest : boundary - 1;
      const title = `Years ${start * YEARS}–${end * YEARS}`;
      result.push({ opening: null, start, end, title, heading: title, years: [] });
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
    (isQuiet(annal, annals) ? year.quiet : year.headlines).push(annal);
  }
  return result;
}

/// Consecutive years with only quieter entries, read as one run: a
/// people's fortunes or a quiet stretch of the world's chronicle would
/// otherwise be a column of "This year: one quieter entry". Years `apart`
/// marks (the book's years of the author's decisions) stand alone.
export function quietRuns(years: EraYear[], apart: (year: EraYear) => boolean = () => false): EraYear[] {
  const out: EraYear[] = [];
  let open = false;
  for (const year of years) {
    const quiet = !year.headlines.length && !apart(year);
    const previous = out.at(-1);
    if (previous && open && quiet) {
      out[out.length - 1] = { ...previous, through: year.generation, quiet: [...previous.quiet, ...year.quiet] };
    } else out.push(year);
    open = quiet;
  }
  return out;
}

/// The collapsed row stays brief; the full category account lives inside.
export function quietSummary(year: EraYear): string {
  const count = year.quiet.reduce((sum, annal) => sum + (annal.members.length || 1), 0);
  const lead = year.through !== undefined ? "In these years" : year.headlines.length ? "Also this year" : "This year";
  return `${lead}: ${count} quieter ${count === 1 ? "entry" : "entries"}.`;
}

/// A folded year describes recorded activity; names remain the engine's.
/// `lead` opens the line: "Also that year", or a span's own words.
export function quietLine(year: EraYear, overview: Overview, lead: string = CHRONICLE.also): string {
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
      if (names.length) phrase += ` in ${listed(names)}`;
    }
    parts.push(phrase);
  }
  for (const [kind, entries] of byKind) {
    if (DESCRIBED.has(kind)) continue;
    const name = EVENT_KIND[kind].name;
    parts.push(`${name[0].toLowerCase()}${name.slice(1)}${entries.length > 1 ? ` (${countWord(entries.length)} times)` : ""}`);
  }
  return `${lead}: ${parts.join("; ")}.`;
}

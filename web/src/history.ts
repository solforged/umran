import type { Annal, Overview, Subject, Variety, WorldMap } from "./model";

export const HISTORY_GROUPS = ["All events", "Peoples & journeys", "Rule & cities", "Faith & ideas", "Languages & words", "Land & livelihood"] as const;
export type HistoryGroup = typeof HISTORY_GROUPS[number];
export interface HistoryView {
  query: string;
  group: HistoryGroup;
  order: "newest" | "oldest";
  sounds: "all" | "none" | number;
  limit: number;
}
export const INITIAL_HISTORY: HistoryView = { query: "", group: "All events", order: "newest", sounds: "all", limit: 100 };
const GROUP: Record<Annal["kind"], HistoryGroup> = {
  settlement: "Peoples & journeys",
  found: "Peoples & journeys", split: "Peoples & journeys", migration: "Peoples & journeys", shift: "Languages & words",
  contact: "Peoples & journeys", parted: "Peoples & journeys", neighbours: "Peoples & journeys", conquest: "Rule & cities",
  law: "Languages & words", spread: "Peoples & journeys", displaced: "Peoples & journeys", hardship: "Land & livelihood",
  climate: "Land & livelihood", "river-flow": "Land & livelihood",
  livelihood: "Land & livelihood", ended: "Peoples & journeys", rose: "Rule & cities", fell: "Rule & cities",
  standard: "Languages & words", classical: "Languages & words", vernacular: "Languages & words", craft: "Faith & ideas",
  faith: "Faith & ideas", conversion: "Faith & ideas", meaning: "Languages & words", respelling: "Languages & words",
  schism: "Faith & ideas", pilgrimage: "Faith & ideas", "holy-land": "Faith & ideas", city: "Rule & cities",
  koine: "Languages & words", temper: "Land & livelihood", grammar: "Languages & words",
  "pronoun-renewed": "Languages & words", "pronoun-polite": "Languages & words", "pronoun-borrowed": "Languages & words",
  "class-emerged": "Languages & words", "class-merged": "Languages & words", "class-lost": "Languages & words",
  "harmony-gained": "Languages & words", "harmony-lost": "Languages & words",
  "tone-gained": "Languages & words", "tone-lost": "Languages & words",
  coinage: "Languages & words", calque: "Languages & words",
  "purist-reform": "Languages & words", "purist-replacement": "Languages & words",
  "tenet-adopted": "Faith & ideas", "tenet-disputed": "Faith & ideas", "taboo-replacement": "Faith & ideas",
};
export function searchText(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase();
}

export function findAnnal(annals: Annal[], id: string): Annal | undefined {
  for (const annal of annals) {
    if (annal.id === id) return annal;
    const member = annal.members.find((a) => a.id === id);
    if (member) return member;
  }
}

/// A subject's investigation uses individual encounters, not an aggregate's
/// union of unrelated participants. This is presentation filtering of evidence.
export function individualAnnals(annals: Annal[]): Annal[] {
  return annals.flatMap((a) => a.members.length ? a.members : [a]);
}

export function concerns(annal: Annal, subject: Subject, overview: Overview, map: WorldMap): boolean {
  switch (subject.kind) {
    case "people": return annal.peoples.includes(subject.id);
    case "language": return annal.languages.includes(subject.variety);
    case "word": return annal.variety === subject.variety && annal.specimen.some((w) => w.concept === subject.concept);
    case "state": return annal.states.includes(subject.id);
    case "religion": return annal.religions.includes(subject.id);
    case "craft": return annal.crafts.includes(subject.id);
    case "land": return annal.lands.includes(subject.region);
    case "continent": return annal.lands.some((r) => map.regions[r]?.landmass === subject.landmass);
    case "river": return annal.rivers.includes(subject.id);
    case "zone": return annal.zones.includes(subject.id);
    case "law": return annal.laws.includes(subject.id);
    case "event": return annal.id === subject.id;
    case "world": case "history": return true;
  }
}

export function subjectHistory(subject: Subject, overview: Overview, map: WorldMap): Annal[] {
  return individualAnnals(overview.annals).filter((a) => concerns(a, subject, overview, map));
}

export function relatedMoments(event: Annal, annals: Annal[]): { annal: Annal; evidence: string }[] {
  const overlap = <T,>(a: T[], b: T[]) => a.some((v) => b.includes(v));
  // A grouped year's union cannot establish that two of its subjects met.
  const sources = event.members.length ? event.members : [event];
  return individualAnnals(annals).filter((a) => !sources.some((s) => s.id === a.id)).flatMap((annal) => {
    const reasons = new Set<string>();
    for (const source of sources) {
      if (overlap(source.peoples, annal.peoples)) reasons.add("same people");
      if (overlap(source.lands, annal.lands)) reasons.add("same land");
      if (overlap(source.languages, annal.languages)) reasons.add("same language");
      if (overlap(source.states, annal.states)) reasons.add("same state");
      if (overlap(source.religions, annal.religions)) reasons.add("same faith");
      if (overlap(source.rivers, annal.rivers)) reasons.add("same river");
      if (overlap(source.zones, annal.zones)) reasons.add("same weather zone");
    }
    return reasons.size ? [{ annal, evidence: [...reasons].join(" · ") }] : [];
  }).sort((a, b) => Math.abs(a.annal.generation - event.generation) - Math.abs(b.annal.generation - event.generation));
}

// The selected language's ancestors count only before each fork. An ID
// absent in this year is a valid time-travel state, not a broken lineage.
export function filterHistory(annals: Annal[], varieties: Variety[], view: HistoryView): Annal[] {
  const lineage = new Map<number, number>();
  if (typeof view.sounds === "number") {
    for (let id: number | null = view.sounds, until = Infinity; id !== null && varieties[id] && !lineage.has(id); ) {
      lineage.set(id, until);
      until = Math.min(until, varieties[id].forkedAt ?? 0);
      id = varieties[id].parent;
    }
  }
  const terms = searchText(view.query.trim()).split(/\s+/).filter(Boolean);
  return annals.filter((annal) => {
    if (view.group !== "All events" && GROUP[annal.kind] !== view.group) return false;
    if (annal.kind === "law" && view.sounds !== "all" &&
        (view.sounds === "none" || annal.variety === null || annal.generation > (lineage.get(annal.variety) ?? -1))) return false;
    const text = searchText(`${annal.text} ${annal.notes.join(" ")}`);
    return terms.every((term) => text.includes(term));
  }).sort((a, b) => view.order === "newest" ? b.generation - a.generation : a.generation - b.generation);
}

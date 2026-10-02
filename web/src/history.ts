import type { Annal, Variety } from "./model";

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
  livelihood: "Land & livelihood", ended: "Peoples & journeys", rose: "Rule & cities", fell: "Rule & cities",
  standard: "Languages & words", classical: "Languages & words", vernacular: "Languages & words", craft: "Faith & ideas",
  faith: "Faith & ideas", conversion: "Faith & ideas", meaning: "Languages & words", respelling: "Languages & words",
  schism: "Faith & ideas", pilgrimage: "Faith & ideas", "holy-land": "Faith & ideas", city: "Rule & cities",
  koine: "Languages & words", temper: "Land & livelihood", grammar: "Languages & words",
};
export function searchText(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase();
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

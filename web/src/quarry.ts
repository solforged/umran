// The author's own meaning list for the word quarry: catalog ids or glosses,
// one to a line or separated by commas, resolved the way the engine resolves
// the author's roots at founding.

import type { ConceptView } from "./model";

export interface ResolvedMeanings {
  concepts: string[];
  /// Meanings no catalog entry answers to, with the nearest ids.
  unknown: { meaning: string; suggestions: string[] }[];
}

/// Exact id, then exact gloss, then one alternative of a gloss that lists
/// several ("chief, king" answers to "king"); otherwise the nearest ids.
export function resolveMeanings(text: string, catalog: ConceptView[]): ResolvedMeanings {
  const concepts: string[] = [];
  const unknown: ResolvedMeanings["unknown"] = [];
  for (const raw of text.split(/[\n,;]+/)) {
    const meaning = raw.trim().toLowerCase();
    if (!meaning) continue;
    const exact = catalog.find((concept) => concept.id === meaning) ?? catalog.find((concept) => concept.gloss.toLowerCase() === meaning);
    const alternatives = exact ? [exact] : catalog.filter((concept) => concept.gloss.toLowerCase().split(", ").includes(meaning));
    if (alternatives.length === 1) {
      if (!concepts.includes(alternatives[0].id)) concepts.push(alternatives[0].id);
      continue;
    }
    const near = alternatives.length ? alternatives : catalog.filter((concept) => concept.id.includes(meaning) || concept.gloss.toLowerCase().includes(meaning));
    unknown.push({ meaning: raw.trim(), suggestions: near.slice(0, 6).map((concept) => concept.id) });
  }
  return { concepts, unknown };
}

// How the book words time and relations. The chronicle counts years from
// its first people, as annals count from a founding, and describes ties
// between peoples in words, keeping the figures for the apparatus.

import type { ContactKind, Overview } from "./model";
import { YEARS } from "./model";

export function ordinal(n: number): string {
  const tens = n % 100;
  const suffix = tens >= 11 && tens <= 13 ? "th" : (["th", "st", "nd", "rd"][n % 10] ?? "th");
  return `${n}${suffix}`;
}

/// "the 50th year", or the year of the founding.
export function year(generation: number): string {
  return generation === 0 ? "the year of the founding" : `the ${ordinal(generation * YEARS)} year`;
}

/// A chronicle heading for a year.
export function yearHeading(generation: number): string {
  return generation === 0 ? "In the beginning" : `In the ${ordinal(generation * YEARS)} year`;
}

/// The year in the book's own era, counted from its first people.
export function era(overview: Overview, generation: number): string {
  const founders = overview.communities[0]?.name;
  return founders ? `${year(generation)} of the ${founders}` : year(generation);
}

/// Speeds for letting the years pass, as generations per second.
export const PACES: [number, string][] = [
  [1, "slowly"],
  [4, "steadily"],
  [16, "swiftly"],
];

const BONDS: Record<ContactKind, string[]> = {
  neighbours: ["are distant neighbours of", "are neighbours of", "are near neighbours of", "live close beside"],
  trade: ["trade now and then with", "trade with", "trade much with", "live by trade with"],
  rule: ["are loosely ruled with", "are ruled together with", "are bound under one rule with", "are one realm with"],
  religion: ["share a few gods with", "share their gods with", "keep the same feasts as", "worship as one people with"],
  intermarriage: ["marry now and then into", "marry into", "marry often into", "are much intermarried with"],
};

/// How one people stands to another, in words: "trade much with".
export function bond(kind: ContactKind, intensity: number): string {
  const level = intensity < 0.25 ? 0 : intensity < 0.5 ? 1 : intensity < 0.75 ? 2 : 3;
  return BONDS[kind][level];
}

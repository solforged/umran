// Names and explanations the views share: terrain and contact names, the
// kinds of event with their icons, the colour of a family, how a people
// came by a sound change or a land by a name, and plain explanations of
// the linguist's terms.

import {
  AudioLines,
  CloudRain,
  Crown,
  Expand,
  Flag,
  Footprints,
  GitFork,
  Handshake,
  Languages,
  Sprout,
  UserRoundMinus,
  Unlink,
  Users,
  Wind,
  type LucideIcon,
} from "lucide-react";
import type { Annal, Community, ContactKind, Law, Livelihood, Overview, PlaceName, Terrain } from "./model";
import { YEARS } from "./model";

export const EVENT_KIND: Record<Annal["kind"], { icon: LucideIcon; name: string }> = {
  found: { icon: Flag, name: "A people appears" },
  split: { icon: GitFork, name: "A people parts" },
  migration: { icon: Footprints, name: "A people moves" },
  shift: { icon: Languages, name: "A people changes tongue" },
  contact: { icon: Handshake, name: "Peoples meet" },
  parted: { icon: Unlink, name: "Peoples part" },
  neighbours: { icon: Users, name: "Neighbours come and go" },
  conquest: { icon: Crown, name: "A conquest" },
  law: { icon: AudioLines, name: "A sound change" },
  spread: { icon: Expand, name: "A people spreads" },
  displaced: { icon: Wind, name: "A people is driven out" },
  hardship: { icon: CloudRain, name: "Hard times" },
  livelihood: { icon: Sprout, name: "A new way of life" },
  ended: { icon: UserRoundMinus, name: "A people ends" },
};

export const TERRAIN_NAME: Record<Terrain, string> = {
  plains: "Plains",
  forest: "Forest",
  steppe: "Steppe",
  hills: "Hills",
  mountains: "Mountains",
  desert: "Desert",
  sea: "Sea",
};

/// Shared names for how peoples feed themselves, in cards and choices.
export const LIVELIHOOD_NAME: Record<Livelihood, string> = {
  foraging: "Foragers",
  herding: "Herders",
  farming: "Farmers",
};

export const CONTACT_NAME: Record<ContactKind, string> = {
  neighbours: "Neighbours",
  trade: "Trade",
  rule: "Rule",
  religion: "Religion",
  intermarriage: "Intermarriage",
};

/// A colour for the `n`th family or root, far from its neighbours in hue.
export function hue(n: number): string {
  return `hsl(${Math.round((n * 137.508) % 360)} 55% 48%)`;
}

/// When and how people `c` came to have sound change `law`: before their
/// speech parted from its parent's, from a neighbour, or of themselves.
export function howCame(law: Law, c: Community, overview: Overview): string {
  const year = `year ${law.generation * YEARS}`;
  const variety = overview.varieties[c.variety];
  if (variety.forkedAt !== null && variety.parent !== null && law.generation <= variety.forkedAt) {
    return `${year}, before their speech parted from ${overview.varieties[variety.parent].name}`;
  }
  if (law.from !== null) {
    const source = overview.communities.find((k) => k.variety === law.from);
    return `${year}, spreading from ${source ? `the ${source.name}` : overview.varieties[law.from].name}`;
  }
  return `${year}, arising among them`;
}

/// How a land came by one of its names, given the name before it.
export function howNamed(name: PlaceName, before: PlaceName | undefined, overview: Overview): string {
  const meaning = `“${name.meaning}”`;
  switch (name.origin) {
    case "coined": {
      const by = name.by === null ? null : overview.communities[name.by]?.name;
      return `${meaning}, named in ${name.language}${by ? ` by the ${by}` : ""}`;
    }
    case "borrowed":
      return `${meaning}, ${before ? `${before.spelled} ` : ""}as ${name.language} heard it`;
    case "inherited":
      return `came down into ${name.language}`;
    case "kept":
      return `kept when its people took up ${name.language}`;
  }
}

/// The linguist's terms the encyclopedia uses, each in a sentence or two.
export const TERMS = {
  "sound law":
    "A change in how a language is pronounced that applies to every word with the right sounds, not word by word. That regularity is what lets linguists reconstruct older forms.",
  isogloss:
    "A line on a map marking where a feature of speech stops: one side says it one way, the other side another.",
  wave: "A sound change passing from one community to its neighbours, rather than being inherited. Waves are why dialects share changes their ancestor never had.",
  cognate: "Words in related languages that come down from the same word in their common ancestor.",
  "language shift":
    "A community giving up its own language for another, usually a more prestigious one. Traces of the old language often survive in the new one.",
  substrate:
    "Traces left in a language by the one its speakers spoke before a shift: an accent, a few words, sound habits.",
  exonym: "What outsiders call a people or place, as opposed to what they call themselves.",
  family: "Languages that descend from one ancestor language.",
  "way of life":
    "How a people gets its food: gathering and hunting, keeping herds, or farming. Farming feeds many more people on each land, helping farmers’ languages spread over those of foragers.",
} as const;

export type Term = keyof typeof TERMS;

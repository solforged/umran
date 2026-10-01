// Names and explanations the views share: terrain and contact names, the
// kinds of event with their icons, the colour of a family, how a people
// came by a sound change or a land by a name, and plain explanations of
// the linguist's terms.

import {
  AudioLines,
  CircleX,
  CloudRain,
  Crown,
  Expand,
  Flag,
  Footprints,
  GitFork,
  Handshake,
  Languages,
  Landmark,
  Sprout,
  BookOpen,
  Hammer,
  Sparkles,
  UserRoundMinus,
  Unlink,
  Users,
  Wind,
  WholeWord,
  type LucideIcon,
} from "lucide-react";
import type { Annal, Community, ContactKind, Law, Livelihood, Overview, PlaceName, ReligionView, Rendering, StateView, Terrain } from "./model";
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
  rose: { icon: Landmark, name: "A state rises" },
  fell: { icon: CircleX, name: "A state falls" },
  standard: { icon: Languages, name: "A standard language" },
  craft: { icon: Hammer, name: "A new craft" },
  faith: { icon: Sparkles, name: "A faith is founded" },
  conversion: { icon: Sparkles, name: "A people changes faith" },
  meaning: { icon: WholeWord, name: "A meaning changes" },
  respelling: { icon: BookOpen, name: "A spelling changes" },
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

export const RISE_NAME: Record<StateView["rise"], string> = {
  conquest: "through conquest",
  proclaimed: "by proclamation",
  "hard-times": "in answer to hard times",
  crowded: "after being crowded off their land",
  neighbour: "in answer to a stronger neighbouring state",
  comfort: "in a time of comfort",
};

export const FALL_NAME: Record<NonNullable<StateView["fall"]>, string> = {
  "rulers-ended": "its ruling people ended",
  "capital-lost": "it lost its capital",
  conquered: "it was conquered",
  collapsed: "it collapsed",
};

export const FAITH_HOW: Record<ReligionView["how"], string> = {
  troubles: "in a time of troubles",
  quiet: "in a quiet time",
  proclaimed: "proclaimed",
};

/// How a language found a word for a craft or a faith.
export function renderingOrigin(rendering: Rendering): string {
  switch (rendering.how) {
    case "borrowed":
      return rendering.from === null ? "borrowed" : `borrowed from ${rendering.from}`;
    case "kept":
      return rendering.from === null ? "kept" : `kept from ${rendering.from}`;
    case "stretched":
      return rendering.from === null ? "meaning extended" : `stretched from “${rendering.from}”`;
    case "built":
      return rendering.from === null ? "built from another word" : `built on ${rendering.from}`;
    case "coined":
      return "new word";
    case "inherited":
      return "inherited";
  }
}

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
  state:
    "A realm organized around a ruling people and a capital city, fed by tribute from the peoples it rules. Over time, its court’s speech can become a standard language.",
  "standard language":
    "A form of speech treated as a shared model, here the language of a state’s court. Kindred dialects tend to take up its sounds and words, while the standard itself changes more slowly.",
  "dialect levelling":
    "Kindred dialects under one standard lose their differences as speakers take up the standard’s sounds and words, as English and French dialects did.",
  purism:
    "Guarding a language against foreign words, favouring words made from its own resources instead. Here a purist court makes its standard less open to borrowing.",
  "own words":
    "Meanings expressed with a native word used for that meaning alone. Loans come from other languages; shared words are also the main word for another meaning.",
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
  "sacred language":
    "A language kept for a religion’s teaching and worship. Here its words and sounds stay as they were at the founding, even as the followers’ speech changes.",
  pejoration:
    "A word’s meaning becoming worse. A new faith may use an older word for a god to mean a demon.",
  "learned word":
    "A word taken back from a sacred or classical language. English fragile came from Latin beside frail, which had already descended from the same Latin word.",
  doublet:
    "Two words in one language that came from the same older word by different routes. English frail and fragile are a doublet: one inherited, the other learned from Latin.",
  "given name":
    "A personal name used to call someone, such as Wulfstan. Here names come from the language’s words and change with its sounds.",
  "dithematic name":
    "A given name made of two meaningful parts, like Wulf-stan, “wolf-stone”. The parts may later wear down until their meanings are hard to hear.",
  "spelling vs pronunciation":
    "Writing can keep an older form after speech changes. English knight still writes sounds that are no longer said; respelling brings writing closer to speech again.",
  "meaning extension by livelihood":
    "A familiar word taking on a meaning shaped by how people live. Herders may count wealth in cattle: Latin pecunia, “money”, comes from pecus, “cattle”.",
} as const;

export type Term = keyof typeof TERMS;

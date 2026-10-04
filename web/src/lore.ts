// Names and explanations the views share: terrain and contact names, the
// kinds of event with their icons, the colour of a family, how a people
// came by a sound change or a land by a name, and plain explanations of
// the linguist's terms.

import {
  AudioLines,
  Blend,
  Castle,
  CircleX,
  CloudRain,
  CloudSunRain,
  Crown,
  Expand,
  Flag,
  Flame,
  Footprints,
  GitFork,
  Handshake,
  Languages,
  MapPinned,
  Route,
  Split,
  Landmark,
  Sprout,
  BookOpen,
  Hammer,
  PenLine,
  ScrollText,
  Sparkles,
  UserRoundMinus,
  Unlink,
  Users,
  Wind,
  Waves,
  WholeWord,
  type LucideIcon,
} from "lucide-react";
import type { Annal, ClimateView, Community, ContactKind, Ethos, EthosAxis, Law, Livelihood, Mechanism, Overview, PlaceName, ReligionView, Rendering, SchismCause, StateView, StressRule, Terrain } from "./model";
import { YEARS } from "./model";

export const EVENT_KIND: Record<Annal["kind"], { icon: LucideIcon; name: string }> = {
  settlement: { icon: MapPinned, name: "A choice of homeland" },
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
  climate: { icon: CloudSunRain, name: "The weather turns" },
  "river-flow": { icon: Waves, name: "A river rises or fails" },
  livelihood: { icon: Sprout, name: "A new way of life" },
  ended: { icon: UserRoundMinus, name: "A people ends" },
  rose: { icon: Landmark, name: "A state rises" },
  fell: { icon: CircleX, name: "A state falls" },
  standard: { icon: Languages, name: "A standard language" },
  classical: { icon: ScrollText, name: "A classical language" },
  vernacular: { icon: PenLine, name: "Speech is written" },
  craft: { icon: Hammer, name: "A new craft" },
  faith: { icon: Sparkles, name: "A faith is founded" },
  conversion: { icon: Sparkles, name: "A people changes faith" },
  meaning: { icon: WholeWord, name: "A meaning changes" },
  respelling: { icon: BookOpen, name: "A spelling changes" },
  schism: { icon: Split, name: "A faith divides" },
  pilgrimage: { icon: Route, name: "Pilgrims cross the sea" },
  "holy-land": { icon: MapPinned, name: "A holy land changes hands" },
  city: { icon: Castle, name: "A great city grows" },
  koine: { icon: Blend, name: "A city's speech forms" },
  temper: { icon: Flame, name: "A people's temper turns" },
  grammar: { icon: Languages, name: "Grammar changes" },
  "pronoun-renewed": { icon: Languages, name: "A pronoun is renewed" },
  "pronoun-polite": { icon: Languages, name: "Polite address at court" },
  "pronoun-borrowed": { icon: Languages, name: "A pronoun is borrowed" },
  "class-emerged": { icon: Languages, name: "Noun classes form" },
  "class-merged": { icon: Languages, name: "Two noun classes merge" },
  "class-lost": { icon: Languages, name: "Noun classes are lost" },
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

/// Why a faith divided, completing "It broke from its parent …".
export const SCHISM_CAUSE: Record<SchismCause, string> = {
  distance: "when its faithful lived too far from the first to keep one teaching",
  rule: "when a realm would answer to no church beyond its own",
  reform: "when the faithful no longer understood its sacred speech",
  succession: "in a quarrel over who should follow the founder",
};

/// Where a language puts the stress of a word.
export const STRESS_RULE: Record<StressRule, string> = {
  initial: "on the first syllable",
  penult: "on the next to last syllable",
  final: "on the last syllable",
  weight: "on the next to last syllable if it is heavy, else the one before",
  free: "where each word has it",
};

/// A people's six leanings, in the order a card lists them.
export const ETHOS_AXES: EthosAxis[] = ["martial", "open", "pious", "hierarchical", "roving", "seaward"];

/// Each leaning's two ends, as a word for the people: [low, high].
export const ETHOS_POLES: Record<EthosAxis, [string, string]> = {
  martial: ["peaceable", "warlike"],
  open: ["insular", "welcoming"],
  pious: ["worldly", "devout"],
  hierarchical: ["egalitarian", "hierarchical"],
  roving: ["rooted", "restless"],
  seaward: ["landbound", "seagoing"],
};

/// Below this a leaning goes unremarked; below `STRONG` it is "somewhat".
const MILD = 0.2;
/// Where the engine records a people's temper turning.
export const STRONG = 0.5;

/// A people's temper in a few words, strongest leaning first.
export function temperament(ethos: Ethos, most = 3): string[] {
  return ETHOS_AXES.filter((axis) => Math.abs(ethos[axis]) >= MILD)
    .sort((a, b) => Math.abs(ethos[b]) - Math.abs(ethos[a]))
    .slice(0, most)
    .map((axis) => `${Math.abs(ethos[axis]) < STRONG ? "somewhat " : ""}${ETHOS_POLES[axis][ethos[axis] > 0 ? 1 : 0]}`);
}

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
/// OKLCH keeps every hue at the same perceived lightness (`--hue-light`),
/// so green and yellow names read as well as red ones, by day or lamplight.
export function hue(n: number): string {
  return `oklch(var(--hue-light) 0.13 ${Math.round((n * 137.508) % 360)})`;
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

/// The engine's departure from usual weather, without implying casualties.
export function weatherDeparture(zone: ClimateView["zones"][number]): string {
  const changes = [
    zone.wetness < -0.04 ? "drier" : zone.wetness > 0.04 ? "wetter" : "",
    zone.warmth < -0.04 ? "colder" : zone.warmth > 0.04 ? "warmer" : "",
  ].filter(Boolean);
  if (!changes.length) return "near usual";
  const degree = zone.severity >= 3 ? "much " : zone.severity === 0 ? "slightly " : "";
  return `${degree}${changes.join(" and ")} than usual`;
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
  temper:
    "A people’s settled bent: how warlike, welcoming, devout, hierarchical, restless, and seagoing it is. Peoples inherit it, take it on from those they live among, and change it in answer to what befalls them, as Toynbee held that hard challenges harden a people and long comfort softens it. It shapes how readily they fight, trade, borrow, convert, organize, wander, and take to the sea.",
  "sacred language":
    "A language kept for a religion’s teaching and worship. Here its words and sounds stay as they were at the founding, even as the followers’ speech changes.",
  pejoration:
    "A word’s meaning becoming worse. A new faith may use an older word for a god to mean a demon.",
  "learned word":
    "A word taken back from a sacred or classical language. English fragile came from Latin beside frail, which had already descended from the same Latin word.",
  schism:
    "A faith dividing into branches that each claim the true teaching, as Christianity did between Rome and Constantinople, and again at the Reformation.",
  pilgrimage:
    "A journey to a holy place. Pilgrims meet speakers of other tongues on the road and at the shrine, and carry words home.",
  stress:
    "The extra force one syllable of a word gets. Where it falls shapes sound change: unstressed vowels weaken and drop, as Latin calidus became Italian caldo.",
  geminate:
    "A long, or doubled, consonant, as the tt in Italian fatto “done”. Many arise when one consonant assimilates to the next, as Latin factum became fatto.",
  koiné:
    "The new speech that forms where speakers of related dialects or languages crowd together, keeping what most of them share and dropping what few do. Hellenistic Greek and early London English formed this way.",
  diglossia:
    "Two forms of one language for different purposes: a fixed, written high form for law, worship, and learning, and the everyday speech everyone grows up with. Latin beside the early Romance languages and Classical Arabic beside the spoken dialects are examples.",
  "classical language":
    "A standard fixed as it stood, by grammarians or by the fall of its state, and written long after its speakers’ everyday speech has moved on. It keeps lending learned words.",
  vernacular:
    "The everyday speech of a people, as against a classical or sacred language. Writing the vernacular, as Dante did Italian, ends diglossia.",
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


export const MECHANISM_NAME: Record<Mechanism, string> = {
  hardship: "after hard times",
  crowding: "pressed for land",
  "stronger-neighbour": "under a stronger neighbour",
  climate: "as the weather turned",
  craft: "with a craft learned",
  conquest: "after a conquest",
  city: "as the city grew",
  court: "at court",
  contact: "after a meeting",
  pilgrimage: "along a pilgrim road",
  "unfaithful-holder": "against an unfaithful holder",
};

export const WORD_ORDER_PHRASE = {
  SOV: { choice: "the verb last", prose: "The verb comes last", summary: "Verb last" },
  SVO: { choice: "the verb between", prose: "The verb comes between", summary: "Verb between" },
  VSO: { choice: "the verb first", prose: "The verb comes first", summary: "Verb first" },
} as const;

export const MARKING_PHRASE = {
  case: { choice: "mark the thing acted on", prose: "the thing acted on is marked on the noun", summary: "object marked on the noun" },
  order: { choice: "let the order say who did what", prose: "word order says who did what", summary: "word order marks the object" },
} as const;

export const POSSESSOR_PHRASE = {
  before: { choice: "the child's fish", prose: "the possessor comes before", summary: "possessor before" },
  after: { choice: "the fish of the child", prose: "the possessor comes after", summary: "possessor after" },
} as const;

/// The chronicle's quiet apparatus shares its nouns with cards and the book.
export const CHRONICLE = {
  first: "The first peoples",
  rose: "A state rises",
  fell: "A state falls",
  conquest: "A conquest",
  faith: "A faith is founded",
  classical: "A classical language",
  also: "Also that year",
  weather: "the weather turned",
  neighbours: ["pair of neighbours", "pairs of neighbours"],
  lands: ["land", "lands"],
  contact: ["meeting", "meetings"],
  parted: ["parting", "partings"],
  rivers: ["river changed", "rivers changed"],
  temper: ["temper turned", "tempers turned"],
  law: ["sound change", "sound changes"],
  grammar: ["grammar change", "grammar changes"],
  meaning: ["meaning changed", "meanings changed"],
  respelling: ["spelling changed", "spellings changed"],
} as const;

export function countWord(count: number): string {
  return ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"][count] ?? count.toLocaleString();
}

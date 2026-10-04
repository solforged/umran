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
import type { Annal, ClimateView, Community, ContactKind, Ethos, EthosAxis, Law, Livelihood, Mechanism, Overview, PlaceExonym, PlaceName, ReligionView, Rendering, SchismCause, Seasons, StateView, StressRule, Tenet, Terrain, WorldMap } from "./model";
import { YEARS } from "./model";
import { findAnnal } from "./history";

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
  drought: { icon: CloudSunRain, name: "The rains fail" },
  "hard-winter": { icon: CloudSunRain, name: "A hard winter" },
  flood: { icon: Waves, name: "A river floods its fields" },
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
  "pronoun-generalised": { icon: Languages, name: "Polite address becomes ordinary" },
  "pronoun-borrowed": { icon: Languages, name: "A pronoun is borrowed" },
  "class-emerged": { icon: Languages, name: "Noun classes form" },
  "class-merged": { icon: Languages, name: "Two noun classes merge" },
  "class-lost": { icon: Languages, name: "Noun classes are lost" },
  "harmony-gained": { icon: Languages, name: "Vowels come to agree" },
  "harmony-lost": { icon: Languages, name: "Vowel harmony fades" },
  "tone-gained": { icon: Languages, name: "Words take on tone" },
  "tone-lost": { icon: Languages, name: "Tone is lost" },
  coinage: { icon: Languages, name: "A word is made" },
  calque: { icon: Languages, name: "A word is translated" },
  "purist-reform": { icon: Languages, name: "Borrowed words are struck out" },
  "purist-replacement": { icon: Languages, name: "A native word is written" },
  "tenet-adopted": { icon: Sparkles, name: "A faith holds a teaching" },
  "tenet-disputed": { icon: Split, name: "A faith parts over a teaching" },
  "taboo-replacement": { icon: Sparkles, name: "A word is forbidden" },
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
  religion: "Religious contact",
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
  weight: "on a heavy next-to-last syllable, otherwise the syllable before it; on the first syllable in a two-syllable word",
  free: "on a syllable specified separately for each word",
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

const SEASON_COLD = ["mild all year", "a marked cold season", "a long hard winter"] as const;
const SEASON_RAIN: Record<Seasons["wet"], string> = {
  none: "rain at any time",
  summer: "rain in summer",
  winter: "rain in winter",
  monsoon: "a monsoon",
};

export function seasonalPhrase(seasons: Seasons): string {
  const cold = SEASON_COLD[seasons.amplitude < 0.25 ? 0 : seasons.amplitude < 0.5 ? 1 : 2];
  return [cold, SEASON_RAIN[seasons.wet], ...(seasons.floods ? ["its river floods its fields"] : [])].join("; ");
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
    "Dialect differences become less marked as speakers in contact adopt shared forms. Here related dialects can converge on a court standard’s sounds and words.",
  purism:
    "Guarding a language against foreign words, favouring words made from its own resources instead. Here a purist court makes its standard less open to borrowing.",
  "own words":
    "Meanings expressed with a native word used for that meaning alone. Loans come from other languages; shared words are also the main word for another meaning.",
  "sound law":
    "A regular change in pronunciation affecting words with the same sounds in the same conditions. Here a change also respects the language’s minimum word size and never removes a word’s last vowel.",
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
    "A word borrowed through study or formal tradition, often from a classical or sacred language. French fragile was borrowed from Latin beside frêle, which French inherited from the same Latin source.",
  schism:
    "A faith dividing into branches that each claim the true teaching, as Christianity did between Rome and Constantinople, and again at the Reformation.",
  pilgrimage:
    "A journey to a holy place. Pilgrims meet speakers of other tongues on the road and at the shrine, and carry words home.",
  stress:
    "The prominence of one syllable, heard through a combination of pitch, duration, loudness, and vowel quality. Its position can affect sound change.",
  geminate:
    "A long, or doubled, consonant, as the tt in Italian fatto “done”. Many arise when one consonant assimilates to the next, as Latin factum became fatto.",
  koiné:
    "The new speech that forms where speakers of related dialects or languages crowd together, keeping what most of them share and dropping what few do. Hellenistic Greek and early London English formed this way.",
  diglossia:
    "Different varieties used for different purposes: a high variety for formal settings and an everyday variety for ordinary life. Here the high variety is preserved while everyday speech changes.",
  "classical language":
    "A language preserved for learned or literary use after everyday speech has changed. Here a court standard is fixed by grammarians or when its state falls.",
  vernacular:
    "The everyday speech of a community. It can be written while a classical or sacred language remains in formal use. Here beginning to write it marks the end of the modelled high-language writing arrangement.",
  doublet:
    "Two words in one language that ultimately come from the same source by different routes. French frêle and fragile are a doublet: one inherited, the other learned from Latin.",
  "given name":
    "A personal name used to call someone, such as Wulfstan. Here names come from the language’s words and change with its sounds.",
  "dithematic name":
    "A given name made of two meaningful parts, like Wulf-stan, “wolf-stone”. The parts may later wear down until their meanings are hard to hear.",
  "spelling vs pronunciation":
    "Writing can keep an older form after speech changes. English knight still writes sounds that are no longer said; respelling brings writing closer to speech again.",
  "meaning extension by livelihood":
    "A familiar word taking on a meaning shaped by how people live. Herders may count wealth in cattle: Latin pecunia, “money”, comes from pecus, “cattle”.",
  "minimal word": "The shortest independent word a language permits. Here sound change cannot reduce a word below that size.",
  "heavy syllable": "A syllable with two units of weight, or moras: here a long vowel or a short vowel followed by a closing consonant.",
  affix: "A meaningful piece attached to a word: a prefix before the stem or a suffix after it.",
  "root and pattern": "Words built by fitting a consonant root into a pattern of vowels and, sometimes, other sounds.",
  paradigm: "The related grammatical forms of one word, such as its singular and plural.",
  particle: "A separate word with a grammatical job, such as marking past time.",
  productive: "Available for making new forms, rather than surviving only in particular old words.",
  "grammatical contrast": "An audible difference that distinguishes a grammatical category; it may be carried by a stem change, affix, or separate word.",
  grammaticalization: "A word or construction developing a grammatical role; here an ordinary lexical word also supplies a separate marker.",
  fusion: "A separate grammatical word becoming attached to its neighbour.",
  analogy: "A form changing to follow a pattern found in other forms.",
  IPA: "International Phonetic Alphabet: symbols for speech sounds.",
} as const;

export type Term = keyof typeof TERMS;


export const MECHANISM_NAME: Record<Mechanism, string> = {
  hardship: "after hard times",
  crowding: "pressed for land",
  "stronger-neighbour": "under pressure from a stronger neighbour",
  climate: "as the weather turned",
  craft: "with a craft learned",
  conquest: "after a conquest",
  city: "as the city grew",
  court: "at court",
  "word-need": "for want of a word",
  contact: "after a meeting",
  pilgrimage: "along a pilgrim road",
  "unfaithful-holder": "against an unfaithful holder",
  "foreign-prestige": "against a prouder neighbour's tongue",
  "religious-revival": "in a revival of the faith",
  "purist-norm": "by the purists' rule",
  doctrine: "by its teaching",
};

/// A faith's teaching on each tenet, as [held, rejected].
export const TENET_TEACHING: Record<Tenet, [string, string]> = {
  "sacred-language": ["scripture read in the sacred tongue", "scripture taught in the people's speech"],
  images: ["images venerated", "images forbidden"],
  hierarchy: ["an order of priests", "no order of priests"],
  purity: ["strict rules of purity", "little care for purity"],
  pilgrimage: ["pilgrimage", "no pilgrimage"],
  monasticism: ["withdrawal from the world honoured", "withdrawal from the world frowned on"],
};

export function faithTeaching(religion: ReligionView): string | null {
  const teaching = religion.doctrine
    .filter((p) => Math.abs(p.stance) >= 0.15)
    .map((p) => TENET_TEACHING[p.tenet][p.stance > 0 ? 0 : 1]).join("; ");
  return teaching || null;
}

export const TENET_NOUN: Record<Tenet, string> = {
  "sacred-language": "the sacred tongue",
  images: "images",
  hierarchy: "the priesthood",
  purity: "purity",
  pilgrimage: "pilgrimage",
  monasticism: "withdrawal from the world",
};

export const WORD_ORDER_PHRASE = {
  SOV: { choice: "the verb last", prose: "The verb comes last", summary: "Verb last" },
  SVO: { choice: "the verb between", prose: "The verb comes between", summary: "Verb between" },
  VSO: { choice: "the verb first", prose: "The verb comes first", summary: "Verb first" },
} as const;

export const MARKING_PHRASE = {
  case: { choice: "mark the object", prose: "the object has a grammatical marker", summary: "object marked" },
  order: { choice: "let word order distinguish the roles", prose: "word order distinguishes subject and object", summary: "roles shown by word order" },
} as const;

export const POSSESSOR_PHRASE = {
  before: { choice: "the possessor first (“child — fish”)", prose: "the possessor comes before", summary: "possessor before" },
  after: { choice: "the possessor last (“fish — child”)", prose: "the possessor comes after", summary: "possessor after" },
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

export const FAITH_NAME = { action: "Found a faith", kind: "A faith", plural: "Faiths" } as const;
export const PEOPLE_NAME = "People";

export function peoplePhrases(ended: Community["ended"]) {
  return ended === null
    ? { land: "Where they live", homeland: "Their heart is", livelihood: "How they live", speech: "What they speak", lastSpeech: "" }
    : { land: "Where they lived", homeland: "Their last homeland was", livelihood: "How they lived", speech: "What they spoke", lastSpeech: "Their last recorded speech was " };
}

export function unnamedName(kind: string, prose = false): string {
  const lower = kind.toLowerCase();
  const plural = lower === "plains" || lower === "hills" || lower === "mountains";
  return prose ? `${plural ? "" : "an "}unnamed ${lower}` : `Unnamed ${lower}`;
}

export function landLabel(overview: Overview, map: WorldMap, region: number, prose = false): string {
  return overview.places.find((p) => p.region === region)?.names.at(-1)?.spelled ??
    unnamedName(TERRAIN_NAME[map.regions[region].terrain], prose);
}

/// Languages' names for a land, grouped by how they spell it, those that
/// say it as its holders do last; within a group, the earliest hearing.
export function otherNames(exonyms: PlaceExonym[], own: string | undefined) {
  const groups = new Map<string, { spelled: string; ipa: string; heard: number; once: string | null; same: boolean; varieties: number[] }>();
  for (const x of exonyms) {
    const group = groups.get(x.spelled);
    if (!group) {
      groups.set(x.spelled, { spelled: x.spelled, ipa: x.ipa, heard: x.heard, once: x.once, same: x.spelled === own, varieties: [x.variety] });
    } else {
      if (!group.varieties.includes(x.variety)) group.varieties.push(x.variety);
      if (x.heard < group.heard) Object.assign(group, { heard: x.heard, once: x.once });
    }
  }
  return [...groups.values()].sort((a, b) => Number(a.same) - Number(b.same) || a.heard - b.heard);
}

/** The recorded trigger, including grouped individual entries, not a guessed cause. */
export function causePhrase(annal: Annal, overview: Overview): { trigger: Annal; text: string } | null {
  if (!annal.cause) return null;
  const trigger = findAnnal(overview.annals, `world:${annal.cause.event}`);
  if (!trigger) return null;
  const year = trigger.generation * YEARS;
  const place = trigger.lands.length ? overview.places.find((p) => p.region === trigger.lands[0])?.names.filter((n) => n.since <= trigger.generation).at(-1)?.spelled : undefined;
  const where = place ? ` in ${place}` : "";
  let text: string;
  switch (annal.cause.mechanism) {
    case "hardship": text = `after hard times${where} in ${year}`; break;
    case "stronger-neighbour": text = `under pressure from a stronger neighbour, after ${trigger.kind === "contact" || trigger.kind === "neighbours" ? "their meeting" : trigger.kind === "rose" ? "a state rose" : "the recorded event"} in ${year}`; break;
    case "city": text = `after the city grew in ${year}`; break;
    default: text = `${MECHANISM_NAME[annal.cause.mechanism]} in ${year}`;
  }
  return { trigger, text };
}

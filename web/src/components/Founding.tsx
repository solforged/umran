import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { ArrowLeft, ArrowRight, ChevronDown, Feather, LocateFixed, Plus, RefreshCw } from "lucide-react";
import { attempt, checkRoots, concepts, createEngine, message, presetDesign, preview } from "../engine";
import type { Action, Catalog, Craft, Engine, EthosAxis, Forming, FoundingPreview, FoundingTraits, GrammarChoice, GrammarDesign, LakeNamesView, LanguageDesign, Livelihood, MapSize, Meaning, MeaningField, MeaningSet, MeaningTier, Naming, Overview, PossessorOrder, RiverNamesView, RootCheck, Variety, WordOrder, WorldMap } from "../model";
import { YEARS } from "../model";
import { countWord, ETHOS_AXES, ETHOS_DRIVES, ETHOS_POLES, hue, landCardLabel, LIVELIHOOD_DRIVES, LIVELIHOOD_NAME, MARKING_PHRASE, MEANING_CLASS, MEANING_TIER, MILD, OPENNESS, OPENNESS_DRIVES, peopleLabel, POSSESSOR_PHRASE, STANDING, STANDING_DRIVES, STRONG, TERRAIN_NAME, WORD_ORDER_PHRASE } from "../lore";
import { newBookId, readDraft, saveDraft, worldName, type DraftEntry } from "../shelf";
import { Designer, randomSeed, type Founding as FoundingDesign } from "./Designer";
import { FormingChart } from "./FormingChart";
import { MapView } from "./MapView";
import { useMapProjection } from "./MapProjectionSwitch";
import { MapInspector, type MapInspection } from "./MapInspector";
import { Modal, plainTitle } from "./Modal";
import { decodeNaming, encodeNaming, namingChoices } from "./NamingSelect";
import { Phrase } from "./Phrase";
import { Sample } from "./Sample";
import { useAnswer } from "./Pending";
import { Popover } from "./Popover";
import { Sheet, type SheetDetent } from "./Sheet";
import "./founding.css";

const MOST_PEOPLES = 12;
/// Where an author's "somewhat" and "strongly" put a leaning.
const LEANING = [0.35, 0.7];
/// The chronicle's default length, in years.
const DEFAULT_YEARS = 4000;
/// Words a voice card shows, by the preview's gloss.
const VOICE_WORDS = ["water", "fire", "sun", "night"];
const WORLD_SIZES: Record<MapSize, string> = { small: "small", medium: "middling", large: "wide", vast: "vast" };

type Step = "world" | "homeland" | "peoples" | "speech" | "title";
const STEPS: { id: Step; name: string; next: string }[] = [
  { id: "world", name: "The world", next: "Choose a homeland" },
  { id: "homeland", name: "The homeland", next: "Who lives there" },
  { id: "peoples", name: "The peoples", next: "Their speech" },
  { id: "speech", name: "Their speech", next: "The title page" },
  { id: "title", name: "The title page", next: "Write the chronicle" },
];
type Settling = "one" | "kin" | "neighbours";

interface Founder extends FoundingDesign {
  key: number;
  preset: string | null;
  /// A stable draft key, not a community index. Related members inherit this speech.
  source: number | null;
  /// Null only while explicitly redrawing the draft's geography.
  region: number | null;
  livelihood: Livelihood | null;
  /// Leanings the author set; other axes are as land and life make them.
  ethos: Partial<Record<EthosAxis, number>>;
  /// What they bring to year 0, each a dated decision in the chronicle.
  crafts: Craft[];
  realm: boolean;
  faith: boolean;
}

/// Authored constraints only. The engine derives the map, names, and speech
/// again through the ordinary founding path when this manuscript is opened.
interface DraftDesign {
  version: 1;
  seed: number;
  size: MapSize;
  founders: Founder[];
  /// Meanings the author adds to the world before any people is founded.
  meanings: Meaning[];
  step: number;
  selected: number | null;
  land: number | null;
  title: string | null;
  author: string;
  years: number | null;
  settling: Settling;
  groupCount: number;
  nextKey: number;
}

interface OpenedDraft {
  design: DraftDesign;
  id: string | null;
  notice: string | null;
}

function freshDesign(): DraftDesign {
  let author = "";
  try { author = localStorage.getItem("umran.author") ?? ""; } catch { /* A title page may still be written. */ }
  return { version: 1, seed: randomSeed(), size: "medium", founders: [], meanings: [], step: 0, selected: null, land: null,
    title: null, author, years: DEFAULT_YEARS, settling: "one", groupCount: 3, nextKey: 0 };
}

function isDraftDesign(value: unknown): value is DraftDesign {
  const record = (value: unknown): value is Record<string, unknown> =>
    value !== null && typeof value === "object" && !Array.isArray(value);
  const index = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
  const seed = (value: unknown): value is number => index(value) && value <= 0xffff_ffff;
  const finite = (value: unknown): value is number => typeof value === "number" && Number.isFinite(value);
  if (!record(value) || value.version !== 1 || !seed(value.seed)
    || !["small", "medium", "large", "vast"].includes(String(value.size))
    || !index(value.step) || value.step >= STEPS.length || !index(value.nextKey)
    || (value.selected !== null && !index(value.selected)) || (value.land !== null && !index(value.land))
    || (value.title !== null && typeof value.title !== "string") || typeof value.author !== "string"
    || (value.years !== null && !finite(value.years))
    || !["one", "kin", "neighbours"].includes(String(value.settling))
    || !index(value.groupCount) || value.groupCount < 2 || value.groupCount > 6
    || !Array.isArray(value.founders) || value.founders.length > MOST_PEOPLES) return false;
  // Older drafts have no meanings; they open with none.
  if (value.meanings === undefined) value.meanings = [];
  if (!Array.isArray(value.meanings) || !value.meanings.every((meaning: unknown) => record(meaning)
    && typeof meaning.id === "string" && typeof meaning.gloss === "string" && typeof meaning.field === "string"
    && ["entity", "event", "property"].includes(String(meaning.class)) && typeof meaning.countable === "boolean"
    && ["basic", "everyday", "specialized"].includes(String(meaning.tier))
    && (meaning.from === null || (record(meaning.from) && typeof meaning.from.base === "string" && typeof meaning.from.relation === "string")))) return false;
  const keys = new Set<number>();
  for (const founder of value.founders) {
    if (!record(founder) || !index(founder.key) || keys.has(founder.key) || founder.key >= value.nextKey
      || (founder.source !== null && (!index(founder.source) || !keys.has(founder.source)))
      || (founder.region !== null && !index(founder.region))
      || (founder.preset !== null && typeof founder.preset !== "string")
      || !seed(founder.seed) || !finite(founder.power) || !finite(founder.openness)
      || (founder.livelihood !== null && !["foraging", "herding", "farming"].includes(String(founder.livelihood)))
      || !record(founder.ethos) || !Object.entries(founder.ethos).every(([axis, leaning]) =>
        ETHOS_AXES.includes(axis as EthosAxis) && finite(leaning))
      || !Array.isArray(founder.crafts) || !founder.crafts.every((craft: unknown) =>
        ["metalworking", "riding", "seafaring", "writing"].includes(String(craft)))
      || typeof founder.realm !== "boolean" || typeof founder.faith !== "boolean"
      || !record(founder.naming) || !["people", "speakers", "place", "epithet"].includes(String(founder.naming.kind))
      || (founder.naming.kind === "place" && typeof founder.naming.place !== "string")
      || (founder.naming.kind === "epithet" && typeof founder.naming.epithet !== "string")
      || !record(founder.design) || !Array.isArray(founder.design.sounds)
      || !record(founder.design.spelling)) return false;
    keys.add(founder.key);
  }
  return (value.selected === null || keys.has(value.selected))
    && (value.step < 2 || value.founders.length > 0 || (value.step === 2 && value.land !== null));
}

function openDesign(id?: string): OpenedDraft {
  if (id) {
    try {
      const raw = readDraft(id);
      const design: unknown = raw === null ? null : JSON.parse(raw);
      if (isDraftDesign(design)) return { design, id, notice: null };
    } catch { /* Leave the original manuscript untouched. */ }
    return { design: freshDesign(), id: null, notice: "That founding draft could not be opened; a fresh founding is open, and the saved draft has not been changed." };
  }
  return { design: freshDesign(), id: null, notice: null };
}

interface Built {
  map: WorldMap;
  forming: Forming | null;
  overview: Overview;
  /// Per founder, in founding order: what land and life gave them.
  traits: FoundingTraits[];
  /// Per founder key: year-0 decisions the engine did not take, by item.
  refused: Map<number, Set<string>>;
  preview: FoundingPreview;
  /// Every river's and lake's names at year 0, for the chart.
  riverNames: RiverNamesView[];
  lakeNames: LakeNamesView[];
  founders: Founder[];
  meanings: Meaning[];
  /// The catalog's fields, for naming a meaning's.
  fields: MeaningSet[];
  seed: number;
  size: MapSize;
}

function drawNaming(catalog: Catalog, key: number): Naming {
  const choices: Naming[] = [
    { kind: "people" }, { kind: "speakers" },
    ...catalog.namePlaces.map((place) => ({ kind: "place" as const, place })),
    ...catalog.nameEpithets.map((epithet) => ({ kind: "epithet" as const, epithet })),
  ];
  return key === 0 ? { kind: "people" } : choices[Math.floor(Math.random() * choices.length)];
}

function drawFounder(catalog: Catalog, key: number, region: number): Founder {
  const preset = catalog.presets[Math.floor(Math.random() * catalog.presets.length)].id;
  const seed = randomSeed();
  return { key, preset, source: null, seed, design: presetDesign(preset, seed), naming: drawNaming(catalog, key), power: 0.5, openness: 0.5, region, livelihood: null, ethos: {}, crafts: [], realm: false, faith: false };
}

function markerChoice(speech: Variety, category: "plural" | "past" | "object"): GrammarChoice {
  const marker = speech.grammar.markers.find((m) => m.category === category && m.productive && m.retired === null);
  return !marker || marker.kind === "none" ? "none" : marker.kind === "particle" ? "particle" : marker.side;
}

/// Why a root was refused, with the engine's detail: the unreadable
/// character, the missing sound, or the earlier line.
function rootTrouble(root: RootCheck): string {
  switch (root.status) {
    case "unknown-meaning": return "no such meaning in the catalog";
    case "bad-ipa": return root.detail === "no-vowel" ? "has no vowel" : root.detail ? `cannot be read at “${root.detail}”` : "cannot be read";
    case "outside-inventory": return `uses ${root.detail ?? "a sound"} this speech does not have`;
    case "duplicate": return root.detail ? `already given on line ${root.detail}` : "given twice";
    default: return "";
  }
}
const ROOT_WARNING: Record<RootCheck["warnings"][number], string> = {
  cluster: "a cluster their shape avoids",
  length: "longer than their words run",
  final: "an ending their shape avoids",
};

/// The author's own words, one `meaning form` to a line, checked against
/// the speech's sounds as they are typed. Accepted roots go into the design;
/// the rest of the vocabulary is drawn around them.
function RootsBox({ founder, meanings, update }: { founder: Founder; meanings: Meaning[]; update: (key: number, patch: Partial<Founder>) => void }) {
  const [text, setText] = useState(() => (founder.design.roots ?? []).map((root) => `${root.concept} ${root.ipa}`).join("\n"));
  const [open, setOpen] = useState(() => (founder.design.roots?.length ?? 0) > 0);
  const bare = JSON.stringify({ ...founder.design, roots: [] });
  const report = useMemo(() => text.trim() ? checkRoots(JSON.parse(bare) as LanguageDesign, text, meanings) : null, [bare, text, meanings]);
  const accepted = report?.design.roots ?? [];
  useEffect(() => {
    if (JSON.stringify(accepted) !== JSON.stringify(founder.design.roots ?? [])) {
      const { roots: _, ...design } = founder.design;
      update(founder.key, { design: accepted.length ? { ...design, roots: accepted } : design });
    }
  }, [accepted, founder, update]);
  const trouble = report?.roots.filter((root) => root.status !== "ok" && root.status !== "warned") ?? [];
  const replaceMeaning = (root: RootCheck, concept: string) => {
    const lines = text.split("\n");
    lines[root.line - 1] = `${concept} ${root.ipa}`;
    setText(lines.join("\n"));
  };
  return <section className="founding-roots">
    <h4>Their own words</h4>
    {open ? <>
      <p>Give any meanings their words, one to a line, in the catalog’s sounds: <code>water nara</code>, <code>king kaːr</code>. The rest of the vocabulary is drawn around them, and sound change wears them like any word.</p>
      <textarea className="founding-roots-text" aria-label="The author’s roots" value={text} rows={Math.min(12, Math.max(3, text.split("\n").length + 1))}
        spellCheck={false} autoCapitalize="off" autoCorrect="off" onChange={(event) => setText(event.target.value)} />
      {report ? <>
        <p className={trouble.length ? "notice" : "muted small"}>
          {accepted.length} {accepted.length === 1 ? "root" : "roots"} given{trouble.length ? ` · ${trouble.length} ${trouble.length === 1 ? "needs" : "need"} attention` : ""}
        </p>
        <ul className="founding-roots-check">{report.roots.map((root) => <li key={root.line} className={root.status === "ok" ? "ok" : root.status === "warned" ? "warned" : "trouble"}>
          <span className="founding-roots-meaning">{root.gloss ?? root.meaning}</span>
          {root.spelled !== null ? <span><i className="word">{root.spelled}</i> <span className="muted">/{root.ipa}/</span></span> : <span className="muted">/{root.ipa}/</span>}
          {root.status === "warned" ? <span className="muted small">{root.warnings.map((warning) => ROOT_WARNING[warning]).join(", ")}</span> : null}
          {root.status !== "ok" && root.status !== "warned" ? <span className="small">{rootTrouble(root)}
            {root.suggestions.length ? <> · try {root.suggestions.map((concept, index) => <span key={concept}>{index ? ", " : ""}<button type="button" className="link" onClick={() => replaceMeaning(root, concept)}>{concept}</button></span>)}</> : null}</span> : null}
        </li>)}</ul>
      </> : null}
    </> : <p className="founding-roots-closed">The words are drawn from their sounds. <button type="button" className="link" onClick={() => setOpen(true)}>Give them words of your own…</button></p>}
  </section>;
}

const MEANING_SHAPES = ["count", "mass", "event", "property"] as const;
type MeaningShape = typeof MEANING_SHAPES[number];

/// Meanings the catalog lacks, added to the world before its peoples are
/// founded, so every founding draws a word for them and the author's roots
/// may name them. The engine checks each on the next build.
function MeaningsBox({ meanings, fields, onChange }: { meanings: Meaning[]; fields: MeaningSet[]; onChange: (meanings: Meaning[]) => void }) {
  const [open, setOpen] = useState(meanings.length > 0);
  const [gloss, setGloss] = useState("");
  const [shape, setShape] = useState<MeaningShape>("count");
  const [field, setField] = useState<MeaningField>("house");
  const [tier, setTier] = useState<MeaningTier>("everyday");
  const id = gloss.toLowerCase().normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/^to /, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 32);
  const taken = id ? concepts().some((concept) => concept.id === id) || meanings.some((meaning) => meaning.id === id) : false;
  const add = () => {
    if (!id || taken) return;
    onChange([...meanings, {
      id, gloss: gloss.trim(), field, class: shape === "event" ? "event" : shape === "property" ? "property" : "entity",
      countable: shape === "count", tier, from: null,
    }]);
    setGloss("");
  };
  return <section className="founding-roots founding-meanings">
    <h4>Meanings of your own</h4>
    {open ? <>
      <p>Add what the catalog lacks: <i>hearth</i>, <i>to forge</i>, <i>sacred</i>. Every people founded here draws a word for it, and roots above may name it by its short name.</p>
      {meanings.length ? <ul className="founding-roots-check">{meanings.map((meaning) => <li key={meaning.id} className="ok">
        <span className="founding-roots-meaning">{meaning.gloss}</span>
        <span className="muted small"><code>{meaning.id}</code> · {MEANING_CLASS[meaning.class === "event" ? "event" : meaning.class === "property" ? "property" : meaning.countable ? "count" : "mass"].replace(/ \(.*\)$/, "").toLowerCase()} · {fields.find((set) => set.id === meaning.field)?.label ?? meaning.field}</span>
        <button type="button" className="link" onClick={() => onChange(meanings.filter((other) => other.id !== meaning.id))}>Remove</button>
      </li>)}</ul> : null}
      <div className="founding-meaning-form">
        <input type="text" aria-label="Meaning" value={gloss} maxLength={40} placeholder="hearth" onChange={(event) => setGloss(event.target.value)}
          onKeyDown={(event) => { if (event.key === "Enter") { event.preventDefault(); add(); } }} />
        <select aria-label="Kind" value={shape} onChange={(event) => setShape(event.target.value as MeaningShape)}>
          {MEANING_SHAPES.map((kind) => <option key={kind} value={kind}>{MEANING_CLASS[kind]}</option>)}
        </select>
        <select aria-label="Field" value={field} onChange={(event) => setField(event.target.value as MeaningField)}>
          {fields.map((set) => <option key={set.id} value={set.id}>{set.label}</option>)}
        </select>
        <select aria-label="How bound to a way of life" value={tier} onChange={(event) => setTier(event.target.value as MeaningTier)}>
          {(["basic", "everyday", "specialized"] as const).map((t) => <option key={t} value={t}>{MEANING_TIER[t].name}</option>)}
        </select>
        <button type="button" disabled={!id || taken} onClick={add}><Plus size={14} aria-hidden="true" /> Add</button>
      </div>
      {taken ? <p className="notice">“{id}” is a meaning already; the catalog or your list has it.</p> : <p className="muted small">{MEANING_TIER[tier].drives}</p>}
    </> : <p className="founding-roots-closed">The catalog’s {concepts().length} meanings are theirs. <button type="button" className="link" onClick={() => setOpen(true)}>Add meanings of your own…</button></p>}
  </section>;
}

/// Draft on the real engine, one step at a time: the coasts, a homeland,
/// the peoples, their speech, and the title page. Writing the chronicle
/// hands that same world to the workshop with the years to run.
export function Founding({ catalog, draft, onBegin, onChartRoom, onDraft }: {
  catalog: Catalog;
  draft?: string;
  onBegin: (engine: Engine, title: string | null, author: string | null, years: number, id: string) => Promise<void>;
  onChartRoom: () => void;
  onDraft?: (id: string) => void;
}) {
  const [opening] = useState(() => openDesign(draft));
  const [step, setStep] = useState<Step>(STEPS[opening.design.step].id);
  const [detent, setDetent] = useState<SheetDetent>("half");
  const [worldSeed, setWorldSeed] = useState(opening.design.seed);
  const [size, setSize] = useState<MapSize>(opening.design.size);
  const [founders, setFounders] = useState<Founder[]>(opening.design.founders);
  const [meanings, setMeanings] = useState<Meaning[]>(opening.design.meanings);
  const [selected, setSelected] = useState<number | null>(opening.design.selected);
  const [inspected, setInspected] = useState<number | null>(opening.design.land);
  const mapContainer = useRef<HTMLElement>(null);
  const [inspection, setInspection] = useState<MapInspection | null>(null);
  const [focus, setFocus] = useState<number | null>(opening.design.land);
  const [title, setTitle] = useState<string | null>(opening.design.title);
  const [author, setAuthor] = useState(opening.design.author);
  const [years, setYears] = useState(opening.design.years ?? NaN);
  const [projection, setProjection] = useMapProjection(mapContainer);
  const [adjusting, setAdjusting] = useState<number | null>(null);
  const [redraw, setRedraw] = useState<{ seed: number; size: MapSize } | null>(null);
  const [clearing, setClearing] = useState(false);
  /// The founder whose homeland the next land click sets.
  const [moving, setMoving] = useState<number | null>(null);
  /// Whether the next land click founds another people there.
  const [adding, setAdding] = useState(false);
  const [moveNotice, setMoveNotice] = useState<string | null>(null);
  const [settling, setSettling] = useState<Settling>(opening.design.settling);
  const [groupCount, setGroupCount] = useState(opening.design.groupCount);
  /// The world whose coasts have finished forming on the chart.
  const [formed, setFormed] = useState<string | null>(() =>
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ? "always" : null);
  const [built, setBuilt] = useState<Built | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [draftNotice, setDraftNotice] = useState(opening.notice);
  const [saveError, setSaveError] = useState<string | null>(null);
  const engine = useRef<Engine | null>(null);
  const base = useRef<{ seed: number; size: MapSize; engine: Engine } | null>(null);
  const published = useRef<Built | null>(null);
  const handedOver = useRef(false);
  const grammarDraws = useRef(new Map<number, { order: WordOrder; object: GrammarChoice; possessor: PossessorOrder }>());
  const nextKey = useRef(opening.design.nextKey);
  const pages = useRef<HTMLDivElement>(null);
  const roster = useRef<HTMLElement>(null);
  const draftId = useRef(opening.id);
  const restoring = useRef(opening.id !== null);
  const requestedDraft = useRef(draft);
  const baseline = useRef(JSON.stringify(opening.design));
  const saved = useRef<string | null>(null);
  const saving = useRef<number | undefined>(undefined);
  const draftChanged = useRef(onDraft);
  draftChanged.current = onDraft;
  const design = useMemo<DraftDesign>(() => ({
    version: 1, seed: worldSeed, size, founders, meanings, step: STEPS.findIndex((entry) => entry.id === step),
    selected, land: inspected, title, author, years: Number.isFinite(years) ? years : null,
    settling, groupCount, nextKey: nextKey.current,
  }), [worldSeed, size, founders, meanings, step, selected, inspected, title, author, years, settling, groupCount]);
  const rawDesign = useMemo(() => JSON.stringify(design), [design]);
  const manuscript = useRef({ design, raw: rawDesign, built });
  manuscript.current = { design, raw: rawDesign, built };

  const keepDraft = useCallback((): boolean => {
    if (handedOver.current || restoring.current) return true;
    const { design, raw, built } = manuscript.current;
    if (draftId.current === null && raw === baseline.current) return true;
    if (draftId.current === null) {
      draftId.current = newBookId();
      draftChanged.current?.(draftId.current);
    }
    const current = built?.seed === design.seed && built.size === design.size && built.founders === design.founders ? built : null;
    const peoples = current?.overview.communities.map((people) => ({
      name: people.name, family: current.overview.varieties.find((variety) => variety.id === people.variety)!.family,
      region: people.region, lands: people.lands,
    }));
    const signature = `${raw}\n${JSON.stringify(peoples)}`;
    if (signature === saved.current) return true;
    const worldSize = WORLD_SIZES[design.size];
    const peopleCount = design.founders.length;
    const subject = peopleCount
      ? `${countWord(peopleCount).replace(/^./, (letter) => letter.toUpperCase())} ${peopleCount === 1 ? "people" : "peoples"} on a ${worldSize} world`
      : `A ${worldSize} world`;
    const entry: DraftEntry = {
      id: draftId.current, title: design.title, summary: `${subject} · ${STEPS[design.step].name.toLowerCase()}`,
      step: design.step, updated: Date.now(), seed: design.seed, size: design.size,
      ...(peoples?.length ? { peoples } : {}),
    };
    try {
      saveDraft(entry, raw);
      saved.current = signature;
      setSaveError(null);
      return true;
    } catch (failure) {
      setSaveError(`This founding could not be kept in this browser: ${message(failure)}.`);
      return false;
    }
  }, []);
  const installDesign = useCallback(({ design, id, notice }: OpenedDraft) => {
    window.clearTimeout(saving.current);
    restoring.current = id !== null;
    handedOver.current = false;
    draftId.current = id;
    saved.current = null;
    baseline.current = JSON.stringify(design);
    manuscript.current = { design, raw: baseline.current, built: null };
    nextKey.current = design.nextKey;
    grammarDraws.current.clear();
    engine.current?.dispose();
    engine.current = null;
    base.current?.engine.dispose();
    base.current = null;
    published.current = null;
    setBuilt(null);
    setStep(STEPS[design.step].id);
    setWorldSeed(design.seed);
    setSize(design.size);
    setFounders(design.founders);
    setSelected(design.selected);
    setInspected(design.land);
    setFocus(design.land);
    setTitle(design.title);
    setAuthor(design.author);
    setYears(design.years ?? NaN);
    setSettling(design.settling);
    setGroupCount(design.groupCount);
    setMoving(null);
    setAdding(false);
    setAdjusting(null);
    setRedraw(null);
    setClearing(false);
    setInspection(null);
    setMoveNotice(null);
    setError(null);
    setSaveError(null);
    setDraftNotice(notice);
  }, []);

  useEffect(() => {
    if (requestedDraft.current === draft) return;
    requestedDraft.current = draft;
    // The first change replaces /found with this very draft, not a new one.
    if ((draft ?? null) === draftId.current) return;
    keepDraft();
    installDesign(openDesign(draft));
  }, [draft, keepDraft, installDesign]);

  useEffect(() => {
    if (manuscript.current.raw !== rawDesign) return;
    if (restoring.current || handedOver.current || (draftId.current === null && rawDesign === baseline.current)) return;
    if (draftId.current === null) {
      draftId.current = newBookId();
      draftChanged.current?.(draftId.current);
    }
    saving.current = window.setTimeout(keepDraft, 500);
    return () => {
      window.clearTimeout(saving.current);
      saving.current = undefined;
    };
  }, [rawDesign, built, keepDraft]);

  useEffect(() => {
    const hidden = () => { if (document.visibilityState === "hidden") keepDraft(); };
    window.addEventListener("pagehide", keepDraft);
    document.addEventListener("visibilitychange", hidden);
    return () => {
      window.removeEventListener("pagehide", keepDraft);
      document.removeEventListener("visibilitychange", hidden);
      keepDraft();
    };
  }, [keepDraft]);

  useEffect(() => {
    if (pages.current) pages.current.scrollTop = 0;
    roster.current?.querySelector('[aria-pressed="true"]')?.scrollIntoView({ block: "nearest" });
  }, [selected, step]);

  useEffect(() => {
    if (moving === null && !adding) return;
    const cancel = (event: KeyboardEvent) => { if (event.key === "Escape") { setMoving(null); setAdding(false); setMoveNotice(null); } };
    document.addEventListener("keydown", cancel);
    return () => document.removeEventListener("keydown", cancel);
  }, [moving, adding]);

  useEffect(() => {
    const current = manuscript.current.design;
    if (current.seed !== worldSeed || current.size !== size || current.founders !== founders || current.meanings !== meanings) return;
    // Resolving new homelands publishes their draft in the same render. It
    // must not immediately rebuild the identical world a second time.
    const previous = published.current;
    if (previous?.founders === founders && previous.meanings === meanings && previous.seed === worldSeed && previous.size === size) return;
    let live = true;
    const isCurrent = () => {
      const design = manuscript.current.design;
      return live && design.seed === worldSeed && design.size === size && design.founders === founders && design.meanings === meanings;
    };
    const failed = (failure: unknown) => {
      if (!isCurrent()) return;
      if (!restoring.current) { setError(message(failure)); return; }
      // A refused replay is not permission to replace the saved design.
      installDesign({
        design: freshDesign(), id: null,
        notice: "That founding draft could not be rebuilt; a fresh founding is open, and the saved draft has not been changed.",
      });
    };
    const build = async () => {
      if (base.current?.seed !== worldSeed || base.current.size !== size) {
        const fresh = await createEngine(worldSeed, size);
        if (!isCurrent()) { fresh.dispose(); return; }
        base.current?.engine.dispose();
        base.current = { seed: worldSeed, size, engine: fresh };
      }
      if (!isCurrent()) return;
      const origin = base.current.engine;
      const next = await origin.foundingDraft();
      if (!isCurrent()) { next.dispose(); return; }
      try {
        // Meanings come first so every founding can draw a root for them.
        for (const meaning of meanings) await next.act({ kind: "meaning", meaning });
        const indices = new Map<number, number>();
        for (const [index, founder] of founders.entries()) {
          const options = {
            ...(founder.livelihood === null ? {} : { livelihood: founder.livelihood }),
            ...(Object.keys(founder.ethos).length ? { ethos: founder.ethos } : {}),
          };
          if (founder.source === null) {
            await next.act({
              kind: "found", naming: founder.naming, design: founder.design, seed: founder.seed,
              power: founder.power, openness: founder.openness,
              ...(founder.region === null ? {} : { region: founder.region }), ...options,
            });
          } else {
            const source = indices.get(founder.source);
            if (source === undefined) throw new Error("Their ancestral people must be founded first.");
            const region = founder.region ?? (await next.foundingSites((await next.tip().overview(0)).communities[source].region, 1))[0];
            if (region === undefined) throw new Error("There is no nearby homeland for these related peoples. Try another coast or a larger world.");
            await next.act({ kind: "found-related", source, region, naming: founder.naming, ...options });
          }
          indices.set(founder.key, index);
        }
        // Year-0 decisions follow every founding, so community indices hold.
        // The engine may refuse one or quietly decline it (riding needs open
        // country); the overview says what was taken.
        const refused = new Map<number, Set<string>>();
        for (const [community, founder] of founders.entries()) {
          const declined = new Set<string>();
          const settle = async (item: string, action: Action) => {
            try { await next.act(action); } catch { declined.add(item); }
          };
          if (founder.realm) await settle("realm", { kind: "state", community });
          for (const craft of founder.crafts) await settle(craft, { kind: "craft", community, craft });
          if (founder.faith) await settle("faith", { kind: "religion", community });
          if (declined.size) refused.set(founder.key, declined);
        }
        const year = next.tip();
        const sameWorld = previous && previous.seed === worldSeed && previous.size === size ? previous : null;
        const [overview, traits, reach, riverNames, lakeNames, drawn, forming, fields] = await Promise.all([
          year.overview(0), Promise.all(founders.map((_, index) => next.foundingTraits(index))), next.foundingPreview(),
          year.rivers(0), year.lakes(0),
          sameWorld ? sameWorld.map : next.map(), sameWorld ? sameWorld.forming : origin.forming(),
          year.meaningSets(),
        ]);
        if (!isCurrent()) { next.dispose(); return; }
        if (restoring.current && inspected !== null && (!drawn.regions[inspected] || drawn.regions[inspected].terrain === "sea")) {
          throw new Error("The draft's selected homeland could not be found.");
        }
        founders.forEach((founder, index) => {
          const held = overview.communities[index];
          const declined = refused.get(founder.key) ?? new Set<string>();
          for (const craft of founder.crafts) if (!held.crafts.includes(craft)) declined.add(craft);
          if (founder.realm && !overview.states.some((state) => state.rulers === held.id)) declined.add("realm");
          if (founder.faith && held.faith === null) declined.add("faith");
          if (declined.size) refused.set(founder.key, declined);
        });
        const resolved = founders.map((founder, index) => founder.region === null
          ? { ...founder, region: overview.communities[index].region } : founder);
        const settled = resolved.some((founder, index) => founder !== founders[index]) ? resolved : founders;
        founders.forEach((founder, index) => {
          if (founder.source !== null) return;
          const speech = overview.varieties.find((variety) => variety.id === overview.communities[index].variety)!;
          const drawn = grammarDraws.current.get(founder.key);
          grammarDraws.current.set(founder.key, {
            order: !drawn || founder.design.grammar?.order == null ? speech.grammar.order : drawn.order,
            object: !drawn || founder.design.grammar?.object == null ? markerChoice(speech, "object") : drawn.object,
            possessor: !drawn || founder.design.grammar?.possessor == null ? speech.grammar.possessor : drawn.possessor,
          });
        });
        const founded: Built = {
          map: drawn, forming, overview, traits, refused, preview: reach, riverNames, lakeNames, founders: settled, meanings, fields, seed: worldSeed, size,
        };
        engine.current?.dispose();
        engine.current = next;
        published.current = founded;
        restoring.current = false;
        setBuilt(founded);
        if (settled !== founders) setFounders(settled);
        setError(null);
      } catch (failure) {
        next.dispose();
        failed(failure);
      }
    };
    void build().catch(failed);
    return () => { live = false; };
  }, [worldSeed, size, founders, meanings, installDesign]);

  useEffect(() => () => {
    base.current?.engine.dispose();
    base.current = null;
    if (!handedOver.current) engine.current?.dispose();
  }, []);

  const overview = built?.overview;
  const map = built?.map;
  const worldKey = `${worldSeed}:${size}`;
  const currentBuild = !!built && built.founders === founders && built.seed === worldSeed && built.size === size;
  const selectedIndex = founders.findIndex((founder) => founder.key === selected);
  const current = founders[selectedIndex];
  const people = overview?.communities[selectedIndex];
  const traits: FoundingTraits | undefined = built?.traits[selectedIndex];
  const speech = people ? overview?.varieties.find((variety) => variety.id === people.variety) : undefined;
  const speechOwner = current?.source == null ? current : founders.find((founder) => founder.key === current.source);
  const inherited = !!current && current.source !== null;
  const drawn = speechOwner ? grammarDraws.current.get(speechOwner.key) : undefined;
  const land = inspected === null ? undefined : map?.regions[inspected];
  const occupied = !!land && !!overview?.communities.some((community) => community.lands.includes(land.id));
  const defaultTitle = overview?.communities.length ? worldName(overview) : null;
  const chartReady = !!map && !!overview && built?.seed === worldSeed && built.size === size;
  const forming = step === "world" && chartReady && built.forming && formed !== worldKey && formed !== "always" ? built.forming : null;
  const riverNames = built?.riverNames ?? [];
  const lakeNames = built?.lakeNames ?? [];
  const nameOf = (id: number) => overview ? peopleLabel(overview, id) : "Settling…";
  const landName = (region: number) => {
    const named = overview?.places.find((place) => place.region === region)?.names.at(-1)?.spelled;
    return named && overview && map ? landCardLabel(overview, map, region, named) : named ?? "unnamed land";
  };
  const landTitle = (region: number) => {
    const named = landName(region);
    const r = map?.regions[region];
    return named !== "unnamed land" || !r ? named : `${TERRAIN_NAME[r.terrain]}${r.coastal ? " by the sea" : " inland"}`;
  };
  const available = MOST_PEOPLES - founders.length;
  const count = Math.min(groupCount, available);
  const sites = useAnswer(currentBuild && inspected !== null && count >= 2 && !occupied && engine.current
    ? attempt(engine.current.foundingSites(inspected, count)) : null);
  const group = { sites: sites?.value ?? [], error: sites?.error ?? "" };

  const update = (key: number, patch: Partial<Founder>) => setFounders((all) => all.map((founder) => founder.key === key ? { ...founder, ...patch } : founder));
  const movingFounder = founders.find((founder) => founder.key === moving);
  const movingName = movingFounder && overview?.communities[founders.indexOf(movingFounder)]
    ? nameOf(founders.indexOf(movingFounder)) : undefined;
  const homeTo = (region: number, except?: Founder) => overview?.communities
    .filter((community, i) => founders[i] !== except && community.lands.includes(region)) ?? [];
  const settle = (region: number) => {
    if ((!movingFounder && !adding) || !map || !overview) return;
    const others = homeTo(region, movingFounder);
    if (others.length) {
      setMoveNotice(`${landName(region)} is home to ${others.map((community) => `the ${nameOf(community.id)}`).join(" and ")}. Choose another land.`);
      return;
    }
    if (adding) {
      const founder = drawFounder(catalog, nextKey.current++, region);
      setFounders((all) => [...all, founder]);
      setSelected(founder.key);
    } else if (movingFounder && region !== movingFounder.region) update(movingFounder.key, { region });
    setMoving(null);
    setAdding(false);
    setMoveNotice(null);
    setInspected(region);
  };
  const choose = (key: number) => {
    const founder = founders.find((candidate) => candidate.key === key);
    if (!founder) return;
    setSelected(key);
    setInspected(founder.region);
    setFocus(founder.region);
  };
  const changeWorld = (next: { seed: number; size: MapSize }) => {
    setWorldSeed(next.seed);
    setSize(next.size);
    setInspected(null);
    setFocus(null);
    setFounders((all) => all.map((founder) => ({ ...founder, region: null })));
    setRedraw(null);
  };
  const requestWorld = (next: { seed: number; size: MapSize }) => {
    if (next.seed === worldSeed && next.size === size) return;
    if (founders.length) setRedraw(next);
    else changeWorld(next);
  };
  const found = () => {
    if (!land || !currentBuild || occupied) return;
    if (settling === "one") {
      if (available < 1) return;
      const founder = drawFounder(catalog, nextKey.current++, land.id);
      setFounders((all) => [...all, founder]);
      setSelected(founder.key);
      return;
    }
    if (count < 2 || group.sites.length !== count) return;
    const first = drawFounder(catalog, nextKey.current++, group.sites[0]);
    const members = group.sites.slice(1).map((region) => {
      const key = nextKey.current++;
      return settling === "kin"
        ? { ...first, key, source: first.key, region, naming: drawNaming(catalog, key) }
        : drawFounder(catalog, key, region);
    });
    setFounders((all) => [...all, first, ...members]);
    setSelected(first.key);
  };
  const addHere = () => {
    if (!land || !currentBuild || available < 1 || occupied) return;
    const founder = drawFounder(catalog, nextKey.current++, land.id);
    setFounders((all) => [...all, founder]);
    setSelected(founder.key);
  };
  const remove = () => {
    if (!current) return;
    const firstChild = founders.find((founder) => founder.source === current.key);
    const remaining = founders.filter((founder) => founder.key !== current.key).map((founder) => {
      if (!firstChild || founder.source !== current.key) return founder;
      // Keep a family's speech when its first draft member is removed.
      return founder === firstChild
        ? { ...current, key: founder.key, source: null, naming: founder.naming, region: founder.region, livelihood: founder.livelihood, ethos: founder.ethos, crafts: founder.crafts, realm: founder.realm, faith: founder.faith }
        : { ...founder, source: firstChild.key };
    });
    grammarDraws.current.delete(current.key);
    setFounders(remaining);
    setSelected(remaining[Math.min(selectedIndex, remaining.length - 1)]?.key ?? null);
  };
  const setGrammar = (patch: Partial<GrammarDesign>) => {
    if (!speechOwner || !speech) return;
    const grammar = speechOwner.design.grammar ?? { plural: markerChoice(speech, "plural"), past: markerChoice(speech, "past") };
    update(speechOwner.key, { design: { ...speechOwner.design, grammar: { ...grammar, ...patch } } });
  };
  /// A preset's sounds, keeping the grammar and roots the author chose.
  const redesign = (owner: Founder, design: LanguageDesign): LanguageDesign =>
    ({ ...design, ...(owner.design.grammar ? { grammar: owner.design.grammar } : {}), ...(owner.design.roots?.length ? { roots: owner.design.roots } : {}) });
  const choosePreset = (owner: Founder, key: string) => {
    if (key === owner.preset) return;
    update(owner.key, { preset: key, design: redesign(owner, presetDesign(key, owner.seed)) });
  };
  const adjustingFounder = founders.find((founder) => founder.key === adjusting);

  // Every voice card is the engine's own preview of this people's words
  // under that preset, with the seed founding would use; the chosen voice
  // also carries the author's own roots.
  const roots = speechOwner?.design.roots;
  const voices = useMemo(() => {
    if (!currentBuild || step !== "speech" || !speechOwner) return [];
    return catalog.presets.map((preset) => {
      const design = presetDesign(preset.id, speechOwner.seed);
      const result = preview(preset.id === speechOwner.preset && roots?.length ? { ...design, roots } : design, speechOwner.seed, speechOwner.naming);
      const words = typeof result === "string" ? [] : VOICE_WORDS
        .map((gloss) => result.words.find((word) => word.gloss.split(",")[0] === gloss))
        .filter((word) => word !== undefined);
      return { preset, words };
    });
  }, [currentBuild, step, catalog, speechOwner?.key, speechOwner?.seed, speechOwner?.naming, speechOwner?.preset, roots]);

  const stepIndex = STEPS.findIndex((s) => s.id === step);
  const reachable = (id: Step) => id === "world" || (chartReady && (id === "homeland" || (id === "peoples" ? (inspected !== null || founders.length > 0) : founders.length > 0)));
  const canBegin = !!engine.current && currentBuild && chartReady && founders.length > 0 && Number.isFinite(years) && years >= YEARS;
  const go = (next: Step) => {
    if (!reachable(next)) return;
    setMoving(null); setAdding(false); setMoveNotice(null);
    if ((next === "peoples" || next === "speech") && selected === null && founders[0]) choose(founders[0].key);
    setStep(next);
  };
  const begin = () => {
    if (!engine.current || !canBegin || handedOver.current) return;
    if (!keepDraft()) return;
    const id = draftId.current ?? newBookId();
    handedOver.current = true;
    window.clearTimeout(saving.current);
    onBegin(engine.current, title?.trim() && title.trim() !== defaultTitle ? title.trim() : null, author.trim() || null, years, id)
      .catch((failure: unknown) => { handedOver.current = false; setError(message(failure)); keepDraft(); });
  };
  const formedNow = useCallback(() => setFormed(worldKey), [worldKey]);

  const rosterNav = founders.length ? <nav className="founding-roster" aria-label="Peoples at the beginning" ref={roster}>
    {founders.map((founder, index) => {
      const entry = overview?.communities[index];
      const variety = entry ? overview?.varieties.find((candidate) => candidate.id === entry.variety) : undefined;
      return <button type="button" key={founder.key} aria-pressed={selected === founder.key} onClick={() => choose(founder.key)}
        style={{ "--tone": variety ? hue(variety.family) : undefined } as CSSProperties}>
        <span className="founding-roster-number">{String(index + 1).padStart(2, "0")}</span>
        <span className="founding-roster-name">{entry ? nameOf(entry.id) : "Settling…"}</span>
        <span className="founding-roster-place">{step === "speech" ? (variety?.name ?? "") : entry ? landName(entry.region) : ""}</span>
      </button>;
    })}
  </nav> : null;

  const head = (heading: string, lead?: ReactNode) => <header className="founding-head">
    <div className="eyebrow">{stepIndex + 1} · {STEPS[stepIndex].name}</div>
    <h2 className={plainTitle(heading) ? "plain-title" : undefined}>{heading}</h2>
    {lead ? <p>{lead}</p> : null}
  </header>;

  const hint = moving !== null || adding
    ? <>{moveNotice ?? (adding ? "Choose a land for another people." : `Choose a new homeland for the ${movingName ?? "people"}.`)} <button type="button" className="link" onClick={() => { setMoving(null); setAdding(false); setMoveNotice(null); }}>Cancel</button></>
    : step === "world" ? null
    : step === "homeland" ? "Select a land to read it."
    : step === "peoples" ? (founders.length ? "Choose a name to read its people." : null)
    : null;

  let plate: ReactNode = null;
  if (step === "world") {
    plate = <>
      {head(built?.forming ? "Coasts drawn from moving plates" : "The coasts",
        built?.forming ? "One land broke apart over ten ages of drifting plates. Its pieces became these coasts, ranges, and islands." : "Coasts, ranges, and rivers, drawn for this world.")}
      <fieldset className="founding-sizes">
        <legend>How large a world</legend>
        {catalog.mapSizes.map((choice) => <label key={choice.id} className={size === choice.id ? "chosen" : undefined}>
          <input type="radio" name="founding-size" checked={size === choice.id} onChange={() => requestWorld({ seed: worldSeed, size: choice.id as MapSize })} />
          <span><b>{choice.name}</b> <small>{choice.description}</small></span>
        </label>)}
      </fieldset>
      <div className="founding-world-actions">
        <button type="button" onClick={() => requestWorld({ seed: randomSeed(), size })}><RefreshCw size={13} aria-hidden="true" /> Draw other coasts</button>
        {built?.forming && !forming ? <button type="button" className="link" onClick={() => setFormed(null)}>Watch them form</button> : null}
      </div>
      <p className="founding-seed">Seed <code>{worldSeed}</code>{map ? ` · ${map.radiusKm.toLocaleString()} km radius · ${map.regions.length.toLocaleString()} lands and seas` : ""}</p>
      {chartReady && !forming ? <p className="founding-invitation">Select a land on the chart to begin there.</p> : null}
    </>;
  } else if (step === "homeland") {
    plate = <>
      {head("Where does their story begin?", "Rivers, coasts, and the way through neighbouring lands will shape the people who live there.")}
      {land ? <section className="founding-land" aria-label="Selected homeland">
        <div className="eyebrow">{land.island ? "On an island" : "On the mainland"}</div>
        <h3>{landTitle(land.id)}</h3>
        <p>{landName(land.id) !== "unnamed land" ? `${TERRAIN_NAME[land.terrain]}, ${land.coastal ? "coastal" : "inland"}. ` : ""}
          {Math.round(land.areaKm2).toLocaleString()} km²{map?.rivers.some((river) => river.course.includes(land.id)) ? " · a river runs through it" : ""}.</p>
        {occupied ? <p className="muted">Home to {homeTo(land.id).map((community) => `the ${nameOf(community.id)}`).join(", ")}.</p> : null}
        {founders.length ? <div className="founding-land-actions">
          <button type="button" disabled={!currentBuild || available < 1 || occupied} onClick={addHere}><Plus size={13} aria-hidden="true" /> Found another people here</button>
          {current && people && current.region !== land.id && !occupied ? <button type="button" className="link founding-move" disabled={!currentBuild}
            onClick={() => update(current.key, { region: land.id })}><LocateFixed size={13} aria-hidden="true" /> Move the {nameOf(people.id)} here</button> : null}
        </div> : null}
      </section> : <p className="founding-invitation">Select a land on the chart.</p>}
    </>;
  } else if (step === "peoples") {
    plate = founders.length ? <>
      {head(founders.length === 1 ? "One people at the beginning" : `${countWord(founders.length).replace(/^./, (c) => c.toUpperCase())} peoples at the beginning`)}
      {rosterNav}
      <div className="founding-pages" ref={pages}>
        {current && people && speech && traits ? <article className="founding-portrait" key={current.key} aria-label={`The ${nameOf(people.id)}`}>
          <header><h3>{nameOf(people.id)}</h3><span>{people.meaning}</span>
            <button type="button" className="link founding-move-start" aria-pressed={moving === current.key} disabled={!currentBuild}
              onClick={() => { setAdding(false); setMoving(moving === current.key ? null : current.key); setMoveNotice(null); }}><LocateFixed size={13} aria-hidden="true" /> Move</button></header>
          <p>They live in <button type="button" className="link" onClick={() => { setInspected(people.region); setFocus(people.region); }}>{landName(people.region)}</button>.
            {" "}They name themselves <Phrase label="Name" value={encodeNaming(current.naming)} choices={namingChoices(catalog)}
            onChange={(key) => { const naming = decodeNaming(key); if (naming) update(current.key, { naming }); }} />, <b>{people.name}</b>,
            and speak <button type="button" className="link" onClick={() => go("speech")}><i>{speech.name}</i></button>{inherited ? ", a branch of their kin's speech" : ""}.</p>

          <section className="founding-trait" aria-labelledby="founding-livelihood">
            <h4 id="founding-livelihood">How they live</h4>
            <div className="founding-choices" role="radiogroup" aria-labelledby="founding-livelihood">
              {([null, "farming", "herding", "foraging"] as (Livelihood | null)[]).map((way) => <button type="button" role="radio" key={way ?? "land"}
                aria-checked={current.livelihood === way} onClick={() => update(current.key, { livelihood: way })}>
                <b>{way === null ? "As the land suggests" : LIVELIHOOD_NAME[way]}</b>
                <small>{way === null ? LIVELIHOOD_NAME[traits.suggested].toLowerCase() : `the land feeds ${(Math.round(traits.feeds[way] / 100) * 100).toLocaleString()}`}</small>
              </button>)}
            </div>
            <p className="founding-drive">{LIVELIHOOD_DRIVES[people.livelihood]}</p>
          </section>

          <section className="founding-trait" aria-labelledby="founding-temper">
            <h4 id="founding-temper">What they are like <button type="button" className="link" disabled={!Object.keys(current.ethos).length}
              onClick={() => update(current.key, { ethos: {} })}>as land and life made them</button></h4>
            <p className="founding-leaning-legend">Diamond: drawn from the land; ring: the author’s choice. Click the chosen stop again to return to the drawn value.</p>
            <ul className="founding-leanings">
              {ETHOS_AXES.map((axis) => {
                const [low, high] = ETHOS_POLES[axis];
                const authored = current.ethos[axis];
                const lean = (value: number | undefined) => {
                  const { [axis]: _, ...rest } = current.ethos;
                  update(current.key, { ethos: value === undefined ? rest : { ...rest, [axis]: value } });
                };
                return <li key={axis}>
                  <span className={people.ethos[axis] <= -MILD ? "pole low leans" : "pole low"}>{low}</span>
                  <span className="founding-leaning" role="radiogroup" aria-label={`${low} or ${high}`}>
                    <button type="button" role="radio" className="drawn" aria-checked={authored === undefined}
                      style={{ left: `${(traits.drawn[axis] + 1) * 50}%` }} title="As land and life made them"
                      aria-label="As land and life made them"
                      onClick={() => lean(undefined)} />
                    {[-LEANING[1], -LEANING[0], LEANING[0], LEANING[1]].map((value) => <button type="button" role="radio" key={value}
                      aria-checked={authored === value} style={{ left: `${(value + 1) * 50}%` }}
                      aria-label={`${Math.abs(value) < STRONG ? "somewhat" : "strongly"} ${value < 0 ? low : high}`}
                      title={`${Math.abs(value) < STRONG ? "somewhat" : "strongly"} ${value < 0 ? low : high}`}
                      onClick={() => lean(authored === value ? undefined : value)} />)}
                  </span>
                  <span className={people.ethos[axis] >= MILD ? "pole leans" : "pole"}>{high}</span>
                </li>;
              })}
            </ul>
            <ul className="founding-drives">
              {ETHOS_AXES.filter((axis) => Math.abs(people.ethos[axis]) >= MILD)
                .sort((a, b) => Math.abs(people.ethos[b]) - Math.abs(people.ethos[a]))
                .map((axis) => <li key={axis}><b>{ETHOS_POLES[axis][people.ethos[axis] > 0 ? 1 : 0]}.</b> {ETHOS_DRIVES[axis][people.ethos[axis] > 0 ? 1 : 0]}</li>)}
            </ul>
          </section>

          <section className="founding-trait" aria-labelledby="founding-standing">
            <h4 id="founding-standing">Among other peoples</h4>
            {inherited && speechOwner ? <p className="founding-drive">Their standing and openness are their kin's: <button type="button" className="link"
              onClick={() => choose(speechOwner.key)}>{nameOf(founders.indexOf(speechOwner))}</button>.</p> : null}
            {([["Standing", STANDING, "power", STANDING_DRIVES], ["Foreign words", OPENNESS, "openness", OPENNESS_DRIVES]] as const).map(([label, stops, field, drives]) => {
              const owner = speechOwner ?? current;
              const nearest = stops.reduce((best, stop) => Math.abs(stop.value - owner[field]) < Math.abs(best.value - owner[field]) ? stop : best);
              return <div className="founding-stops" key={field}>
                <span>{label}</span>
                <div role="radiogroup" aria-label={label}>
                  {stops.map((stop) => <button type="button" role="radio" key={stop.value} aria-checked={stop === nearest} disabled={inherited}
                    onClick={() => update(owner.key, field === "power" ? { power: stop.value } : { openness: stop.value })}>{stop.name}</button>)}
                </div>
                <small>{drives}</small>
              </div>;
            })}
          </section>

          <section className="founding-trait" aria-labelledby="founding-bring">
            <h4 id="founding-bring">What they begin with</h4>
            <div className="founding-brings">
              {[
                ...catalog.crafts.map((craft) => ({ id: craft.id, name: craft.name, description: craft.description, on: current.crafts.includes(craft.id as Craft),
                  toggle: () => update(current.key, { crafts: current.crafts.includes(craft.id as Craft) ? current.crafts.filter((c) => c !== craft.id) : [...current.crafts, craft.id as Craft] }) })),
                { id: "realm", name: "A realm", description: `A state of their own, its seat in ${landName(people.region)}.`, on: current.realm, toggle: () => update(current.key, { realm: !current.realm }) },
                { id: "faith", name: "A faith", description: "A faith of their own, with a name and a teaching.", on: current.faith, toggle: () => update(current.key, { faith: !current.faith }) },
              ].map((item) => {
                const declined = item.on && !!built?.refused.get(current.key)?.has(item.id);
                return <button type="button" key={item.id} aria-pressed={item.on} className={declined ? "declined" : undefined}
                  title={item.description} onClick={item.toggle}>
                  <b>{item.name}</b><small>{declined ? "Not possible for them here" : item.description}</small>
                </button>;
              })}
            </div>
            <p className="founding-drive">Each is a decision of the author's, dated year 0 in the chronicle.</p>
          </section>

          {speech.names.length ? <section className="founding-trait" aria-labelledby="founding-given">
            <h4 id="founding-given">What they call their children</h4>
            <ul className="founding-given">{speech.names.slice(0, 8).map((given) => <li key={given.name} title={`/${given.ipa}/`}>
              <i>{given.name}</i> <small>{given.meaning}</small></li>)}</ul>
            <p className="founding-drive">Drawn from their words, as their way of life and temper favour.</p>
          </section> : null}

          <button type="button" className="link founding-remove" onClick={remove}>Leave this people out</button>
        </article> : null}
      </div>
      <div className="founding-land-actions founding-more">
        <button type="button" aria-pressed={adding} disabled={!currentBuild || available < 1}
          onClick={() => { setMoving(null); setAdding(!adding); setMoveNotice(null); }}><Plus size={13} aria-hidden="true" /> Another people…</button>
        <button type="button" className="link" onClick={() => setClearing(true)}>Start again</button>
      </div>
    </> : <>
      {head(land ? `Who lives in ${landTitle(land.id)}?` : "Who lives there?")}
      {land ? <>
        <fieldset className="founding-settling">
          <legend>At the beginning</legend>
          <label className={settling === "one" ? "chosen" : undefined}>
            <input type="radio" name="founding-settling" checked={settling === "one"} onChange={() => setSettling("one")} />
            <span><b>One people</b> Alone in their land. Others may come of them as the years pass.</span>
          </label>
          <label className={settling === "kin" ? "chosen" : undefined}>
            <input type="radio" name="founding-settling" checked={settling === "kin"} onChange={() => setSettling("kin")} />
            <span><b>Kin who parted</b> Peoples in neighbouring lands, speaking branches of one ancestral tongue.</span>
          </label>
          <label className={settling === "neighbours" ? "chosen" : undefined}>
            <input type="radio" name="founding-settling" checked={settling === "neighbours"} onChange={() => setSettling("neighbours")} />
            <span><b>Neighbours</b> Peoples in neighbouring lands, each with a language of its own.</span>
          </label>
        </fieldset>
        {settling !== "one" ? <p className="founding-count"><label>How many <select aria-label="Number of founding peoples" value={count} onChange={(event) => setGroupCount(Number(event.target.value))}>
          {Array.from({ length: Math.max(0, Math.min(6, available) - 1) }, (_, i) => i + 2).map((n) => <option key={n} value={n}>{countWord(n)}</option>)}
        </select></label>
          <span className={group.error || group.sites.length < count ? "notice" : "muted"}>
            {group.error || (group.sites.length < count ? `Only ${group.sites.length} free homelands are within reach. Choose fewer, or another land.` : `${countWord(count).replace(/^./, (c) => c.toUpperCase())} homelands within reach over land, with room between them.`)}
          </span></p> : null}
        <button type="button" className="primary founding-settle" disabled={!currentBuild || occupied || (settling !== "one" && group.sites.length !== count)} onClick={found}>
          {settling === "one" ? "Settle them here" : "Settle them"}</button>
        {occupied ? <p className="notice">{landTitle(land.id)} is already home to a people. Choose another land.</p> : null}
      </> : <p className="founding-invitation">First <button type="button" className="link" onClick={() => go("homeland")}>choose a homeland</button>.</p>}
    </>;
  } else if (step === "speech") {
    plate = <>
      {head(people ? `How do the ${nameOf(people.id)} speak?` : "How do they speak?",
        "Each voice is a set of sound preferences. Every card shows the words this people would have.")}
      {founders.length > 1 ? rosterNav : null}
      <div className="founding-pages" ref={pages}>
        {current && people && speech && speechOwner ? <>
          {inherited ? <p className="founding-inheritance">The {nameOf(people.id)} speak a branch of the speech of the{" "}
            <button type="button" className="link" onClick={() => choose(speechOwner.key)}>{nameOf(founders.indexOf(speechOwner))}</button>.
            {" "}Their kin share these choices.</p> : null}
          <div className="founding-voices" role="radiogroup" aria-label="Voices">
            {speechOwner.preset === null ? <button type="button" role="radio" aria-checked className="founding-voice">
              <b>Shaped by hand</b>
              <span className="founding-voice-words">{speech.specimen.slice(0, 4).map((w) => <span key={w.concept}><i>{w.spelled}</i><small>{w.concept}</small></span>)}</span>
            </button> : null}
            {voices.map(({ preset, words }) => <button type="button" role="radio" key={preset.id} aria-checked={speechOwner.preset === preset.id}
              className="founding-voice" title={preset.description} onClick={() => choosePreset(speechOwner, preset.id)}>
              <b>{preset.name}</b>
              <span className="founding-voice-words">{words.map((w) => <span key={w.gloss}><i>{w.spelled}</i><small>{w.gloss.split(",")[0]}</small></span>)}</span>
            </button>)}
          </div>
          <div className="founding-speech-actions">
            <button type="button" className="link" title="Draw a new vocabulary and names" onClick={() => { const seed = randomSeed(); update(speechOwner.key, { seed, design: speechOwner.preset === null ? speechOwner.design : redesign(speechOwner, presetDesign(speechOwner.preset, seed)) }); }}>Draw other words</button>
            <button type="button" className="link" onClick={() => setAdjusting(speechOwner.key)}>Shape the sounds by hand…</button>
          </div>
          <section className="founding-grammar">
            <h4>Grammar</h4>
            <p>They put <Phrase label="Word order" value={speechOwner.design.grammar?.order ?? ""}
              choices={[{ key: "", text: `${WORD_ORDER_PHRASE[drawn?.order ?? speech.grammar.order].choice}, as their speech falls out` },
                ...(["SOV", "SVO", "VSO"] as WordOrder[]).map((order) => ({ key: order, text: WORD_ORDER_PHRASE[order].choice }))]}
              onChange={(key) => setGrammar({ order: key === "" ? null : key as WordOrder })} />, <Phrase label="Object marking"
              value={speechOwner.design.grammar?.object == null ? "" : speechOwner.design.grammar.object === "none" ? "order" : "case"}
              choices={[{ key: "", text: `${MARKING_PHRASE[(drawn?.object ?? markerChoice(speech, "object")) === "none" ? "order" : "case"].choice}, as their speech falls out` },
                ...(["case", "order"] as const).map((marking) => ({ key: marking, text: MARKING_PHRASE[marking].choice }))]}
              onChange={(key) => {
                const object = drawn?.object ?? markerChoice(speech, "object");
                setGrammar({ object: key === "" ? null : key === "order" ? "none" : object !== "none" ? object : speechOwner.design.suffixing >= 0.5 ? "suffix" : "prefix" });
              }} />, and put <Phrase label="Possessor placement" value={speechOwner.design.grammar?.possessor ?? ""}
              choices={[{ key: "", text: `${POSSESSOR_PHRASE[drawn?.possessor ?? speech.grammar.possessor].choice}, as their speech falls out` },
                ...(["before", "after"] as PossessorOrder[]).map((possessor) => ({ key: possessor, text: POSSESSOR_PHRASE[possessor].choice }))]}
              onChange={(key) => setGrammar({ possessor: key === "" ? null : key as PossessorOrder })} />.</p>
            {speech.grammar.sample ? <><Sample rendering={speech.grammar.sample.sentence} label="Sample sentence" />
              <Sample rendering={speech.grammar.sample.possession} label="Sample possession" />
              <Sample rendering={speech.grammar.sample.future} label="Sample future" /></> : null}
          </section>
          <RootsBox key={speechOwner.key} founder={speechOwner} meanings={meanings} update={update} />
          {built ? <MeaningsBox meanings={meanings} fields={built.fields.filter((set) => !["core", "specimen", "custom"].includes(set.id))} onChange={setMeanings} /> : null}
        </> : null}
      </div>
    </>;
  } else {
    const peoples = overview?.communities ?? [];
    const homelands = new Map<number, typeof peoples>();
    for (const people of peoples) {
      const residents = homelands.get(people.region) ?? [];
      residents.push(people);
      homelands.set(people.region, residents);
    }
    plate = <section className="founding-title-page" aria-label="The title page">
      <p className="founding-title-kicker">The book of a world</p>
      <input className="founding-title" aria-label="World name" value={title ?? defaultTitle ?? ""}
        placeholder="An unnamed world" onChange={(event) => setTitle(event.target.value)} />
      <p className="founding-byline">by <input aria-label="Author" placeholder="your name" value={author} onChange={(event) => setAuthor(event.target.value)} /></p>
      <p className="founding-title-peoples">{[...homelands].map(([region, residents], group) => <span key={region}>
        {group ? "; " : ""}{residents.map((c, i) => <span key={c.id} style={{ "--tone": hue(overview?.varieties[c.variety]?.family ?? 0) } as CSSProperties}>
          {i === 0 ? "" : i < residents.length - 1 ? ", " : residents.length > 2 ? ", and " : " and "}the <b>{nameOf(c.id)}</b>
        </span>)} in {landTitle(region)}
      </span>)}.</p>
      <p className="founding-title-years"><label>Their chronicle runs to year <input type="number" aria-label="Years to write" min={YEARS} step={YEARS} value={Number.isNaN(years) ? "" : years}
        onChange={(event) => setYears(event.currentTarget.valueAsNumber)} /></label></p>
      <p className="founding-seed">Seed <code>{worldSeed}</code> · {catalog.mapSizes.find((choice) => choice.id === size)?.name} world</p>
    </section>;
  }

  return (
    <div className="stage setup founding" data-step={step} data-detent={detent}>
      <header className="founding-rail">
        <button type="button" className="brand" aria-label="Back to the chart room" onClick={() => { if (keepDraft()) onChartRoom(); }}>
          <span className="brand-name"><span>ʿUmrān</span></span>
        </button>
        <Popover label="Founding a world" role="menu" align="end"
          trigger={(props) => <button type="button" className="founding-step-menu" {...props}>
            {stepIndex + 1} of 6 · {STEPS[stepIndex].name} <ChevronDown size={14} aria-hidden="true" />
          </button>}>
          {(close) => <>{STEPS.map((s, i) => <button type="button" key={s.id}
            aria-current={s.id === step ? "step" : undefined} disabled={!reachable(s.id)}
            onClick={() => { close(); go(s.id); }}>{i + 1} · {s.name}</button>)}
            <button type="button" disabled>6 · The years</button></>}
        </Popover>
        <ol className="founding-steps" aria-label="Founding a world">
          {STEPS.map((s, i) => <li key={s.id}>
            <button type="button" aria-current={s.id === step ? "step" : undefined} disabled={!reachable(s.id)} onClick={() => go(s.id)}>
              <span>{i + 1}</span> {s.name}
            </button>
          </li>)}
          <li><span className="founding-steps-last" aria-disabled="true"><span>6</span> The years</span></li>
        </ol>
      </header>
      <section ref={mapContainer} className="stage-map" aria-label={projection === "globe" ? "Globe" : "Chart"} aria-busy={!currentBuild}>
        {forming && map ? <FormingChart key={worldKey} map={map} forming={forming} onDone={formedNow} />
          : chartReady ? <MapView key={worldKey} map={map} overview={overview} generation={0} tint={{ kind: "peoples" }}
          projection={projection} onProjection={setProjection}
          riverNames={riverNames}
          lakeNames={lakeNames}
          reach={people && built && step !== "world" ? { preview: built.preview, people: people.id } : undefined}
          chosen={new Set(people ? [people.id] : [])}
          lands={new Set(inspected === null ? [] : [inspected])}
          focus={focus === null ? null : map.regions[focus]?.site ?? null}
          zoomable
          onPeople={(id) => {
            if (!currentBuild || !founders[id]) return;
            const region = founders[id].region;
            if ((moving !== null || adding) && region !== null) settle(region);
            else choose(founders[id].key);
          }}
          onInspect={(feature, at) => setInspection(feature ? { feature, at } : null)}
          onClearLand={() => { setInspected(null); setInspection(null); }}
          onLand={(region) => {
            if (!currentBuild || map.regions[region].terrain === "sea") return;
            if (moving !== null || adding) settle(region);
            else {
              setInspected(region);
              if (step === "world") setStep("homeland");
              setDetent((current) => current === "peek" ? "half" : current);
            }
          }} /> : <p className="founding-drawing">Drawing the coasts…</p>}
        {chartReady && !forming ? <MapInspector container={mapContainer} inspection={inspection} map={map} overview={overview} generation={0} selectedVariety={people?.variety} riverNames={riverNames} lakeNames={lakeNames} pinnable={false} /> : null}
        {hint ? <p className={moving !== null || adding ? "map-hint founding-moving" : "map-hint"}>{hint}</p> : null}
      </section>

      <Sheet detent={detent} onDetent={setDetent} label={`Raise or lower ${STEPS[stepIndex].name.toLowerCase()}`}>
      <aside className="pedia setup-panel founding-plate" aria-label={STEPS[stepIndex].name} aria-busy={!currentBuild}>
        {draftNotice ? <p className="notice" role="status">{draftNotice}</p> : null}
        {plate}
      </aside>

      <footer className="timebar setup-foot">
        {stepIndex > 0 ? <button type="button" className="link founding-back" onClick={() => go(STEPS[stepIndex - 1].id)}>
          <ArrowLeft size={14} aria-hidden="true" /> {STEPS[stepIndex - 1].name}</button> : <span />}
        <span className="year">{saveError || error ? <span className="error" role="alert">{saveError ?? error}</span> : founders.length
          ? <><strong>Year 0.</strong> {founders.length} {founders.length === 1 ? "people" : "peoples"}; their chronicle is yet to be written.</>
          : "A world before its first peoples."}</span>
        {step === "title"
          ? <button type="button" className="primary begin" disabled={!canBegin} onClick={begin}><Feather size={18} aria-hidden="true" /> Write the chronicle</button>
          : <button type="button" className="primary founding-next" disabled={!reachable(STEPS[stepIndex + 1].id)} onClick={() => go(STEPS[stepIndex + 1].id)}>
            {STEPS[stepIndex].next} <ArrowRight size={16} aria-hidden="true" /></button>}
      </footer>
      </Sheet>
      {adjustingFounder ? <Modal open wide title="Shape their sounds" onClose={() => setAdjusting(null)}>
        {founders.some((founder) => founder.source === adjustingFounder.key) ? <p className="shared-speech-note">These are the founding sounds shared by this people and their kin.</p> : null}
        <Designer catalog={catalog} submit="Use these sounds" initial={adjustingFounder} onCancel={() => setAdjusting(null)}
          onFound={(founder) => { setAdjusting(null); update(adjustingFounder.key, { ...founder, preset: null }); }} />
      </Modal> : null}
      {redraw ? <Modal open title="Redraw this world?" onClose={() => setRedraw(null)}
        footer={<><button type="button" onClick={() => setRedraw(null)}>Keep these lands</button>
          <button type="button" className="primary" onClick={() => changeWorld(redraw)}>Redraw and relocate</button></>}>
        <p>The coastlines and homelands will change. These peoples keep their speech, kinship, and choices, and settle in the redrawn world.</p>
      </Modal> : null}
      {clearing ? <Modal open title="Start again?" onClose={() => setClearing(false)}
        footer={<><button type="button" onClick={() => setClearing(false)}>Keep them</button>
          <button type="button" className="primary" onClick={() => { setFounders([]); setSelected(null); grammarDraws.current.clear(); setClearing(false); }}>Clear the peoples</button></>}>
        <p>Every people founded here, with their speech and choices, will be cleared. The coasts and the homeland stay.</p>
      </Modal> : null}
    </div>
  );
}

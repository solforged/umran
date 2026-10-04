import { createContext, Fragment, useCallback, useContext, useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { createPortal } from "react-dom";
import {
  ArrowLeft,
  Eye,
  Feather,
  AudioLines,
  Earth,
  Globe,
  Languages,
  Landmark,
  Hammer,
  Sparkles,
  MapPin,
  Play,
  ScrollText,
  Search,
  ChevronRight,
  Users,
  WholeWord,
  Waves,
  CloudSunRain,
  X,
  type LucideIcon,
} from "lucide-react";
import type { Annal, Catalog, CityView, Community, Craft, CraftView, ReadEngine, Ethos, HistoryPoint, HolyLand, NotebookNote, Overview, PlaceExonym, ReligionView, RenderingRow, ShrineKind, StateView, Variety, WordMap, WorldMap } from "../model";
import { YEARS } from "../model";
import { causePhrase, CONTACT_NAME, ETHOS_AXES, ETHOS_POLES, EVENT_KIND, faithTeaching, FAITH_HOW, FAITH_NAME, FALL_NAME, howCame, howNamed, hue, landLabel, LIVELIHOOD_NAME, peoplePhrases, RISE_NAME, SCHISM_CAUSE, seasonalPhrase, STRONG, temperament, TENET_NOUN, TERRAIN_NAME, unnamedName, weatherDeparture } from "../lore";
import { filterHistory, findAnnal, individualAnnals, relatedMoments, subjectHistory, HISTORY_GROUPS, INITIAL_HISTORY, type HistoryView } from "../history";
import { eras, quietLine, QUIET_KINDS, type Era } from "../eras";
import { bond } from "../words";
import type { InterventionKind } from "./ActionDialog";
import { SettlementAccount } from "./SettlementDesk";
import { Told } from "./Told";
import { Explained } from "./Explained";
import { INITIAL_DICTIONARY, type DictionaryView } from "./Dictionary";
import { peoplesByRegion } from "./MapView";
import { Specimen } from "./Specimen";
import { DescentChart, FamilyForest, type Lineage } from "./FamilyTree";
import { WordGloss } from "./WordGloss";
import { Renderings } from "./Renderings";
import { closeClosingDialogs, emphasizeInk, reducedMotion, useLiftedValue } from "../motion";
import { Popover } from "./Popover";
import { CHAPTER, ChapterSummary, LawEvidence, LoanCauseText, WordOrigin, type ChapterContext } from "./LanguageChapter";
import { Margin } from "./Margin";
import "./chronicle.css";

/// What the encyclopedia is open at.
export type Focus = import("../model").Subject;

/// Share of core words below which two languages count as unrelated.
const KIN_FLOOR = 0.05;

interface Context {
  storyView: HistoryView;
  onStoryView: (view: HistoryView) => void;
  notes: NotebookNote[];
  onSaveNote: (note: NotebookNote) => string | null;
  onRemoveNote: (id: string) => string | null;
  onReadNote: (note: NotebookNote) => string | null;
  onStartNote: () => void;
  onFollow: (subject: Focus, label: string) => void;
  historyView: HistoryView;
  onHistoryView: (view: HistoryView) => void;
  dictionaryViews: Record<number, DictionaryView>;
  onDictionaryView: (variety: number, view: DictionaryView) => void;
  engine: ReadEngine;
  catalog: Catalog;
  version: number;
  generation: number;
  overview: Overview;
  map: WorldMap;
  /// Every people's word for the meaning in view, if a word is in view.
  words: WordMap | null;
  go: (focus: Focus) => void;
  onScrub: (generation: number) => void;
  onVisit: (subject: Focus, generation: number) => void;
  title?: string;
  onRun: () => void;
  running?: boolean;
  canUndo: boolean;
  onUndo: () => void;
  onDialog: (kind: InterventionKind, community: number) => void;
  onReconsider: (annal: Annal) => void;
  onReturnBefore: (point: HistoryPoint) => void;
  /// Tell the history again as a telling set aside told it.
  onRestore: (telling: number) => void;
  onRenameTelling: (telling: number, name: string) => void;
  onCompare: (telling: number) => void;
  /// The folio page open over the map, by its section's id.
  leaf: string | null;
  onLeaf: (leaf: string | null) => void;
  /// Where the folio is laid: over the map.
  folioHost: HTMLElement | null;
  /// The language whose known world veils the map, if one does.
  knownBy: number | null;
  onKnownBy: (variety: number | null) => void;
}

/// The folio: a card's long and wide sections, laid open over the map one
/// at a time. Each section registers its title for the folio's tabs.
interface Folio {
  open: string | null;
  page: HTMLElement | null;
  show: (leaf: string | null) => void;
  register: (id: string, title: string) => () => void;
  closing: boolean;
  previousCard: boolean;
  remember: (children: ReactNode) => void;
}

const FolioContext = createContext<Folio | null>(null);

/// Most cards the trail names before the one open.
const TRAIL_SHOWN = 3;

/// The encyclopedia: one card at a time about whatever is in focus, with
/// every name in it leading to that thing's own card. Each card opens the
/// same way: what kind of thing it is, its name, a box of facts, and its
/// specimen words where it has a language, then sections to read on. Long
/// and wide sections show a line on the card and open in the folio over
/// the map. The last few cards visited stay named above it, to step back to.
export function Pedia({
  trail,
  onReturn,
  onIndex,
  hidden,
  cardMotion,
  instantFolio,
  folioFromCard,
  onFolioLifted,
  ...context
}: Context & { trail: Focus[]; onReturn: (index: number) => void; onIndex: () => void; hidden: boolean;
  cardMotion: { direction: string; keyboard: boolean; ink: boolean }; instantFolio: boolean; onFolioLifted: () => void; folioFromCard: boolean }) {
  const focus = trail.at(-1)!;
  const cardKey = JSON.stringify(focus);
  const aside = useRef<HTMLElement>(null);
  const folioElement = useRef<HTMLElement>(null);
  const opener = useRef<HTMLElement | null>(null);
  const folioId = useId();
  const readingPositions = useRef(new Map<string, number>());
  const first = Math.max(0, trail.length - 1 - TRAIL_SHOWN);
  const { leaf, onLeaf, folioHost } = context;
  const liftedLeaf = useLiftedValue(leaf, onFolioLifted);
  const renderedLeaf = liftedLeaf.value;
  const lastCard = useRef(cardKey);
  const sheetWasOpen = useRef(false);
  const [switchingLeaf, setSwitchingLeaf] = useState(false);
  const [leaves, setLeaves] = useState<{ id: string; title: string }[]>([]);
  const [page, setPage] = useState<HTMLDivElement | null>(null);
  const lastContents = useRef<ReactNode>(null);
  const entry = focus.kind === "event" ? findAnnal(context.overview.annals, focus.id) : undefined;
  const cardLabel = entry ? eventHeading(entry, context).title : focusLabel(focus, context);
  const lastSheet = useRef({ card: cardKey, label: cardLabel, leaves });
  const remember = useCallback((children: ReactNode) => { lastContents.current = children; }, []);
  const contentLift = useRef<number | undefined>(undefined);
  const show = useCallback((next: string | null) => {
    window.clearTimeout(contentLift.current);
    page?.querySelector(".folio-content-leaving")?.remove();
    if (next && leaf && next !== leaf && !reducedMotion()) {
      const outgoing = page?.querySelector(".folio-content")?.cloneNode(true) as HTMLElement | undefined;
      if (outgoing) {
        outgoing.className = "folio-content-leaving";
        outgoing.inert = true; outgoing.setAttribute("aria-hidden", "true");
        page?.append(outgoing);
        contentLift.current = window.setTimeout(() => outgoing.remove(), 90);
      }
    }
    onLeaf(next);
  }, [leaf, page, onLeaf]);
  useEffect(() => () => { window.clearTimeout(contentLift.current); }, []);
  const register = useCallback((id: string, title: string) => {
    setLeaves((all) => (all.some((l) => l.id === id) ? all.map((l) => (l.id === id ? { id, title } : l)) : [...all, { id, title }]));
    return () => setLeaves((all) => all.filter((l) => l.id !== id));
  }, []);
  // A return visit restores the card's chosen section, once it has registered.
  const shown = leaf !== null && leaves.some((l) => l.id === leaf);
  if (shown) lastSheet.current = { card: cardKey, label: cardLabel, leaves };
  const visible = shown || (renderedLeaf !== null && liftedLeaf.closing);
  const sheetLabel = liftedLeaf.closing ? lastSheet.current.label : cardLabel;
  const sheetLeaves = liftedLeaf.closing ? lastSheet.current.leaves : leaves;
  const previousCard = liftedLeaf.closing && lastSheet.current.card !== cardKey;
  const folio = useMemo<Folio>(() => ({ open: renderedLeaf, page, show, register, closing: liftedLeaf.closing, previousCard, remember }), [renderedLeaf, page, show, register, liftedLeaf.closing, previousCard, remember]);
  useLayoutEffect(() => {
    if (lastCard.current === cardKey) return;
    lastCard.current = cardKey;
    const card = aside.current?.querySelector<HTMLElement>("article.card");
    if (card) card.dataset.direction = cardMotion.direction;
    const heading = card?.querySelector<HTMLElement>(".card-head h2") ?? null;
    if (heading) {
      heading.tabIndex = -1;
      if (cardMotion.keyboard) heading.focus({ preventScroll: true });
      if (cardMotion.ink) emphasizeInk(heading);
    }
  }, [cardKey, cardMotion]);
  useLayoutEffect(() => {
    setSwitchingLeaf(sheetWasOpen.current && shown);
    sheetWasOpen.current = shown;
  }, [leaf]);
  useLayoutEffect(() => {
    aside.current?.scrollTo({ top: readingPositions.current.get(cardKey) ?? 0 });
  }, [cardKey]);
  useLayoutEffect(() => {
    if (!liftedLeaf.closing) page?.scrollTo({ top: readingPositions.current.get(`${cardKey}:${renderedLeaf}`) ?? 0 });
  }, [cardKey, renderedLeaf, page, liftedLeaf.closing]);
  useLayoutEffect(() => {
    if (!shown || instantFolio) return;
    closeClosingDialogs();
    if (!opener.current || !opener.current.isConnected) opener.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    if (!folioFromCard || (cardMotion.keyboard && window.matchMedia("(max-width: 900px)").matches)) {
      folioElement.current?.querySelector<HTMLButtonElement>("[aria-selected=true]")?.focus({ preventScroll: true });
    }
    return () => { if (opener.current?.isConnected) opener.current.focus({ preventScroll: true }); opener.current = null; };
  }, [shown, instantFolio]);
  useLayoutEffect(() => {
    if (!shown || !instantFolio) return;
    if (window.matchMedia("(max-width: 900px)").matches) {
      folioElement.current?.querySelector<HTMLElement>("[aria-selected=true]")?.focus({ preventScroll: true });
    } else {
      aside.current?.querySelector<HTMLElement>(`.leaf-title[data-leaf="${leaf}"]`)?.focus({ preventScroll: true });
    }
  }, [shown, instantFolio, page, leaf]);
  useEffect(() => {
    if (!shown) return;
    const close = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !document.querySelector("dialog:modal")) onLeaf(null);
    };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [shown, onLeaf]);
  return (
    <aside className="pedia" hidden={hidden} ref={aside} aria-label="Encyclopedia" onScroll={(e) => readingPositions.current.set(cardKey, e.currentTarget.scrollTop)}>
      <nav className="pedia-nav" aria-label="Card navigation">
        <button
          type="button"
          className="icon"
          disabled={trail.length < 2}
          onClick={() => onReturn(trail.length - 2)}
          title="Back"
          aria-label="Back to the previous card"
        >
          <ArrowLeft size={16} />
        </button>
        <button type="button" className="icon" onClick={() => context.go({ kind: "world" })} title="The world">
          <Globe size={16} aria-hidden="true" /><span>World</span>
        </button>
        <button
          type="button"
          className="icon"
          onClick={() => context.go({ kind: "history" })}
          title="Everything that has happened"
        >
          <ScrollText size={16} aria-hidden="true" /><span>Chronicle</span>
        </button>
        <button type="button" className="icon pedia-search" onClick={onIndex} title="Search the chart" aria-label="Search the chart"><Search size={16} /></button>
      </nav>
          <ol className="trail" aria-label="Cards visited">
            <li className="trail-year" aria-current="date">Year {context.generation * YEARS}</li>
            {first > 0 ? <li aria-hidden="true">…</li> : null}
            {trail.slice(first, -1).map((f, i) => (
              <li key={first + i}>
                <button type="button" className="link" onClick={() => onReturn(first + i)}>
                  {focusLabel(f, context)}
                </button>
              </li>
            ))}
          </ol>
      <FolioContext.Provider value={folio}>
        <article className="card" key={cardKey}>
          {!["world", "history", "event", "word"].includes(focus.kind) ? <div className="card-reading-tools">
            <button type="button" className="link" onClick={() => context.onFollow(focus, focusLabel(focus, context))}><Eye size={14} aria-hidden="true" /> Follow</button>
          </div> : null}
          <Card focus={focus} context={context} />
          <Margin notes={context.notes} overview={context.overview} subject={focus}
            label={`${focusLabel(focus, context)}${focus.kind === "word" ? ` in ${context.overview.varieties[focus.variety]?.name ?? "its language"}` : ""}`}
            onSave={context.onSaveNote} onRemove={context.onRemoveNote} onRead={context.onReadNote} onStart={context.onStartNote} />
        </article>
      </FolioContext.Provider>
      {folioHost && visible && !hidden
        ? createPortal(
            <section className="folio" ref={folioElement} data-instant={instantFolio || undefined} data-closing={liftedLeaf.closing || undefined} inert={liftedLeaf.closing} aria-hidden={liftedLeaf.closing || undefined} aria-label={`Card: ${sheetLabel}`}>
              <header className="folio-head">
                <h2 className="folio-of">{sheetLabel}</h2>
                <div className="folio-tabs" role="tablist" aria-label="Sections" onKeyDown={(e) => {
                  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) return;
                  const current = leaves.findIndex((l) => l.id === renderedLeaf);
                  const next = e.key === "Home" ? 0 : e.key === "End" ? leaves.length - 1
                    : (current + (e.key === "ArrowRight" ? 1 : -1) + leaves.length) % leaves.length;
                  e.preventDefault(); show(leaves[next].id);
                  e.currentTarget.querySelectorAll<HTMLButtonElement>("[role=tab]")[next]?.focus();
                }}>
                  {sheetLeaves.map((l) => (
                    <button
                      key={l.id}
                      type="button"
                      role="tab"
                      id={`${folioId}-${l.id}`}
                      aria-controls={`${folioId}-page`}
                      tabIndex={l.id === renderedLeaf ? 0 : -1}
                      aria-selected={l.id === renderedLeaf}
                      className="folio-tab"
                      onClick={() => show(l.id)}
                    >
                      {l.title}
                    </button>
                  ))}
                </div>
                <button type="button" className="icon folio-close" title="Close the wide card (Esc)" aria-label="Close the wide card" onClick={() => onLeaf(null)}>
                  <X size={16} />
                </button>
              </header>
              <div className="folio-page card" data-switch={switchingLeaf || undefined} role="tabpanel" id={`${folioId}-page`} aria-labelledby={`${folioId}-${renderedLeaf}`} tabIndex={0} ref={setPage} onScroll={(e) => readingPositions.current.set(`${cardKey}:${renderedLeaf}`, e.currentTarget.scrollTop)} />
            </section>,
            folioHost,
          )
        : null}
      {previousCard && page ? createPortal(<div className="folio-content">{lastContents.current}</div>, page) : null}
    </aside>
  );
}

/// A card's name in a few words, for the trail.
function focusLabel(focus: Focus, context: Context): string {
  const { overview } = context;
  switch (focus.kind) {
    case "world":
      return context.title ?? "The world";
    case "people":
      return overview.communities[focus.id]?.name ?? "A people";
    case "state":
      return overview.states[focus.id]?.name ?? "A state";
    case "religion":
      return overview.religions[focus.id]?.name ?? FAITH_NAME.kind;
    case "craft":
      return overview.crafts.find((c) => c.id === focus.id)?.name ?? "A craft";
    case "language":
      return overview.varieties[focus.variety]?.name ?? "A language";
    case "word":
      return `“${focus.concept.replaceAll("_", " ")}”`;
    case "law":
      return overview.varieties.flatMap((v) => v.laws).find((l) => l.id === focus.id)?.label ?? "A sound change";
    case "land":
      return landName(focus.region, context);
    case "continent":
      return continentName(focus.landmass, context);
    case "river":
      return context.engine.river(context.generation, focus.id).names.at(-1)?.spelled ?? unnamedName("river");
    case "lake":
      return context.engine.lake(context.generation, focus.id).names.at(-1)?.spelled ?? unnamedName("lake");
    case "zone":
      return zoneName(focus.id, context);
    case "event":
      return `Year ${(findAnnal(overview.annals, focus.id)?.generation ?? context.generation) * YEARS}`;
    case "history":
      return "The chronicle";
  }
}

function Card({ focus, context }: { focus: Focus; context: Context }) {
  const { overview } = context;
  switch (focus.kind) {
    case "world":
      return <WorldCard context={context} />;
    case "people":
      return overview.communities[focus.id] ? (
        <PeopleCard c={overview.communities[focus.id]} context={context} />
      ) : (
        <p className="muted">This people has not yet appeared in this year.</p>
      );
    case "state":
      return overview.states[focus.id] ? (
        <StateCard state={overview.states[focus.id]} context={context} />
      ) : (
        <p className="muted">This state has not yet arisen in this year.</p>
      );
    case "religion":
      return overview.religions[focus.id] ? (
        <ReligionCard religion={overview.religions[focus.id]} context={context} />
      ) : (
        <p className="muted">This religion has not yet been founded in this year.</p>
      );
    case "craft": {
      const craft = overview.crafts.find((c) => c.id === focus.id);
      return craft ? <CraftCard craft={craft} context={context} /> : null;
    }
    case "language":
      return overview.varieties[focus.variety] ? (
        <LanguageCard variety={focus.variety} context={context} />
      ) : (
        <p className="muted">This language has not yet arisen in this year.</p>
      );
    case "word":
      return overview.varieties[focus.variety] ? <WordCard variety={focus.variety} concept={focus.concept} context={context} />
        : <p className="muted">This language has not yet arisen in this year.</p>;
    case "law":
      return <LawCard id={focus.id} context={context} />;
    case "land":
      return <LandCard region={focus.region} context={context} />;
    case "continent":
      return <ContinentCard landmass={focus.landmass} context={context} />;
    case "river":
      return <RiverCard id={focus.id} context={context} />;
    case "lake":
      return <LakeCard id={focus.id} context={context} />;
    case "zone":
      return <ZoneCard id={focus.id} context={context} />;
    case "event": {
      const annal = findAnnal(overview.annals, focus.id);
      return annal ? <EventCard annal={annal} context={context} />
        : <p className="muted">This entry is not recorded in this year.</p>;
    }
    case "history":
      return <HistoryCard context={context} />;
  }
}

/// How every card opens: what kind of thing this is, its name in a colour
/// of its own, and a line saying what the name means or how it sounds.
function CardHead({
  icon: Icon,
  kind,
  title,
  sub,
  tone,
  hand,
}: {
  icon: LucideIcon;
  kind: string;
  title: ReactNode;
  sub?: ReactNode;
  tone?: string;
  hand?: number;
}) {
  return (
    <header className="card-head" style={tone ? ({ "--tone": tone } as CSSProperties) : undefined}>
      <p className="card-kind">
        <Icon size={14} aria-hidden="true" /> {kind}
      </p>
      <h2 tabIndex={-1} className={hand === undefined ? undefined : `hand-${hand % 5}`}>{title}</h2>
      {sub ? <p className="card-sub">{sub}</p> : null}
    </header>
  );
}

/// The facts to take in at a glance, as an encyclopedia's infobox. Rows
/// with nothing to say are left out.
function Facts({ rows }: { rows: [string, ReactNode][] }) {
  const shown = rows.filter(([, value]) => value !== null && value !== undefined && value !== false);
  if (shown.length === 0) return null;
  return (
    <dl className="infobox">
      {shown.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}


function PeopleLink({ c, context }: { c: Community; context: Context }) {
  const family = context.overview.varieties[c.variety].family;
  return (
    <button
      type="button"
      className="link word people-link"
      style={{ color: hue(family) }}
      onClick={() => context.go({ kind: "people", id: c.id })}
    >
      {c.name}
    </button>
  );
}

function StateLink({ state, context }: { state: StateView; context: Context }) {
  return (
    <button
      type="button"
      className="link word"
      style={{ color: hue(state.id) }}
      onClick={() => context.go({ kind: "state", id: state.id })}
    >
      {state.name}
    </button>
  );
}

function ReligionLink({ religion, context }: { religion: ReligionView; context: Context }) {
  return (
    <button type="button" className="link word" style={{ color: hue(religion.id) }}
      onClick={() => context.go({ kind: "religion", id: religion.id })}>
      {religion.name}
    </button>
  );
}

function CraftLink({ craft, context }: { craft: Craft; context: Context }) {
  return (
    <button type="button" className="link" onClick={() => context.go({ kind: "craft", id: craft })}>
      {context.overview.crafts.find((c) => c.id === craft)?.name}
    </button>
  );
}

function AnnalLinks({ annal, context }: { annal: Annal; context: Context }) {
  const { overview } = context;
  const links = [
    ...annal.states.filter((id) => overview.states[id]).map((id) => <StateLink key={`state-${id}`} state={overview.states[id]} context={context} />),
    ...annal.religions.filter((id) => overview.religions[id]).map((id) => <ReligionLink key={`religion-${id}`} religion={overview.religions[id]} context={context} />),
    ...annal.crafts.map((id) => <CraftLink key={id} craft={id} context={context} />),
    ...annal.rivers.map((id) => <RiverLink key={`river-${id}`} id={id} context={context} />),
    ...annal.zones.map((id) => <ZoneLink key={`zone-${id}`} id={id} context={context} />),
  ];
  if (links.length === 0) return null;
  return <span className="annal-states"><Joined items={links} link={(link) => link} /></span>;
}

function LanguageLink({ variety, context }: { variety: number; context: Context }) {
  return (
    <button type="button" className="link word" onClick={() => context.go({ kind: "language", variety })}>
      {context.overview.varieties[variety].name}
    </button>
  );
}

function landName(region: number, context: Context): string {
  return landLabel(context.overview, context.map, region);
}

function LandLink({ region, context }: { region: number; context: Context }) {
  return (
    <button type="button" className="link word" onClick={() => context.go({ kind: "land", region })}>
      {landName(region, context)}
    </button>
  );
}

/// A continent's name on the chart, or a plain description before anyone
/// has named it.
function continentName(landmass: number, context: Context): string {
  return context.overview.continents.find((c) => c.landmass === landmass)?.name?.name ?? unnamedName(context.map.landmasses[landmass].kind);
}

function ContinentLink({ landmass, context }: { landmass: number; context: Context }) {
  return (
    <button type="button" className="link word" onClick={() => context.go({ kind: "continent", landmass })}>
      {continentName(landmass, context)}
    </button>
  );
}

/// Which body of land a region lies on: its continent, as a link, or how
/// large its island is.
function LandmassOf({ region, context }: { region: number; context: Context }) {
  const m = context.map.regions[region].landmass;
  if (m === null) return null;
  const mass = context.map.landmasses[m];
  if (mass.kind === "continent") return <ContinentLink landmass={m} context={context} />;
  return <>an island of {mass.regions.length === 1 ? "one land" : `${mass.regions.length} lands`}</>;
}

/// Why a faith reveres its shrine, in a few words.
const SHRINE_KIND: Record<ShrineKind, string> = {
  home: "the founders’ own land",
  mountain: "a holy mountain",
  island: "a holy island",
  "far-shore": "a shore across the sea",
};

/// The lands a language knows by name, grouped by the body of land they
/// lie on, with a switch to veil the rest of the map.
function KnownWorldLeaf({ variety, context }: { variety: number; context: Context }) {
  const { map, overview } = context;
  const known = overview.varieties[variety].knownLands;
  if (known.length === 0) return null;
  const groups = new Map<number, typeof known>();
  for (const land of known) {
    const m = map.regions[land.region].landmass;
    if (m === null) continue;
    groups.set(m, [...(groups.get(m) ?? []), land]);
  }
  const continents = [...groups.keys()].filter((m) => map.landmasses[m].kind === "continent");
  const islands = groups.size - continents.length;
  const total = map.regions.filter((r) => r.landmass !== null).length;
  const showing = context.knownBy === variety;
  return (
    <Leaf
      id="known"
      title="Their known world"
      summary={
        <p>
          {known.length} of {total} lands known by name
          {continents.length > 0 ? (
            <>, on <Joined items={continents} link={(m) => <ContinentLink landmass={m} context={context} />} /></>
          ) : null}
          {islands > 0 ? <>{continents.length > 0 ? " and" : ","} {islands === 1 ? "one island" : `${islands} islands`}</> : null}.{" "}
          <button type="button" className="link" aria-pressed={showing} onClick={() => context.onKnownBy(showing ? null : variety)}>
            {showing ? "Show the whole chart" : "Show only these on the chart"}
          </button>
        </p>
      }
    >
      <p className="muted small">
        Lands its speakers hold or held, border, or have heard of from peoples they deal with, each as they say its name. A
        name heard long ago may be stale.
      </p>
      {[...groups].map(([m, lands]) => (
        <section key={m}>
          <h4>
            {map.landmasses[m].kind === "continent" ? <ContinentLink landmass={m} context={context} /> : "An island"}
            {map.landmasses[m].kind === "continent" ? (
              <span className="muted"> · {lands.length} of {map.landmasses[m].regions.length} lands</span>
            ) : null}
          </h4>
          <p>
            <Joined
              items={lands}
              link={(land) => (
                <button type="button" className="link word" title={`/${land.ipa}/`} onClick={() => context.go({ kind: "land", region: land.region })}>
                  {land.spelled}
                </button>
              )}
            />
          </p>
        </section>
      ))}
    </Leaf>
  );
}

/// Cities by name, each once: a city that fell and rose again under
/// another state keeps its name, and the chart names it once.
function cityNames(cities: CityView[]): CityView[] {
  const seen = new Set<string>();
  return cities.filter((city) => !seen.has(city.name.name) && seen.add(city.name.name));
}

function ContinentCard({ landmass, context }: { landmass: number; context: Context }) {
  const { map, overview } = context;
  const mass = map.landmasses[landmass];
  const view = overview.continents.find((c) => c.landmass === landmass);
  const name = view?.name ?? null;
  const islands = map.landmasses.filter((m) => m.kind === "island").length;
  const knowers = overview.varieties.filter((v) => v.spoken && v.knownLands.some((l) => mass.regions.includes(l.region)));
  const peoples = (view?.peoples ?? []).map((id) => overview.communities[id]).sort((a, b) => b.size - a.size);
  const languages = [...new Set(peoples.map((c) => c.variety))].filter((id) => overview.varieties[id].spoken);
  const families = new Set(languages.map((id) => overview.varieties[id].family));
  const terrainCounts = new Map<(typeof map.regions)[number]["terrain"], number>();
  for (const id of mass.regions) {
    const terrain = map.regions[id].terrain;
    terrainCounts.set(terrain, (terrainCounts.get(terrain) ?? 0) + 1);
  }
  const lands = [...terrainCounts].sort((a, b) => b[1] - a[1])
    .map(([terrain, count]) => `${count} ${TERRAIN_NAME[terrain].toLowerCase()}`).join(", ");
  const states = overview.states.filter((state) => state.lands.some((id) => mass.regions.includes(id)));
  const cities = cityNames(overview.cities.filter((city) => mass.regions.includes(city.region)));
  return (
    <>
      <CardHead
        icon={Earth}
        kind="A continent"
        title={name?.name ?? unnamedName(map.landmasses[landmass].kind)}
        sub={name ? <>“{name.meaning}” <span className="ipa">/{name.ipa}/</span></> : null}
      />
      <Facts
        rows={[
          ["Area", `${mass.regions.length} lands, ${Math.round(mass.regions.reduce((area, id) => area + map.regions[id].areaKm2, 0)).toLocaleString()} km²`],
          ["Named by", name ? (
            <><PeopleLink c={overview.communities[name.people]} context={context} />, in{" "}
              <LanguageLink variety={name.variety} context={context} />, year <Year generation={name.since} context={context} /></>
          ) : "no one yet"],
          ["Known from", name ? <LandLink region={name.witness} context={context} /> : null],
          ["Home of", view && view.peoples.length > 0 ? (
            <Joined items={view.peoples} link={(id) => <PeopleLink c={overview.communities[id]} context={context} />} />
          ) : "no one now"],
          ["Languages", languages.length > 0 ? (
            <Joined items={languages} link={(variety) => <LanguageLink variety={variety} context={context} />} />
          ) : null],
          ["Families", families.size],
          ["Lands", `${mass.regions.length}: ${lands}`],
          ["States", states.length > 0 ? (
            <Joined items={states} link={(state) => <StateLink state={state} context={context} />} />
          ) : null],
          ["Cities", cities.length > 0 ? (
            <Joined items={cities} link={(city) => <span className="word">{city.name.name}</span>} />
          ) : null],
          ["Shrines", view && view.religions.length > 0 ? (
            <Joined items={view.religions} link={(id) => <ReligionLink religion={overview.religions[id]} context={context} />} />
          ) : null],
          ["Known to", knowers.length > 0 ? (
            <Joined items={knowers} link={(v) => <LanguageLink variety={v.id} context={context} />} />
          ) : "no living language"],
        ]}
      />
      {peoples.length > 3 ? (
        <Leaf
          id="peoples"
          title="Its peoples"
          summary={`${peoples.length} peoples; the largest is ${peoples[0].name}`}
        >
          <ul className="roster">
            {peoples.map((c) => (
              <li key={c.id}>
                <PeopleLink c={c} context={context} /> <span className="muted">{overview.varieties[c.variety].name}</span>
              </li>
            ))}
          </ul>
        </Leaf>
      ) : null}
      <p className="muted small">
        {name ? <>Entered on the chart from the speech of the {overview.communities[name.people].name}, the first people known to have lived on or heard of it; the name stays as it was written, whatever becomes of their language.</> :
          <>No living people knows any of this land, so the chart has no name for it.</>}{" "}
        Each land is a region of the world; its area is shown in square kilometres. Lands vary in size. Beyond its shores lie{" "}
        <Joined
          items={[
            ...map.landmasses.map((_, i) => i).filter((i) => i !== landmass && map.landmasses[i].kind === "continent"),
            ...(islands > 0 ? [-1] : []),
          ]}
          link={(i) => (i < 0 ? <>{islands === 1 ? "one island" : `${islands} islands`}</> : <ContinentLink landmass={i} context={context} />)}
        />
        {map.landmasses.length === 1 ? "only the sea" : ""}.
      </p>
    </>
  );
}

function RiverLink({ id, context }: { id: number; context: Context }) {
  const view = context.engine.river(context.generation, id);
  return <button type="button" className="link word" onClick={() => context.go({ kind: "river", id })}>
    {view.names.at(-1)?.spelled ?? unnamedName("river")}
  </button>;
}

function LakeLink({ id, context }: { id: number; context: Context }) {
  const view = context.engine.lake(context.generation, id);
  return <button type="button" className="link word" onClick={() => context.go({ kind: "lake", id })}>
    {view.names.at(-1)?.spelled ?? unnamedName("lake")}
  </button>;
}

function ZoneLink({ id, context }: { id: number; context: Context }) {
  return <button type="button" className="link" onClick={() => context.go({ kind: "zone", id })}>{zoneName(id, context)}</button>;
}

/// A weather zone has no name of its own; call it after its first named
/// land, as a traveller would, or else by where it lies on its continent.
function zoneName(id: number, context: Context): string {
  const regions = context.map.climateZones[id]?.regions ?? [];
  const named = regions.find((r) => context.overview.places.some((p) => p.region === r && p.names.length > 0));
  if (named !== undefined) return `The weather around ${landName(named, context)}`;
  const first = context.map.regions[regions[0]];
  const mass = first?.landmass === null || first === undefined ? undefined : context.map.landmasses[first.landmass];
  if (!mass) return "The weather of unnamed lands";
  const [longitude, latitude] = context.map.regions[mass.anchor].center;
  const lon = longitude * Math.PI / 180, lat = latitude * Math.PI / 180;
  // An area-weighted spherical centroid does not jump across the chart seam.
  const centroid = regions.reduce(([x, y, z], id) => {
    const region = context.map.regions[id];
    const l = region.center[0] * Math.PI / 180, p = region.center[1] * Math.PI / 180;
    return [
      x + region.areaKm2 * Math.cos(p) * Math.cos(l),
      y + region.areaKm2 * Math.cos(p) * Math.sin(l),
      z + region.areaKm2 * Math.sin(p),
    ];
  }, [0, 0, 0]);
  const [x, y, z] = centroid;
  const east = -x * Math.sin(lon) + y * Math.cos(lon);
  const north = -x * Math.sin(lat) * Math.cos(lon) - y * Math.sin(lat) * Math.sin(lon) + z * Math.cos(lat);
  const toward = x * Math.cos(lat) * Math.cos(lon) + y * Math.cos(lat) * Math.sin(lon) + z * Math.sin(lat);
  const bearingLength = Math.hypot(east, north), centerLength = Math.hypot(x, y, z);
  const distanceKm = Math.atan2(bearingLength, toward) * context.map.radiusKm;
  // Balanced/antipodal districts and a polar anchor have no unique bearing.
  const side = centerLength < 1e-6 || Math.abs(latitude) > 89.999 || (toward < 0 && bearingLength < centerLength * 1e-9) ? "lands"
    : distanceKm < 100 ? "heart"
    : `${north > Math.abs(east) / 2 ? "north" : north < -Math.abs(east) / 2 ? "south" : ""}${Math.abs(east) > Math.abs(north) / 2 ? (east > 0 ? "-east" : "-west") : ""}`.replace(/^-/, "");
  const continent = context.overview.continents.find((c) => c.landmass === mass.id)?.name?.name;
  return continent ? `The weather of ${continent}'s ${side}` : `The weather of an unnamed land's ${side}`;
}

function RiverCard({ id, context }: { id: number; context: Context }) {
  const { engine, generation, version, map, overview } = context;
  const view = useMemo(() => engine.river(generation, id), [engine, generation, version, id]);
  const climate = useMemo(() => engine.climate(generation), [engine, generation, version]);
  const river = map.rivers[id];
  const now = view.names.at(-1);
  const living = overview.communities.filter((c) => c.ended === null && c.lands.some((land) => river.course.includes(land)));
  const flow = climate.rivers.find((flow) => flow.id === id);
  return <>
    <CardHead icon={Waves} kind="A river" title={now?.spelled ?? unnamedName("river")}
      sub={now ? <span className="ipa">/{now.ipa}/</span> : null} />
    <Facts rows={[
      ["Length", `${Math.round(river.lengthKm).toLocaleString()} km`],
      ["Lands", <Joined items={river.course} link={(region) => <LandLink region={region} context={context} />} />],
      ["Home of", living.length ? <Joined items={living} link={(c) => <PeopleLink c={c} context={context} />} /> : "no one now"],
      ["Flow", flow?.flowing ? "flowing now" : "flow has failed"],
      ["Joins", river.joins === null ? "the sea" : <RiverLink id={river.joins} context={context} />],
    ]} />
    {view.names.length || view.exonyms.length ? <Leaf id="names" title="Names"
      summary={<p>{view.names.length ? <>First recorded in year <Year generation={view.names[0].since} context={context} />.</> : "Known in other languages."}</p>}>
      <ol className="history">
        {view.names.map((name, i) => <li key={i}>
          <Year generation={name.since} context={context} />
          <span><span className="word">{name.spelled}</span> <span className="ipa">/{name.ipa}/</span>{" "}
            in <LanguageLink variety={name.variety} context={context} />.
            <span className="muted"> {howNamed(name, view.names[i - 1], overview)}{name.once ? `; once ${name.once}` : ""}.</span>
          </span>
        </li>)}
      </ol>
      {view.exonyms.length ? <>
        <h3>What others call it</h3>
        <ul className="roster">{view.exonyms.map((name, i) => <li key={i}>
          <span className="word">{name.spelled}</span> <span className="ipa">/{name.ipa}/</span>{" "}
          in <LanguageLink variety={name.variety} context={context} />, heard in year <Year generation={name.heard} context={context} />
          {name.once ? `; once ${name.once}` : ""}.
        </li>)}</ul>
      </> : null}
    </Leaf> : <p className="muted">No name has been recorded for this river yet.</p>}
    <StoryLeaf title="The river's chronicle" annals={subjectHistory({ kind: "river", id }, overview, map)} context={context} />
  </>;
}

function LakeCard({ id, context }: { id: number; context: Context }) {
  const { engine, generation, version, map, overview } = context;
  const view = useMemo(() => engine.lake(generation, id), [engine, generation, version, id]);
  const lake = map.lakes[id];
  const now = view.names.at(-1);
  return <>
    <CardHead icon={Waves} kind="A lake" title={now?.spelled ?? unnamedName("lake")}
      sub={now ? <span className="ipa">/{now.ipa}/</span> : null} />
    <Facts rows={[
      ["Lies in", <Joined items={lake.regions} link={(region) => <LandLink region={region} context={context} />} />],
      ["Drains to", lake.outlet === null ? "No outlet; a closed basin" : <RiverLink id={lake.outlet} context={context} />],
    ]} />
    {view.names.length || view.exonyms.length ? <Leaf id="names" title="Names"
      summary={<p>{view.names.length ? <>First recorded in year <Year generation={view.names[0].since} context={context} />.</> : "Known in other languages."}</p>}>
      <ol className="history">
        {view.names.map((name, i) => <li key={i}>
          <Year generation={name.since} context={context} />
          <span><span className="word">{name.spelled}</span> <span className="ipa">/{name.ipa}/</span>{" "}
            in <LanguageLink variety={name.variety} context={context} />.
            <span className="muted"> {howNamed(name, view.names[i - 1], overview)}{name.once ? `; once ${name.once}` : ""}.</span>
          </span>
        </li>)}
      </ol>
      {view.exonyms.length ? <>
        <h3>What others call it</h3>
        <ul className="roster">{view.exonyms.map((name, i) => <li key={i}>
          <span className="word">{name.spelled}</span> <span className="ipa">/{name.ipa}/</span>{" "}
          in <LanguageLink variety={name.variety} context={context} />, heard in year <Year generation={name.heard} context={context} />
          {name.once ? `; once ${name.once}` : ""}.
        </li>)}</ul>
      </> : null}
    </Leaf> : <p className="muted">No name has been recorded for this lake yet.</p>}
    <StoryLeaf title="The lake's chronicle" annals={subjectHistory({ kind: "lake", id }, overview, map)} context={context} />
  </>;
}

function ZoneCard({ id, context }: { id: number; context: Context }) {
  const { engine, generation, version, map, overview } = context;
  const climate = useMemo(() => engine.climate(generation), [engine, generation, version]);
  const zone = climate.zones.find((zone) => zone.id === id)!;
  const lands = map.climateZones[id].regions;
  return <>
    <CardHead icon={CloudSunRain} kind="A weather zone" title={zoneName(id, context)} />
    <Facts rows={[
      ["Weather", weatherDeparture(zone)],
      ["Lands", <Joined items={lands} link={(region) => <LandLink region={region} context={context} />} />],
    ]} />
    <StoryLeaf title="The weather of these lands" annals={subjectHistory({ kind: "zone", id }, overview, map)} context={context} />
  </>;
}

/// What a land is like, in a few words: "coastal forest, an island".
function terrain(region: number, context: Context): string {
  const r = context.map.regions[region];
  return `${r.coastal ? "coastal " : ""}${TERRAIN_NAME[r.terrain].toLowerCase()}${r.island ? ", an island" : ""}`;
}

/// Several names, each a link, joined as a sentence would join them. A
/// comma stays on the line of the name before it: the links are buttons,
/// and a line may otherwise break between one and its comma.
function Joined<T>({ items, link }: { items: T[]; link: (item: T) => ReactNode }) {
  return (
    <>
      {items.map((item, i) => (
        <Fragment key={i}>
          <span className="joined">
            {link(item)}
            {i < items.length - 2 ? "," : ""}
          </span>
          {i === items.length - 1 ? "" : i === items.length - 2 ? " and " : " "}
        </Fragment>
      ))}
    </>
  );
}

function Year({ generation, context }: { generation: number; context: Context }) {
  return (
    <button type="button" className="gen" title="See the world in this year" onClick={() => context.onScrub(generation)}>
      {generation * YEARS}
    </button>
  );
}


/// Annals as a list of moments, each opening its own card.
function Story({ annals, context }: { annals: Annal[]; context: Context }) {
  return (
    <ol className="history">
      {annals.map((a, i) => (
        <li key={i}>
          <Year generation={a.generation} context={context} />
          <span>
            <button type="button" className="moment" onClick={() => context.go({ kind: "event", id: a.id })}>
              <Told text={a.text} />
            </button>
            <EntryAnnotations annal={a} context={context} />
            <AnnalLinks annal={a} context={context} />
          </span>
        </li>
      ))}
    </ol>
  );
}

/// A section too long or too wide for the card. The card keeps its title
/// and a line about it; the whole of it opens in the folio over the map.
function Leaf({ id, title, summary, children }: { id: string; title: string; summary: ReactNode; children: ReactNode }) {
  const folio = useContext(FolioContext)!;
  const { register } = folio;
  useEffect(() => register(id, title), [register, id, title]);
  const open = folio.open === id;
  if (open && !folio.closing) folio.remember(children);
  return (
    <section className={open ? "leaf open" : "leaf"}>
      <h3>
        <button type="button" className="leaf-title" data-leaf={id} aria-expanded={open} onClick={() => folio.show(open ? null : id)}>
          {title}
          <ChevronRight size={13} aria-hidden="true" />
        </button>
      </h3>
      <div className="leaf-summary">{summary}</div>
      {open && folio.page && !folio.previousCard ? createPortal(<div className="folio-content" key={id}>{children}</div>, folio.page) : null}
    </section>
  );
}

/// The first few of a list, then how many more, which opens the folio.
function Few<T>({ items, link, leaf, max = 3 }: { items: T[]; link: (item: T) => ReactNode; leaf: string; max?: number }) {
  const folio = useContext(FolioContext)!;
  if (items.length <= max + 1) return <Joined items={items} link={link} />;
  return (
    <>
      {items.slice(0, max).map((item, i) => (
        <Fragment key={i}>
          <span className="joined">{link(item)},</span>{" "}
        </Fragment>
      ))}
      and{" "}
      <button type="button" className="link" onClick={() => folio.show(leaf)}>
        {items.length - max} more
      </button>
    </>
  );
}

/// Something's story in the folio, newest first, with its latest moment
/// on the card: the latest headline, or the latest entry of any kind
/// when the story holds nothing louder.
function StoryLeaf({ title, annals, context }: { title: string; annals: Annal[]; context: Context }) {
  const last = annals.findLast((annal) => !QUIET_KINDS.has(annal.kind)) ?? annals.at(-1);
  if (!last) return null;
  return (
    <Leaf
      id="story"
      title={title}
      summary={
        <p>
          Latest, in year <Year generation={last.generation} context={context} />: <Told text={last.text} />
          <EntryAnnotations annal={last} context={context} />
        </p>
      }
    >
      <SubjectTimeline annals={annals} context={context} />
    </Leaf>
  );
}

function ChronicleFilters({ view, update, overview, sounds = false }: {
  view: HistoryView; update: (patch: Partial<HistoryView>) => void; overview: Overview; sounds?: boolean;
}) {
  return <div className="chronicle-tools">
    <label className="chronicle-search">Search the chronicle<input type="search" value={view.query} onChange={(e) => update({ query: e.target.value })} placeholder="A name, a word, a journey…" /></label>
    <Popover label="Entries to show" trigger={(props) => <button type="button" className="phrase" {...props}>Show {view.group.toLowerCase()}</button>}>
      {(close) => <>{HISTORY_GROUPS.map((group) => <button type="button" key={group} aria-pressed={view.group === group} onClick={() => { update({ group }); close(); }}>{group}</button>)}</>}
    </Popover>
    <Popover label="Chronicle order" trigger={(props) => <button type="button" className="phrase" {...props}>{view.order === "newest" ? "Latest first" : "From the beginning"}</button>}>
      {(close) => <>{(["newest", "oldest"] as const).map((order) => <button type="button" key={order} aria-pressed={view.order === order} onClick={() => { update({ order }); close(); }}>{order === "newest" ? "Latest first" : "From the beginning"}</button>)}</>}
    </Popover>
    {sounds ? <Popover label="Sound changes to show" trigger={(props) => <button type="button" className="phrase" {...props}>Sounds: {view.sounds === "all" ? "every language" : view.sounds === "none" ? "none" : overview.varieties[view.sounds]?.name ?? "not yet born"}</button>}>
      {(close) => <><button type="button" onClick={() => { update({ sounds: "all" }); close(); }}>Every language</button><button type="button" onClick={() => { update({ sounds: "none" }); close(); }}>Leave them out</button>
        {overview.varieties.map((v) => <button type="button" key={v.id} onClick={() => { update({ sounds: v.id }); close(); }}>{v.name} and its ancestors</button>)}</>}
    </Popover> : null}
  </div>;
}

function SubjectTimeline({ annals, context }: { annals: Annal[]; context: Context }) {
  const view = context.storyView;
  const update = (patch: Partial<HistoryView>) => context.onStoryView({ ...view, limit: 100, ...patch });
  const lines = filterHistory(annals, context.overview.varieties, view);
  const eras = new Map<number, Annal[]>();
  for (const a of lines.slice(0, view.limit)) {
    const era = Math.floor(a.generation / 20) * 20;
    eras.set(era, [...(eras.get(era) ?? []), a]);
  }
  return <div className="subject-timeline">
    <ChronicleFilters view={view} update={update} overview={context.overview} />
    <p className="muted small" role="status">{lines.length} recorded entries. Names are written as they were known then.</p>
    {[...eras].map(([era, entries], index) => <details className="story-era" key={`${view.order}:${era}`} open={index === 0 ? true : undefined}>
      <summary>Years {era * YEARS}–{(era + 19) * YEARS}<span>{entries.length} {entries.length === 1 ? "entry" : "entries"}</span></summary>
      <Story annals={entries} context={context} />
    </details>)}
    {lines.length > view.limit ? <button type="button" className="chronicle-load" onClick={() => update({ limit: view.limit + 100 })}>Read another {Math.min(100, lines.length - view.limit)} entries</button> : null}
    {!lines.length ? <p className="muted">No recorded entries match these filters.</p> : null}
  </div>;
}

/// A language's specimen, each word opening its own card.
function LanguageSpecimen({ variety, context }: { variety: number; context: Context }) {
  return (
    <Specimen
      words={context.overview.varieties[variety].specimen}
      onWord={(concept) => context.go({ kind: "word", variety, concept })}
    />
  );
}

/// The specimen of every living language side by side, family by family,
/// with words from one root shaded alike: a comparative word list, the
/// table a linguist starts from to find which languages are kin.
function WordsCompared({ spoken, context }: { spoken: Variety[]; context: Context }) {
  const { engine, version, generation, overview } = context;
  const concepts = spoken[0]?.specimen.map((w) => w.concept) ?? [];
  const groups = useMemo(() => {
    const speaker = new Map(overview.communities.map((c) => [c.id, c.variety]));
    return new Map(
      concepts.map((concept) => [
        concept,
        new Map(engine.wordMap(generation, concept).words.map((w) => [speaker.get(w.community), w.group])),
      ]),
    );
    // `version` changes whenever the history does.
  }, [engine, generation, version, concepts.join()]);
  const rows = [...spoken].sort((a, b) => a.family - b.family || a.id - b.id);
  return (
    <div className="compared">
      <table>
        <thead>
          <tr>
            <th />
            {concepts.map((c) => (
              <th key={c} scope="col">
                {c}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((v) => (
            <tr key={v.id}>
              <th scope="row" style={{ color: hue(v.family) }}>
                <LanguageLink variety={v.id} context={context} />
              </th>
              {concepts.map((concept) => {
                const w = v.specimen.find((s) => s.concept === concept);
                const group = groups.get(concept)?.get(v.id);
                return (
                  <td key={concept} style={group === undefined ? undefined : ({ "--root": hue(group) } as CSSProperties)}>
                    {w ? (
                      <button
                        type="button"
                        className="link word"
                        title={`/${w.ipa}/`}
                        onClick={() => context.go({ kind: "word", variety: v.id, concept })}
                      >
                        {w.spelled}
                      </button>
                    ) : null}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}


function WorldCard({ context }: { context: Context }) {
  const { overview } = context;
  const peoples = overview.communities.filter((c) => c.ended === null);
  const spoken = overview.varieties.filter((v) => v.spoken);
  const crafts = overview.crafts.filter((c) => c.first !== null);
  const continent = overview.continents.find((c) => c.name)?.name?.name ?? context.title ?? "these lands";
  const last = overview.annals.at(-1);
  return <>
    <CardHead icon={Globe} kind="A chart of" title={context.title ?? continent} />
    <p className="frontispiece-caption">{overview.generation === 0 ? `${peoples.length === 1 ? "One people settles" : `${peoples.length} peoples settle`} ${continent}.`
      : last ? <><Told text={last.text} /><EntryAnnotations annal={last} context={context} /></> : `Year ${overview.generation * YEARS}.`}</p>
    <ul className="frontispiece-peoples">{peoples.map((c) => <li key={c.id}><PeopleLink c={c} context={context} /></li>)}</ul>
    <button type="button" className="primary frontispiece-run" disabled={!overview.atTip || context.running} onClick={context.onRun}><Play size={15} aria-hidden="true" /> Run</button>
    {overview.generation > 0 ? <p><button type="button" className="link" onClick={() => context.go({ kind: "history" })}>The chronicle</button></p> : null}
    <Leaf id="crafts" title="Crafts" summary={<p>{crafts.length ? <Joined items={crafts} link={(craft) => <CraftLink craft={craft.id} context={context} />} /> : <span className="muted">No people has a craft yet.</span>}</p>}>
      <table className="peoples"><thead><tr><th>Craft</th><th>First held</th><th>Holders</th></tr></thead><tbody>
        {overview.crafts.map((craft) => <tr key={craft.id}><th scope="row"><CraftLink craft={craft.id} context={context} /></th>
          <td>{craft.first === null ? "not yet" : <Year generation={craft.first} context={context} />}</td><td className="num">{craft.holders.length}</td></tr>)}
      </tbody></table>
    </Leaf>
    {spoken.length > 1 ? <Leaf id="compared" title="Words compared" summary={<p>Basic words across {spoken.length} living languages, shaded by shared root.</p>}>
      <p className="muted small">Words shaded alike come from one root: they are <Explained term="cognate">cognates</Explained>.</p>
      <WordsCompared spoken={spoken} context={context} />
    </Leaf> : null}
    <FamilyTrees context={context} />
  </>;
}

/// Every family of more than one language, each as a chart of descent.
function FamilyTrees({ context }: { context: Context }) {
  const { overview } = context;
  const families = [...new Set(overview.varieties.map((v) => v.family))].filter(
    (f) => overview.varieties.filter((v) => v.family === f && v.born <= overview.generation).length > 1,
  );
  if (families.length === 0) return null;
  return (
    <Leaf
      id="families"
      title="Family trees"
      summary={
        <p>
          {families.length} <Explained term="family">{families.length === 1 ? "family" : "families"}</Explained> of more
          than one language, each drawn as a chart of descent.
        </p>
      }
    >
      <FamilyForest
        overview={overview}
        families={families}
        onOpen={(id) => context.go({ kind: "language", variety: id })}
      />
    </Leaf>
  );
}

/// Entries keep their evidence and links whether full-weight or unfolded.
function ChronicleEvents({ annals, context }: { annals: Annal[]; context: Context }) {
  return <ol className="chronicle-events">{annals.map((annal) => {
    const kind = EVENT_KIND[annal.kind];
    return <li key={annal.id}>
      <kind.icon size={17} aria-hidden="true" />
      <div><span className="event-kind">{kind.name}</span>
        <button type="button" className="moment" onClick={() => context.go({ kind: "event", id: annal.id })}>{annal.decision !== undefined ? <span className="pen" title="The author's decision"><Feather size={12} aria-hidden="true" /></span> : null}<Told text={annal.text} /></button>
        <EntryAnnotations annal={annal} context={context} />
        <AnnalLinks annal={annal} context={context} />
        {annal.specimen.length ? <Specimen words={annal.specimen} changes={annal.laws.length > 0} onWord={(concept) => { if (annal.variety !== null) context.go({ kind: "word", variety: annal.variety, concept }); }} /> : null}
      </div>
    </li>;
  })}</ol>;
}

function EraSubjects({ era, context }: { era: Era; context: Context }) {
  const { overview } = context;
  const subjects = new Map<string, ReactNode>();
  for (const year of era.years) for (const annal of year.headlines) {
    for (const id of annal.peoples) if (overview.communities[id]) subjects.set(`people-${id}`, <PeopleLink c={overview.communities[id]} context={context} />);
    for (const id of annal.states) if (overview.states[id]) subjects.set(`state-${id}`, <StateLink state={overview.states[id]} context={context} />);
    for (const id of annal.languages) if (overview.varieties[id]) subjects.set(`language-${id}`, <LanguageLink variety={id} context={context} />);
  }
  return <span className="chronicle-era-subjects"><Joined items={[...subjects.values()].slice(0, 4)} link={(link) => link} />{subjects.size > 4 ? ` and ${subjects.size - 4} more` : null}</span>;
}

/// The card is the contents; the wide card reads the same eras in detail.
function HistoryCard({ context }: { context: Context }) {
  const { overview, historyView: view, onHistoryView } = context;
  const folio = useContext(FolioContext)!;
  const [destination, setDestination] = useState<number | null>(null);
  const update = (patch: Partial<HistoryView>) => onHistoryView({ ...view, limit: 100, ...patch });
  const contents = useMemo(() => eras(overview.annals, overview.generation, overview), [overview]);
  const lines = useMemo(() => filterHistory(overview.annals, overview.varieties, view), [overview, view]);
  const shown = lines.slice(0, view.limit);
  const selected = new Set(shown.map((a) => a.id));
  const unfold = view.group !== "All events" || view.query.trim() !== "" || view.sounds !== "all";
  const ordered = view.order === "newest" ? [...contents].reverse() : contents;
  const headlineCount = contents.reduce((sum, era) => sum + era.years.reduce((n, year) => n + year.headlines.length, 0), 0);
  const quietYears = contents.reduce((sum, era) => sum + era.years.filter((year) => year.quiet.length).length, 0);
  const missingLanguage = typeof view.sounds === "number" && !overview.varieties[view.sounds];
  useEffect(() => {
    if (destination === null || folio.open !== "history" || !folio.page) return;
    const frame = requestAnimationFrame(() => {
      const header = folio.page?.querySelector<HTMLElement>(`[data-chronicle-era="${destination}"]`);
      const section = header?.closest<HTMLElement>(".chronicle-era");
      if (header && section && folio.page) {
        folio.page.scrollTo({ top: folio.page.scrollTop + section.getBoundingClientRect().top - folio.page.getBoundingClientRect().top });
        header.focus({ preventScroll: true });
        setDestination(null);
      }
    });
    return () => cancelAnimationFrame(frame);
  }, [destination, folio.open, folio.page, view.limit]);
  const openEra = (era: Era) => {
    const all = filterHistory(overview.annals, overview.varieties, { ...INITIAL_HISTORY, order: view.order });
    const last = all.reduce((index, annal, i) => annal.generation >= era.start && annal.generation <= era.end ? i : index, -1);
    onHistoryView({ ...INITIAL_HISTORY, order: view.order, limit: Math.max(100, last + 1) });
    setDestination(era.start);
    folio.show("history");
  };
  return (
    <>
      <CardHead icon={ScrollText} kind={overview.tellings.find((t) => t.id === overview.telling)?.name ?? "This telling"} title="The chronicle" />
      <ol className="chronicle-contents" aria-label="The chronicle's eras">{contents.map((era) => <li key={era.start}>
        <button type="button" className="chronicle-contents-row" title={era.title} onClick={() => openEra(era)}>
          <span className="chronicle-era-span">{era.start * YEARS}–{era.end * YEARS}</span>
          <span>{era.heading}</span>
        </button>
        <EraSubjects era={era} context={context} />
      </li>)}</ol>
      <Leaf id="history" title="Entries" summary={
        <p>{headlineCount.toLocaleString()} headline {headlineCount === 1 ? "entry" : "entries"} and {quietYears.toLocaleString()} quiet {quietYears === 1 ? "year" : "years"}.</p>
      }>
        <ChronicleFilters view={view} update={update} overview={overview} sounds />
        <div className="chronicle-count"><span role="status">{lines.length.toLocaleString()} {lines.length === 1 ? "entry" : "entries"}{unfold ? " matching these filters" : " in this telling"}</span>
          <button type="button" className="link" onClick={() => onHistoryView(INITIAL_HISTORY)}>Reset filters</button>
        </div>
        {missingLanguage ? <p className="muted small">The selected language has not arisen in this year. Its sound changes will appear when you return to its time.</p> : null}
        {lines.length === 0 ? <p className="index-empty">No entries match these filters. Try a different name or broaden the filters.</p> : null}
        <div className="chronicle-years">
          {ordered.map((era) => {
            const years = (view.order === "newest" ? [...era.years].reverse() : era.years).flatMap((year) => {
              const matching = shown.filter((annal) => annal.generation === year.generation);
              if (!matching.length) return [];
              return [{ generation: year.generation, headlines: unfold ? matching : year.headlines.filter((a) => selected.has(a.id)), quiet: unfold ? [] : year.quiet.filter((a) => selected.has(a.id)) }];
            });
            if (!years.length) return null;
            return <section className="chronicle-era" key={era.start}>
              <h3 className="chronicle-era-head" data-chronicle-era={era.start} tabIndex={-1}>{era.opening ? <>Year {era.start * YEARS} · <Told text={era.opening.text} /></> : era.title}</h3>
              {years.map((year) => <section className="chronicle-year-group" key={year.generation}>
                <h4><button type="button" className="link" title="See the world in this year" onClick={() => context.onScrub(year.generation)}>Year {year.generation * YEARS}</button></h4>
                <div>
                  {year.headlines.length ? <ChronicleEvents annals={year.headlines} context={context} /> : null}
                  {year.quiet.length ? <details className="chronicle-quiet" key={`${view.order}:${year.generation}`}>
                    <summary>{quietLine(year, overview)}</summary>
                    <ChronicleEvents annals={year.quiet} context={context} />
                  </details> : null}
                </div>
              </section>)}
            </section>;
          })}
        </div>
        {lines.length > shown.length ? <button type="button" className="chronicle-load" onClick={() => update({ limit: view.limit + 100 })}>Read another {Math.min(100, lines.length - shown.length)} entries</button> : null}
      </Leaf>
      <Leaf id="tellings" title="Other tellings" summary={<p>{overview.tellings.length} {overview.tellings.length === 1 ? "telling" : "tellings"} kept. Name them, read them, or compare the same year.</p>}>
        <section className="other-tellings">
          <p className="muted small">Each telling keeps its name, its decisions, and its own future. Read freely; deciding in the past begins a new telling.</p>
          {overview.tellings.map((telling) => <article key={telling.id} className={`telling-entry${telling.id === overview.telling ? " current" : ""}`}>
            <label>Name of this telling<input aria-label={`Name of ${telling.name}`} defaultValue={telling.name} key={telling.name}
              onBlur={(e) => { if (e.target.value.trim() !== telling.name) context.onRenameTelling(telling.id, e.target.value); }}
              onKeyDown={(e) => { if (e.key === "Enter") e.currentTarget.blur(); }} maxLength={120} /></label>
            <p className="small">{telling.id === overview.telling ? "This telling · " : ""}Through year {telling.latest * YEARS}
              {telling.parent ? ` · from ${overview.tellings.find((t) => t.id === telling.parent?.telling)?.name ?? "an earlier telling"}, year ${(telling.from ?? 0) * YEARS}` : " · the first telling"}</p>
            <div className="row">{telling.id !== overview.telling ? <><button type="button" onClick={() => context.onRestore(telling.id)}>Read this telling</button>
              <button type="button" onClick={() => context.onCompare(telling.id)}>Compare these tellings</button></> : null}</div>
          </article>)}
        </section>
      </Leaf>
    </>
  );
}

/// A people's temper: each leaning a ruled line between its two ends,
/// marked where the people stands, the end it leans to in darker ink.
function TemperScales({ ethos }: { ethos: Ethos }) {
  return (
    <ul className="temper">
      {ETHOS_AXES.map((axis) => {
        const value = ethos[axis];
        const [low, high] = ETHOS_POLES[axis];
        const leans = Math.abs(value) >= STRONG ? (value > 0 ? "high" : "low") : null;
        return (
          <li key={axis}>
            <span className={leans === "low" ? "pole low leans" : "pole low"}>{low}</span>
            <span className="temper-scale" role="meter" aria-label={`${low} to ${high}`}
              aria-valuemin={-1} aria-valuemax={1} aria-valuenow={value}>
              <i style={{ left: `${(value + 1) * 50}%` }} />
            </span>
            <span className={leans === "high" ? "pole leans" : "pole"}>{high}</span>
          </li>
        );
      })}
    </ul>
  );
}

/// A people's temper on its card, and in the folio, each time it turned.
function TemperLeaf({ c, told, context }: { c: Community; told: Annal[]; context: Context }) {
  const turns = told.filter((a) => a.kind === "temper" && a.peoples[0] === c.id);
  if (turns.length === 0) {
    return (
      <>
        <h3><Explained term="temper">Temper</Explained></h3>
        <TemperScales ethos={c.ethos} />
      </>
    );
  }
  return (
    <Leaf
      id="temper"
      title="Temper"
      summary={
        <>
          <TemperScales ethos={c.ethos} />
          <p>
            It turned {turns.length === 1 ? "once" : turns.length === 2 ? "twice" : `${turns.length} times`}, last in year{" "}
            <Year generation={turns.at(-1)!.generation} context={context} />.
          </p>
        </>
      }
    >
      <Story annals={[...turns].reverse()} context={context} />
    </Leaf>
  );
}

function Decisions({ community, choices, context, disabled = false, label = "Decide for this people" }: { community: number; choices: [InterventionKind, string][]; context: Context; disabled?: boolean; label?: string }) {
  return <Popover label={label} trigger={(props) => <button type="button" className="decide" disabled={context.running || disabled} {...props}>Decide…</button>}>
    {(close) => <>{choices.map(([kind, label]) => <button type="button" key={kind} onClick={() => { close(); context.onDialog(kind, community); }}>{label}</button>)}</>}
  </Popover>;
}

function PeopleCard({ c, context }: { c: Community; context: Context }) {
  const { overview } = context;
  const name = (id: number) => overview.communities[id];
  const v = overview.varieties[c.variety];
  const phrases = peoplePhrases(c.ended);
  const contacts = overview.contacts.filter((k) => k.a === c.id || k.b === c.id);
  const other = (k: (typeof contacts)[number]) => name(k.a === c.id ? k.b : k.a);
  const kinds = [...new Set(contacts.map((k) => k.kind))];
  const moves = overview.moves.filter((m) => m.community === c.id);
  const told = subjectHistory({ kind: "people", id: c.id }, overview, context.map);
  const realm = overview.states.find((s) => s.fell === null &&
    (s.rulers === c.id || s.members.some((m) => m.community === c.id && m.left === null)));
  return (
    <>
      <CardHead
        icon={Users}
        kind="A people"
        title={c.name}
        tone={hue(v.family)}
        hand={v.family}
        sub={
          <>
            “{c.meaning}” <span className="ipa">/{c.ipa}/</span>
            {c.once ? `, once ${c.once}` : ""}
          </>
        }
      />
      {c.ended === null ? <Decisions community={c.id} context={context} choices={[
        ["settlement", "Settle, divide, or move"], ["connect", "Meet another people"], ["shift", "Take up another language"],
        ...(!realm ? [["state", "Found a state"] as [InterventionKind, string]] : []),
        ["religion", FAITH_NAME.action],
        ...(c.crafts.length < context.catalog.crafts.length ? [["craft", "Teach a craft"] as [InterventionKind, string]] : []),
        ["temper", "Their temper turns"],
      ]} /> : null}
      <section className="people-land">
        <h3>{phrases.land}</h3>
        <p>{phrases.homeland} <LandLink region={c.region} context={context} />, {terrain(c.region, context)}.</p>
        <p>Lands: <Joined items={c.lands} link={(region) => <LandLink region={region} context={context} />} />.</p>
      </section>
      <section>
        <h3>{phrases.livelihood}</h3>
        <p><Explained term="way of life">{LIVELIHOOD_NAME[c.livelihood]}</Explained> · {Math.round(c.size).toLocaleString()} souls.</p>
      </section>
      <p className="people-temper"><Explained term="temper">{temperament(c.ethos).join(", ") || "Even-tempered"}</Explained>.</p>
      <TemperLeaf c={c} told={told} context={context} />
      <StoryLeaf title="Their fortunes" annals={told} context={context} />
      <section>
        <h3>{phrases.speech}</h3>
        <p>{phrases.lastSpeech}<LanguageLink variety={c.variety} context={context} /></p>
        <LanguageSpecimen variety={c.variety} context={context} />
      </section>
      <KnownWorldLeaf variety={c.variety} context={context} />
      <h3>Kin</h3>
      <Facts rows={[
        ["Came from", c.parents.length > 0 ? <Joined items={c.parents} link={(id) => <PeopleLink c={name(id)} context={context} />} /> : "a founding people"],
        ["Peoples descended", overview.communities.some((p) => p.parents.includes(c.id)) ? <Joined items={overview.communities.filter((p) => p.parents.includes(c.id))} link={(p) => <PeopleLink c={p} context={context} />} /> : null],
        ["Since", told.length > 0 ? `year ${told[0].generation * YEARS}` : null],
        ["Ended", c.ended === null ? null : <>year <Year generation={c.ended} context={context} />, {c.endedInto === null ? "died out" : <>merged into the <PeopleLink c={name(c.endedInto)} context={context} /></>}</>],
        ["Language family", v.family === c.variety ? null : <LanguageLink variety={v.family} context={context} />],
        [realm?.rulers === c.id ? "Rule" : "Subject of", realm ? <StateLink state={realm} context={context} /> : null],
      ]} />
      {contacts.length > 0 ? (
        <Leaf
          id="dealings"
          title="Dealings"
          summary={
            <p>
              {kinds.map((kind, i) => (
                <Fragment key={kind}>
                  {i > 0 ? "; " : ""}
                  {CONTACT_NAME[kind]}:{" "}
                  <Few
                    items={contacts.filter((k) => k.kind === kind)}
                    link={(k) => <PeopleLink c={other(k)} context={context} />}
                    leaf="dealings"
                  />
                </Fragment>
              ))}
              .
            </p>
          }
        >
          <ul className="roster">
            {contacts.map((k, i) => (
              <li key={i}>
                They {bond(k.kind, k.intensity)} the <PeopleLink c={other(k)} context={context} />{" "}
                <span className="muted">({CONTACT_NAME[k.kind].toLowerCase()})</span>
              </li>
            ))}
          </ul>
        </Leaf>
      ) : (
        <>
          <h3>Dealings</h3>
          <p className="muted">They deal with no one.</p>
        </>
      )}
      {moves.length > 0 ? (
        <Leaf
          id="wanderings"
          title="Wanderings"
          summary={
            <p>
              {moves.length === 1 ? "Once" : `${moves.length} times`}, last to{" "}
              <LandLink region={moves.at(-1)!.to} context={context} /> in year{" "}
              <Year generation={moves.at(-1)!.generation} context={context} />.
            </p>
          }
        >
          <ol className="history">
            {moves.map((m, i) => (
              <li key={i}>
                <Year generation={m.generation} context={context} />
                <span>
                  {m.kind === "split" ? "Went out to" : "Moved to"} <LandLink region={m.to} context={context} />
                  {m.overseas ? " over the sea" : ""}
                </span>
              </li>
            ))}
          </ol>
        </Leaf>
      ) : null}
      <h3>Faith and crafts</h3>
      <Facts rows={[
        ["Faith", c.faith === null ? "its own gods" : <ReligionLink religion={overview.religions[c.faith]} context={context} />],
        ["Crafts", c.crafts.length === 0 ? "none yet" : <Joined items={c.crafts} link={(craft) => <CraftLink craft={craft} context={context} />} />],
      ]} />
      {c.exonyms.length ? <section><h3>Called by others</h3><p><Joined items={c.exonyms} link={(e) => <><span className="word">{e.name}</span> by the <PeopleLink c={name(e.by)} context={context} /></>} /></p></section> : null}
    </>
  );
}

function StateCard({ state, context }: { state: StateView; context: Context }) {
  const { overview } = context;
  const rulers = overview.communities[state.rulers];
  const standard = overview.varieties.find((v) => v.standardOf === state.id);
  const current = state.fell === null ? state.members.filter((m) => m.left === null) : [];
  const former = state.members.filter((m) => m.left !== null);
  const told = subjectHistory({ kind: "state", id: state.id }, overview, context.map);
  return (
    <>
      <CardHead
        icon={Landmark}
        kind={state.fell === null ? "A standing state" : "A fallen state"}
        title={state.name}
        tone={hue(state.id)}
        hand={overview.varieties[rulers.variety].family}
        sub={<>“{state.meaning}” <span className="ipa">/{state.ipa}/</span>{state.once ? `, once ${state.once}` : ""}</>}
      />
      {state.fell === null && rulers.ended === null ? <Decisions community={rulers.id} context={context} choices={[["religion", FAITH_NAME.action], ["craft", "Teach a craft"], ["temper", "Their temper turns"]]} /> : null}
      <Facts rows={[
        ["Rulers", <PeopleLink c={rulers} context={context} />],
        ["Founder", <><span className="word">{state.founder.name}</span>, “{state.founder.meaning}” <span className="ipa">/{state.founder.ipa}/</span></>],
        ["Capital", <LandLink region={state.capital} context={context} />],
        ["City", <GreatCity state={state} context={context} />],
        ["Arose", <>year <Year generation={state.rose} context={context} />, {RISE_NAME[state.rise]}</>],
        ["Fell", state.fell === null ? null : (
          <>year <Year generation={state.fell} context={context} />{state.fall ? `, ${FALL_NAME[state.fall]}` : ""}
            {state.fallenTo !== null ? <> by the <PeopleLink c={overview.communities[state.fallenTo]} context={context} /></> : null}
          </>
        )],
        ["Standard", state.standard === null ? (state.fell === null ? "no court standard yet" : "no court standard arose") : (
          <>{standard ? <LanguageLink variety={standard.id} context={context} /> : "No named court standard recorded"}, since year <Year generation={state.standard} context={context} /></>
        )],
        ["Purism", state.standard === null ? null : (
          <Explained term="purism">{state.purism >= 0.5 ? "guarded against foreign words" : "open to foreign words"}</Explained>
        )],
        ["Classical form", state.classical === null ? null : (
          <><LanguageLink variety={state.classical.variety} context={context} />, fixed in year{" "}
            <Year generation={state.classical.fixed} context={context} />{" "}
            {state.classical.how === "age" ? "by grammarians" : state.classical.how === "purism" ? "by purists" : "when the state fell"}</>
        )],
        ["Current subjects", current.length > 0 ? (
          <Joined items={current} link={(m) => <PeopleLink c={overview.communities[m.community]} context={context} />} />
        ) : "none"],
        ["Former subjects", former.length === 0 ? null : (
          <Joined items={former} link={(m) => <>
            <PeopleLink c={overview.communities[m.community]} context={context} />{" "}
            (years <Year generation={m.joined} context={context} />–<Year generation={m.left!} context={context} />)
          </>} />
        )],
      ]} />
      <p className="muted small">
        A <Explained term="state">state</Explained> gathers tribute into its capital city.
        {state.standard === null ? null : <> Its court’s <Explained term="standard language">standard language</Explained>{" "}
          {state.fell === null ? "draws" : "drew"} kindred dialects towards it through <Explained term="dialect levelling">dialect levelling</Explained>.</>}
      </p>
      <StoryLeaf title="Its story" annals={told} context={context} />
    </>
  );
}

/// A state's capital city: its number, and once great, its name and
/// whose speech fills its streets.
function GreatCity({ state, context }: { state: StateView; context: Context }) {
  const { overview } = context;
  const city = overview.cities.findLast((c) => c.state === state.id);
  const souls = `${Math.round(city?.size ?? state.city).toLocaleString()} souls`;
  if (!city) return <>{souls}</>;
  const townsfolk = city.townsfolk === null ? null : overview.communities[city.townsfolk];
  return (
    <>
      <span className="word">{city.name.name}</span>, “{city.name.meaning}”, {souls}, great since year{" "}
      <Year generation={city.since} context={context} />; its people speak{" "}
      <Joined items={city.makeup.slice(0, 3)}
        link={(m) => <><LanguageLink variety={m.variety} context={context} /> {Math.round(m.share * 100)}%</>} />
      {townsfolk ? <>; its townsfolk, the <PeopleLink c={townsfolk} context={context} />, speak a{" "}
        <Explained term="koiné">koiné</Explained> of their own</> : null}
    </>
  );
}

function ReligionCard({ religion, context }: { religion: ReligionView; context: Context }) {
  const { overview } = context;
  const told = individualAnnals(overview.annals).filter((a) => a.religions.includes(religion.id) &&
    (a.kind === "faith" || a.kind === "conversion" || a.kind === "meaning" ||
      a.kind === "schism" || a.kind === "pilgrimage" || a.kind === "holy-land"));
  const parent = religion.parent === null ? null : overview.religions[religion.parent];
  return (
    <>
      <CardHead icon={Sparkles} kind={parent ? "A branch of a faith" : FAITH_NAME.kind} title={<i>{religion.name}</i>}
        tone={hue(religion.id)}
        sub={<>“{religion.meaning}” <span className="ipa">/{religion.ipa}/</span></>} />
      {overview.communities[religion.people]?.ended === null ? <Decisions community={religion.people} context={context} choices={[["connect", "Meet another people"], ["craft", "Teach a craft"], ["temper", "Their temper turns"]]} /> : null}
      <Facts rows={[
        ["Founder", <><span className="word">{religion.founder.name}</span>, “{religion.founder.meaning}” <span className="ipa">/{religion.founder.ipa}/</span></>],
        parent && religion.split !== null && religion.cause !== null
          ? ["Broke from", <><ReligionLink religion={parent} context={context} /> in year <Year generation={religion.split} context={context} />, {SCHISM_CAUSE[religion.cause]}</>]
          : ["Founded", <>year <Year generation={religion.founded} context={context} />, {FAITH_HOW[religion.how]}</>],
        [parent ? "Begun among" : "Founder's people", <PeopleLink c={overview.communities[religion.people]} context={context} />],
        [parent ? "Where it began" : "Founding land", <LandLink region={religion.land} context={context} />],
        ["Branches", religion.branches.length === 0 ? null : (
          <Joined items={religion.branches} link={(id) => <ReligionLink religion={overview.religions[id]} context={context} />} />
        )],
        ...religion.shrines.map((shrine, i): [string, ReactNode] => [i === 0 ? "Holy place" : "Also holy", <>
          <span className="word">{shrine.name.name}</span>, “{shrine.name.meaning}”, {i > 0 && shrine.kind === "home" ? "the land where it began" : SHRINE_KIND[shrine.kind]}:{" "}
          <LandLink region={shrine.region} context={context} /> (<LandmassOf region={shrine.region} context={context} />),{" "}
          <HolyLandKeeper held={religion.holyLand.find((h) => h.region === shrine.region)} context={context} />
        </>]),
        ["Sacred language", <LanguageLink variety={religion.sacred} context={context} />],
        ["Converts", religion.converts ? "seeks converts" : "keeps to its own"],
        ["Words", religion.translates ? "followers translate its words" : "followers borrow its words"],
        ["Scripture", religion.scripture ? "written teaching; brings writing" : "unwritten teaching"],
        ["Teaching", faithTeaching(religion)],
        ["Parted over", religion.disputed === null ? null : TENET_NOUN[religion.disputed]],
      ]} />
      <p className="muted small">Its <Explained term="sacred language">sacred language</Explained> keeps the founder’s words and sounds.</p>
      <h3>Followers</h3>
      {religion.followers.length === 0 ? <p className="muted">No living people holds this faith.</p> : (
        <p><Joined items={religion.followers} link={(id) => <PeopleLink c={overview.communities[id]} context={context} />} /></p>
      )}
      {religion.pilgrims.length > 0 ? (
        <Leaf
          id="pilgrims"
          title="Pilgrim roads"
          summary={
            <p>
              <Explained term="pilgrimage">Pilgrims</Explained> come from{" "}
              <Few items={religion.pilgrims} leaf="pilgrims"
                link={(p) => <PeopleLink c={overview.communities[p.people]} context={context} />} />.
            </p>
          }
        >
          <ul className="roster">
            {religion.pilgrims.map((p) => (
              <li key={`${p.people}-${p.to}`}>
                <PeopleLink c={overview.communities[p.people]} context={context} /> from{" "}
                <LandLink region={p.from} context={context} /> to <LandLink region={p.to} context={context} />,{" "}
                {p.path.length - 1 === 1 ? "next door" : `${p.path.length - 1} lands on the road`},{" "}
                <small className="muted">since year <Year generation={p.since} context={context} /></small>
              </li>
            ))}
          </ul>
          <p className="muted small">Pilgrims meet speakers of other tongues on the road and at the shrine, and carry words and sounds home.</p>
        </Leaf>
      ) : null}
      <FaithTreeLeaf religion={religion} context={context} />
      <RenderingsLeaf title="Words of the faith" rows={religion.words} sacred={religion.sacred} context={context}>
        A <Explained term="learned word">learned word</Explained> may return from the sacred language beside its older descendant,
        making a <Explained term="doublet">doublet</Explained>. Older gods may become demons through <Explained term="pejoration">pejoration</Explained>.
      </RenderingsLeaf>
      <StoryLeaf title="Its story" annals={told} context={context} />
    </>
  );
}

/// Who keeps a holy place now.
function HolyLandKeeper({ held, context }: { held: HolyLand | undefined; context: Context }) {
  if (!held || held.heldBy === null) return <>where no one lives now</>;
  const keeper = context.overview.communities[held.heldBy];
  return held.faithful
    ? <>kept by the faithful, the <PeopleLink c={keeper} context={context} /></>
    : <>held by the <PeopleLink c={keeper} context={context} />, who do not keep it</>;
}

/// The faith a branch descends from at the root, and every branch of it,
/// drawn as a chart of descent.
function FaithTreeLeaf({ religion, context }: { religion: ReligionView; context: Context }) {
  const { overview } = context;
  const rootOf = (r: ReligionView): ReligionView => (r.parent === null ? r : rootOf(overview.religions[r.parent]));
  const root = rootOf(religion);
  const members = overview.religions.filter((r) => rootOf(r).id === root.id);
  if (members.length < 2) return null;
  const lineages: Lineage[] = members.map((r) => ({
    id: r.id,
    name: r.name,
    parent: r.parent,
    born: r.split ?? r.founded,
    // A faith no living people holds is drawn faded, up to now.
    ended: r.followers.length === 0 ? overview.generation : null,
    tone: hue(r.id),
    label: `${r.name}: ${r.parent === null ? "founded" : "broke away"} in year ${(r.split ?? r.founded) * YEARS}${
      r.followers.length === 0 ? ", held by no one now" : ""}`,
  }));
  return (
    <Leaf
      id="faith-tree"
      title="Its branches"
      summary={
        <p>
          A chart of the {members.length} faiths that descend from <ReligionLink religion={root} context={context} />,
          through each <Explained term="schism">schism</Explained>.
        </p>
      }
    >
      <DescentChart lineages={lineages} now={overview.generation} chosen={religion.id} what="faiths"
        onOpen={(id) => context.go({ kind: "religion", id })} />
    </Leaf>
  );
}

function CraftCard({ craft, context }: { craft: CraftView; context: Context }) {
  const { overview } = context;
  const told = subjectHistory({ kind: "craft", id: craft.id }, overview, context.map);
  return (
    <>
      <CardHead icon={Hammer} kind="A craft" title={craft.name} />
      <p>{context.catalog.crafts.find((c) => c.id === craft.id)?.description}</p>
      <Facts rows={[
        ["First held", craft.first === null ? "not yet" : <>year <Year generation={craft.first} context={context} /></>],
        ["Inventors", craft.inventors.length === 0 ? "none yet" :
          <Joined items={craft.inventors} link={(id) => <PeopleLink c={overview.communities[id]} context={context} />} />],
        ["Holders", craft.holders.length === 0 ? "no living people" :
          <Joined items={craft.holders} link={(id) => <PeopleLink c={overview.communities[id]} context={context} />} />],
      ]} />
      <RenderingsLeaf title="Words of the craft" rows={craft.words} context={context}>
        New meanings may use older words, much like <Explained term="meaning extension by livelihood">meanings shaped by a way of life</Explained>.
      </RenderingsLeaf>
      <StoryLeaf title="Its story" annals={told} context={context} />
    </>
  );
}

/// The words languages found for a faith's or a craft's meanings, and how.
function RenderingsLeaf({ title, rows, sacred, context, children }: {
  title: string;
  rows: RenderingRow[];
  sacred?: number;
  context: Context;
  children: ReactNode;
}) {
  const languages = new Set(rows.flatMap((row) => row.renderings.map((r) => r.variety))).size;
  return (
    <Leaf
      id="words"
      title={title}
      summary={
        rows.length === 0 ? (
          <p className="muted">No words yet.</p>
        ) : (
          <p>
            {rows.length} {rows.length === 1 ? "meaning" : "meanings"} in {languages}{" "}
            {languages === 1 ? "language" : "languages"}, and how each found its word.
          </p>
        )
      }
    >
      <Renderings
        rows={rows}
        overview={context.overview}
        sacred={sacred}
        onLanguage={(variety) => context.go({ kind: "language", variety })}
        onWord={(variety, concept) => context.go({ kind: "word", variety, concept })}
      />
      <p className="muted small">{children}</p>
    </Leaf>
  );
}

function LanguageCard({ variety, context }: { variety: number; context: Context }) {
  const v = context.overview.varieties[variety];
  const speakers = context.overview.communities.filter((c) => c.ended === null && c.variety === variety);
  const ctx = languageChapterContext(context, variety);
  return <div className="language-card">
    <CardHead icon={Languages} kind="A language" title={v.name} tone={hue(v.family)} hand={v.family} sub={v.meaning ? `“${v.meaning}”` : null} />
    <Decisions community={speakers[0]?.id ?? -1} context={context} choices={[["law", "A sound change"]]} disabled={!speakers.length} label="Decide for this language" />
    <LanguageSpecimen variety={variety} context={context} />
    <Facts rows={[
      ["Spoken by", speakers.length ? <Joined items={speakers} link={(c) => <PeopleLink c={c} context={context} />} /> : "no one now"],
      ["Standard of", v.standardOf === null ? null : <StateLink state={context.overview.states[v.standardOf]} context={context} />],
      ["Sacred to", v.sacredOf === null ? null : <ReligionLink religion={context.overview.religions[v.sacredOf]} context={context} />],
      ["Classical of", v.classicalOf === null ? null : <StateLink state={context.overview.states[v.classicalOf]} context={context} />],
      ["Writing", v.high !== null && v.vernacular === null ? <LanguageLink variety={v.high} context={context} /> : v.written === null ? "unwritten" : `written or last respelled in year ${v.written * YEARS}`],
    ]} />
    {CHAPTER.map(({ id, title, Section }) => <Leaf key={id} id={id} title={title} summary={<ChapterSummary variety={v} section={id} ctx={ctx} />}>
      <Section variety={v} ctx={ctx} />
    </Leaf>)}
  </div>;
}


function languageChapterContext(context: Context, variety: number): ChapterContext {
  return {
    engine: context.engine, version: context.version, generation: context.generation,
    overview: context.overview, map: context.map,
    link: (subject, label) => <button type="button" className="link" onClick={() => context.go(subject)}>{label}</button>,
    open: context.go, onYear: context.onScrub,
    dictionaryView: context.dictionaryViews[variety] ?? INITIAL_DICTIONARY,
    onDictionaryView: (view) => context.onDictionaryView(variety, view),
  };
}

function WordCard({ variety, concept, context }: { variety: number; concept: string; context: Context }) {
  const { engine, version, generation, overview, words } = context;
  const groups = useMemo(() => {
    if (!words) return [];
    const seen = new Map<number, string[]>();
    for (const w of words.words) seen.set(w.group, [...(seen.get(w.group) ?? []), w.spelled]);
    return [...seen.entries()].map(([group, forms]) => ({ group, forms: [...new Set(forms)] }));
  }, [words]);
  const v = overview.varieties[variety];
  const wordStory = useMemo(() => {
    try {
      const { annals } = engine.story(generation, { kind: "word", variety, concept });
      return annals.map((id) => findAnnal(overview.annals, id)).filter((a): a is Annal => a !== undefined);
    } catch {
      return [];
    }
  }, [engine, version, generation, variety, concept, overview]);
  if (!v) return <p className="muted">This language has not yet arisen in this year.</p>;
  return (
    <>
      <CardHead
        icon={WholeWord}
        kind="A word"
        title={`“${words?.gloss ?? concept}”`}
        tone={hue(v.family)}
        sub={
          <>
            in <LanguageLink variety={variety} context={context} />
          </>
        }
      />
      <WordGloss
        engine={engine}
        version={version}
        generation={generation}
        variety={variety}
        concept={concept}
        onScrub={(at) => {
          // An inherited word predates its daughter language. Follow the
          // recorded parent chain to the language that existed in that year.
          let then = variety;
          while (overview.varieties[then].born > at && overview.varieties[then].parent !== null) {
            then = overview.varieties[then].parent!;
          }
          context.onVisit({ kind: "word", variety: then, concept }, at);
        }}
        onOpenVariety={(other) => context.go({ kind: "word", variety: other, concept })}
        renderCause={(cause) => <LoanCauseText cause={cause} ctx={languageChapterContext(context, variety)} />}
        renderOrigin={(word) => <WordOrigin word={word} ctx={languageChapterContext(context, variety)} />}
      />
      {groups.length > 1 ? (
        <>
          <h3>Across the chart</h3>
          <p className="muted small">
            Lands are coloured by the root their word comes from; words of one colour are{" "}
            <Explained term="cognate">cognates</Explained>.
          </p>
          <ul className="atlas-legend">
            {groups.map((g) => (
              <li key={g.group}>
                <span className="swatch" style={{ background: hue(g.group) }} />
                <span className="word">{g.forms.join(", ")}</span>
              </li>
            ))}
          </ul>
        </>
      ) : null}
      <StoryLeaf title="The chronicle of this word" annals={wordStory} context={context} />
    </>
  );
}


function LawCard({ id, context }: { id: string; context: Context }) {
  const { overview } = context;
  const peoples = overview.communities.filter((c) => c.ended === null);
  const had = peoples.flatMap((c) => {
    const law = overview.varieties[c.variety].laws.find((l) => l.id === id);
    return law ? [{ c, law }] : [];
  });
  const label =
    had[0]?.law.label ?? overview.varieties.flatMap((v) => v.laws).find((l) => l.id === id)?.label ?? id;
  const without = peoples.filter((c) => !had.some((h) => h.c.id === c.id));
  const told = subjectHistory({ kind: "law", id }, overview, context.map);
  const waves = had.filter(({ law }) => law.from !== null).length;
  const living = overview.varieties.filter((v) => v.spoken);
  const livingHad = living.filter((v) => v.laws.some((law) => law.id === id));
  const silentHad = overview.varieties.filter((v) => !v.spoken && v.laws.some((law) => law.id === id));
  return (
    <>
      <CardHead icon={AudioLines} kind="A sound change" title={label} />
      <Facts
        rows={[
          ["Undergone by", `${had.length} of ${peoples.length} peoples`],
          ["First heard", told.length > 0 ? `year ${told[0].generation * YEARS}` : null],
          ["Spread", waves > 0 ? `to ${waves} by neighbours’ speech` : null],
        ]}
      />
      <p className="muted small">
        A <Explained term="sound law">sound law</Explained>. On the chart, orange land underwent it and grey land did not;
        red lines are <Explained term="isogloss">isoglosses</Explained>, where it stopped.
      </p>
      {overview.varieties.filter((v) => v.laws.some((law) => law.id === id)).map((v) => (
        <Leaf key={v.id} id={`evidence-${v.id}`} title={`Words changed in ${v.name}`} summary={<p>Recorded before and after forms, in year {v.laws.find((law) => law.id === id)!.generation * YEARS}.</p>}>
          <LawEvidence variety={v} law={v.laws.find((law) => law.id === id)!} ctx={languageChapterContext(context, v.id)} />
        </Leaf>
      ))}
      {living.length > 0 && livingHad.length === living.length ? (
        <p className="muted small">Every living language has undergone this change.</p>
      ) : null}
      {had.length > 0 ? (
        <Leaf
          id="who"
          title="Who has it"
          summary={
            <p>
              <Few items={had} link={({ c }) => <PeopleLink c={c} context={context} />} leaf="who" />.
            </p>
          }
        >
          <ul className="roster">
            {had.map(({ c, law }) => (
              <li key={c.id}>
                <PeopleLink c={c} context={context} /> <span className="muted">{howCame(law, c, overview)}</span>
              </li>
            ))}
          </ul>
          {without.length > 0 ? (
            <>
              <h3>Not undergone by</h3>
              <p>
                <Joined items={without} link={(c) => <PeopleLink c={c} context={context} />} />
              </p>
            </>
          ) : null}
        </Leaf>
      ) : (
        <>
          <h3>Who has it</h3>
          <p className="muted small">
            No living language has undergone it{silentHad.length > 0 ? (
              <>; it is known from <Joined items={silentHad} link={(v) => <LanguageLink variety={v.id} context={context} />} /></>
            ) : null}.
          </p>
        </>
      )}
      <StoryLeaf title="As it happened" annals={told} context={context} />
    </>
  );
}

function LandCard({ region, context }: { region: number; context: Context }) {
  const { engine, generation, version, map, overview } = context;
  const climate = useMemo(() => engine.climate(generation), [engine, generation, version]);
  const seasons = climate.regions.find((r) => r.id === region)?.seasons;
  const place = overview.places.find((p) => p.region === region);
  const names = place?.names ?? [];
  const exonyms = place?.exonyms ?? [];
  const dwellers = peoplesByRegion(overview).get(region) ?? [];
  const arrivals = overview.moves.filter((m) => m.to === region || m.from === region);
  const now = names.at(-1);
  const neighbours = map.regions[region].neighbours.filter((id) => map.regions[id].terrain !== "sea");
  const states = overview.states.filter((state) => state.lands.includes(region));
  const religions = overview.religions.filter((religion) => religion.land === region || religion.shrines.some((shrine) => shrine.region === region));
  const cities = cityNames(overview.cities.filter((city) => city.region === region));
  const here = individualAnnals(overview.annals).filter((annal) => annal.lands.includes(region))
    .sort((a, b) => b.generation - a.generation);
  return (
    <>
      <CardHead
        icon={MapPin}
        kind="A land"
        title={landLabel(overview, map, region)}
        sub={now ? <span className="ipa">/{now.ipa}/</span> : null}
      />
      <Facts
        rows={[
          ["Land", terrain(region, context)],
          ["Area", `${Math.round(context.map.regions[region].areaKm2).toLocaleString()} km²`],
          ["Rivers", context.map.rivers.filter((river) => river.course.includes(region)).length ? <Joined
            items={context.map.rivers.filter((river) => river.course.includes(region))}
            link={(river) => <RiverLink id={river.id} context={context} />} /> : null],
          ["Lakes", context.map.lakes.filter((lake) => lake.regions.includes(region)).length ? <Joined
            items={context.map.lakes.filter((lake) => lake.regions.includes(region))}
            link={(lake) => <LakeLink id={lake.id} context={context} />} /> : null],
          ["Weather", context.map.regions[region].climateZone === null ? null : <ZoneLink id={context.map.regions[region].climateZone!} context={context} />],
          ["Seasons", seasons ? seasonalPhrase(seasons) : null],
          ["On", <LandmassOf region={region} context={context} />],
          ["Neighbours", neighbours.length > 0 ? (
            <Joined items={neighbours} link={(id) => <LandLink region={id} context={context} />} />
          ) : null],
          [
            "Home of",
            dwellers.length > 0 ? (
              <Joined items={dwellers} link={(c) => <PeopleLink c={c} context={context} />} />
            ) : names.length > 0 ? (
              "no one now"
            ) : (
              "no one yet"
            ),
          ],
          ["Held by", states.length > 0 ? (
            <Joined items={states} link={(state) => <StateLink state={state} context={context} />} />
          ) : null],
          ["Shrine of", religions.length > 0 ? (
            <Joined items={religions} link={(religion) => <ReligionLink religion={religion} context={context} />} />
          ) : null],
          ["City", cities.length > 0 ? (
            <Joined items={cities} link={(city) => <span className="word">{city.name.name}</span>} />
          ) : null],
          ["Names", names.length > 1 ? `${names.length} so far` : null],
        ]}
      />
      {here.length > 0 ? (
        <Leaf
          id="here"
          title="What happened here"
          summary={`${here.length} entries, from year ${here.at(-1)!.generation * YEARS} to year ${here[0].generation * YEARS}`}
        >
          <ol className="history">
            {here.slice(0, 60).map((annal) => {
              const kind = EVENT_KIND[annal.kind];
              return (
                <li key={annal.id}>
                  <Year generation={annal.generation} context={context} />
                  <span>
                    <kind.icon size={14} aria-label={kind.name} />{" "}
                    <button type="button" className="link" onClick={() => context.go({ kind: "event", id: annal.id })}>
                      <Told text={momentExcerpt(annal.text)} />
                    </button>
                    <EntryAnnotations annal={annal} context={context} />
                  </span>
                </li>
              );
            })}
          </ol>
          {here.length > 60 ? <p className="muted small">and {here.length - 60} earlier</p> : null}
        </Leaf>
      ) : null}
      {names.length > 0 ? (
        <Leaf
          id="names"
          title="Its names"
          summary={
            <p>
              First called <span className="word">{names[0].spelled}</span>, in year{" "}
              <Year generation={names[0].since} context={context} />
              {names.length > 1 ? `; ${names.length} names so far` : ""}.
            </p>
          }
        >
          <ol className="history">
            {names.map((n, i) => (
              <li key={i}>
                <Year generation={n.since} context={context} />
                <span>
                  <span className="word">{n.spelled}</span> <span className="ipa">/{n.ipa}/</span>{" "}
                  <span className="muted">
                    {howNamed(n, names[i - 1], overview)}
                    {n.once ? `; once ${n.once}` : ""}
                  </span>
                </span>
              </li>
            ))}
          </ol>
        </Leaf>
      ) : null}
      {exonyms.length > 0 ? (
        <Leaf
          id="exonyms"
          title="What others call it"
          summary={
            <p>
              How {new Set(exonyms.map((e) => e.variety)).size} other languages say it
              {otherNames(exonyms, now?.spelled).some((g) => !g.same) ? (
                <>, some with an <Explained term="exonym">exonym</Explained> of their own</>
              ) : null}
              .
            </p>
          }
        >
          <ul className="roster">
            {otherNames(exonyms, now?.spelled).map((group) => (
              <li key={group.spelled}>
                {group.same ? (
                  <span className="muted">As its holders say it, in </span>
                ) : (
                  <>
                    <span className="word">{group.spelled}</span> <span className="ipa">/{group.ipa}/</span> in{" "}
                  </>
                )}
                <Joined items={group.varieties} link={(v) => <LanguageLink variety={v} context={context} />} />
                {group.same ? null : (
                  <span className="muted">
                    , heard in year {group.heard * YEARS}
                    {group.once ? ` as ${group.once}` : ""}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </Leaf>
      ) : null}
      {arrivals.length > 0 ? (
        <Leaf
          id="comings"
          title="Comings and goings"
          summary={
            <p>
              {arrivals.length === 1 ? "One people came or went" : `${arrivals.length} comings and goings`}, the
              latest in year <Year generation={arrivals.at(-1)!.generation} context={context} />.
            </p>
          }
        >
          <ol className="history">
            {arrivals.map((m, i) => (
              <li key={i}>
                <Year generation={m.generation} context={context} />
                <span>
                  <PeopleLink c={overview.communities[m.community]} context={context} />{" "}
                  {m.to === region ? "came from " : "left for "}
                  <LandLink region={m.to === region ? m.from : m.to} context={context} />
                  {m.overseas ? " over the sea" : ""}
                </span>
              </li>
            ))}
          </ol>
        </Leaf>
      ) : null}
      <StoryLeaf title="The land's chronicle" annals={subjectHistory({ kind: "land", region }, overview, context.map)} context={context} />
    </>
  );
}

function momentExcerpt(text: string): string {
  if (text.length <= 90) return text;
  const cut = text.slice(0, 90).replace(/\s+\S*$/, "");
  return `${cut}${(cut.match(/\*/g)?.length ?? 0) % 2 ? "*" : ""}…`;
}

/// Languages' names for a land, grouped by how they spell it, those that
/// say it as its holders do last; within a group, the earliest hearing.
function otherNames(exonyms: PlaceExonym[], own: string | undefined) {
  const groups = new Map<string, { spelled: string; ipa: string; heard: number; once: string | null; same: boolean; varieties: number[] }>();
  for (const x of exonyms) {
    const group = groups.get(x.spelled);
    if (!group) {
      groups.set(x.spelled, { spelled: x.spelled, ipa: x.ipa, heard: x.heard, once: x.once, same: x.spelled === own, varieties: [x.variety] });
    } else {
      group.varieties.push(x.variety);
      if (x.heard < group.heard) Object.assign(group, { heard: x.heard, once: x.once });
    }
  }
  return [...groups.values()].sort((a, b) => Number(a.same) - Number(b.same) || a.heard - b.heard);
}

function eventHeading(annal: Annal, context: Context) {
  const { overview } = context;
  const peoples = annal.peoples.filter((id) => overview.communities[id]);
  const people = overview.communities[peoples[0]];
  const land = annal.lands.at(-1);
  let title = people?.name ?? `Year ${annal.generation * YEARS}`;
  let sub: ReactNode = land === undefined ? null : <LandLink region={land} context={context} />;
  switch (annal.kind) {
    case "conquest":
      if (peoples.length > 1) title = `${overview.communities[peoples[0]].name} over ${overview.communities[peoples[1]].name}`;
      break;
    case "contact": case "neighbours": case "parted":
      if (peoples.length > 1) title = `${overview.communities[peoples[0]].name} and ${overview.communities[peoples[1]].name}`;
      break;
    case "law": case "meaning": case "respelling": case "standard": case "classical": case "vernacular": case "koine":
      title = (annal.variety === null ? null : overview.varieties[annal.variety]?.name) ?? title;
      break;
    case "rose": case "fell":
      title = overview.states[annal.states[0]]?.name ?? title;
      break;
    case "faith": case "schism": case "conversion": case "pilgrimage": case "holy-land":
      title = overview.religions[annal.religions[0]]?.name ?? title;
      if (annal.kind === "conversion" && people) sub = <PeopleLink c={people} context={context} />;
      break;
    case "craft":
      title = context.catalog.crafts.find((craft) => craft.id === annal.crafts[0])?.name ?? title;
      if (people) sub = <PeopleLink c={people} context={context} />;
      break;
    case "city":
      title = overview.cities.find((city) => city.since === annal.generation &&
        (annal.states.includes(city.state) || annal.lands.includes(city.region)))?.name.name ?? title;
      break;
    case "settlement": {
      const parent = annal.settlement ? overview.communities[annal.settlement.plan.choice.community] : people;
      title = parent?.name ?? title;
      const daughter = annal.settlement?.daughter;
      if (daughter !== null && daughter !== undefined && overview.communities[daughter]) {
        sub = <PeopleLink c={overview.communities[daughter]} context={context} />;
      }
      break;
    }
  }
  return { title, sub };
}

function EventCard({ annal, context }: { annal: Annal; context: Context }) {
  const { overview } = context;
  const peoples = annal.peoples.filter((id) => overview.communities[id]);
  const kind = EVENT_KIND[annal.kind];
  const lawLabel = (id: string) => overview.varieties.flatMap((v) => v.laws).find((l) => l.id === id)?.label ?? id;
  const { title, sub } = eventHeading(annal, context);
  return (
    <>
      <CardHead icon={kind.icon} kind={`${kind.name} · year ${annal.generation * YEARS}`} title={title} sub={sub} />
      {annal.decision !== undefined ? <p className="book-decision">The author's decision, year {annal.generation * YEARS}</p> : null}
      <p className="event-text">
        <Told text={annal.text} />
        <EntryAnnotations annal={annal} context={context} />
      </p>
      {annal.settlement ? <>
        <SettlementAccount plan={annal.settlement.plan} overview={overview} map={context.map} recorded />
      </> : null}
      {annal.before ? <button type="button" className="reconsider" onClick={() => annal.settlement ? context.onReconsider(annal) : context.onReturnBefore(annal.before!)}>Return before this decision</button> : null}
      {annal.variety !== null && annal.specimen.length > 0 ? (
        <Specimen
          words={annal.specimen}
          changes
          onWord={(concept) => context.go({ kind: "word", variety: annal.variety as number, concept })}
        />
      ) : null}
      {annal.laws.length > 0 ? (
        <>
          <h3>
            <Explained term="sound law">The change</Explained>
          </h3>
          <ul className="roster">
            {annal.laws.map((id, i) => {
              // A note says where a change came from, after its label.
              const note = annal.notes[i] ?? "";
              const label = lawLabel(id);
              return (
                <li key={id}>
                  <button type="button" className="link" onClick={() => context.go({ kind: "law", id })}>
                    {label}
                  </button>
                  {note.startsWith(label) && note.length > label.length ? (
                    <span className="muted">{note.slice(label.length)}</span>
                  ) : null}
                </li>
              );
            })}
          </ul>
        </>
      ) : annal.notes.length > 0 ? (
        <ul className="apparatus">
          {annal.notes.map((note) => (
            <li key={note}>
              <Told text={note} />
            </li>
          ))}
        </ul>
      ) : null}
      {peoples.length > 0 ? (
        <>
          <h3>Who</h3>
          <ul className="roster">
            {peoples.map((id) => (
              <li key={id}>
                <PeopleLink c={overview.communities[id]} context={context} />
                {annal.kind === "law" ? null : (
                  <LanguageSpecimen variety={overview.communities[id].variety} context={context} />
                )}
              </li>
            ))}
          </ul>
        </>
      ) : null}
      {annal.states.some((id) => overview.states[id]) ? (
        <>
          <h3>States</h3>
          <p><Joined items={annal.states.filter((id) => overview.states[id])}
            link={(id) => <StateLink state={overview.states[id]} context={context} />} /></p>
        </>
      ) : null}
      {annal.religions.some((id) => overview.religions[id]) ? (
        <>
          <h3>{FAITH_NAME.plural}</h3>
          <p><Joined items={annal.religions.filter((id) => overview.religions[id])}
            link={(id) => <ReligionLink religion={overview.religions[id]} context={context} />} /></p>
        </>
      ) : null}
      {annal.crafts.length > 0 ? (
        <>
          <h3>Crafts</h3>
          <p><Joined items={annal.crafts} link={(id) => <CraftLink craft={id} context={context} />} /></p>
        </>
      ) : null}
      {annal.rivers.length > 0 ? <>
        <h3>Rivers</h3>
        <p><Joined items={annal.rivers} link={(id) => <RiverLink id={id} context={context} />} /></p>
      </> : null}
      {annal.zones.length > 0 ? <>
        <h3>Weather</h3>
        <p><Joined items={annal.zones} link={(id) => <ZoneLink id={id} context={context} />} /></p>
      </> : null}
      {annal.lands.length > 0 ? (
        <>
          <h3>Where</h3>
          <p>
            <Joined items={[...new Set(annal.lands)]} link={(region) => <LandLink region={region} context={context} />} />
          </p>
        </>
      ) : null}
      {annal.generation !== context.generation ? (
        <button type="button" onClick={() => context.onScrub(annal.generation)}>
          See the world in year {annal.generation * YEARS}
        </button>
      ) : null}
      {annal.members.length ? <Leaf id="entries" title="The individual entries" summary={<p>{annal.members.length} encounters or movements gathered into this entry.</p>}>
        <Story annals={annal.members} context={context} />
      </Leaf> : null}
      <Leaf id="related" title="Follow the threads" summary={<p>Other entries involving the same people, places, or languages.</p>}>
        <p className="muted small">Shared subjects offer places to investigate. They do not, by themselves, establish cause and consequence.</p>
        <ol className="related-moments">{relatedMoments(annal, overview.annals).slice(0, 24).map(({ annal: related, evidence }) => <li key={related.id}>
          <span className="event-kind">{evidence} · year {related.generation * YEARS}</span>
          <button type="button" className="moment" onClick={() => context.go({ kind: "event", id: related.id })}><Told text={related.text} /></button>
          <EntryAnnotations annal={related} context={context} />
        </li>)}</ol>
      </Leaf>
    </>
  );
}

export function EntryAnnotations({ annal, context }: { annal: Annal; context: Pick<Context, "go" | "overview"> }) {
  const cause = causePhrase(annal, context.overview);
  return <>
    {annal.settlement !== undefined ? <span className="author-decision"> Author's decision</span> : null}
    {cause ? <> <button type="button" className="link annal-cause" onClick={() => context.go({ kind: "event", id: cause.trigger.id })}>{cause.text} ↗</button></> : null}
  </>;
}

import { createContext, Fragment, use, useCallback, useContext, useDeferredValue, useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
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
  ScrollText,
  Search,
  Square,
  ChevronRight,
  Users,
  WholeWord,
  Waves,
  CloudSunRain,
  X,
  Download,
  type LucideIcon,
} from "lucide-react";
import type { Annal, Catalog, CityView, Community, Craft, CraftView, DecisionView, ReadEngine, Ethos, HistoryPoint, HolyLand, LakeNamesView, LawOccurrenceRef, MeaningSet, NotebookNote, Overview, Read, ReligionView, RenderingRow, RiverNamesView, ShrineKind, StateView, Subject, Variety, WordMap, WordTable, WorldMap } from "../model";
import { YEARS } from "../model";
import { resolveMeanings } from "../quarry";
import { download, fileName, wordTableCsv } from "../takeout";
import { causePhrase, CHRONICLE, CONTACT_NAME, countWord, ETHOS_AXES, ETHOS_POLES, EVENT_KIND, faithTeaching, FAITH_HOW, FAITH_NAME, FALL_NAME, howCame, howNamed, hue, landCardLabel, LEG_BY, LIVELIHOOD_NAME, otherNames, peopleLabel, peoplePhrases, RISE_NAME, SCHISM_CAUSE, seasonalPhrase, stateLabel, STRONG, temperament, TENET_NOUN, TERRAIN_NAME, unnamedName, weatherDeparture } from "../lore";
import { cityLabel, continentLabel, waterLabel, weatherZoneLabel } from "../lore";
import { filterHistory, findAnnal, individualAnnals, relatedMoments, searchText, subjectHistory, HISTORY_GROUPS, INITIAL_HISTORY, type HistoryView } from "../history";
import { eraBirths, eras, isQuiet, languageEraChange, quietLine, quietRuns, quietSummary, speechLine, type Era, type EraYear } from "../eras";
import { bond } from "../words";
import type { InterventionKind } from "./ActionDialog";
import { SettlementAccount } from "./SettlementDesk";
import { Told } from "./Told";
import { Explained } from "./Explained";
import { INITIAL_DICTIONARY, type DictionaryView } from "./Dictionary";
import { peoplesByRegion } from "./MapView";
import { Specimen } from "./Specimen";
import { LifeRibbon, occurrenceKey, sameOccurrence } from "./LifeRibbon";
import { DescentChart, FamilyForest, type Lineage } from "./FamilyTree";
import { WordGloss } from "./WordGloss";
import { Renderings } from "./Renderings";
import { closeClosingDialogs, emphasizeInk, reducedMotion, useLiftedValue } from "../motion";
import { Popover } from "./Popover";
import { CHAPTER, ChapterSummary, LawEvidence, LoanCauseText, WordOrigin, type ChapterContext } from "./LanguageChapter";
import { Margin } from "./Margin";
import { Pending } from "./Pending";
import { Desk } from "./Desk";
import { attempt, concepts } from "../engine";
import "./chronicle.css";

/// What the encyclopedia is open at.
export type Focus = Exclude<Subject, { kind: "history" }> | { kind: "history"; generation?: number };

/// Share of core words below which two languages count as unrelated.
const KIN_FLOOR = 0.05;

interface Context {
  storyView: HistoryView;
  onStoryView: (view: HistoryView) => void;
  notes: NotebookNote[];
  onSaveNote: (note: NotebookNote) => Promise<string | null>;
  onRemoveNote: (id: string) => Promise<string | null>;
  onReadNote: (note: NotebookNote) => Promise<string | null>;
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
  /// Every river's and every lake's names this year, in map order.
  rivers: RiverNamesView[];
  lakes: LakeNamesView[];
  /// Every people's word for the meaning in view, if a word is in view.
  words: WordMap | null;
  go: (focus: Focus) => void;
  onScrub: (generation: number) => void;
  onVisit: (subject: Focus, generation: number) => void;
  onFortunes: (people: number) => void;
  title?: string;
  onBook: () => void;
  /// The generation a run is writing toward, or null when the years are still.
  running?: number | null;
  onStopRun: () => void;
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
const CardToolsContext = createContext<ReactNode>(null);

/// Most cards the trail names before the one open.
const TRAIL_SHOWN = 3;

/// The encyclopedia: one card at a time about whatever is in focus, with
/// every name in it leading to that thing's own card. Each card opens the
/// same way: what kind of thing it is, its name, a box of facts, and its
/// specimen words where it has a language, then sections to read on. Long
/// and wide sections show a line on the card and open over the map.
/// The trail is the path to this card; returning to a subject truncates it.
export function Pedia({
  trail,
  onReturn,
  onIndex,
  hidden,
  cardMotion,
  instantFolio,
  folioFromCard,
  onFolioLifted,
  ...live
}: Omit<Context, "rivers" | "lakes"> & { trail: Focus[]; onReturn: (index: number) => void; onIndex: () => void; hidden: boolean;
  cardMotion: { direction: string; keyboard: boolean; ink: boolean }; instantFolio: boolean; onFolioLifted: () => void; folioFromCard: boolean }) {
  // Cards read the engine on its own thread. While the years pass, a card
  // keeps showing the year it has until the next year's answers arrive,
  // rather than blanking each year.
  const reading = useDeferredValue(useMemo(() => ({ engine: live.engine, generation: live.generation, overview: live.overview, version: live.version, words: live.words }),
    [live.engine, live.generation, live.overview, live.version, live.words]));
  const rivers = use(reading.engine.rivers(reading.generation));
  const lakes = use(reading.engine.lakes(reading.generation));
  const context: Context = { ...live, ...reading, rivers, lakes };
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
      if (e.key === "Escape" && !e.defaultPrevented && !document.querySelector("dialog:modal")) onLeaf(null);
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
          <ol className="trail" aria-label="Card path">
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
          <CardToolsContext.Provider value={!["world", "history", "event", "word"].includes(focus.kind) ? <div className="card-reading-tools">
            <button type="button" className="link" onClick={() => context.onFollow(focus, focusLabel(focus, context))}><Eye size={14} aria-hidden="true" /> Follow</button>
          </div> : null}>
            <Pending reading={context.engine}><Card focus={focus} context={context} /></Pending>
          </CardToolsContext.Provider>
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
      return context.title?.trim() || "The world";
    case "people":
      return peopleLabel(overview, focus.id);
    case "state":
      return stateLabel(overview, focus.id);
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
      return waterLabel(context.overview, context.rivers, context.map, "river", focus.id);
    case "lake":
      return waterLabel(context.overview, context.lakes, context.map, "lake", focus.id);
    case "zone":
      return weatherZoneLabel(focus.id, context.overview, context.map);
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
      return <LawCard subject={focus} context={context} />;
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
      return <HistoryCard generation={focus.generation} context={context} />;
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
}: {
  icon: LucideIcon;
  kind: string;
  title: ReactNode;
  sub?: ReactNode;
  tone?: string;
}) {
  const tools = useContext(CardToolsContext);
  return (
    <header className="card-head" style={tone ? ({ "--tone": tone } as CSSProperties) : undefined}>
      <p className="card-kind">
        <Icon size={14} aria-hidden="true" /> {kind}
      </p>
      <div className="card-title"><h2 tabIndex={-1}>{title}</h2>{tools}</div>
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
      {peopleLabel(context.overview, c.id)}
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
      {stateLabel(context.overview, state.id)}
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
  return landCardLabel(context.overview, context.map, region);
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
  return continentLabel(context.overview, context.map, landmass);
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
            {showing ? "Show all lands" : "Show these lands with the author's names"}
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
                  {landCardLabel(overview, map, land.region, land.spelled, known.filter((other) => other.spelled === land.spelled).map((other) => other.region))}
                </button>
              )}
            />
          </p>
        </section>
      ))}
    </Leaf>
  );
}

/// A city's repeated fortunes stay one place, but namesakes elsewhere do not.
function cityNames(cities: CityView[]): CityView[] {
  const seen = new Set<string>();
  return cities.filter((city) => {
    const key = `${city.region}:${city.name.name}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
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
        title={continentName(landmass, context)}
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
            <Joined items={cities} link={(city) => <button type="button" className="link word" onClick={() => context.go({ kind: "land", region: city.region })}>{cityLabel(overview, map, city)}</button>} />
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
  return <button type="button" className="link word" onClick={() => context.go({ kind: "river", id })}>
    {waterLabel(context.overview, context.rivers, context.map, "river", id)}
  </button>;
}

function LakeLink({ id, context }: { id: number; context: Context }) {
  return <button type="button" className="link word" onClick={() => context.go({ kind: "lake", id })}>
    {waterLabel(context.overview, context.lakes, context.map, "lake", id)}
  </button>;
}

function ZoneLink({ id, context }: { id: number; context: Context }) {
  return <button type="button" className="link" onClick={() => context.go({ kind: "zone", id })}>{weatherZoneLabel(id, context.overview, context.map)}</button>;
}


function RiverCard({ id, context }: { id: number; context: Context }) {
  const { engine, generation, map, overview } = context;
  const view = use(engine.river(generation, id));
  const climate = use(engine.climate(generation));
  const river = map.rivers[id];
  const now = view.names.at(-1);
  const living = overview.communities.filter((c) => c.ended === null && c.lands.some((land) => river.course.includes(land)));
  const flow = climate.rivers.find((flow) => flow.id === id);
  return <>
    <CardHead icon={Waves} kind="A river" title={waterLabel(context.overview, context.rivers, context.map, "river", id)}
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
  const { engine, generation, map, overview } = context;
  const view = use(engine.lake(generation, id));
  const lake = map.lakes[id];
  const now = view.names.at(-1);
  return <>
    <CardHead icon={Waves} kind="A lake" title={waterLabel(context.overview, context.lakes, context.map, "lake", id)}
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
  const { engine, generation, map, overview } = context;
  const climate = use(engine.climate(generation));
  const zone = climate.zones.find((zone) => zone.id === id)!;
  const lands = map.climateZones[id].regions;
  return <>
    <CardHead icon={CloudSunRain} kind="A weather zone" title={weatherZoneLabel(id, overview, map)} />
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
            <button type="button" className="event-kind link" onClick={() => context.go({ kind: "event", id: a.id })}>{EVENT_KIND[a.kind].name}</button>
            <span><Told text={a.text} names={a.names} go={context.go} /></span>
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

/// Decisions stay separate evidence: only those without an entry in this
/// reading need their own row, including actions with no threshold annal.
function untoldDecisions(annals: Annal[], decisions: DecisionView[], generation: number): DecisionView[] {
  const entries = [...annals, ...individualAnnals(annals)];
  const ids = new Set(entries.map((annal) => annal.id));
  const recorded = new Set(entries.flatMap((annal) => annal.decision === undefined ? [] : [annal.decision]));
  return decisions.filter((decision) => decision.generation <= generation &&
    !recorded.has(decision.index) && !decision.annals.some((id) => ids.has(id)));
}

const DECISION_GROUP: Record<DecisionView["kind"], HistoryView["group"]> = {
  settle: "Peoples & journeys", found: "Peoples & journeys", "found-related": "Peoples & journeys",
  connect: "Peoples & journeys", shift: "Languages & words", state: "Rule & cities",
  religion: "Faith & ideas", craft: "Faith & ideas", temper: "Land & livelihood", law: "Languages & words",
};

function filterDecisions(decisions: DecisionView[], overview: Overview, view: HistoryView): DecisionView[] {
  const lineage = new Map<number, number>();
  if (typeof view.sounds === "number") {
    for (let id: number | null = view.sounds, until = Infinity; id !== null && overview.varieties[id] && !lineage.has(id); ) {
      lineage.set(id, until);
      until = Math.min(until, overview.varieties[id].forkedAt ?? 0);
      id = overview.varieties[id].parent;
    }
  }
  const terms = searchText(view.query.trim()).split(/\s+/).filter(Boolean);
  return decisions.filter((decision) => {
    if (view.group !== "All events" && DECISION_GROUP[decision.kind] !== view.group) return false;
    if (decision.kind === "law" && view.sounds !== "all" &&
      (view.sounds === "none" || decision.variety === null || decision.generation > (lineage.get(decision.variety) ?? -1))) return false;
    const text = searchText(decision.text);
    return terms.every((term) => text.includes(term));
  });
}

type ChronicleRow = { generation: number; annal: Annal } | { generation: number; decision: DecisionView };

/// Keep a year's entries together and its otherwise untold decisions after
/// its annals, as the book does. Decisions retain the engine's action order.
function chronicleRows(annals: Annal[], decisions: DecisionView[], order: HistoryView["order"]): ChronicleRow[] {
  const rows: ChronicleRow[] = [
    ...annals.map((annal) => ({ generation: annal.generation, annal })),
    ...decisions.map((decision) => ({ generation: decision.generation, decision })),
  ];
  return rows.sort((a, b) => (order === "newest" ? b.generation - a.generation : a.generation - b.generation) ||
    Number("decision" in a) - Number("decision" in b) ||
    ("decision" in a && "decision" in b ? a.decision.index - b.decision.index : 0));
}

/// Something's story in the folio, read in the world's eras, with its
/// latest turning on the card: the latest headline, or the latest entry of
/// any kind when the story holds nothing louder.
function StoryLeaf({ title, annals, people, language, context }: { title: string; annals: Annal[]; people?: number[]; language?: number; context: Context }) {
  const { overview } = context;
  const recorded = people ? use(context.engine.decisions()) : [];
  const decisions = untoldDecisions(annals, recorded.filter((decision) => decision.people.some((id) => people?.includes(id))), context.generation);
  const contents = useMemo(() => eras(overview.annals, overview.generation, overview), [overview]);
  const lastAnnal = annals.findLast((annal) => !isQuiet(annal, overview.annals)) ?? annals.at(-1);
  const lastDecision = decisions.at(-1);
  const last = lastDecision && (!lastAnnal || lastDecision.generation >= lastAnnal.generation) ? lastDecision : lastAnnal;
  if (!last) return null;
  const count = annals.length + decisions.length;
  const spanned = contents.filter((era) => annals.some((entry) => entry.generation >= era.start && entry.generation <= era.end) ||
    decisions.some((entry) => entry.generation >= era.start && entry.generation <= era.end)).length;
  return (
    <Leaf
      id="story"
      title={title}
      summary={
        <p>
          {count.toLocaleString()} {count === 1 ? "entry" : "entries"} across {countWord(spanned)} {spanned === 1 ? "era" : "eras"}.
          {" "}Last turning, in year <Year generation={last.generation} context={context} />: <Told text={last.text} names={"names" in last ? last.names : []} go={context.go} />
          {"id" in last ? <EntryAnnotations annal={last} context={context} /> : <span className="author-decision"> The author’s decision</span>}
        </p>
      }
    >
      <SubjectTimeline annals={annals} decisions={decisions} contents={contents} language={language} context={context} />
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

function SubjectTimeline({ annals, decisions, contents, language, context }: { annals: Annal[]; decisions: DecisionView[]; contents: Era[]; language?: number; context: Context }) {
  const view = context.storyView;
  const update = (patch: Partial<HistoryView>) => context.onStoryView({ ...view, limit: 100, ...patch });
  const lines = filterHistory(annals, context.overview.varieties, view);
  const choices = filterDecisions(decisions, context.overview, view);
  const count = lines.length + choices.length;
  return <div className="subject-timeline">
    <ChronicleFilters view={view} update={update} overview={context.overview} />
    <p className="muted small" role="status">{count.toLocaleString()} recorded {count === 1 ? "entry" : "entries"}, read in the world’s eras. Names are written as they were known then.</p>
    <EraReading contents={contents} lines={lines} decisions={choices} view={view} update={update} language={language} context={context} />
    {!count ? <p className="muted">No recorded entries match these filters.</p> : null}
  </div>;
}

/// A chronicle read in the world's eras, for the whole world or for one
/// subject: each era under its opening entry with a line on what happened
/// to speech, each year's headlines at full weight and its quieter entries
/// folded into one line. A filter or search unfolds what it matches. A
/// page of entries never splits a year. `births` names the languages that
/// arose in each era, which only the whole world's reading tells.
function EraReading({ contents, lines, decisions = [], view, update, births = false, revealGeneration, language, context }: {
  contents: Era[]; lines: Annal[]; decisions?: DecisionView[]; view: HistoryView; update: (patch: Partial<HistoryView>) => void; births?: boolean; revealGeneration?: number; language?: number; context: Context;
}) {
  const { overview } = context;
  const rows = chronicleRows(lines, decisions, view.order);
  const boundary = rows[view.limit - 1]?.generation;
  const nextYear = boundary === undefined ? -1 : rows.findIndex((row, index) => index >= view.limit && row.generation !== boundary);
  const shown = rows.slice(0, nextYear < 0 ? rows.length : nextYear);
  const unfold = view.group !== "All events" || view.query.trim() !== "" || view.sounds !== "all";
  const ordered = view.order === "newest" ? [...contents].reverse() : contents;
  return <>
    <div className="chronicle-years">
      {ordered.map((era) => {
        const within = (entry: { generation: number }) => entry.generation >= era.start && entry.generation <= era.end;
        const years: EraYear[] = [];
        const decisionsByYear = new Map<number, DecisionView[]>();
        for (const row of shown) {
          if (!within(row)) continue;
          let year = years.at(-1);
          if (year?.generation !== row.generation) years.push(year = { generation: row.generation, headlines: [], quiet: [] });
          if ("annal" in row) {
            const annal = row.annal;
            (!unfold && isQuiet(annal, overview.annals) ? year.quiet : year.headlines).push(annal);
          } else {
            const choices = decisionsByYear.get(row.generation) ?? [];
            choices.push(row.decision);
            decisionsByYear.set(row.generation, choices);
          }
        }
        if (!years.length) return null;
        const speech = language === undefined ? speechLine(lines.filter(within), overview, births ? eraBirths(era, overview) : []) : null;
        const change = language === undefined ? null : languageEraChange(era, overview.annals, language);
        return <section className="chronicle-era" key={era.start}>
          <h3 className="chronicle-era-head" data-chronicle-era={era.start} tabIndex={-1}>{era.opening ? <>Year {era.start * YEARS} · <Told text={era.opening.text} names={era.opening.names} go={context.go} /></> : era.title}</h3>
          {speech ? <p className="chronicle-era-speech">{speech}</p> : null}
          {language !== undefined ? <p className="chronicle-era-speech">{change ? <><span className="gen">{change.generation * YEARS}</span> <Told text={change.text} names={change.names} go={context.go} /></> : "No recorded turning point for this language in the era."}</p> : null}
          {quietRuns(years, (year) => decisionsByYear.has(year.generation) || year.generation === revealGeneration).map((year) => <section className="chronicle-year-group" key={year.generation} data-chronicle-year={year.generation}>
            <h4 tabIndex={-1}><button type="button" className="link" title="See the world in this year" onClick={() => context.onScrub(year.generation)}>
              {year.through === undefined ? `Year ${year.generation * YEARS}` : `Years ${Math.min(year.generation, year.through) * YEARS}–${Math.max(year.generation, year.through) * YEARS}`}
            </button></h4>
            <div>
              {year.headlines.length || decisionsByYear.has(year.generation) ? <ChronicleEvents annals={year.headlines} decisions={decisionsByYear.get(year.generation)} order={view.order} context={context} /> : null}
              {year.quiet.length ? <details className="chronicle-quiet" key={`${view.order}:${year.generation}`} open={year.generation === revealGeneration ? true : undefined}>
                <summary><span>{quietSummary(year)}</span></summary>
                <p className="chronicle-quiet-detail">{quietLine(year, overview, year.through === undefined ? CHRONICLE.also : CHRONICLE.alsoEra)}</p>
                <ChronicleEvents annals={individualAnnals(year.quiet)} order={view.order} dated={year.through !== undefined} context={context} />
              </details> : null}
            </div>
          </section>)}
        </section>;
      })}
    </div>
    {rows.length > shown.length ? <button type="button" className="chronicle-load" onClick={() => update({ limit: view.limit + 100 })}>Read another {Math.min(100, rows.length - shown.length)} entries</button> : null}
  </>;
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

/// The record's word list over chosen meanings, with roots labelled, a way
/// into a blind study, and a sheet to take away.
function WordsCompared({ spoken, context, onOpenDesk }: { spoken: Variety[]; context: Context; onOpenDesk: (first: number, second: number, meanings: string) => void }) {
  const [meanings, setMeanings] = useState("specimen");
  const [own, setOwn] = useState("");
  const [excluded, setExcluded] = useState<number[]>([]);
  const [chosenReference, setChosenReference] = useState(spoken[0].id);
  const rows = spoken.filter((variety) => !excluded.includes(variety.id)).sort((a, b) => a.name.localeCompare(b.name) || a.id - b.id);
  const reference = rows.find((variety) => variety.id === chosenReference) ?? rows[0];
  const sets = context.engine.meaningSets();
  const catalog = concepts();
  const listed = useMemo(() => meanings === "own" ? resolveMeanings(own, catalog)
    : { concepts: catalog.filter((concept) => meanings === "core" ? concept.core : meanings === "specimen" ? concept.specimen : concept.field === meanings).map((concept) => concept.id), unknown: [] },
  [meanings, own, catalog]);
  const id = useId();
  const table = rows.length && listed.concepts.length ? context.engine.wordTable(context.generation, rows.map((variety) => variety.id), listed.concepts) : null;
  return <>
    <div className="compared-choices">
      <Pending reading={sets} label="Reading the meaning sets…">
        <ComparedMeanings reading={sets} meanings={meanings} onChange={setMeanings} id={`${id}-meanings`} />
      </Pending>
      <label htmlFor={`${id}-reference`}>Compare each row with
        <select id={`${id}-reference`} value={reference?.id ?? ""} disabled={!rows.length} onChange={(event) => setChosenReference(Number(event.currentTarget.value))}>
          {!rows.length ? <option value="">Choose a language below</option> : null}
          {rows.map((variety) => <option key={variety.id} value={variety.id}>{variety.name}</option>)}
        </select>
      </label>
    </div>
    {meanings === "own" ? <div className="compared-own">
      <textarea aria-label="Meanings of your own" placeholder={"water\nfire\nking\nwolf"} rows={4} value={own} spellCheck={false} onChange={(event) => setOwn(event.currentTarget.value)} />
      <p className="muted small">{listed.concepts.length} {listed.concepts.length === 1 ? "meaning" : "meanings"} found in the catalog.
        {listed.unknown.map((entry) => <span key={entry.meaning} className="compared-unknown"> <b>{entry.meaning}</b>: {entry.suggestions.length ? <>not a meaning here; try {entry.suggestions.map((concept, index) => <span key={concept}>{index ? ", " : ""}<button type="button" className="link" onClick={() => setOwn(own.replace(entry.meaning, concept))}>{concept}</button></span>)}</> : "not a meaning here"}.</span>)}
      </p>
    </div> : null}
    <details className="compared-language-choice">
      <summary>Languages in the list · {rows.length} of {spoken.length}</summary>
      <fieldset><legend>Living languages</legend>{[...spoken].sort((a, b) => a.name.localeCompare(b.name) || a.id - b.id).map((variety) => <label key={variety.id}>
        <input type="checkbox" checked={!excluded.includes(variety.id)} onChange={(event) => setExcluded(event.currentTarget.checked ? excluded.filter((id) => id !== variety.id) : [...excluded, variety.id])} />{variety.name}
      </label>)}</fieldset>
    </details>
    <p className="muted small"><strong>Recorded roots.</strong> Words shaded alike share a root in the record. Open a pair in the desk to study its current sounds before revealing that history.</p>
    {table && reference ? <Pending reading={table} label="Reading the words in these meanings…">
      <WordsComparedTable reading={table} reference={reference} meanings={meanings === "own" ? "core" : meanings} context={context} onOpenDesk={onOpenDesk} />
    </Pending> : <p className="muted">{rows.length ? "Name at least one meaning the catalog knows." : "Choose at least one language for the list, and two to open a pair in the desk."}</p>}
  </>;
}

function ComparedMeanings({ reading, meanings, onChange, id }: { reading: Read<MeaningSet[]>; meanings: string; onChange: (meanings: string) => void; id: string }) {
  const sets = use(reading);
  return <label htmlFor={id}>Meanings<select id={id} value={meanings} onChange={(event) => onChange(event.currentTarget.value)}>
    {sets.map((set) => <option key={set.id} value={set.id}>{set.label}</option>)}
    <option value="own">A list of your own…</option>
  </select></label>;
}

function WordsComparedTable({ reading, reference, meanings, context, onOpenDesk }: { reading: Read<WordTable>; reference: Variety; meanings: string; context: Context; onOpenDesk: (first: number, second: number, meanings: string) => void }) {
  const table = use(reading);
  return <div className="compared">
    <table>
      <thead><tr><th scope="col">Language</th>{table.rows.map((row) => <th key={row.concept} scope="col">{row.gloss}</th>)}<th scope="col">With {reference.name}</th></tr></thead>
      <tbody>{table.varieties.map((variety, column) => <tr key={variety.id}>
        <th scope="row"><LanguageLink variety={variety.id} context={context} /></th>
        {table.rows.map((row) => {
          const word = row.cells[column];
          return <td key={row.concept} className="compared-cell">
            <span className="compared-root" style={word ? ({ "--root": hue(word.root) } as CSSProperties) : undefined}>{word
              ? <button type="button" className="link word" title={`/${word.ipa}/${word.authored ? " · the author's root" : ""}`} onClick={() => context.go({ kind: "word", variety: variety.id, concept: row.concept })}>{word.spelled}</button>
              : <span className="muted">–</span>}</span>
          </td>;
        })}
        <td className="compared-desk-action">{variety.id === reference.id ? <span className="muted small">Reference language</span> : <button type="button" className="link" aria-label={`Open ${reference.name} and ${variety.name} in the desk`} onClick={() => onOpenDesk(reference.id, variety.id, meanings)}>Open in the desk</button>}</td>
      </tr>)}</tbody>
    </table>
    {!table.rows.length ? <p className="muted">No words are recorded for these meanings.</p> : null}
    <p className="compared-take"><button type="button" className="link" onClick={() => download(fileName(`${context.title ?? "umran"}-words`, "csv"), wordTableCsv(table), "text/csv;charset=utf-8")}>
      <Download size={13} aria-hidden="true" /> Take this sheet as CSV</button> <span className="muted small">{table.rows.length} meanings × {table.varieties.length} languages, with root numbers.</span></p>
  </div>;
}


function WorldCard({ context }: { context: Context }) {
  const { overview } = context;
  const peoples = overview.communities.filter((c) => c.ended === null);
  const spoken = overview.varieties.filter((v) => v.spoken);
  const crafts = overview.crafts.filter((c) => c.first !== null);
  const continent = overview.continents.find((c) => c.name)?.name?.name ?? context.title ?? "these lands";
  const writing = context.running ?? null;
  const folio = useContext(FolioContext)!;
  const [desk, setDesk] = useState<{ first: number; second: number; meanings: string } | null>(null);
  // While the years run, the frontispiece tells the latest turning points.
  const told = writing === null ? [] : overview.annals.filter((annal) => !isQuiet(annal, overview.annals)).slice(-6);
  return <>
    <CardHead icon={Globe} kind="A chart of" title={context.title ?? continent} />
    {writing === null ? <>
      {overview.generation === 0 ? <p className="frontispiece-caption">Choose Run on the time bar to write the book.</p> : <div className="frontispiece-reading">
        {overview.atTip ? <button type="button" className="primary" onClick={context.onBook}>Read the book</button> : null}
        <button type="button" className="link" onClick={() => context.go({ kind: "history" })}>Read the chronicle</button>
      </div>}
    </> : <section className="frontispiece-writing" aria-label="The chronicle being written">
      <p className="frontispiece-year">The chronicle is being written <span>year {(overview.generation * YEARS).toLocaleString()} <small>of {(writing * YEARS).toLocaleString()}</small></span></p>
      <progress max={writing} value={overview.generation} aria-label="Years written" />
      <ol aria-live="polite">{told.map((annal) => <li key={annal.id}><span className="frontispiece-when">{annal.generation * YEARS}</span><span><Told text={annal.text} names={annal.names} go={context.go} /></span></li>)}</ol>
      <button type="button" onClick={context.onStopRun}><Square size={13} aria-hidden="true" /> Stop here</button>
    </section>}
    <FrontispiecePeoples peoples={peoples} context={context} />
    <Leaf id="crafts" title="Crafts" summary={<p>{crafts.length ? <Joined items={crafts} link={(craft) => <CraftLink craft={craft.id} context={context} />} /> : <span className="muted">No people has a craft yet.</span>}</p>}>
      <table className="peoples"><thead><tr><th>Craft</th><th>First held</th><th>Holders</th></tr></thead><tbody>
        {overview.crafts.map((craft) => <tr key={craft.id}><th scope="row"><CraftLink craft={craft.id} context={context} /></th>
          <td>{craft.first === null ? "not yet" : <Year generation={craft.first} context={context} />}</td><td className="num">{craft.holders.length}</td></tr>)}
      </tbody></table>
    </Leaf>
    {spoken.length > 1 ? <>
      <Leaf id="compared" title="Words compared" summary={<p>Choose meanings and living languages; the shading shows recorded roots.</p>}>
        <WordsCompared spoken={spoken} context={context} onOpenDesk={(first, second, meanings) => { setDesk({ first, second, meanings }); folio.show("desk"); }} />
      </Leaf>
      <Leaf id="desk" title="The philologist’s desk" summary={<p>Study a pair of languages from their sounds, with their history covered.</p>}>
        <Desk key={desk ? `${desk.first}:${desk.second}:${desk.meanings}` : "first-pair"} first={desk?.first ?? spoken[0].id} second={desk?.second ?? spoken[1].id} meanings={desk?.meanings ?? "core"} ctx={languageChapterContext(context, desk?.first ?? spoken[0].id)} onSaveNote={context.onSaveNote} onStartNote={context.onStartNote} />
      </Leaf>
    </> : null}
    <FamilyTrees context={context} />
  </>;
}

function FrontispiecePeoples({ peoples, context }: { peoples: Community[]; context: Context }) {
  if (peoples.length <= 12) return <ul className="frontispiece-peoples">{peoples.map((c) => <li key={c.id}><PeopleLink c={c} context={context} /></li>)}</ul>;
  const families = new Map<number, Community[]>();
  for (const people of peoples) {
    const family = context.overview.varieties[people.variety].family;
    const kin = families.get(family);
    if (kin) kin.push(people);
    else families.set(family, [people]);
  }
  const grouped = [...families].map(([family, kin]) => ({
    family,
    kin,
    largest: kin.reduce((largest, people) => people.size > largest.size ? people : largest),
  }));
  return <>
    <ul className="frontispiece-families">{grouped.map(({ family, kin, largest }) => <li key={family} style={{ borderColor: hue(family) }}>
      <PeopleLink c={largest} context={context} />{kin.length > 1 ? <span className="muted"> and {kin.length - 1} kin</span> : null}
    </li>)}</ul>
    <details className="frontispiece-all-peoples">
      <summary>All {peoples.length} peoples</summary>
      <ul className="frontispiece-families">{grouped.map(({ family, kin }) => <li key={family} style={{ borderColor: hue(family) }}>
        <ul className="frontispiece-peoples">{kin.map((c) => <li key={c.id}><PeopleLink c={c} context={context} /></li>)}</ul>
      </li>)}</ul>
    </details>
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
function ChronicleEvents({ annals, decisions = [], order = "oldest", dated = false, context }: { annals: Annal[]; decisions?: DecisionView[]; order?: HistoryView["order"]; dated?: boolean; context: Context }) {
  return <ol className="chronicle-events">{chronicleRows(annals, decisions, order).map((row) => {
    if ("decision" in row) {
      const decision = row.decision;
      return <li className="chronicle-decision" key={`decision-${decision.index}`}>
        <Feather size={17} aria-hidden="true" />
        <div><span className="event-kind">{dated ? <>Year <Year generation={decision.generation} context={context} /> · </> : null}The author’s decision</span>
          <p><Told text={decision.text} /></p>
          <span className="annal-states"><Joined items={decision.people.filter((id) => context.overview.communities[id])}
            link={(id) => <PeopleLink c={context.overview.communities[id]} context={context} />} />
            {decision.variety !== null ? <> · <LanguageLink variety={decision.variety} context={context} /></> : null}
          </span>
        </div>
      </li>;
    }
    const annal = row.annal;
    const kind = EVENT_KIND[annal.kind];
    return <li key={annal.id}>
      <kind.icon size={17} aria-hidden="true" />
      <div><span className="event-kind">{dated ? <>Year <Year generation={annal.generation} context={context} /> · </> : null}<button type="button" className="link" title="Open this entry" onClick={() => context.go({ kind: "event", id: annal.id })}>{kind.name}</button></span>
        <div>{annal.decision !== undefined ? <span className="pen" title="The author's decision"><Feather size={12} aria-hidden="true" /></span> : null}<Told text={annal.text} names={annal.names} go={context.go} /></div>
        <EntryAnnotations annal={annal} context={context} />
        <AnnalLinks annal={annal} context={context} />
        {annal.specimen.length ? <Specimen words={annal.specimen} changes={annal.laws.length > 0} onWord={(concept) => { if (annal.variety !== null) context.go({ kind: "word", variety: annal.variety, concept }); }} /> : null}
      </div>
    </li>;
  })}</ol>;
}

function EraSubjects({ era, context, onOpen }: { era: Era; context: Context; onOpen: () => void }) {
  const { overview } = context;
  const subjects = new Map<string, ReactNode>();
  const entries = [...(era.opening ? [era.opening] : []), ...era.years.flatMap((year) => year.headlines)];
  for (const annal of entries) {
    for (const id of annal.peoples) if (overview.communities[id]) subjects.set(`people-${id}`, <PeopleLink c={overview.communities[id]} context={context} />);
    for (const id of annal.states) if (overview.states[id]) subjects.set(`state-${id}`, <StateLink state={overview.states[id]} context={context} />);
    for (const id of annal.religions) if (overview.religions[id]) subjects.set(`religion-${id}`, <ReligionLink religion={overview.religions[id]} context={context} />);
    for (const id of annal.languages) if (overview.varieties[id]) subjects.set(`language-${id}`, <LanguageLink variety={id} context={context} />);
  }
  const links = [...subjects.values()];
  if (links.length > 4) links.splice(4, links.length - 4, <button type="button" className="link" onClick={onOpen}>{subjects.size - 4} more</button>);
  return <span className="chronicle-era-subjects"><Joined items={links} link={(link) => link} /></span>;
}

/// The card is the contents; the wide card reads the same eras in detail.
function HistoryCard({ generation, context }: { generation?: number; context: Context }) {
  const { overview, historyView: view, onHistoryView } = context;
  const folio = useContext(FolioContext)!;
  const [destination, setDestination] = useState<{ kind: "era" | "year"; generation: number } | null>(null);
  const update = (patch: Partial<HistoryView>) => onHistoryView({ ...view, limit: 100, ...patch });
  const contents = useMemo(() => eras(overview.annals, overview.generation, overview), [overview]);
  const recorded = use(context.engine.decisions());
  const decisions = useMemo(() => untoldDecisions(overview.annals, recorded, context.generation), [overview, recorded, context.generation]);
  const choices = useMemo(() => filterDecisions(decisions, overview, view), [decisions, overview, view]);
  const speech = useMemo(() => contents.map((era) => speechLine(era.years.flatMap((year) => [...year.headlines, ...year.quiet]), overview, eraBirths(era, overview), true)), [contents, overview]);
  const lines = useMemo(() => filterHistory(overview.annals, overview.varieties, view), [overview, view]);
  const count = lines.length + choices.length;
  const unfold = view.group !== "All events" || view.query.trim() !== "" || view.sounds !== "all";
  const headlineCount = decisions.length + contents.reduce((sum, era) => sum + era.years.reduce((n, year) => n + year.headlines.length, 0), 0);
  const quietYears = contents.reduce((sum, era) => sum + era.years.filter((year) => year.quiet.length).length, 0);
  const missingLanguage = typeof view.sounds === "number" && !overview.varieties[view.sounds];
  useEffect(() => {
    if (destination === null || folio.open !== "history" || !folio.page) return;
    const frame = requestAnimationFrame(() => {
      const section = folio.page?.querySelector<HTMLElement>(destination.kind === "year"
        ? `[data-chronicle-year="${destination.generation}"]` : `[data-chronicle-era="${destination.generation}"]`)?.closest<HTMLElement>(destination.kind === "year" ? ".chronicle-year-group" : ".chronicle-era");
      const header = section?.querySelector<HTMLElement>(destination.kind === "year" ? "h4" : "h3");
      if (header && section && folio.page) {
        folio.page.scrollTo({ top: folio.page.scrollTop + section.getBoundingClientRect().top - folio.page.getBoundingClientRect().top });
        header.focus({ preventScroll: true });
        setDestination(null);
      }
    });
    return () => cancelAnimationFrame(frame);
  }, [destination, folio.open, folio.page, view.limit]);
  const openEra = (era: Era) => {
    const all = chronicleRows(filterHistory(overview.annals, overview.varieties, { ...INITIAL_HISTORY, order: view.order }), decisions, view.order);
    const last = all.reduce((index, row, i) => row.generation >= era.start && row.generation <= era.end ? i : index, -1);
    onHistoryView({ ...INITIAL_HISTORY, order: view.order, limit: Math.max(100, last + 1) });
    setDestination({ kind: "era", generation: era.start });
    folio.show("history");
  };
  useEffect(() => {
    if (generation === undefined) return;
    const all = chronicleRows(overview.annals, decisions, view.order);
    const last = all.reduce((index, row, i) => row.generation === generation ? i : index, -1);
    onHistoryView({ ...INITIAL_HISTORY, order: view.order, limit: Math.max(100, last + 1) });
    setDestination({ kind: "year", generation });
    folio.show("history");
  }, [generation]); // eslint-disable-line react-hooks/exhaustive-deps
  return (
    <>
      <CardHead icon={ScrollText} kind={overview.tellings.find((t) => t.id === overview.telling)?.name ?? "This telling"} title="The chronicle" />
      <ol className="chronicle-contents" aria-label="The chronicle's eras">{contents.map((era, index) => <li key={era.start}>
        <button type="button" className="chronicle-contents-row" title={era.title} onClick={() => openEra(era)}>
          <span className="chronicle-era-span">{era.start * YEARS}–{era.end * YEARS}</span>
          <span>{era.heading}</span>
        </button>
        <EraSubjects era={era} context={context} onOpen={() => openEra(era)} />
        {speech[index] ? <span className="chronicle-era-speech">{speech[index]}</span> : null}
      </li>)}</ol>
      <Leaf id="history" title="Entries" summary={
        <p>{headlineCount.toLocaleString()} headline {headlineCount === 1 ? "entry" : "entries"} and {quietYears.toLocaleString()} quiet {quietYears === 1 ? "year" : "years"}.</p>
      }>
        <ChronicleFilters view={view} update={update} overview={overview} sounds />
        <div className="chronicle-count"><span role="status">{count.toLocaleString()} {count === 1 ? "entry" : "entries"}{unfold ? " matching these filters" : " in this telling"}</span>
          <button type="button" className="link" onClick={() => onHistoryView(INITIAL_HISTORY)}>Reset filters</button>
        </div>
        {missingLanguage ? <p className="muted small">The selected language has not arisen in this year. Its sound changes will appear when you return to its time.</p> : null}
        {count === 0 ? <p className="index-empty">No entries match these filters. Try a different name or broaden the filters.</p> : null}
        <EraReading contents={contents} lines={lines} decisions={choices} view={view} update={update} births revealGeneration={generation} context={context} />
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
  return <Popover label={label} trigger={(props) => <button type="button" className="decide" disabled={context.running != null || disabled} {...props}>Decide…</button>}>
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
        title={peopleLabel(overview, c.id)}
        tone={hue(v.family)}
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
      <p className="people-temper"><Explained term="temper">{temperament(c.ethos).join(", ").replace(/^./, (first) => first.toUpperCase()) || "Even-tempered"}</Explained>.</p>
      <TemperLeaf c={c} told={told} context={context} />
      <StoryLeaf title="Their fortunes" annals={told} people={[c.id]} context={context} />
      <p><button type="button" className="link" disabled={context.running != null || (!told.length && !moves.length)}
        onClick={() => context.onFortunes(c.id)}>Follow their fortunes</button></p>
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
                  {m.legs.length > 1 ? (
                    <span className="muted">
                      {" "}(
                      {m.legs.map((leg, j) => (
                        <Fragment key={j}>
                          {j > 0 ? ", then " : ""}
                          {LEG_BY[leg.by]} {Math.round(leg.km).toLocaleString()} km to <LandLink region={leg.to} context={context} />
                        </Fragment>
                      ))}
                      )
                    </span>
                  ) : m.bySea ? " over the sea" : ""}
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
        ["Ships", c.fleet === null ? null : <>since year <Year generation={c.fleet.since} context={context} />, sailing from <Joined items={c.fleet.ports} link={(port) => <LandLink region={port} context={context} />} /></>],
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
        title={stateLabel(overview, state.id)}
        tone={hue(state.id)}
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
  const ended = state.fell !== null;
  const souls = `${Math.max(0, Math.round(city?.size ?? state.city)).toLocaleString()} souls`;
  if (!city) return <>{ended ? "no longer a court city" : souls}</>;
  const townsfolk = city.townsfolk === null ? null : overview.communities[city.townsfolk];
  return (
    <>
      <span className="word">{cityLabel(context.overview, context.map, city)}</span>, “{city.name.meaning}”,
      {ended ? " a great court city from year " : ` ${souls}, great since year `}
      <Year generation={city.since} context={context} />
      {ended ? <> until the state fell in year <Year generation={state.fell!} context={context} /></> : null}
      {city.makeup.length ? <>; its people {ended ? "spoke" : "speak"}{" "}
        <Joined items={city.makeup.slice(0, 3)}
          link={(m) => <><LanguageLink variety={m.variety} context={context} /> {Math.round(m.share * 100)}%</>} />
      </> : null}
      {townsfolk ? <>; its townsfolk, the <PeopleLink c={townsfolk} context={context} />, {ended || townsfolk.ended !== null ? "spoke" : "speak"} a{" "}
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
  const openingSpeakers = speakers.length ? speakers : context.overview.communities.filter((c) => c.variety === variety);
  const speakerIds = new Set(openingSpeakers.map((c) => c.id));
  const fortunes = individualAnnals(context.overview.annals).filter((annal) => annal.variety === variety || annal.peoples.some((id) => speakerIds.has(id)));
  const ctx = languageChapterContext(context, variety);
  const folio = useContext(FolioContext)!;
  return <div className="language-card">
    <CardHead icon={Languages} kind="A language" title={v.name} tone={folio.open === "desk" ? undefined : hue(v.family)} sub={v.meaning ? `“${v.meaning}”` : null} />
    <Decisions community={speakers[0]?.id ?? -1} context={context} choices={[["law", "A sound change"]]} disabled={!speakers.length} label="Decide for this language" />
    <button type="button" className="link language-desk-action" disabled={!context.overview.varieties.some((other) => other.spoken && other.id !== variety)} onClick={() => folio.show("desk")}>Compare with another language…</button>
    <div className="language-opening">
      {openingSpeakers.map((c) => {
        const phrases = peoplePhrases(c.ended);
        return <section key={c.id}>
          <h3>{phrases.land}</h3>
          <p>{phrases.homeland} <LandLink region={c.region} context={context} />, {terrain(c.region, context)}.</p>
          <p>The <PeopleLink c={c} context={context} /> {c.ended === null ? "are" : "were"} <Explained term="way of life">{LIVELIHOOD_NAME[c.livelihood].toLowerCase()}</Explained>.</p>
        </section>;
      })}
      <StoryLeaf title="Their fortunes" annals={fortunes} people={[...speakerIds]} language={variety} context={context} />
    </div>
    <LanguageSpecimen variety={variety} context={context} />
    <LifeRibbon variety={variety} ctx={ctx} compact onExpand={() => folio.show("laws")} />
    <Facts rows={[
      ["Spoken by", speakers.length ? <Joined items={speakers} link={(c) => <PeopleLink c={c} context={context} />} /> : "no one now"],
      ["Standard of", v.standardOf === null ? null : <StateLink state={context.overview.states[v.standardOf]} context={context} />],
      ["Sacred to", v.sacredOf === null ? null : <ReligionLink religion={context.overview.religions[v.sacredOf]} context={context} />],
      ["Classical of", v.classicalOf === null ? null : <StateLink state={context.overview.states[v.classicalOf]} context={context} />],
      ["Writing", v.high !== null && v.vernacular === null ? <LanguageLink variety={v.high} context={context} /> : v.written === null ? "unwritten" : `written or last respelled in year ${v.written * YEARS}`],
    ]} />
    <Leaf id="desk" title="The philologist’s desk" summary={<p>Aligned words and recurring sounds, with the family record covered.</p>}>
      <Desk first={variety} ctx={ctx} onSaveNote={context.onSaveNote} onStartNote={context.onStartNote} />
    </Leaf>
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
  const { engine, generation, overview, words } = context;
  const groups = useMemo(() => {
    if (!words) return [];
    const seen = new Map<number, string[]>();
    for (const w of words.words) seen.set(w.group, [...(seen.get(w.group) ?? []), w.spelled]);
    return [...seen.entries()].map(([group, forms]) => ({ group, forms: [...new Set(forms)] }));
  }, [words]);
  const v = overview.varieties[variety];
  const story = use(attempt(engine.story(generation, { kind: "word", variety, concept }))).value;
  const wordStory = useMemo(() => (story?.annals ?? []).map((id) => findAnnal(overview.annals, id)).filter((a): a is Annal => a !== undefined), [story, overview]);
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
        renderLink={(subject, label) => <button type="button" className="link" onClick={() => context.go(subject)}>{label}</button>}
        overview={overview}
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


function LawCard(props: { subject: Extract<Subject, { kind: "law" }>; context: Context }) {
  return <Pending reading={props.context.engine}><LawCardReading {...props} /></Pending>;
}

function LawCardReading({ subject, context }: { subject: Extract<Subject, { kind: "law" }>; context: Context }) {
  const { overview } = context;
  const candidates = overview.varieties.filter((variety) => subject.variety === undefined
    ? variety.laws.some((law) => law.id === subject.id) : variety.id === subject.variety);
  const reads = candidates.map((variety) => context.engine.lawOccurrences(context.generation, variety.id, subject.id));
  const selected: LawOccurrenceRef | null = subject.variety !== undefined && subject.generation !== undefined && subject.index !== undefined
    ? { variety: subject.variety, generation: subject.generation, law: subject.id, index: subject.index } : null;
  const groups = candidates.map((variety, i) => ({
    variety,
    occurrences: use(reads[i]).filter((occurrence) => selected ? sameOccurrence(selected, occurrence.ref)
      : (subject.generation === undefined || occurrence.ref.generation === subject.generation) &&
        (subject.index === undefined || occurrence.ref.index === subject.index)),
  }));
  const occurrences = groups.flatMap((group) => group.occurrences);
  const label = occurrences[0]?.label ?? subject.id;
  const annals = new Set(occurrences.flatMap((occurrence) => occurrence.annal ? [occurrence.annal] : []));
  const told = individualAnnals(overview.annals).filter((annal) => annals.has(annal.id));
  const peoples = overview.communities.filter((people) => people.ended === null);
  const had = peoples.filter((people) => overview.varieties[people.variety].laws.some((law) => law.id === subject.id));
  const without = peoples.filter((people) => !had.includes(people));
  const recorded = [...new Map(occurrences.map((occurrence) => [occurrenceKey(occurrence.ref), occurrence])).values()].sort((a, b) => a.ref.generation - b.ref.generation || a.ref.index - b.ref.index);
  const waves = recorded.filter((occurrence) => occurrence.arrival.kind === "wave").length;
  return <>
    <CardHead icon={AudioLines} kind={selected ? "One application of a sound change" : "A sound change"} title={label}
      sub={selected ? <>In <LanguageLink variety={selected.variety} context={context} /> · year {selected.generation * YEARS} · application {selected.index + 1}</> : null} />
    <Facts rows={[
      ["Recorded applications", recorded.length],
      ["First recorded here", recorded.length ? `year ${recorded[0].ref.generation * YEARS}` : null],
      ["Waves", waves || null],
    ]} />
    <p className="muted small">A <Explained term="sound law">sound law</Explained>. On the chart, orange land underwent the law and grey land did not; red lines are <Explained term="isogloss">isoglosses</Explained>, where it stopped. Each dated application below has its own evidence.</p>
    {groups.flatMap(({ variety, occurrences: applied }) => applied.map((occurrence) => <Leaf
      key={`${variety.id}:${occurrenceKey(occurrence.ref)}`}
      id={`evidence-${variety.id}-${occurrence.ref.generation}-${occurrence.ref.index}`}
      title={`${variety.name} · year ${occurrence.ref.generation * YEARS}${occurrence.ref.index > 0 ? ` · application ${occurrence.ref.index + 1}` : ""}`}
      summary={<p>{occurrence.words.length} {occurrence.words.length === 1 ? "word" : "words"}, {occurrence.markers.length} grammatical {occurrence.markers.length === 1 ? "form" : "forms"}, {occurrence.names.length} {occurrence.names.length === 1 ? "name" : "names"}.</p>}>
      <LifeRibbon variety={variety.id} selected={occurrence.ref} ctx={languageChapterContext(context, variety.id)} />
      <LawEvidence variety={variety} occurrence={occurrence} ctx={languageChapterContext(context, variety.id)} />
    </Leaf>))}
    {!recorded.length ? <p className="muted">No application of this law is recorded at this point in the telling.</p> : null}
    <Leaf id="who" title="Where the law is known" summary={<p>{had.length} of {peoples.length} living peoples speak a language that has undergone the law.</p>}>
      {had.length ? <ul className="roster">{had.map((people) => <li key={people.id}><PeopleLink c={people} context={context} /> · <LanguageLink variety={people.variety} context={context} /></li>)}</ul> : <p className="muted small">No living people speaks a language that has undergone it.</p>}
      {without.length ? <><h3>Not undergone by</h3><p><Joined items={without} link={(people) => <PeopleLink c={people} context={context} />} /></p></> : null}
    </Leaf>
    <StoryLeaf title="As it happened" annals={told} context={context} />
  </>;
}

function LandCard({ region, context }: { region: number; context: Context }) {
  const { engine, generation, map, overview } = context;
  const climate = use(engine.climate(generation));
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
        title={landCardLabel(overview, map, region)}
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
            <Joined items={cities} link={(city) => <button type="button" className="link word" onClick={() => context.go({ kind: "state", id: city.state })}>{cityLabel(overview, map, city)}</button>} />
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
                    <button type="button" className="link" title="Open this entry" onClick={() => context.go({ kind: "event", id: annal.id })}><kind.icon size={14} aria-label={kind.name} /></button>{" "}
                    <Told text={momentExcerpt(annal.text)} names={annal.names} go={context.go} />
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
                  {m.bySea ? " over the sea" : ""}
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
        <Told text={annal.text} names={annal.names} go={context.go} />
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
          <button type="button" className="event-kind link" title="Open this entry" onClick={() => context.go({ kind: "event", id: related.id })}>{evidence} · year {related.generation * YEARS}</button>
          <span><Told text={related.text} names={related.names} go={context.go} /></span>
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

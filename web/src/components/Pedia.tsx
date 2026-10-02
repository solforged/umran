import { createContext, Fragment, useCallback, useContext, useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { createPortal } from "react-dom";
import {
  ArrowLeft,
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
  X,
  type LucideIcon,
} from "lucide-react";
import type { Annal, Catalog, Community, Craft, CraftView, Engine, Ethos, HolyLand, Overview, PlaceExonym, ReligionView, RenderingRow, ShrineKind, StateView, Variety, WordMap, WorldMap } from "../model";
import { YEARS } from "../model";
import { CONTACT_NAME, ETHOS_AXES, ETHOS_POLES, EVENT_KIND, FAITH_HOW, FALL_NAME, howCame, howNamed, hue, LIVELIHOOD_NAME, RISE_NAME, SCHISM_CAUSE, STRESS_RULE, STRONG, temperament, TERMS, TERRAIN_NAME, type Term } from "../lore";
import { filterHistory, HISTORY_GROUPS, INITIAL_HISTORY, type HistoryView } from "../history";
import { bond } from "../words";
import type { DialogKind } from "./ActionDialog";
import { Told } from "./Told";
import { Dictionary, INITIAL_DICTIONARY, type DictionaryView } from "./Dictionary";
import { peoplesByRegion } from "./MapView";
import { Specimen } from "./Specimen";
import { DescentChart, FamilyTree, type Lineage } from "./FamilyTree";
import { WordGloss } from "./WordGloss";
import { Renderings } from "./Renderings";

/// What the encyclopedia is open at.
export type Focus =
  | { kind: "world" }
  | { kind: "people"; id: number }
  | { kind: "state"; id: number }
  | { kind: "religion"; id: number }
  | { kind: "craft"; id: Craft }
  | { kind: "language"; variety: number }
  | { kind: "word"; variety: number; concept: string }
  | { kind: "law"; id: string }
  | { kind: "land"; region: number }
  | { kind: "continent"; landmass: number }
  | { kind: "event"; annal: Annal }
  | { kind: "history" };

/// Share of core words below which two languages count as unrelated.
const KIN_FLOOR = 0.05;

interface Context {
  historyView: HistoryView;
  onHistoryView: (view: HistoryView) => void;
  dictionaryViews: Record<number, DictionaryView>;
  onDictionaryView: (variety: number, view: DictionaryView) => void;
  engine: Engine;
  catalog: Catalog;
  version: number;
  generation: number;
  overview: Overview;
  map: WorldMap;
  /// Every people's word for the meaning in view, if a word is in view.
  words: WordMap | null;
  go: (focus: Focus) => void;
  onScrub: (generation: number) => void;
  onPlay: () => void;
  onDialog: (kind: DialogKind, community: number) => void;
  /// Tell the history again as a telling set aside told it.
  onRestore: (telling: number) => void;
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
  ...context
}: Context & { trail: Focus[]; onReturn: (index: number) => void; onIndex: () => void }) {
  const focus = trail.at(-1)!;
  const cardKey = JSON.stringify(focus);
  const aside = useRef<HTMLElement>(null);
  const folioElement = useRef<HTMLElement>(null);
  const opener = useRef<HTMLElement | null>(null);
  const folioId = useId();
  const readingPositions = useRef(new Map<string, number>());
  const first = Math.max(0, trail.length - 1 - TRAIL_SHOWN);
  const { leaf, onLeaf, folioHost } = context;
  const [leaves, setLeaves] = useState<{ id: string; title: string }[]>([]);
  const [page, setPage] = useState<HTMLDivElement | null>(null);
  const register = useCallback((id: string, title: string) => {
    setLeaves((all) => (all.some((l) => l.id === id) ? all.map((l) => (l.id === id ? { id, title } : l)) : [...all, { id, title }]));
    return () => setLeaves((all) => all.filter((l) => l.id !== id));
  }, []);
  const folio = useMemo<Folio>(() => ({ open: leaf, page, show: onLeaf, register }), [leaf, page, onLeaf, register]);
  // A return visit restores the card's chosen section, once it has registered.
  const shown = leaf !== null && leaves.some((l) => l.id === leaf);
  useLayoutEffect(() => {
    aside.current?.scrollTo({ top: readingPositions.current.get(cardKey) ?? 0 });
  }, [cardKey]);
  useLayoutEffect(() => {
    page?.scrollTo({ top: readingPositions.current.get(`${cardKey}:${leaf}`) ?? 0 });
  }, [cardKey, leaf, page]);
  useEffect(() => {
    if (!shown) return;
    opener.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    folioElement.current?.querySelector<HTMLButtonElement>("[aria-selected=true]")?.focus({ preventScroll: true });
    return () => { if (opener.current?.isConnected) opener.current.focus({ preventScroll: true }); };
  }, [shown]);
  useEffect(() => {
    if (!shown) return;
    const close = (e: KeyboardEvent) => e.key === "Escape" && onLeaf(null);
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [shown, onLeaf]);
  return (
    <aside className="pedia" ref={aside} aria-label="Encyclopedia" onScroll={(e) => readingPositions.current.set(cardKey, e.currentTarget.scrollTop)}>
      <nav className="pedia-nav" aria-label="Reading navigation">
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
        <button type="button" className="icon pedia-search" onClick={onIndex} title="Search the atlas" aria-label="Search the atlas"><Search size={16} /></button>
      </nav>
        {trail.length > 1 ? (
          <ol className="trail" aria-label="Cards visited">
            {first > 0 ? <li aria-hidden="true">…</li> : null}
            {trail.slice(first, -1).map((f, i) => (
              <li key={first + i}>
                <button type="button" className="link" onClick={() => onReturn(first + i)}>
                  {focusLabel(f, context)}
                </button>
              </li>
            ))}
          </ol>
        ) : null}
      <FolioContext.Provider value={folio}>
        <article className="card" key={cardKey}>
          <Card focus={focus} context={context} />
        </article>
      </FolioContext.Provider>
      {folioHost && shown
        ? createPortal(
            <section className="folio" ref={folioElement} aria-label={`Folio: ${focusLabel(focus, context)}`}>
              <header className="folio-head">
                <span className="folio-of">{focusLabel(focus, context)}</span>
                <div className="folio-tabs" role="tablist" aria-label="Sections" onKeyDown={(e) => {
                  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) return;
                  const current = leaves.findIndex((l) => l.id === leaf);
                  const next = e.key === "Home" ? 0 : e.key === "End" ? leaves.length - 1
                    : (current + (e.key === "ArrowRight" ? 1 : -1) + leaves.length) % leaves.length;
                  e.preventDefault(); onLeaf(leaves[next].id);
                  e.currentTarget.querySelectorAll<HTMLButtonElement>("[role=tab]")[next]?.focus();
                }}>
                  {leaves.map((l) => (
                    <button
                      key={l.id}
                      type="button"
                      role="tab"
                      id={`${folioId}-${l.id}`}
                      aria-controls={`${folioId}-page`}
                      tabIndex={l.id === leaf ? 0 : -1}
                      aria-selected={l.id === leaf}
                      className="folio-tab"
                      onClick={() => onLeaf(l.id)}
                    >
                      {l.title}
                    </button>
                  ))}
                </div>
                <button type="button" className="icon folio-close" title="Close the folio (Esc)" aria-label="Close the folio" onClick={() => onLeaf(null)}>
                  <X size={16} />
                </button>
              </header>
              <div className="folio-page card" role="tabpanel" id={`${folioId}-page`} aria-labelledby={`${folioId}-${leaf}`} tabIndex={0} ref={setPage} onScroll={(e) => readingPositions.current.set(`${cardKey}:${leaf}`, e.currentTarget.scrollTop)} />
            </section>,
            folioHost,
          )
        : null}
    </aside>
  );
}

/// A card's name in a few words, for the trail.
function focusLabel(focus: Focus, context: Context): string {
  const { overview } = context;
  switch (focus.kind) {
    case "world":
      return "The world";
    case "people":
      return overview.communities[focus.id]?.name ?? "A people";
    case "state":
      return overview.states[focus.id]?.name ?? "A state";
    case "religion":
      return overview.religions[focus.id]?.name ?? "A religion";
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
    case "event":
      return `Year ${focus.annal.generation * YEARS}`;
    case "history":
      return "History";
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
    case "event":
      return <EventCard annal={focus.annal} context={context} />;
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
      <h2 className={hand === undefined ? undefined : `hand-${hand % 5}`}>{title}</h2>
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

/// A linguist's term: clicking it floats a short explanation under it, so
/// the sentence it sits in stays whole. Clicking anywhere, scrolling, or
/// Escape puts it away.
function Explained({ term, children }: { term: Term; children: ReactNode }) {
  const [at, setAt] = useState<CSSProperties | null>(null);
  const button = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!at) return;
    const close = () => setAt(null);
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    const press = (e: PointerEvent) => {
      if (!button.current?.contains(e.target as Node)) close();
    };
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    window.addEventListener("keydown", key);
    window.addEventListener("pointerdown", press);
    return () => {
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
      window.removeEventListener("keydown", key);
      window.removeEventListener("pointerdown", press);
    };
  }, [at]);
  const toggle = () => {
    if (at || !button.current) return setAt(null);
    const r = button.current.getBoundingClientRect();
    const width = Math.min(320, window.innerWidth - 16);
    const left = Math.max(8, Math.min(r.left, window.innerWidth - width - 8));
    // Under the term, or over it when the term is near the foot of the window.
    const below = r.bottom + 160 < window.innerHeight;
    setAt(below ? { left, top: r.bottom + 4 } : { left, top: r.top - 4, transform: "translateY(-100%)" });
  };
  return (
    <>
      <button ref={button} type="button" className="term" aria-expanded={at !== null} onClick={toggle}>
        {children}
      </button>
      {at ? (
        <span className="term-note" role="note" style={at}>
          {TERMS[term]}
        </span>
      ) : null}
    </>
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
  const name = context.overview.places.find((p) => p.region === region)?.names.at(-1)?.spelled;
  return name ?? `a nameless ${TERRAIN_NAME[context.map.regions[region].terrain].toLowerCase()}`;
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
  return context.overview.continents.find((c) => c.landmass === landmass)?.name?.name ?? "Unnamed continent";
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
            {showing ? "Show the whole chart" : "Show only these on the map"}
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

function ContinentCard({ landmass, context }: { landmass: number; context: Context }) {
  const { map, overview } = context;
  const mass = map.landmasses[landmass];
  const view = overview.continents.find((c) => c.landmass === landmass);
  const name = view?.name ?? null;
  const islands = map.landmasses.filter((m) => m.kind === "island").length;
  const knowers = overview.varieties.filter((v) => v.spoken && v.knownLands.some((l) => mass.regions.includes(l.region)));
  return (
    <>
      <CardHead
        icon={Earth}
        kind="A continent"
        title={name?.name ?? "Unnamed continent"}
        sub={name ? <>“{name.meaning}” <span className="ipa">/{name.ipa}/</span></> : null}
      />
      <Facts
        rows={[
          ["Lands", `${mass.regions.length}, about ${(mass.regions.length * 9000).toLocaleString()} km²`],
          ["Named by", name ? (
            <><PeopleLink c={overview.communities[name.people]} context={context} />, in{" "}
              <LanguageLink variety={name.variety} context={context} />, year <Year generation={name.since} context={context} /></>
          ) : "no one yet"],
          ["Known from", name ? <LandLink region={name.witness} context={context} /> : null],
          ["Home of", view && view.peoples.length > 0 ? (
            <Joined items={view.peoples} link={(id) => <PeopleLink c={overview.communities[id]} context={context} />} />
          ) : "no one now"],
          ["Shrines", view && view.religions.length > 0 ? (
            <Joined items={view.religions} link={(id) => <ReligionLink religion={overview.religions[id]} context={context} />} />
          ) : null],
          ["Known to", knowers.length > 0 ? (
            <Joined items={knowers} link={(v) => <LanguageLink variety={v.id} context={context} />} />
          ) : "no living language"],
        ]}
      />
      <p className="muted small">
        {name ? <>Entered on the chart from the speech of the {overview.communities[name.people].name}, the first people known to have lived on or heard of it; the name stays as it was written, whatever becomes of their language.</> :
          <>No living people knows any of this land, so the chart has no name for it.</>}{" "}
        A land here stands for country about 100 km across. Beyond its shores lie{" "}
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
            <button type="button" className="moment" onClick={() => context.go({ kind: "event", annal: a })}>
              <Told text={a.text} />
            </button>
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
  return (
    <section className={open ? "leaf open" : "leaf"}>
      <h3>
        <button type="button" className="leaf-title" aria-expanded={open} onClick={() => folio.show(open ? null : id)}>
          {title}
          <ChevronRight size={13} aria-hidden="true" />
        </button>
      </h3>
      <div className="leaf-summary">{summary}</div>
      {open && folio.page ? createPortal(children, folio.page) : null}
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
/// on the card.
function StoryLeaf({ title, annals, context }: { title: string; annals: Annal[]; context: Context }) {
  const last = annals.at(-1);
  if (!last) return null;
  return (
    <Leaf
      id="story"
      title={title}
      summary={
        <p>
          Latest, in year <Year generation={last.generation} context={context} />: <Told text={last.text} />
        </p>
      }
    >
      <Story annals={[...annals].reverse()} context={context} />
    </Leaf>
  );
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

/// Before anything has happened: how to watch, in three steps.
function Welcome({ context }: { context: Context }) {
  return (
    <section className="welcome">
      <h3>Your world is ready</h3>
      <ol>
        <li>
          <button type="button" className="primary play" onClick={context.onPlay}>
            <Play size={16} aria-hidden="true" /> Play
          </button>{" "}
          Time passes 25 years a step and stops by itself when something happens to a people. “Stop for”, below,
          chooses what.
        </li>
        <li>Click a people or a land on the map, or an entry as it appears, to read about it here.</li>
        <li>A people’s card lets you shape their history: part them, bring them to meet others, or have them take up another language.</li>
      </ol>
    </section>
  );
}

function WorldCard({ context }: { context: Context }) {
  const { overview, map } = context;
  const peoples = overview.communities.filter((c) => c.ended === null).sort((a, b) => b.size - a.size);
  const spoken = overview.varieties.filter((v) => v.spoken);
  const families = new Set(spoken.map((v) => v.family)).size;
  const lost = new Set(overview.varieties.map((v) => v.family)).size - families;
  const land = map.regions.filter((r) => r.terrain !== "sea").length;
  const held = new Set(peoples.flatMap((c) => c.lands)).size;
  const silent = overview.varieties.filter((v) => !v.spoken);
  const states = overview.states.filter((s) => s.fell === null);
  const crafts = overview.crafts.filter((c) => c.first !== null);
  return (
    <>
      <CardHead icon={Globe} kind="The world" title={`Year ${overview.generation * YEARS}`} />
      {overview.latest === 0 ? <Welcome context={context} /> : null}
      <Facts
        rows={[
          ["Peoples", peoples.length],
          ["States", <Explained term="state">{states.length} standing</Explained>],
          ["Religions", overview.religions.length],
          ["Crafts", `${overview.crafts.filter((c) => c.first !== null).length} of ${overview.crafts.length} held`],
          ["Meanings", `${context.catalog.meanings} available to a language`],
          ["Languages", silent.length > 0 ? `${spoken.length} spoken, ${silent.length} silent` : spoken.length],
          ["Families", <Explained term="family">{lost > 0 ? `${families} living, ${lost} lost` : families}</Explained>],
          ["Lands held", `${held} of ${land}`],
        ]}
      />
      {overview.latest > 0 ? (
        <p>
          <button type="button" className="link" onClick={() => context.go({ kind: "history" })}>
            Everything that has happened, year by year
          </button>
        </p>
      ) : null}
      <Leaf
        id="peoples"
        title="Peoples"
        summary={
          <p>
            The largest: <Few items={peoples} link={(c) => <PeopleLink c={c} context={context} />} leaf="peoples" />.
          </p>
        }
      >
        <table className="peoples">
          <thead><tr><th>People</th><th>Souls</th><th>Speech</th></tr></thead>
          <tbody>
            {peoples.map((c) => (
              <tr key={c.id}>
                <th scope="row">
                  <PeopleLink c={c} context={context} />
                </th>
                <td className="num">{Math.round(c.size).toLocaleString()}</td>
                <td>
                  <LanguageLink variety={c.variety} context={context} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </Leaf>
      {states.length > 0 ? (
        <Leaf
          id="states"
          title="States that stand"
          summary={<p><Few items={states} link={(state) => <StateLink state={state} context={context} />} leaf="states" />.</p>}
        >
          <table className="peoples">
            <thead><tr><th>State</th><th>Rulers</th><th>Subjects</th><th>City</th></tr></thead>
            <tbody>
              {states.map((state) => (
                <tr key={state.id}>
                  <th scope="row"><StateLink state={state} context={context} /></th>
                  <td><PeopleLink c={overview.communities[state.rulers]} context={context} /></td>
                  <td className="num">{state.members.filter((m) => m.left === null).length}</td>
                  <td className="num">{Math.round(state.city).toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Leaf>
      ) : null}
      {overview.religions.length > 0 ? (
        <Leaf
          id="religions"
          title="Religions"
          summary={<p><Few items={overview.religions} link={(religion) => <ReligionLink religion={religion} context={context} />} leaf="religions" />.</p>}
        >
          <table className="peoples">
            <thead><tr><th>Religion</th><th>Founded</th><th>Followers</th></tr></thead>
            <tbody>
              {overview.religions.map((religion) => (
                <tr key={religion.id}>
                  <th scope="row"><ReligionLink religion={religion} context={context} /></th>
                  <td><Year generation={religion.founded} context={context} /></td>
                  <td className="num">{religion.followers.length}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Leaf>
      ) : null}
      <Leaf
        id="crafts"
        title="Crafts"
        summary={
          crafts.length === 0 ? (
            <p className="muted">No people has a craft yet.</p>
          ) : (
            <p>
              Held: <Few items={crafts} link={(craft) => <CraftLink craft={craft.id} context={context} />} leaf="crafts" />.
            </p>
          )
        }
      >
        <table className="peoples">
          <thead><tr><th>Craft</th><th>First held</th><th>Holders</th></tr></thead>
          <tbody>
            {overview.crafts.map((craft) => (
              <tr key={craft.id}>
                <th scope="row"><CraftLink craft={craft.id} context={context} /></th>
                <td>{craft.first === null ? "not yet" : <Year generation={craft.first} context={context} />}</td>
                <td className="num">{craft.holders.length}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </Leaf>
      {spoken.length > 1 ? (
        <Leaf
          id="compared"
          title="Words compared"
          summary={
            <p>
              Basic words across {spoken.length} living languages, shaded by shared root.
            </p>
          }
        >
          <p className="muted small">
            Words shaded alike come from one root: they are <Explained term="cognate">cognates</Explained>.
          </p>
          <WordsCompared spoken={spoken} context={context} />
        </Leaf>
      ) : null}
      <FamilyTrees context={context} />
      {silent.length > 0 ? (
        <>
          <h3>No longer spoken</h3>
          <p>
            <Joined items={silent} link={(v) => <LanguageLink variety={v.id} context={context} />} />
          </p>
        </>
      ) : null}
    </>
  );
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
      {families.map((f) => (
        <FamilyTree
          key={f}
          overview={overview}
          family={f}
          chosen={-1}
          onOpen={(id) => context.go({ kind: "language", variety: id })}
        />
      ))}
    </Leaf>
  );
}

/// A reading of the annals, with filters kept by the stage while the
/// reader follows a person or moment and then returns to the chronicle.
function HistoryCard({ context }: { context: Context }) {
  const { overview, historyView: view, onHistoryView } = context;
  const update = (patch: Partial<HistoryView>) => onHistoryView({ ...view, limit: 100, ...patch });
  const lines = useMemo(() => filterHistory(overview.annals, overview.varieties, view), [overview, view]);
  const shown = lines.slice(0, view.limit);
  const years = new Map<number, Annal[]>();
  for (const annal of shown) years.set(annal.generation, [...(years.get(annal.generation) ?? []), annal]);
  const missingLanguage = typeof view.sounds === "number" && !overview.varieties[view.sounds];
  return (
    <>
      <CardHead icon={ScrollText} kind="The chronicle" title="A history, still unfolding" />
      <p className="muted">Follow the journeys of peoples, the fortunes of their realms, and the words they leave behind.</p>
      <Leaf id="history" title="Read the chronicle" summary={
        <p>{overview.annals.length.toLocaleString()} {overview.annals.length === 1 ? "moment" : "moments"} written through year {overview.generation * YEARS}.
          {overview.tellings.length > 0 ? ` ${overview.tellings.length} other ${overview.tellings.length === 1 ? "telling" : "tellings"} kept.` : ""}</p>
      }>
        <div className="chronicle-tools">
          <label className="chronicle-search">Search the chronicle
            <input type="search" value={view.query} placeholder="A name, a word, a journey…" onChange={(e) => update({ query: e.target.value })} />
          </label>
          <label>Follow
            <select value={view.group} onChange={(e) => update({ group: e.target.value as HistoryView["group"] })}>
              {HISTORY_GROUPS.map((group) => <option key={group}>{group}</option>)}
            </select>
          </label>
          <label>Read
            <select value={view.order} onChange={(e) => update({ order: e.target.value as HistoryView["order"] })}>
              <option value="newest">Latest first</option><option value="oldest">From the beginning</option>
            </select>
          </label>
          <label>Sound changes
            <select value={view.sounds} onChange={(e) => update({ sounds: ["all", "none"].includes(e.target.value) ? e.target.value as "all" | "none" : Number(e.target.value) })}>
              <option value="all">Every language</option><option value="none">Leave them out</option>
              {missingLanguage ? <option value={view.sounds}>Language not yet born</option> : null}
              {overview.varieties.map((v) => <option key={v.id} value={v.id}>{v.name} and its ancestors</option>)}
            </select>
          </label>
        </div>
        <div className="chronicle-count"><span role="status">{lines.length.toLocaleString()} {lines.length === 1 ? "moment" : "moments"}{view.query || view.group !== "All events" || view.sounds !== "all" ? " matching this reading" : " in this telling"}</span>
          <button type="button" className="link" onClick={() => onHistoryView(INITIAL_HISTORY)}>Reset filters</button>
        </div>
        {missingLanguage ? <p className="muted small">The selected language has not arisen in this year. Its sound changes will appear when you return to its time.</p> : null}
        {lines.length === 0 ? <p className="index-empty">No moments match this reading. Try a different name or broaden the filters.</p> : null}
        <div className="chronicle-years">
          {[...years].map(([generation, annals]) => <section className="chronicle-year-group" key={generation}>
            <h3><button type="button" className="link" title="See the world in this year" onClick={() => context.onScrub(generation)}>Year {generation * YEARS}</button></h3>
            <ol className="chronicle-events">{annals.map((annal, i) => {
              const kind = EVENT_KIND[annal.kind];
              return <li key={i}>
                <kind.icon size={17} aria-hidden="true" />
                <div><span className="event-kind">{kind.name}</span>
                  <button type="button" className="moment" onClick={() => context.go({ kind: "event", annal })}><Told text={annal.text} /></button>
                  <AnnalLinks annal={annal} context={context} />
                </div>
              </li>;
            })}</ol>
          </section>)}
        </div>
        {lines.length > shown.length ? <button type="button" className="chronicle-load" onClick={() => update({ limit: view.limit + 100 })}>Read another {Math.min(100, lines.length - shown.length)} moments</button> : null}
        {overview.tellings.length > 0 ? <section className="other-tellings">
          <h3>Other tellings</h3><p className="muted small">Histories set aside are kept here. Taking one up keeps this telling in its place.</p>
          {overview.tellings.map((telling) => <details key={telling.index}>
            <summary>From year {telling.from * YEARS} · {telling.why === "undone" ? "struck out" : "told differently"} · {telling.struck.length} {telling.struck.length === 1 ? "moment" : "moments"}</summary>
            <button type="button" onClick={() => context.onRestore(telling.index)}>Take up this telling</button>
            <ol className="history">{telling.struck.map((annal, i) => <li key={i}><span>{annal.generation * YEARS}</span><del><Told text={annal.text} /></del></li>)}</ol>
          </details>)}
        </section> : null}
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

function PeopleCard({ c, context }: { c: Community; context: Context }) {
  const { overview } = context;
  const name = (id: number) => overview.communities[id];
  const v = overview.varieties[c.variety];
  const contacts = overview.contacts.filter((k) => k.a === c.id || k.b === c.id);
  const other = (k: (typeof contacts)[number]) => name(k.a === c.id ? k.b : k.a);
  const kinds = [...new Set(contacts.map((k) => k.kind))];
  const moves = overview.moves.filter((m) => m.community === c.id);
  const told = overview.annals.filter((a) => a.peoples.includes(c.id));
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
      <Facts
        rows={[
          ["Souls", Math.round(c.size).toLocaleString()],
          [
            "Heart land",
            <>
              <LandLink region={c.region} context={context} />, {terrain(c.region, context)}
            </>,
          ],
          [
            "Lands",
            <Joined items={c.lands} link={(region) => <LandLink region={region} context={context} />} />,
          ],
          [
            "Way of life",
            <Explained term="way of life">{LIVELIHOOD_NAME[c.livelihood]}</Explained>,
          ],
          [
            "Temper",
            <Explained term="temper">{temperament(c.ethos).join(", ") || "even-tempered"}</Explained>,
          ],
          ["Speak", <LanguageLink variety={c.variety} context={context} />],
          [realm?.rulers === c.id ? "Rule" : "Subject of", realm ? <StateLink state={realm} context={context} /> : null],
          ["Faith", c.faith === null ? "its own gods" : <ReligionLink religion={overview.religions[c.faith]} context={context} />],
          ["Crafts", c.crafts.length === 0 ? "none yet" : <Joined items={c.crafts} link={(craft) => <CraftLink craft={craft} context={context} />} />],
          [
            "Family",
            v.family === c.variety ? null : (
              <>
                of <LanguageLink variety={v.family} context={context} />
              </>
            ),
          ],
          ["Since", told.length > 0 ? `year ${told[0].generation * YEARS}` : null],
          [
            "Ended",
            c.ended === null ? null : (
              <>
                year <Year generation={c.ended} context={context} />,{" "}
                {c.endedInto === null ? "died out" : (
                  <>merged into the <PeopleLink c={name(c.endedInto)} context={context} /></>
                )}
              </>
            ),
          ],
          [
            "Called",
            c.exonyms.length === 0 ? null : (
              <Joined
                items={c.exonyms}
                link={(e) => (
                  <>
                    <span className="word">{e.name}</span> by the <PeopleLink c={name(e.by)} context={context} />
                  </>
                )}
              />
            ),
          ],
        ]}
      />
      <LanguageSpecimen variety={c.variety} context={context} />
      <KnownWorldLeaf variety={c.variety} context={context} />
      <TemperLeaf c={c} told={told} context={context} />
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
      <StoryLeaf title="Their story" annals={told} context={context} />
      {c.ended === null ? (
        <>
          <h3>Shape their history</h3>
          <div className="card-actions">
            <button type="button" onClick={() => context.onDialog("split", c.id)}>
              Some go their own way
            </button>
            <button type="button" onClick={() => context.onDialog("connect", c.id)}>
              They meet another people
            </button>
            <button type="button" onClick={() => context.onDialog("shift", c.id)}>
              They take up another language
            </button>
            {!realm ? (
              <button type="button" onClick={() => context.onDialog("state", c.id)}>
                Found a state
              </button>
            ) : null}
            <button type="button" onClick={() => context.onDialog("religion", c.id)}>Found a religion</button>
            {c.crafts.length < context.catalog.crafts.length ? (
              <button type="button" onClick={() => context.onDialog("craft", c.id)}>Teach a craft</button>
            ) : null}
            <button type="button" onClick={() => context.onDialog("temper", c.id)}>Their temper turns</button>
          </div>
        </>
      ) : null}
    </>
  );
}

function StateCard({ state, context }: { state: StateView; context: Context }) {
  const { overview } = context;
  const rulers = overview.communities[state.rulers];
  const current = state.fell === null ? state.members.filter((m) => m.left === null) : [];
  const former = state.members.filter((m) => m.left !== null);
  const told = overview.annals.filter((a) => a.states.includes(state.id));
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
          <><LanguageLink variety={rulers.variety} context={context} />, since year <Year generation={state.standard} context={context} /></>
        )],
        ["Purism", state.standard === null ? null : (
          <Explained term="purism">{state.purism >= 0.5 ? "guarded against foreign words" : "open to foreign words"}</Explained>
        )],
        ["Classical form", state.classical === null ? null : (
          <><LanguageLink variety={state.classical.variety} context={context} />, fixed in year{" "}
            <Year generation={state.classical.fixed} context={context} />{" "}
            {state.classical.how === "age" ? "by grammarians" : "when the state fell"}</>
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
  const told = overview.annals.filter((a) => a.religions.includes(religion.id) &&
    (a.kind === "faith" || a.kind === "conversion" || a.kind === "meaning" ||
      a.kind === "schism" || a.kind === "pilgrimage" || a.kind === "holy-land"));
  const parent = religion.parent === null ? null : overview.religions[religion.parent];
  return (
    <>
      <CardHead icon={Sparkles} kind={parent ? "A branch of a faith" : "A religion"} title={<i>{religion.name}</i>}
        tone={hue(religion.id)}
        sub={<>“{religion.meaning}” <span className="ipa">/{religion.ipa}/</span></>} />
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
  const told = overview.annals.filter((a) => a.crafts.includes(craft.id));
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
  const { engine, version, generation, overview } = context;
  const v = overview.varieties[variety];
  const respelled = overview.annals.some((a) => a.kind === "respelling" && a.variety === variety && a.generation === v.written);
  const speakers = overview.communities.filter((c) => c.ended === null && c.variety === variety);
  const daughters = overview.varieties.filter((d) => d.parent === variety);
  // Spoken languages whose people still write this one as their classical form.
  const writers = overview.varieties.filter((w) => w.spoken && w.high === variety && w.vernacular === null);
  const kin = useMemo(
    () => engine.kin(generation, variety).filter((k) => k.score >= KIN_FLOOR),
    // `version` changes whenever the history does.
    [engine, generation, variety, version],
  );
  const laws = [...v.laws].reverse();
  return (
    <>
      <CardHead
        icon={Languages}
        kind="A language"
        title={v.name}
        tone={hue(v.family)}
        hand={v.family}
        sub={v.meaning ? `“${v.meaning}”` : null}
      />
      <LanguageSpecimen variety={variety} context={context} />
      <Facts
        rows={[
          [
            "Spoken by",
            speakers.length > 0 ? <Joined items={speakers} link={(c) => <PeopleLink c={c} context={context} />} /> : "no one now",
          ],
          ["Standard of", v.standardOf === null ? null : <StateLink state={overview.states[v.standardOf]} context={context} />],
          ["Sacred to", v.sacredOf === null ? null : <ReligionLink religion={overview.religions[v.sacredOf]} context={context} />],
          ["Classical of", v.classicalOf === null ? null : <StateLink state={overview.states[v.classicalOf]} context={context} />],
          ["Written by", v.classicalOf === null ? null : writers.length > 0 ? (
            <Joined items={writers} link={(w) => <LanguageLink variety={w.id} context={context} />} />
          ) : "no one now"],
          ["Written in", v.high === null ? null : v.vernacular === null ? (
            <><LanguageLink variety={v.high} context={context} />, a <Explained term="classical language">classical language</Explained>, beside its <Explained term="diglossia">speech</Explained></>
          ) : (
            <>its own <Explained term="vernacular">speech</Explained> since year <Year generation={v.vernacular} context={context} />, after <LanguageLink variety={v.high} context={context} /></>
          )],
          ["Kept from it", v.keptFromHigh === null || v.high === null ? null : `${Math.round(v.keptFromHigh * 100)}% of core words`],
          ["Writing", v.high !== null ? null : v.written === null ? <Explained term="spelling vs pronunciation">unwritten</Explained> :
            <><Explained term="spelling vs pronunciation">{respelled ? "last respelled" : "written"}</Explained>{" "}
              {respelled ? "in" : "since"} year <Year generation={v.written} context={context} /></>],
          ["Name style", v.nameStyle === "double" ? <Explained term="dithematic name">two-part names</Explained> : "one-word names"],
          ["Formed as", v.koineOf === null ? null : (
            <>a <Explained term="koiné">koiné</Explained> of{" "}
              <Joined items={v.koineOf}
                link={(m) => <><LanguageLink variety={m.variety} context={context} /> {Math.round(m.share * 100)}%</>} />
            </>
          )],
          [
            "Parent",
            v.parent === null ? null : (
              <>
                <LanguageLink variety={v.parent} context={context} />, parted in year {(v.forkedAt ?? 0) * YEARS}
              </>
            ),
          ],
          [
            "Daughters",
            daughters.length === 0 ? null : (
              <Joined items={daughters} link={(d) => <LanguageLink variety={d.id} context={context} />} />
            ),
          ],
          ["Sounds", <>
            {v.consonants.length} consonants, {v.vowels.length} vowels
            {v.geminates ? <>, and <Explained term="geminate">long consonants</Explained></> : null}
          </>],
          ["Stress", <Explained term="stress">{STRESS_RULE[v.stress]}</Explained>],
          ["Words", `${v.words.toLocaleString()}, built with ${v.wordBuilding}`],
          ["Own words", <Explained term="own words">
            {v.ownWords.own} of {v.ownWords.meanings} meanings
            {v.ownWords.meanings > 0 ? ` (${Math.round(v.ownWords.own / v.ownWords.meanings * 100)}%)` : ""}
            {` · ${v.ownWords.loans} loans · ${v.ownWords.shared} shared`}
          </Explained>],
          ["Sound change stops at", v.minimalWord],
        ]}
      />

      {v.names.length > 0 ? (
        <Leaf
          id="names"
          title="Given names in fashion"
          summary={
            <p>
              <Few
                items={v.names}
                link={(name) => <span className="word">{name.name}</span>}
                leaf="names"
              />
              .
            </p>
          }
        >
          <p className="muted small">
            Each is a <Explained term="given name">given name</Explained>, most built from the language’s own words.
          </p>
          <ul className="roster given-names">
            {v.names.map((name, i) => (
              <li key={i}>
                <span className="word">{name.name}</span> <span className="ipa">/{name.ipa}/</span>{" "}
                “{name.meaning}”
                {name.from !== null ? <small className="muted"> from <LanguageLink variety={name.from} context={context} /> (sacred)</small> : null}
              </li>
            ))}
          </ul>
        </Leaf>
      ) : null}
      {kin.length > 0 ? (
        <Leaf
          id="kin"
          title="Shares core words with"
          summary={
            <p>
              <Few
                items={kin}
                link={(k) => <><LanguageLink variety={k.other} context={context} /> {Math.round(k.score * 100)}%</>}
                leaf="kin"
              />
              .
            </p>
          }
        >
          <ul className="roster">
            {kin.map((k) => (
              <li key={k.other} className="meter-row">
                <LanguageLink variety={k.other} context={context} />
                <meter min={0} max={1} value={k.score} />
                <span className="muted">{Math.round(k.score * 100)}%</span>
              </li>
            ))}
          </ul>
        </Leaf>
      ) : null}
      <KnownWorldLeaf variety={variety} context={context} />
      <FamilyHead variety={v} context={context} />
      <h3>Sounds</h3>
      <p className="segments">
        {v.consonants.join(" ")}
        <br />
        {v.vowels.join(" ")}
      </p>
      <h3>Word building</h3>
      <dl className="builders">
        {v.builders.map((b) => (
          <div key={b.relation}>
            <dt>{b.relation}</dt>
            <dd className="ipa">{b.shape}</dd>
          </div>
        ))}
      </dl>
      {laws.length === 0 ? (
        <>
          <h3><Explained term="sound law">Sound laws</Explained></h3>
          <p className="muted">None yet.</p>
        </>
      ) : (
        <Leaf
          id="laws"
          title="Sound laws"
          summary={
            <p>
              {laws.length} so far; the latest, in year <Year generation={laws[0].generation} context={context} />:{" "}
              <button type="button" className="link" onClick={() => context.go({ kind: "law", id: laws[0].id })}>
                {laws[0].label}
              </button>
              .
            </p>
          }
        >
          <p className="muted small">
            Each <Explained term="sound law">sound law</Explained> changed every living word with that sound at once.
          </p>
          <ol className="history">
            {laws.map((law, i) => (
              <li key={i}>
                <Year generation={law.generation} context={context} />
                <span>
                  <button type="button" className="link" onClick={() => context.go({ kind: "law", id: law.id })}>
                    {law.label}
                  </button>
                  {law.from !== null ? (
                    <span className="muted">
                      {" "}
                      (a <Explained term="wave">wave</Explained> from <LanguageLink variety={law.from} context={context} />)
                    </span>
                  ) : null}
                </span>
              </li>
            ))}
          </ol>
        </Leaf>
      )}
      <Leaf
        id="dictionary"
        title="Every word"
        summary={<p>{v.words.toLocaleString()} words, each with its meaning, sound, and history.</p>}
      >
        <Dictionary
          engine={engine}
          version={version}
          generation={generation}
          variety={variety}
          view={context.dictionaryViews[variety] ?? INITIAL_DICTIONARY}
          onView={(view) => context.onDictionaryView(variety, view)}
          onConcept={(concept) => context.go({ kind: "word", variety, concept })}
        />
      </Leaf>
    </>
  );
}

/// A language's family drawn as a chart of descent, if it has kin.
function FamilyHead({ variety, context }: { variety: Variety; context: Context }) {
  const { overview } = context;
  const size = overview.varieties.filter((v) => v.family === variety.family && v.born <= overview.generation).length;
  if (size < 2) return null;
  return (
    <Leaf
      id="family"
      title="Its family"
      summary={
        <p>
          A chart of the {size} languages of its <Explained term="family">family</Explained>, descended from{" "}
          <LanguageLink variety={variety.family} context={context} />.
        </p>
      }
    >
      <FamilyTree
        overview={overview}
        family={variety.family}
        chosen={variety.id}
        onOpen={(id) => context.go({ kind: "language", variety: id })}
      />
    </Leaf>
  );
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
        onScrub={context.onScrub}
        onOpenVariety={(other) => context.go({ kind: "word", variety: other, concept })}
      />
      {groups.length > 1 ? (
        <>
          <h3>Across the map</h3>
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
  const told = overview.annals.filter((a) => a.laws.includes(id));
  const waves = had.filter(({ law }) => law.from !== null).length;
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
        A <Explained term="sound law">sound law</Explained>. On the map, orange land underwent it and grey land did not;
        red lines are <Explained term="isogloss">isoglosses</Explained>, where it stopped.
      </p>
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
          <p className="muted">No living people has it.</p>
        </>
      )}
      <StoryLeaf title="As it happened" annals={told} context={context} />
    </>
  );
}

function LandCard({ region, context }: { region: number; context: Context }) {
  const { overview } = context;
  const place = overview.places.find((p) => p.region === region);
  const names = place?.names ?? [];
  const exonyms = place?.exonyms ?? [];
  const dwellers = peoplesByRegion(overview).get(region) ?? [];
  const arrivals = overview.moves.filter((m) => m.to === region || m.from === region);
  const now = names.at(-1);
  return (
    <>
      <CardHead
        icon={MapPin}
        kind="A land"
        title={now?.spelled ?? "A land without a name"}
        sub={now ? <span className="ipa">/{now.ipa}/</span> : null}
      />
      <Facts
        rows={[
          ["Land", terrain(region, context)],
          ["On", <LandmassOf region={region} context={context} />],
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
          ["Names", names.length > 1 ? `${names.length} so far` : null],
        ]}
      />
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
    </>
  );
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

function EventCard({ annal, context }: { annal: Annal; context: Context }) {
  const { overview } = context;
  const peoples = annal.peoples.filter((id) => overview.communities[id]);
  const kind = EVENT_KIND[annal.kind];
  const lawLabel = (id: string) => overview.varieties.flatMap((v) => v.laws).find((l) => l.id === id)?.label ?? id;
  return (
    <>
      <CardHead icon={kind.icon} kind={kind.name} title={`Year ${annal.generation * YEARS}`} />
      <p className="event-text">
        <Told text={annal.text} />
      </p>
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
          <h3>Religions</h3>
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
    </>
  );
}

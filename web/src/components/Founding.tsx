import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { ArrowLeft, ArrowRight, ChevronDown, Feather, LocateFixed, Plus, RefreshCw } from "lucide-react";
import { attempt, createEngine, message, presetDesign, preview } from "../engine";
import type { Action, Catalog, Craft, Engine, EthosAxis, Forming, FoundingPreview, FoundingTraits, GrammarChoice, GrammarDesign, LakeNamesView, Livelihood, MapSize, Naming, Overview, PossessorOrder, RiverNamesView, Variety, WordOrder, WorldMap } from "../model";
import { YEARS } from "../model";
import { countWord, ETHOS_AXES, ETHOS_DRIVES, ETHOS_POLES, hue, landCardLabel, LIVELIHOOD_DRIVES, LIVELIHOOD_NAME, MARKING_PHRASE, MILD, OPENNESS, OPENNESS_DRIVES, peopleLabel, POSSESSOR_PHRASE, STANDING, STANDING_DRIVES, STRONG, TERRAIN_NAME, WORD_ORDER_PHRASE } from "../lore";
import { worldName } from "../shelf";
import { Designer, randomSeed, type Founding as FoundingDesign } from "./Designer";
import { FormingChart } from "./FormingChart";
import { MapView } from "./MapView";
import { useMapProjection } from "./MapProjectionSwitch";
import { MapInspector, type MapInspection } from "./MapInspector";
import { Modal } from "./Modal";
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

/// Draft on the real engine, one step at a time: the coasts, a homeland,
/// the peoples, their speech, and the title page. Writing the chronicle
/// hands that same world to the workshop with the years to run.
export function Founding({ catalog, onBegin, onChartRoom }: {
  catalog: Catalog;
  onBegin: (engine: Engine, title: string | null, author: string | null, years: number) => Promise<void>;
  onChartRoom: () => void;
}) {
  const [step, setStep] = useState<Step>("world");
  const [detent, setDetent] = useState<SheetDetent>("half");
  const [worldSeed, setWorldSeed] = useState(() => randomSeed());
  const [size, setSize] = useState<MapSize>("medium");
  const [founders, setFounders] = useState<Founder[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [inspected, setInspected] = useState<number | null>(null);
  const mapContainer = useRef<HTMLElement>(null);
  const [inspection, setInspection] = useState<MapInspection | null>(null);
  const [focus, setFocus] = useState<number | null>(null);
  const [title, setTitle] = useState<string | null>(null);
  const [author, setAuthor] = useState(() => {
    try { return localStorage.getItem("umran.author") ?? ""; }
    catch { return ""; }
  });
  const [years, setYears] = useState(DEFAULT_YEARS);
  const [projection, setProjection] = useMapProjection(mapContainer);
  const [adjusting, setAdjusting] = useState<number | null>(null);
  const [redraw, setRedraw] = useState<{ seed: number; size: MapSize } | null>(null);
  const [clearing, setClearing] = useState(false);
  /// The founder whose homeland the next land click sets.
  const [moving, setMoving] = useState<number | null>(null);
  /// Whether the next land click founds another people there.
  const [adding, setAdding] = useState(false);
  const [moveNotice, setMoveNotice] = useState<string | null>(null);
  const [settling, setSettling] = useState<Settling>("one");
  const [groupCount, setGroupCount] = useState(3);
  /// The world whose coasts have finished forming on the chart.
  const [formed, setFormed] = useState<string | null>(() =>
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ? "always" : null);
  const [built, setBuilt] = useState<Built | null>(null);
  const [error, setError] = useState<string | null>(null);
  const engine = useRef<Engine | null>(null);
  const base = useRef<{ seed: number; size: MapSize; engine: Engine } | null>(null);
  const published = useRef<Built | null>(null);
  const handedOver = useRef(false);
  const grammarDraws = useRef(new Map<number, { order: WordOrder; object: GrammarChoice; possessor: PossessorOrder }>());
  const nextKey = useRef(0);
  const pages = useRef<HTMLDivElement>(null);
  const roster = useRef<HTMLElement>(null);

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
    // Resolving new homelands publishes their draft in the same render. It
    // must not immediately rebuild the identical world a second time.
    const previous = published.current;
    if (previous?.founders === founders && previous.seed === worldSeed && previous.size === size) return;
    let live = true;
    const build = async () => {
      if (base.current?.seed !== worldSeed || base.current.size !== size) {
        const fresh = await createEngine(worldSeed, size);
        if (!live) { fresh.dispose(); return; }
        base.current?.engine.dispose();
        base.current = { seed: worldSeed, size, engine: fresh };
      }
      if (!live) return;
      const origin = base.current.engine;
      const next = await origin.foundingDraft();
      if (!live) { next.dispose(); return; }
      try {
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
        const [overview, traits, reach, riverNames, lakeNames, drawn, forming] = await Promise.all([
          year.overview(0), Promise.all(founders.map((_, index) => next.foundingTraits(index))), next.foundingPreview(),
          year.rivers(0), year.lakes(0),
          sameWorld ? sameWorld.map : next.map(), sameWorld ? sameWorld.forming : origin.forming(),
        ]);
        if (!live) { next.dispose(); return; }
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
          map: drawn, forming, overview, traits, refused, preview: reach, riverNames, lakeNames, founders: settled, seed: worldSeed, size,
        };
        engine.current?.dispose();
        engine.current = next;
        published.current = founded;
        setBuilt(founded);
        if (settled !== founders) setFounders(settled);
        setError(null);
      } catch (failure) {
        next.dispose();
        setError(message(failure));
      }
    };
    void build().catch((failure) => { if (live) setError(message(failure)); });
    return () => { live = false; };
  }, [worldSeed, size, founders]);

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
  const choosePreset = (owner: Founder, key: string) => {
    if (key === owner.preset) return;
    update(owner.key, { preset: key, design: { ...presetDesign(key, owner.seed), ...(owner.design.grammar ? { grammar: owner.design.grammar } : {}) } });
  };
  const adjustingFounder = founders.find((founder) => founder.key === adjusting);

  // Every voice card is the engine's own preview of this people's words
  // under that preset, with the seed founding would use.
  const voices = useMemo(() => {
    if (step !== "speech" || !speechOwner) return [];
    return catalog.presets.map((preset) => {
      const result = preview(presetDesign(preset.id, speechOwner.seed), speechOwner.seed, speechOwner.naming);
      const words = typeof result === "string" ? [] : VOICE_WORDS
        .map((gloss) => result.words.find((word) => word.gloss.split(",")[0] === gloss))
        .filter((word) => word !== undefined);
      return { preset, words };
    });
  }, [step, catalog, speechOwner?.key, speechOwner?.seed, speechOwner?.naming]);

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
    if (!engine.current || !canBegin) return;
    handedOver.current = true;
    onBegin(engine.current, title?.trim() && title.trim() !== defaultTitle ? title.trim() : null, author.trim() || null, years)
      .catch((failure: unknown) => { handedOver.current = false; setError(message(failure)); });
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
    <h2>{heading}</h2>
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
            <button type="button" className="link" onClick={() => { const seed = randomSeed(); update(speechOwner.key, { seed, design: speechOwner.preset === null ? speechOwner.design : { ...presetDesign(speechOwner.preset, seed), ...(speechOwner.design.grammar ? { grammar: speechOwner.design.grammar } : {}) } }); }}>Hear other words</button>
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
        <button type="button" className="brand" aria-label="Back to the chart room" onClick={onChartRoom}>
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
        {plate}
      </aside>

      <footer className="timebar setup-foot">
        {stepIndex > 0 ? <button type="button" className="link founding-back" onClick={() => go(STEPS[stepIndex - 1].id)}>
          <ArrowLeft size={14} aria-hidden="true" /> {STEPS[stepIndex - 1].name}</button> : <span />}
        <span className="year">{error ? <span className="error" role="alert">{error}</span> : founders.length
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

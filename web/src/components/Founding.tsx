import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { ChevronDown, Feather, LocateFixed, Plus } from "lucide-react";
import { createEngine, message, presetDesign } from "../engine";
import type { Catalog, Engine, EthosAxis, FoundingPreview, GrammarChoice, GrammarDesign, Livelihood, MapSize, Naming, Overview, PossessorOrder, Variety, WordOrder, WorldMap } from "../model";
import { ETHOS_AXES, ETHOS_POLES, hue, LIVELIHOOD_NAME, MARKING_PHRASE, POSSESSOR_PHRASE, temperament, TERRAIN_NAME, WORD_ORDER_PHRASE } from "../lore";
import { worldName } from "../shelf";
import { Designer, randomSeed, type Founding as FoundingDesign } from "./Designer";
import { MapView } from "./MapView";
import { useMapProjection } from "./MapProjectionSwitch";
import { MapInspector, type MapInspection } from "./MapInspector";
import { Modal } from "./Modal";
import { decodeNaming, encodeNaming, namingChoices } from "./NamingSelect";
import { Phrase } from "./Phrase";
import { Popover } from "./Popover";
import { Specimen } from "./Specimen";
import { Sample } from "./Sample";
import "./founding.css";

const MOST_PEOPLES = 12;
const BENT = 0.7;
const LIVELIHOOD_PHRASE: Record<Livelihood, string> = { farming: "farming", herding: "herding", foraging: "foraging" };
const PEOPLE_LIVELIHOOD: Record<Livelihood, string> = { farming: "farmers", herding: "herders", foraging: "foragers" };

function lower(name: string): string {
  return name.charAt(0).toLowerCase() + name.slice(1);
}

interface Founder extends FoundingDesign {
  key: number;
  preset: string | null;
  /// A stable draft key, not a community index. Related members inherit this speech.
  source: number | null;
  /// Null only while explicitly redrawing the draft's geography.
  region: number | null;
  livelihood: Livelihood | null;
  bent: { axis: EthosAxis; toward: 1 | -1 } | null;
}

interface Built {
  map: WorldMap;
  overview: Overview;
  preview: FoundingPreview;
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
  return { key, preset, source: null, seed, design: presetDesign(preset, seed), naming: drawNaming(catalog, key), power: 0.5, openness: 0.5, region, livelihood: null, bent: null };
}

function markerChoice(speech: Variety, category: "plural" | "past" | "object"): GrammarChoice {
  const marker = speech.grammar.markers.find((m) => m.category === category && m.productive && m.retired === null);
  return !marker || marker.kind === "none" ? "none" : marker.kind === "particle" ? "particle" : marker.side;
}

/// Draft on the real engine; Begin hands that same world to the workshop.
export function Founding({ catalog, onBegin, onChartRoom, onSample }: {
  catalog: Catalog;
  onBegin: (engine: Engine, title: string | null, author: string | null) => void;
  onChartRoom: () => void;
  onSample: () => Promise<void>;
}) {
  const [sampling, setSampling] = useState(false);
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
  const [projection, setProjection] = useMapProjection();
  const [adjusting, setAdjusting] = useState<number | null>(null);
  const [redraw, setRedraw] = useState<{ seed: number; size: MapSize } | null>(null);
  const [groupCount, setGroupCount] = useState(3);
  const [related, setRelated] = useState(true);
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
  }, [selected, inspected]);

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
      const next = base.current.engine.foundingDraft();
      try {
        const indices = new Map<number, number>();
        for (const [index, founder] of founders.entries()) {
          const options = {
            ...(founder.livelihood === null ? {} : { livelihood: founder.livelihood }),
            ...(founder.bent === null ? {} : { ethos: { [founder.bent.axis]: founder.bent.toward * BENT } }),
          };
          if (founder.source === null) {
            next.act({
              kind: "found", naming: founder.naming, design: founder.design, seed: founder.seed,
              power: founder.power, openness: founder.openness,
              ...(founder.region === null ? {} : { region: founder.region }), ...options,
            });
          } else {
            const source = indices.get(founder.source);
            if (source === undefined) throw new Error("Their ancestral people must be founded first.");
            const region = founder.region ?? next.foundingSites(next.overview(0).communities[source].region, 1)[0];
            if (region === undefined) throw new Error("There is no nearby homeland for these related peoples. Try another coast or a larger world.");
            next.act({ kind: "found-related", source, region, naming: founder.naming, ...options });
          }
          indices.set(founder.key, index);
        }
        const overview = next.overview(0);
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
          map: previous?.seed === worldSeed && previous.size === size ? previous.map : next.map(),
          overview, preview: next.foundingPreview(), founders: settled, seed: worldSeed, size,
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
  const currentBuild = !!built && built.founders === founders && built.seed === worldSeed && built.size === size;
  const selectedIndex = founders.findIndex((founder) => founder.key === selected);
  const current = founders[selectedIndex];
  const people = overview?.communities[selectedIndex];
  const speech = people ? overview?.varieties.find((variety) => variety.id === people.variety) : undefined;
  const speechOwner = current?.source == null ? current : founders.find((founder) => founder.key === current.source);
  const inherited = !!current && current.source !== null;
  const drawn = speechOwner ? grammarDraws.current.get(speechOwner.key) : undefined;
  const land = inspected === null ? undefined : map?.regions[inspected];
  const occupied = !!land && !!overview?.communities.some((community) => community.lands.includes(land.id));
  const defaultTitle = overview?.communities.length ? worldName(overview) : null;
  const chartReady = !!map && !!overview;
  const riverNames = useMemo(() => map && engine.current ? map.rivers.map((river) => engine.current!.river(0, river.id)) : [], [built]);
  const lakeNames = useMemo(() => map && engine.current ? map.lakes.map((lake) => engine.current!.lake(0, lake.id)) : [], [built]);
  const landName = (region: number) => overview?.places.find((place) => place.region === region)?.names.at(-1)?.spelled ?? "unnamed land";
  const available = MOST_PEOPLES - founders.length;
  const count = Math.min(groupCount, available);
  const group = useMemo(() => {
    if (!currentBuild || inspected === null || count < 2 || occupied || !engine.current) return { sites: [], error: "" };
    try {
      return { sites: engine.current.foundingSites(inspected, count), error: "" };
    } catch (failure) {
      return { sites: [], error: message(failure) };
    }
  }, [built, currentBuild, inspected, count, occupied]);

  const update = (key: number, patch: Partial<Founder>) => setFounders((all) => all.map((founder) => founder.key === key ? { ...founder, ...patch } : founder));
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
  const add = () => {
    if (!land || !currentBuild || available < 1) return;
    const founder = drawFounder(catalog, nextKey.current++, land.id);
    setFounders((all) => [...all, founder]);
    setSelected(founder.key);
  };
  const addGroup = () => {
    if (!land || !currentBuild || occupied || count < 2 || group.sites.length !== count) return;
    const first = drawFounder(catalog, nextKey.current++, group.sites[0]);
    const members = group.sites.slice(1).map((region) => {
      const key = nextKey.current++;
      return related
        ? { ...first, key, source: first.key, region, naming: drawNaming(catalog, key) }
        : drawFounder(catalog, key, region);
    });
    setFounders((all) => [...all, first, ...members]);
    setSelected(first.key);
  };
  const remove = () => {
    if (!current) return;
    const firstChild = founders.find((founder) => founder.source === current.key);
    const remaining = founders.filter((founder) => founder.key !== current.key).map((founder) => {
      if (!firstChild || founder.source !== current.key) return founder;
      // Keep a family's speech when its first draft member is removed.
      return founder === firstChild
        ? { ...current, key: founder.key, source: null, naming: founder.naming, region: founder.region, livelihood: founder.livelihood, bent: founder.bent }
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
  const adjustingFounder = founders.find((founder) => founder.key === adjusting);
  const speechPhrase = speechOwner?.preset == null ? "their own speech, shaped by hand" : lower(catalog.presets.find((preset) => preset.id === speechOwner.preset)!.name);

  return (
    <div className="stage setup founding">
      <header className="founding-rail">
        <button type="button" className="brand" aria-label="Back to the chart room" onClick={onChartRoom}>
          <span className="brand-name"><span>ʿUmrān</span></span>
        </button>
      </header>
      <section ref={mapContainer} className="stage-map" aria-label={projection === "globe" ? "Globe" : "Chart"} aria-busy={!currentBuild}>
        {chartReady ? <MapView key={`${worldSeed}:${size}`} map={map} overview={overview} generation={0} tint={{ kind: "peoples" }}
          projection={projection} onProjection={setProjection}
          riverNames={riverNames}
          lakeNames={lakeNames}
          reach={people && built ? { preview: built.preview, people: people.id } : undefined}
          chosen={new Set(people ? [people.id] : [])}
          lands={new Set(inspected === null ? [] : [inspected])}
          focus={focus === null ? null : map.regions[focus]?.site ?? null}
          zoomable
          onPeople={(id) => { if (currentBuild && founders[id]) choose(founders[id].key); }}
          onInspect={(feature, at) => setInspection(feature ? { feature, at } : null)}
          onLand={(region) => { if (currentBuild && map.regions[region].terrain !== "sea") setInspected(region); }} /> : null}
        {chartReady ? <MapInspector container={mapContainer} inspection={inspection} map={map} overview={overview} generation={0} selectedVariety={people?.variety} riverNames={riverNames} lakeNames={lakeNames} pinnable={false} /> : null}
        <div className="cartouche founding-cartouche">
          <input className="founding-title" aria-label="World name" value={title ?? defaultTitle ?? ""}
            placeholder="An unnamed world" onChange={(event) => setTitle(event.target.value)} />
          <div className="founding-world-tools">
            <select aria-label="World size" value={size} onChange={(event) => requestWorld({ seed: worldSeed, size: event.target.value as MapSize })}>
              {catalog.mapSizes.map((choice) => <option key={choice.id} value={choice.id} title={choice.description}>{choice.name}</option>)}
            </select>
            <Popover label="World details" role="dialog" align="start"
              trigger={(props) => <button type="button" className="link" {...props}>Details <ChevronDown size={12} aria-hidden="true" /></button>}>
              {(close) => <div className="founding-world-details">
                <label>Author<input aria-label="Author" placeholder="Your name" value={author} onChange={(event) => setAuthor(event.target.value)} /></label>
                <dl>
                  <div><dt>Seed</dt><dd><code>{worldSeed}</code></dd></div>
                  {map ? <><div><dt>Radius</dt><dd>{map.radiusKm.toLocaleString()} km</dd></div>
                    <div><dt>Regions</dt><dd>{map.regions.length.toLocaleString()}</dd></div></> : null}
                </dl>
                <p>{catalog.mapSizes.find((choice) => choice.id === size)?.description}</p>
                <button type="button" onClick={() => { close(); requestWorld({ seed: randomSeed(), size }); }}>Redraw the coasts</button>
              </div>}
            </Popover>
          </div>
        </div>
        <p className="map-hint">{founders.length ? "Select a land to explore or settle; choose a name to read its people." : "Choose a homeland. The rest of the world can wait."}</p>
      </section>

      <aside className="pedia setup-panel" aria-label="Founding peoples" aria-busy={!currentBuild}>
        <header className="founding-head">
          <div className="eyebrow">Before the first year</div>
          <h2>The founding peoples</h2>
          {!founders.length ? <p>Start in one place, with one people or a few neighbours.</p> : null}
        </header>
        {founders.length ? <nav className="founding-roster" aria-label="Peoples at the beginning" ref={roster}>
          {founders.map((founder, index) => {
            const entry = overview?.communities[index];
            const variety = entry ? overview?.varieties.find((candidate) => candidate.id === entry.variety) : undefined;
            return <button type="button" key={founder.key} aria-pressed={selected === founder.key} onClick={() => choose(founder.key)}
              style={{ "--tone": variety ? hue(variety.family) : undefined } as CSSProperties}>
              <span className="founding-roster-number">{String(index + 1).padStart(2, "0")}</span>
              <span className={variety ? `founding-roster-name hand-${variety.family % 5}` : "founding-roster-name"}>{entry?.name ?? "Settling…"}</span>
              <span className="founding-roster-place">{entry ? landName(entry.region) : ""}</span>
            </button>;
          })}
        </nav> : null}
        <div className="founding-pages" ref={pages}>
          {land ? <section className="founding-land" aria-label="Selected homeland">
            <div className="eyebrow">{land.island ? "On an island" : "On the mainland"}</div>
            <h3>{landName(land.id) === "unnamed land" ? `${TERRAIN_NAME[land.terrain]}${land.coastal ? " by the sea" : " inland"}` : landName(land.id)}</h3>
            <p>{landName(land.id) !== "unnamed land" ? `${TERRAIN_NAME[land.terrain]}, ${land.coastal ? "coastal" : "inland"}. ` : ""}
              {Math.round(land.areaKm2).toLocaleString()} km²{map?.rivers.some((river) => river.course.includes(land.id)) ? " · a river runs through it" : ""}.</p>
            {occupied ? <p className="muted">Home to {overview?.communities.filter((community) => community.lands.includes(land.id)).map((community) => community.name).join(", ")}.</p> : null}
            <div className="founding-land-actions">
              <button type="button" disabled={!currentBuild || available < 1} onClick={add}><Plus size={13} aria-hidden="true" /> Found a people here</button>
              <Popover label="Found a group" role="dialog" align="end"
                trigger={(props) => <button type="button" className="link" disabled={!currentBuild || available < 2 || occupied} {...props}>A group… <ChevronDown size={12} aria-hidden="true" /></button>}>
                {(close) => <div className="founding-group">
                  <h3>A few neighbours</h3>
                  <label>Peoples<select aria-label="Number of founding peoples" value={count} onChange={(event) => setGroupCount(Number(event.target.value))}>
                    {Array.from({ length: Math.max(0, Math.min(6, available) - 1) }, (_, i) => i + 2).map((n) => <option key={n} value={n}>{n}</option>)}
                  </select></label>
                  <fieldset><legend>At the beginning</legend>
                    <label><input type="radio" name="founding-kin" checked={related} onChange={() => setRelated(true)} /> Related peoples, sharing ancestral speech</label>
                    <label><input type="radio" name="founding-kin" checked={!related} onChange={() => setRelated(false)} /> Neighbouring peoples, with separate languages</label>
                  </fieldset>
                  <p>{related ? "Their words begin alike and have a common origin. Their speech can part as the years pass." : "Each starts with its own language. Living nearby can bring them into contact."}</p>
                  <p className={group.error || group.sites.length < count ? "notice" : "muted"}>
                    {group.error || (group.sites.length < count ? `Only ${group.sites.length} unoccupied homelands are within reach. Choose fewer peoples or another land.` : `${count} nearby homelands, connected over land. Each can be changed before you begin.`)}
                  </p>
                  <button type="button" className="primary" disabled={group.sites.length !== count || !currentBuild} onClick={() => { addGroup(); close(); }}>Found these peoples</button>
                </div>}
              </Popover>
            </div>
            {current && current.region !== land.id ? <button type="button" className="link founding-move" disabled={!currentBuild}
              onClick={() => update(current.key, { region: land.id })}><LocateFixed size={13} aria-hidden="true" /> Move {people?.name ?? "the selected people"} here</button> : null}
          </section> : <section className="founding-invitation">
            <h3>Where will their story begin?</h3>
            <p>Select a land to read it. Rivers, coasts, and the way through neighbouring lands will shape the people who live there.</p>
          </section>}

          {current && people && speech && speechOwner && map ? <article className="founding-portrait" key={current.key} aria-label={`The ${people.name}`}>
            <header><h3 className={`hand-${speech.family % 5}`}>{people.name}</h3><span>{people.meaning}</span></header>
            <p>They live in <button type="button" className="link" onClick={() => { setInspected(people.region); setFocus(people.region); }}>{landName(people.region)}</button>,
              {" "}<Phrase label="Way of life" value={current.livelihood ?? ""}
                choices={[{ key: "", text: `as the land suggests (${LIVELIHOOD_PHRASE[people.livelihood]})` },
                  ...(Object.keys(LIVELIHOOD_NAME) as Livelihood[]).map((livelihood) => ({ key: livelihood, text: `by ${LIVELIHOOD_PHRASE[livelihood]}` }))]}
                onChange={(key) => update(current.key, { livelihood: key === "" ? null : key as Livelihood })} />.</p>
            <p>They are <Phrase label="Temper" value={current.bent ? `${current.bent.axis} ${current.bent.toward}` : ""}
              choices={[{ key: "", text: "shaped by land and life" }, ...ETHOS_AXES.flatMap((axis) => [1, -1].map((toward) => ({ key: `${axis} ${toward}`, text: ETHOS_POLES[axis][toward > 0 ? 1 : 0] })))]}
              onChange={(key) => { const [axis, toward] = key.split(" "); update(current.key, { bent: key === "" ? null : { axis: axis as EthosAxis, toward: Number(toward) as 1 | -1 } }); }} />
              {current.bent === null ? ` (${temperament(people.ethos, 2).join(" and ") || "even-tempered"})` : ""}.</p>
            <section className="founding-speech">
              <h4>What they speak</h4>
              <Specimen words={speech.specimen} />
              {inherited ? <p className="founding-inheritance">Their speech is inherited from {" "}
                <button type="button" className="link" onClick={() => choose(speechOwner.key)}>{overview?.communities[founders.indexOf(speechOwner)]?.name}</button>.
                {" "}The same words, a new branch.</p> : <p>Their speech is <Phrase label="Sounds" value={speechOwner.preset ?? ""}
                choices={[...(speechOwner.preset === null ? [{ key: "", text: "their own, shaped by hand" }] : []),
                  ...catalog.presets.map((preset) => ({ key: preset.id, text: lower(preset.name), title: preset.description }))]}
                onChange={(key) => { if (key && key !== speechOwner.preset) update(speechOwner.key, { preset: key, design: { ...presetDesign(key, speechOwner.seed), ...(speechOwner.design.grammar ? { grammar: speechOwner.design.grammar } : {}) } }); }} />.</p>}
              <div className="founding-speech-actions">
                {!inherited ? <button type="button" className="link" onClick={() => { const seed = randomSeed(); update(speechOwner.key, { seed, design: speechOwner.preset === null ? speechOwner.design : { ...presetDesign(speechOwner.preset, seed), ...(speechOwner.design.grammar ? { grammar: speechOwner.design.grammar } : {}) } }); }}>Hear other words</button> : null}
                <button type="button" className="link" onClick={() => setAdjusting(speechOwner.key)}>{inherited ? "Shape their shared speech…" : "Adjust their sounds…"}</button>
              </div>
            </section>
            <details className="founding-grammar">
              <summary>Grammar <span>{WORD_ORDER_PHRASE[speech.grammar.order].choice}</span></summary>
              {inherited ? <p>Grammar is inherited too. <button type="button" className="link" onClick={() => choose(speechOwner.key)}>Edit their ancestral speech</button>.</p> : <p>They put <Phrase label="Word order" value={speechOwner.design.grammar?.order ?? ""}
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
                onChange={(key) => setGrammar({ possessor: key === "" ? null : key as PossessorOrder })} />.</p>}
              <p className="muted small">Both mean “the child’s fish”; these choices set only the order, not genitive marking.</p>
              {speech.grammar.sample ? <><Sample rendering={speech.grammar.sample.sentence} label="Sample sentence" />
                <Sample rendering={speech.grammar.sample.possession} label="Sample possession" />
                <Sample rendering={speech.grammar.sample.future} label="Sample future" /></> : null}
            </details>
            <details className="founding-identity">
              <summary>Names <span>{speech.name}</span></summary>
              <p>They name themselves <Phrase label="Name" value={encodeNaming(current.naming)} choices={namingChoices(catalog)}
                onChange={(key) => { const naming = decodeNaming(key); if (naming) update(current.key, { naming }); }} />, <b>{people.name}</b>.
                They call their speech <i>{speech.name}</i>.</p>
              <p className="muted">{PEOPLE_LIVELIHOOD[people.livelihood]} · {speechPhrase}</p>
            </details>
            <button type="button" className="link founding-remove" onClick={remove}>Leave this people out</button>
          </article> : null}
        </div>
        <footer className="founding-panel-foot">
          <button type="button" className="link" disabled={sampling} onClick={() => {
            setSampling(true);
            setTimeout(() => void onSample().finally(() => setSampling(false)), 30);
          }}>{sampling ? "Writing four thousand years…" : "Read a chronicle already written"}</button>
        </footer>
      </aside>

      <footer className="timebar setup-foot">
        <span className="year">{error ? <span className="error" role="alert">{error}</span> : founders.length
          ? <><strong>Year 0.</strong> {founders.length} {founders.length === 1 ? "people" : "peoples"}; their chronicle is yet to be written.</>
          : "A world before its first peoples."}</span>
        <button type="button" className="primary begin" disabled={!engine.current || !currentBuild || !chartReady || !founders.length} onClick={() => {
          if (!engine.current || !currentBuild || !founders.length) return;
          try {
            handedOver.current = true;
            onBegin(engine.current, title?.trim() && title.trim() !== defaultTitle ? title.trim() : null, author.trim() || null);
          } catch (failure) { handedOver.current = false; setError(message(failure)); }
        }}><Feather size={18} aria-hidden="true" /> Begin</button>
      </footer>
      {adjustingFounder ? <Modal open wide title="Adjust their sounds" onClose={() => setAdjusting(null)}>
        {founders.some((founder) => founder.source === adjustingFounder.key) ? <p className="shared-speech-note">These are the founding sounds shared by this people and their kin.</p> : null}
        <Designer catalog={catalog} submit="Use these sounds" initial={adjustingFounder} onCancel={() => setAdjusting(null)}
          onFound={(founder) => { setAdjusting(null); update(adjustingFounder.key, { ...founder, preset: null }); }} />
      </Modal> : null}
      {redraw ? <Modal open title="Redraw this world?" onClose={() => setRedraw(null)}
        footer={<><button type="button" onClick={() => setRedraw(null)}>Keep these lands</button>
          <button type="button" className="primary" onClick={() => changeWorld(redraw)}>Redraw and relocate</button></>}>
        <p>The coastlines and homelands will change. These peoples keep their speech, kinship, and choices, and settle in the redrawn world.</p>
      </Modal> : null}
    </div>
  );
}

import { useEffect, useRef, useState, type CSSProperties } from "react";
import { Feather, Plus } from "lucide-react";
import { createEngine, message, presetDesign } from "../engine";
import type { Catalog, Engine, EthosAxis, FoundingPreview, Livelihood, MapSize, Naming, Overview, WorldMap } from "../model";
import { ETHOS_AXES, ETHOS_POLES, hue, LIVELIHOOD_NAME, temperament, TERRAIN_NAME } from "../lore";
import { worldName } from "../shelf";
import { Designer, randomSeed, type Founding as FoundingDesign } from "./Designer";
import { MapView, type MapCamera } from "./MapView";
import { Modal } from "./Modal";
import { decodeNaming, encodeNaming, namingChoices } from "./NamingSelect";
import { Phrase } from "./Phrase";
import { ReachOverlay } from "./ReachOverlay";
import { Specimen } from "./Specimen";
import "./founding.css";

const ORDINAL = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth", "eleventh", "twelfth"];
const FIRST_PEOPLES = 3;
const MOST_PEOPLES = 12;
const BENT = 0.7;
const LIVELIHOOD_PHRASE: Record<Livelihood, string> = { farming: "farming", herding: "herding", foraging: "foraging" };
const PEOPLE_LIVELIHOOD: Record<Livelihood, string> = { farming: "farmers", herding: "herders", foraging: "foragers" };

function lower(name: string): string {
  return name.charAt(0).toLowerCase() + name.slice(1);
}

/// A null land, livelihood, or bent lets the engine choose from the land.
interface Founder extends FoundingDesign {
  key: number;
  preset: string | null;
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

function drawFounder(catalog: Catalog, key: number): Founder {
  const preset = catalog.presets[Math.floor(Math.random() * catalog.presets.length)].id;
  const seed = randomSeed();
  const namingChoices: Naming[] = [
    { kind: "people" }, { kind: "speakers" },
    ...catalog.namePlaces.map((place) => ({ kind: "place" as const, place })),
    ...catalog.nameEpithets.map((epithet) => ({ kind: "epithet" as const, epithet })),
  ];
  const naming: Naming = key === 0 ? { kind: "people" } : namingChoices[Math.floor(Math.random() * namingChoices.length)];
  return { key, preset, seed, design: presetDesign(preset, seed), naming, power: 0.5, openness: 0.5, region: null, livelihood: null, bent: null };
}

/// Found for real behind the chart, then hand that same engine to the workshop.
export function Founding({ catalog, onBegin, onChartRoom, onSample }: {
  catalog: Catalog;
  onBegin: (engine: Engine, title: string | null, author: string | null) => void;
  onChartRoom: () => void;
  onSample: () => Promise<void>;
}) {
  const [sampling, setSampling] = useState(false);
  const [worldSeed, setWorldSeed] = useState(() => randomSeed());
  const [size, setSize] = useState<MapSize>("medium");
  const [founders, setFounders] = useState<Founder[]>(() => Array.from({ length: FIRST_PEOPLES }, (_, key) => drawFounder(catalog, key)));
  const [selected, setSelected] = useState<number | null>(0);
  const [title, setTitle] = useState<string | null>(null);
  const [author, setAuthor] = useState(() => {
    try { return localStorage.getItem("umran.author") ?? ""; }
    catch { return ""; }
  });
  const [camera, setCamera] = useState<MapCamera | undefined>();
  const [adjusting, setAdjusting] = useState(false);
  const [built, setBuilt] = useState<Built | null>(null);
  const [error, setError] = useState<string | null>(null);
  const engine = useRef<Engine | null>(null);
  const handedOver = useRef(false);
  const nextKey = useRef(FIRST_PEOPLES);
  const accounts = useRef<HTMLDivElement>(null);

  useEffect(() => {
    accounts.current?.querySelector(".account.chosen")?.scrollIntoView({ block: "nearest" });
  }, [selected, built?.overview.communities.length]);

  useEffect(() => {
    let live = true;
    createEngine(worldSeed, size).then((next) => {
      if (!live) return next.dispose();
      try {
        for (const founder of founders) {
          next.act({
            kind: "found", naming: founder.naming, design: founder.design, seed: founder.seed,
            power: founder.power, openness: founder.openness,
            ...(founder.region === null ? {} : { region: founder.region }),
            ...(founder.livelihood === null ? {} : { livelihood: founder.livelihood }),
            ...(founder.bent === null ? {} : { ethos: { [founder.bent.axis]: founder.bent.toward * BENT } }),
          });
        }
        const overview = next.overview(0);
        const founded: Built = { map: next.map(), overview, preview: next.foundingPreview(), founders, seed: worldSeed, size };
        engine.current?.dispose();
        engine.current = next;
        setBuilt(founded);
        setError(null);
        // Remember the engine's placement so editing one account leaves the rest alone.
        if (founders.some((founder) => founder.region === null)) {
          setFounders((all) => all.map((founder, i) => founder.region === null ? { ...founder, region: overview.communities[i].region } : founder));
        }
      } catch (failure) {
        next.dispose();
        setError(message(failure));
      }
    }, (failure) => { if (live) setError(message(failure)); });
    return () => { live = false; };
  }, [worldSeed, size, founders]);

  useEffect(() => () => { if (!handedOver.current) engine.current?.dispose(); }, []);

  const update = (i: number, patch: Partial<Founder>) => setFounders((all) => all.map((founder, j) => j === i ? { ...founder, ...patch } : founder));
  const rehome = () => {
    setCamera(undefined);
    setFounders((all) => all.map((founder) => ({ ...founder, region: null })));
  };
  const add = () => {
    if (founders.length >= MOST_PEOPLES) return;
    setFounders((all) => [...all, drawFounder(catalog, nextKey.current++)]);
    setSelected(founders.length);
  };
  const remove = () => {
    if (selected === null || founders.length <= 1) return;
    setFounders((all) => all.filter((_, i) => i !== selected));
    setSelected(Math.max(0, selected - 1));
  };

  const overview = built?.overview;
  const map = built?.map;
  const current = selected === null ? undefined : founders[selected];
  const currentBuild = built?.founders === founders && built.seed === worldSeed && built.size === size;
  const chartReady = !!map && !!overview && overview.communities.length === founders.length;
  const defaultTitle = overview ? worldName(overview) : null;
  const lands = map?.regions.filter((region) => region.terrain !== "sea").length ?? 0;
  const landName = (region: number) => overview?.places.find((place) => place.region === region)?.names.at(-1)?.spelled ?? "unnamed land";

  return (
    <div className="stage setup founding">
      <header className="founding-rail">
        <button type="button" className="brand" aria-label="Back to the chart room" onClick={onChartRoom}>
          <span className="brand-name"><span>ʿUmrān</span></span>
        </button>
      </header>
      <section className="stage-map" aria-label="Chart">
        {chartReady ? <>
          <MapView key={`${overview.seed}:${map.size}`} map={map} overview={overview} generation={0} tint={{ kind: "peoples" }}
            chosen={new Set(selected === null ? [] : [overview.communities[selected].id])}
            lands={new Set(current?.region === null || !current ? [] : [current.region])}
            focus={current?.region === null || !current ? null : map.regions[current.region].site}
            zoomable camera={camera} onCamera={setCamera} onPeople={setSelected}
            onLand={(region) => { if (map.regions[region].terrain !== "sea" && current && selected !== null) update(selected, { region }); }} />
          {selected !== null && built ? <ReachOverlay map={map} overview={overview} preview={built.preview}
            people={overview.communities[selected].id} camera={camera} /> : null}
        </> : null}
        <div className="cartouche">
          <div className="cartouche-kicker">A chart of</div>
          <input className="founding-title" aria-label="World name" value={title ?? defaultTitle ?? ""}
            placeholder="An unnamed world" onChange={(event) => setTitle(event.target.value)} />
          <label className="founding-by">by <input className="founding-author" aria-label="Author"
            placeholder="Your name" value={author} onChange={(event) => setAuthor(event.target.value)} /></label>
          <p className="cartouche-note">{lands} lands, as the first travellers drew them · seed {worldSeed}</p>
          <p className="map-scale-note">{catalog.mapSizes.find((choice) => choice.id === size)?.description}</p>
          <div className="cartouche-tools">
            <span className="sizes" role="radiogroup" aria-label="How wide">
              {catalog.mapSizes.map((choice) => <button key={choice.id} type="button" className="link" role="radio"
                aria-checked={choice.id === size} title={choice.description} onClick={() => { setSize(choice.id as MapSize); rehome(); }}>{choice.name}</button>)}
            </span>
          </div>
          <div className="cartouche-tools"><button type="button" className="link" onClick={() => { setWorldSeed(randomSeed()); rehome(); }}>Redraw the coasts</button></div>
        </div>
        <p className="map-hint">Choose an account, then touch a land to set its people there.</p>
      </section>

      <aside className="pedia setup-panel" aria-label="Book of accounts" aria-busy={!currentBuild}>
        <header className="book-head">
          <div className="book-kicker">Book I</div>
          <h2>Of the peoples at the beginning</h2>
          <p>What travellers report before any year is counted: where each people lives, how they live, and how they speak.</p>
          <p className="sample-note">Or <button type="button" className="link" disabled={sampling} onClick={() => {
            setSampling(true);
            setTimeout(() => void onSample().finally(() => setSampling(false)), 30);
          }}>{sampling ? "writing four thousand years…" : "read a chronicle already written"}</button>.</p>
        </header>
        <div className="founding-accounts" ref={accounts}>
          {founders.map((founder, i) => {
            const people = overview?.communities[i];
            const speech = people ? overview?.varieties.find((variety) => variety.id === people.variety) : undefined;
            const chosen = selected === i;
            const speechPhrase = founder.preset === null ? "their own speech, shaped by hand" : lower(catalog.presets.find((preset) => preset.id === founder.preset)!.name);
            const description = people ? `${PEOPLE_LIVELIHOOD[people.livelihood]} of ${landName(people.region)} · ${speechPhrase}` : "Settling…";
            return <article className={`account${chosen ? " chosen" : ""}`} key={founder.key}
              style={{ "--tone": speech ? hue(speech.family) : undefined } as CSSProperties}>
              <button type="button" className="account-line" id={`founding-account-${founder.key}`} aria-expanded={chosen}
                aria-controls={`founding-stages-${founder.key}`} title={people ? `${people.name} · ${people.meaning} · ${description}` : undefined}
                onClick={() => setSelected(chosen ? null : i)}>
                <span className="account-ordinal" aria-label={`The ${ORDINAL[i]} account`}>{String(i + 1).padStart(2, "0")}</span>
                <span className="account-summary"><span className={speech ? `account-people hand-${speech.family % 5}` : "account-people"}>{people?.name ?? `The ${ORDINAL[i]} people`}</span>
                  {people ? <> <span className="meaning">· {people.meaning}</span></> : null} <span className="account-description">· {description}</span>
                </span>
              </button>
              {chosen && people && speech && map ? <div className="account-stages" id={`founding-stages-${founder.key}`} role="region" aria-labelledby={`founding-account-${founder.key}`}>
                <section className="founding-stage">
                  <h3 className="eyebrow">1 · Homeland</h3>
                  <p className="account-text">They live in <Phrase label="Homeland" value={String(founder.region ?? people.region)}
                    choices={map.regions.filter((region) => region.terrain !== "sea").map((region) => ({ key: String(region.id),
                      text: landName(region.id) === "unnamed land" ? `land ${region.id + 1}` : landName(region.id), title: lower(TERRAIN_NAME[region.terrain]) }))}
                    onChange={(key) => update(i, { region: Number(key) })} />, {lower(TERRAIN_NAME[map.regions[people.region].terrain])}, {map.regions[people.region].coastal ? "coastal" : "inland"}.</p>
                  <p className="muted small">Choose a land here or on the chart. Hover a traced journey for its effort; crossing water would need boats.</p>
                </section>
                <section className="founding-stage">
                  <h3 className="eyebrow">2 · Livelihood</h3>
                  <p className="account-text">They live <Phrase label="Way of life" value={founder.livelihood ?? ""}
                    choices={[{ key: "", text: `as their land suggests (${LIVELIHOOD_PHRASE[people.livelihood]})` },
                      ...(Object.keys(LIVELIHOOD_NAME) as Livelihood[]).map((livelihood) => ({ key: livelihood, text: `by ${LIVELIHOOD_PHRASE[livelihood]}` }))]}
                    onChange={(key) => update(i, { livelihood: key === "" ? null : key as Livelihood })} />.</p>
                </section>
                <section className="founding-stage">
                  <h3 className="eyebrow">3 · Temper</h3>
                  <p className="account-text">Among them, <Phrase label="Temper" value={founder.bent ? `${founder.bent.axis} ${founder.bent.toward}` : ""}
                    choices={[{ key: "", text: "as their land and life make them" }, ...ETHOS_AXES.flatMap((axis) => [1, -1].map((toward) => ({ key: `${axis} ${toward}`, text: ETHOS_POLES[axis][toward > 0 ? 1 : 0] })))]}
                    onChange={(key) => { const [axis, toward] = key.split(" "); update(i, { bent: key === "" ? null : { axis: axis as EthosAxis, toward: Number(toward) as 1 | -1 } }); }} />
                    {founder.bent === null ? ` (${temperament(people.ethos, 2).join(" and ") || "even-tempered"})` : ""}.</p>
                </section>
                <section className="founding-stage">
                  <h3 className="eyebrow">4 · Speech</h3>
                  <Specimen words={speech.specimen} />
                  <p className="account-text">Their speech is <Phrase label="Sounds" value={founder.preset ?? ""}
                    choices={[...(founder.preset === null ? [{ key: "", text: "their own, shaped by hand" }] : []),
                      ...catalog.presets.map((preset) => ({ key: preset.id, text: lower(preset.name), title: preset.description }))]}
                    onChange={(key) => { if (key && key !== founder.preset) update(i, { preset: key, design: presetDesign(key, founder.seed) }); }} />.</p>
                  <div className="account-acts">
                    <button type="button" className="link" onClick={() => { const seed = randomSeed(); update(i, { seed, design: founder.preset === null ? founder.design : presetDesign(founder.preset, seed) }); }}>Hear other words</button>
                    <button type="button" className="link" onClick={() => setAdjusting(true)}>Adjust their sounds…</button>
                  </div>
                </section>
                <section className="founding-stage">
                  <h3 className="eyebrow">5 · Identity</h3>
                  <p className="account-text">They name themselves <Phrase label="Name" value={encodeNaming(founder.naming)} choices={namingChoices(catalog)}
                    onChange={(key) => { const naming = decodeNaming(key); if (naming) update(i, { naming }); }} />, <b>{people.name}</b>. They call their speech <i>{speech.name}</i>.</p>
                </section>
              </div> : chosen ? <p className="muted">Settling…</p> : null}
            </article>;
          })}
        </div>
        <div className="founding-account-acts">
          <button type="button" className="link" disabled={founders.length >= MOST_PEOPLES} onClick={add}><Plus size={13} aria-hidden="true" /> Another people</button>
          {selected !== null && founders.length > 1 ? <button type="button" className="link" onClick={remove}>Leave this people out</button> : null}
        </div>
      </aside>

      <footer className="timebar setup-foot">
        <span className="year">{error ? <span className="error">{error}</span> : <><strong>Year 0.</strong> {founders.length} peoples settle {defaultTitle ?? "the world"}; nothing is written yet.</>}</span>
        <button type="button" className="primary begin" disabled={!engine.current || !currentBuild || !chartReady} onClick={() => {
          if (!engine.current || !currentBuild) return;
          try {
            handedOver.current = true;
            onBegin(engine.current, title?.trim() && title.trim() !== defaultTitle ? title.trim() : null, author.trim() || null);
          } catch (failure) { handedOver.current = false; setError(message(failure)); }
        }}><Feather size={18} aria-hidden="true" /> Begin</button>
      </footer>
      {adjusting && current && selected !== null ? <Modal open wide title="Adjust their sounds" onClose={() => setAdjusting(false)}>
        <Designer catalog={catalog} submit="Use these sounds" initial={current} onCancel={() => setAdjusting(false)}
          onFound={(founder) => { setAdjusting(false); update(selected, { ...founder, preset: null }); }} />
      </Modal> : null}
    </div>
  );
}

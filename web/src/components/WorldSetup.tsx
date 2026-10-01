import { useEffect, useRef, useState } from "react";
import { Dices, Play, Plus, SlidersHorizontal, Sparkles, X } from "lucide-react";
import { createEngine, message, presetDesign } from "../engine";
import type { Catalog, Engine, MapSize, Naming, Overview, WorldMap } from "../model";
import { hue, TERRAIN_NAME } from "../lore";
import { Designer, randomSeed, type Founding } from "./Designer";
import { MapView } from "./MapView";
import { Modal } from "./Modal";
import { NamingSelect } from "./NamingSelect";
import { Specimen } from "./Specimen";

const SIZES: { id: MapSize; name: string; title: string }[] = [
  { id: "small", name: "Small", title: "About forty lands; peoples soon meet." },
  { id: "medium", name: "Middling", title: "About eighty lands." },
  { id: "large", name: "Wide", title: "About a hundred and fifty lands; peoples keep apart longer." },
];

/// Peoples a new world starts with, and the most it can.
const FIRST_PEOPLES = 3;
const MOST_PEOPLES = 8;

/// A people to found: its language's design and seed, what it calls
/// itself, and where it lives. `preset` is the starting sound it was drawn
/// from, or `null` once its sounds were adjusted by hand; `region` is
/// `null` until the world has chosen a land for it.
interface Founder extends Founding {
  key: number;
  preset: string | null;
  region: number | null;
}

/// The world as the founders would leave it in year 0.
interface Built {
  map: WorldMap;
  overview: Overview;
}

function pick<T>(list: T[]): T {
  return list[Math.floor(Math.random() * list.length)];
}

function anyNaming(catalog: Catalog): Naming {
  return pick<Naming>([
    { kind: "people" },
    { kind: "speakers" },
    ...catalog.namePlaces.map((place) => ({ kind: "place" as const, place })),
    ...catalog.nameEpithets.map((epithet) => ({ kind: "epithet" as const, epithet })),
  ]);
}

function drawFounder(catalog: Catalog, key: number): Founder {
  const preset = pick(catalog.presets).id;
  const seed = randomSeed();
  return {
    key,
    preset,
    seed,
    design: presetDesign(preset, seed),
    // Most peoples call themselves simply "the people".
    naming: key === 0 ? { kind: "people" } : anyNaming(catalog),
    power: 0.5,
    openness: 0.5,
    region: null,
  };
}

/// A new world, set up on its map: how wide it is, who lives in it, how
/// each people sounds, and where each one starts. Every choice is shown at
/// once as the world would begin, with real names and words, because the
/// world is founded for real behind the scenes and handed over on Begin.
export function WorldSetup({
  catalog,
  onBegin,
  onShelf,
  onSample,
}: {
  catalog: Catalog;
  onBegin: (engine: Engine) => void;
  /// Back to the shelf, if there are worlds on it.
  onShelf?: () => void;
  /// Open the sample world, already thousands of years along.
  onSample: () => Promise<void>;
}) {
  const [sampling, setSampling] = useState(false);
  const [worldSeed, setWorldSeed] = useState(() => randomSeed());
  const [size, setSize] = useState<MapSize>("medium");
  const [founders, setFounders] = useState<Founder[]>(() =>
    Array.from({ length: FIRST_PEOPLES }, (_, key) => drawFounder(catalog, key)),
  );
  const [selected, setSelected] = useState(0);
  const [adjusting, setAdjusting] = useState(false);
  const [built, setBuilt] = useState<Built | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The engine behind `built`; disposed when replaced, unless handed over.
  const engine = useRef<Engine | null>(null);
  const handedOver = useRef(false);
  const nextKey = useRef(FIRST_PEOPLES);

  useEffect(() => {
    let live = true;
    createEngine(worldSeed, size).then(
      (next) => {
        if (!live) return next.dispose();
        try {
          for (const f of founders) {
            next.act({
              kind: "found",
              naming: f.naming,
              design: f.design,
              seed: f.seed,
              power: f.power,
              openness: f.openness,
              ...(f.region === null ? {} : { region: f.region }),
            });
          }
        } catch (e) {
          next.dispose();
          setError(message(e));
          return;
        }
        const overview = next.overview(0);
        engine.current?.dispose();
        engine.current = next;
        setBuilt({ map: next.map(), overview });
        setError(null);
        // Keep the lands the world chose, so later choices leave them be.
        if (founders.some((f) => f.region === null)) {
          setFounders((all) => all.map((f, i) => (f.region === null ? { ...f, region: overview.communities[i].region } : f)));
        }
      },
      (e) => setError(message(e)),
    );
    return () => {
      live = false;
    };
  }, [worldSeed, size, founders]);

  useEffect(
    () => () => {
      if (!handedOver.current) engine.current?.dispose();
    },
    [],
  );

  const update = (i: number, patch: Partial<Founder>) =>
    setFounders((all) => all.map((f, j) => (j === i ? { ...f, ...patch } : f)));
  // A new world or a new size draws new lands for everyone.
  const rehome = () => setFounders((all) => all.map((f) => ({ ...f, region: null })));
  const add = () => {
    setFounders((all) => [...all, drawFounder(catalog, nextKey.current++)]);
    setSelected(founders.length);
  };
  const remove = (i: number) => {
    setFounders((all) => all.filter((_, j) => j !== i));
    setSelected((s) => Math.max(0, s >= i ? s - 1 : s));
  };

  const overview = built?.overview;
  const map = built?.map;
  const current = founders[selected];
  const lands = map ? map.regions.filter((r) => r.terrain !== "sea").length : 0;
  const landName = (region: number) =>
    overview?.places.find((p) => p.region === region)?.names.at(-1)?.spelled ?? "unnamed land";

  return (
    <div className="stage setup">
      <header className="stage-head">
        <nav>
          {onShelf ? (
            <button type="button" className="link brand" onClick={onShelf} title="Back to the shelf">
              Umran
            </button>
          ) : (
            <span className="brand">Umran</span>
          )}
        </nav>
        <span className="stage-title">A new world</span>
      </header>

      <section className="stage-map" aria-label="Map">
        {map && overview && overview.communities.length === founders.length ? (
          <MapView
            map={map}
            overview={overview}
            generation={0}
            tint={{ kind: "peoples" }}
            chosen={new Set([selected])}
            lands={new Set(current?.region === null || !current ? [] : [current.region])}
            zoomable
            onPeople={setSelected}
            onLand={(region) => {
              if (map.regions[region].terrain !== "sea" && current) update(selected, { region });
            }}
          />
        ) : null}
        <p className="map-hint">
          Choose a people on the right, then click a land to move them there.
        </p>
      </section>

      <aside className="pedia setup-panel" aria-label="Peoples">
        <button
          type="button"
          className="sample-callout"
          disabled={sampling}
          onClick={() => {
            setSampling(true);
            // Let the button say it is working before the engine, which
            // blocks the page while it plays the years, starts.
            setTimeout(() => void onSample().finally(() => setSampling(false)), 30);
          }}
        >
          <Sparkles size={20} aria-hidden="true" />
          <span>
            <strong>{sampling ? "Playing four thousand years…" : "Watch a sample world"}</strong>
            <span>Three peoples, four thousand years on: their families spread, one rules another, and a people changes its language. Or make your own below.</span>
          </span>
        </button>
        <section className="setup-world">
          <h3>The world</h3>
          <div className="row spread">
            <div className="segmented" role="radiogroup" aria-label="How wide">
              {SIZES.map((s) => (
                <button
                  key={s.id}
                  type="button"
                  role="radio"
                  aria-checked={s.id === size}
                  title={s.title}
                  onClick={() => {
                    setSize(s.id);
                    rehome();
                  }}
                >
                  {s.name}
                </button>
              ))}
            </div>
            <button
              type="button"
              onClick={() => {
                setWorldSeed(randomSeed());
                rehome();
              }}
            >
              <Dices size={16} aria-hidden="true" /> Another world
            </button>
          </div>
          <p className="muted small">
            {lands} lands. Where peoples start and how they sound shapes everything after.
          </p>
        </section>

        <h3>Peoples</h3>
        <ol className="founders">
          {founders.map((f, i) => {
            const c = overview?.communities[i];
            const v = c ? overview?.varieties[c.variety] : undefined;
            const chosen = i === selected;
            return (
              <li key={f.key} className={chosen ? "founder chosen" : "founder"}>
                <div className="founder-head">
                  <button type="button" className="founder-name" onClick={() => setSelected(i)} aria-expanded={chosen}>
                    {c && v ? (
                      <>
                        <strong style={{ color: hue(v.family) }}>{c.name}</strong>{" "}
                        <span className="muted">“{c.meaning}”</span>
                      </>
                    ) : (
                      <span className="muted">Settling…</span>
                    )}
                  </button>
                  {founders.length > 1 ? (
                    <button type="button" className="icon quiet" title="Leave them out" onClick={() => remove(i)}>
                      <X size={15} />
                    </button>
                  ) : null}
                </div>
                {c && v && map ? (
                  <>
                    <p className="muted small">
                      Speaking <i>{v.name}</i>, on {map.regions[c.region].coastal ? "coastal " : ""}
                      {TERRAIN_NAME[map.regions[c.region].terrain].toLowerCase()} in <i>{landName(c.region)}</i>
                    </p>
                    <Specimen words={v.specimen} />
                  </>
                ) : null}
                {chosen ? (
                  <div className="founder-controls">
                    <label>
                      Sounds
                      <select
                        value={f.preset ?? ""}
                        onChange={(e) => update(i, { preset: e.target.value, design: presetDesign(e.target.value, f.seed) })}
                      >
                        {f.preset === null ? <option value="">Their own, adjusted by hand</option> : null}
                        {catalog.presets.map((p) => (
                          <option key={p.id} value={p.id} title={p.description}>
                            {p.name}
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      They call themselves
                      <NamingSelect catalog={catalog} value={f.naming} onChange={(n) => n && update(i, { naming: n })} />
                    </label>
                    <div className="row">
                      <button
                        type="button"
                        onClick={() => {
                          const seed = randomSeed();
                          update(i, { seed, design: f.preset === null ? f.design : presetDesign(f.preset, seed) });
                        }}
                      >
                        <Dices size={16} aria-hidden="true" /> Other words
                      </button>
                      <button type="button" onClick={() => setAdjusting(true)}>
                        <SlidersHorizontal size={16} aria-hidden="true" /> Adjust the sounds…
                      </button>
                    </div>
                  </div>
                ) : null}
              </li>
            );
          })}
        </ol>
        {founders.length < MOST_PEOPLES ? (
          <button type="button" className="add-founder" onClick={add}>
            <Plus size={16} aria-hidden="true" /> Another people
          </button>
        ) : null}
      </aside>

      <footer className="timebar setup-foot">
        <span className="muted">
          {error ? <span className="error">{error}</span> : "Time begins in year 0. Once begun, press play and watch."}
        </span>
        <span className="row">
          {onShelf ? (
            <button type="button" onClick={onShelf}>
              Back to the shelf
            </button>
          ) : null}
          <button
            type="button"
            className="primary"
            disabled={!engine.current || !overview || overview.communities.length !== founders.length}
            onClick={() => {
              if (!engine.current) return;
              handedOver.current = true;
              onBegin(engine.current);
            }}
          >
            <Play size={16} aria-hidden="true" /> Begin
          </button>
        </span>
      </footer>

      {adjusting && current ? (
        <Modal open wide title="Adjust the sounds" onClose={() => setAdjusting(false)}>
          <Designer
            catalog={catalog}
            submit="Use these sounds"
            initial={current}
            onCancel={() => setAdjusting(false)}
            onFound={(f) => {
              setAdjusting(false);
              update(selected, { ...f, preset: null });
            }}
          />
        </Modal>
      ) : null}
    </div>
  );
}

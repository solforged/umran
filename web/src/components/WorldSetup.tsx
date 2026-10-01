import { useEffect, useRef, useState, type CSSProperties } from "react";
import { Feather, Plus } from "lucide-react";
import { createEngine, message, presetDesign } from "../engine";
import type { Catalog, Engine, Livelihood, MapSize, Naming, Overview, Terrain, WorldMap } from "../model";
import { hue, LIVELIHOOD_NAME, TERRAIN_NAME } from "../lore";
import { Designer, randomSeed, type Founding } from "./Designer";
import { MapView } from "./MapView";
import { Modal } from "./Modal";
import { NamingSelect } from "./NamingSelect";
import { Specimen } from "./Specimen";

const SIZES: { id: MapSize; name: string; title: string }[] = [
  { id: "small", name: "small", title: "About forty lands; peoples soon meet." },
  { id: "medium", name: "middling", title: "About eighty lands." },
  { id: "large", name: "wide", title: "About a hundred and fifty lands; peoples keep apart longer." },
];

/// Each account is numbered as a historian would: the first, the second.
const ORDINAL = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth"];

/// Whether a people lives on a land or in it.
const AMID: Record<Terrain, string> = {
  plains: "on the",
  steppe: "on the",
  forest: "in the",
  hills: "in the",
  mountains: "in the",
  desert: "in the",
  sea: "on the",
};

/// A choice's name as it reads mid-sentence.
function lower(name: string): string {
  return name.charAt(0).toLowerCase() + name.slice(1);
}

/// Peoples a new world starts with, and the most it can.
const FIRST_PEOPLES = 3;
const MOST_PEOPLES = 8;

/// A people to found: its language's design and seed, what it calls
/// itself, and where it lives. `preset` is the starting sound it was drawn
/// from, or `null` once its sounds were adjusted by hand; `region` is
/// `null` until the world has chosen a land for it. A null `livelihood`
/// lets that land choose how the people feeds itself.
interface Founder extends Founding {
  key: number;
  preset: string | null;
  region: number | null;
  livelihood: Livelihood | null;
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
    livelihood: null,
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
              ...(f.livelihood === null ? {} : { livelihood: f.livelihood }),
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
        <div className="cartouche">
          <div className="cartouche-kicker">A chart of</div>
          <h1>The world before the chronicle</h1>
          <p className="cartouche-note">
            {lands} lands, as the first travellers drew them · seed {worldSeed}
          </p>
          <div className="cartouche-tools">
            <span className="sizes" role="radiogroup" aria-label="How wide">
              {SIZES.map((s) => (
                <button
                  key={s.id}
                  type="button"
                  className="link"
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
            </span>
            <button
              type="button"
              className="link"
              onClick={() => {
                setWorldSeed(randomSeed());
                rehome();
              }}
            >
              Redraw the coasts
            </button>
          </div>
        </div>
        <p className="map-hint">Choose an account, then touch a land to set its people there.</p>
      </section>

      <aside className="pedia setup-panel" aria-label="Peoples">
        <header className="book-head">
          <div className="book-kicker">Book I</div>
          <h2>Of the peoples at the beginning</h2>
          <p>
            What travellers report of those who live here before any year is counted: what they call themselves, how
            they live, and how they speak.
          </p>
          <p className="sample-note">
            Or{" "}
            <button
              type="button"
              className="link"
              disabled={sampling}
              onClick={() => {
                setSampling(true);
                // Let the link say it is working before the engine, which
                // blocks the page while it plays the years, starts.
                setTimeout(() => void onSample().finally(() => setSampling(false)), 30);
              }}
            >
              {sampling ? "playing four thousand years…" : "read a chronicle already written"}
            </button>
            : three peoples, four thousand years on, one ruling another and a faith born among the ruled.
          </p>
        </header>

        <ol className="accounts">
          {founders.map((f, i) => {
            const c = overview?.communities[i];
            const v = c ? overview?.varieties[c.variety] : undefined;
            const chosen = i === selected;
            const where =
              c && map
                ? `${AMID[map.regions[c.region].terrain]} ${map.regions[c.region].coastal ? "coastal " : ""}${TERRAIN_NAME[map.regions[c.region].terrain].toLowerCase()} of `
                : "";
            return (
              <li
                key={f.key}
                className={chosen ? "account chosen" : "account"}
                style={v ? ({ "--tone": hue(v.family) } as CSSProperties) : undefined}
              >
                <div className="account-top">
                  <span className="account-number">The {ORDINAL[i]} account</span>
                  {founders.length > 1 ? (
                    <button type="button" className="link strike" title="Leave this people out" onClick={() => remove(i)}>
                      strike out
                    </button>
                  ) : null}
                </div>
                <button type="button" className="account-name" onClick={() => setSelected(i)} aria-expanded={chosen}>
                  {c && v ? (
                    <>
                      <span className={`hand-${v.family % 5}`}>{c.name}</span> <span className="meaning">“{c.meaning}”</span>
                    </>
                  ) : (
                    <span className="muted">Settling…</span>
                  )}
                </button>
                {c && v && map ? (
                  chosen ? (
                    <p className="account-text">
                      They call themselves <b>{c.name}</b>,{" "}
                      <NamingSelect catalog={catalog} value={f.naming} onChange={(n) => n && update(i, { naming: n })} />.
                      They live as{" "}
                      <select
                        aria-label="Way of life"
                        value={f.livelihood ?? ""}
                        onChange={(e) => update(i, { livelihood: e.target.value === "" ? null : (e.target.value as Livelihood) })}
                      >
                        <option value="">{lower(LIVELIHOOD_NAME[c.livelihood])}, as the land suits</option>
                        {(Object.keys(LIVELIHOOD_NAME) as Livelihood[]).map((livelihood) => (
                          <option key={livelihood} value={livelihood}>
                            {lower(LIVELIHOOD_NAME[livelihood])}
                          </option>
                        ))}
                      </select>{" "}
                      {where}
                      <i>{landName(c.region)}</i>. Their speech, <i>{v.name}</i>, is{" "}
                      <select
                        aria-label="Sounds"
                        value={f.preset ?? ""}
                        onChange={(e) => update(i, { preset: e.target.value, design: presetDesign(e.target.value, f.seed) })}
                      >
                        {f.preset === null ? <option value="">their own, shaped by hand</option> : null}
                        {catalog.presets.map((p) => (
                          <option key={p.id} value={p.id} title={p.description}>
                            {lower(p.name)}
                          </option>
                        ))}
                      </select>
                      .
                    </p>
                  ) : (
                    <p className="account-text">
                      {LIVELIHOOD_NAME[c.livelihood]} {where}
                      <i>{landName(c.region)}</i>, speaking <i>{v.name}</i>.
                    </p>
                  )
                ) : null}
                {v ? <Specimen words={v.specimen} /> : null}
                {chosen ? (
                  <div className="account-acts">
                    <button
                      type="button"
                      className="link"
                      onClick={() => {
                        const seed = randomSeed();
                        update(i, { seed, design: f.preset === null ? f.design : presetDesign(f.preset, seed) });
                      }}
                    >
                      Hear other words
                    </button>
                    <button type="button" className="link" onClick={() => setAdjusting(true)}>
                      Adjust their sounds…
                    </button>
                  </div>
                ) : null}
              </li>
            );
          })}
        </ol>
        {founders.length < MOST_PEOPLES ? (
          <button type="button" className="link add-account" onClick={add}>
            <Plus size={15} aria-hidden="true" /> Add an account of another people
          </button>
        ) : null}
      </aside>

      <footer className="timebar setup-foot">
        <span className="year">
          {error ? (
            <span className="error">{error}</span>
          ) : (
            <>
              <strong>Year 0.</strong> Nothing is written yet; once begun, the years run and the chronicle fills.
            </>
          )}
        </span>
        <span className="row">
          {onShelf ? (
            <button type="button" className="link" onClick={onShelf}>
              Back to the shelf
            </button>
          ) : null}
          <button
            type="button"
            className="primary begin"
            disabled={!engine.current || !overview || overview.communities.length !== founders.length}
            onClick={() => {
              if (!engine.current) return;
              handedOver.current = true;
              onBegin(engine.current);
            }}
          >
            <Feather size={18} aria-hidden="true" /> Begin the chronicle
          </button>
        </span>
      </footer>

      {adjusting && current ? (
        <Modal open wide title="Adjust their sounds" onClose={() => setAdjusting(false)}>
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

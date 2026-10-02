import { useEffect, useRef, useState, type CSSProperties } from "react";
import { Feather, Plus } from "lucide-react";
import { createEngine, message, presetDesign } from "../engine";
import type { Catalog, Engine, EthosAxis, FoundingPreview, Livelihood, MapSize, Naming, Overview, Subject, WorldMap } from "../model";
import { ETHOS_AXES, ETHOS_POLES, hue, LIVELIHOOD_NAME, temperament, TERRAIN_NAME } from "../lore";
import { Designer, randomSeed, type Founding } from "./Designer";
import { MapView } from "./MapView";
import { Modal } from "./Modal";
import { decodeNaming, encodeNaming, namingChoices } from "./NamingSelect";
import { Phrase } from "./Phrase";
import { Specimen } from "./Specimen";
import { makeNote } from "./Notebook";

/// Each account is numbered as a historian would: the first, the second.
const ORDINAL = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth"];

/// A choice's name as it reads mid-sentence.
function lower(name: string): string {
  return name.charAt(0).toLowerCase() + name.slice(1);
}

/// Peoples a new world starts with, and the most it can.
const FIRST_PEOPLES = 3;
const MOST_PEOPLES = 8;

/// How strongly a people leans when its account names its temper.
const BENT = 0.7;

/// A people to found: its language's design and seed, what it calls
/// itself, and where it lives. `preset` is the starting sound it was drawn
/// from, or `null` once its sounds were adjusted by hand; `region` is
/// `null` until the world has chosen a land for it. A null `livelihood`
/// lets that land choose how the people feeds itself, and a null `bent`
/// lets its land and life shape its temper.
interface Founder extends Founding {
  key: number;
  preset: string | null;
  region: number | null;
  livelihood: Livelihood | null;
  bent: { axis: EthosAxis; toward: 1 | -1 } | null;
}

/// The world as the founders would leave it in year 0.
interface Built {
  map: WorldMap;
  overview: Overview;
  preview: FoundingPreview;
  founders: Founder[];
  seed: number;
  size: MapSize;
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
    bent: null,
  };
}

type FoundingPair = FoundingPreview["pairs"][number];
const REACH_GROUPS = [
  { id: "walking", title: "Within walking reach" },
  { id: "sea", title: "Across the water" },
  { id: "apart", title: "Far apart" },
] as const;
const LIVELIHOOD_PHRASE: Record<Livelihood, string> = {
  farming: "farming",
  herding: "herding",
  foraging: "foraging",
};

function pairEvidence(pair: FoundingPair): string {
  const routes = [
    pair.walk === null ? null : `${Math.round(pair.walk).toLocaleString()} effort-km on foot`,
    pair.voyage === null ? null : `${Math.round(pair.voyage).toLocaleString()} effort-km by sea`,
  ].filter(Boolean).join("; ");
  if (pair.reach === "neighbours") return `may meet as neighbours · ${routes}`;
  if (pair.reach === "walking") return `may meet on foot · ${routes}`;
  if (pair.reach === "sea") return `would need boats · ${routes}`;
  return routes ? `too far for first journeys · ${routes}` : "no way between them by land or along a coast";
}

function placeName(overview: Overview, region: number): string {
  return overview.places.find((p) => p.region === region)?.names.at(-1)?.spelled ?? "unnamed land";
}

/// Questions use the engine's people, places, routes, and existing specimens.
/// Their subjects become exact notebook destinations only when Begin is pressed.
function foundingQuestions(overview: Overview, preview: FoundingPreview): { title: string; label: string; subject: Subject }[] {
  const questions: { title: string; label: string; subject: Subject }[] = [];
  for (const pair of preview.pairs.slice(0, 2)) {
    const a = overview.communities.find((c) => c.id === pair.a)!;
    const b = overview.communities.find((c) => c.id === pair.b)!;
    questions.push({
      title: pair.reach === "sea" ? `Will the ${a.name} and the ${b.name} meet across the water?`
        : pair.reach === "apart" ? `Will the ${a.name} and the ${b.name} ever meet?`
          : `Will the ${a.name} and the ${b.name} meet in ${placeName(overview, a.region)}?`,
      label: a.name, subject: { kind: "people", id: a.id },
    });
  }
  const coast = preview.peoples.find((p) => p.coastal);
  const first = overview.communities[0];
  if (!first) return questions;
  const land = coast ? overview.communities.find((c) => c.id === coast.community)! : first;
  questions.push({
    title: coast ? `Who will first cross the water from ${placeName(overview, land.region)}?`
      : `Who will come to live in ${placeName(overview, land.region)}?`,
    label: placeName(overview, land.region), subject: { kind: "land", region: land.region },
  });
  const language = overview.varieties.find((v) => v.id === first.variety)!;
  const water = language.specimen.find((word) => word.concept === "water");
  if (water) questions.push({
    title: `Will the ${first.name}'s word for water stay ${water.spelled}?`,
    label: `${language.name} · water`, subject: { kind: "word", variety: language.id, concept: water.concept },
  });
  questions.push({
    title: `How will ${language.name} sound as the years pass?`,
    label: language.name, subject: { kind: "language", variety: language.id },
  });
  return questions.slice(0, 5);
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
  /// Back to the shelf, even before the first world is begun.
  onShelf: () => void;
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
  const [charter, setCharter] = useState(false);
  const [keepQuestions, setKeepQuestions] = useState(true);
  const [adjusting, setAdjusting] = useState(false);
  const [built, setBuilt] = useState<Built | null>(null);
  const [error, setError] = useState<string | null>(null);
  // The engine behind `built`; disposed when replaced, unless handed over.
  const engine = useRef<Engine | null>(null);
  const handedOver = useRef(false);
  const nextKey = useRef(FIRST_PEOPLES);
  const book = useRef<HTMLElement>(null);

  useEffect(() => { book.current?.scrollTo(0, 0); }, [selected, charter]);

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
              ...(f.bent === null ? {} : { ethos: { [f.bent.axis]: f.bent.toward * BENT } }),
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
        setBuilt({ map: next.map(), overview, preview: next.foundingPreview(), founders, seed: worldSeed, size });
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
    setCharter(false);
  };
  const remove = (i: number) => {
    setFounders((all) => all.filter((_, j) => j !== i));
    setSelected((s) => Math.max(0, s >= i ? s - 1 : s));
  };

  const overview = built?.overview;
  const map = built?.map;
  const current = founders[selected];
  const c = overview?.communities[selected];
  const v = c ? overview?.varieties[c.variety] : undefined;
  const currentBuild = built?.founders === founders && built.seed === worldSeed && built.size === size;
  const questions = built ? foundingQuestions(built.overview, built.preview) : [];
  const openAccount = (i: number) => { setSelected(i); setCharter(false); };
  const lands = map ? map.regions.filter((r) => r.terrain !== "sea").length : 0;
  const landName = (region: number) => overview ? placeName(overview, region) : "unnamed land";

  return (
    <div className="stage setup">
      <header className="stage-head">
        <nav>
          <button type="button" className="link brand" onClick={onShelf} title="Back to the shelf" aria-label="Back to the shelf">
            <span className="brand-name"><span>ʿUmrān</span></span>
          </button>
        </nav>
      </header>

      <section className="stage-map" aria-label="Map">
        {map && overview && overview.communities.length === founders.length ? (
          <MapView
            key={`${overview.seed}:${map.size}`}
            map={map}
            overview={overview}
            generation={0}
            tint={{ kind: "peoples" }}
            chosen={new Set([selected])}
            lands={new Set(current?.region === null || !current ? [] : [current.region])}
            focus={current?.region === null || !current ? null : map.regions[current.region].site}
            zoomable
            onPeople={openAccount}
            onLand={(region) => {
              if (map.regions[region].terrain !== "sea" && current) {
                setCharter(false);
                update(selected, { region });
              }
            }}
          />
        ) : null}
        <div className="cartouche">
          <div className="cartouche-kicker">A chart of</div>
          <h1>The world before the chronicle</h1>
          <p className="cartouche-note">
            {lands} lands, as the first travellers drew them · seed {worldSeed}
          </p>
          <p className="map-scale-note">{catalog.mapSizes.find((s) => s.id === size)?.description}</p>
          <div className="cartouche-tools">
            <span className="sizes" role="radiogroup" aria-label="How wide">
              {catalog.mapSizes.map((s) => (
                <button
                  key={s.id}
                  type="button"
                  className="link"
                  role="radio"
                  aria-checked={s.id === size}
                  title={s.description}
                  onClick={() => {
                    setSize(s.id as MapSize);
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

      <aside ref={book} className="pedia setup-panel" aria-label="Book of accounts" aria-busy={!currentBuild}>
        <div className="founding-contents">
          <div role="tablist" aria-label="Founding accounts" className="founding-tabs" onKeyDown={(event) => {
            const tabs = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]'));
            const index = tabs.indexOf(event.target as HTMLButtonElement);
            if (index < 0) return;
            const next = event.key === "ArrowRight" ? (index + 1) % tabs.length
              : event.key === "ArrowLeft" ? (index + tabs.length - 1) % tabs.length
                : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : null;
            if (next === null) return;
            event.preventDefault();
            tabs[next].click();
            tabs[next].focus();
          }}>
            {founders.map((f, i) => {
              const people = overview?.communities[i];
              const speech = people ? overview?.varieties[people.variety] : undefined;
              return <button type="button" role="tab" key={f.key} id={`founding-tab-${f.key}`}
                aria-controls={`founding-account-${f.key}`} aria-selected={!charter && selected === i}
                tabIndex={!charter && selected === i ? 0 : -1} onClick={() => openAccount(i)}>
                <span className="swatch" style={speech ? { background: hue(speech.family) } : undefined} aria-hidden="true" />
                <span className={speech ? `hand-${speech.family % 5}` : undefined}>{people?.name ?? `The ${ORDINAL[i]} people`}</span>
              </button>;
            })}
            <button type="button" role="tab" id="founding-tab-charter" aria-controls="founding-charter"
              aria-selected={charter} tabIndex={charter ? 0 : -1} onClick={() => setCharter(true)}>The charter</button>
          </div>
          <div className="founding-contents-acts">
            <button type="button" className="link" disabled={founders.length >= MOST_PEOPLES} onClick={add}>
              <Plus size={13} aria-hidden="true" /> Another people
            </button>
            {founders.length > 1 && !charter ? <button type="button" className="link" onClick={() => remove(selected)}>
              Leave this people out
            </button> : null}
          </div>
        </div>
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
            : three peoples over four thousand years, a conquest, and a faith born among the conquered.
          </p>
        </header>

        {charter && overview && built ? (
          <article className="founding-charter" id="founding-charter" role="tabpanel" aria-labelledby="founding-tab-charter" tabIndex={0}>
            <h2>The founding charter</h2>
            <section>
              <h3>Who lives where</h3>
              <ul className="charter-peoples">{overview.communities.map((people) => (
                <li key={people.id}><strong>{people.name}</strong> live in <i>{landName(people.region)}</i>, by{" "}
                  {LIVELIHOOD_PHRASE[people.livelihood]}; among them, {temperament(people.ethos, 2).join(" and ") || "an even temper"}.</li>
              ))}</ul>
            </section>
            <section>
              <h3>How their speech differs</h3>
              <div className="table-wrap charter-specimens">
                <table>
                  <thead><tr><th scope="col">Word for</th>{overview.communities.map((people) => (
                    <th scope="col" key={people.id}>{people.name}</th>
                  ))}</tr></thead>
                  <tbody>{overview.varieties[overview.communities[0].variety].specimen.map((word) => (
                    <tr key={word.concept}><th scope="row" title={word.gloss}>{word.concept}</th>
                      {overview.communities.map((people) => {
                        const specimen = overview.varieties[people.variety].specimen.find((w) => w.concept === word.concept);
                        return <td key={people.id}><span className="word" title={specimen ? `/${specimen.ipa}/` : undefined}>{specimen?.spelled ?? "—"}</span></td>;
                      })}
                    </tr>
                  ))}</tbody>
                </table>
              </div>
            </section>
            <section>
              <h3>Who may meet</h3>
              <p className="muted small">These are possible first journeys, not promises. Crossing water needs seafaring, which no founder yet knows.</p>
              {REACH_GROUPS.map((group) => {
                const pairs = built.preview.pairs.filter((pair) => group.id === "walking"
                  ? pair.reach === "walking" || pair.reach === "neighbours" : pair.reach === group.id);
                return <div className="founding-reach" key={group.id}><h4>{group.title}</h4>
                  {pairs.length ? <ul>{pairs.map((pair) => <li key={`${pair.a}:${pair.b}`}>
                    <strong>{overview.communities.find((p) => p.id === pair.a)?.name}</strong> and{" "}
                    <strong>{overview.communities.find((p) => p.id === pair.b)?.name}</strong>: {pairEvidence(pair)}.
                  </li>)}</ul> : <p className="muted small">None.</p>}
                </div>;
              })}
            </section>
            <section>
              <h3>Questions to follow</h3>
              <ul className="founding-questions">{questions.map((question) => <li key={question.title}>{question.title}</li>)}</ul>
              <label className="founding-keep"><input type="checkbox" checked={keepQuestions} onChange={(event) => setKeepQuestions(event.target.checked)} />
                Keep these questions in the notebook</label>
              <p className="muted small">After beginning, follow each question from the field notebook to its people, place, or word.</p>
            </section>
          </article>
        ) : current && c && v && map ? (
          <article className="account chosen" id={`founding-account-${current.key}`} role="tabpanel"
            aria-labelledby={`founding-tab-${current.key}`} tabIndex={0} style={{ "--tone": hue(v.family) } as CSSProperties}>
            <div className="account-number">The {ORDINAL[selected]} account</div>
            <h2 className="account-name"><span className={`hand-${v.family % 5}`}>{c.name}</span> <span className="meaning">“{c.meaning}”</span></h2>
            <section className="founding-stage">
              <h3 className="eyebrow">1 · Homeland</h3>
              <p className="account-text">They live in{" "}
                <select aria-label="Homeland" value={current.region ?? c.region} onChange={(event) => update(selected, { region: Number(event.target.value) })}>
                  {map.regions.filter((region) => region.terrain !== "sea").map((region) => (
                    <option key={region.id} value={region.id} title={lower(TERRAIN_NAME[region.terrain])}>
                      {landName(region.id) === "unnamed land" ? `land ${region.id + 1}` : landName(region.id)}
                    </option>
                  ))}
                </select>, {lower(TERRAIN_NAME[map.regions[c.region].terrain])}, {map.regions[c.region].coastal ? "coastal" : "inland"}.
              </p>
              <p className="muted small">Choose a land here or on the chart.</p>
              <ul className="founding-neighbours" aria-label="Possible first encounters">
                {built?.preview.pairs.filter((pair) => pair.a === c.id || pair.b === c.id).map((pair) => {
                  const other = overview?.communities.find((people) => people.id === (pair.a === c.id ? pair.b : pair.a));
                  return <li key={`${pair.a}:${pair.b}`}><strong>{other?.name}</strong>: {pairEvidence(pair)}.</li>;
                })}
              </ul>
              {founders.length === 1 ? <p className="muted small">No other founding people yet.</p> : null}
            </section>
            <section className="founding-stage">
              <h3 className="eyebrow">2 · Livelihood</h3>
              <p className="account-text">They live{" "}
                <Phrase
                  label="Way of life"
                  value={current.livelihood ?? ""}
                  choices={[
                    { key: "", text: `as their land suggests (${LIVELIHOOD_PHRASE[c.livelihood]})` },
                    ...(Object.keys(LIVELIHOOD_NAME) as Livelihood[]).map((livelihood) => ({ key: livelihood, text: `by ${LIVELIHOOD_PHRASE[livelihood]}` })),
                  ]}
                  onChange={(key) => update(selected, { livelihood: key === "" ? null : (key as Livelihood) })}
                />.
              </p>
            </section>
            <section className="founding-stage">
              <h3 className="eyebrow">3 · Temper</h3>
              <p className="account-text">Among them,{" "}
                <Phrase
                  label="Temper"
                  value={current.bent ? `${current.bent.axis} ${current.bent.toward}` : ""}
                  choices={[
                    { key: "", text: "as their land and life make them" },
                    ...ETHOS_AXES.flatMap((axis) => [1, -1].map((toward) => ({ key: `${axis} ${toward}`, text: ETHOS_POLES[axis][toward > 0 ? 1 : 0] }))),
                  ]}
                  onChange={(key) => {
                    const [axis, toward] = key.split(" ");
                    update(selected, { bent: key === "" ? null : { axis: axis as EthosAxis, toward: Number(toward) as 1 | -1 } });
                  }}
                />{current.bent === null ? ` (${temperament(c.ethos, 2).join(" and ") || "even-tempered"})` : ""}.
              </p>
            </section>
            <section className="founding-stage">
              <h3 className="eyebrow">4 · Speech</h3>
              <Specimen words={v.specimen} />
              <p className="account-text">Their speech is{" "}
                <Phrase
                  label="Sounds"
                  value={current.preset ?? ""}
                  choices={[
                    ...(current.preset === null ? [{ key: "", text: "their own, shaped by hand" }] : []),
                    ...catalog.presets.map((preset) => ({ key: preset.id, text: lower(preset.name), title: preset.description })),
                  ]}
                  onChange={(key) => update(selected, { preset: key, design: presetDesign(key, current.seed) })}
                />.
              </p>
              <div className="account-acts">
                <button type="button" className="link" onClick={() => {
                  const seed = randomSeed();
                  update(selected, { seed, design: current.preset === null ? current.design : presetDesign(current.preset, seed) });
                }}>Hear other words</button>
                <button type="button" className="link" onClick={() => setAdjusting(true)}>Adjust their sounds…</button>
              </div>
            </section>
            <section className="founding-stage">
              <h3 className="eyebrow">5 · Identity</h3>
              <p className="account-text">They name themselves{" "}
                <Phrase
                  label="Name"
                  value={encodeNaming(current.naming)}
                  choices={namingChoices(catalog)}
                  onChange={(key) => { const naming = decodeNaming(key); if (naming) update(selected, { naming }); }}
                />,
                <b> {c.name}</b>. They call their speech <i>{v.name}</i>.
              </p>
            </section>
          </article>
        ) : <p className="muted">Settling…</p>}
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
          <button type="button" className="link" onClick={onShelf}>
            Back to the shelf
          </button>
          <button
            type="button"
            className="primary begin"
            disabled={!engine.current || !currentBuild || !overview || overview.communities.length !== founders.length}
            onClick={() => {
              if (!engine.current || !overview || !currentBuild) return;
              try {
                if (keepQuestions) for (const question of questions) {
                  const note = makeNote(overview, question.subject, question.label, "question");
                  engine.current.saveNote({ ...note, title: question.title });
                }
                handedOver.current = true;
                onBegin(engine.current);
              } catch (failure) {
                setError(message(failure));
              }
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

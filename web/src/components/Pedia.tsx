import { useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import {
  ArrowLeft,
  AudioLines,
  Globe,
  Languages,
  Landmark,
  Hammer,
  Sparkles,
  MapPin,
  Play,
  ScrollText,
  Users,
  WholeWord,
  type LucideIcon,
} from "lucide-react";
import type { Annal, Catalog, Community, Craft, CraftView, Engine, Overview, ReligionView, StateView, TellingView, Variety, WordMap, WorldMap } from "../model";
import { YEARS } from "../model";
import { CONTACT_NAME, EVENT_KIND, FAITH_HOW, FALL_NAME, howCame, howNamed, hue, LIVELIHOOD_NAME, RISE_NAME, TERMS, TERRAIN_NAME, type Term } from "../lore";
import { bond } from "../words";
import type { DialogKind } from "./ActionDialog";
import { Told } from "./Told";
import { Dictionary } from "./Dictionary";
import { peoplesByRegion } from "./MapView";
import { Specimen } from "./Specimen";
import { FamilyTree } from "./FamilyTree";
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
  | { kind: "event"; annal: Annal }
  | { kind: "history" };

/// Most of a people's story the card lists, newest first.
const STORY_LENGTH = 12;
/// Most examples of a sound change the card lists.
const EXAMPLES = 6;
/// Sound laws a language card lists before "all of them".
const RECENT_LAWS = 8;

/// Share of core words below which two languages count as unrelated.
const KIN_FLOOR = 0.05;

interface Context {
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
}

/// Most cards the trail names before the one open.
const TRAIL_SHOWN = 3;

/// The encyclopedia: one card at a time about whatever is in focus, with
/// every name in it leading to that thing's own card. Each card opens the
/// same way: what kind of thing it is, its name, a box of facts, and its
/// specimen words where it has a language, then sections to read on. The
/// last few cards visited stay named above it, to step back to.
export function Pedia({
  trail,
  onReturn,
  ...context
}: Context & { trail: Focus[]; onReturn: (index: number) => void }) {
  const focus = trail.at(-1)!;
  const first = Math.max(0, trail.length - 1 - TRAIL_SHOWN);
  return (
    <aside className="pedia" aria-label="Encyclopedia">
      <nav className="pedia-nav">
        <button
          type="button"
          className="icon"
          disabled={trail.length < 2}
          onClick={() => onReturn(trail.length - 2)}
          title="Back"
        >
          <ArrowLeft size={16} />
        </button>
        <button type="button" className="icon" onClick={() => context.go({ kind: "world" })} title="The world">
          <Globe size={16} />
        </button>
        <button
          type="button"
          className="icon"
          onClick={() => context.go({ kind: "history" })}
          title="Everything that has happened"
        >
          <ScrollText size={16} />
        </button>
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
      </nav>
      <article className="card">
        <Card focus={focus} context={context} />
      </article>
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
      return <WordCard variety={focus.variety} concept={focus.concept} context={context} />;
    case "law":
      return <LawCard id={focus.id} context={context} />;
    case "land":
      return <LandCard region={focus.region} context={context} />;
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

/// What a land is like, in a few words: "coastal forest, an island".
function terrain(region: number, context: Context): string {
  const r = context.map.regions[region];
  return `${r.coastal ? "coastal " : ""}${TERRAIN_NAME[r.terrain].toLowerCase()}${r.island ? ", an island" : ""}`;
}

/// Several names, each a link, joined as a sentence would join them.
function Joined<T>({ items, link }: { items: T[]; link: (item: T) => ReactNode }) {
  return (
    <>
      {items.map((item, i) => (
        <span key={i}>
          {i === 0 ? "" : i === items.length - 1 ? " and " : ", "}
          {link(item)}
        </span>
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
      <h3>Peoples</h3>
      <table className="peoples">
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
      {states.length > 0 ? (
        <>
          <h3>States that stand</h3>
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
        </>
      ) : null}
      {overview.religions.length > 0 ? (
        <>
          <h3>Religions</h3>
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
        </>
      ) : null}
      <h3>Crafts</h3>
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
      {spoken.length > 1 ? (
        <>
          <h3>Words compared</h3>
          <p className="muted small">
            Words shaded alike come from one root: they are <Explained term="cognate">cognates</Explained>.
          </p>
          <WordsCompared spoken={spoken} context={context} />
        </>
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
    <>
      <h3>
        <Explained term="family">Families</Explained>
      </h3>
      {families.map((f) => (
        <FamilyTree
          key={f}
          overview={overview}
          family={f}
          chosen={-1}
          onOpen={(id) => context.go({ kind: "language", variety: id })}
        />
      ))}
    </>
  );
}

/// One line of the whole history: written, or struck out from a telling
/// set aside.
interface Line {
  annal: Annal;
  telling: TellingView | null;
  /// The first struck line of its telling, which carries the note.
  opens: boolean;
}

/// Everything that has happened, oldest first. Nothing written is erased:
/// what was undone or told otherwise stays where it was, struck through,
/// and can be told that way again. Sound changes are many, so only one
/// language's are shown, with its ancestors' before it parted from them.
function HistoryCard({ context }: { context: Context }) {
  const { overview } = context;
  const [variety, setVariety] = useState<number | null>(null);
  const lines = useMemo(() => {
    const lineage: [number, number][] = [];
    for (let v = variety, until = Infinity; v !== null; ) {
      lineage.push([v, until]);
      until = overview.varieties[v].forkedAt ?? 0;
      v = overview.varieties[v].parent;
    }
    const relevant = (a: Annal) =>
      a.kind !== "law" || lineage.some(([v, until]) => a.variety === v && a.generation <= until);
    const written: Line[] = overview.annals.filter(relevant).map((annal) => ({ annal, telling: null, opens: false }));
    const struck: Line[] = overview.tellings.flatMap((telling) =>
      telling.struck.filter(relevant).map((annal, i) => ({ annal, telling, opens: i === 0 })),
    );
    // Struck lines follow what was written in the same year.
    return [...written, ...struck].sort((a, b) => a.annal.generation - b.annal.generation);
  }, [overview, variety]);
  const spoken = overview.varieties.filter((v) => v.spoken);
  return (
    <>
      <CardHead icon={ScrollText} kind="History" title="Everything that has happened" />
      <label className="history-laws">
        <span>
          <Explained term="sound law">Sound changes</Explained> in
        </span>
        <select
          value={variety ?? ""}
          onChange={(e) => setVariety(e.target.value === "" ? null : Number(e.target.value))}
        >
          <option value="">no language</option>
          {spoken.map((v) => (
            <option key={v.id} value={v.id}>
              {v.name}
            </option>
          ))}
        </select>
      </label>
      {lines.length === 0 ? (
        <p className="muted">Nothing has happened yet.</p>
      ) : (
        <ol className="history">
          {lines.map(({ annal, telling, opens }, i) => (
            <li key={i} className={telling ? "struck" : undefined}>
              <Year generation={annal.generation} context={context} />
              {telling ? (
                <span>
                  {opens ? (
                    <span className="struck-note">
                      {telling.why === "undone" ? "Struck out" : "In another telling"}
                      {" · "}
                      <button type="button" className="link" onClick={() => context.onRestore(telling.index)}>
                        tell it this way
                      </button>
                    </span>
                  ) : null}
                  <del>
                    <Told text={annal.text} />
                  </del>
                </span>
              ) : (
                <span>
                  <button type="button" className="moment" onClick={() => context.go({ kind: "event", annal })}>
                    <Told text={annal.text} />
                  </button>
                  <AnnalLinks annal={annal} context={context} />
                </span>
              )}
            </li>
          ))}
        </ol>
      )}
    </>
  );
}

function PeopleCard({ c, context }: { c: Community; context: Context }) {
  const { overview } = context;
  const name = (id: number) => overview.communities[id];
  const v = overview.varieties[c.variety];
  const contacts = overview.contacts.filter((k) => k.a === c.id || k.b === c.id);
  const moves = overview.moves.filter((m) => m.community === c.id);
  const told = overview.annals.filter((a) => a.peoples.includes(c.id));
  const story = told.slice(-STORY_LENGTH).reverse();
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
      <h3>Dealings</h3>
      {contacts.length > 0 ? (
        <ul className="roster">
          {contacts.map((k, i) => (
            <li key={i}>
              They {bond(k.kind, k.intensity)} the <PeopleLink c={name(k.a === c.id ? k.b : k.a)} context={context} />{" "}
              <span className="muted">({CONTACT_NAME[k.kind].toLowerCase()})</span>
            </li>
          ))}
        </ul>
      ) : (
        <p className="muted">They deal with no one.</p>
      )}
      {moves.length > 0 ? (
        <>
          <h3>Wanderings</h3>
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
        </>
      ) : null}
      <h3>Their story</h3>
      <Story annals={story} context={context} />
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
        ["City", `${Math.round(state.city).toLocaleString()} souls`],
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
      <h3>Its story</h3>
      <Story annals={told.slice(-STORY_LENGTH).reverse()} context={context} />
    </>
  );
}

function ReligionCard({ religion, context }: { religion: ReligionView; context: Context }) {
  const { overview } = context;
  const told = overview.annals.filter((a) => a.religions.includes(religion.id) &&
    (a.kind === "faith" || a.kind === "conversion" || a.kind === "meaning"));
  return (
    <>
      <CardHead icon={Sparkles} kind="A religion" title={<i>{religion.name}</i>}
        tone={hue(religion.id)}
        sub={<>“{religion.meaning}” <span className="ipa">/{religion.ipa}/</span></>} />
      <Facts rows={[
        ["Founder", <><span className="word">{religion.founder.name}</span>, “{religion.founder.meaning}” <span className="ipa">/{religion.founder.ipa}/</span></>],
        ["Founded", <>year <Year generation={religion.founded} context={context} />, {FAITH_HOW[religion.how]}</>],
        ["Founder's people", <PeopleLink c={overview.communities[religion.people]} context={context} />],
        ["Founding land", <LandLink region={religion.land} context={context} />],
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
      <h3>Words of the faith</h3>
      <Renderings rows={religion.words} overview={overview} sacred={religion.sacred}
        onLanguage={(variety) => context.go({ kind: "language", variety })}
        onWord={(variety, concept) => context.go({ kind: "word", variety, concept })} />
      <p className="muted small">
        A <Explained term="learned word">learned word</Explained> may return from the sacred language beside its older descendant,
        making a <Explained term="doublet">doublet</Explained>. Older gods may become demons through <Explained term="pejoration">pejoration</Explained>.
      </p>
      <h3>Its story</h3>
      <Story annals={told.slice(-STORY_LENGTH).reverse()} context={context} />
    </>
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
      <h3>Words of the craft</h3>
      <Renderings rows={craft.words} overview={overview}
        onLanguage={(variety) => context.go({ kind: "language", variety })}
        onWord={(variety, concept) => context.go({ kind: "word", variety, concept })} />
      <p className="muted small">
        New meanings may use older words, much like <Explained term="meaning extension by livelihood">meanings shaped by a way of life</Explained>.
      </p>
      <h3>Its story</h3>
      <Story annals={told.slice(-STORY_LENGTH).reverse()} context={context} />
    </>
  );
}

function LanguageCard({ variety, context }: { variety: number; context: Context }) {
  const { engine, version, generation, overview } = context;
  const v = overview.varieties[variety];
  const respelled = overview.annals.some((a) => a.kind === "respelling" && a.variety === variety && a.generation === v.written);
  const [allLaws, setAllLaws] = useState(false);
  const speakers = overview.communities.filter((c) => c.ended === null && c.variety === variety);
  const daughters = overview.varieties.filter((d) => d.parent === variety);
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
      <Facts
        rows={[
          [
            "Spoken by",
            speakers.length > 0 ? <Joined items={speakers} link={(c) => <PeopleLink c={c} context={context} />} /> : "no one now",
          ],
          ["Standard of", v.standardOf === null ? null : <StateLink state={overview.states[v.standardOf]} context={context} />],
          ["Sacred to", v.sacredOf === null ? null : <ReligionLink religion={overview.religions[v.sacredOf]} context={context} />],
          ["Writing", v.written === null ? <Explained term="spelling vs pronunciation">unwritten</Explained> :
            <><Explained term="spelling vs pronunciation">{respelled ? "last respelled" : "written"}</Explained>{" "}
              {respelled ? "in" : "since"} year <Year generation={v.written} context={context} /></>],
          ["Name style", v.nameStyle === "double" ? <Explained term="dithematic name">two-part names</Explained> : "one-word names"],
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
          ["Sounds", `${v.consonants.length} consonants, ${v.vowels.length} vowels`],
          ["Words", `${v.words.toLocaleString()}, built with ${v.wordBuilding}`],
          ["Own words", <Explained term="own words">
            {v.ownWords.own} of {v.ownWords.meanings} meanings
            {v.ownWords.meanings > 0 ? ` (${Math.round(v.ownWords.own / v.ownWords.meanings * 100)}%)` : ""}
            {` · ${v.ownWords.loans} loans · ${v.ownWords.shared} shared`}
          </Explained>],
          ["Shortest", v.minimalWord],
        ]}
      />
      <LanguageSpecimen variety={variety} context={context} />
      <h3><Explained term="given name">Given names in fashion</Explained></h3>
      {v.names.length === 0 ? <p className="muted">None yet.</p> : (
        <ul className="roster given-names">
          {v.names.map((name, i) => (
            <li key={i}>
              <span className="word">{name.name}</span> <span className="ipa">/{name.ipa}/</span>{" "}
              “{name.meaning}”
              {name.from !== null ? <small className="muted"> from <LanguageLink variety={name.from} context={context} /> (sacred)</small> : null}
            </li>
          ))}
        </ul>
      )}
      {kin.length > 0 ? (
        <>
          <h3>Shares core words with</h3>
          <ul className="roster">
            {kin.map((k) => (
              <li key={k.other} className="meter-row">
                <LanguageLink variety={k.other} context={context} />
                <meter min={0} max={1} value={k.score} />
                <span className="muted">{Math.round(k.score * 100)}%</span>
              </li>
            ))}
          </ul>
        </>
      ) : null}
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
      <h3>
        <Explained term="sound law">Sound laws</Explained>
      </h3>
      {laws.length === 0 ? (
        <p className="muted">None yet.</p>
      ) : (
        <>
          <ol className="history">
            {(allLaws ? laws : laws.slice(0, RECENT_LAWS)).map((law, i) => (
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
          {laws.length > RECENT_LAWS ? (
            <button type="button" className="link more" onClick={() => setAllLaws(!allLaws)}>
              {allLaws ? "Only the latest" : `All ${laws.length}`}
            </button>
          ) : null}
        </>
      )}
      <details className="every-word">
        <summary>Every word ({v.words.toLocaleString()})</summary>
        <Dictionary
          engine={engine}
          version={version}
          generation={generation}
          variety={variety}
          concept={null}
          onConcept={(concept) => context.go({ kind: "word", variety, concept })}
        />
      </details>
    </>
  );
}

/// A language's family drawn as a chart of descent, if it has kin.
function FamilyHead({ variety, context }: { variety: Variety; context: Context }) {
  const { overview } = context;
  const size = overview.varieties.filter((v) => v.family === variety.family && v.born <= overview.generation).length;
  if (size < 2) return null;
  return (
    <>
      <h3>
        Its <Explained term="family">family</Explained>
      </h3>
      <FamilyTree
        overview={overview}
        family={variety.family}
        chosen={variety.id}
        onOpen={(id) => context.go({ kind: "language", variety: id })}
      />
    </>
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
      <h3>Who has it</h3>
      {had.length > 0 ? (
        <ul className="roster">
          {had.map(({ c, law }) => (
            <li key={c.id}>
              <PeopleLink c={c} context={context} /> <span className="muted">{howCame(law, c, overview)}</span>
            </li>
          ))}
        </ul>
      ) : (
        <p className="muted">No living people has it.</p>
      )}
      {without.length > 0 && had.length > 0 ? (
        <>
          <h3>Not undergone by</h3>
          <p>
            <Joined items={without} link={(c) => <PeopleLink c={c} context={context} />} />
          </p>
        </>
      ) : null}
      {told.length > 0 ? (
        <>
          <h3>As it happened</h3>
          <Story annals={told.slice(-EXAMPLES).reverse()} context={context} />
        </>
      ) : null}
    </>
  );
}

function LandCard({ region, context }: { region: number; context: Context }) {
  const { overview } = context;
  const names = overview.places.find((p) => p.region === region)?.names ?? [];
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
        <>
          <h3>Its names</h3>
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
        </>
      ) : null}
      {arrivals.length > 0 ? (
        <>
          <h3>Comings and goings</h3>
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
        </>
      ) : null}
    </>
  );
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

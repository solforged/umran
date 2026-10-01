import { useMemo, useState, type CSSProperties, type ReactNode } from "react";
import { ArrowLeft, Globe } from "lucide-react";
import type { Annal, Community, Engine, Overview, Variety, WordMap, WorldMap } from "../model";
import { YEARS } from "../model";
import { CONTACT_NAME, howCame, howNamed, hue, TERMS, TERRAIN_NAME, type Term } from "../lore";
import { bond } from "../words";
import type { DialogKind } from "./ActionDialog";
import { Told } from "./Chronicle";
import { Dictionary } from "./Dictionary";
import { peoplesByRegion } from "./MapView";
import { Specimen } from "./Specimen";
import { WordGloss } from "./WordGloss";

/// What the encyclopedia is open at.
export type Focus =
  | { kind: "world" }
  | { kind: "people"; id: number }
  | { kind: "language"; variety: number }
  | { kind: "word"; variety: number; concept: string }
  | { kind: "law"; id: string }
  | { kind: "land"; region: number }
  | { kind: "event"; annal: Annal };

const KIND_NAME: Record<Focus["kind"], string> = {
  world: "The world",
  people: "A people",
  language: "A language",
  word: "A word",
  law: "A sound change",
  land: "A land",
  event: "What happened",
};

/// Most of a people's story the card lists, newest first.
const STORY_LENGTH = 12;
/// Most examples of a sound change the card lists.
const EXAMPLES = 6;

/// Share of core words below which two languages count as unrelated.
const KIN_FLOOR = 0.05;

interface Context {
  engine: Engine;
  version: number;
  generation: number;
  overview: Overview;
  map: WorldMap;
  /// Every people's word for the meaning in view, if a word is in view.
  words: WordMap | null;
  go: (focus: Focus) => void;
  onScrub: (generation: number) => void;
  onDialog: (kind: DialogKind, community: number) => void;
}

/// The encyclopedia: one card at a time about whatever is in focus, with
/// every name in it leading to that thing's own card.
export function Pedia({
  focus,
  canBack,
  onBack,
  ...context
}: Context & { focus: Focus; canBack: boolean; onBack: () => void }) {
  return (
    <aside className="pedia" aria-label="Encyclopedia">
      <nav className="pedia-nav">
        <button type="button" className="icon" disabled={!canBack} onClick={onBack} title="Back">
          <ArrowLeft size={16} />
        </button>
        <button type="button" className="icon" onClick={() => context.go({ kind: "world" })} title="The world">
          <Globe size={16} />
        </button>
        <span className="eyebrow">{KIND_NAME[focus.kind]}</span>
      </nav>
      <article className="card">
        <Card focus={focus} context={context} />
      </article>
    </aside>
  );
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
  }
}

/// A linguist's term, explained in place when clicked.
function Explained({ term, children }: { term: Term; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" className="term" aria-expanded={open} onClick={() => setOpen(!open)}>
        {children}
      </button>
      {open ? <span className="term-note">{TERMS[term]}</span> : null}
    </>
  );
}

function PeopleLink({ c, context }: { c: Community; context: Context }) {
  const family = context.overview.varieties[c.variety].family;
  return (
    <button
      type="button"
      className={`link word hand-${family % 5}`}
      style={{ color: hue(family) }}
      onClick={() => context.go({ kind: "people", id: c.id })}
    >
      {c.name}
    </button>
  );
}

function LanguageLink({ variety, context }: { variety: number; context: Context }) {
  return (
    <button type="button" className="link word" onClick={() => context.go({ kind: "language", variety })}>
      {context.overview.varieties[variety].name}
    </button>
  );
}

function LandLink({ region, context }: { region: number; context: Context }) {
  const name = context.overview.places.find((p) => p.region === region)?.names.at(-1)?.spelled;
  return (
    <button type="button" className="link word" onClick={() => context.go({ kind: "land", region })}>
      {name ?? `a nameless ${TERRAIN_NAME[context.map.regions[region].terrain].toLowerCase()}`}
    </button>
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
          <button type="button" className="moment" onClick={() => context.go({ kind: "event", annal: a })}>
            <Told text={a.text} />
          </button>
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

function WorldCard({ context }: { context: Context }) {
  const { overview, map } = context;
  const peoples = [...overview.communities].sort((a, b) => b.size - a.size);
  const spoken = overview.varieties.filter((v) => v.spoken);
  const families = new Set(spoken.map((v) => v.family)).size;
  const land = map.regions.filter((r) => r.terrain !== "sea").length;
  const held = new Set(overview.communities.map((c) => c.region)).size;
  const silent = overview.varieties.filter((v) => !v.spoken);
  return (
    <>
      <h2>The world in year {overview.generation * YEARS}</h2>
      <p>
        {peoples.length} {peoples.length === 1 ? "people speaks" : "peoples speak"} {spoken.length}{" "}
        {spoken.length === 1 ? "language" : "languages"} of {families}{" "}
        <Explained term="family">{families === 1 ? "family" : "families"}</Explained>, on {held} of {land} lands.
      </p>
      <h3>Peoples</h3>
      <ul className="roster">
        {peoples.map((c) => (
          <li key={c.id}>
            <PeopleLink c={c} context={context} />
            <span className="muted">
              {" "}
              {Math.round(c.size).toLocaleString()} souls, speaking{" "}
            </span>
            <LanguageLink variety={c.variety} context={context} />
          </li>
        ))}
      </ul>
      {spoken.length > 1 ? (
        <>
          <h3>Words compared</h3>
          <p className="muted small">
            Words shaded alike come from one root: they are{" "}
            <Explained term="cognate">cognates</Explained>.
          </p>
          <WordsCompared spoken={spoken} context={context} />
        </>
      ) : null}
      {silent.length > 0 ? (
        <>
          <h3>Languages no longer spoken</h3>
          <ul className="roster">
            {silent.map((v) => (
              <li key={v.id}>
                <LanguageLink variety={v.id} context={context} />
              </li>
            ))}
          </ul>
        </>
      ) : null}
    </>
  );
}

function PeopleCard({ c, context }: { c: Community; context: Context }) {
  const { overview, map } = context;
  const region = map.regions[c.region];
  const name = (id: number) => overview.communities[id];
  const contacts = overview.contacts.filter((k) => k.a === c.id || k.b === c.id);
  const moves = overview.moves.filter((m) => m.community === c.id);
  const story = overview.annals
    .filter((a) => a.peoples.includes(c.id))
    .slice(-STORY_LENGTH)
    .reverse();
  const family = overview.varieties[c.variety].family;
  return (
    <>
      <h2 className={`hand-${family % 5}`} style={{ color: hue(family) }}>
        {c.name} <span className="ipa">/{c.ipa}/</span>
      </h2>
      <p>
        “{c.meaning}”{c.once ? `, once ${c.once}` : ""}. {Math.round(c.size).toLocaleString()} souls on{" "}
        {region.coastal ? "coastal " : ""}
        {TERRAIN_NAME[region.terrain].toLowerCase()} in <LandLink region={c.region} context={context} />, speaking{" "}
        <LanguageLink variety={c.variety} context={context} />.
      </p>
      <LanguageSpecimen variety={c.variety} context={context} />
      {c.exonyms.length > 0 ? (
        <p className="muted">
          <Explained term="exonym">Called by others</Explained>:{" "}
          {c.exonyms.map((e, i) => (
            <span key={e.by}>
              {i > 0 ? ", " : ""}
              <span className="word">{e.name}</span> by the <PeopleLink c={name(e.by)} context={context} />
            </span>
          ))}
        </p>
      ) : null}
      <h3>Dealings</h3>
      {contacts.length > 0 ? (
        <ul className="roster">
          {contacts.map((k, i) => (
            <li key={i}>
              They {bond(k.kind, k.intensity)} the{" "}
              <PeopleLink c={name(k.a === c.id ? k.b : k.a)} context={context} />{" "}
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
          <ul className="roster">
            {moves.map((m, i) => (
              <li key={i}>
                <Year generation={m.generation} context={context} />{" "}
                {m.kind === "split" ? "went out to" : "moved to"} <LandLink region={m.to} context={context} />
                {m.overseas ? " over the sea" : ""}
              </li>
            ))}
          </ul>
        </>
      ) : null}
      <h3>Their story</h3>
      <Story annals={story} context={context} />
      <h3>Shape their history</h3>
      <div className="card-actions">
        <button type="button" onClick={() => context.onDialog("split", c.id)}>
          Some go their own way
        </button>
        <button type="button" onClick={() => context.onDialog("connect", c.id)}>
          They meet another people
        </button>
        <button type="button" onClick={() => context.onDialog("shift", c.id)}>
          They take up another tongue
        </button>
      </div>
    </>
  );
}

function LanguageCard({ variety, context }: { variety: number; context: Context }) {
  const { engine, version, generation, overview } = context;
  const v = overview.varieties[variety];
  const speakers = overview.communities.filter((c) => c.variety === variety);
  const daughters = overview.varieties.filter((d) => d.parent === variety);
  const kin = overview.intelligibility
    .filter((p) => (p.a === variety || p.b === variety) && p.score >= KIN_FLOOR)
    .map((p) => ({ other: p.a === variety ? p.b : p.a, score: p.score }))
    .sort((a, b) => b.score - a.score);
  return (
    <>
      <h2 className={`hand-${v.family % 5}`}>{v.name}</h2>
      <p>
        {v.meaning ? `“${v.meaning}”. ` : ""}
        {speakers.length > 0 ? (
          <>
            Spoken by{" "}
            {speakers.map((c, i) => (
              <span key={c.id}>
                {i > 0 ? ", " : ""}
                <PeopleLink c={c} context={context} />
              </span>
            ))}
            .
          </>
        ) : (
          "No longer spoken."
        )}
      </p>
      <LanguageSpecimen variety={variety} context={context} />
      {v.parent !== null ? (
        <p className="muted">
          A daughter of <LanguageLink variety={v.parent} context={context} />, parted in year{" "}
          {(v.forkedAt ?? 0) * YEARS}.
        </p>
      ) : null}
      {daughters.length > 0 ? (
        <p className="muted">
          Mother of{" "}
          {daughters.map((d, i) => (
            <span key={d.id}>
              {i > 0 ? ", " : ""}
              <LanguageLink variety={d.id} context={context} />
            </span>
          ))}
          .
        </p>
      ) : null}
      {kin.length > 0 ? (
        <>
          <h3>Shares core words with</h3>
          <ul className="roster">
            {kin.map((k) => (
              <li key={k.other}>
                <LanguageLink variety={k.other} context={context} />{" "}
                <span className="muted">{Math.round(k.score * 100)}% of core words shared</span>
              </li>
            ))}
          </ul>
        </>
      ) : null}
      <h3>Sounds</h3>
      <p className="segments">
        {v.consonants.join(" ")}
        <br />
        {v.vowels.join(" ")}
      </p>
      <p className="muted">
        {v.profile}. Builds words with {v.wordBuilding}; never wears a word shorter than {v.minimalWord}.
      </p>
      <h3>
        <Explained term="sound law">Sound laws</Explained>
      </h3>
      {v.laws.length === 0 ? (
        <p className="muted">None yet.</p>
      ) : (
        <ol className="history">
          {[...v.laws].reverse().map((law, i) => (
            <li key={i}>
              <Year generation={law.generation} context={context} />
              <span>
                <button type="button" className="link" onClick={() => context.go({ kind: "law", id: law.id })}>
                  {law.label}
                </button>
                {law.from !== null ? (
                  <span className="muted">
                    {" "}
                    (a <Explained term="wave">wave</Explained> from{" "}
                    <LanguageLink variety={law.from} context={context} />)
                  </span>
                ) : null}
              </span>
            </li>
          ))}
        </ol>
      )}
      <h3>Words</h3>
      <Dictionary
        engine={engine}
        version={version}
        generation={generation}
        variety={variety}
        concept={null}
        onConcept={(concept) => context.go({ kind: "word", variety, concept })}
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
  if (!overview.varieties[variety]) return <p className="muted">This language has not yet arisen in this year.</p>;
  return (
    <>
      <p className="muted">
        In <LanguageLink variety={variety} context={context} />
      </p>
      <WordGloss
        engine={engine}
        version={version}
        generation={generation}
        variety={variety}
        concept={concept}
        onScrub={context.onScrub}
        onOpenVariety={(v) => context.go({ kind: "word", variety: v, concept })}
      />
      {groups.length > 1 ? (
        <>
          <h3>Across the map</h3>
          <p className="muted">
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
  const had = overview.communities.flatMap((c) => {
    const law = overview.varieties[c.variety].laws.find((l) => l.id === id);
    return law ? [{ c, law }] : [];
  });
  const label =
    had[0]?.law.label ?? overview.varieties.flatMap((v) => v.laws).find((l) => l.id === id)?.label ?? id;
  const without = overview.communities.filter((c) => !had.some((h) => h.c.id === c.id));
  const examples = overview.annals
    .filter((a) => a.laws.includes(id))
    .slice(-EXAMPLES)
    .reverse();
  return (
    <>
      <h2>{label}</h2>
      <p className="muted">
        A <Explained term="sound law">sound law</Explained>. On the map, orange land underwent it and grey land did
        not; red lines are <Explained term="isogloss">isoglosses</Explained>, where it stopped.
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
        <p className="muted">
          Not undergone by{" "}
          {without.map((c, i) => (
            <span key={c.id}>
              {i > 0 ? ", " : ""}
              <PeopleLink c={c} context={context} />
            </span>
          ))}
          .
        </p>
      ) : null}
      {examples.length > 0 ? (
        <>
          <h3>As it happened</h3>
          <Story annals={examples} context={context} />
        </>
      ) : null}
    </>
  );
}

function LandCard({ region, context }: { region: number; context: Context }) {
  const { overview, map } = context;
  const r = map.regions[region];
  const names = overview.places.find((p) => p.region === region)?.names ?? [];
  const dwellers = peoplesByRegion(overview).get(region) ?? [];
  const arrivals = overview.moves.filter((m) => m.to === region || m.from === region);
  const now = names.at(-1);
  return (
    <>
      <h2 className="word">
        {now?.spelled ?? "A land without a name"} {now ? <span className="ipa">/{now.ipa}/</span> : null}
      </h2>
      <p>
        {r.coastal ? "Coastal " : ""}
        {r.coastal ? TERRAIN_NAME[r.terrain].toLowerCase() : TERRAIN_NAME[r.terrain]}
        {r.island ? ", an island" : ""}.{" "}
        {dwellers.length > 0 ? (
          <>
            Home of{" "}
            {dwellers.map((c, i) => (
              <span key={c.id}>
                {i > 0 ? ", " : ""}
                <PeopleLink c={c} context={context} />
              </span>
            ))}
            .
          </>
        ) : names.length > 0 ? (
          "No one lives here now."
        ) : (
          "No one has lived here."
        )}
      </p>
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
          <ul className="roster">
            {arrivals.map((m, i) => (
              <li key={i}>
                <Year generation={m.generation} context={context} />{" "}
                <PeopleLink c={overview.communities[m.community]} context={context} />{" "}
                {m.to === region ? "came from " : "left for "}
                <LandLink region={m.to === region ? m.from : m.to} context={context} />
                {m.overseas ? " over the sea" : ""}
              </li>
            ))}
          </ul>
        </>
      ) : null}
    </>
  );
}

function EventCard({ annal, context }: { annal: Annal; context: Context }) {
  const { overview } = context;
  const peoples = annal.peoples.filter((id) => overview.communities[id]);
  return (
    <>
      <p className="eyebrow">Year {annal.generation * YEARS}</p>
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
      {annal.notes.length > 0 ? (
        <ul className="apparatus">
          {annal.notes.map((note) => (
            <li key={note}>
              <Told text={note} />
            </li>
          ))}
        </ul>
      ) : null}
      {annal.laws.length > 0 ? (
        <>
          <h3>
            <Explained term="sound law">The change</Explained>
          </h3>
          <ul className="roster">
            {annal.laws.map((id) => (
              <li key={id}>
                <button type="button" className="link" onClick={() => context.go({ kind: "law", id })}>
                  {overview.varieties.flatMap((v) => v.laws).find((l) => l.id === id)?.label ?? id}
                </button>
              </li>
            ))}
          </ul>
        </>
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
      {annal.lands.length > 0 ? (
        <>
          <h3>Where</h3>
          <ul className="roster">
            {[...new Set(annal.lands)].map((region) => (
              <li key={region}>
                <LandLink region={region} context={context} />
              </li>
            ))}
          </ul>
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

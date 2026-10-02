import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { Annal, Catalog, Community, ContactKind, Craft, EthosAxis, NotebookNote, Overview, ReadEngine, SettlementIntent, Subject, WorldMap } from "../model";
import { YEARS } from "../model";
import * as lore from "../lore";
import { CONTACT_NAME, LIVELIHOOD_NAME, TERRAIN_NAME, temperament, weatherDeparture } from "../lore";
import { findAnnal, individualAnnals } from "../history";
import { bookMarkdown, craftLines, download, fileName, givenNameLines, peopleLines, religionLines, renderingLines, stateLines } from "../takeout";
import { CHAPTER, ChapterSummary, type ChapterContext } from "./LanguageChapter";
import { Miniature } from "./MapView";
import { Specimen } from "./Specimen";
import { Renderings } from "./Renderings";
import { Told } from "./Told";
import "./book.css";

const CONTENTS = [
  ["frontispiece", "Frontispiece"],
  ["lands", "The lands"],
  ["peoples", "The peoples"],
  ["languages", "The languages"],
  ["institutions", "States, faiths, crafts, cities"],
  ["chronicle", "The chronicle"],
  ["notes", "Notes"],
  ["tellings", "Other tellings"],
  ["elsewhere", "Take it elsewhere"],
] as const;

type SavedAction =
  | { kind: "found" }
  | { kind: "settle"; choice: { community: number; intent: SettlementIntent; destination: number; share: number } }
  | { kind: "connect"; a: number; b: number; contact: ContactKind }
  | { kind: "shift"; community: number; toward: number }
  | { kind: "state" | "religion"; community: number }
  | { kind: "craft"; community: number; craft: Craft }
  | { kind: "temper"; community: number; axis: EthosAxis; amount: number }
  | { kind: "run"; generations: number };
interface SavedTelling { id: number; actions: SavedAction[] }

/// Sections whose full form runs to hundreds of rows per language. In the
/// book they open on request, so thirty languages do not render thirty
/// lexicons at once.
const FOLDED: Record<string, true> = { laws: true, lexicon: true };

function Folded({ summary, children }: { summary: ReactNode; children: () => ReactNode }) {
  const [open, setOpen] = useState(false);
  return <details className="book-folded" open={open} onToggle={(e) => setOpen(e.currentTarget.open)}>
    <summary>{summary}</summary>
    {open ? children() : null}
  </details>;
}

/** The recipe is the true authorship record; a facade decisions() will replace
 * this parser. Annal.before currently identifies authored settlements only.
 * Never infer authorship by matching an annal's prose or kind. */
function recipeDecisions(save: string, telling: number, throughAction: number, generation: number) {
  const saved = JSON.parse(save) as { tellings: SavedTelling[] };
  const decisions: { generation: number; index: number; action: SavedAction }[] = [];
  let at = 0;
  const actions = saved.tellings.find((t) => t.id === telling)?.actions ?? [];
  for (let index = 0; index < Math.min(actions.length, throughAction); index++) {
    const action = actions[index];
    if (action.kind === "run") at += action.generations;
    else if (at <= generation) decisions.push({ generation: at, index, action });
  }
  return { saved, decisions };
}

/** Labels describe the recorded constraint, not an inferred effect on speech. */
function decisionLabel(action: SavedAction, overview: Overview): string {
  const people = (id: number) => overview.communities[id]?.name ?? `people ${id + 1}`;
  switch (action.kind) {
    case "found": return "Found a people";
    case "settle": return `${action.choice.intent}: ${people(action.choice.community)} to land ${action.choice.destination + 1}, ${Math.round(action.choice.share * 100)}% of the people`;
    case "connect": return `${CONTACT_NAME[action.contact]} between ${people(action.a)} and ${people(action.b)}`;
    case "shift": return `${people(action.community)} take up the tongue of ${people(action.toward)}`;
    case "state": return `${people(action.community)} found a state`;
    case "religion": return `${people(action.community)} found a faith`;
    case "craft": return `Teach ${action.craft} to ${people(action.community)}`;
    case "temper": return `Turn the ${action.axis} leaning of ${people(action.community)} by ${action.amount}`;
    case "run": return `Run ${action.generations * YEARS} years`;
  }
}

function subjectId(subject: Subject, overview: Overview): string {
  switch (subject.kind) {
    case "world": return "book-frontispiece";
    case "history": return "book-chronicle";
    case "language": return `language-${subject.variety}`;
    case "word": return `word-${subject.variety}-${subject.concept}`;
    case "people": return `people-${subject.id}`;
    case "land": return `land-${subject.region}`;
    case "continent": return `continent-${subject.landmass}`;
    case "state": return `state-${subject.id}`;
    case "religion": return `religion-${subject.id}`;
    case "craft": return `craft-${subject.id}`;
    case "event": return `event-${subject.id}`;
    case "law": {
      const annal = individualAnnals(overview.annals).find((a) => a.laws.includes(subject.id));
      return annal ? `event-${annal.id}` : "book-languages";
    }
    case "river": case "zone": return "book-lands";
  }
}

function CausePhrase({ annal, overview }: { annal: Annal; overview: Overview }) {
  const cause = (annal as Annal & { cause?: { event: number | string; mechanism: string } }).cause;
  const eventId = cause?.event ?? annal.temper?.cause;
  if (eventId === undefined || eventId === null) return null;
  // Current cause ids are strings; numeric ids are supported by the lane contract.
  const trigger = findAnnal(overview.annals, String(eventId));
  if (!trigger) return null;
  const names = (lore as typeof lore & { MECHANISM_NAME?: Record<string, string> }).MECHANISM_NAME;
  return <span className="book-cause"> After <a href={`#event-${trigger.id}`} title={trigger.text}>{lore.EVENT_KIND[trigger.kind].name.toLowerCase()} in year {trigger.generation * YEARS}</a>{cause ? ` · ${names?.[cause.mechanism] ?? cause.mechanism}` : ""}.</span>;
}

function ChronicleEntry({ annal, overview, anchor = false }: { annal: Annal; overview: Overview; anchor?: boolean }) {
  return <li id={anchor ? `event-${annal.id}` : undefined} className="book-annal">
    <span className="gen">{annal.generation * YEARS}</span>
    <div><Told text={annal.text} /><CausePhrase annal={annal} overview={overview} />
      {annal.before ? <small className="book-decision">The author’s decision</small> : null}
      {annal.specimen.length ? <Specimen words={annal.specimen} changes={annal.laws.length > 0} /> : null}
      {annal.notes.length ? <ul className="apparatus">{annal.notes.map((note, i) => <li key={i}><Told text={note} /></li>)}</ul> : null}
    </div>
  </li>;
}

/** The people has its own chapter: land, livelihood, temper, fortunes, speech. */
export function PeopleChapter({ community: c, ctx }: { community: Community; ctx: ChapterContext }) {
  const { overview, map } = ctx;
  const annals = useMemo(() => ctx.engine.story(ctx.generation, { kind: "people", id: c.id }).annals.map((id) => findAnnal(overview.annals, id)).filter((a): a is Annal => a !== undefined), [ctx.engine, ctx.generation, ctx.version, c.id, overview]);
  const contacts = overview.contacts.filter((contact) => contact.a === c.id || contact.b === c.id);
  const kin = overview.communities.filter((other) => other.id !== c.id && (c.parents.includes(other.id) || other.parents.includes(c.id) || c.parents.some((parent) => other.parents.includes(parent))));
  const language = overview.varieties[c.variety];
  const landLink = (region: number) => ctx.link({ kind: "land", region }, overview.places.find((p) => p.region === region)?.names.at(-1)?.spelled ?? `Land ${region + 1}`);
  const personLink = (id: number) => ctx.link({ kind: "people", id }, overview.communities[id]?.name ?? "An unrecorded people");
  return <article className="book-subject people-chapter" id={`people-${c.id}`}>
    <h3>{c.name}</h3><p className="book-subtitle">“{c.meaning}” · <span className="ipa">/{c.ipa}/</span>{c.once ? ` · once ${c.once}` : ""}{c.ended !== null ? ` · ended in year ${c.ended * YEARS}` : ""}</p>
    <h4>Where they live</h4><p>{c.ended === null ? "Their heart is" : "Their last heart was"} {landLink(c.region)}, {map.regions[c.region].coastal ? "coastal " : ""}{TERRAIN_NAME[map.regions[c.region].terrain].toLowerCase()}{map.regions[c.region].island ? ", on an island" : ""}. {c.lands.length > 1 ? <>Their lands: {c.lands.map((region, i) => <span key={region}>{i ? ", " : ""}{landLink(region)}</span>)}.</> : null}</p>
    <h4>How they live</h4><p>{LIVELIHOOD_NAME[c.livelihood]}; {Math.round(c.size).toLocaleString()} people{c.ended !== null ? " when they ended" : " now"}.</p>
    <h4>What they are like</h4><p>{temperament(c.ethos, 6).join(", ") || "No marked leaning"}.</p>
    {annals.some((a) => a.temper !== null) ? <ol className="book-chronicle">{annals.filter((a) => a.temper !== null).map((a) => <ChronicleEntry key={a.id} annal={a} overview={overview} />)}</ol> : null}
    <h4>Their fortunes</h4>{annals.length ? <ol className="book-chronicle">{annals.map((a) => <ChronicleEntry key={a.id} annal={a} overview={overview} />)}</ol> : <p className="muted">No fortunes recorded.</p>}
    <h4>What they speak</h4><p>{ctx.link({ kind: "language", variety: language.id }, language.name)}{language.meaning ? `, “${language.meaning}”` : ""}.</p><Specimen words={language.specimen} onWord={(concept) => ctx.open({ kind: "word", variety: language.id, concept })} />
    <dl className="chapter-facts">
      <div><dt>Kin</dt><dd>{kin.length ? kin.map((other, i) => <span key={other.id}>{i ? ", " : ""}{personLink(other.id)}</span>) : "No recorded parents, daughters, or siblings"}.</dd></div>
      <div><dt>Dealings</dt><dd>{contacts.length ? contacts.map((contact, i) => <span key={i}>{i ? "; " : ""}{CONTACT_NAME[contact.kind]} with {personLink(contact.a === c.id ? contact.b : contact.a)}</span>) : "No current contact recorded"}.</dd></div>
      <div><dt>Faith</dt><dd>{c.faith === null ? "Their own gods" : ctx.link({ kind: "religion", id: c.faith }, overview.religions[c.faith]?.name)}.</dd></div>
      <div><dt>Crafts</dt><dd>{c.crafts.length ? c.crafts.map((craft, i) => <span key={craft}>{i ? ", " : ""}{ctx.link({ kind: "craft", id: craft }, overview.crafts.find((item) => item.id === craft)?.name ?? craft)}</span>) : "None recorded"}.</dd></div>
      <div><dt>Called by others</dt><dd>{c.exonyms.length ? c.exonyms.map((name, i) => <span key={i}>{i ? "; " : ""}<span className="word">{name.name}</span> by {personLink(name.by)}</span>) : "No other name recorded"}.</dd></div>
    </dl>
  </article>;
}

export function Book({ engine, catalog, version, generation, overview, map, title, notes, onBack }: {
  engine: ReadEngine; catalog: Catalog; version: number; generation: number;
  overview: Overview; map: WorldMap; title: string; notes: NotebookNote[]; onBack: () => void;
}) {
  const [copied, setCopied] = useState<string | null>(null);
  const [copyError, setCopyError] = useState<string | null>(null);
  const close = useRef<HTMLButtonElement>(null);
  useEffect(() => { close.current?.focus(); }, []);
  useEffect(() => {
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || document.querySelector("dialog[open]") || event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement || event.target instanceof HTMLSelectElement) return;
      onBack();
    };
    document.addEventListener("keydown", escape);
    return () => document.removeEventListener("keydown", escape);
  }, [onBack]);
  const ctx: ChapterContext = {
    engine, version, generation, overview, map,
    link: (subject, label) => <a href={`#${subjectId(subject, overview)}`} onClick={(event) => {
      if (subject.kind === "word") { event.preventDefault(); ctx.open(subject); }
    }}>{label}</a>,
    open: (subject) => {
      const target = document.getElementById(subjectId(subject, overview));
      target?.scrollIntoView({ block: "start" });
      if (subject.kind === "word") {
        const evidence = target?.querySelector("details");
        if (evidence) evidence.open = true;
      }
    },
    wordAnchor: (variety, concept) => `word-${variety}-${concept}`,
  };
  const annals = useMemo(() => individualAnnals(overview.annals).sort((a, b) => a.generation - b.generation), [overview]);
  const { saved, decisions } = useMemo(
    () => recipeDecisions(engine.save(), overview.telling, overview.point.action, generation),
    [engine, version, overview.telling, overview.point.action, generation],
  );
  const chronicle = [
    ...annals.map((annal) => ({ generation: annal.generation, annal, decision: null })),
    ...decisions.map((decision) => ({ generation: decision.generation, annal: null, decision })),
  ].sort((a, b) => a.generation - b.generation || Number(a.annal === null) - Number(b.annal === null));
  const peoples = [...overview.communities].sort((a, b) => a.coined - b.coined || a.id - b.id);
  const families = [...new Set(overview.varieties.map((v) => v.family))];
  const climate = useMemo(() => engine.climate(generation), [engine, generation, version]);
  const telling = overview.tellings.find((t) => t.id === overview.telling);
  const religions = religionLines(overview);
  const states = stateLines(overview);
  const crafts = craftLines(overview, catalog);
  const copy = async (id: string, text: string) => {
    try { await navigator.clipboard.writeText(text); setCopied(id); setCopyError(null); }
    catch { setCopyError("Copy was not available. Use As text on the last page instead."); }
  };
  const head = (index: number, copyLines?: string[] | (() => string[])) => (
    <header className="book-running-head">
      <span>{index + 1} · {CONTENTS[index][1]}</span>
      <a href="#book-contents">Contents</a>
      {copyLines ? <button type="button" className="book-copy" onClick={() => void copy(CONTENTS[index][0], (typeof copyLines === "function" ? copyLines() : copyLines).join("\n"))}>
        {copied === CONTENTS[index][0] ? "Copied" : "Copy"}
      </button> : null}
    </header>
  );
  return <section className="export-sheet book" role="dialog" aria-modal="false" aria-label={`The book of ${title}`}>
    <header className="sheet-head">
      <span className="sheet-of">The book</span><h2>{title}</h2>
      <button ref={close} type="button" className="icon-btn sheet-close" aria-label="Back to the chart" onClick={onBack}>×</button>
    </header>
    <div className="sheet-page book-page">
      <nav className="book-contents" id="book-contents" aria-label="Table of contents">
        <h3>Contents</h3>
        <ol>{CONTENTS.map(([id, name]) => <li key={id}><a href={`#book-${id}`}>{name}</a></li>)}</ol>
      </nav>
      <section className="book-chapter book-frontispiece" id="book-frontispiece" data-book-chapter="frontispiece">
        {head(0)}
        <p className="book-kicker">The book of a world</p><h1>{title}</h1>
        <figure>
          <Miniature map={map} peoples={overview.communities.filter((c) => c.ended === null).sort((a, b) => b.size - a.size).map((c) => ({
            name: c.name, family: overview.varieties[c.variety].family, region: c.region, lands: c.lands,
          }))} />
          <figcaption>The chart in year {generation * YEARS}</figcaption>
        </figure>
        <p>Seed {overview.seed} · engine revision {overview.revision}
          {overview.savedRevision !== null && overview.savedRevision !== overview.revision ? ` · saved under revision ${overview.savedRevision}` : ""}
        </p>
        {telling?.parent ? <p>Telling: {telling.name}</p> : null}
      </section>
      <section className="book-chapter" id="book-lands" data-book-chapter="lands">
        {head(1)}<h2>The lands</h2>
        {map.landmasses.map((mass) => {
          const continent = overview.continents.find((c) => c.landmass === mass.id);
          const places = overview.places.filter((p) => map.regions[p.region].landmass === mass.id);
          return <article className="book-subject" id={`continent-${mass.id}`} key={mass.id}>
            <h3>{continent?.name?.name ?? (mass.kind === "island" ? "An unnamed island" : "An unnamed continent")}</h3>
            {continent?.name ? <p>
              “{continent.name.meaning}” · <span className="ipa">/{continent.name.ipa}/</span>;
              entered on the chart in year {continent.name.since * YEARS}, from{" "}
              {ctx.link({ kind: "language", variety: continent.name.variety }, overview.varieties[continent.name.variety]?.name)}.
            </p> : null}
            <p>{mass.regions.length} lands; {Array.from(new Set(mass.regions.map((id) => TERRAIN_NAME[map.regions[id].terrain].toLowerCase()))).join(", ")}.</p>
            {places.map((place) => <section className="book-land" id={`land-${place.region}`} key={place.region}>
              <h4>{place.names.at(-1)?.spelled ?? `Land ${place.region + 1}`}</h4>
              <p>{map.regions[place.region].coastal ? "Coastal " : ""}{TERRAIN_NAME[map.regions[place.region].terrain].toLowerCase()}
                {map.regions[place.region].island ? ", on an island" : ""}.
              </p>
              <ol className="book-name-ledger">{place.names.map((n, i) => <li key={i}>
                <span className="gen">{n.since * YEARS}</span>
                <span><span className="word">{n.spelled}</span> <span className="ipa">/{n.ipa}/</span> “{n.meaning}” · {n.origin},
                  in {ctx.link({ kind: "language", variety: n.variety }, n.language)}
                  {n.by !== null ? <>; coined by {ctx.link({ kind: "people", id: n.by }, overview.communities[n.by]?.name)}</> : null}
                  {n.once ? `; once ${n.once}` : ""}.
                </span>
              </li>)}</ol>
              {place.exonyms.length ? <p>Called by others: {place.exonyms.map((n, i) => <span key={i}>
                {i ? "; " : ""}<span className="word">{n.spelled}</span> <span className="ipa">/{n.ipa}/</span>
                {" "}in {ctx.link({ kind: "language", variety: n.variety }, n.language)}, heard in year {n.heard * YEARS}
                {n.once ? `, once ${n.once}` : ""}
              </span>)}.</p> : null}
            </section>)}
          </article>;
        })}
        <h3>Climate and rivers</h3>
        {climate.zones.map((zone) => <p key={zone.id}>Weather zone {zone.id + 1}: {weatherDeparture(zone)}.</p>)}
        <ul className="roster">{map.rivers.map((river) => {
          const name = engine.river(generation, river.id).names.at(-1);
          return <li key={river.id}>{name?.spelled ?? `River ${river.id + 1}`} · {climate.rivers.find((flow) => flow.id === river.id)?.flowing ? "flowing" : "flow has failed"}.</li>;
        })}</ul>
      </section>
      <section className="book-chapter" id="book-peoples" data-book-chapter="peoples">
        {head(2, peopleLines(overview))}<h2>The peoples</h2>
        {peoples.map((c) => <PeopleChapter key={c.id} community={c} ctx={ctx} />)}
      </section>
      <section className="book-chapter" id="book-languages" data-book-chapter="languages">
        {head(3, () => overview.varieties.flatMap((v) => [
          v.name, ...givenNameLines(v, overview),
          ...engine.lexicon(generation, v.id).map((row) => `${row.spelled}${row.said === null ? "" : ` · said ${row.said}`} /${row.ipa}/ ${row.gloss}`),
        ]))}
        <h2>The languages</h2>
        {families.map((family) => <section className="book-family" key={family}>
          <h3>The family of {overview.varieties[family]?.name}</h3>
          {overview.varieties.filter((v) => v.family === family).sort((a, b) => a.born - b.born || a.id - b.id).map((v) => (
            <article className="book-subject language-chapter" id={`language-${v.id}`} key={v.id}>
              <h3>{v.name}</h3>
              <p className="book-subtitle">{v.meaning ? `“${v.meaning}” · ` : ""}{v.spoken ? "Spoken" : "Silent"} · first spoken in year {v.born * YEARS}</p>
              <Specimen words={v.specimen} onWord={(concept) => ctx.open({ kind: "word", variety: v.id, concept })} />
              {CHAPTER.map(({ id, title: sectionTitle, Section }) => (
                <section className="book-language-section" key={id} id={`language-${v.id}-${id}`}>
                  <h4>{sectionTitle}</h4>
                  {FOLDED[id]
                    ? <Folded summary={<ChapterSummary variety={v} section={id} ctx={ctx} />}>{() => <Section variety={v} ctx={ctx} />}</Folded>
                    : <Section variety={v} ctx={ctx} />}
                </section>
              ))}
            </article>
          ))}
        </section>)}
      </section>
      <section className="book-chapter" id="book-institutions" data-book-chapter="institutions">
        {head(4, [...states, ...religions, ...crafts, ...overview.religions.flatMap((faith) => renderingLines(faith.words, overview)), ...overview.crafts.flatMap((craft) => renderingLines(craft.words, overview))])}
        <h2>States, faiths, crafts, cities</h2><h3>States</h3>
        {overview.states.length ? overview.states.map((state, i) => (
          <article className="book-subject" id={`state-${state.id}`} key={state.id}>
            <h4>{state.name}</h4><p>{states[i]}.</p>
            <p>{state.standard !== null ? <>Court speech: {ctx.link({ kind: "language", variety: state.standard }, overview.varieties[state.standard]?.name)}.</> : "No court standard recorded."}
              {" "}{state.classical ? <>Classical form: {ctx.link({ kind: "language", variety: state.classical.variety }, overview.varieties[state.classical.variety]?.name)}, fixed in year {state.classical.fixed * YEARS}.</> : null}
            </p>
          </article>
        )) : <p className="muted">No states yet.</p>}
        <h3>Faiths</h3>
        {overview.religions.length ? overview.religions.map((faith) => (
          <article className="book-subject" id={`religion-${faith.id}`} key={faith.id}>
            <h4>{faith.name}</h4><p>{religions[faith.id]}.</p>
            <p>Sacred speech: {ctx.link({ kind: "language", variety: faith.sacred }, overview.varieties[faith.sacred]?.name)};
              {" "}{faith.translates ? "its words are translated" : "its words are borrowed"}{faith.scripture ? "; scripture brings writing" : ""}.
            </p>
            <Renderings rows={faith.words} overview={overview} sacred={faith.sacred} />
          </article>
        )) : <p className="muted">No faiths founded yet.</p>}
        <h3>Crafts</h3>
        {overview.crafts.map((craft, i) => (
          <article className="book-subject" id={`craft-${craft.id}`} key={craft.id}>
            <h4>{craft.name}</h4><p>{crafts[i]}.</p><Renderings rows={craft.words} overview={overview} />
          </article>
        ))}
        <h3>Cities</h3>
        {overview.cities.length ? overview.cities.map((city) => (
          <article className="book-subject" key={city.id}>
            <h4>{city.name.name}</h4>
            <p>“{city.name.meaning}”; a great city since year {city.since * YEARS},
              in {ctx.link({ kind: "state", id: city.state }, overview.states[city.state]?.name)}.
              {" "}Its speech: {city.makeup.map((share, i) => <span key={share.variety}>
                {i ? ", " : ""}{ctx.link({ kind: "language", variety: share.variety }, overview.varieties[share.variety]?.name)} {Math.round(share.share * 100)}%
              </span>)}.
            </p>
            {city.townsfolk !== null ? <p>Its townsfolk became {ctx.link({ kind: "people", id: city.townsfolk }, overview.communities[city.townsfolk]?.name)}.</p> : null}
          </article>
        )) : <p className="muted">No great cities yet.</p>}
      </section>
      <section className="book-chapter" id="book-chronicle" data-book-chapter="chronicle">
        {head(5)}<h2>The chronicle</h2>
        <p className="muted">Every recorded entry, in year order. Challenges are named only where the engine recorded a cause.</p>
        <ol className="book-chronicle">{chronicle.map(({ annal, decision }) => annal
          ? <ChronicleEntry key={annal.id} annal={annal} overview={overview} anchor />
          : <li className="book-decision-row" key={`decision-${decision!.index}`}>
              <span className="gen">{decision!.generation * YEARS}</span>
              <div>{decisionLabel(decision!.action, overview)}<small className="book-decision">The author’s decision</small></div>
            </li>
        )}</ol>
      </section>
      <section className="book-chapter" id="book-notes" data-book-chapter="notes">
        {head(6)}<h2>Notes</h2>
        {notes.length ? notes.map((note) => (
          <article className="book-subject book-note" key={note.id}>
            <h3>{note.title}</h3>
            <p className="muted">Year {note.generation * YEARS}{note.archived ? " · archived" : ""}
              {note.target ? ` · ${overview.tellings.find((t) => t.id === note.target!.reading.telling)?.name ?? "An unavailable telling"}` : ""}
            </p>
            {note.target ? <p>Subject: {note.target.reading.telling === overview.telling ? ctx.link(note.target.subject, note.label) : note.label}.</p> : null}
            <p>{note.body}</p>
          </article>
        )) : <p className="muted">No notes written yet.</p>}
      </section>
      <section className="book-chapter" id="book-tellings" data-book-chapter="tellings">
        {head(7)}<h2>Other tellings</h2>
        {overview.tellings.filter((t) => t.id !== overview.telling).length ? <ul className="roster">
          {overview.tellings.filter((t) => t.id !== overview.telling).map((t) => {
            const diverging = t.parent
              ? saved.tellings.find((savedTelling) => savedTelling.id === t.id)?.actions.slice(t.parent.point.action).find((action) => action.kind !== "run")
              : undefined;
            return <li key={t.id}>
              <h3>{t.name}</h3>
              <p>{t.parent ? `Diverged in year ${(t.from ?? 0) * YEARS}, at decision ${t.parent.point.action}` : "The first telling"}; latest year {t.latest * YEARS}.</p>
              {diverging ? <p>The diverging decision: {decisionLabel(diverging, overview)}.</p> : null}
            </li>;
          })}
        </ul> : <p className="muted">This is the only telling.</p>}
      </section>
      <section className="book-chapter" id="book-elsewhere" data-book-chapter="elsewhere">
        {head(8)}<h2>Take it elsewhere</h2>
        <ul className="takeout">
          <li><button type="button" onClick={() => download(fileName(title, "json"), engine.save(), "application/json")}>As a save file</button>
            {" "}<span className="muted">The world, every telling, and all the author’s notes; open it from the chart room.</span>
          </li>
          <li><button type="button" onClick={() => download(fileName(title, "md"), bookMarkdown(title, overview, overview.varieties.map((v) => [v.name, engine.lexicon(generation, v.id)]), catalog, notes), "text/markdown")}>As text</button>
            {" "}<span className="muted">The engine’s peoples, institutions, chronicle, lexicons, and notes as Markdown.</span>
          </li>
        </ul>
        {copyError ? <p role="status">{copyError}</p> : null}
        <p className="book-colophon">{title} · year {generation * YEARS} · seed {overview.seed} · engine revision {catalog.revision}</p>
      </section>
    </div>
  </section>;
}

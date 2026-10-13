/// <reference lib="es2024.promise" />
import { memo, use, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { createRoot } from "react-dom/client";
import type { Annal, Catalog, ClimateView, Community, DecisionView, NotebookNote, Overview, Read, ReadEngine, Subject, TellingView, Variety, WorldMap } from "../model";
import { YEARS } from "../model";
import * as lore from "../lore";
import { CONTACT_NAME, countWord, faithTeaching, LIVELIHOOD_NAME, seasonalPhrase, TENET_NOUN, TERRAIN_NAME, temperament, weatherDeparture } from "../lore";
import { findAnnal, individualAnnals } from "../history";
import { eraBirths, eras, isQuiet, listed, quietLine, quietRuns, speechLine, type Era } from "../eras";
import { download, fileName } from "../takeout";
import { EditionContext, editionReading, makeEdition } from "../edition";
import { CHAPTER, ChapterSummary, type ChapterContext } from "./LanguageChapter";
import { Miniature } from "./MapView";
import { Specimen } from "./Specimen";
import { Renderings } from "./Renderings";
import { Told } from "./Told";
import { format } from "../route";
import { Pending, useAnswer } from "./Pending";
import { readPlace, savePlace, type ReadingPlace } from "../shelf";
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


/// Sections whose full form runs to hundreds of rows per language. In the
/// book they open on request beneath a one-line summary, so thirty
/// languages do not render thirty lexicons and paradigm sets at once.
const FOLDED: Record<string, true> = { laws: true, grammar: true, lexicon: true, names: true };

function Folded({ summary, children }: { summary: ReactNode; children: () => ReactNode }) {
  const [open, setOpen] = useState(false);
  const { complete } = use(EditionContext);
  return <details className="book-folded" open={complete || open} onToggle={(e) => setOpen(e.currentTarget.open)}>
    <summary>{summary}</summary>
    {complete || open ? children() : null}
  </details>;
}

function commonestSeasons(climate: ClimateView, zone: number | null, lands?: readonly number[]): string | null {
  const counts = new Map<string, number>();
  let commonest: string | null = null;
  let most = 0;
  for (const region of climate.regions) {
    if ((zone !== null && region.zone !== zone) || (lands && !lands.includes(region.id))) continue;
    const phrase = seasonalPhrase(region.seasons);
    const count = (counts.get(phrase) ?? 0) + 1;
    counts.set(phrase, count);
    if (count > most) {
      commonest = phrase;
      most = count;
    }
  }
  return commonest;
}


function subjectId(subject: Subject): string {
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
    case "lake": return `lake-${subject.id}`;
    case "law": return subject.variety !== undefined && subject.generation !== undefined && subject.index !== undefined
      ? `law-${subject.variety}-${subject.generation}-${subject.id}-${subject.index}` : "book-chronicle";
    case "river": return `river-${subject.id}`;
    case "zone": return `zone-${subject.id}`;
  }
}

function subjectChapter(subject: Subject): string {
  switch (subject.kind) {
    case "world": return "frontispiece";
    case "people": return "peoples";
    case "language": case "word": return "languages";
    case "state": case "religion": case "craft": return "institutions";
    case "law": return subject.variety === undefined ? "chronicle" : "languages";
    case "history": case "event": return "chronicle";
    default: return "lands";
  }
}

function CausePhrase({ annal, overview }: { annal: Annal; overview: Overview }) {
  const cause = lore.causePhrase(annal, overview);
  return cause ? <span className="book-cause"> <button type="button" className="link" data-book-target={`event-${cause.trigger.id}`} title={cause.trigger.text}>{cause.text[0].toUpperCase() + cause.text.slice(1)}</button>.</span> : null;
}

/// `marked` names the author's decision on the entry itself. `dated`
/// prints its year, which an entry under its year's heading leaves out.
function ChronicleEntry({ annal, overview, link, anchor = false, marked = true, dated = true }: { annal: Annal; overview: Overview; link: ChapterContext["link"]; anchor?: boolean; marked?: boolean; dated?: boolean }) {
  return <li id={anchor ? `event-${annal.id}` : undefined} className={dated ? "book-annal" : "book-annal undated"}>
    {dated ? <span className="gen">{annal.generation * YEARS}</span> : null}
    <div><Told text={annal.text} names={annal.names} link={link} /><CausePhrase annal={annal} overview={overview} />
      {marked && annal.decision !== undefined ? <small className="book-decision">The author’s decision</small> : null}
      {annal.specimen.length ? <Specimen words={annal.specimen} changes={annal.laws.length > 0} /> : null}
      {annal.notes.length ? <ul className="apparatus">{annal.notes.map((note, i) => <li key={i}><Told text={note} /></li>)}</ul> : null}
    </div>
  </li>;
}

/// A subject's fortunes in the world's eras: the headlines in full, the
/// quieter entries of each era folded into one line that opens to them.
function Fortunes({ annals, eras, overview, link, people }: { annals: Annal[]; eras: Era[]; overview: Overview; link: ChapterContext["link"]; people: number }) {
  return <>{eras.map((era) => {
    const within = annals.filter((a) => a.generation >= era.start && a.generation <= era.end);
    if (!within.length) return null;
    const headlines = within.filter((a) => !isQuiet(a, overview.annals));
    const quiet = within.filter((a) => isQuiet(a, overview.annals));
    return <section className="book-fortune-era" id={`people-${people}-era-${era.start}`} key={era.start}>
      <h5><span className="book-era-span">{era.start * YEARS}–{era.end * YEARS}</span> {era.heading}</h5>
      {headlines.length ? <ol className="book-chronicle">{headlines.map((a) => <ChronicleEntry key={a.id} annal={a} overview={overview} link={link} />)}</ol> : null}
      {quiet.length ? <Folded summary={<span className="book-quiet-line">{quietLine({ generation: era.start, headlines, quiet }, overview, lore.CHRONICLE.alsoEra)}</span>}>
        {() => <ol className="book-quiet">{quiet.map((a) => <ChronicleEntry key={a.id} annal={a} overview={overview} link={link} />)}</ol>}
      </Folded> : null}
    </section>;
  })}</>;
}

/** The people has its own chapter: land, livelihood, temper, fortunes,
 * speech. A people that has ended keeps a paragraph; the rest folds. */
function PeopleChapter({ community: c, eras, ctx }: { community: Community; eras: Era[]; ctx: ChapterContext }) {
  const { overview, map } = ctx;
  const story = use(ctx.engine.story(ctx.generation, { kind: "people", id: c.id }));
  const annals = useMemo(() => story.annals.map((id) => findAnnal(overview.annals, id)).filter((a): a is Annal => a !== undefined), [story, overview]);
  const contacts = overview.contacts.filter((contact) => contact.a === c.id || contact.b === c.id);
  const kin = overview.communities.filter((other) => other.id !== c.id && (c.parents.includes(other.id) || other.parents.includes(c.id) || c.parents.some((parent) => other.parents.includes(parent))));
  const language = overview.varieties[c.variety];
  const phrases = lore.peoplePhrases(c.ended);
  const landLink = (region: number) => ctx.link({ kind: "land", region }, lore.landCardLabel(overview, map, region));
  const personLink = (id: number) => ctx.link({ kind: "people", id }, lore.peopleLabel(overview, id));
  // One name others share is said once, with every people that says it.
  const exonyms = new Map<string, number[]>();
  for (const name of c.exonyms) exonyms.set(name.name, [...(exonyms.get(name.name) ?? []), name.by]);
  const temper = temperament(c.ethos, 6).join(", ");
  const body = () => <>
    <h4>{phrases.land}</h4><p>{phrases.homeland} {landLink(c.region)}, {map.regions[c.region].coastal ? "coastal " : ""}{TERRAIN_NAME[map.regions[c.region].terrain].toLowerCase()}{map.regions[c.region].island ? ", on an island" : ""}. {c.ended === null ? "They hold" : "They last held"} {countWord(c.lands.length)} {c.lands.length === 1 ? "land" : "lands"}.</p>
    <div id={`people-${c.id}-holdings`} className="book-apparatus"><Folded summary="The full land roster">{() => <ul className="roster">{c.lands.map((region) => <li key={region}>{landLink(region)}</li>)}</ul>}</Folded></div>
    <h4>{phrases.livelihood}</h4><p>{LIVELIHOOD_NAME[c.livelihood]}; {Math.round(c.size).toLocaleString()} people{c.ended !== null ? " when they ended" : " now"}.</p>
    <h4>What they are like</h4><p>{temper ? `${temper[0].toUpperCase()}${temper.slice(1)}` : "No marked leaning"}.</p>
    {annals.some((a) => a.temper !== null) ? <ol className="book-chronicle">{annals.filter((a) => a.temper !== null).map((a) => <ChronicleEntry key={a.id} annal={a} overview={overview} link={ctx.link} />)}</ol> : null}
    <h4>Their fortunes</h4>{annals.length ? <Fortunes annals={annals} eras={eras} overview={overview} link={ctx.link} people={c.id} /> : <p className="muted">No fortunes recorded.</p>}
    <h4>{phrases.speech}</h4><p>{phrases.lastSpeech}{ctx.link({ kind: "language", variety: language.id }, language.name)}{language.meaning ? `, “${language.meaning}”` : ""}.</p><Specimen words={language.specimen} onWord={(concept) => ctx.open({ kind: "word", variety: language.id, concept })} />
    <dl className="chapter-facts">
      <div><dt>Kin</dt><dd>{kin.length ? kin.map((other, i) => <span key={other.id}>{i ? ", " : ""}{personLink(other.id)}</span>) : "No recorded parents, daughters, or siblings"}.</dd></div>
      <div><dt>Dealings</dt><dd>{contacts.length ? contacts.map((contact, i) => <span key={i}>{i ? "; " : ""}{CONTACT_NAME[contact.kind]} with {personLink(contact.a === c.id ? contact.b : contact.a)}</span>) : "No current contact recorded"}.</dd></div>
      <div><dt>Faith</dt><dd>{c.faith === null ? "Their own gods" : ctx.link({ kind: "religion", id: c.faith }, overview.religions[c.faith]?.name)}.</dd></div>
      <div><dt>Crafts</dt><dd>{c.crafts.length ? c.crafts.map((craft, i) => <span key={craft}>{i ? ", " : ""}{ctx.link({ kind: "craft", id: craft }, overview.crafts.find((item) => item.id === craft)?.name ?? craft)}</span>) : "None recorded"}.</dd></div>
      <div><dt>Called by others</dt><dd>{exonyms.size ? [...exonyms].map(([name, by], i) => <span key={name}>{i ? "; " : ""}<span className="word">{name}</span> by {by.map((id, j) => <span key={id}>{j ? ", " : ""}{personLink(id)}</span>)}</span>) : "No other name recorded"}.</dd></div>
    </dl>
  </>;
  return <article className="book-subject people-chapter" id={`people-${c.id}`}>
    <h3>{lore.peopleLabel(overview, c.id)}</h3><p className="book-subtitle">“{c.meaning}” · <span className="ipa">/{c.ipa}/</span>{c.once ? ` · once ${c.once}` : ""}{c.ended !== null ? ` · ended in year ${c.ended * YEARS}` : ""}</p>
    {c.ended === null ? body() : <>
      <p>{phrases.homeland} {landLink(c.region)}. They last held {countWord(c.lands.length)} {c.lands.length === 1 ? "land" : "lands"} and were {LIVELIHOOD_NAME[c.livelihood].toLowerCase()}. {phrases.lastSpeech}{ctx.link({ kind: "language", variety: language.id }, language.name)}.
        {" "}{c.endedInto !== null ? <>They merged into the {personLink(c.endedInto)}.</> : "They died out."}</p>
      <Folded summary={`Their chapter: ${annals.length.toLocaleString()} recorded ${annals.length === 1 ? "entry" : "entries"}, kin, and dealings`}>{body}</Folded>
    </>}
  </article>;
}

function foldKey(fold: HTMLDetailsElement): string {
  const anchor = fold.parentElement?.closest<HTMLElement>("[id]");
  const path: number[] = [];
  for (let node: Element | null = fold; node && node !== anchor; node = node.parentElement) {
    path.unshift(Array.from(node.parentElement?.children ?? []).indexOf(node));
  }
  return `${anchor?.id ?? "book"}:${path.join(".")}`;
}

function TitlePageEditor({ title, author, onSave }: { title: string; author: string | null; onSave: (title: string, author: string) => Promise<string | null> }) {
  const [editing, setEditing] = useState(false);
  const [draftTitle, setDraftTitle] = useState(title);
  const [draftAuthor, setDraftAuthor] = useState(author ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const saved = useRef({ title, author: author ?? "" });
  const save = useRef(onSave);
  save.current = onSave;
  const queue = useRef<Promise<string | null>>(Promise.resolve(null));
  const saveDraft = () => {
    const draft = { title: draftTitle, author: draftAuthor };
    const job = queue.current.then(async () => {
      if (draft.title === saved.current.title && draft.author === saved.current.author) return null;
      setSaving(true); setError(null);
      try {
        const failure = await save.current(draft.title, draft.author);
        setError(failure);
        if (failure === null) saved.current = draft;
        return failure;
      } finally { setSaving(false); }
    });
    queue.current = job;
    return job;
  };
  const autosave = useRef(saveDraft);
  autosave.current = saveDraft;
  useEffect(() => {
    if (!editing) return;
    const timer = window.setTimeout(() => void autosave.current(), 450);
    return () => window.clearTimeout(timer);
  }, [editing, draftTitle, draftAuthor]);
  return <div className="book-title-editor" data-edition-omit>
    {editing ? <form onBlur={(event) => {
      if (!event.currentTarget.contains(event.relatedTarget as Node | null)) void saveDraft();
    }} onSubmit={async (event) => {
      event.preventDefault();
      if (await saveDraft() === null) setEditing(false);
    }}>
      <label>Title<input autoFocus value={draftTitle} onChange={(event) => setDraftTitle(event.target.value)} /></label>
      <label>Author<input value={draftAuthor} onChange={(event) => setDraftAuthor(event.target.value)} /></label>
      <div className="book-actions"><button type="submit" disabled={saving}>Done</button></div>
      <p className="muted" role="status">{error ?? (saving ? "Saving…" : draftTitle === saved.current.title && draftAuthor === saved.current.author ? "Saved in this browser." : "Waiting to save…")}</p>
    </form> : <button type="button" className="link" onClick={() => {
      saved.current = { title, author: author ?? "" };
      setDraftTitle(title); setDraftAuthor(author ?? ""); setError(null); setEditing(true);
    }}>Edit the title page</button>}
  </div>;
}

function TellingChapter({ telling, decisionsFor, onRead, onCompare, complete }: {
  telling: TellingView; decisionsFor: (telling: number) => Read<DecisionView[]>;
  onRead: (telling: number) => void; onCompare: (telling: number) => void; complete: boolean;
}) {
  const decisions = use(decisionsFor(telling.id));
  const diverging = telling.parent ? decisions.find((decision) => decision.index === telling.parent!.point.action) : undefined;
  return <article className="book-subject" id={`telling-${telling.id}`}>
    <h3>{telling.name}</h3>
    <p>{telling.parent ? `Diverged in year ${(telling.from ?? 0) * YEARS}` : "The first telling"}; latest year {telling.latest * YEARS}.</p>
    {diverging ? <p>The diverging decision: <Told text={diverging.text} /></p> : telling.parent ? <p className="muted">No divergence decision is recorded at this point.</p> : null}
    {!complete ? <div className="book-actions" data-edition-omit>
      <button type="button" onClick={() => onRead(telling.id)}>Read this telling</button>
      <button type="button" onClick={() => onCompare(telling.id)}>Compare with this one</button>
    </div> : null}
  </article>;
}
export function Book(props: {
  worldId: string; chapter: string | null; onChapter: (chapter: string | null) => void;
  engine: ReadEngine; catalog: Catalog; version: number; generation: number;
  overview: Overview; map: WorldMap; title: string; author: string | null;
  decisionsFor: (telling: number) => Read<DecisionView[]>; notes: NotebookNote[]; onBack: () => void;
  onReadNote: (note: NotebookNote) => Promise<string | null>;
  onEditTitlePage: (title: string, author: string) => Promise<string | null>;
  onReadTelling: (telling: number) => void; onCompare: (telling: number) => void;
  /// Edition-only admission; completed language chapters stay mounted.
  editionLanguages?: number;
}) {
  const { worldId, chapter, onChapter, engine, catalog, version, generation, overview, map, title, author, decisionsFor, notes, onReadNote, onBack, onEditTitlePage, onReadTelling, onCompare } = props;
  const { complete } = use(EditionContext);
  const [noteError, setNoteError] = useState<string | null>(null);
  const [contentsOpen, setContentsOpen] = useState(false);
  const [activeChapter, setActiveChapter] = useState(chapter ?? "frontispiece");
  const [activeAnchor, setActiveAnchor] = useState<string | null>(null);
  const root = useRef<HTMLElement>(null);
  const page = useRef<HTMLDivElement>(null);
  const close = useRef<HTMLButtonElement>(null);
  const chapterChanged = useRef(onChapter);
  chapterChanged.current = onChapter;
  const navigating = useRef(false);
  const openFolds = useRef(new Set<string>());
  const bookmark = useRef<ReadingPlace["book"]>(undefined);
  const lastChapter = useRef(chapter);
  const destination = useRef<ReadingPlace["book"]>(undefined);
  const waitingForTarget = useRef<MutationObserver | null>(null);
  useEffect(() => {
    if (complete || !page.current) return;
    const readingPage = page.current;
    const saved = readPlace(worldId)?.book;
    const kept = saved && CONTENTS.some(([id]) => id === saved.chapter)
      && (saved.anchor === null || typeof saved.anchor === "string")
      && typeof saved.scroll === "number" && Number.isFinite(saved.scroll) && saved.scroll >= 0
      && Array.isArray(saved.open) && saved.open.every((key) => typeof key === "string") ? saved : undefined;
    const requestedChapter = chapter ?? kept?.chapter ?? "frontispiece";
    const restoring = kept?.chapter === requestedChapter ? kept : { chapter: requestedChapter, anchor: null, scroll: 0, open: kept?.open ?? [] };
    bookmark.current = restoring;
    destination.current = restoring;
    openFolds.current = new Set(restoring.open);
    navigating.current = false;
    let frame = 0;
    const offset = (element: HTMLElement) => element.getBoundingClientRect().top - readingPage.getBoundingClientRect().top + readingPage.scrollTop;
    const remember = () => {
      if (!readingPage.getClientRects().length) return;
      const requested = destination.current;
      if (requested?.anchor && !readingPage.querySelector(`#${CSS.escape(requested.anchor)}`)) return;
      const chapters = Array.from(readingPage.querySelectorAll<HTMLElement>("[data-book-chapter]"));
      const current = chapters.filter((element) => offset(element) <= readingPage.scrollTop + 1).at(-1) ?? chapters[0];
      if (!current) return;
      const name = current.dataset.bookChapter!;
      const subjects = Array.from(current.querySelectorAll<HTMLElement>(".book-subject[id], .book-language-section[id], .book-era[id], .book-era-year[id], .book-land[id]"));
      const anchor = subjects.filter((element) => element.getClientRects().length && offset(element) <= readingPage.scrollTop + 1).at(-1) ?? current;
      const next = { chapter: name, anchor: anchor === current ? null : anchor.id, scroll: Math.max(0, readingPage.scrollTop - offset(anchor)), open: [...openFolds.current] };
      bookmark.current = next;
      setActiveChapter(name); setActiveAnchor(next.anchor);
      if (navigating.current) {
        const place = readPlace(worldId) ?? { telling: overview.telling, point: overview.point, viewing: overview.generation, focus: { kind: "world" as const } };
        savePlace(worldId, { ...place, book: next });
      }
      if (lastChapter.current !== name) {
        lastChapter.current = name;
        chapterChanged.current(name);
      }
    };
    const restore = () => {
      for (const fold of readingPage.querySelectorAll<HTMLDetailsElement>("details")) {
        const key = foldKey(fold);
        fold.dataset.bookFold = key;
        if (openFolds.current.has(key) && !fold.open) fold.open = true;
      }
      const requested = destination.current;
      if (requested) {
        const target = readingPage.querySelector<HTMLElement>(`#${CSS.escape(requested.anchor ?? `book-${requested.chapter}`)}`);
        if (!target) return;
        readingPage.scrollTop = offset(target) + requested.scroll;
      }
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(remember);
    };
    const scrolling = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(remember);
    };
    const interacting = () => {
      navigating.current = true;
      destination.current = undefined;
      waitingForTarget.current?.disconnect();
      waitingForTarget.current = null;
    };
    const toggled = (event: Event) => {
      if (!(event.target instanceof HTMLDetailsElement)) return;
      const fold = event.target;
      const key = fold.dataset.bookFold ?? foldKey(fold);
      fold.dataset.bookFold = key;
      if (fold.open) openFolds.current.add(key); else openFolds.current.delete(key);
      if (navigating.current) remember();
    };
    const watcher = new MutationObserver(restore);
    watcher.observe(readingPage, { childList: true, subtree: true });
    readingPage.addEventListener("scroll", scrolling, { passive: true });
    readingPage.addEventListener("toggle", toggled, true);
    for (const event of ["wheel", "touchstart", "pointerdown", "keydown"]) root.current?.addEventListener(event, interacting, { passive: true });
    window.addEventListener("pagehide", remember);
    restore();
    const readingRoot = root.current;
    return () => {
      watcher.disconnect(); cancelAnimationFrame(frame);
      waitingForTarget.current?.disconnect();
      readingPage.removeEventListener("scroll", scrolling); readingPage.removeEventListener("toggle", toggled, true);
      for (const event of ["wheel", "touchstart", "pointerdown", "keydown"]) readingRoot?.removeEventListener(event, interacting);
      window.removeEventListener("pagehide", remember);
      const place = readPlace(worldId);
      if (place && bookmark.current) savePlace(worldId, { ...place, book: bookmark.current });
    };
    // Restore only when opening a book or changing its telling, not as its rail follows the page.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [complete, worldId, overview.telling]);
  useEffect(() => {
    if (complete || chapter === lastChapter.current) return;
    lastChapter.current = chapter;
    destination.current = { chapter: chapter ?? "frontispiece", anchor: null, scroll: 0, open: [...openFolds.current] };
    const target = page.current?.querySelector<HTMLElement>(`#book-${CSS.escape(chapter ?? "frontispiece")}`);
    if (target) { navigating.current = true; target.scrollIntoView({ block: "start", behavior: "instant" }); }
  }, [chapter, complete]);
  useEffect(() => { if (!complete) close.current?.focus(); }, [complete]);
  useEffect(() => {
    if (complete) return;
    const current = root.current?.querySelector<HTMLElement>(contentsOpen ? "#book-contents a[aria-current=page]" : "#book-contents a[aria-current=location]");
    if (contentsOpen) current?.focus();
    current?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [complete, contentsOpen, activeChapter, activeAnchor]);
  const scrollTo = useCallback((target: HTMLElement) => {
    waitingForTarget.current?.disconnect();
    waitingForTarget.current = null;
    const targetChapter = target.closest<HTMLElement>("[data-book-chapter]")?.dataset.bookChapter ?? "frontispiece";
    destination.current = { chapter: targetChapter, anchor: target.id === `book-${targetChapter}` ? null : target.id, scroll: 0, open: [...openFolds.current] };
    for (let parent = target.parentElement; parent && parent !== page.current; parent = parent.parentElement) {
      if (parent instanceof HTMLDetailsElement) parent.open = true;
    }
    requestAnimationFrame(() => {
      target.scrollIntoView({ block: "start", behavior: "instant" });
      const next = target.closest<HTMLElement>("[data-book-chapter]")?.dataset.bookChapter ?? null;
      lastChapter.current = next;
      setActiveChapter(next ?? "frontispiece");
      setContentsOpen(false);
      chapterChanged.current(next);
    });
  }, []);
  const seek = useCallback((id: string, targetChapter: string, unfold?: () => void, arrive?: (target: HTMLElement) => void) => {
    waitingForTarget.current?.disconnect();
    destination.current = { chapter: targetChapter, anchor: id === `book-${targetChapter}` ? null : id, scroll: 0, open: [...openFolds.current] };
    lastChapter.current = targetChapter;
    setActiveChapter(targetChapter);
    setContentsOpen(false);
    chapterChanged.current(targetChapter);
    const locate = () => {
      const target = page.current?.querySelector<HTMLElement>(`#${CSS.escape(id)}`);
      if (!target) { unfold?.(); return; }
      arrive?.(target);
      scrollTo(target);
    };
    if (page.current) {
      waitingForTarget.current = new MutationObserver(locate);
      waitingForTarget.current.observe(page.current, { childList: true, subtree: true });
    }
    locate();
  }, [scrollTo]);
  useEffect(() => {
    if (complete) return;
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || Array.from(document.querySelectorAll("dialog[open]")).some((dialog) => !dialog.closest("[hidden]"))) return;
      event.stopPropagation();
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement || event.target instanceof HTMLSelectElement) return;
      if (contentsOpen) setContentsOpen(false); else onBack();
    };
    document.addEventListener("keydown", escape, true);
    return () => document.removeEventListener("keydown", escape, true);
  }, [complete, contentsOpen, onBack]);
  const ctx = useMemo(() => {
    const context: ChapterContext = {
      engine, version, generation, overview, map,
      link: (subject, label) => <a data-book-target={subjectId(subject)} href={format({ kind: "world", id: worldId, page: "book", chapter: subjectChapter(subject) })} onClick={(event) => {
        if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
        event.preventDefault(); context.open(subject);
      }}>{label}</a>,
      open: (subject) => {
        navigating.current = true;
        const id = subjectId(subject);
        const unfold = () => {
          const language = subject.kind === "word" || subject.kind === "law" ? subject.variety : undefined;
          const section = subject.kind === "word" ? "lexicon" : "laws";
          const apparatus = subject.kind === "land" ? `gazetteer-${map.regions[subject.region].landmass}`
            : subject.kind === "river" || subject.kind === "lake" || subject.kind === "zone" ? "book-water-apparatus" : null;
          const fold = language !== undefined
            ? page.current?.querySelector<HTMLDetailsElement>(`#language-${language} > details.book-folded:not([open])`)
              ?? page.current?.querySelector<HTMLDetailsElement>(`#language-${language}-${section} > details.book-folded:not([open])`)
            : apparatus ? page.current?.querySelector<HTMLDetailsElement>(`#${apparatus} > details.book-folded:not([open])`) : null;
          if (fold) fold.open = true;
        };
        seek(id, subjectChapter(subject), unfold, subject.kind === "word" ? (target) => {
          const evidence = target.querySelector("details");
          if (evidence) evidence.open = true;
        } : undefined);
      },
      wordAnchor: (variety, concept) => `word-${variety}-${concept}`,
    };
    return context;
  }, [engine, version, generation, overview, map, worldId, seek]);
  const recorded = useAnswer(engine.decisions());
  const decisions = useMemo(() => (recorded ?? []).filter((decision) => decision.generation <= generation), [recorded, generation]);
  const chronicle = useMemo(() => {
    const sections = eras(overview.annals, generation, overview);
    // A decision remains visible even when it produced no recorded entry.
    for (const decision of decisions) {
      const era = sections.find((section) => decision.generation >= section.start && decision.generation <= section.end);
      if (era && !era.years.some((year) => year.generation === decision.generation)) {
        era.years.push({ generation: decision.generation, headlines: [], quiet: [] });
        era.years.sort((a, b) => a.generation - b.generation);
      }
    }
    return sections;
  }, [overview, generation, decisions]);
  const annalOrder = new Map<string, number>();
  overview.annals.forEach((annal, index) => {
    annalOrder.set(annal.id, index);
    for (const member of annal.members) annalOrder.set(member.id, index);
  });
  const decisionsByYear = new Map<number, DecisionView[]>();
  for (const decision of decisions) {
    const year = decisionsByYear.get(decision.generation) ?? [];
    year.push(decision);
    decisionsByYear.set(decision.generation, year);
  }
  const peoples = [...overview.communities].sort((a, b) => a.coined - b.coined || a.id - b.id);
  const families = [...new Set(overview.varieties.map((v) => v.family))];
  const editionLanguages = props.editionLanguages ?? overview.varieties.length;
  const admittedLanguages = new Set(overview.varieties.slice(0, editionLanguages).map((variety) => variety.id));
  const telling = overview.tellings.find((t) => t.id === overview.telling);
  const subjects: { id: string; label: string }[] = activeChapter === "peoples"
    ? peoples.map((people) => ({ id: `people-${people.id}`, label: lore.peopleLabel(overview, people.id) }))
    : activeChapter === "languages"
      ? families.flatMap((family) => overview.varieties.filter((variety) => variety.family === family).sort((a, b) => a.born - b.born || a.id - b.id).map((variety) => ({ id: `language-${variety.id}`, label: variety.name })))
      : activeChapter === "lands"
        ? map.landmasses.map((mass) => ({ id: `continent-${mass.id}`, label: lore.continentLabel(overview, map, mass.id) }))
        : activeChapter === "institutions"
          ? [...overview.states.map((state) => ({ id: `state-${state.id}`, label: lore.stateLabel(overview, state.id) })),
            ...overview.religions.map((faith) => ({ id: `religion-${faith.id}`, label: faith.name })),
            ...overview.crafts.map((craft) => ({ id: `craft-${craft.id}`, label: craft.name })),
            ...overview.cities.map((city) => ({ id: `city-${city.id}`, label: lore.cityLabel(overview, map, city) }))]
          : activeChapter === "chronicle" ? chronicle.map((era) => ({ id: `era-${era.start}`, label: `${era.start * YEARS}–${era.end * YEARS} · ${era.heading}` }))
            : activeChapter === "notes" ? [...notes].sort((a, b) => a.generation - b.generation).map((note) => ({ id: `note-${note.id}`, label: note.title }))
              : activeChapter === "tellings" ? overview.tellings.filter((other) => other.id !== overview.telling).map((other) => ({ id: `telling-${other.id}`, label: other.name })) : [];
  const head = (index: number) => (
    <header className="book-running-head">
      <span>{index + 1} · {CONTENTS[index][1]}</span>
      {!complete ? <button type="button" className="link" data-edition-omit onClick={() => {
        setContentsOpen(true);
        root.current?.querySelector<HTMLElement>("#book-contents a[aria-current=page]")?.focus();
      }}>Contents</button> : null}
    </header>
  );
  return <section ref={root} className="book" role="main" aria-label={`The book of ${title}`} data-contents-open={contentsOpen || undefined} onClick={(event) => {
    if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    const link = event.target instanceof Element ? event.target.closest<HTMLElement>("[data-book-target]") : null;
    const id = link?.dataset.bookTarget;
    if (!id) return;
    event.preventDefault(); navigating.current = true;
    const target = page.current?.querySelector<HTMLElement>(`#${CSS.escape(id)}`);
    const chapterName = target?.closest<HTMLElement>("[data-book-chapter]")?.dataset.bookChapter
      ?? CONTENTS.find(([name]) => id === `book-${name}`)?.[0]
      ?? (id.startsWith("people-") ? "peoples" : id.startsWith("language-") ? "languages" : activeChapter);
    seek(id, chapterName);
  }}>
    {!complete ? <header className="book-sheet-head" data-edition-omit>
      <span className="book-seal"><span className="book-of">The book</span><span>year {generation * YEARS} · {telling?.name ?? "This telling"}</span></span>
      <h2>{title}</h2>
      <button type="button" className="book-contents-toggle" aria-expanded={contentsOpen} aria-controls="book-contents" onClick={() => setContentsOpen((open) => !open)}>Contents</button>
      <button ref={close} type="button" className="book-close" onClick={onBack}>Back to the workshop</button>
    </header> : null}
    <div className="book-body">
      {!complete && contentsOpen ? <button type="button" className="book-contents-shade" aria-label="Close contents" data-edition-omit onClick={() => setContentsOpen(false)} /> : null}
      <nav className={`book-contents${complete ? " book-edition-contents" : ""}`} id="book-contents" aria-label="Table of contents">
        <div className="book-contents-head"><h3>Contents</h3>{!complete ? <button type="button" className="icon-btn book-contents-close" aria-label="Close contents" onClick={() => setContentsOpen(false)}>×</button> : null}</div>
        <ol>{CONTENTS.map(([id, name]) => <li key={id}>
          <a href={format({ kind: "world", id: worldId, page: "book", chapter: id })} data-book-target={`book-${id}`} aria-current={activeChapter === id ? "page" : undefined}>{name}</a>
          {!complete && activeChapter === id && subjects.length ? <ol className="book-subject-contents">{subjects.map((subject) => <li key={subject.id}>
            <a href={format({ kind: "world", id: worldId, page: "book", chapter: id })} data-book-target={subject.id} aria-current={activeAnchor === subject.id || activeAnchor?.startsWith(`${subject.id}-`) ? "location" : undefined}>{subject.label}</a>
          </li>)}</ol> : null}
        </li>)}</ol>
      </nav>
      <div ref={page} className="book-page" tabIndex={0} aria-label="Book chapters">
      <section className="book-chapter book-frontispiece" id="book-frontispiece" data-book-chapter="frontispiece">
        {head(0)}
        <p className="book-kicker">The book of a world</p><h1>{title}</h1>
        {author ? <p className="book-author">by {author}</p> : null}
        <figure>
          <Miniature map={map} peoples={overview.communities.filter((c) => c.ended === null).sort((a, b) => b.size - a.size).map((c) => ({
            name: c.name, family: overview.varieties[c.variety].family, region: c.region, lands: c.lands,
          }))} />
          <figcaption>The chart in year {generation * YEARS}</figcaption>
        </figure>
        <FrontispieceMetadata overview={overview} catalog={catalog} generation={generation} />
        {!complete && overview.communities.length ? <TitlePageEditor title={title} author={author} onSave={onEditTitlePage} /> : null}
        {generation > 0 ? <section className="book-came-of-it">
          <h2>What came of it</h2>
          <dl className="book-outcomes">
            <div><dt>Peoples</dt><dd>
              <a href={format({ kind: "world", id: worldId, page: "book", chapter: "peoples" })} data-book-target="book-peoples">{peoples.filter((people) => people.ended === null).length} living</a>
              {" · "}<a href={format({ kind: "world", id: worldId, page: "book", chapter: "peoples" })} data-book-target={peoples.find((people) => people.ended !== null) ? `people-${peoples.find((people) => people.ended !== null)!.id}` : "book-peoples"}>{peoples.filter((people) => people.ended !== null).length} ended</a>
            </dd></div>
            <div><dt>Languages</dt><dd>
              <a href={format({ kind: "world", id: worldId, page: "book", chapter: "languages" })} data-book-target="book-languages">{overview.varieties.length} born</a>
              {" · "}<a href={format({ kind: "world", id: worldId, page: "book", chapter: "languages" })} data-book-target={overview.varieties.find((variety) => variety.silentSince !== null) ? `language-${overview.varieties.find((variety) => variety.silentSince !== null)!.id}` : "book-languages"}>{overview.varieties.filter((variety) => variety.silentSince !== null).length} silenced</a>
            </dd></div>
          </dl>
          <h3>The founding peoples</h3>
          <ul className="book-founding-fates">{peoples.filter((people) => people.coined === 0).map((people) => <li key={people.id}>
            {ctx.link({ kind: "people", id: people.id }, lore.peopleLabel(overview, people.id))}: {people.ended === null
              ? <>still living, holding {countWord(people.lands.length)} {people.lands.length === 1 ? "land" : "lands"} and speaking {ctx.link({ kind: "language", variety: people.variety }, overview.varieties[people.variety].name)}</>
              : <>ended in year {people.ended * YEARS}{people.endedInto !== null ? <>, merging into {ctx.link({ kind: "people", id: people.endedInto }, lore.peopleLabel(overview, people.endedInto))}</> : "; died out"}</>}.
          </li>)}</ul>
          {chronicle.some((era) => era.opening) ? <><h3>Turning points</h3><ol className="book-turning-points">{chronicle.filter((era) => era.opening).map((era) => <li key={era.start}>
            <span className="gen">{era.start * YEARS}</span> {ctx.link({ kind: "event", id: era.opening!.id }, era.heading)}
            <p><Told text={era.opening!.text} names={era.opening!.names} link={ctx.link} /></p>
          </li>)}</ol></> : <p className="muted">No era-opening turning point is recorded in this reading.</p>}
        </section> : null}
      </section>
      <section className="book-chapter" id="book-lands" data-book-chapter="lands">
        {head(1)}<h2>The lands</h2>
        <Pending reading={engine} label="Reading these lands…"><LandsChapter peoples={peoples} ctx={ctx} /></Pending>
      </section>
      <section className="book-chapter" id="book-peoples" data-book-chapter="peoples">
        {head(2)}<h2>The peoples</h2>
        {peoples.map((c) => <Pending key={c.id} reading={engine} label={`Reading the ${lore.peopleLabel(overview, c.id)}…`}><PeopleChapter community={c} eras={chronicle} ctx={ctx} /></Pending>)}
      </section>
      <section className="book-chapter" id="book-languages" data-book-chapter="languages">
        {head(3)}
        <h2>The languages</h2>
        {families.map((family) => <section className="book-family" key={family}>
          <h3>The family of {overview.varieties[family]?.name}</h3>
          {overview.varieties.filter((v) => v.family === family).sort((a, b) => a.born - b.born || a.id - b.id).map((v) =>
            <LanguageBookChapter key={v.id} variety={v} ctx={ctx} admitted={!complete || admittedLanguages.has(v.id)} />)}
        </section>)}
      </section>
      <section className="book-chapter" id="book-institutions" data-book-chapter="institutions">
        {head(4)}
        <h2>States, faiths, crafts, cities</h2><h3>States</h3>
        {overview.states.length ? overview.states.map((state) => {
          const standard = overview.varieties.find((v) => v.standardOf === state.id);
          return (
          <article className="book-subject" id={`state-${state.id}`} key={state.id}>
            <h4>{lore.stateLabel(overview, state.id)}</h4><p>“{state.meaning}”; ruled by {ctx.link({ kind: "people", id: state.rulers }, lore.peopleLabel(overview, state.rulers))}{state.once ? `; once ${state.once}` : ""}.
              {" "}Founded in year {state.rose * YEARS} by <span className="word">{state.founder.name}</span>, “{state.founder.meaning}”, <span className="ipa">/{state.founder.ipa}/</span>{state.fell !== null ? `; fell in year ${state.fell * YEARS}` : ""}.</p>
            <p>{state.standard !== null && standard ? <>Court speech: {ctx.link({ kind: "language", variety: standard.id }, standard.name)}; standard since year {state.standard * YEARS}.</> : "No court standard recorded."}
              {" "}{state.classical ? <>Classical form: {ctx.link({ kind: "language", variety: state.classical.variety }, overview.varieties[state.classical.variety]?.name)}, fixed in year {state.classical.fixed * YEARS}.</> : null}
            </p>
          </article>
        ); }) : <p className="muted">No states yet.</p>}
        <h3>Faiths</h3>
        {overview.religions.length ? overview.religions.map((faith) => {
          const teaching = faithTeaching(faith);
          return (
          <article className="book-subject" id={`religion-${faith.id}`} key={faith.id}>
            <h4>{faith.name}</h4><p>“{faith.meaning}” · <span className="ipa">/{faith.ipa}/</span>; founded in year {faith.founded * YEARS}, {lore.FAITH_HOW[faith.how]},
              {" "}by <span className="word">{faith.founder.name}</span>, “{faith.founder.meaning}”, <span className="ipa">/{faith.founder.ipa}/</span>, among {ctx.link({ kind: "people", id: faith.people }, lore.peopleLabel(overview, faith.people))}
              {" "}in {ctx.link({ kind: "land", region: faith.land }, lore.landCardLabel(overview, map, faith.land))}.</p>
            <p>{faith.converts ? "Seeks converts" : "Keeps to its own"}; {faith.scripture ? "written scripture" : "unwritten teaching"}.
              {" "}Followers: {faith.followers.length ? faith.followers.map((people, i) => <span key={people}>{i ? ", " : ""}{ctx.link({ kind: "people", id: people }, lore.peopleLabel(overview, people))}</span>) : "none now"}.</p>
            <p>Sacred speech: {ctx.link({ kind: "language", variety: faith.sacred }, overview.varieties[faith.sacred]?.name)};
              {" "}{faith.translates ? "its words are translated" : "its words are borrowed"}{faith.scripture ? "; scripture brings writing" : ""}.
            </p>
            {teaching ? <p>Teaching: {teaching}.</p> : null}
            {faith.disputed !== null ? <p>Parted over: {TENET_NOUN[faith.disputed]}.</p> : null}
            <Renderings rows={faith.words} overview={overview} sacred={faith.sacred} />
          </article>
        ); }) : <p className="muted">No faiths founded yet.</p>}
        <h3>Crafts</h3>
        {overview.crafts.map((craft) => (
          <article className="book-subject" id={`craft-${craft.id}`} key={craft.id}>
            <h4>{craft.name}</h4><p>{catalog.crafts.find((choice) => choice.id === craft.id)?.description}.
              {" "}{craft.first === null ? "Not yet held" : `First held in year ${craft.first * YEARS}`}.
              {" "}Inventors: {craft.inventors.length ? craft.inventors.map((people, i) => <span key={people}>{i ? ", " : ""}{ctx.link({ kind: "people", id: people }, lore.peopleLabel(overview, people))}</span>) : "none yet"}.
              {" "}Holders: {craft.holders.length ? craft.holders.map((people, i) => <span key={people}>{i ? ", " : ""}{ctx.link({ kind: "people", id: people }, lore.peopleLabel(overview, people))}</span>) : "none now"}.</p>
            <Renderings rows={craft.words} overview={overview} />
          </article>
        ))}
        <h3>Cities</h3>
        {overview.cities.length ? overview.cities.map((city) => (
          <article className="book-subject" id={`city-${city.id}`} key={city.id}>
            <h4>{ctx.link({ kind: "land", region: city.region }, lore.cityLabel(overview, map, city))}</h4>
            <p>“{city.name.meaning}”; a great city since year {city.since * YEARS},
              in {ctx.link({ kind: "state", id: city.state }, lore.stateLabel(overview, city.state))}.
              {" "}Its speech: {city.makeup.map((share, i) => <span key={share.variety}>
                {i ? ", " : ""}{ctx.link({ kind: "language", variety: share.variety }, overview.varieties[share.variety]?.name)} {Math.round(share.share * 100)}%
              </span>)}.
            </p>
            {city.townsfolk !== null ? <p>Its townsfolk became {ctx.link({ kind: "people", id: city.townsfolk }, lore.peopleLabel(overview, city.townsfolk))}.</p> : null}
          </article>
        )) : <p className="muted">No great cities yet.</p>}
      </section>
      <section className="book-chapter" id="book-chronicle" data-book-chapter="chronicle">
        {head(5)}<h2>The chronicle</h2>
        <p className="muted">The headline entries of each era, with what happened to speech. Each year’s quieter entries fold into one line that opens to them. Challenges are named only where the engine recorded a cause.</p>
        {recorded === undefined ? <p className="pending" role="status">Reading the chronicle…</p> : chronicle.map((era) => {
          const speech = speechLine(era.years.flatMap((year) => [...year.headlines, ...year.quiet]), overview, eraBirths(era, overview));
          return <section className="book-era" id={`era-${era.start}`} key={era.start}>
          <h3><span className="book-era-span">{era.start * YEARS}–{era.end * YEARS}</span> {era.heading}</h3>
          {speech ? <p className="book-era-speech">{speech}</p> : null}
          {quietRuns(era.years, (year) => decisionsByYear.has(year.generation)).map((year) => {
            // A decision that produced a headline is marked on it; only
            // one whose entries are all quiet, or that left none, stands alone.
            const told = new Set(year.headlines.flatMap((annal) => [annal.id, ...annal.members.map((member) => member.id)]));
            const rows = [
              ...year.headlines.map((annal) => ({ order: annalOrder.get(annal.id) ?? Infinity, annal, decision: null })),
              ...(decisionsByYear.get(year.generation) ?? []).filter((decision) => !decision.annals.some((id) => told.has(id))).map((decision) => ({
                order: Math.min(...decision.annals.map((id) => annalOrder.get(id) ?? Infinity)), annal: null, decision,
              })),
            ].sort((a, b) => a.order - b.order || Number(a.annal !== null) - Number(b.annal !== null));
            const quietIds = year.quiet.flatMap((annal) => [annal.id, ...annal.members.map((member) => member.id)]);
            return <section className="book-era-year" id={`year-${year.generation}`} key={year.generation}>
              <h4>{year.through === undefined ? `Year ${year.generation * YEARS}` : `Years ${year.generation * YEARS}–${year.through * YEARS}`}</h4>
              {rows.length ? <ol className="book-chronicle">{rows.map(({ annal, decision }) => annal
                ? <ChronicleEntry key={annal.id} annal={annal} overview={overview} link={ctx.link} anchor dated={false} />
                : <li className="book-decision-row book-annal undated" key={`decision-${decision!.index}`}>
                    <div><Told text={decision!.text} /><small className="book-decision">The author’s decision</small></div>
                  </li>
              )}</ol> : null}
              {year.quiet.length ? <Folded summary={<span className="book-quiet-line" data-annals={quietIds.join(" ")}>{quietIds.map((id) => <span id={`event-${id}`} key={id} />)}{quietLine(year, overview, year.through === undefined ? lore.CHRONICLE.also : lore.CHRONICLE.alsoEra)}</span>}>
                {() => <ol className="book-quiet">{individualAnnals(year.quiet).map((annal) => <ChronicleEntry key={annal.id} annal={annal} overview={overview} link={ctx.link} dated={year.through !== undefined} />)}</ol>}
              </Folded> : null}
            </section>;
          })}
        </section>;
        })}
      </section>
      <section className="book-chapter" id="book-notes" data-book-chapter="notes">
        {head(6)}<h2>Notes</h2>
        {notes.length ? [...notes].sort((a, b) => a.generation - b.generation).map((note) => {
          const target = note.target;
          const tellingName = target ? overview.tellings.find((t) => t.id === target.reading.telling)?.name ?? "An unavailable telling" : null;
          const local = target !== null && target.reading.telling === overview.telling
            && target.reading.point.action === overview.point.action && target.reading.point.offset === overview.point.offset;
          return <article className="book-subject book-note" id={`note-${note.id}`} key={note.id}>
            <h3>{note.title}</h3>
            <p className="muted">Year {note.generation * YEARS}{note.archived ? " · archived" : ""}
              {tellingName ? ` · ${tellingName}` : ""}
            </p>
            {target ? <p>Subject: {local ? ctx.link(target.subject, note.label)
              : <button type="button" className="link" onClick={async () => {
                const failure = await onReadNote(note);
                setNoteError(failure);
                if (failure === null) onBack();
              }}>{note.label} (from {tellingName}, year {note.generation * YEARS})</button>}.</p> : null}
            {note.body.split(/\n\s*\n/).filter((paragraph) => paragraph.trim()).map((paragraph, index) => <p key={index}>{paragraph}</p>)}
          </article>;
        }) : <p className="muted">No notes written yet.</p>}
        {noteError ? <p role="status">{noteError}</p> : null}
      </section>
      <section className="book-chapter" id="book-tellings" data-book-chapter="tellings">
        {head(7)}<h2>Other tellings</h2>
        {overview.tellings.some((other) => other.id !== overview.telling)
          ? overview.tellings.filter((other) => other.id !== overview.telling).map((other) => <Pending key={other.id} reading={engine} label={`Reading ${other.name}…`}><TellingChapter telling={other} decisionsFor={decisionsFor} onRead={onReadTelling} onCompare={onCompare} complete={complete} /></Pending>)
          : <p className="muted">This is the only telling.</p>}
      </section>
      <section className="book-chapter" id="book-elsewhere" data-book-chapter="elsewhere">
        {head(8)}<h2>Take it elsewhere</h2>
        <EditionActions reading={props} />
      </section>
    </div>
    </div>
  </section>;
}

function FrontispieceMetadata({ overview, catalog, generation }: { overview: Overview; catalog: Catalog; generation: number }) {
  const { complete } = use(EditionContext);
  const [made] = useState(() => new Date());
  const telling = overview.tellings.find((candidate) => candidate.id === overview.telling);
  return <div className="book-metadata">
    <p>Year read: {generation * YEARS}</p>
    <p>Seed {overview.seed} · engine revision {catalog.revision}
      {overview.savedRevision !== null && overview.savedRevision !== catalog.revision ? ` · saved under revision ${overview.savedRevision}` : ""}
    </p>
    {overview.tellings.length > 1 && telling ? <p>Telling: {telling.name}</p> : null}
    {complete ? <p>Edition made <time dateTime={made.toISOString()}>{made.toLocaleDateString(undefined, { year: "numeric", month: "long", day: "numeric" })}</time>.</p> : null}
  </div>;
}

/** The edition uses the same fixed-reading props as the Book on screen. */
function EditionActions({ reading }: { reading: Parameters<typeof Book>[0] }) {
  const { complete } = use(EditionContext);
  const [progress, setProgress] = useState<{ done: number; total: number; phase: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const job = useRef<AbortController | null>(null);
  useEffect(() => () => { job.current?.abort(); }, []);
  const { title, engine, generation, overview, catalog } = reading;
  const make = async (format: "print" | "html" | "markdown") => {
    if (job.current) return;
    const controller = new AbortController();
    job.current = controller;
    const { signal } = controller;
    const host = document.createElement("div");
    host.className = "edition-host";
    host.setAttribute("aria-hidden", "true");
    host.inert = true;
    document.body.append(host);
    const rendered = createRoot(host);
    const total = overview.varieties.length;
    setError(null);
    setProgress({ done: 0, total, phase: "Opening every chapter…" });
    try {
      const fixedReading = editionReading(engine);
      const mode = { complete: true };
      const render = (opened: number) => rendered.render(<EditionContext.Provider value={mode}>
        <Pending reading={fixedReading}><Book {...reading} engine={fixedReading} editionLanguages={Math.min(total, opened + 2)} /></Pending>
      </EditionContext.Provider>);
      render(0);
      const book = await completeBook(host, signal, total, (done) => {
        setProgress({ done, total, phase: "Opening every chapter…" });
        render(done);
      });
      if (format === "print") {
        setProgress({ done: total, total, phase: "Preparing the printed book…" });
        await cancellable(document.fonts.ready, signal);
        // Print links as text, without thousands of PDF URL annotations.
        book.querySelectorAll("a[href]").forEach((link) => link.removeAttribute("href"));
        host.dataset.editionPrint = "true";
        const { promise: printed, resolve, reject } = Promise.withResolvers<void>();
        const finished = () => { clear(); resolve(); };
        const cancelled = () => { clear(); reject(signal.reason); };
        const clear = () => {
          window.removeEventListener("afterprint", finished);
          signal.removeEventListener("abort", cancelled);
        };
        window.addEventListener("afterprint", finished, { once: true });
        signal.addEventListener("abort", cancelled, { once: true });
        try { window.print(); } catch (failure) { clear(); reject(failure); }
        await printed;
      } else {
        setProgress({ done: total, total, phase: format === "html" ? "Embedding the book’s fonts…" : "Writing the text…" });
        const edition = makeEdition(book);
        const text = format === "html" ? await cancellable(edition.html(), signal) : edition.markdown();
        if (signal.aborted) throw signal.reason;
        download(fileName(title, format === "html" ? "html" : "md"), text, format === "html" ? "text/html;charset=utf-8" : "text/markdown;charset=utf-8");
      }
    } catch (failure) {
      if (!signal.aborted) setError(failure instanceof Error ? failure.message : "The edition could not be made.");
    } finally {
      rendered.unmount();
      host.remove();
      if (job.current === controller) {
        job.current = null;
        setProgress(null);
      }
    }
  };
  return <>
    {!complete ? <ul className="takeout">
      <li><button type="button" disabled={progress !== null} onClick={() => void make("print")}>Print</button>{" "}<span className="muted">Every chapter opened, with the chart; only the book prints.</span></li>
      <li><button type="button" disabled={progress !== null} onClick={() => void make("html")}>As a web page</button>{" "}<span className="muted">One self-contained page, with its fonts, chart, and links between chapters.</span></li>
      <li><button type="button" disabled={progress !== null} onClick={() => void make("markdown")}>As text</button>{" "}<span className="muted">The same complete book as Markdown, with every recorded entry.</span></li>
      <li><button type="button" onClick={() => download(fileName(title, "json"), engine.save(), "application/json")}>As a save file</button>{" "}<span className="muted">The world, every telling, and all the author’s notes; open it from the chart room.</span></li>
    </ul> : null}
    {error ? <p role="alert">{error}</p> : null}
    {progress ? createPortal(<dialog className="edition-progress" aria-labelledby="edition-progress-title"
      ref={(dialog) => { if (dialog && !dialog.open) dialog.showModal(); }}
      onCancel={(event) => { event.preventDefault(); job.current?.abort(); }}>
      <div><h2 id="edition-progress-title">Making an edition</h2>
        <p role="status" aria-live="polite">{progress.phase}{progress.total ? ` ${progress.done} of ${progress.total} languages` : ""}</p>
        <progress value={progress.done} max={Math.max(1, progress.total)} aria-label="Languages opened" />
        <button type="button" autoFocus onClick={() => job.current?.abort()}>Cancel</button>
      </div>
    </dialog>, document.body) : null}
    <p className="book-colophon">{title} · year {generation * YEARS} · seed {overview.seed} · engine revision {catalog.revision}</p>
  </>;
}

function cancellable<T>(reading: PromiseLike<T>, signal: AbortSignal): Promise<T> {
  if (signal.aborted) return Promise.reject(signal.reason);
  const { promise, resolve, reject } = Promise.withResolvers<T>();
  const cancel = () => reject(signal.reason);
  signal.addEventListener("abort", cancel, { once: true });
  Promise.resolve(reading).then(resolve, reject).finally(() => signal.removeEventListener("abort", cancel));
  return promise;
}

/** Suspense fallbacks, including WordGloss’s nested Pending, must disappear. */
function completeBook(host: HTMLElement, signal: AbortSignal, total: number, onProgress: (done: number) => void): Promise<HTMLElement> {
  const { promise, resolve, reject } = Promise.withResolvers<HTMLElement>();
  let frame = 0;
  let done = -1;
  const languageReady = (language: HTMLElement) => language.querySelectorAll(".book-language-section").length === CHAPTER.length
    && !language.querySelector(".pending") && !language.closest('[hidden], [style*="display: none"]');
  const ready = () => host.querySelector<HTMLElement>(".book")
    && !host.querySelector(".pending, details:not([open])")
    && host.querySelectorAll(".book-chapter").length === CONTENTS.length
    && Array.from(host.querySelectorAll<HTMLElement>(".language-chapter")).filter(languageReady).length === total;
  const clear = () => {
    observer.disconnect();
    cancelAnimationFrame(frame);
    signal.removeEventListener("abort", cancel);
  };
  const cancel = () => { clear(); reject(signal.reason); };
  const inspect = () => {
    const failure = host.querySelector<HTMLElement>(".notice.error[role=alert]");
    if (failure) { clear(); reject(new Error(failure.textContent ?? "A chapter could not be read.")); return; }
    // Non-lazy details (grammar and word-history ledgers) also open in an edition.
    host.querySelectorAll<HTMLDetailsElement>("details:not([open])").forEach((fold) => { fold.open = true; });
    const opened = Array.from(host.querySelectorAll<HTMLElement>(".language-chapter")).filter(languageReady).length;
    if (opened !== done) { done = opened; onProgress(done); }
    if (frame || !ready()) return;
    // Allow nested disclosures and their Suspense boundaries to commit before
    // trusting the absence of a fallback. Read completion has no time limit.
    frame = requestAnimationFrame(() => {
      frame = requestAnimationFrame(() => {
        frame = 0;
        if (!ready()) return inspect();
        clear();
        resolve(host.querySelector<HTMLElement>(".book")!);
      });
    });
  };
  const observer = new MutationObserver(inspect);
  observer.observe(host, { childList: true, subtree: true, attributes: true, attributeFilter: ["open", "hidden", "style"] });
  signal.addEventListener("abort", cancel, { once: true });
  if (signal.aborted) cancel(); else inspect();
  return promise;
}

function LandsChapter({ peoples, ctx }: { peoples: Community[]; ctx: ChapterContext }) {
  const { overview, map, engine, generation } = ctx;
  const climateRead = engine.climate(generation);
  const riversRead = engine.rivers(generation);
  const lakesRead = engine.lakes(generation);
  const climate = use(climateRead);
  const rivers = use(riversRead);
  const lakes = use(lakesRead);
  return <>
        {map.landmasses.map((mass) => {
          const continent = overview.continents.find((c) => c.landmass === mass.id);
          const namedPlaces = overview.places.filter((place) => map.regions[place.region].landmass === mass.id);
          if (mass.kind === "island" && !namedPlaces.length) return null;
          const places = mass.regions.map((region) => overview.places.find((place) => place.region === region) ?? { region, names: [], exonyms: [] });
          const terrain = new Map<string, number>();
          for (const region of mass.regions) {
            const land = map.regions[region];
            const name = TERRAIN_NAME[land.terrain].toLowerCase();
            terrain.set(name, (terrain.get(name) ?? 0) + land.areaKm2);
          }
          const area = mass.regions.reduce((sum, region) => sum + map.regions[region].areaKm2, 0);
          const inhabitants = peoples.filter((people) => people.ended === null && people.lands.some((land) => map.regions[land].landmass === mass.id));
          const waterways = map.rivers.filter((river) => river.course.some((region) => map.regions[region].landmass === mass.id));
          const seasons = commonestSeasons(climate, null, mass.regions);
          const zones = climate.zones.filter((zone) => map.climateZones[zone.id].regions.some((region) => map.regions[region].landmass === mass.id));
          return <article className="book-subject" id={`continent-${mass.id}`} key={mass.id}>
            <h3>{lore.continentLabel(overview, map, mass.id)}</h3>
            {continent?.name ? <p>
              “{continent.name.meaning}” · <span className="ipa">/{continent.name.ipa}/</span>;
              entered on the chart in year {continent.name.since * YEARS}, from{" "}
              {ctx.link({ kind: "language", variety: continent.name.variety }, overview.varieties[continent.name.variety]?.name)}.
            </p> : null}
            <p>{mass.regions.length.toLocaleString()} lands, {Math.round(area).toLocaleString()} square kilometres: {[...terrain].sort((a, b) => b[1] - a[1]).map(([name, covered]) => `${name} (${Math.round(covered / area * 100)}%)`).join(", ")}.</p>
            {seasons ? <p>Usual seasons: {seasons}.</p> : null}
            <p>{waterways.length ? <>{countWord(waterways.length)} {waterways.length === 1 ? "river crosses" : "rivers cross"} these lands{waterways.some((river) => rivers[river.id]?.names.length) ? <>; among them {waterways.filter((river) => rivers[river.id]?.names.length).slice(0, 3).map((river, i) => <span key={river.id}>{i ? ", " : ""}{ctx.link({ kind: "river", id: river.id }, lore.waterLabel(overview, rivers, map, "river", river.id))}</span>)}</> : ", none yet named"}.</> : "No river is drawn on these lands."}</p>
            <p>Inhabitants: {inhabitants.length ? inhabitants.map((people, i) => <span key={people.id}>{i ? ", " : ""}{ctx.link({ kind: "people", id: people.id }, lore.peopleLabel(overview, people.id))}</span>) : "no living people holds these lands"}.</p>
            {zones.length ? <p>Weather: {zones.map((zone, i) => <span key={zone.id}>{i ? "; " : ""}{ctx.link({ kind: "zone", id: zone.id }, lore.weatherZoneLabel(zone.id, overview, map))}, {weatherDeparture(zone)}</span>)}.</p> : null}
            <div className="book-apparatus" id={`gazetteer-${mass.id}`}>
              <Folded summary={`The gazetteer: ${countWord(places.length)} lands, their holders, and their names`}>{() => <>{places.map((place) => {
              const holders = overview.communities.filter((c) => c.ended === null && c.lands.includes(place.region));
              const languages = new Set([...place.names.map((n) => n.variety), ...place.exonyms.map((x) => x.variety)]).size;
              return <section className="book-land" id={`land-${place.region}`} key={place.region}>
                <h4>{lore.landCardLabel(overview, map, place.region)}</h4>
                <p>{map.regions[place.region].coastal ? `Coastal ${TERRAIN_NAME[map.regions[place.region].terrain].toLowerCase()}` : TERRAIN_NAME[map.regions[place.region].terrain]}
                  {map.regions[place.region].island ? ", on an island" : ""}.
                  {holders.length ? <> Held by {holders.map((c, i) => <span key={c.id}>{i ? ", " : ""}{ctx.link({ kind: "people", id: c.id }, lore.peopleLabel(overview, c.id))}</span>)}.</> : null}
                </p>
                {languages ? <Folded summary={`Its names, in ${countWord(languages)} ${languages === 1 ? "language" : "languages"}`}>{() => <>
                  <ol className="book-name-ledger">{place.names.map((n, i) => <li key={i}>
                    <span className="gen">{n.since * YEARS}</span>
                    <span><span className="word">{n.spelled}</span> <span className="ipa">/{n.ipa}/</span> “{n.meaning}” · {n.origin},
                      in {ctx.link({ kind: "language", variety: n.variety }, n.language)}
                      {n.by !== null ? <>; coined by {ctx.link({ kind: "people", id: n.by }, lore.peopleLabel(overview, n.by))}</> : null}
                      {n.once ? `; once ${n.once}` : ""}.
                    </span>
                  </li>)}</ol>
                  {place.exonyms.length ? <>
                    <h5>Called by others</h5>
                    <ul className="book-name-ledger book-exonym-ledger">{lore.otherNames(place.exonyms, place.names.at(-1)?.spelled).map((group) => <li key={group.spelled}>
                      <span className="gen" title="First heard in year">{group.heard * YEARS}</span>
                      <span><span className="word">{group.spelled}</span> <span className="ipa">/{group.ipa}/</span>{group.once ? `, once ${group.once}` : ""}</span>
                      <span>{group.varieties.map((variety, i) => <span key={variety}>
                        {i ? ", " : ""}{ctx.link({ kind: "language", variety }, overview.varieties[variety]?.name)}
                      </span>)}</span>
                    </li>)}</ul>
                  </> : null}
                </>}</Folded> : <p className="muted">No name has been recorded for this land.</p>}
              </section>;
            })}</>}</Folded></div>
          </article>;
        })}
        {(() => {
          const islands = map.landmasses.filter((mass) => mass.kind === "island" && !overview.places.some((p) => map.regions[p.region].landmass === mass.id));
          return islands.length ? <p className="book-unnamed">
            {islands.map((mass) => <span id={`continent-${mass.id}`} key={mass.id} />)}
            {lore.capitalized(countWord(islands.length))} {islands.length === 1 ? "island" : "islands"} no people has named,
            {" "}{islands.reduce((sum, mass) => sum + mass.regions.length, 0).toLocaleString()} lands in all.
          </p> : null;
        })()}
        <div className="book-apparatus" id="book-water-apparatus">
          <Folded summary={`Rivers, lakes, and the weather: ${countWord(map.rivers.length)} rivers, ${countWord(map.lakes.length)} lakes, ${countWord(climate.zones.length)} weather zones`}>
            {() => <>
              <h3>Rivers</h3>
              <ul className="roster">{map.rivers.map((river) => <li key={river.id} id={`river-${river.id}`}>
                {lore.waterLabel(overview, rivers, map, "river", river.id)} · {Math.round(river.lengthKm).toLocaleString()} kilometres · {climate.rivers.find((flow) => flow.id === river.id)?.flowing ? "flowing" : "flow has failed"}.
              </li>)}</ul>
              <h3>The weather by land</h3>
              {climate.zones.map((zone) => {
                const seasons = commonestSeasons(climate, zone.id);
                return <section id={`zone-${zone.id}`} key={zone.id}>
                  <h4>{lore.weatherZoneLabel(zone.id, overview, map)}</h4>
                  <p>{lore.capitalized(weatherDeparture(zone))}{seasons ? `; usual seasons: ${seasons}` : ""}.</p>
                </section>;
              })}
              <h3>Lakes</h3>
              <ul className="roster">{map.lakes.map((lake) => <li key={lake.id} id={`lake-${lake.id}`}>
                {lore.waterLabel(overview, lakes, map, "lake", lake.id)} ·{" "}
                {lake.outlet === null ? "no outlet; a closed basin" : <>drains to {ctx.link({ kind: "river", id: lake.outlet }, lore.waterLabel(overview, rivers, map, "river", lake.outlet))}</>}.
              </li>)}</ul>
            </>}
          </Folded>
        </div>
  </>;
}

const LanguageBookChapter = memo(function LanguageBookChapter({ variety: v, ctx, admitted }: { variety: Variety; ctx: ChapterContext; admitted: boolean }) {
  const sections = () => CHAPTER.map(({ id, title: sectionTitle, Section }) => (
    <section className="book-language-section" key={id} id={`language-${v.id}-${id}`}>
      <h4>{sectionTitle}</h4>
      <Pending reading={ctx.engine} label={`Reading ${sectionTitle.toLowerCase()}…`}>
        {FOLDED[id]
          ? <Folded summary={<ChapterSummary variety={v} section={id} ctx={ctx} />}>{() => <Section variety={v} ctx={ctx} />}</Folded>
          : <Section variety={v} ctx={ctx} />}
      </Pending>
    </section>
  ));
  return <article className="book-subject language-chapter" id={`language-${v.id}`}>
    <h3>{v.name}</h3>
    <p className="book-subtitle">{v.meaning ? `“${v.meaning}” · ` : ""}{v.spoken ? "Spoken" : v.silentSince !== null ? `Silent since year ${v.silentSince * YEARS}` : "Silent"} · first spoken in year {v.born * YEARS}</p>
    <Specimen words={v.specimen} onWord={(concept) => ctx.open({ kind: "word", variety: v.id, concept })} />
    {!admitted ? <p className="pending" role="status">Waiting to open this language…</p>
      : v.spoken ? sections() : <Folded summary={`Its chapter: ${listed(CHAPTER.map(({ title: sectionTitle }) => sectionTitle.toLowerCase()), CHAPTER.length)}`}>{sections}</Folded>}
  </article>;
});

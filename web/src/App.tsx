import { useCallback, useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { loadCatalog, loadEngine, message } from "./engine";
import type { Action, Catalog, DecisionView, Engine, HistoryPoint, NotebookNote, Overview, ReadEngine, SettlementChoice, SettlementPreview, WorldMap } from "./model";
import type { Focus } from "./components/Pedia";
import { YEARS } from "./model";
import { ActionDialog, type DialogKind } from "./components/ActionDialog";
import { Book } from "./components/Book";
import { Designer, type Founding as FoundingDesign } from "./components/Designer";
import { Modal } from "./components/Modal";
import { ChartRoom } from "./components/ChartRoom";
import { Stage, type UndoDestination } from "./components/Stage";
import { TellingComparison } from "./components/TellingComparison";
import type { MapMotionReading } from "./components/MapView";
import { Founding } from "./components/Founding";
import { Pending } from "./components/Pending";
import { sampleWorld } from "./sample";
import { download } from "./takeout";
import { useLiftedValue } from "./motion";
import { format as formatRoute, parse, push, replace, type Route } from "./route";
import { EVENT_KIND } from "./lore";
import { individualAnnals } from "./history";
import {
  bringBack,
  describe,
  discardDraft,
  loadShelf,
  newBookId,
  onShelfChange,
  putAway,
  rawShelf,
  readBook,
  readPlace,
  rebuildShelf,
  removeForGood,
  saveBook,
  savePlace,
  scanBooks,
  setLast,
  StaleSave,
  worldName,
  type BookEntry,
  type ReadingPlace,
  type ScannedBook,
  type Shelf as ShelfIndex,
} from "./shelf";

/// Longest wait for something to happen before giving up.
const EVENT_LIMIT = 400;

const DECISION_NAME: Record<DecisionView["kind"], string> = {
  settle: "the settlement", found: "the founding", "found-related": "the founding",
  connect: "the contact", shift: "the language shift", state: "the state decision",
  religion: "the faith decision", craft: "the craft decision", temper: "the change of temper", law: "the sound law",
};

type View =
  | { kind: "loading" }
  | { kind: "shelf" }
  | { kind: "setup"; draft?: string }
  | { kind: "book"; id: string }
  | { kind: "recovery"; what: string; raw: string; error: string; books?: ScannedBook[] };

function shelfRecovery(error: unknown): View {
  let books: ScannedBook[] = [];
  let reason = message(error);
  try { books = scanBooks(); }
  catch (e) { reason += ` The saved worlds could not be listed: ${message(e)}.`; }
  return { kind: "recovery", what: "The shelf", raw: rawShelf() ?? "", error: reason, books };
}

interface OpenBookSave {
  id: string;
  revision: number;
  history: string | null;
  keptTitle?: { title: string; named: boolean };
}


/// Which telling and year are on screen. Null telling and point read the
/// present telling; null viewing is its latest year.
interface Place { telling: number | null; point: HistoryPoint | null; viewing: number | null }
const PRESENT: Place = { telling: null, point: null, viewing: null };
/// A place opened: its reading and the year's overview, committed together so
/// the stage never draws one reading's year with another's views.
interface Frame extends Place { reading: ReadEngine; latest: number; overview: Overview }

function foundingAction(f: FoundingDesign): Action {
  return { kind: "found", naming: f.naming, design: f.design, seed: f.seed, power: f.power, openness: f.openness };
}

export default function App() {
  const [view, setView] = useState<View>({ kind: "loading" });
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [shelf, setShelf] = useState<ShelfIndex>({ books: [], last: null, drafts: [], putAway: [] });
  const mapMotion = useRef<MapMotionReading | null>(null);
  const chartPlace = useRef<ReadingPlace["chart"]>(undefined);
  const engine = useRef<Engine | null>(null);
  const [version, setVersion] = useState(0);
  // Entity IDs belong to one telling. Replacing it starts a fresh reading
  // so cards and language filters cannot silently point into another branch.
  const [tellingVersion, setTellingVersion] = useState(0);
  const [initialFocus, setInitialFocus] = useState<Focus | null>(null);
  const [frame, setFrame] = useState<Frame | null>(null);
  const [compareWith, setCompareWith] = useState<number | null>(null);
  const [notebook, setNotebook] = useState<NotebookNote[]>([]);
  const [community, setCommunity] = useState(0);
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [route, setRoute] = useState<Route>({ kind: "shelf" });
  const routeRef = useRef<Route>(route);
  const page = route.kind === "world" ? route.page : "stage";
  const bookChapter = route.kind === "world" ? route.chapter ?? null : null;
  /// Years a newly founded world runs to when the workshop first opens.
  const [firstRun, setFirstRun] = useState<number | null>(null);
  const [runTo, setRunTo] = useState<number | null>(null);
  const runToRef = useRef<number | null>(null);
  const runningRef = useRef(false);
  const [interruptedRun, setInterruptedRun] = useState<{ year: number; target: number } | null>(null);
  const [writing, setWriting] = useState(false);
  const readyId = useRef<string | null>(null);
  const [comparisonReturn, setComparisonReturn] = useState(0);
  const [sheetReturn, setSheetReturn] = useState(0);
  const liftedDialog = useLiftedValue(dialog, () => setSheetReturn((n) => n + 1));
  const liftedComparison = useLiftedValue(compareWith, () => setComparisonReturn((n) => n + 1));
  const [worldMap, setWorldMap] = useState<WorldMap | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const [opening, setOpening] = useState<string | null>(null);
  const [newerBook, setNewerBook] = useState<string | null>(null);
  const openedBook = useRef<OpenBookSave | null>(null);
  const openingRequest = useRef(0);
  const pendingWrites = useRef(0);
  const bookId = view.kind === "book" ? view.id : null;

  // The present telling at its latest year, which the shelf describes. Kept
  // whenever it is opened, so saving never waits on the engine's thread.
  const present = useRef<Overview | null>(null);
  const placeRef = useRef<{ frame: Frame | null; focus: Focus }>({ frame: null, focus: { kind: "world" } });
  const keepPlace = useCallback(() => {
    const id = openedBook.current?.id;
    const { frame: reading, focus: subject } = placeRef.current;
    if (id && readyId.current === id && reading) {
      const book = readPlace(id)?.book;
      savePlace(id, {
        telling: reading.telling, point: reading.point, viewing: reading.viewing, focus: subject,
        ...(runToRef.current === null ? {} : { runTo: runToRef.current }),
        ...(book ? { book } : {}),
        ...(chartPlace.current ? { chart: chartPlace.current } : {}),
      });
    }
  }, []);
  const persist = useCallback(() => {
    const world = engine.current;
    const saved = openedBook.current;
    if (!world || !saved || readyId.current !== saved.id || !present.current) return true;
    try {
      const currentHistory = world.save();
      const entry = describe(saved.id, present.current, world.title(), world.author());
      entry.generation = world.latest();
      let history = currentHistory;
      if (saved.keptTitle) {
        entry.title = saved.keptTitle.title;
        entry.named = saved.keptTitle.named;
        history = JSON.stringify({ ...JSON.parse(currentHistory), title: entry.title }, null, 2);
      }
      let kept;
      try {
        kept = saveBook(entry, history, saved.revision);
      } catch (e) {
        if (!(e instanceof StaleSave)) throw e;
        const id = newBookId();
        const title = `${entry.title} (kept from another tab)`;
        const named = entry.named === true;
        const copy = { ...entry, id, title, named };
        const copyHistory = JSON.stringify({ ...JSON.parse(currentHistory), title }, null, 2);
        kept = saveBook(copy, copyHistory, 0);
        saved.id = id;
        saved.keptTitle = { title, named };
        readyId.current = id;
        const currentRoute = routeRef.current;
        const destination: Route = currentRoute.kind === "world"
          ? { ...currentRoute, id } : { kind: "world", id, page: "stage" };
        replace(destination);
        routeRef.current = destination;
        setRoute(destination);
        setView({ kind: "book", id });
        setInfo("Another tab saved this world. Both versions are kept; this tab is now reading its own copy.");
      }
      saved.revision = kept.revision;
      saved.history = currentHistory;
      setShelf(kept.shelf);
      keepPlace();
      setNewerBook(null);
      setSaveError(null);
      return true;
    } catch (e) {
      setSaveError(`Not saved in this browser: ${message(e)}. The book's save file still works.`);
      return false;
    }
  }, [keepPlace]);

  const startWriting = useCallback((target: number) => {
    runningRef.current = true; setWriting(true);
    runToRef.current = target; setRunTo(target); setInterruptedRun(null);
    setDialog(null);
    keepPlace();
  }, [keepPlace]);
  const writingTarget = useCallback((target: number) => {
    runToRef.current = target; setRunTo(target);
    keepPlace();
  }, [keepPlace]);
  const stopWriting = useCallback(() => {
    runningRef.current = false; setWriting(false);
    runToRef.current = null; setRunTo(null);
    persist();
  }, [persist]);
  const dismissRun = useCallback(() => {
    runToRef.current = null; setRunTo(null); setInterruptedRun(null);
    keepPlace();
  }, [keepPlace]);
  useEffect(() => {
    if (!writing) return;
    const timer = window.setInterval(() => { if (runningRef.current) persist(); }, 3000);
    return () => window.clearInterval(timer);
  }, [writing, persist]);

  useEffect(() => {
    // Checkpoints and lifecycle saves use the worker's last completed chunk,
    // synchronously: a closing page cannot wait for another engine reading.
    const beforeUpdate = (event: Event) => {
      if (!persist()) event.preventDefault();
    };
    window.addEventListener("umran:before-update", beforeUpdate);
    const pagehide = () => { if (runningRef.current) persist(); };
    const hidden = () => { if (runningRef.current && document.visibilityState === "hidden") persist(); };
    window.addEventListener("pagehide", pagehide);
    document.addEventListener("visibilitychange", hidden);
    return () => {
      window.removeEventListener("umran:before-update", beforeUpdate);
      window.removeEventListener("pagehide", pagehide);
      document.removeEventListener("visibilitychange", hidden);
    };
  }, [persist]);

  // Saving needs the book open first, so new books save on the next render.
  const [unsaved, setUnsaved] = useState(false);
  useEffect(() => {
    if (unsaved && bookId !== null) {
      if (persist()) setUnsaved(false);
    }
  }, [unsaved, bookId, persist]);

  /// Opens a place on the engine's thread, then commits it and `then`'s state
  /// in one render. A later call supersedes an earlier one still reading,
  /// which then resolves to null.
  const showing = useRef(0);
  const show = useCallback(async (place: Place, then?: (frame: Frame) => void): Promise<Frame | null> => {
    const world = engine.current;
    if (!world) return null;
    const request = ++showing.current;
    const atPresent = place.telling === null && place.point === null;
    const reading = atPresent ? world.tip()
      : world.read(place.telling ?? (await world.tip().overview(world.latest())).telling, place.point);
    const latest = await reading.latest();
    const overview = await reading.overview(place.viewing === null ? latest : Math.min(place.viewing, latest));
    // The chart's own views, so the stage draws the year at once.
    await Promise.all([reading.climate(overview.generation), reading.rivers(overview.generation), reading.lakes(overview.generation)]);
    if (request !== showing.current || world !== engine.current) return null;
    if (atPresent && overview.generation === latest) present.current = overview;
    const next: Frame = { ...place, reading, latest, overview };
    placeRef.current.frame = next;
    flushSync(() => { setFrame(next); then?.(next); });
    return next;
  }, []);

  const adopt = useCallback(async (id: string, next: Engine, fresh: boolean, entry?: BookEntry, placeOverride?: ReadingPlace, canAdopt?: () => boolean, request = openingRequest.current) => {
    const [map, tip] = await Promise.all([next.map(), next.tip().overview(next.latest())]);
    if (!fresh && !entry) {
      const stored = loadShelf();
      entry = [...stored.books, ...stored.putAway].find((book) => book.id === id);
      setShelf(stored);
    }
    if (request !== openingRequest.current || (canAdopt && !canAdopt())) { next.dispose(); return false; }
    // The world on screen stays readable until the new one is committed.
    const previous = engine.current;
    engine.current = next;
    present.current = tip;
    openedBook.current = {
      id, revision: entry?.revision ?? 0, history: fresh ? null : next.save(),
      keptTitle: entry?.named === false && next.title() !== null ? { title: entry.title, named: false } : undefined,
    };
    readyId.current = null;
    setNewerBook(null);
    setSaveError(null);
    mapMotion.current = null;
    // Reopen where the reader left off, if that reading still exists; a place
    // from another engine revision or a removed telling opens the latest year.
    const place = fresh ? null : placeOverride ?? readPlace(id);
    const opened = place ? await show({ telling: place.telling, point: place.point, viewing: place.viewing }).catch(() => null) : null;
    if (!opened) await show(PRESENT);
    if (previous !== next) previous?.dispose();
    if (request !== openingRequest.current || engine.current !== next) return false;
    chartPlace.current = place?.chart;
    readyId.current = id;
    setNotebook(next.notebook());
    setWorldMap(map);
    setCompareWith(null);
    setInitialFocus(opened && place ? place.focus : null);
    setFocus(opened && place ? place.focus : { kind: "world" });
    setTellingVersion((v) => v + 1);
    setCommunity(0);
    setDialog(null);
    const target = place?.runTo;
    runningRef.current = false; setWriting(false);
    const unfinished = typeof target === "number" && Number.isFinite(target) && target > tip.latest * YEARS ? target : null;
    runToRef.current = unfinished;
    setRunTo(unfinished);
    setInterruptedRun(unfinished === null ? null : { year: tip.latest * YEARS, target: unfinished });
    if (fresh) {
      // A begun founding belongs to the chart room, not an unfinished form
      // reached by going Back from its workshop.
      if (routeRef.current.kind === "setup") replace({ kind: "shelf" });
      const destination: Route = { kind: "world", id, page: "stage" };
      push(destination);
      routeRef.current = destination;
      setRoute(destination);
    }
    setError(null);
    setInfo(null);
    setVersion((v) => v + 1);
    setView({ kind: "book", id });
    if (fresh) setUnsaved(true);
    return true;
  }, [show]);

  const [focus, setFocus] = useState<Focus>({ kind: "world" });
  placeRef.current.focus = focus;
  useEffect(() => {
    keepPlace();
  }, [view, frame, focus, runTo, keepPlace]);

  const openBook = useCallback(async (id: string, place?: ReadingPlace, fromAnotherTab = false): Promise<void> => {
    const request = ++openingRequest.current;
    const previous = engine.current;
    const previousHistory = previous?.save();
    const safeToRefresh = () => request === openingRequest.current
      && (!fromAnotherTab || (previous === engine.current && !runningRef.current && pendingWrites.current === 0
        && previous?.save() === previousHistory && openedBook.current?.history === previousHistory));
    let entry: BookEntry | undefined;
    try {
      const stored = loadShelf();
      setShelf(stored);
      entry = [...stored.books, ...stored.putAway].find((book) => book.id === id);
    } catch (e) { setView(shelfRecovery(e)); return; }
    const raw = readBook(id);
    if (raw === null) {
      setError("That world could not be found in this browser.");
      if (!fromAnotherTab) {
        const destination: Route = { kind: "shelf" };
        replace(destination); routeRef.current = destination; setRoute(destination);
        setOpening(null);
        setView({ kind: "shelf" });
      }
      return;
    }
    setOpening(id);
    try {
      const next = await loadEngine(raw);
      if (!safeToRefresh()) {
        next.dispose();
        if (fromAnotherTab && request === openingRequest.current) setNewerBook(id);
        return;
      }
      const adopted = await adopt(id, next, false, entry, place, safeToRefresh, request);
      // adopt(false) leaves the current route, page, and chapter untouched.
      if (fromAnotherTab && request === openingRequest.current && !adopted) setNewerBook(id);
    } catch (e) {
      if (request !== openingRequest.current) return;
      // Keep malformed single-world recovery separate from an unreadable index.
      try { setShelf(setLast(null)); } catch { /* The original is still kept. */ }
      setView({ kind: "recovery", what: "This world", raw, error: message(e) });
    } finally {
      if (request === openingRequest.current) setOpening(null);
    }
  }, [adopt]);

  const applyRoute = useCallback((destination: Route) => {
    routeRef.current = destination;
    setRoute(destination);
    setDialog(null); setCompareWith(null); setFirstRun(null); setError(null);
    if (destination.kind === "world") {
      if (readyId.current === destination.id && openedBook.current?.id === destination.id && engine.current && placeRef.current.frame) {
        ++openingRequest.current;
        setOpening(null);
        setView({ kind: "book", id: destination.id });
      } else void openBook(destination.id);
    } else {
      ++openingRequest.current; ++showing.current;
      setOpening(null);
      setView(destination);
    }
  }, [openBook]);
  const navigate = useCallback((destination: Route, method: "push" | "replace" = "push") => {
    if (method === "push") {
      if (formatRoute(destination) !== formatRoute(routeRef.current)) push(destination);
    } else replace(destination);
    applyRoute(destination);
  }, [applyRoute]);

  useEffect(() => {
    loadCatalog().then(setCatalog, (e) => setError(message(e)));
    let loaded: ShelfIndex;
    try {
      loaded = loadShelf();
    } catch (e) {
      setView(shelfRecovery(e));
      return;
    }
    setShelf(loaded);
    const destination = parse(window.location.hash, loaded.last);
    if (!destination || (destination.kind === "world" && !loaded.books.some((entry) => entry.id === destination.id))) {
      const room: Route = { kind: "shelf" };
      replace(room); applyRoute(room);
      setError(destination ? "That world could not be found in this browser." : "That page could not be found. Your worlds are still in the chart room.");
      return;
    }
    // Seed a direct link's screen ancestry once. A reload already carries
    // our history state and must not add another copy of these destinations.
    if (window.history.state?.umran !== true && destination.kind !== "shelf") {
      replace({ kind: "shelf" });
      if (destination.kind === "world" && destination.page === "book") push({ kind: "world", id: destination.id, page: "stage" });
      push(destination);
    } else replace(destination);
    applyRoute(destination);
  }, [applyRoute]);

  useEffect(() => {
    const backOrForward = () => {
      const destination = parse(window.location.hash, shelf.last);
      if (!destination || (destination.kind === "world" && !shelf.books.some((entry) => entry.id === destination.id))) {
        const room: Route = { kind: "shelf" };
        replace(room); applyRoute(room);
        setError(destination ? "That world could not be found in this browser." : "That page could not be found. Your worlds are still in the chart room.");
      } else applyRoute(destination);
    };
    window.addEventListener("popstate", backOrForward);
    return () => window.removeEventListener("popstate", backOrForward);
  }, [applyRoute, shelf]);

  useEffect(() => onShelfChange(({ book }) => {
    let latestShelf: ShelfIndex;
    try {
      latestShelf = loadShelf();
      setShelf(latestShelf);
    } catch (e) { setView(shelfRecovery(e)); return; }
    const saved = openedBook.current;
    const world = engine.current;
    if (view.kind !== "book" || !saved || !world || (book !== null && book !== saved.id)) return;
    const stored = [...latestShelf.books, ...latestShelf.putAway].find((entry) => entry.id === saved.id);
    if (!stored || (stored.revision ?? 0) <= saved.revision) return;
    if (runningRef.current || pendingWrites.current > 0 || world.save() !== saved.history) {
      setNewerBook(saved.id);
      return;
    }
    const place = frame ? {
      telling: frame.telling, point: frame.point, viewing: frame.viewing, focus,
      ...(runToRef.current === null ? {} : { runTo: runToRef.current }),
    } : undefined;
    void openBook(saved.id, place, true);
  }), [view, frame, focus, openBook]);

  // Record the destination after the stage's playback cleanup has saved.
  // Opening an unchanged world must also make it the one resumed next time.
  useEffect(() => {
    if (view.kind !== "book") return;
    try { setShelf(setLast(view.id)); }
    catch (e) { setView(shelfRecovery(e)); }
  }, [view]);

  const toShelf = useCallback(() => navigate({ kind: "shelf" }), [navigate]);

  const overview = frame?.overview ?? null;
  const latest = frame?.latest ?? 0;
  const generation = overview?.generation ?? 0;
  const selected = overview ? Math.min(community, overview.communities.length - 1) : 0;
  const canUndo = !!overview && (overview.point.offset > 0 || overview.point.action > 1);
  const [undoReading, setUndoReading] = useState<{ frame: Frame; destination: UndoDestination } | null>(null);
  const undoDestination = undoReading && undoReading.frame === frame ? undoReading.destination : null;
  useEffect(() => {
    const world = engine.current;
    if (!world || !frame || !canUndo || writing || view.kind !== "book") return;
    let cancelled = false;
    const readDestination = async () => {
      try {
        const [previous, decisions] = await Promise.all([
          world.previous({ telling: frame.overview.telling, point: frame.overview.point }),
          frame.reading.decisions(),
        ]);
        const before = await world.read(previous.telling).overviewAt(previous.point);
        const decision = decisions.find((entry) => entry.index === previous.point.action);
        const annal = individualAnnals(frame.overview.annals).find((entry) => entry.decision === previous.point.action);
        const shortName = annal ? EVENT_KIND[annal.kind].name : decision ? DECISION_NAME[decision.kind] : "the years ran";
        const label = `Return to before ${shortName.charAt(0).toLowerCase()}${shortName.slice(1)}, year ${before.generation * YEARS}`;
        if (!cancelled && world === engine.current) setUndoReading({ frame, destination: { reading: previous, label } });
      } catch (e) { if (!cancelled && world === engine.current) setError(message(e)); }
    };
    void readDestination();
    return () => { cancelled = true; };
  }, [frame, canUndo, writing, view.kind]);
  const scrub = useCallback(async (g: number): Promise<HistoryPoint | null> => {
    if (!engine.current || !frame) return null;
    try {
      const point = (await engine.current.read(frame.overview.telling).overview(g)).point;
      return await show({ telling: frame.telling, point: null, viewing: g >= frame.latest ? null : g }) ? point : null;
    } catch (e) { setError(message(e)); return null; }
  }, [frame, show]);

  /// After a change: the present telling at its latest year, or, if the
  /// change began another telling, a fresh reading of it.
  const changed = useCallback((from: Overview, then?: (frame: Frame) => void) => show(PRESENT, (next) => {
    if (!from.atTip || next.overview.telling !== frame?.overview.telling) {
      setInitialFocus(null); setCommunity(0); setTellingVersion((v) => v + 1);
    }
    setVersion((v) => v + 1);
    setError(null);
    then?.(next);
  }), [frame, show]);

  const run = useCallback(
    async (action: Action, from: Overview | null = overview) => {
      const current = engine.current;
      if (!current || !from) return;
      pendingWrites.current++;
      try {
        await current.actAt({ telling: from.telling, point: from.point }, from.mutation, action);
        await changed(from);
        persist();
      } catch (e) {
        setError(message(e));
      }
      finally { pendingWrites.current--; }
    },
    [overview, changed, persist],
  );

  // Acting while viewing the past begins another telling from there; the
  // later years are set aside, not lost.
  const perform = run;

  const readTelling = useCallback(async (telling: number, at?: number, focus?: Focus) => {
    const world = engine.current;
    if (!world) return;
    try {
      // Validate a saved telling before switching; an unreadable alternate
      // stays listed and keeps its original recipe for recovery/export.
      const last = await world.read(telling).latest();
      await show({ telling, point: null, viewing: at === undefined || at >= last ? null : at }, () => {
        setInitialFocus(focus ?? null); setTellingVersion((v) => v + 1);
        setCompareWith(null); setError(null);
      });
    } catch (e) { setError(`This telling could not be read: ${message(e)}. Its saved telling is still kept.`); }
  }, [show]);
  const renameTelling = async (telling: number, name: string) => {
    pendingWrites.current++;
    try {
      await engine.current?.rename(telling, name);
      if (frame) await show(frame, () => setVersion((v) => v + 1));
      persist();
    } catch (e) { setError(message(e)); }
    finally { pendingWrites.current--; }
  };

  const saveNote = async (note: NotebookNote): Promise<string | null> => {
    pendingWrites.current++;
    try {
      await engine.current?.saveNote(note);
      setNotebook(engine.current?.notebook() ?? []);
      return persist() ? null : "The entry is kept in this open world, but browser storage failed. Export a save file before closing.";
    } catch (e) { return message(e); }
    finally { pendingWrites.current--; }
  };
  const removeNote = async (id: string): Promise<string | null> => {
    pendingWrites.current++;
    try {
      await engine.current?.removeNote(id);
      setNotebook(engine.current?.notebook() ?? []);
      return persist() ? null : "The note was removed in this open world, but browser storage failed. Export a save file before closing.";
    } catch (e) { return message(e); }
    finally { pendingWrites.current--; }
  };
  const openNote = async (note: NotebookNote): Promise<string | null> => {
    try {
      const destination = await engine.current?.resolveNote(note.id);
      if (!destination) return "This note has no fixed year.";
      await show({ telling: destination.reading.telling, point: destination.reading.point, viewing: null }, () => {
        setInitialFocus(destination.subject); setTellingVersion((v) => v + 1); setError(null);
      });
      return null;
    } catch (e) { return message(e); }
  };

  // One completed frame; periodic checkpoints and the final stop save it.
  const runYears = useCallback(async (generations: number): Promise<boolean> => {
    const current = engine.current;
    if (!current || !overview) return false;
    pendingWrites.current++;
    try {
      await current.actAt({ telling: overview.telling, point: overview.point }, overview.mutation, { kind: "run", generations });
      return await changed(overview) !== null;
    } catch (e) {
      setError(message(e));
      return false;
    }
    finally { pendingWrites.current--; }
  }, [overview, changed]);
  const tick = useCallback(() => runYears(1), [runYears]);

  const continueTelling = useCallback(async (targetYear: number): Promise<boolean> => {
    const current = engine.current;
    if (!current || !overview || overview.atTip || pendingWrites.current > 0 || !Number.isFinite(targetYear) || targetYear <= overview.generation * YEARS) return false;
    const target = Math.ceil(targetYear / YEARS) * YEARS;
    pendingWrites.current++;
    startWriting(target);
    try {
      // The first chunk forks at the exact reading point atomically. The
      // remounted workshop writes the rest through its usual run loop.
      await current.actAt({ telling: overview.telling, point: overview.point }, overview.mutation, { kind: "run", generations: 1 });
      const next = await changed(overview, (after) => {
        if (after.overview.generation * YEARS < target) setFirstRun(target);
      });
      persist();
      if (!next || next.overview.generation * YEARS >= target) stopWriting();
      return next !== null;
    } catch (e) {
      setError(message(e));
      stopWriting();
      return false;
    } finally { pendingWrites.current--; }
  }, [overview, changed, persist, startWriting, stopWriting]);

  const nextEvent = useCallback(
    async (limit: number): Promise<boolean> => {
      const current = engine.current;
      if (!current || !overview) return false;
      pendingWrites.current++;
      try {
        const ran = await current.untilAt({ telling: overview.telling, point: overview.point }, overview.mutation, limit);
        return await changed(overview, () => setInfo(ran >= limit ? `${limit * YEARS} years passed, and nothing of note befell.` : null)) !== null;
      } catch (e) { setError(message(e)); return false; }
      finally { pendingWrites.current--; }
    },
    [overview, changed],
  );

  const begin = useCallback(async (next: Engine, title: string | null, author: string | null, years: number, id: string) => {
    await next.setTitle(title ?? "");
    await next.setAuthor(author ?? "");
    const tip = await next.tip().overview(next.latest());
    const saved = saveBook(describe(id, tip, next.title(), next.author()), next.save(), 0);
    setShelf(saved.shelf);
    try { localStorage.setItem("umran.author", author ?? ""); }
    catch { /* The author is still kept in the world save. */ }
    // This world is already saved; do not schedule another initial save.
    if (!await adopt(id, next, false)) return;
    // A begun founding belongs to the chart room, not an unfinished form
    // reached by going Back from its workshop.
    if (routeRef.current.kind === "setup") replace({ kind: "shelf" });
    const destination: Route = { kind: "world", id, page: "stage" };
    push(destination);
    routeRef.current = destination;
    setRoute(destination);
    setFirstRun(years);
    try { setShelf(discardDraft(id)); }
    catch (failure) { setSaveError(`The world is saved, but its founding draft could not be cleared: ${message(failure)}.`); }
  }, [adopt]);

  const sample = async () => {
    let next: Engine | null = null;
    try {
      next = await sampleWorld();
      const expected = JSON.parse(next.save()) as Record<string, unknown>;
      // The title distinguishes copies; the actual history and notes decide
      // whether this is still the untouched chronicle already written.
      delete expected.title;
      delete expected.author;
      const stored = loadShelf();
      const allBooks = [...stored.books, ...stored.putAway];
      const untouched = allBooks.find((book) => {
        const raw = readBook(book.id);
        if (!raw) return false;
        try {
          const recipe = JSON.parse(raw) as Record<string, unknown>;
          delete recipe.title;
          delete recipe.author;
          return JSON.stringify(recipe) === JSON.stringify(expected);
        } catch { return false; }
      });
      if (untouched) {
        next.dispose();
        next = null;
        if (stored.putAway.some((book) => book.id === untouched.id)) setShelf(bringBack(untouched.id));
        navigate({ kind: "world", id: untouched.id, page: "stage" });
        return;
      }
      const sampleOverview = await next.tip().overview(next.latest());
      const name = `${worldName(sampleOverview) ?? "A world"} · sample chronicle`;
      let title = name;
      let copy = 2;
      while (allBooks.some((book) => book.title === title)) title = `${name} ${copy++}`;
      await next.setTitle(title);
      await adopt(newBookId(), next, true);
      next = null;
    } catch (e) {
      next?.dispose();
      setError(`The sample could not be opened: ${message(e)}`);
    }
  };

  const importFile = async (file: File) => {
    try {
      await adopt(newBookId(), await loadEngine(await file.text()), true);
    } catch (e) {
      setError(`Could not bring in ${file.name}: ${message(e)}`);
    }
  };
  if (view.kind === "loading" || !catalog) {
    return <main className="splash title-splash">
      <span className="brand"><span className="brand-name"><span>ʿUmrān</span></span></span>
      <p>The book of a world: its peoples, their fortunes, and the languages that grew out of both.</p>
      <p className={error ? "error" : "muted"} role={error ? "alert" : "status"}>{error ?? "Setting out the instruments…"}</p>
    </main>;
  }

  if (view.kind === "recovery") {
    return (
      <main className="splash">
        <h1>{view.what} could not be opened</h1>
        <p>{view.error}</p>
        <p>It has not been changed or deleted. Download it before going on.</p>
        {view.books ? <>
          <p>These worlds were found in this browser, even though their list could not be read.</p>
          <ul className="recovery-worlds">
            {view.books.map((book) => <li key={book.id}>
              <strong>{book.title ?? `A saved world (${book.id})`}</strong>
              <small>{book.generation === null ? "Year unreadable" : `year ${book.generation * YEARS}`} · {book.bytes.toLocaleString()} bytes</small>
              <button type="button" onClick={() => download(`umran-world-${book.id}.json`, book.raw)}>Download this world</button>
            </li>)}
          </ul>
          {!view.books.length ? <p>No saved worlds could be found.</p> : null}
          <p>Download all keeps the original saved data together in one file.</p>
          <div className="row">
            <button type="button" onClick={() => download("umran-recovered-worlds.json", JSON.stringify({ shelf: view.raw, books: view.books }, null, 2))}>Download all</button>
            <button type="button" className="primary" onClick={() => {
              try {
                setShelf(rebuildShelf());
                setError(null);
                navigate({ kind: "shelf" }, "replace");
              } catch (e) { setView(shelfRecovery(e)); }
            }}>Rebuild the list of worlds</button>
          </div>
        </> : null}
        <div className="row">
          <button type="button" onClick={() => download("umran-unreadable.json", view.raw)}>
            Download saved data
          </button>
          <button type="button" onClick={toShelf}>
            Go to the chart room
          </button>
        </div>
      </main>
    );
  }

  if (view.kind === "setup") {
    return (
      <div className="app">
        <Founding
          catalog={catalog}
          draft={view.draft}
          onDraft={(draft) => navigate({ kind: "setup", draft }, "replace")}
          onBegin={begin}
          onChartRoom={toShelf}
        />
      </div>
    );
  }

  if (view.kind === "shelf" || !frame || !overview || !engine.current || !worldMap) {
    return (
      <>
        <ChartRoom
          revision={catalog.revision}
          shelf={shelf}
          opening={opening}
          onOpen={(id) => navigate({ kind: "world", id, page: "stage" })}
          onBegin={() => navigate({ kind: "setup" })}
          onSample={sample}
          onImport={(f) => void importFile(f)}
          onFailure={(failure) => setView(shelfRecovery(failure))}
          onPutAway={(id) => {
            try { setShelf(putAway(id)); return true; }
            catch (e) { setError(`Could not put it away: ${message(e)}`); return false; }
          }}
          onBringBack={(id) => {
            try { setShelf(bringBack(id)); return true; }
            catch (e) { setError(`Could not bring it back: ${message(e)}`); return false; }
          }}
          onRemoveForGood={(id) => {
            try { setShelf(removeForGood(id)); return true; }
            catch (e) { setError(`Could not remove it: ${message(e)}`); return false; }
          }}
          onDownload={(id) => {
            const raw = readBook(id);
            if (raw !== null) download(`umran-world-${id}.json`, raw);
            else setError("That world's saved data could not be found in this browser.");
          }}
          onResumeDraft={(id) => navigate({ kind: "setup", draft: id })}
          onDiscardDraft={(id) => {
            try { setShelf(discardDraft(id)); return true; }
            catch (e) { setError(`Could not discard it: ${message(e)}`); return false; }
          }}
        />
        {error ? <p className="notice error">{error}</p> : null}
      </>
    );
  }

  const title = openedBook.current?.keptTitle?.title ?? engine.current?.title() ?? worldName(overview) ?? "A new world";

  const notices = (
    <>
      {overview.savedRevision !== null ? (
        <p className="notice">
          This world was saved with engine revision {overview.savedRevision}; this is revision{" "}
          {overview.revision}, so its chronicle and languages may differ from when it was saved.
        </p>
      ) : null}
      {error ? (
        <p className="notice error" role="alert">
          {error} <button type="button" className="link" onClick={() => setError(null)}>Dismiss</button>
        </p>
      ) : null}
      {saveError ? <p className="notice error">{saveError}</p> : null}
      {newerBook === view.id ? <p className="notice" role="status">
        Another tab has saved a newer version of this world. Your open version has not been replaced.{" "}
        <button type="button" className="link" disabled={opening !== null || pendingWrites.current > 0 || runningRef.current} onClick={() => {
          const id = newerBook;
          const currentRoute = routeRef.current;
          if (!id || !persist()) return;
          const destination: Route = currentRoute.kind === "world"
            ? { ...currentRoute, id } : { kind: "world", id, page: "stage" };
          replace(destination); routeRef.current = destination; setRoute(destination);
          const place = {
            telling: frame.telling, point: frame.point, viewing: frame.viewing, focus,
            ...(runToRef.current === null ? {} : { runTo: runToRef.current }),
          };
          void openBook(id, place);
        }}>Open the newer version</button>
      </p> : null}
      {info ? (
        <p className="notice" role="status">
          {info} <button type="button" className="link" onClick={() => setInfo(null)}>Dismiss</button>
        </p>
      ) : null}
    </>
  );

  const strike = async () => {
    if (!engine.current || !undoDestination) return;
    try {
      const previous = undoDestination.reading;
      await show({ telling: previous.telling, point: previous.point, viewing: frame.viewing }, () => {
        setInitialFocus(null); setTellingVersion((v) => v + 1);
      });
    } catch (e) { setError(message(e)); }
  };

  const settle = async (choice: SettlementChoice, preview: SettlementPreview) => {
    const current = engine.current;
    if (!current) return;
    pendingWrites.current++;
    try {
      await current.actAt({ telling: preview.telling, point: preview.point }, preview.mutation, { kind: "settle", ...choice });
      await show(PRESENT, (after) => {
        const event = after.overview.annals.findLast((a) => a.kind === "settlement");
        setInitialFocus(event ? { kind: "event", id: event.id } : { kind: "people", id: choice.community });
        setCommunity(event?.settlement?.daughter ?? choice.community);
        setTellingVersion((v) => v + 1);
        setVersion((v) => v + 1);
        setError(null);
      });
      persist();
    } catch (e) { setError(message(e)); }
    finally { pendingWrites.current--; }
  };

  const dialogs =
    liftedDialog.value === "found" ? (
      <Modal open={dialog !== null} wide title="A new people arrives" onClose={() => setDialog(null)}>
        {!overview.atTip ? <p className="telling-note">Writing in year {generation * YEARS} begins another telling. The years through {latest * YEARS} stay in the chronicle.</p> : null}
        <Designer
          catalog={catalog}
          submit="Found them"
          onCancel={() => setDialog(null)}
          onFound={(f) => {
            setDialog(null);
            perform(foundingAction(f));
          }}
        />
      </Modal>
    ) : liftedDialog.value ? (
      <ActionDialog
        open={dialog !== null}
        kind={liftedDialog.value}
        catalog={catalog}
        overview={overview}
        engine={frame.reading}
        selected={selected}
        onClose={() => setDialog(null)}
        onAction={(action) => {
          setDialog(null);
          perform(action);
        }}
      />
    ) : null;

  return (
    <div className="app">
      <Pending reading={frame.reading}>
      <Stage
        key={`${view.id}:${tellingVersion}`}
        hidden={page === "book"}
        engine={frame.reading}
        catalog={catalog}
        map={worldMap}
        version={version}
        generation={generation}
        overview={overview}
        title={title}
        notices={notices}
        initialFocus={initialFocus}
        onFocus={setFocus}
        initialChart={chartPlace.current}
        onChart={(chart) => { chartPlace.current = chart; keepPlace(); }}
        onSettle={settle}
        comparisonReturn={comparisonReturn}
        sheetReturn={sheetReturn}
        mapMotion={mapMotion}
        notes={notebook}
        onSaveNote={saveNote}
        onRemoveNote={removeNote}
        onReadNote={openNote}
        onReadPoint={(point) => void show({ telling: overview.telling, point, viewing: null })}
        canUndo={canUndo}
        undoDestination={undoDestination}
        selected={selected}
        onSelect={setCommunity}
        onShelf={toShelf}
        onBook={(chapter) => navigate({ kind: "world", id: view.id, page: "book", ...(chapter ? { chapter } : {}) })}
        onRestore={readTelling}
        onRenameTelling={renameTelling}
        onCompare={setCompareWith}
        onScrub={scrub}
        onTick={tick}
        onRun={runYears}
        onContinueTelling={continueTelling}
        onRunStart={startWriting}
        onRunTarget={writingTarget}
        onStop={stopWriting}
        interruptedRun={interruptedRun}
        onDismissRun={dismissRun}
        onNextEvent={() => nextEvent(EVENT_LIMIT)}
        onUndo={strike}
        onDialog={setDialog}
        runOnOpen={firstRun}
        onRunOpened={() => setFirstRun(null)}
      />
      </Pending>
      {page === "book" ? <Pending reading={frame.reading}>
        <Book
          key={`${view.id}:${tellingVersion}`}
          worldId={view.id}
          chapter={bookChapter}
          onChapter={(chapter) => navigate({ kind: "world", id: view.id, page: "book", ...(chapter ? { chapter } : {}) }, "replace")}
          engine={frame.reading}
          author={engine.current.author()}
          decisionsFor={(telling) => engine.current!.read(telling).decisions()}
          notes={notebook}
          onReadNote={openNote}
          onEditTitlePage={async (nextTitle, nextAuthor) => {
            const world = engine.current;
            const saved = openedBook.current;
            if (!world || saved?.id !== view.id) return "This world is no longer open.";
            pendingWrites.current++;
            try {
              await world.setTitle(nextTitle);
              await world.setAuthor(nextAuthor);
              if (world !== engine.current || saved !== openedBook.current) return "This world is no longer open.";
              saved.keptTitle = undefined;
              setVersion((value) => value + 1);
              return persist() ? null : "The title page is kept in this open world, but browser storage failed. Export a save file before closing.";
            } catch (failure) { return message(failure); }
            finally { pendingWrites.current--; }
          }}
          onReadTelling={(telling) => void readTelling(telling)}
          onCompare={setCompareWith}
          catalog={catalog}
          version={version}
          generation={generation}
          overview={overview}
          title={title}
          map={worldMap}
          onBack={() => navigate({ kind: "world", id: view.id, page: "stage" })}
        />
      </Pending> : null}
      <Pending reading={frame.reading}>{dialogs}</Pending>
      {liftedComparison.value !== null ? <Pending reading={frame.reading}><TellingComparison open={compareWith !== null} engine={engine.current} overview={overview} map={worldMap} other={liftedComparison.value}
        onClose={() => setCompareWith(null)} onRead={readTelling} onContinue={(from) => { setCompareWith(null); void run({ kind: "run", generations: 1 }, from); }} /></Pending> : null}
    </div>
  );
}

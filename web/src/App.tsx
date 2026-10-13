import { useCallback, useEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { loadCatalog, loadEngine, message } from "./engine";
import type { Action, Catalog, Engine, HistoryPoint, NotebookNote, Overview, ReadEngine, SettlementChoice, SettlementPreview, WorldMap } from "./model";
import type { Focus } from "./components/Pedia";
import { YEARS } from "./model";
import { ActionDialog, type DialogKind } from "./components/ActionDialog";
import { Book } from "./components/Book";
import { Designer, type Founding as FoundingDesign } from "./components/Designer";
import { Modal } from "./components/Modal";
import { ChartRoom } from "./components/ChartRoom";
import { Stage } from "./components/Stage";
import { TellingComparison } from "./components/TellingComparison";
import type { MapMotionReading } from "./components/MapView";
import { Founding } from "./components/Founding";
import { Pending } from "./components/Pending";
import { sampleWorld } from "./sample";
import { download } from "./takeout";
import { useLiftedValue } from "./motion";
import {
  describe,
  loadShelf,
  newBookId,
  rawShelf,
  readBook,
  readPlace,
  removeBook,
  saveBook,
  savePlace,
  setLast,
  worldName,
  type Shelf as ShelfIndex,
} from "./shelf";

/// Longest wait for something to happen before giving up.
const EVENT_LIMIT = 400;

type View =
  | { kind: "loading" }
  | { kind: "shelf" }
  | { kind: "setup" }
  | { kind: "book"; id: string }
  | { kind: "recovery"; what: string; raw: string; error: string };

/// The workshop, or the book laid open over its chart.
type Page = "stage" | "book";

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
  const [shelf, setShelf] = useState<ShelfIndex>({ books: [], last: null });
  const mapMotion = useRef<MapMotionReading | null>(null);
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
  const [page, setPage] = useState<Page>("stage");
  const [bookChapter, setBookChapter] = useState<string | null>(null);
  /// Years a newly founded world runs to when the workshop first opens.
  const [firstRun, setFirstRun] = useState<number | null>(null);
  useEffect(() => {
    if (page === "book" && bookChapter) document.getElementById(`book-${bookChapter}`)?.scrollIntoView({ block: "start" });
  }, [page, bookChapter]);
  const [comparisonReturn, setComparisonReturn] = useState(0);
  const [sheetReturn, setSheetReturn] = useState(0);
  const liftedDialog = useLiftedValue(dialog, () => setSheetReturn((n) => n + 1));
  const liftedComparison = useLiftedValue(compareWith, () => setComparisonReturn((n) => n + 1));
  const [worldMap, setWorldMap] = useState<WorldMap | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const bookId = view.kind === "book" ? view.id : null;

  // The present telling at its latest year, which the shelf describes. Kept
  // whenever it is opened, so saving never waits on the engine's thread.
  const present = useRef<Overview | null>(null);
  const persist = useCallback(() => {
    const world = engine.current;
    if (!world || bookId === null || !present.current) return true;
    try {
      const entry = describe(bookId, present.current, world.title(), world.author());
      setShelf(saveBook(shelf, entry, world.save()));
      setSaveError(null);
      return true;
    } catch (e) {
      setSaveError(`Not saved in this browser: ${message(e)}. The book's save file still works.`);
      return false;
    }
  }, [bookId, shelf]);

  useEffect(() => {
    // Playback saves when it stops; an accepted update also saves its latest
    // generation. A full or unavailable store must not cause a lossy reload.
    const beforeUpdate = (event: Event) => {
      if (!persist()) event.preventDefault();
    };
    window.addEventListener("umran:before-update", beforeUpdate);
    return () => window.removeEventListener("umran:before-update", beforeUpdate);
  }, [persist]);

  // Saving needs the book open first, so new books save on the next render.
  const [unsaved, setUnsaved] = useState(false);
  useEffect(() => {
    if (unsaved && bookId !== null) {
      persist();
      setUnsaved(false);
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
    flushSync(() => { setFrame(next); then?.(next); });
    return next;
  }, []);

  const adopt = useCallback(async (id: string, next: Engine, fresh: boolean) => {
    const [map, tip] = await Promise.all([next.map(), next.tip().overview(next.latest())]);
    // The world on screen stays readable until the new one is committed.
    const previous = engine.current;
    engine.current = next;
    present.current = tip;
    mapMotion.current = null;
    // Reopen where the reader left off, if that reading still exists; a place
    // from another engine revision or a removed telling opens the latest year.
    const place = fresh ? null : readPlace(id);
    const opened = place ? await show({ telling: place.telling, point: place.point, viewing: place.viewing }).catch(() => null) : null;
    if (!opened) await show(PRESENT);
    if (previous !== next) previous?.dispose();
    setNotebook(next.notebook());
    setWorldMap(map);
    setCompareWith(null);
    setInitialFocus(opened && place ? place.focus : null);
    setFocus(opened && place ? place.focus : { kind: "world" });
    setTellingVersion((v) => v + 1);
    setCommunity(0);
    setDialog(null);
    setPage("stage");
    setError(null);
    setInfo(null);
    setVersion((v) => v + 1);
    setView({ kind: "book", id });
    if (fresh) setUnsaved(true);
  }, [show]);

  const [focus, setFocus] = useState<Focus>({ kind: "world" });
  useEffect(() => {
    if (view.kind === "book" && frame) savePlace(view.id, { telling: frame.telling, point: frame.point, viewing: frame.viewing, focus });
  }, [view, frame, focus]);

  const openBook = useCallback(
    (id: string) => {
      const raw = readBook(id);
      if (raw === null) {
        setError("That world could not be found in this browser.");
        setView({ kind: "shelf" });
        return;
      }
      loadEngine(raw).then((next) => adopt(id, next, false)).catch(
        (e) => {
          // Open the shelf next time rather than this book again.
          setShelf((current) => setLast(current, null));
          setView({ kind: "recovery", what: "This world", raw, error: message(e) });
        },
      );
    },
    [adopt],
  );

  useEffect(() => {
    loadCatalog().then(setCatalog, (e) => setError(message(e)));
    let loaded: ShelfIndex;
    try {
      loaded = loadShelf();
    } catch (e) {
      setView({ kind: "recovery", what: "The shelf", raw: rawShelf() ?? "", error: message(e) });
      return;
    }
    setShelf(loaded);
    if (loaded.last !== null && loaded.books.some((b) => b.id === loaded.last)) openBook(loaded.last);
    else setView({ kind: "shelf" });
  }, [openBook]);

  // Record the destination after the stage's playback cleanup has saved.
  // Opening an unchanged world must also make it the one resumed next time.
  useEffect(() => {
    if (view.kind === "loading" || view.kind === "recovery") return;
    const last = view.kind === "book" ? view.id : null;
    setShelf((current) => current.last === last ? current : setLast(current, last));
  }, [view]);

  const toShelf = useCallback(() => {
    setError(null);
    setView({ kind: "shelf" });
  }, []);

  const overview = frame?.overview ?? null;
  const latest = frame?.latest ?? 0;
  const generation = overview?.generation ?? 0;
  const selected = overview ? Math.min(community, overview.communities.length - 1) : 0;
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
      try {
        await current.actAt({ telling: from.telling, point: from.point }, from.mutation, action);
        await changed(from);
        persist();
      } catch (e) {
        setError(message(e));
      }
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
    try {
      await engine.current?.rename(telling, name);
      if (frame) await show(frame, () => setVersion((v) => v + 1));
      persist();
    } catch (e) { setError(message(e)); }
  };

  const saveNote = async (note: NotebookNote): Promise<string | null> => {
    try {
      await engine.current?.saveNote(note);
      setNotebook(engine.current?.notebook() ?? []);
      return persist() ? null : "The entry is kept in this open world, but browser storage failed. Export a save file before closing.";
    } catch (e) { return message(e); }
  };
  const removeNote = async (id: string): Promise<string | null> => {
    try {
      await engine.current?.removeNote(id);
      setNotebook(engine.current?.notebook() ?? []);
      return persist() ? null : "The note was removed in this open world, but browser storage failed. Export a save file before closing.";
    } catch (e) { return message(e); }
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

  // One frame's run at the selected point; saving waits until it stops.
  const runYears = useCallback(async (generations: number): Promise<boolean> => {
    const current = engine.current;
    if (!current || !overview) return false;
    try {
      await current.actAt({ telling: overview.telling, point: overview.point }, overview.mutation, { kind: "run", generations });
      return await changed(overview) !== null;
    } catch (e) {
      setError(message(e));
      return false;
    }
  }, [overview, changed]);
  const tick = useCallback(() => runYears(1), [runYears]);

  const nextEvent = useCallback(
    async (limit: number): Promise<boolean> => {
      const current = engine.current;
      if (!current || !overview) return false;
      try {
        const ran = await current.untilAt({ telling: overview.telling, point: overview.point }, overview.mutation, limit);
        return await changed(overview, () => setInfo(ran >= limit ? `${limit * YEARS} years passed, and nothing of note befell.` : null)) !== null;
      } catch (e) { setError(message(e)); return false; }
    },
    [overview, changed],
  );

  const begin = useCallback(async (next: Engine, title: string | null, author: string | null, years: number) => {
    await next.setTitle(title ?? "");
    await next.setAuthor(author ?? "");
    try { localStorage.setItem("umran.author", author ?? ""); }
    catch { /* The author is still kept in the world save. */ }
    await adopt(newBookId(), next, true);
    setFirstRun(years);
  }, [adopt]);

  const sample = async () => {
    try {
      await adopt(newBookId(), await sampleWorld(), true);
    } catch (e) {
      setError(`The sample could not be written: ${message(e)}`);
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
          books={shelf.books}
          onOpen={openBook}
          onBegin={() => setView({ kind: "setup" })}
          onSample={sample}
          onImport={(f) => void importFile(f)}
          onFailure={(failure) => setView({ kind: "recovery", what: "The shelf", raw: rawShelf() ?? "", error: message(failure) })}
          onRemove={(id) => {
            try {
              setShelf((current) => removeBook(current, id));
            } catch (e) {
              setError(`Could not remove it: ${message(e)}`);
            }
          }}
        />
        {error ? <p className="notice error">{error}</p> : null}
      </>
    );
  }

  const title = engine.current?.title() ?? worldName(overview) ?? "A new world";

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
      {info ? (
        <p className="notice" role="status">
          {info} <button type="button" className="link" onClick={() => setInfo(null)}>Dismiss</button>
        </p>
      ) : null}
    </>
  );

  const strike = async () => {
    if (!engine.current) return;
    try {
      const previous = await engine.current.previous({ telling: overview.telling, point: overview.point });
      await show({ telling: previous.telling, point: previous.point, viewing: frame.viewing }, () => {
        setInitialFocus(null); setTellingVersion((v) => v + 1);
      });
    } catch (e) { setError(message(e)); }
  };

  const settle = async (choice: SettlementChoice, preview: SettlementPreview) => {
    const current = engine.current;
    if (!current) return;
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
        engine={frame.reading}
        catalog={catalog}
        map={worldMap}
        version={version}
        generation={generation}
        overview={overview}
        title={title}
        notices={notices}
        sheet={page === "book" ? (
          <Pending reading={frame.reading}>
          <Book
            engine={frame.reading}
            author={engine.current.author()}
            decisionsFor={(telling) => engine.current!.read(telling).decisions()}
            notes={notebook}
            onReadNote={openNote}
            catalog={catalog}
            version={version}
            generation={generation}
            overview={overview}
            title={title}
            map={worldMap}
            onBack={() => setPage("stage")}
          />
          </Pending>
        ) : null}
        initialFocus={initialFocus}
        onFocus={setFocus}
        onSettle={settle}
        comparisonReturn={comparisonReturn}
        sheetReturn={sheetReturn}
        mapMotion={mapMotion}
        notes={notebook}
        onSaveNote={saveNote}
        onRemoveNote={removeNote}
        onReadNote={openNote}
        onReadPoint={(point) => void show({ telling: overview.telling, point, viewing: null })}
        canUndo={overview.point.offset > 0 || overview.point.action > 1}
        selected={selected}
        onSelect={setCommunity}
        onShelf={toShelf}
        onBook={(chapter) => { setBookChapter(chapter ?? null); setPage("book"); }}
        onRestore={readTelling}
        onRenameTelling={renameTelling}
        onCompare={setCompareWith}
        onScrub={scrub}
        onTick={tick}
        onRun={runYears}
        onRunStart={() => setDialog(null)}
        onStop={persist}
        onNextEvent={() => nextEvent(EVENT_LIMIT)}
        onUndo={strike}
        onDialog={setDialog}
        runOnOpen={firstRun}
        onRunOpened={() => setFirstRun(null)}
      />
      </Pending>
      <Pending reading={frame.reading}>{dialogs}</Pending>
      {liftedComparison.value !== null ? <Pending reading={frame.reading}><TellingComparison open={compareWith !== null} engine={engine.current} overview={overview} map={worldMap} other={liftedComparison.value}
        onClose={() => setCompareWith(null)} onRead={readTelling} onContinue={(from) => { setCompareWith(null); void run({ kind: "run", generations: 1 }, from); }} /></Pending> : null}
    </div>
  );
}

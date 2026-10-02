import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { loadCatalog, loadEngine, message } from "./engine";
import type { Action, Catalog, Engine, HistoryPoint, NotebookNote, Overview, SettlementChoice, SettlementPreview, WorldMap } from "./model";
import type { Focus } from "./components/Pedia";
import { YEARS } from "./model";
import { ActionDialog, type DialogKind } from "./components/ActionDialog";
import { Appendix } from "./components/Appendix";
import { Designer, type Founding } from "./components/Designer";
import { Modal } from "./components/Modal";
import { Shelf } from "./components/Shelf";
import { Stage } from "./components/Stage";
import { TellingComparison } from "./components/TellingComparison";
import { Notebook, makeNote } from "./components/Notebook";
import type { MapMotionReading } from "./components/MapView";
import { WorldSetup } from "./components/WorldSetup";
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

/// The stage, or the page for taking things out of the world.
type Page = "stage" | "export";

function foundingAction(f: Founding): Action {
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
  const [viewTelling, setViewTelling] = useState<number | null>(null);
  const [viewPoint, setViewPoint] = useState<HistoryPoint | null>(null);
  const [compareWith, setCompareWith] = useState<number | null>(null);
  const [notebook, setNotebook] = useState<NotebookNote[]>([]);
  const [notebookDraft, setNotebookDraft] = useState<NotebookNote | null | undefined>(undefined);
  const [viewing, setViewing] = useState<number | null>(null); // null = latest
  const [community, setCommunity] = useState(0);
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [page, setPage] = useState<Page>("stage");
  const [comparisonReturn, setComparisonReturn] = useState(0);
  const [sheetReturn, setSheetReturn] = useState(0);
  const liftedDialog = useLiftedValue(dialog, () => setSheetReturn((n) => n + 1));
  const notebookPage = useMemo(() => notebookDraft === undefined ? null : { initial: notebookDraft }, [notebookDraft]);
  const liftedNotebook = useLiftedValue(notebookPage, () => setSheetReturn((n) => n + 1));
  const liftedComparison = useLiftedValue(compareWith, () => setComparisonReturn((n) => n + 1));
  const [worldMap, setWorldMap] = useState<WorldMap | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const bookId = view.kind === "book" ? view.id : null;

  const persist = useCallback(() => {
    if (!engine.current || bookId === null) return true;
    try {
      const entry = describe(bookId, engine.current.overview(engine.current.latest()));
      setShelf(saveBook(shelf, entry, engine.current.save()));
      setSaveError(null);
      return true;
    } catch (e) {
      setSaveError(`Not saved in this browser: ${message(e)}. Export still works.`);
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

  const adopt = useCallback((id: string, next: Engine, fresh: boolean) => {
    engine.current?.dispose();
    engine.current = next;
    mapMotion.current = null;
    setNotebook(next.notebook()); setNotebookDraft(undefined);
    setWorldMap(next.map());
    setViewTelling(null); setViewPoint(null); setCompareWith(null);
    setInitialFocus(null); setFocus({ kind: "world" });
    setViewing(null);
    // Reopen where the reader left off, if that reading still exists.
    const place = fresh ? null : readPlace(id);
    if (place) {
      try {
        const reading = next.read(place.telling ?? next.overview(next.latest()).telling, place.point);
        reading.overview(place.viewing === null ? reading.latest() : Math.min(place.viewing, reading.latest()));
        setViewTelling(place.telling); setViewPoint(place.point); setViewing(place.viewing);
        setInitialFocus(place.focus); setFocus(place.focus);
      } catch {
        // A place from another engine revision or a removed telling: start at the latest year.
      }
    }
    setTellingVersion((v) => v + 1);
    setCommunity(0);
    setDialog(null);
    setPage("stage");
    setError(null);
    setInfo(null);
    setVersion((v) => v + 1);
    setView({ kind: "book", id });
    if (fresh) setUnsaved(true);
  }, []);

  const [focus, setFocus] = useState<Focus>({ kind: "world" });
  useEffect(() => {
    if (view.kind === "book") savePlace(view.id, { telling: viewTelling, point: viewPoint, viewing, focus });
  }, [view, viewTelling, viewPoint, viewing, focus]);

  const openBook = useCallback(
    (id: string) => {
      const raw = readBook(id);
      if (raw === null) {
        setError("That world could not be found in this browser.");
        setView({ kind: "shelf" });
        return;
      }
      loadEngine(raw).then(
        (next) => adopt(id, next, false),
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

  const readingEngine = useMemo(() => {
    const current = engine.current;
    if (!current || view.kind !== "book") return null;
    const telling = viewTelling ?? current.overview(current.latest()).telling;
    return current.read(telling, viewPoint);
  }, [bookId, view.kind, viewTelling, viewPoint, version]);
  const latest = readingEngine?.latest() ?? 0;
  const requestedGeneration = viewing === null ? latest : Math.min(viewing, latest);
  const overview = useMemo(() => readingEngine?.overview(requestedGeneration) ?? null, [readingEngine, requestedGeneration]);
  const generation = overview?.generation ?? requestedGeneration;
  const selected = overview ? Math.min(community, overview.communities.length - 1) : 0;
  const scrub = useCallback((g: number): HistoryPoint | null => {
    if (!engine.current || !overview) return null;
    try {
      const point = engine.current.read(overview.telling).overview(g).point;
      setViewPoint(null); setViewing(g >= latest ? null : g);
      return point;
    } catch (e) { setError(message(e)); return null; }
  }, [latest, overview]);

  const run = useCallback(
    (action: Action, from: Overview | null = overview) => {
      const current = engine.current;
      if (!current) return;
      try {
        if (!from) return;
        current.actAt({ telling: from.telling, point: from.point }, from.mutation, action);
        if (!from.atTip || current.overview(current.latest()).telling !== overview?.telling) {
          setInitialFocus(null); setCommunity(0); setTellingVersion((v) => v + 1);
        }
        setViewTelling(null); setViewPoint(null);
        setViewing(null);
        setVersion((v) => v + 1);
        setError(null);
        persist();
      } catch (e) {
        setError(message(e));
      }
    },
    [overview, persist],
  );

  // Acting while viewing the past begins another telling from there; the
  // later years are set aside, not lost.
  const perform = run;

  const readTelling = useCallback((telling: number, at?: number, focus?: Focus) => {
    try {
      // Validate a saved telling before switching; an unreadable alternate
      // stays listed and keeps its original recipe for recovery/export.
      const reading = engine.current?.read(telling);
      if (!reading) return;
      const last = reading.latest();
      setViewTelling(telling); setViewPoint(null);
      setInitialFocus(focus ?? null); setTellingVersion((v) => v + 1);
      setViewing(at === undefined || at >= last ? null : at);
      setCompareWith(null); setError(null);
    } catch (e) { setError(`This telling could not be read: ${message(e)}. Its saved account is still kept.`); }
  }, []);
  const renameTelling = (telling: number, name: string) => {
    try { engine.current?.rename(telling, name); setVersion((v) => v + 1); persist(); }
    catch (e) { setError(message(e)); }
  };

  const saveNote = (note: NotebookNote): string | null => {
    try {
      engine.current?.saveNote(note);
      setNotebook(engine.current?.notebook() ?? []);
      return persist() ? null : "The entry is kept in this open world, but browser storage failed. Export a save file before closing.";
    } catch (e) { return message(e); }
  };
  const openNote = (note: NotebookNote): string | null => {
    try {
      const destination = engine.current?.resolveNote(note.id);
      if (!destination) return "This entry has no fixed reading.";
      setViewTelling(destination.reading.telling); setViewPoint(destination.reading.point);
      setInitialFocus(destination.subject); setTellingVersion((v) => v + 1);
      setViewing(null); setNotebookDraft(undefined); setError(null);
      return null;
    } catch (e) { return message(e); }
  };

  // One generation at the present, for play; saving waits until play stops.
  const tick = useCallback((): boolean => {
    const current = engine.current;
    if (!current) return false;
    try {
      if (!overview) return false;
      current.actAt({ telling: overview.telling, point: overview.point }, overview.mutation, { kind: "run", generations: 1 });
      if (!overview.atTip) { setInitialFocus(null); setTellingVersion((v) => v + 1); }
      setViewTelling(null); setViewPoint(null); setViewing(null);
      setVersion((v) => v + 1);
      return true;
    } catch (e) {
      setError(message(e));
      return false;
    }
  }, [overview]);

  const nextEvent = useCallback(
    (limit: number) => {
      const current = engine.current;
      if (!current) return;
      if (!overview) return;
      try {
        const ran = current.untilAt({ telling: overview.telling, point: overview.point }, overview.mutation, limit);
        if (!overview.atTip) { setInitialFocus(null); setTellingVersion((v) => v + 1); }
        setViewTelling(null); setViewPoint(null); setViewing(null);
        setVersion((v) => v + 1); persist();
        setInfo(ran >= limit ? `${limit * YEARS} years passed, and nothing of note befell.` : null);
      } catch (e) { setError(message(e)); }
    },
    [overview, persist],
  );

  const begin = useCallback((next: Engine) => adopt(newBookId(), next, true), [adopt]);

  const sample = async () => {
    try {
      adopt(newBookId(), await sampleWorld(), true);
    } catch (e) {
      setError(`The sample could not be written: ${message(e)}`);
    }
  };

  const importFile = async (file: File) => {
    try {
      adopt(newBookId(), await loadEngine(await file.text()), true);
    } catch (e) {
      setError(`Could not bring in ${file.name}: ${message(e)}`);
    }
  };

  if (view.kind === "loading" || !catalog) {
    return <main className="splash">{error ?? "Loading the language engine…"}</main>;
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
            Go to the shelf
          </button>
        </div>
      </main>
    );
  }

  if (view.kind === "setup") {
    return (
      <div className="app">
        <WorldSetup
          catalog={catalog}
          onBegin={begin}
          onShelf={toShelf}
          onSample={sample}
        />
      </div>
    );
  }

  if (view.kind === "shelf" || !overview || !engine.current || !readingEngine) {
    return (
      <>
        <Shelf
          revision={catalog.revision}
          books={shelf.books}
          onOpen={openBook}
          onBegin={() => setView({ kind: "setup" })}
          onSample={() => void sample()}
          onImport={(f) => void importFile(f)}
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

  const title = shelf.books.find((b) => b.id === view.id)?.title ?? "A new world";

  const notices = (
    <>
      {overview.savedRevision !== null ? (
        <p className="notice">
          This history was saved with engine revision {overview.savedRevision}; this is revision{" "}
          {overview.revision}, so its words may differ from when it was saved.
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

  const strike = () => {
    if (!engine.current) return;
    const previous = engine.current.previous({ telling: overview.telling, point: overview.point });
    setViewTelling(previous.telling); setViewPoint(previous.point);
    setInitialFocus(null); setTellingVersion((v) => v + 1);
  };

  const settle = (choice: SettlementChoice, preview: SettlementPreview) => {
    const current = engine.current;
    if (!current) return;
    try {
      current.actAt({ telling: preview.telling, point: preview.point }, preview.mutation, { kind: "settle", ...choice });
      const after = current.overview(current.latest());
      const event = after.annals.findLast((a) => a.kind === "settlement");
      setInitialFocus(event ? { kind: "event", id: event.id } : { kind: "people", id: choice.community });
      setCommunity(event?.settlement?.daughter ?? choice.community);
      setViewTelling(null); setViewPoint(null); setViewing(null);
      setTellingVersion((v) => v + 1);
      setVersion((v) => v + 1);
      setError(null);
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
        selected={selected}
        onClose={() => setDialog(null)}
        onAction={(action) => {
          setDialog(null);
          perform(action);
        }}
      />
    ) : null;

  if (!worldMap) {
    return (
      <div className="app">
        {notices}
        <Appendix
          engine={readingEngine}
          notebook={notebook}
          catalog={catalog}
          version={version}
          generation={generation}
          overview={overview}
          title={title}
          variety={overview.communities[selected].variety}
          onBack={() => setPage("stage")}
        />
        {dialogs}
      </div>
    );
  }

  return (
    <div className="app">
      <Stage
        key={`${view.id}:${tellingVersion}`}
        engine={readingEngine}
        catalog={catalog}
        map={worldMap}
        version={version}
        generation={generation}
        overview={overview}
        title={title}
        notices={notices}
        sheet={page === "export" ? (
          <Appendix
            engine={readingEngine}
            notebook={notebook}
            catalog={catalog}
            version={version}
            generation={generation}
            overview={overview}
            title={title}
            variety={overview.communities[selected].variety}
            onBack={() => setPage("stage")}
          />
        ) : null}
        initialFocus={initialFocus}
        onFocus={setFocus}
        onSettle={settle}
        comparisonReturn={comparisonReturn}
        sheetReturn={sheetReturn}
        mapMotion={mapMotion}
        onNotebook={() => setNotebookDraft(null)}
        onKeep={(subject, label) => setNotebookDraft(makeNote(overview, subject, label))}
        onReadPoint={(point) => { setViewTelling(overview.telling); setViewPoint(point); setViewing(null); }}
        canUndo={overview.point.offset > 0 || overview.point.action > 1}
        selected={selected}
        onSelect={setCommunity}
        onShelf={toShelf}
        onExport={() => setPage("export")}
        onRestore={readTelling}
        onRenameTelling={renameTelling}
        onCompare={setCompareWith}
        onScrub={scrub}
        onTick={tick}
        onStop={persist}
        onNextEvent={() => nextEvent(EVENT_LIMIT)}
        onUndo={strike}
        onDialog={setDialog}
      />
      {dialogs}
      {liftedNotebook.value ? <Notebook open={notebookDraft !== undefined} notes={notebook} overview={overview} initial={liftedNotebook.value.initial}
        onClose={() => setNotebookDraft(undefined)} onSave={saveNote} onRead={openNote} /> : null}
      {liftedComparison.value !== null ? <TellingComparison open={compareWith !== null} engine={engine.current} overview={overview} map={worldMap} other={liftedComparison.value}
        onClose={() => setCompareWith(null)} onRead={readTelling} onContinue={(from) => { setCompareWith(null); run({ kind: "run", generations: 1 }, from); }} /> : null}
    </div>
  );
}

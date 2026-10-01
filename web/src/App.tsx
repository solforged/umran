import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { loadCatalog, loadEngine, message } from "./engine";
import type { Action, Catalog, Engine, WorldMap } from "./model";
import { YEARS } from "./model";
import { ActionDialog, type DialogKind } from "./components/ActionDialog";
import { Appendix } from "./components/Appendix";
import { Designer, type Founding } from "./components/Designer";
import { Modal } from "./components/Modal";
import { Shelf } from "./components/Shelf";
import { Stage } from "./components/Stage";
import { WorldSetup } from "./components/WorldSetup";
import { sampleWorld } from "./sample";
import { download } from "./takeout";
import {
  describe,
  loadShelf,
  newBookId,
  rawShelf,
  readBook,
  removeBook,
  saveBook,
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
  const engine = useRef<Engine | null>(null);
  const [version, setVersion] = useState(0);
  const [viewing, setViewing] = useState<number | null>(null); // null = latest
  const [community, setCommunity] = useState(0);
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [page, setPage] = useState<Page>("stage");
  const [worldMap, setWorldMap] = useState<WorldMap | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const bookId = view.kind === "book" ? view.id : null;

  const persist = useCallback(() => {
    if (!engine.current || bookId === null) return;
    try {
      const entry = describe(bookId, engine.current.overview(engine.current.latest()));
      setShelf((current) => saveBook(current, entry, engine.current!.save()));
      setSaveError(null);
    } catch (e) {
      setSaveError(`Not saved in this browser: ${message(e)}. Export still works.`);
    }
  }, [bookId]);

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
    setWorldMap(next.map());
    setViewing(null);
    setCommunity(0);
    setDialog(null);
    setPage("stage");
    setError(null);
    setInfo(null);
    setVersion((v) => v + 1);
    setView({ kind: "book", id });
    if (fresh) setUnsaved(true);
  }, []);

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
    else setView({ kind: loaded.books.length === 0 ? "setup" : "shelf" });
  }, [openBook]);

  const toShelf = useCallback(() => {
    setShelf((current) => setLast(current, null));
    setError(null);
    setView({ kind: "shelf" });
  }, []);

  const latest = engine.current?.latest() ?? 0;
  const generation = viewing === null ? latest : Math.min(viewing, latest);
  const overview = useMemo(
    () => (view.kind === "book" && engine.current ? engine.current.overview(generation) : null),
    // `version` changes whenever the history does.
    [view.kind, generation, version],
  );
  const selected = overview ? Math.min(community, overview.communities.length - 1) : 0;
  const scrub = useCallback((g: number) => setViewing(g >= (engine.current?.latest() ?? 0) ? null : g), []);

  const run = useCallback(
    (action: Action) => {
      const current = engine.current;
      if (!current) return;
      try {
        if (generation < current.latest()) current.branch(generation);
        current.act(action);
        setViewing(null);
        setVersion((v) => v + 1);
        setError(null);
        persist();
      } catch (e) {
        setError(message(e));
      }
    },
    [generation, persist],
  );

  // Acting while viewing the past begins another telling from there; the
  // later years are set aside, not lost.
  const perform = run;

  const restore = useCallback(
    (telling: number) => {
      try {
        engine.current?.restore(telling);
        setViewing(null);
        setVersion((v) => v + 1);
        setError(null);
        persist();
      } catch (e) {
        setError(message(e));
      }
    },
    [persist],
  );

  // One generation at the present, for play; saving waits until play stops.
  const tick = useCallback((): boolean => {
    const current = engine.current;
    if (!current) return false;
    try {
      current.act({ kind: "run", generations: 1 });
      setViewing(null);
      setVersion((v) => v + 1);
      return true;
    } catch (e) {
      setError(message(e));
      return false;
    }
  }, []);

  const nextEvent = useCallback(
    (limit: number) => {
      const current = engine.current;
      if (!current) return;
      const ran = current.runUntilEvent(limit);
      setViewing(null);
      setVersion((v) => v + 1);
      persist();
      setInfo(ran >= limit ? `${limit * YEARS} years passed, and nothing of note befell.` : null);
    },
    [persist],
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
          <button type="button" onClick={() => setView({ kind: "shelf" })}>
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
          onShelf={shelf.books.length > 0 ? () => setView({ kind: "shelf" }) : undefined}
          onSample={sample}
        />
      </div>
    );
  }

  if (view.kind === "shelf" || !overview || !engine.current) {
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
    engine.current?.undo();
    setViewing(null);
    setVersion((v) => v + 1);
    persist();
  };

  const dialogs =
    dialog === "found" ? (
      <Modal open wide title="A new people arrives" onClose={() => setDialog(null)}>
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
    ) : dialog ? (
      <ActionDialog
        kind={dialog}
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

  if (page === "export" || !worldMap) {
    return (
      <div className="app">
        {notices}
        <Appendix
          engine={engine.current}
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
        engine={engine.current}
        catalog={catalog}
        map={worldMap}
        version={version}
        generation={generation}
        overview={overview}
        title={title}
        notices={notices}
        canUndo={overview.timeline.length > 1}
        selected={selected}
        onSelect={setCommunity}
        onShelf={toShelf}
        onExport={() => setPage("export")}
        onRestore={restore}
        onScrub={scrub}
        onTick={tick}
        onStop={persist}
        onNextEvent={() => nextEvent(EVENT_LIMIT)}
        onUndo={strike}
        onDialog={setDialog}
      />
      {dialogs}
    </div>
  );
}

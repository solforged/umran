import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createEngine, loadCatalog, loadEngine, message } from "./engine";
import type { Action, Catalog, Engine } from "./model";
import { YEARS } from "./model";
import { ActionDialog, type DialogKind } from "./components/ActionDialog";
import { Communities } from "./components/Communities";
import { Inspector } from "./components/Inspector";
import { Lexicon } from "./components/Lexicon";
import { Modal } from "./components/Modal";
import { RunControls } from "./components/RunControls";
import { Timeline } from "./components/Timeline";

// The previous workbench used `langgen.workbench.v2`; that data is left
// untouched rather than silently deleted.
const STORAGE_KEY = "langgen.sim.v1";

type Boot =
  | { status: "loading" }
  | { status: "fresh" }
  | { status: "ready" }
  | { status: "recovery"; raw: string; error: string };

function readSaved(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function download(name: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  URL.revokeObjectURL(url);
}

export default function App() {
  const [boot, setBoot] = useState<Boot>({ status: "loading" });
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const engine = useRef<Engine | null>(null);
  const [version, setVersion] = useState(0);
  const [viewing, setViewing] = useState<number | null>(null); // null = latest
  const [community, setCommunity] = useState(0);
  const [concept, setConcept] = useState<string | null>(null);
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [pending, setPending] = useState<Action | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const importRef = useRef<HTMLInputElement>(null);

  const persist = useCallback(() => {
    if (!engine.current) return;
    try {
      localStorage.setItem(STORAGE_KEY, engine.current.save());
      setSaveError(null);
    } catch (e) {
      setSaveError(`Not saved in this browser: ${message(e)}. Export still works.`);
    }
  }, []);

  const adopt = useCallback((next: Engine) => {
    engine.current?.dispose();
    engine.current = next;
    setViewing(null);
    setCommunity(0);
    setConcept(null);
    setVersion((v) => v + 1);
    setBoot({ status: "ready" });
  }, []);

  useEffect(() => {
    loadCatalog().then(setCatalog, (e) => setError(message(e)));
    const saved = readSaved();
    if (saved === null) {
      setBoot({ status: "fresh" });
      return;
    }
    loadEngine(saved).then(adopt, (e) => setBoot({ status: "recovery", raw: saved, error: message(e) }));
  }, [adopt]);

  const latest = engine.current?.latest() ?? 0;
  const generation = viewing === null ? latest : Math.min(viewing, latest);
  const overview = useMemo(
    () => (boot.status === "ready" && engine.current ? engine.current.overview(generation) : null),
    // `version` changes whenever the history does.
    [boot.status, generation, version],
  );
  const selected = overview?.communities[Math.min(community, (overview?.communities.length ?? 1) - 1)];
  const variety = selected ? overview?.varieties[selected.variety] : undefined;

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

  // Acting while viewing the past starts a new branch from there.
  const perform = useCallback(
    (action: Action) => {
      if (engine.current && generation < engine.current.latest()) setPending(action);
      else run(action);
    },
    [generation, run],
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
      const before = current.latest();
      const ran = current.runUntilEvent(limit);
      setViewing(null);
      setVersion((v) => v + 1);
      persist();
      setInfo(
        ran >= limit
          ? `Nothing happened in ${limit} generations.`
          : `Something happened after ${ran} generation${ran === 1 ? "" : "s"} (generation ${before + ran}).`,
      );
    },
    [persist],
  );

  const startWorld = useCallback(
    async (seed: number, founding: Action) => {
      try {
        const next = await createEngine(seed);
        next.act(founding);
        adopt(next);
        persist();
        setDialog(null);
      } catch (e) {
        setError(message(e));
      }
    },
    [adopt, persist],
  );

  const importFile = async (file: File) => {
    try {
      adopt(await loadEngine(await file.text()));
      persist();
    } catch (e) {
      setError(`Could not import ${file.name}: ${message(e)}`);
    }
  };

  if (boot.status === "loading" || !catalog) {
    return <main className="splash">{error ?? "Loading the language engine…"}</main>;
  }

  if (boot.status === "recovery") {
    return (
      <main className="splash">
        <h1>Saved work could not be opened</h1>
        <p>{boot.error}</p>
        <p>It has not been changed or deleted. Download it before starting over.</p>
        <div className="row">
          <button type="button" onClick={() => download("langgen-unreadable-save.json", boot.raw)}>
            Download saved data
          </button>
          <button type="button" onClick={() => setBoot({ status: "fresh" })}>
            Start a new world
          </button>
        </div>
      </main>
    );
  }

  if (boot.status === "fresh" || !overview || !selected || !variety) {
    return (
      <main className="splash">
        <h1>Langgen</h1>
        <p>Start from a seed and one founding community, then poke at its words and let history happen.</p>
        <ActionDialog
          kind="world"
          inline
          catalog={catalog}
          overview={null}
          selected={0}
          onClose={() => undefined}
          onAction={() => undefined}
          onWorld={startWorld}
        />
        {error ? <p className="error">{error}</p> : null}
      </main>
    );
  }

  return (
    <div className="app">
      <header className="top">
        <div className="brand">
          <strong>Langgen</strong>
          <span>seed {overview.seed}</span>
        </div>
        <div className="clock">
          Generation {generation}
          <span> · about {generation * YEARS} years</span>
        </div>
        <div className="row">
          <RunControls
            generation={generation}
            atPresent={generation === latest}
            onRun={(generations) => perform({ kind: "run", generations })}
            onTick={tick}
            onNextEvent={nextEvent}
            onStop={persist}
          />
          <button
            type="button"
            disabled={overview.timeline.length <= 1}
            onClick={() => {
              engine.current?.undo();
              setViewing(null);
              setVersion((v) => v + 1);
              persist();
            }}
          >
            Undo
          </button>
          <button type="button" onClick={() => setDialog("world")}>
            New world
          </button>
          <button type="button" onClick={() => importRef.current?.click()}>
            Import
          </button>
          <button
            type="button"
            onClick={() => engine.current && download(`langgen-${overview.seed}.json`, engine.current.save())}
          >
            Export
          </button>
          <input
            ref={importRef}
            type="file"
            accept="application/json,.json"
            hidden
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) void importFile(file);
              e.target.value = "";
            }}
          />
        </div>
      </header>

      <Timeline overview={overview} generation={generation} onScrub={(g) => setViewing(g >= latest ? null : g)} />

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

      <main className="panes">
        <Communities
          overview={overview}
          selected={selected.id}
          onSelect={(id) => setCommunity(id)}
          onDialog={setDialog}
        />
        <Lexicon
          engine={engine.current!}
          version={version}
          generation={generation}
          variety={variety}
          community={selected}
          concept={concept}
          onConcept={setConcept}
        />
        <Inspector
          engine={engine.current!}
          version={version}
          generation={generation}
          variety={variety}
          concept={concept}
          onScrub={(g) => setViewing(g >= latest ? null : g)}
          onOpenVariety={(v) => {
            const owner = overview.communities.find((c) => c.variety === v);
            if (owner) setCommunity(owner.id);
          }}
        />
      </main>

      {dialog ? (
        <ActionDialog
          kind={dialog}
          catalog={catalog}
          overview={overview}
          selected={selected.id}
          onClose={() => setDialog(null)}
          onAction={(action) => {
            setDialog(null);
            perform(action);
          }}
          onWorld={startWorld}
        />
      ) : null}

      <Modal
        open={pending !== null}
        title="Continue from here?"
        onClose={() => setPending(null)}
        footer={
          <>
            <button type="button" onClick={() => setPending(null)}>
              Cancel
            </button>
            <button
              type="button"
              className="primary"
              onClick={() => {
                const action = pending;
                setPending(null);
                if (action) run(action);
              }}
            >
              Discard later history
            </button>
          </>
        }
      >
        <p>
          You are viewing generation {generation} of {latest}. Acting here starts a new history from this point;
          everything after it is discarded. Export first to keep it.
        </p>
      </Modal>
    </div>
  );
}

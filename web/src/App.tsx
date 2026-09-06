import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { Download, GitFork, History, Plus, Search, Upload } from "lucide-react";
import { createEngine } from "./engine";
import { Inspector } from "./components/Inspector";
import { Modal } from "./components/Modal";
import type {
  Community,
  ContactDomain,
  Engine,
  FormationOption,
  HistoryEvent,
  Lexeme,
  NewHistory,
  Snapshot,
} from "./model";

export const STORAGE_KEY = "langgen.workbench.v2";

const AESTHETICS = ["elvish", "kuo-toa", "illithid", "neutral"] as const;
const AESTHETIC_LABELS: Record<(typeof AESTHETICS)[number], string> = {
  elvish: "Elvish",
  "kuo-toa": "Kuo-toa",
  illithid: "Illithid",
  neutral: "Neutral",
};
const DEFAULT_CONFIG: NewHistory = { seed: 42, aesthetic: "elvish", empty: false };

type Boot = "loading" | "ready" | "error" | "recover";
type Dialog = "none" | "history" | "event" | "scenario" | "import" | "derive";
type OriginFilter = "all" | "root" | "derived" | "loan";
type EventKind = "Found" | "Separate" | "Develop" | "Contact";

type EventDraft = {
  kind: EventKind;
  name: string;
  aesthetic: string;
  parent: number;
  community: number;
  donor: number;
  recipient: number;
  steps: number;
  count: number;
  domain: ContactDomain;
};

type Pending = { event: HistoryEvent; snapshot: Snapshot };

export default function App() {
  const engineRef = useRef<Engine | null>(null);
  const importInputRef = useRef<HTMLInputElement>(null);
  const searchId = useId();

  const [boot, setBoot] = useState<Boot>("loading");
  const [engine, setEngine] = useState<Engine | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [bootMessage, setBootMessage] = useState("");
  const [notice, setNotice] = useState("");
  const [persistError, setPersistError] = useState("");

  const [communityId, setCommunityId] = useState<number | null>(null);
  const [lexemeId, setLexemeId] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [originFilter, setOriginFilter] = useState<OriginFilter>("all");
  const [dialog, setDialog] = useState<Dialog>("none");
  const [options, setOptions] = useState<FormationOption[]>([]);

  const [draft, setDraft] = useState<EventDraft>(emptyDraft());
  const [pending, setPending] = useState<Pending | null>(null);
  const [eventError, setEventError] = useState("");

  const [scenario, setScenario] = useState<NewHistory>({ ...DEFAULT_CONFIG });
  const [scenarioError, setScenarioError] = useState("");
  const [importPending, setImportPending] = useState<{ name: string; text: string } | null>(null);
  const [importError, setImportError] = useState("");

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const stored = readStoredHistory();
      try {
        if (stored.kind === "malformed") {
          if (!cancelled) {
            setBoot("recover");
            setBootMessage(stored.message);
          }
          return;
        }
        const next =
          stored.kind === "value"
            ? await createEngine(DEFAULT_CONFIG, stored.raw)
            : await createEngine(DEFAULT_CONFIG);
        if (cancelled) {
          next.dispose();
          return;
        }
        adoptEngine(next, next.snapshot(), false);
        setBoot("ready");
      } catch (err) {
        if (cancelled) return;
        if (stored.kind === "value") {
          setBoot("recover");
          setBootMessage(asError(err));
        } else {
          setBoot("error");
          setBootMessage(asError(err));
        }
      }
    })();
    return () => {
      cancelled = true;
      engineRef.current?.dispose();
      engineRef.current = null;
    };
  }, []);

  const community = snapshot?.communities.find((item) => item.id === communityId) ?? snapshot?.communities[0] ?? null;
  const atLatest = snapshot != null && snapshot.checkpoint === snapshot.latest;

  const filtered = useMemo(() => {
    if (!community) return [];
    const query = search.trim().toLowerCase();
    return community.lexicon.filter((item) => {
      const origin = originKind(item);
      if (originFilter !== "all" && origin !== originFilter) return false;
      if (query.length === 0) return true;
      return (
        item.form.toLowerCase().includes(query) ||
        item.gloss.toLowerCase().includes(query) ||
        item.ipa.toLowerCase().includes(query)
      );
    });
  }, [community, search, originFilter]);

  const lexeme = filtered.find((item) => item.id === lexemeId) ?? filtered[0] ?? null;

  useEffect(() => {
    const nextCommunityId = community?.id ?? null;
    const nextLexemeId = lexeme?.id ?? null;
    if (nextCommunityId !== communityId) setCommunityId(nextCommunityId);
    if (nextLexemeId !== lexemeId) setLexemeId(nextLexemeId);
  }, [community, lexeme, communityId, lexemeId]);

  useEffect(() => {
    if (!engine || !snapshot || !community || !lexeme) {
      setOptions([]);
      return;
    }
    try {
      setOptions(engine.options(snapshot.checkpoint, community.id, lexeme.id));
    } catch (err) {
      setOptions([]);
      setNotice(asError(err));
    }
  }, [engine, snapshot, community, lexeme]);

  function adoptEngine(next: Engine, nextSnapshot: Snapshot, write: boolean) {
    if (engineRef.current && engineRef.current !== next) engineRef.current.dispose();
    engineRef.current = next;
    setEngine(next);
    applySnapshot(nextSnapshot);
    if (write) persist(next);
  }

  function applySnapshot(next: Snapshot) {
    setSnapshot(next);
    setNotice("");
  }

  function persist(target: Engine) {
    try {
      localStorage.setItem(STORAGE_KEY, target.save());
      setPersistError("");
    } catch (err) {
      setPersistError(asError(err));
    }
  }

  function closeDialog() {
    setDialog("none");
    setPending(null);
    setEventError("");
    setImportError("");
    setImportPending(null);
    setScenarioError("");
  }

  function browseCheckpoint(id: number) {
    if (!engine) return;
    try {
      applySnapshot(engine.snapshot(id));
    } catch (err) {
      setNotice(asError(err));
    }
  }

  function returnLatest() {
    if (!engine || !snapshot) return;
    browseCheckpoint(snapshot.latest);
  }

  function selectCommunity(id: number) {
    setSearch("");
    setOriginFilter("all");
    const next = snapshot?.communities.find((item) => item.id === id);
    setCommunityId(id);
    setLexemeId(next?.lexicon[0]?.id ?? null);
  }

  function openEvent(kind: EventKind) {
    if (!snapshot || !atLatest) return;
    setDraft(draftFrom(kind, snapshot, communityId));
    setPending(null);
    setEventError("");
    setDialog("event");
  }

  function previewEvent(event: HistoryEvent, nextDialog: Dialog = "event") {
    if (!engine) return;
    try {
      const projected = engine.preview(event);
      setPending({ event, snapshot: projected });
      setEventError("");
      setDialog(nextDialog);
    } catch (err) {
      setPending(null);
      setEventError(asError(err));
    }
  }

  function acceptPending() {
    if (!engine || !pending) return;
    try {
      const next = engine.commit(pending.event);
      persist(engine);
      applySnapshot(next);
      setSearch("");
      setOriginFilter("all");
      if ("Derive" in pending.event) {
        const derive = pending.event.Derive;
        const host = next.communities.find((item) => item.id === derive.community);
        const effectId = next.effects.find(
          (effect) => effect.community === derive.community && effect.lexeme !== "",
        )?.lexeme;
        const created =
          host?.lexicon.find(
            (item) => item.baseLexeme === derive.base && item.formation === derive.kind,
          ) ?? host?.lexicon.find((item) => item.id === effectId);
        setCommunityId(derive.community);
        setLexemeId(created?.id ?? effectId ?? null);
      } else if ("Found" in pending.event || "Separate" in pending.event) {
        const named = "Found" in pending.event ? pending.event.Found.name : pending.event.Separate.name;
        const born = next.communities.find((item) => item.name === named);
        if (born) {
          setCommunityId(born.id);
          setLexemeId(born.lexicon[0]?.id ?? null);
        }
      }
      closeDialog();
    } catch (err) {
      setEventError(asError(err));
    }
  }

  async function startScenario(config: NewHistory) {
    setBoot("loading");
    setBootMessage("");
    try {
      const next = await createEngine(config);
      adoptEngine(next, next.snapshot(), true);
      setScenario({ ...config });
      setBoot("ready");
      closeDialog();
    } catch (err) {
      setBoot(engineRef.current ? "ready" : "error");
      setBootMessage(asError(err));
      setScenarioError(asError(err));
      setNotice(asError(err));
    }
  }

  function exportHistory() {
    if (!engine || !snapshot) return;
    try {
      const text = engine.save();
      const blob = new Blob([text], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `langgen-history-${snapshot.seed}.json`;
      anchor.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setNotice(asError(err));
    }
  }

  function onImportFile(file: File) {
    const reader = new FileReader();
    reader.onload = () => {
      const text = typeof reader.result === "string" ? reader.result : "";
      try {
        JSON.parse(text);
        setImportPending({ name: file.name, text });
        setImportError("");
        setDialog("import");
      } catch {
        setImportPending(null);
        setImportError("That file is not valid JSON.");
        setDialog("import");
      }
    };
    reader.onerror = () => {
      setImportError("The file could not be read.");
      setDialog("import");
    };
    reader.readAsText(file);
  }

  function acceptImport() {
    if (!engine || !importPending) return;
    try {
      const next = engine.load(importPending.text);
      persist(engine);
      applySnapshot(next);
      closeDialog();
    } catch (err) {
      setImportError(asError(err));
    }
  }

  function onLexiconKey(event: KeyboardEvent<HTMLTableSectionElement>) {
    if (!filtered.length) return;
    const index = filtered.findIndex((item) => item.id === lexemeId);
    if (event.key === "ArrowDown") {
      event.preventDefault();
      const next = filtered[Math.min(filtered.length - 1, Math.max(0, index) + 1)];
      if (next) setLexemeId(next.id);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      const next = filtered[Math.max(0, (index === -1 ? 1 : index) - 1)];
      if (next) setLexemeId(next.id);
    }
  }

  if (boot === "loading") {
    return (
      <div className="page-state">
        <p className="brand-mark">Langgen</p>
        <p>Opening the workshop…</p>
      </div>
    );
  }

  if (boot === "error") {
    return (
      <div className="page-state" role="alert">
        <p className="brand-mark">Langgen</p>
        <p>{bootMessage || "The workshop could not start."}</p>
        <button type="button" className="btn btn-primary" onClick={() => void startScenario(DEFAULT_CONFIG)}>
          Try again
        </button>
      </div>
    );
  }

  if (boot === "recover") {
    return (
      <div className="page-state" role="alert">
        <p className="brand-mark">Langgen</p>
        <h1>Saved history could not be restored</h1>
        <p>{bootMessage}</p>
        <p className="muted">
          The copy in this browser is unchanged. Start a new workshop or import a JSON file to replace it.
        </p>
        <div className="page-actions">
          <button type="button" className="btn btn-primary" onClick={() => void startScenario(DEFAULT_CONFIG)}>
            Start coastal workshop
          </button>
          <button
            type="button"
            className="btn"
            onClick={() => void startScenario({ ...DEFAULT_CONFIG, empty: true })}
          >
            Start empty
          </button>
          <button type="button" className="btn" onClick={() => importInputRef.current?.click()}>
            Import JSON
          </button>
        </div>
        {importError ? <p className="error">{importError}</p> : null}
        <input
          ref={importInputRef}
          type="file"
          accept="application/json,.json"
          hidden
          onChange={(event) => {
            const file = event.target.files?.[0];
            event.target.value = "";
            if (!file) return;
            const reader = new FileReader();
            reader.onload = () => {
              const text = typeof reader.result === "string" ? reader.result : "";
              try {
                JSON.parse(text);
              } catch {
                setImportError("That file is not valid JSON. Stored history was not changed.");
                return;
              }
              if (!engineRef.current) {
                void (async () => {
                  try {
                    const next = await createEngine(DEFAULT_CONFIG, text);
                    adoptEngine(next, next.snapshot(), true);
                    setBoot("ready");
                  } catch (err) {
                    setImportError(asError(err));
                  }
                })();
                return;
              }
              try {
                const next = engineRef.current.load(text);
                persist(engineRef.current);
                applySnapshot(next);
                setBoot("ready");
              } catch (err) {
                setImportError(asError(err));
              }
            };
            reader.readAsText(file);
          }}
        />
      </div>
    );
  }

  if (!snapshot || !engine) {
    return (
      <div className="page-state" role="alert">
        <p>The workshop has no history yet.</p>
      </div>
    );
  }

  const latestSummary =
    snapshot.checkpoints.find((item) => item.id === snapshot.latest)?.summary ?? snapshot.summary;

  return (
    <div className="workbench">
      <header className="header">
        <div className="brand">
          <p className="brand-mark">Langgen</p>
          <p className="brand-sub">language workshop</p>
        </div>
        <label className="header-field">
          <span>Community</span>
          <select
            aria-label="Selected community"
            value={community?.id ?? ""}
            disabled={snapshot.communities.length === 0}
            onChange={(event) => selectCommunity(Number(event.target.value))}
          >
            {snapshot.communities.length === 0 ? <option value="">None</option> : null}
            {snapshot.communities.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </select>
        </label>
        <button
          type="button"
          className="chip"
          onClick={() => setDialog("history")}
          aria-haspopup="dialog"
        >
          <History size={16} strokeWidth={1.75} aria-hidden />
          Checkpoint {snapshot.checkpoint} · {shortEvent(snapshot.event)}
        </button>
        <div className="header-actions">
          <button
            type="button"
            className="btn btn-primary"
            disabled={!atLatest}
            title={atLatest ? "Compose a historical event" : "Return to latest to add events"}
            onClick={() => openEvent(snapshot.communities.length === 0 ? "Found" : "Develop")}
          >
            <Plus size={16} strokeWidth={1.75} aria-hidden />
            Add event
          </button>
          <button type="button" className="btn btn-quiet" onClick={() => importInputRef.current?.click()}>
            <Upload size={16} strokeWidth={1.75} aria-hidden />
            Import
          </button>
          <button type="button" className="btn btn-quiet" onClick={exportHistory}>
            <Download size={16} strokeWidth={1.75} aria-hidden />
            Export
          </button>
          <button type="button" className="btn btn-quiet" onClick={() => setDialog("scenario")}>
            New
          </button>
        </div>
      </header>

      {!atLatest ? (
        <div className="readonly-banner" role="status">
          Viewing checkpoint {snapshot.checkpoint} of {snapshot.latest}. History is read-only here.
          <button type="button" className="text-link" onClick={returnLatest}>
            Return to latest
          </button>
        </div>
      ) : null}

      {notice ? (
        <p className="notice" role="alert">
          {notice}
        </p>
      ) : null}
      {persistError ? (
        <p className="notice" role="alert">
          Could not save to this browser: {persistError}
        </p>
      ) : null}

      <div className="workspace">
        <nav className="communities" aria-label="Communities">
          <div className="pane-head">
            <h2>Communities</h2>
            <span className="count">{snapshot.communities.length}</span>
          </div>
          {snapshot.communities.length === 0 ? (
            <p className="empty-block">No communities yet.</p>
          ) : (
            <ul>
              {snapshot.communities.map((item) => (
                <li key={item.id}>
                  <button
                    type="button"
                    className={item.id === communityId ? "community-btn selected" : "community-btn"}
                    aria-current={item.id === communityId}
                    onClick={() => selectCommunity(item.id)}
                  >
                    <span className="community-name">{item.name}</span>
                    <span className="badges">
                      <span className="badge">{item.aesthetic.name}</span>
                      <span className="badge">
                        {item.parent == null
                          ? "founded"
                          : `from ${snapshot.communities.find((parent) => parent.id === item.parent)?.name ?? item.parent}`}
                      </span>
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
          <button
            type="button"
            className="btn found-btn"
            disabled={!atLatest}
            onClick={() => openEvent("Found")}
          >
            <GitFork size={16} strokeWidth={1.75} aria-hidden />
            Found community
          </button>
        </nav>

        <main className="lexicon">
          <div className="pane-head lexicon-toolbar">
            <div>
              <h2>Lexicon</h2>
              <p className="count">
                {search.trim() || originFilter !== "all"
                  ? `${filtered.length} of ${community?.lexicon.length ?? 0}`
                  : `${community?.lexicon.length ?? 0} words`}
              </p>
            </div>
            <div className="search-wrap">
              <Search size={16} strokeWidth={1.75} aria-hidden />
              <input
                id={searchId}
                type="search"
                value={search}
                placeholder="Search form, meaning, or IPA"
                aria-label="Search lexicon"
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>
            <div className="filters" role="group" aria-label="Origin filter">
              {(["all", "root", "derived", "loan"] as const).map((filter) => (
                <button
                  key={filter}
                  type="button"
                  className={originFilter === filter ? "chip selected" : "chip"}
                  aria-pressed={originFilter === filter}
                  onClick={() => setOriginFilter(filter)}
                >
                  {filter === "all" ? "All" : filter === "root" ? "Roots" : filter === "derived" ? "Derived" : "Loans"}
                </button>
              ))}
            </div>
          </div>
          {!community ? (
            <div className="empty-block">
              <p>No community selected.</p>
              <p className="muted">Found a community to mint a lexicon.</p>
            </div>
          ) : community.lexicon.length === 0 ? (
            <div className="empty-block">
              <p>This community has no words yet.</p>
            </div>
          ) : filtered.length === 0 ? (
            <div className="empty-block">
              <p>No words match {search.trim() ? `“${search.trim()}”` : "this filter"}.</p>
            </div>
          ) : (
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th>Word</th>
                    <th>Meaning</th>
                    <th>Origin</th>
                  </tr>
                </thead>
                <tbody tabIndex={0} onKeyDown={onLexiconKey} aria-label="Lexicon rows">
                  {filtered.map((item) => {
                    const origin = originKind(item);
                    return (
                      <tr
                        key={item.id}
                        className={item.id === lexemeId ? "selected" : undefined}
                        aria-selected={item.id === lexemeId}
                        onClick={() => setLexemeId(item.id)}
                      >
                        <td className="form">{item.form}</td>
                        <td>{item.gloss}</td>
                        <td>
                          <span className={`origin origin-${origin}`}>
                            {origin === "root" ? "Root" : origin === "derived" ? "Derived" : "Loan"}
                          </span>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </main>

        <Inspector
          snapshot={snapshot}
          community={community}
          lexeme={lexeme}
          options={options}
          readOnly={!atLatest}
          onSelectLexeme={(id) => {
            setLexemeId(id);
            setSearch("");
            setOriginFilter("all");
          }}
          onSelectCommunity={selectCommunity}
          onPreviewDerive={(option) => {
            if (!community || !lexeme || !atLatest) return;
            previewEvent(
              { Derive: { community: community.id, base: lexeme.id, kind: option.kind } },
              "derive",
            );
          }}
        />
      </div>

      <input
        ref={importInputRef}
        type="file"
        accept="application/json,.json"
        hidden
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (file) onImportFile(file);
        }}
      />

      <Modal
        open={dialog === "history"}
        title="History"
        onClose={closeDialog}
        wide
        footer={
          atLatest ? (
            <button type="button" className="btn" onClick={closeDialog}>
              Close
            </button>
          ) : (
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => {
                returnLatest();
                closeDialog();
              }}
            >
              Return to latest
            </button>
          )
        }
      >
        <ol className="history-list">
          {snapshot.checkpoints.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                className={item.id === snapshot.checkpoint ? "history-item selected" : "history-item"}
                onClick={() => browseCheckpoint(item.id)}
              >
                <span className="history-id">ck{item.id}</span>
                <span className="history-summary">{item.summary || shortEvent(item.event)}</span>
              </button>
            </li>
          ))}
        </ol>
        <p className="muted">
          Seed {snapshot.seed}. Latest is checkpoint {snapshot.latest}: {latestSummary}
        </p>
      </Modal>

      <Modal
        open={dialog === "event"}
        title={pending ? "Preview event" : "Add event"}
        onClose={() => {
          if (pending) {
            setPending(null);
            setEventError("");
            return;
          }
          closeDialog();
        }}
        wide
        footer={
          pending ? (
            <>
              <button
                type="button"
                className="btn"
                onClick={() => {
                  setPending(null);
                  setEventError("");
                }}
              >
                Back
              </button>
              <button type="button" className="btn btn-primary" onClick={acceptPending}>
                Accept
              </button>
            </>
          ) : (
            <>
              <button type="button" className="btn" onClick={closeDialog}>
                Cancel
              </button>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  const built = eventFromDraft(draft);
                  const message = validateDraft(draft, snapshot.communities);
                  if (message) {
                    setEventError(message);
                    return;
                  }
                  previewEvent(built);
                }}
              >
                Preview
              </button>
            </>
          )
        }
      >
        {pending ? (
          <PreviewBody pending={pending} error={eventError} />
        ) : (
          <EventFields
            draft={draft}
            communities={snapshot.communities}
            error={eventError}
            onChange={setDraft}
          />
        )}
      </Modal>

      <Modal
        open={dialog === "derive"}
        title="Preview formation"
        onClose={closeDialog}
        wide
        footer={
          <>
            <button type="button" className="btn" onClick={closeDialog}>
              Cancel
            </button>
            <button type="button" className="btn btn-primary" disabled={!pending} onClick={acceptPending}>
              Accept
            </button>
          </>
        }
      >
        {pending ? <PreviewBody pending={pending} error={eventError} /> : <p className="error">{eventError}</p>}
      </Modal>

      <Modal
        open={dialog === "scenario"}
        title="New workshop"
        onClose={closeDialog}
        footer={
          <>
            <button type="button" className="btn" onClick={closeDialog}>
              Cancel
            </button>
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => {
                if (!Number.isInteger(scenario.seed) || scenario.seed < 0 || scenario.seed > 0xffff_ffff) {
                  setScenarioError("Choose a whole-number seed between 0 and 4294967295.");
                  return;
                }
                void startScenario(scenario);
              }}
            >
              Replace and start
            </button>
          </>
        }
      >
        <p>This replaces the current language history stored in this browser.</p>
        <ScenarioFields value={scenario} onChange={setScenario} />
        {scenarioError ? <p className="error" role="alert">{scenarioError}</p> : null}
      </Modal>

      <Modal
        open={dialog === "import"}
        title="Import history"
        onClose={closeDialog}
        footer={
          <>
            <button type="button" className="btn" onClick={closeDialog}>
              Cancel
            </button>
            <button
              type="button"
              className="btn btn-primary"
              disabled={!importPending}
              onClick={acceptImport}
            >
              Replace current work
            </button>
          </>
        }
      >
        {importPending ? (
          <p>
            Import <strong>{importPending.name}</strong>? This replaces the current history after the engine
            validates the file.
          </p>
        ) : (
          <p>Choose a JSON history file.</p>
        )}
        {importError ? <p className="error">{importError}</p> : null}
      </Modal>
    </div>
  );
}

function EventFields({
  draft,
  communities,
  error,
  onChange,
}: {
  draft: EventDraft;
  communities: Community[];
  error: string;
  onChange: (draft: EventDraft) => void;
}) {
  return (
    <form className="stack" onSubmit={(event) => event.preventDefault()}>
      <div className="filters" role="radiogroup" aria-label="Event kind">
        {(["Found", "Separate", "Develop", "Contact"] as const).map((kind) => (
          <button
            key={kind}
            type="button"
            className={draft.kind === kind ? "chip selected" : "chip"}
            aria-pressed={draft.kind === kind}
            onClick={() => onChange({ ...draft, kind })}
          >
            {kind}
          </button>
        ))}
      </div>
      {draft.kind === "Found" || draft.kind === "Separate" ? (
        <label className="field">
          <span>Name</span>
          <input
            value={draft.name}
            onChange={(event) => onChange({ ...draft, name: event.target.value })}
            autoComplete="off"
          />
        </label>
      ) : null}
      {draft.kind === "Found" ? (
        <label className="field">
          <span>Aesthetic</span>
          <select
            value={draft.aesthetic}
            onChange={(event) => onChange({ ...draft, aesthetic: event.target.value })}
          >
            {AESTHETICS.map((id) => (
              <option key={id} value={id}>
                {AESTHETIC_LABELS[id]}
              </option>
            ))}
          </select>
        </label>
      ) : null}
      {draft.kind === "Separate" ? (
        <CommunitySelect
          label="Parent"
          value={draft.parent}
          communities={communities}
          onChange={(parent) => onChange({ ...draft, parent })}
        />
      ) : null}
      {draft.kind === "Develop" ? (
        <>
          <CommunitySelect
            label="Community"
            value={draft.community}
            communities={communities}
            onChange={(community) => onChange({ ...draft, community })}
          />
          <label className="field">
            <span>Steps</span>
            <input
              type="number"
              min={1}
              max={24}
              value={draft.steps}
              onChange={(event) => onChange({ ...draft, steps: Number(event.target.value) })}
            />
          </label>
        </>
      ) : null}
      {draft.kind === "Contact" ? (
        <>
          <CommunitySelect
            label="Donor"
            value={draft.donor}
            communities={communities}
            onChange={(donor) => onChange({ ...draft, donor })}
          />
          <CommunitySelect
            label="Recipient"
            value={draft.recipient}
            communities={communities}
            onChange={(recipient) => onChange({ ...draft, recipient })}
          />
          <label className="field">
            <span>Domain</span>
            <select
              value={draft.domain}
              onChange={(event) => onChange({ ...draft, domain: event.target.value as ContactDomain })}
            >
              <option value="General">General</option>
              <option value="Maritime">Maritime</option>
            </select>
          </label>
          <label className="field">
            <span>Loan count</span>
            <input
              type="number"
              min={1}
              max={64}
              value={draft.count}
              onChange={(event) => onChange({ ...draft, count: Number(event.target.value) })}
            />
          </label>
        </>
      ) : null}
      {error ? <p className="error">{error}</p> : null}
    </form>
  );
}

function CommunitySelect({
  label,
  value,
  communities,
  onChange,
}: {
  label: string;
  value: number;
  communities: Community[];
  onChange: (id: number) => void;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <select
        value={value}
        disabled={communities.length === 0}
        onChange={(event) => onChange(Number(event.target.value))}
      >
        {communities.length === 0 ? <option value={0}>None</option> : null}
        {communities.map((item) => (
          <option key={item.id} value={item.id}>
            {item.name}
          </option>
        ))}
      </select>
    </label>
  );
}

function ScenarioFields({
  value,
  onChange,
}: {
  value: NewHistory;
  onChange: (value: NewHistory) => void;
}) {
  return (
    <form className="stack" onSubmit={(event) => event.preventDefault()}>
      <label className="field">
        <span>Seed</span>
        <input
          type="number"
          min={0}
          max={4294967295}
          value={value.seed}
          onChange={(event) => onChange({ ...value, seed: Number(event.target.value) })}
        />
      </label>
      <label className="field">
        <span>Aesthetic</span>
        <select
          value={value.aesthetic}
          onChange={(event) => onChange({ ...value, aesthetic: event.target.value })}
        >
          {AESTHETICS.map((id) => (
            <option key={id} value={id}>
              {AESTHETIC_LABELS[id]}
            </option>
          ))}
        </select>
      </label>
      <fieldset className="field">
        <legend>Starting point</legend>
        <label className="choice">
          <input
            type="radio"
            name="starting-point"
            checked={!value.empty}
            onChange={() => onChange({ ...value, empty: false })}
          />
          Coastal scenario
        </label>
        <label className="choice">
          <input
            type="radio"
            name="starting-point"
            checked={value.empty}
            onChange={() => onChange({ ...value, empty: true })}
          />
          Empty workshop
        </label>
      </fieldset>
    </form>
  );
}

function PreviewBody({ pending, error }: { pending: Pending; error: string }) {
  const communities = pending.snapshot.communities;
  return (
    <div className="stack">
      <p className="lead">{pending.snapshot.summary}</p>
      <p className="muted">
        Involved{" "}
        {involvedNames(pending.event, pending.snapshot).join(", ") || "none yet"}
        {pending.snapshot.checkpoint !== pending.snapshot.latest
          ? ` · projected checkpoint ${pending.snapshot.checkpoint}`
          : ""}
      </p>
      {pending.snapshot.rules.length > 0 ? (
        <section>
          <h3>Rules</h3>
          <ul className="rules">
            {pending.snapshot.rules.map((rule) => (
              <li key={rule.id}>
                <strong>{rule.label}</strong>
                <span className="muted">{rule.detail}</span>
              </li>
            ))}
          </ul>
        </section>
      ) : null}
      <section>
        <h3>Effects</h3>
        {pending.snapshot.effects.length === 0 ? (
          <p className="muted">No changes in this projection.</p>
        ) : (
          <ul className="effects">
            {pending.snapshot.effects.map((effect, index) => {
              const host =
                communities.find((item) => item.id === effect.community)?.name ?? `Community ${effect.community}`;
              const pattern = effect.lexeme === "";
              return (
                <li
                  key={`effect-${index}-${effect.community}-${effect.lexeme}-${effect.before}-${effect.after}`}
                  className={pattern ? "effect-pattern" : undefined}
                >
                  <span className="effect-head">
                    {pattern
                      ? `${host} · suffix ${effect.gloss} · ${effect.before ?? "—"} → ${effect.after}`
                      : `${host} · ${effect.gloss} · ${effect.before ?? "—"} → ${effect.after}`}
                  </span>
                  <span className="muted">{effect.explanation}</span>
                </li>
              );
            })}
          </ul>
        )}
      </section>
      {error ? <p className="error">{error}</p> : null}
    </div>
  );
}

function emptyDraft(): EventDraft {
  return {
    kind: "Found",
    name: "",
    aesthetic: "elvish",
    parent: 0,
    community: 0,
    donor: 0,
    recipient: 0,
    steps: 1,
    count: 8,
    domain: "Maritime",
  };
}

function draftFrom(kind: EventKind, snapshot: Snapshot, communityId: number | null): EventDraft {
  const selected = communityId ?? snapshot.communities[0]?.id ?? 0;
  const other = snapshot.communities.find((item) => item.id !== selected)?.id ?? selected;
  const host = snapshot.communities.find((item) => item.id === selected);
  return {
    ...emptyDraft(),
    kind,
    aesthetic: host?.aesthetic.id ?? "elvish",
    parent: selected,
    community: selected,
    donor: other,
    recipient: selected,
  };
}

function eventFromDraft(draft: EventDraft): HistoryEvent {
  if (draft.kind === "Found") return { Found: { name: draft.name.trim(), aesthetic: draft.aesthetic } };
  if (draft.kind === "Separate") return { Separate: { parent: draft.parent, name: draft.name.trim() } };
  if (draft.kind === "Develop") return { Develop: { community: draft.community, steps: draft.steps } };
  return {
    Contact: {
      donor: draft.donor,
      recipient: draft.recipient,
      domain: draft.domain,
      count: draft.count,
    },
  };
}

function validateDraft(draft: EventDraft, communities: Community[]): string | null {
  if (draft.kind === "Found" && draft.name.trim().length === 0) return "Name the community.";
  if (draft.kind === "Separate") {
    if (communities.length === 0) return "Found a community before separating.";
    if (draft.name.trim().length === 0) return "Name the new community.";
  }
  if (draft.kind === "Develop") {
    if (communities.length === 0) return "Found a community before developing.";
    if (!Number.isInteger(draft.steps) || draft.steps < 1) return "Development needs at least one step.";
  }
  if (draft.kind === "Contact") {
    if (communities.length < 2) return "Contact needs two communities.";
    if (draft.donor === draft.recipient) return "Donor and recipient must differ.";
    if (!Number.isInteger(draft.count) || draft.count < 1) return "Loan count must be at least 1.";
  }
  return null;
}

function originKind(lexeme: Lexeme): "root" | "derived" | "loan" {
  if (lexeme.baseLexeme) return "derived";
  if (lexeme.traces.some((trace) => trace.explanation.includes("loaned from"))) return "loan";
  return "root";
}

function shortEvent(event: HistoryEvent | null): string {
  if (!event) return "Begin";
  if ("Found" in event) return `Found ${event.Found.name}`;
  if ("Separate" in event) return `Split ${event.Separate.name}`;
  if ("Develop" in event) return `Develop ${event.Develop.steps}`;
  if ("Contact" in event) return `Loan ${event.Contact.domain}`;
  if ("Derive" in event) return `Derive ${event.Derive.kind}`;
  return "Event";
}

function involvedNames(event: HistoryEvent, snapshot: Snapshot): string[] {
  const names: string[] = [];
  const label = (id: number) => snapshot.communities.find((item) => item.id === id)?.name ?? `Community ${id}`;
  if ("Found" in event) names.push(event.Found.name);
  if ("Separate" in event) names.push(label(event.Separate.parent), event.Separate.name);
  if ("Develop" in event) names.push(label(event.Develop.community));
  if ("Contact" in event) names.push(label(event.Contact.donor), label(event.Contact.recipient));
  if ("Derive" in event) names.push(label(event.Derive.community));
  return names.filter((name, index) => names.indexOf(name) === index);
}

function asError(err: unknown): string {
  if (err instanceof Error && err.message.trim()) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  return "Something went wrong.";
}

function readStoredHistory():
  | { kind: "none" }
  | { kind: "value"; raw: string }
  | { kind: "malformed"; raw: string; message: string } {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw == null || raw.trim() === "") return { kind: "none" };
    try {
      JSON.parse(raw);
    } catch {
      return { kind: "malformed", raw, message: "The stored history is not valid JSON." };
    }
    return { kind: "value", raw };
  } catch {
    return { kind: "none" };
  }
}

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
  Role,
  SemanticFrame,
  Snapshot,
  UseDomain,
  Variety,
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
const ROLES: Role[] = ["Agent", "Experiencer", "Patient"];
const USE_DOMAINS: UseDomain[] = ["Home", "Trade", "Ritual"];
const GLOSS_MAX = 80;
const ORIGIN_FILTER: Record<Lexeme["origin"]["kind"], "unrecorded" | "formed" | "borrowed" | "inherited"> = {
  Unrecorded: "unrecorded",
  Formed: "formed",
  Borrowed: "borrowed",
  Inherited: "inherited",
};
const EVENT_KINDS = [
  "Found",
  "UseLanguage",
  "Separate",
  "Develop",
  "Contact",
  "Derive",
  "Sense",
  "Replace",
  "Lexicalize",
  "Remodel",
] as const;
const EVENT_KIND_LABELS: Record<(typeof EVENT_KINDS)[number], string> = {
  Found: "Found",
  UseLanguage: "Use",
  Separate: "Separate",
  Develop: "Develop",
  Contact: "Contact",
  Derive: "Derive",
  Sense: "Sense",
  Replace: "Replace",
  Lexicalize: "Lexicalize",
  Remodel: "Remodel",
};

type Boot = "loading" | "ready" | "error" | "recover";
type Dialog = "none" | "history" | "event" | "scenario" | "import" | "preview";
type OriginFilter = "all" | "unrecorded" | "formed" | "borrowed" | "inherited" | "archived";
type EventKind = (typeof EVENT_KINDS)[number];
type SenseMode = "extend" | "shift";

type EventDraft = {
  kind: EventKind;
  name: string;
  group: string;
  location: string;
  ancestry: string;
  languageMode: "New" | "Existing";
  languageName: string;
  aesthetic: string;
  existingVariety: number;
  parent: number;
  community: number;
  variety: number;
  domain: UseDomain;
  donor: number;
  recipient: number;
  steps: number;
  count: number;
  contactDomain: ContactDomain;
  lexeme: string;
  sense: number;
  construction: string;
  gloss: string;
  senseMode: SenseMode;
  frameKind: "Entity" | "Event";
  countable: boolean;
  roles: Role[];
  replacement: string;
  base: string;
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
  const [varietyId, setVarietyId] = useState<number | null>(null);
  const [lexemeId, setLexemeId] = useState<string | null>(null);
  const [senseId, setSenseId] = useState<number | null>(null);
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

  const community =
    snapshot?.communities.find((item) => item.id === communityId) ?? snapshot?.communities[0] ?? null;
  const communityVarieties = community && snapshot ? usedVarieties(snapshot, community) : [];
  const variety =
    communityVarieties.find((item) => item.id === varietyId) ?? communityVarieties[0] ?? null;
  const atLatest = snapshot != null && snapshot.checkpoint === snapshot.latest;

  const filtered = useMemo(() => {
    if (!variety) return [];
    const query = search.trim().toLowerCase();
    return variety.lexicon.filter((item) => {
      if (originFilter === "archived") {
        if (!item.retired) return false;
      } else if (item.retired) {
        return false;
      } else if (originFilter !== "all" && ORIGIN_FILTER[item.origin.kind] !== originFilter) {
        return false;
      }
      if (query.length === 0) return true;
      const meanings = item.senses.map((sense) => sense.gloss).join(" ");
      return (
        item.form.toLowerCase().includes(query) ||
        item.ipa.toLowerCase().includes(query) ||
        item.classLabel.toLowerCase().includes(query) ||
        item.classId.toLowerCase().includes(query) ||
        meanings.toLowerCase().includes(query) ||
        item.origin.label.toLowerCase().includes(query)
      );
    });
  }, [variety, search, originFilter]);

  const lexeme = variety?.lexicon.find((item) => item.id === lexemeId) ?? null;
  const sense = lexeme?.senses.find((item) => item.id === senseId) ?? lexeme?.senses[0] ?? null;

  useEffect(() => {
    if (!snapshot) return;
    const nextCommunity = snapshot.communities.find((item) => item.id === communityId) ?? snapshot.communities[0] ?? null;
    if (nextCommunity?.id !== communityId) setCommunityId(nextCommunity?.id ?? null);
    const nextVarieties = nextCommunity ? usedVarieties(snapshot, nextCommunity) : [];
    const nextVariety = nextVarieties.find((item) => item.id === varietyId) ?? nextVarieties[0] ?? null;
    if (nextVariety?.id !== varietyId) setVarietyId(nextVariety?.id ?? null);
    const nextLexeme =
      nextVariety?.lexicon.find((item) => item.id === lexemeId) ??
      nextVariety?.lexicon.find((item) => !item.retired) ??
      nextVariety?.lexicon[0] ??
      null;
    if (nextLexeme?.id !== lexemeId) setLexemeId(nextLexeme?.id ?? null);
    const nextSense = nextLexeme?.senses.find((item) => item.id === senseId) ?? nextLexeme?.senses[0] ?? null;
    if (nextSense?.id !== senseId) setSenseId(nextSense?.id ?? null);
  }, [snapshot, communityId, varietyId, lexemeId, senseId]);

  useEffect(() => {
    if (!engine || !snapshot || !variety || !lexeme || !sense || lexeme.retired) {
      setOptions([]);
      return;
    }
    try {
      setOptions(engine.options(snapshot.checkpoint, variety.id, lexeme.id, sense.id));
    } catch (err) {
      setOptions([]);
      setNotice(asError(err));
    }
  }, [engine, snapshot, variety, lexeme, sense]);

  const composerOptions = useMemo(() => {
    if (!engine || !snapshot || (draft.kind !== "Derive" && draft.kind !== "Remodel")) return [];
    if (!draft.base) return [];
    try {
      return engine.options(snapshot.checkpoint, draft.variety, draft.base, draft.sense);
    } catch {
      return [];
    }
  }, [engine, snapshot, draft.kind, draft.variety, draft.base, draft.sense]);

  function adoptEngine(next: Engine, nextSnapshot: Snapshot, write: boolean) {
    if (engineRef.current && engineRef.current !== next) engineRef.current.dispose();
    engineRef.current = next;
    setEngine(next);
    applySnapshot(nextSnapshot);
    const firstCommunity = nextSnapshot.communities[0] ?? null;
    const firstVariety =
      (firstCommunity ? usedVarieties(nextSnapshot, firstCommunity)[0] : null) ?? nextSnapshot.varieties[0] ?? null;
    const firstLexeme =
      firstVariety?.lexicon.find((item) => !item.retired) ?? firstVariety?.lexicon[0] ?? null;
    setCommunityId(firstCommunity?.id ?? null);
    setVarietyId(firstVariety?.id ?? null);
    setLexemeId(firstLexeme?.id ?? null);
    setSenseId(firstLexeme?.senses[0]?.id ?? null);
    setSearch("");
    setOriginFilter("all");
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
    setCommunityId(id);
    const next = snapshot?.communities.find((item) => item.id === id);
    const keepVariety = next?.uses.some((use) => use.variety === varietyId);
    const nextVarietyId = keepVariety ? varietyId : (next?.uses[0]?.variety ?? null);
    setVarietyId(nextVarietyId);
    const nextVariety = snapshot?.varieties.find((item) => item.id === nextVarietyId);
    if (!nextVariety?.lexicon.some((item) => item.id === lexemeId)) {
      const fallback = nextVariety?.lexicon.find((item) => !item.retired) ?? nextVariety?.lexicon[0] ?? null;
      setLexemeId(fallback?.id ?? null);
      setSenseId(fallback?.senses[0]?.id ?? null);
    }
  }

  function selectVariety(id: number) {
    setSearch("");
    setOriginFilter("all");
    setVarietyId(id);
    const next = snapshot?.varieties.find((item) => item.id === id);
    const fallback = next?.lexicon.find((item) => !item.retired) ?? next?.lexicon[0] ?? null;
    setLexemeId(fallback?.id ?? null);
    setSenseId(fallback?.senses[0]?.id ?? null);
  }

  function openLexeme(args: { variety: number; lexeme: string; sense?: number; checkpoint?: number }) {
    if (!snapshot || !engine) return;
    try {
      const target = args.checkpoint === undefined ? snapshot : engine.snapshot(args.checkpoint);
      const host = communityUsing(target, args.variety, communityId);
      const word = target.varieties
        .find((item) => item.id === args.variety)
        ?.lexicon.find((item) => item.id === args.lexeme);
      if (!host || !word) {
        setNotice("That word is not available at this checkpoint.");
        return;
      }
      if (target !== snapshot) applySnapshot(target);
      setSearch("");
      setOriginFilter(word.retired ? "archived" : "all");
      setCommunityId(host.id);
      setVarietyId(args.variety);
      setLexemeId(word.id);
      setSenseId(args.sense ?? word.senses[0]?.id ?? null);
    } catch (err) {
      setNotice(asError(err));
    }
  }

  function openEvent(kind: EventKind) {
    if (!snapshot || !atLatest) return;
    setDraft(draftFrom(kind, snapshot, communityId, varietyId, lexemeId, senseId));
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
      if (nextDialog === "preview") setDialog("preview");
    }
  }

  function acceptPending() {
    if (!engine || !pending || !snapshot) return;
    try {
      const next = engine.commit(pending.event);
      persist(engine);
      applySnapshot(next);
      setSearch("");
      setOriginFilter("all");
      settleAfterEvent(pending.event, snapshot, next, communityId, setCommunityId, setVarietyId, setLexemeId, setSenseId);
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
      adoptEngine(engine, next, true);
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
      if (next) {
        setLexemeId(next.id);
        setSenseId(next.senses[0]?.id ?? null);
      }
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      const next = filtered[Math.max(0, (index === -1 ? 1 : index) - 1)];
      if (next) {
        setLexemeId(next.id);
        setSenseId(next.senses[0]?.id ?? null);
      }
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
                adoptEngine(engineRef.current, next, true);
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
  const selectedHidden = Boolean(lexeme && !filtered.some((item) => item.id === lexeme.id));

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
        <label className="header-field">
          <span>Language</span>
          <select
            aria-label="Selected language"
            value={variety?.id ?? ""}
            disabled={communityVarieties.length === 0}
            onChange={(event) => selectVariety(Number(event.target.value))}
          >
            {communityVarieties.length === 0 ? <option value="">None</option> : null}
            {communityVarieties.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
                {community
                  ? ` · ${community.uses
                      .filter((use) => use.variety === item.id)
                      .map((use) => use.domain)
                      .join(", ")}`
                  : ""}
              </option>
            ))}
          </select>
        </label>
        <button type="button" className="chip" onClick={() => setDialog("history")} aria-haspopup="dialog">
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
                    <span className="community-meta">
                      {item.group} · {item.location}
                    </span>
                    <span className="community-ancestry">{item.ancestry}</span>
                    <span className="badges">
                      {item.uses.length === 0 ? <span className="badge">no language</span> : null}
                      {item.uses.map((use) => (
                        <span className="badge" key={`${use.variety}-${use.domain}`}>
                          {snapshot.varieties.find((entry) => entry.id === use.variety)?.name ?? use.variety} ·{" "}
                          {use.domain}
                        </span>
                      ))}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div className="community-actions">
            <button type="button" className="btn found-btn" disabled={!atLatest} onClick={() => openEvent("Found")}>
              <GitFork size={16} strokeWidth={1.75} aria-hidden />
              Found community
            </button>
            <button
              type="button"
              className="btn found-btn"
              disabled={!atLatest || snapshot.varieties.length === 0 || snapshot.communities.length === 0}
              onClick={() => openEvent("UseLanguage")}
            >
              Use language
            </button>
          </div>
        </nav>

        <main className="lexicon">
          <div className="pane-head lexicon-toolbar">
            <div>
              <h2>Lexicon</h2>
              <p className="count">
                {variety ? `${variety.name} · ` : ""}
                {search.trim() || originFilter !== "all"
                  ? `${filtered.length} of ${variety?.lexicon.length ?? 0}`
                  : `${variety?.lexicon.filter((item) => !item.retired).length ?? 0} words`}
              </p>
            </div>
            <div className="search-wrap">
              <Search size={16} strokeWidth={1.75} aria-hidden />
              <input
                id={searchId}
                type="search"
                value={search}
                placeholder="Search form, meaning, class, or IPA"
                aria-label="Search lexicon"
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>
            <div className="filters" role="group" aria-label="Origin filter">
              {(["all", "unrecorded", "formed", "borrowed", "inherited", "archived"] as const).map((filter) => (
                <button
                  key={filter}
                  type="button"
                  className={originFilter === filter ? "chip selected" : "chip"}
                  aria-pressed={originFilter === filter}
                  onClick={() => setOriginFilter(filter)}
                >
                  {filter === "all"
                    ? "All"
                    : filter === "unrecorded"
                      ? "Roots"
                      : filter === "formed"
                        ? "Formed"
                        : filter === "borrowed"
                          ? "Loans"
                          : filter === "inherited"
                            ? "Inherited"
                            : "Archived"}
                </button>
              ))}
            </div>
          </div>
          {!community ? (
            <div className="empty-block">
              <p>No community selected.</p>
              <p className="muted">Found a community to mint a lexicon.</p>
            </div>
          ) : !variety ? (
            <div className="empty-block">
              <p>This community has no language yet.</p>
              <p className="muted">Attach an existing language or found a new one.</p>
            </div>
          ) : variety.lexicon.length === 0 ? (
            <div className="empty-block">
              <p>This language has no words yet.</p>
            </div>
          ) : filtered.length === 0 && !selectedHidden ? (
            <div className="empty-block">
              <p>No words match {search.trim() ? `“${search.trim()}”` : "this filter"}.</p>
            </div>
          ) : (
            <>
              {selectedHidden ? (
                <p className="muted hidden-selection">
                  Inspector is showing a word hidden by search or filter.{" "}
                  <button
                    type="button"
                    className="text-link"
                    onClick={() => {
                      setSearch("");
                      setOriginFilter("all");
                    }}
                  >
                    Show in list
                  </button>
                </p>
              ) : null}
              {filtered.length === 0 ? null : (
                <div className="table-scroll">
                  <table>
                    <thead>
                      <tr>
                        <th>Word</th>
                        <th>Meaning</th>
                        <th>Class</th>
                        <th>Origin</th>
                      </tr>
                    </thead>
                    <tbody tabIndex={0} onKeyDown={onLexiconKey} aria-label="Lexicon rows">
                      {filtered.map((item) => {
                        const origin = ORIGIN_FILTER[item.origin.kind];
                        return (
                          <tr
                            key={item.id}
                            className={[
                              item.id === lexemeId ? "selected" : "",
                              item.retired ? "retired" : "",
                            ]
                              .filter(Boolean)
                              .join(" ") || undefined}
                            aria-selected={item.id === lexemeId}
                            onClick={() => {
                              if (item.id === lexemeId) return;
                              setLexemeId(item.id);
                              setSenseId(item.senses[0]?.id ?? null);
                            }}
                          >
                            <td className="form">{item.form}</td>
                            <td>{item.senses.map((entry) => entry.gloss).join(" · ") || "—"}</td>
                            <td>{item.classLabel}</td>
                            <td>
                              <span className={`origin origin-${origin}`} title={item.origin.label}>
                                {item.retired ? "Archived · " : ""}
                                {item.origin.kind === "Unrecorded" ? "Root" : item.origin.kind === "Borrowed" ? "Loan" : item.origin.kind}
                              </span>
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              )}
            </>
          )}
        </main>

        <Inspector
          snapshot={snapshot}
          community={community}
          variety={variety}
          lexeme={lexeme}
          sense={sense}
          options={options}
          readOnly={!atLatest}
          onSelectSense={setSenseId}
          onOpenLexeme={openLexeme}
          onSelectCommunity={selectCommunity}
          onPreviewDerive={(option) => {
            if (!variety || !lexeme || !sense || !atLatest) return;
            previewEvent(
              {
                Derive: {
                  variety: variety.id,
                  base: lexeme.id,
                  sense: sense.id,
                  construction: option.construction,
                },
              },
              "preview",
            );
          }}
          onCompose={(kind) => openEvent(kind)}
          onPreviewLexicalize={() => {
            if (!variety || !lexeme || !atLatest) return;
            previewEvent({ Lexicalize: { variety: variety.id, lexeme: lexeme.id } }, "preview");
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
                  const built = eventFromDraft(draft, snapshot, composerOptions);
                  const message = validateDraft(draft, snapshot, composerOptions);
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
            snapshot={snapshot}
            formations={composerOptions}
            error={eventError}
            onChange={setDraft}
          />
        )}
      </Modal>

      <Modal
        open={dialog === "preview"}
        title="Preview event"
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
        {scenarioError ? (
          <p className="error" role="alert">
            {scenarioError}
          </p>
        ) : null}
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
            <button type="button" className="btn btn-primary" disabled={!importPending} onClick={acceptImport}>
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
  snapshot,
  formations,
  error,
  onChange,
}: {
  draft: EventDraft;
  snapshot: Snapshot;
  formations: FormationOption[];
  error: string;
  onChange: (draft: EventDraft) => void;
}) {
  const variety = snapshot.varieties.find((item) => item.id === draft.variety);
  const hostLexemes = (variety?.lexicon ?? []).filter((item) => !item.retired);
  const baseLexeme = variety?.lexicon.find((item) => item.id === draft.base);
  const targetLexeme = variety?.lexicon.find((item) => item.id === draft.lexeme);
  const construction =
    formations.find((item) => item.construction === draft.construction)?.construction ??
    formations[0]?.construction ??
    "";

  return (
    <form className="stack" onSubmit={(event) => event.preventDefault()}>
      <div className="filters" role="radiogroup" aria-label="Event kind">
        {EVENT_KINDS.map((kind) => (
          <button
            key={kind}
            type="button"
            className={draft.kind === kind ? "chip selected" : "chip"}
            aria-pressed={draft.kind === kind}
            onClick={() => onChange(draftFrom(kind, snapshot, draft.community, draft.variety, draft.lexeme, draft.sense))}
          >
            {EVENT_KIND_LABELS[kind]}
          </button>
        ))}
      </div>

      {draft.kind === "Found" ? (
        <>
          <label className="field">
            <span>Community name</span>
            <input
              value={draft.name}
              onChange={(event) => onChange({ ...draft, name: event.target.value })}
              autoComplete="off"
            />
          </label>
          <label className="field">
            <span>Group</span>
            <input
              value={draft.group}
              onChange={(event) => onChange({ ...draft, group: event.target.value })}
              autoComplete="off"
            />
          </label>
          <label className="field">
            <span>Location</span>
            <input
              value={draft.location}
              onChange={(event) => onChange({ ...draft, location: event.target.value })}
              autoComplete="off"
            />
          </label>
          <label className="field">
            <span>Ancestry</span>
            <input
              value={draft.ancestry}
              onChange={(event) => onChange({ ...draft, ancestry: event.target.value })}
              autoComplete="off"
            />
          </label>
          <fieldset className="field">
            <legend>Language</legend>
            <label className="choice">
              <input
                type="radio"
                name="found-language"
                checked={draft.languageMode === "New"}
                onChange={() => onChange({ ...draft, languageMode: "New" })}
              />
              Found new language
            </label>
            <label className="choice">
              <input
                type="radio"
                name="found-language"
                checked={draft.languageMode === "Existing"}
                disabled={snapshot.varieties.length === 0}
                onChange={() => onChange({ ...draft, languageMode: "Existing" })}
              />
              Share existing language
            </label>
          </fieldset>
          {draft.languageMode === "New" ? (
            <>
              <label className="field">
                <span>Language name</span>
                <input
                  value={draft.languageName}
                  onChange={(event) => onChange({ ...draft, languageName: event.target.value })}
                  autoComplete="off"
                />
              </label>
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
            </>
          ) : (
            <VarietySelect
              label="Existing language"
              value={draft.existingVariety}
              varieties={snapshot.varieties}
              onChange={(existingVariety) => onChange({ ...draft, existingVariety })}
            />
          )}
        </>
      ) : null}

      {draft.kind === "UseLanguage" ? (
        <>
          <CommunitySelect
            label="Community"
            value={draft.community}
            communities={snapshot.communities}
            onChange={(community) => onChange({ ...draft, community })}
          />
          <VarietySelect
            label="Language"
            value={draft.variety}
            varieties={snapshot.varieties}
            onChange={(nextVariety) => onChange({ ...draft, variety: nextVariety })}
          />
          <label className="field">
            <span>Domain</span>
            <select
              value={draft.domain}
              onChange={(event) => onChange({ ...draft, domain: event.target.value as UseDomain })}
            >
              {USE_DOMAINS.map((domain) => (
                <option key={domain} value={domain}>
                  {domain}
                </option>
              ))}
            </select>
          </label>
        </>
      ) : null}

      {draft.kind === "Separate" ? (
        <>
          <VarietySelect
            label="Parent language"
            value={draft.parent}
            varieties={snapshot.varieties}
            onChange={(parent) => onChange({ ...draft, parent })}
          />
          <label className="field">
            <span>Daughter language name</span>
            <input
              value={draft.name}
              onChange={(event) => onChange({ ...draft, name: event.target.value })}
              autoComplete="off"
            />
          </label>
          <CommunitySelect
            label="Community"
            value={draft.community}
            communities={snapshot.communities}
            onChange={(community) => onChange({ ...draft, community })}
          />
        </>
      ) : null}

      {draft.kind === "Develop" ? (
        <>
          <VarietySelect
            label="Language"
            value={draft.variety}
            varieties={snapshot.varieties}
            onChange={(nextVariety) => onChange({ ...draft, variety: nextVariety })}
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
          <VarietySelect
            label="Donor"
            value={draft.donor}
            varieties={snapshot.varieties}
            onChange={(donor) => onChange({ ...draft, donor })}
          />
          <VarietySelect
            label="Recipient"
            value={draft.recipient}
            varieties={snapshot.varieties}
            onChange={(recipient) => onChange({ ...draft, recipient })}
          />
          <label className="field">
            <span>Domain</span>
            <select
              value={draft.contactDomain}
              onChange={(event) => onChange({ ...draft, contactDomain: event.target.value as ContactDomain })}
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

      {draft.kind === "Derive" ? (
        <>
          <VarietySelect
            label="Language"
            value={draft.variety}
            varieties={snapshot.varieties}
            onChange={(nextVariety) => {
              const next = snapshot.varieties.find((item) => item.id === nextVariety);
              const first = next?.lexicon.find((item) => !item.retired);
              onChange({
                ...draft,
                variety: nextVariety,
                base: first?.id ?? "",
                sense: first?.senses[0]?.id ?? 0,
                construction: "",
              });
            }}
          />
          <LexemeSelect
            label="Base"
            value={draft.base}
            lexemes={hostLexemes}
            onChange={(base) => {
              const word = variety?.lexicon.find((item) => item.id === base);
              onChange({ ...draft, base, sense: word?.senses[0]?.id ?? 0, construction: "" });
            }}
          />
          <SenseSelect
            senses={baseLexeme?.senses ?? []}
            value={draft.sense}
            onChange={(nextSense) => onChange({ ...draft, sense: nextSense, construction: "" })}
          />
          <ConstructionSelect
            formations={formations}
            value={construction}
            onChange={(nextConstruction) => onChange({ ...draft, construction: nextConstruction })}
          />
        </>
      ) : null}

      {draft.kind === "Sense" ? (
        <SenseFields draft={draft} snapshot={snapshot} variety={variety} onChange={onChange} />
      ) : null}

      {draft.kind === "Replace" ? (
        <>
          <VarietySelect
            label="Language"
            value={draft.variety}
            varieties={snapshot.varieties}
            onChange={(nextVariety) => {
              const next = snapshot.varieties.find((item) => item.id === nextVariety);
              const first = next?.lexicon.find((item) => !item.retired);
              const replacement =
                next?.lexicon.find(
                  (item) => !item.retired && item.id !== first?.id && item.classId === first?.classId,
                ) ?? null;
              onChange({
                ...draft,
                variety: nextVariety,
                lexeme: first?.id ?? "",
                replacement: replacement?.id ?? "",
              });
            }}
          />
          <LexemeSelect
            label="Retire"
            value={draft.lexeme}
            lexemes={hostLexemes}
            onChange={(nextLexeme) => {
              const word = variety?.lexicon.find((item) => item.id === nextLexeme);
              const replacement =
                variety?.lexicon.find(
                  (item) => !item.retired && item.id !== nextLexeme && item.classId === word?.classId,
                ) ?? null;
              onChange({ ...draft, lexeme: nextLexeme, replacement: replacement?.id ?? "" });
            }}
          />
          <LexemeSelect
            label="Replacement"
            value={draft.replacement}
            lexemes={hostLexemes.filter(
              (item) => item.id !== draft.lexeme && item.classId === targetLexeme?.classId,
            )}
            onChange={(replacement) => onChange({ ...draft, replacement })}
          />
        </>
      ) : null}

      {draft.kind === "Lexicalize" ? (
        <>
          <VarietySelect
            label="Language"
            value={draft.variety}
            varieties={snapshot.varieties}
            onChange={(nextVariety) => {
              const next = snapshot.varieties.find((item) => item.id === nextVariety);
              const first = next?.lexicon.find((item) => !item.retired && item.analysis);
              onChange({ ...draft, variety: nextVariety, lexeme: first?.id ?? "" });
            }}
          />
          <LexemeSelect
            label="Word"
            value={draft.lexeme}
            lexemes={hostLexemes.filter((item) => item.analysis)}
            onChange={(nextLexeme) => onChange({ ...draft, lexeme: nextLexeme })}
          />
        </>
      ) : null}

      {draft.kind === "Remodel" ? (
        <>
          <VarietySelect
            label="Language"
            value={draft.variety}
            varieties={snapshot.varieties}
            onChange={(nextVariety) => {
              const next = snapshot.varieties.find((item) => item.id === nextVariety);
              const first = next?.lexicon.find((item) => !item.retired);
              onChange({
                ...draft,
                variety: nextVariety,
                lexeme: first?.id ?? "",
                base: first?.analysis?.base ?? first?.id ?? "",
                sense: first?.senses[0]?.id ?? 0,
                construction: "",
              });
            }}
          />
          <LexemeSelect
            label="Target"
            value={draft.lexeme}
            lexemes={hostLexemes}
            onChange={(nextLexeme) => onChange({ ...draft, lexeme: nextLexeme })}
          />
          <LexemeSelect
            label="Base"
            value={draft.base}
            lexemes={hostLexemes}
            onChange={(base) => {
              const word = variety?.lexicon.find((item) => item.id === base);
              onChange({ ...draft, base, sense: word?.senses[0]?.id ?? 0, construction: "" });
            }}
          />
          <SenseSelect
            senses={baseLexeme?.senses ?? []}
            value={draft.sense}
            onChange={(nextSense) => onChange({ ...draft, sense: nextSense, construction: "" })}
          />
          <ConstructionSelect
            formations={formations}
            value={construction}
            onChange={(nextConstruction) => onChange({ ...draft, construction: nextConstruction })}
          />
        </>
      ) : null}

      {error ? <p className="error">{error}</p> : null}
    </form>
  );
}

function SenseFields({
  draft,
  snapshot,
  variety,
  onChange,
}: {
  draft: EventDraft;
  snapshot: Snapshot;
  variety: Variety | undefined;
  onChange: (draft: EventDraft) => void;
}) {
  const hostLexemes = (variety?.lexicon ?? []).filter((item) => !item.retired);
  const word = variety?.lexicon.find((item) => item.id === draft.lexeme);
  const current = word?.senses.find((item) => item.id === draft.sense) ?? word?.senses[0];
  const classKind = variety?.classes.find((item) => item.id === word?.classId)?.kind ?? draft.frameKind;

  function applyLexeme(nextLexeme: string, nextVariety = draft.variety) {
    const host = snapshot.varieties.find((item) => item.id === nextVariety);
    const nextWord = host?.lexicon.find((item) => item.id === nextLexeme);
    const kind = host?.classes.find((item) => item.id === nextWord?.classId)?.kind ?? "Entity";
    const nextSense = nextWord?.senses[0];
    onChange({
      ...draft,
      variety: nextVariety,
      lexeme: nextLexeme,
      sense: nextSense?.id ?? 0,
      frameKind: kind,
      countable:
        kind === "Entity" && draft.senseMode === "shift" && nextSense && "Entity" in nextSense.frame
          ? nextSense.frame.Entity.countable
          : true,
      roles:
        kind === "Event" && draft.senseMode === "shift" && nextSense && "Event" in nextSense.frame
          ? nextSense.frame.Event.roles.filter((role) => ROLES.includes(role))
          : [],
      gloss: draft.senseMode === "shift" ? (nextSense?.gloss ?? "") : draft.gloss,
    });
  }

  function applyShiftFrom(senseId: number) {
    const nextSense = word?.senses.find((item) => item.id === senseId) ?? current;
    const frame = nextSense?.frame;
    onChange({
      ...draft,
      senseMode: "shift",
      sense: senseId,
      gloss: nextSense?.gloss ?? "",
      frameKind: classKind,
      countable: frame && "Entity" in frame ? frame.Entity.countable : true,
      roles: frame && "Event" in frame ? frame.Event.roles.filter((role) => ROLES.includes(role)) : [],
    });
  }

  return (
    <>
      <VarietySelect
        label="Language"
        value={draft.variety}
        varieties={snapshot.varieties}
        onChange={(nextVariety) => {
          const next = snapshot.varieties.find((item) => item.id === nextVariety);
          applyLexeme(next?.lexicon.find((item) => !item.retired)?.id ?? "", nextVariety);
        }}
      />
      <LexemeSelect
        label="Word"
        value={draft.lexeme}
        lexemes={hostLexemes}
        onChange={(nextLexeme) => applyLexeme(nextLexeme)}
      />
      <fieldset className="field">
        <legend>Change</legend>
        <label className="choice">
          <input
            type="radio"
            name="sense-mode"
            checked={draft.senseMode === "extend"}
            onChange={() => onChange({ ...draft, senseMode: "extend", gloss: "", countable: true, roles: [] })}
          />
          Add a sense
        </label>
        <label className="choice">
          <input
            type="radio"
            name="sense-mode"
            checked={draft.senseMode === "shift"}
            onChange={() => applyShiftFrom(current?.id ?? draft.sense)}
          />
          Shift this sense
        </label>
      </fieldset>
      {draft.senseMode === "shift" ? (
        <SenseSelect
          senses={word?.senses ?? []}
          value={draft.sense}
          onChange={(nextSense) => applyShiftFrom(nextSense)}
        />
      ) : null}
      <label className="field">
        <span>Gloss</span>
        <input
          value={draft.gloss}
          maxLength={GLOSS_MAX}
          onChange={(event) => onChange({ ...draft, gloss: event.target.value })}
          autoComplete="off"
        />
      </label>
      {classKind === "Event" ? (
        <fieldset className="field">
          <legend>Event roles</legend>
          {ROLES.map((role) => (
            <label className="choice" key={role}>
              <input
                type="checkbox"
                checked={draft.roles.includes(role)}
                onChange={() =>
                  onChange({
                    ...draft,
                    frameKind: "Event",
                    roles: ROLES.filter((item) => (item === role ? !draft.roles.includes(item) : draft.roles.includes(item))),
                  })
                }
              />
              {role}
            </label>
          ))}
        </fieldset>
      ) : (
        <label className="choice">
          <input
            type="checkbox"
            checked={draft.countable}
            onChange={(event) =>
              onChange({ ...draft, frameKind: "Entity", countable: event.target.checked })
            }
          />
          Countable
        </label>
      )}
    </>
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

function VarietySelect({
  label,
  value,
  varieties,
  onChange,
}: {
  label: string;
  value: number;
  varieties: Variety[];
  onChange: (id: number) => void;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <select
        value={value}
        disabled={varieties.length === 0}
        onChange={(event) => onChange(Number(event.target.value))}
      >
        {varieties.length === 0 ? <option value={0}>None</option> : null}
        {varieties.map((item) => (
          <option key={item.id} value={item.id}>
            {item.name}
          </option>
        ))}
      </select>
    </label>
  );
}

function LexemeSelect({
  label,
  value,
  lexemes,
  onChange,
}: {
  label: string;
  value: string;
  lexemes: Lexeme[];
  onChange: (id: string) => void;
}) {
  return (
    <label className="field">
      <span>{label}</span>
      <select
        value={value}
        disabled={lexemes.length === 0}
        onChange={(event) => onChange(event.target.value)}
      >
        {lexemes.length === 0 ? <option value="">None</option> : null}
        {lexemes.map((item) => (
          <option key={item.id} value={item.id}>
            {item.form} · {item.senses.map((sense) => sense.gloss).join(", ") || item.id}
          </option>
        ))}
      </select>
    </label>
  );
}

function SenseSelect({
  senses,
  value,
  onChange,
}: {
  senses: { id: number; gloss: string }[];
  value: number;
  onChange: (id: number) => void;
}) {
  return (
    <label className="field">
      <span>Sense</span>
      <select
        value={Number.isFinite(value) ? value : ""}
        disabled={senses.length === 0}
        onChange={(event) => onChange(Number(event.target.value))}
      >
        {senses.length === 0 ? <option value="">None</option> : null}
        {senses.map((item) => (
          <option key={item.id} value={item.id}>
            {item.gloss}
          </option>
        ))}
      </select>
    </label>
  );
}

function ConstructionSelect({
  formations,
  value,
  onChange,
}: {
  formations: FormationOption[];
  value: string;
  onChange: (id: string) => void;
}) {
  return (
    <label className="field">
      <span>Construction</span>
      <select
        value={value}
        disabled={formations.length === 0}
        onChange={(event) => onChange(event.target.value)}
      >
        {formations.length === 0 ? <option value="">None eligible</option> : null}
        {formations.map((item) => (
          <option key={item.construction} value={item.construction}>
            {item.label} → {item.form}
            {item.existing ? " (existing)" : ""}
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
  const varieties = pending.snapshot.varieties;
  return (
    <div className="stack">
      <p className="lead">{pending.snapshot.summary}</p>
      <p className="muted">
        Involved {involvedNames(pending.event, pending.snapshot).join(", ") || "none yet"}
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
          <p className="muted">No individual words change.</p>
        ) : (
          <ul className="effects">
            {pending.snapshot.effects.map((effect, index) => {
              const host =
                varieties.find((item) => item.id === effect.variety)?.name ?? `Variety ${effect.variety}`;
              const pattern = effect.lexeme === "";
              return (
                <li
                  key={`effect-${index}-${effect.variety}-${effect.lexeme}-${effect.before}-${effect.after}`}
                  className={pattern ? "effect-pattern" : undefined}
                >
                  <span className="effect-head">
                    {pattern
                      ? `${host} · ${effect.gloss || "system"} · ${effect.before ?? "—"} → ${effect.after}`
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
    group: "",
    location: "",
    ancestry: "Unspecified",
    languageMode: "New",
    languageName: "",
    aesthetic: "elvish",
    existingVariety: 0,
    parent: 0,
    community: 0,
    variety: 0,
    domain: "Home",
    donor: 0,
    recipient: 0,
    steps: 1,
    count: 8,
    contactDomain: "Maritime",
    lexeme: "",
    sense: 0,
    construction: "",
    gloss: "",
    senseMode: "extend",
    frameKind: "Entity",
    countable: true,
    roles: [],
    replacement: "",
    base: "",
  };
}

function draftFrom(
  kind: EventKind,
  snapshot: Snapshot,
  communityId: number | null,
  varietyId: number | null,
  lexemeId: string | null,
  senseId: number | null,
): EventDraft {
  const selectedCommunity = communityId ?? snapshot.communities[0]?.id ?? 0;
  const hostCommunity = snapshot.communities.find((item) => item.id === selectedCommunity);
  const selectedVariety =
    varietyId ?? hostCommunity?.uses[0]?.variety ?? snapshot.varieties[0]?.id ?? 0;
  const otherVariety = snapshot.varieties.find((item) => item.id !== selectedVariety)?.id ?? selectedVariety;
  const host = snapshot.varieties.find((item) => item.id === selectedVariety);
  const lexeme =
    host?.lexicon.find((item) => item.id === lexemeId && !item.retired) ??
    host?.lexicon.find((item) => !item.retired);
  const sense = lexeme?.senses.find((item) => item.id === senseId) ?? lexeme?.senses[0];
  const classKind = host?.classes.find((item) => item.id === lexeme?.classId)?.kind ?? "Entity";
  const replacement =
    host?.lexicon.find((item) => !item.retired && item.id !== lexeme?.id && item.classId === lexeme?.classId) ??
    null;
  const lexicalized = host?.lexicon.find((item) => !item.retired && item.analysis);
  return {
    ...emptyDraft(),
    kind,
    aesthetic: host?.aesthetic.id ?? "elvish",
    existingVariety: selectedVariety,
    parent: selectedVariety,
    community: selectedCommunity,
    variety: selectedVariety,
    donor: otherVariety,
    recipient: selectedVariety,
    lexeme: (kind === "Lexicalize" ? lexicalized?.id : lexeme?.id) ?? "",
    sense: sense?.id ?? 0,
    base: lexeme?.analysis?.base ?? lexeme?.id ?? "",
    replacement: replacement?.id ?? "",
    frameKind: classKind,
    languageName: "",
  };
}

function eventFromDraft(draft: EventDraft, snapshot: Snapshot, formations: FormationOption[]): HistoryEvent {
  const construction =
    formations.find((item) => item.construction === draft.construction)?.construction ??
    formations[0]?.construction ??
    draft.construction;
  if (draft.kind === "Found") {
    return {
      Found: {
        name: draft.name.trim(),
        group: draft.group.trim(),
        location: draft.location.trim(),
        ancestry: draft.ancestry.trim() || "Unspecified",
        language:
          draft.languageMode === "Existing"
            ? { Existing: { variety: draft.existingVariety } }
            : { New: { name: draft.languageName.trim(), aesthetic: draft.aesthetic } },
      },
    };
  }
  if (draft.kind === "UseLanguage") {
    return { UseLanguage: { community: draft.community, variety: draft.variety, domain: draft.domain } };
  }
  if (draft.kind === "Separate") {
    return { Separate: { parent: draft.parent, name: draft.name.trim(), community: draft.community } };
  }
  if (draft.kind === "Develop") return { Develop: { variety: draft.variety, steps: draft.steps } };
  if (draft.kind === "Contact") {
    return {
      Contact: {
        donor: draft.donor,
        recipient: draft.recipient,
        domain: draft.contactDomain,
        count: draft.count,
      },
    };
  }
  if (draft.kind === "Derive") {
    return { Derive: { variety: draft.variety, base: draft.base, sense: draft.sense, construction } };
  }
  if (draft.kind === "Sense") {
    const host = snapshot.varieties.find((item) => item.id === draft.variety);
    const word = host?.lexicon.find((item) => item.id === draft.lexeme);
    const classKind = host?.classes.find((item) => item.id === word?.classId)?.kind ?? draft.frameKind;
    const frame: SemanticFrame =
      classKind === "Event"
        ? { Event: { roles: ROLES.filter((role) => draft.roles.includes(role)) } }
        : { Entity: { countable: draft.countable } };
    if (draft.senseMode === "shift") {
      return {
        ShiftSense: {
          variety: draft.variety,
          lexeme: draft.lexeme,
          sense: draft.sense,
          gloss: draft.gloss.trim(),
          frame,
        },
      };
    }
    return { ExtendSense: { variety: draft.variety, lexeme: draft.lexeme, gloss: draft.gloss.trim(), frame } };
  }
  if (draft.kind === "Replace") {
    return { Replace: { variety: draft.variety, lexeme: draft.lexeme, replacement: draft.replacement } };
  }
  if (draft.kind === "Lexicalize") return { Lexicalize: { variety: draft.variety, lexeme: draft.lexeme } };
  return {
    Remodel: {
      variety: draft.variety,
      lexeme: draft.lexeme,
      base: draft.base,
      sense: draft.sense,
      construction,
    },
  };
}

function validateDraft(draft: EventDraft, snapshot: Snapshot, formations: FormationOption[]): string | null {
  if (draft.kind === "Found") {
    if (draft.name.trim().length === 0) return "Name the community.";
    if (draft.group.trim().length === 0) return "Name the group.";
    if (draft.location.trim().length === 0) return "Name the location.";
    if (draft.languageMode === "New" && draft.languageName.trim().length === 0) return "Name the new language.";
    if (draft.languageMode === "Existing" && !snapshot.varieties.some((item) => item.id === draft.existingVariety)) {
      return "Choose an existing language to share.";
    }
  }
  if (draft.kind === "UseLanguage") {
    if (snapshot.communities.length === 0) return "Found a community before attaching a language.";
    if (snapshot.varieties.length === 0) return "Found a language before attaching one.";
    const host = snapshot.communities.find((item) => item.id === draft.community);
    if (!host) return "Choose a community.";
    if (!snapshot.varieties.some((item) => item.id === draft.variety)) return "Choose a language.";
    if (host.uses.some((use) => use.variety === draft.variety && use.domain === draft.domain)) {
      return "This community already uses that language in this domain.";
    }
  }
  if (draft.kind === "Separate") {
    if (snapshot.varieties.length === 0) return "Found a language before separating.";
    if (snapshot.communities.length === 0) return "Found a community before separating.";
    if (draft.name.trim().length === 0) return "Name the daughter language.";
  }
  if (draft.kind === "Develop") {
    if (snapshot.varieties.length === 0) return "Found a language before developing.";
    if (!Number.isInteger(draft.steps) || draft.steps < 1) return "Development needs at least one step.";
  }
  if (draft.kind === "Contact") {
    if (snapshot.varieties.length < 2) return "Contact needs two languages.";
    if (draft.donor === draft.recipient) return "Donor and recipient must differ.";
    if (!Number.isInteger(draft.count) || draft.count < 1) return "Loan count must be at least 1.";
  }
  if (draft.kind === "Derive" || draft.kind === "Remodel") {
    const host = snapshot.varieties.find((item) => item.id === draft.variety);
    if (!host) return "Choose a language.";
    const base = host.lexicon.find((item) => item.id === draft.base);
    if (!base || base.retired) return "Choose an active base word.";
    if (!base.senses.some((item) => item.id === draft.sense)) return "Choose a sense on the base word.";
    const construction =
      formations.find((item) => item.construction === draft.construction) ?? formations[0] ?? null;
    if (!construction) return "No eligible construction for that base and sense.";
    if (draft.kind === "Derive" && construction.existing) {
      return "That formation is already in the lexicon. Open it from the inspector.";
    }
    if (draft.kind === "Remodel") {
      const target = host.lexicon.find((item) => item.id === draft.lexeme);
      if (!target || target.retired) return "Choose an active word to remodel.";
    }
  }
  if (draft.kind === "Sense") {
    const host = snapshot.varieties.find((item) => item.id === draft.variety);
    const word = host?.lexicon.find((item) => item.id === draft.lexeme);
    if (!word || word.retired) return "Choose an active word.";
    const classKind = host?.classes.find((item) => item.id === word.classId)?.kind;
    if (!classKind) return "This word has no lexical class.";
    const gloss = draft.gloss.trim();
    if (gloss.length === 0) return "Give the sense a gloss.";
    if (gloss.length > GLOSS_MAX) return `Gloss must be at most ${GLOSS_MAX} characters.`;
    const frame: SemanticFrame =
      classKind === "Event"
        ? { Event: { roles: ROLES.filter((role) => draft.roles.includes(role)) } }
        : { Entity: { countable: draft.countable } };
    if (draft.senseMode === "shift") {
      const current = word.senses.find((item) => item.id === draft.sense);
      if (!current) return "Choose the sense to shift.";
      const sameGloss = current.gloss === gloss;
      const sameFrame =
        ("Entity" in current.frame && "Entity" in frame && current.frame.Entity.countable === frame.Entity.countable) ||
        ("Event" in current.frame &&
          "Event" in frame &&
          current.frame.Event.roles.length === frame.Event.roles.length &&
          current.frame.Event.roles.every((role, index) => role === frame.Event.roles[index]));
      if (sameGloss && sameFrame) return "Shift needs a different gloss or frame.";
    }
  }
  if (draft.kind === "Replace") {
    const host = snapshot.varieties.find((item) => item.id === draft.variety);
    const source = host?.lexicon.find((item) => item.id === draft.lexeme);
    const target = host?.lexicon.find((item) => item.id === draft.replacement);
    if (!source || source.retired) return "Choose an active word to retire.";
    if (!target || target.retired) return "Choose an active replacement.";
    if (source.id === target.id) return "Replacement must be a different word.";
    if (source.classId !== target.classId) return "Replacement must share the same lexical class.";
  }
  if (draft.kind === "Lexicalize") {
    const host = snapshot.varieties.find((item) => item.id === draft.variety);
    const word = host?.lexicon.find((item) => item.id === draft.lexeme);
    if (!word || word.retired) return "Choose an active word.";
    if (!word.analysis) return "That word has no current analysis to lose.";
  }
  return null;
}

function shortEvent(event: HistoryEvent | null): string {
  if (!event) return "Begin";
  if ("Found" in event) return `Found ${event.Found.name}`;
  if ("UseLanguage" in event) return `Use ${event.UseLanguage.domain}`;
  if ("Separate" in event) return `Split ${event.Separate.name}`;
  if ("Develop" in event) return `Develop ${event.Develop.steps}`;
  if ("Contact" in event) return `Contact ${event.Contact.domain}`;
  if ("Derive" in event) return "Form word";
  if ("ExtendSense" in event) return "Add sense";
  if ("ShiftSense" in event) return "Shift sense";
  if ("Replace" in event) return "Replace word";
  if ("Lexicalize" in event) return "Lose analysis";
  if ("Remodel" in event) return "Remodel word";
  return "Event";
}

function involvedNames(event: HistoryEvent, snapshot: Snapshot): string[] {
  const names: string[] = [];
  const community = (id: number) =>
    snapshot.communities.find((item) => item.id === id)?.name ?? `Community ${id}`;
  const variety = (id: number) => snapshot.varieties.find((item) => item.id === id)?.name ?? `Variety ${id}`;
  if ("Found" in event) {
    names.push(event.Found.name);
    if ("New" in event.Found.language) names.push(event.Found.language.New.name);
    if ("Existing" in event.Found.language) names.push(variety(event.Found.language.Existing.variety));
  }
  if ("UseLanguage" in event) names.push(community(event.UseLanguage.community), variety(event.UseLanguage.variety));
  if ("Separate" in event) {
    names.push(variety(event.Separate.parent), event.Separate.name, community(event.Separate.community));
  }
  if ("Develop" in event) names.push(variety(event.Develop.variety));
  if ("Contact" in event) names.push(variety(event.Contact.donor), variety(event.Contact.recipient));
  if ("Derive" in event) names.push(variety(event.Derive.variety));
  if ("ExtendSense" in event) names.push(variety(event.ExtendSense.variety));
  if ("ShiftSense" in event) names.push(variety(event.ShiftSense.variety));
  if ("Replace" in event) names.push(variety(event.Replace.variety));
  if ("Lexicalize" in event) names.push(variety(event.Lexicalize.variety));
  if ("Remodel" in event) names.push(variety(event.Remodel.variety));
  return names.filter((name, index) => names.indexOf(name) === index);
}

function settleAfterEvent(
  event: HistoryEvent,
  previous: Snapshot,
  next: Snapshot,
  communityId: number | null,
  setCommunityId: (id: number | null) => void,
  setVarietyId: (id: number | null) => void,
  setLexemeId: (id: string | null) => void,
  setSenseId: (id: number | null) => void,
) {
  const selectVariety = (varietyId: number, lexeme: Lexeme | null) => {
    const host = communityUsing(next, varietyId, communityId);
    if (host) setCommunityId(host.id);
    setVarietyId(varietyId);
    setLexemeId(lexeme?.id ?? null);
    setSenseId(lexeme?.senses[0]?.id ?? null);
  };

  if ("Found" in event) {
    const born = next.communities.find((item) => !previous.communities.some((old) => old.id === item.id));
    if (born) {
      setCommunityId(born.id);
      const language = event.Found.language;
      const varietyId =
        "Existing" in language ? language.Existing.variety : (born.uses[0]?.variety ?? null);
      setVarietyId(varietyId);
      const host = next.varieties.find((item) => item.id === varietyId);
      const word = host?.lexicon.find((item) => !item.retired) ?? host?.lexicon[0] ?? null;
      setLexemeId(word?.id ?? null);
      setSenseId(word?.senses[0]?.id ?? null);
    }
    return;
  }
  if ("UseLanguage" in event) {
    setCommunityId(event.UseLanguage.community);
    setVarietyId(event.UseLanguage.variety);
    const host = next.varieties.find((item) => item.id === event.UseLanguage.variety);
    const word = host?.lexicon.find((item) => !item.retired) ?? host?.lexicon[0] ?? null;
    setLexemeId(word?.id ?? null);
    setSenseId(word?.senses[0]?.id ?? null);
    return;
  }
  if ("Separate" in event) {
    const daughter = next.varieties.find((item) => !previous.varieties.some((old) => old.id === item.id));
    setCommunityId(event.Separate.community);
    setVarietyId(daughter?.id ?? event.Separate.parent);
    const host = next.varieties.find((item) => item.id === (daughter?.id ?? event.Separate.parent));
    const word = host?.lexicon.find((item) => !item.retired) ?? host?.lexicon[0] ?? null;
    setLexemeId(word?.id ?? null);
    setSenseId(word?.senses[0]?.id ?? null);
    return;
  }
  if ("Develop" in event) {
    const host = communityUsing(next, event.Develop.variety, communityId);
    if (host) setCommunityId(host.id);
    setVarietyId(event.Develop.variety);
    return;
  }
  if ("Contact" in event) {
    const host = communityUsing(next, event.Contact.recipient, communityId);
    if (host) setCommunityId(host.id);
    setVarietyId(event.Contact.recipient);
    return;
  }
  if ("Derive" in event) {
    const host = next.varieties.find((item) => item.id === event.Derive.variety);
    const effectId = next.effects.find((effect) => effect.variety === event.Derive.variety && effect.lexeme !== "")?.lexeme;
    const created =
      host?.lexicon.find(
        (item) =>
          item.analysis?.base === event.Derive.base &&
          item.analysis.construction === event.Derive.construction &&
          item.analysis.sense === event.Derive.sense &&
          !item.retired,
      ) ?? host?.lexicon.find((item) => item.id === effectId) ?? null;
    selectVariety(event.Derive.variety, created);
    return;
  }
  if ("ExtendSense" in event) {
    const host = next.varieties.find((item) => item.id === event.ExtendSense.variety);
    const word = host?.lexicon.find((item) => item.id === event.ExtendSense.lexeme) ?? null;
    const added = word?.senses.find((item) => item.gloss === event.ExtendSense.gloss);
    let addedId = added?.id ?? null;
    if (addedId == null && word) {
      for (const item of word.senses) {
        if (addedId == null || item.id > addedId) addedId = item.id;
      }
    }
    const speaker = communityUsing(next, event.ExtendSense.variety, communityId);
    if (speaker) setCommunityId(speaker.id);
    setVarietyId(event.ExtendSense.variety);
    setLexemeId(word?.id ?? null);
    setSenseId(addedId);
    return;
  }
  if ("ShiftSense" in event) {
    const host = next.varieties.find((item) => item.id === event.ShiftSense.variety);
    const word = host?.lexicon.find((item) => item.id === event.ShiftSense.lexeme) ?? null;
    const speaker = communityUsing(next, event.ShiftSense.variety, communityId);
    if (speaker) setCommunityId(speaker.id);
    setVarietyId(event.ShiftSense.variety);
    setLexemeId(word?.id ?? null);
    setSenseId(event.ShiftSense.sense);
    return;
  }
  if ("Replace" in event) {
    const host = next.varieties.find((item) => item.id === event.Replace.variety);
    const word = host?.lexicon.find((item) => item.id === event.Replace.replacement) ?? null;
    selectVariety(event.Replace.variety, word);
    return;
  }
  if ("Lexicalize" in event) {
    const host = next.varieties.find((item) => item.id === event.Lexicalize.variety);
    const word = host?.lexicon.find((item) => item.id === event.Lexicalize.lexeme) ?? null;
    selectVariety(event.Lexicalize.variety, word);
    return;
  }
  if ("Remodel" in event) {
    const host = next.varieties.find((item) => item.id === event.Remodel.variety);
    const word = host?.lexicon.find((item) => item.id === event.Remodel.lexeme) ?? null;
    selectVariety(event.Remodel.variety, word);
  }
}

function usedVarieties(snapshot: Snapshot, community: Community): Variety[] {
  const seen: Record<number, true> = {};
  const out: Variety[] = [];
  for (const use of community.uses) {
    if (seen[use.variety]) continue;
    seen[use.variety] = true;
    const variety = snapshot.varieties.find((item) => item.id === use.variety);
    if (variety) out.push(variety);
  }
  return out;
}

function communityUsing(snapshot: Snapshot, varietyId: number, preferred: number | null): Community | undefined {
  const preferredCommunity = snapshot.communities.find((item) => item.id === preferred);
  if (preferredCommunity?.uses.some((use) => use.variety === varietyId)) return preferredCommunity;
  return snapshot.communities.find((item) => item.uses.some((use) => use.variety === varietyId));
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

import { useEffect, useMemo, useRef, useState } from "react";
import type { NotebookNote, Overview, Subject } from "../model";
import { YEARS } from "../model";
import { searchText } from "../history";
import { Modal } from "./Modal";

export function makeNote(overview: Overview, subject: Subject | null, label: string, kind: NotebookNote["kind"] = "observation"): NotebookNote {
  return { id: crypto.randomUUID(), title: kind === "year" ? `Year ${overview.generation * YEARS}` : label,
    body: "", kind, target: subject ? { reading: { telling: overview.telling, point: overview.point }, subject } : null,
    label, generation: overview.generation, revision: overview.revision, archived: false };
}

export function Notebook({ open, notes, overview, initial, onClose, onSave, onRead }: {
  open: boolean;
  notes: NotebookNote[];
  overview: Overview;
  initial: NotebookNote | null;
  onClose: () => void;
  onSave: (note: NotebookNote) => string | null;
  onRead: (note: NotebookNote) => string | null;
}) {
  const [draft, setDraft] = useState<NotebookNote | null>(initial);
  const [query, setQuery] = useState("");
  const [archived, setArchived] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const entry = useRef<HTMLElement>(null);
  useEffect(() => { entry.current?.querySelector<HTMLInputElement>("input")?.focus(); }, [draft?.id]);
  const original = notes.find((note) => note.id === draft?.id);
  const dirty = draft !== null && JSON.stringify(draft) !== JSON.stringify(original);
  const shown = useMemo(() => notes.filter((note) => note.archived === archived &&
    searchText(`${note.title} ${note.body} ${note.label}`).includes(searchText(query))).reverse(), [notes, archived, query]);
  const choose = (note: NotebookNote) => { setDraft(note); setNotice(null); setError(null); };
  const change = (patch: Partial<NotebookNote>) => { setDraft((note) => note && { ...note, ...patch }); setNotice(null); setError(null); };
  const save = () => {
    if (!draft) return;
    const kept = { ...draft, title: draft.title.trim() };
    const failure = onSave(kept);
    setDraft(kept);
    setError(failure); setNotice(failure ? null : "Kept with this world's notes.");
  };
  return <Modal open={open} wide title="Notes" initialFocus={draft ? ".notebook-entry input" : undefined} onClose={() => {
    if (dirty) setError("Keep this note or discard its edits before closing.");
    else onClose();
  }}>
    <div className="notebook">
      <aside className="notebook-index">
        <p className="muted small">Notes, questions, and years worth returning to. Every note travels with the saved world.</p>
        <div className="row notebook-new">
          <button type="button" disabled={dirty} onClick={() => choose(makeNote(overview, null, "A question", "question"))}>Ask a question</button>
          <button type="button" disabled={dirty} onClick={() => choose(makeNote(overview, { kind: "world" }, `Year ${overview.generation * YEARS}`, "year"))}>Name this year</button>
        </div>
        <label>Find a note<input type="search" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="A name, a question, a note…" /></label>
        <label className="notebook-archive"><input type="checkbox" checked={archived} onChange={(e) => setArchived(e.target.checked)} /> Read archived entries</label>
        <ol>{shown.map((note) => <li key={note.id}><button type="button" disabled={dirty} aria-pressed={draft?.id === note.id} onClick={() => choose(note)}>
          <small>{note.kind === "year" ? "Named year" : note.kind} · year {note.generation * YEARS}</small><strong>{note.title}</strong>
          {note.body ? <span>{note.body.slice(0, 140)}{note.body.length > 140 ? "…" : ""}</span> : null}
        </button></li>)}</ol>
        {!shown.length ? <p className="muted">{query ? "No notes match this search." : archived ? "No archived notes." : "Note something from any card, or begin with a question."}</p> : null}
      </aside>
      <section className="notebook-entry" aria-label="Note" ref={entry}>
        {draft ? <>
          <p className="eyebrow">{original ? "A note" : "A new note"}</p>
          <label>Entry title<input value={draft.title} onChange={(e) => change({ title: e.target.value })} /></label>
          <label>Kind of entry<select value={draft.kind} onChange={(e) => change({ kind: e.target.value as NotebookNote["kind"] })}>
            <option value="observation">An observation</option><option value="question">An open question</option><option value="year">A named year</option>
          </select></label>
          <label>Your notes<textarea rows={9} value={draft.body} onChange={(e) => change({ body: e.target.value })} placeholder="What caught your attention? What might be worth comparing later?" /></label>
          {draft.target ? <div className="notebook-reference">
            <p><strong>{draft.label}</strong><br />{overview.tellings.find((t) => t.id === draft.target?.reading.telling)?.name ?? "An unavailable telling"} · year {draft.generation * YEARS}</p>
            {draft.revision !== overview.revision ? <p className="notice">The original reference needs checking after an engine change. Your words are still kept.</p> : null}
            <button type="button" disabled={!original || dirty} onClick={() => setError(onRead(draft))}>Return to this year</button>
          </div> : <p className="muted small">A question for the whole world, without a fixed year.</p>}
          {error ? <p className="notice error" role="alert">{error}</p> : null}
          {notice ? <p className="notice" role="status">{notice}</p> : null}
          <div className="row notebook-actions">
            <button type="button" className="primary" disabled={!dirty || !draft.title.trim()} onClick={save}>Keep this entry</button>
            {dirty ? <button type="button" onClick={() => { setDraft(original ?? null); setError(null); }}>Discard these edits</button> : null}
            {original && !dirty ? <button type="button" onClick={() => {
              const next = { ...draft, archived: !draft.archived };
              const failure = onSave(next); setError(failure);
              if (!failure) { setDraft(null); setNotice(null); }
            }}>{draft.archived ? "Restore to notes" : "Archive this note"}</button> : null}
          </div>
          {dirty ? <p className="muted small">This page has unsaved edits. Keep the entry before choosing another.</p> : null}
        </> : <div className="notebook-empty"><h3>A place for what you notice</h3><p>Choose a note to return to its year, or begin a new note.</p></div>}
      </section>
    </div>
  </Modal>;
}

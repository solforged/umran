import { useEffect, useRef, useState } from "react";
import { Bookmark } from "lucide-react";
import type { NotebookNote, Overview, Subject } from "../model";
import { YEARS } from "../model";
import "./margin.css";

export function makeNote(overview: Overview, subject: Subject | null, label: string, kind: NotebookNote["kind"] = "observation"): NotebookNote {
  return { id: crypto.randomUUID(), title: kind === "year" ? `Year ${overview.generation * YEARS}` : label,
    body: "", kind, target: subject ? { reading: { telling: overview.telling, point: overview.point }, subject } : null,
    label, generation: overview.generation, revision: overview.revision, archived: false };
}

export function Margin({ notes, overview, subject, label, onSave, onRemove, onRead, onStart }: {
  notes: NotebookNote[];
  overview: Overview;
  subject: Subject;
  label: string;
  onSave: (note: NotebookNote) => string | null;
  onRemove: (id: string) => string | null;
  onRead: (note: NotebookNote) => string | null;
  onStart: () => void;
}) {
  const [draft, setDraft] = useState<NotebookNote | null>(null);
  const [error, setError] = useState<string | null>(null);
  const body = useRef<HTMLTextAreaElement>(null);
  const tools = useRef<HTMLDivElement>(null);
  useEffect(() => { if (draft) body.current?.focus(); }, [draft?.id]);
  const shown = notes.filter((note) => subject.kind === "history" || JSON.stringify(note.target?.subject) === JSON.stringify(subject))
    .sort((a, b) => a.generation - b.generation);
  const choose = (note: NotebookNote) => { onStart(); setDraft({ ...note }); setError(null); };
  const discard = () => { setDraft(null); setError(null); tools.current?.querySelector<HTMLButtonElement>("button")?.focus(); };
  const keep = () => {
    if (!draft) return;
    const kept = { ...draft, title: draft.title.trim() };
    if (!kept.title) { setError("Give this note a title."); return; }
    const failure = onSave(kept);
    setError(failure);
    if (!failure) setDraft(null);
  };
  const editor = draft ? <form className="margin-editor" onSubmit={(event) => { event.preventDefault(); keep(); }}
    onKeyDown={(event) => { if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); discard(); } }}>
    <p className="margin-date">Year {draft.generation * YEARS}{draft.archived ? " · archived" : ""}</p>
    <input aria-label="Note title" value={draft.title} onChange={(event) => setDraft({ ...draft, title: event.currentTarget.value })} />
    <textarea ref={body} aria-label="Note body" rows={4} placeholder="Write in the margin…" value={draft.body}
      onChange={(event) => setDraft({ ...draft, body: event.currentTarget.value })} />
    <div className="margin-actions"><button type="submit" className="link">Keep</button><button type="button" className="link" onClick={discard}>Discard</button></div>
  </form> : null;
  return <section className="margin" aria-label={subject.kind === "history" ? "All notes" : "Notes on this card"}>
    {shown.map((note) => <article className="margin-note" key={note.id}>
      {draft?.id === note.id ? editor : <>
        <p className="margin-date">Year {note.generation * YEARS}{note.archived ? " · archived" : ""}</p>
        <button type="button" className="margin-text" aria-label={`Edit note: ${note.title}`} onClick={() => choose(note)}>
          <span className="margin-title">{note.title}</span>{note.body.split(/\n\s*\n/).filter((paragraph) => paragraph.trim()).map((paragraph, index) => <span className="margin-para" key={index}>{paragraph}</span>)}
        </button>
        <div className="margin-actions">
          {subject.kind === "history" && note.target ? <button type="button" className="link margin-subject" onClick={() => { onStart(); setError(onRead(note)); }}>{note.label}</button> : null}
          <button type="button" className="link" aria-label={`Remove note: ${note.title}`} onClick={() => setError(onRemove(note.id))}>Remove</button>
        </div>
      </>}
    </article>)}
    {draft && !shown.some((note) => note.id === draft.id) ? editor : null}
    {error ? <p className="margin-error" role="status">{error}</p> : null}
    <div className="margin-tools" ref={tools}>
      <button type="button" className="link" disabled={draft !== null} onClick={() => choose(makeNote(overview, subject, label))}><Bookmark size={14} aria-hidden="true" /> Note this</button>
      {subject.kind === "world" ? <button type="button" className="link" disabled={draft !== null} onClick={() => choose(makeNote(overview, subject, label, "year"))}>Name this year</button> : null}
    </div>
  </section>;
}

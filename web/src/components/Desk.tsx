import { use, useEffect, useId, useRef, useState, type CSSProperties } from "react";
import type { Correspondences, MeaningSet, NotebookNote, Read, Segment, Truth } from "../model";
import { YEARS } from "../model";
import { LoanCauseText, type ChapterContext } from "./LanguageChapter";
import { makeNote } from "./Margin";
import { Pending } from "./Pending";
import "./desk.css";

interface DeskProps {
  first: number;
  second?: number;
  meanings?: string;
  ctx: ChapterContext;
  onSaveNote: (note: NotebookNote) => Promise<string | null>;
  onStartNote: () => void;
}

const samePair = (a: Segment, b: Segment) => a.a === b.a && a.b === b.b;

/** Current forms first; the independent historical record only on request. */
export function Desk({ first, second, meanings = "core", ctx, onSaveNote, onStartNote }: DeskProps) {
  const choices = ctx.overview.varieties.filter((variety) => variety.spoken && variety.id !== first)
    .sort((a, b) => a.name.localeCompare(b.name) || a.id - b.id);
  const [chosen, setChosen] = useState(second ?? choices[0]?.id ?? null);
  const [meaningSet, setMeaningSet] = useState(meanings);
  const other = choices.some((variety) => variety.id === chosen) ? chosen : choices[0]?.id ?? null;
  const sets = ctx.engine.meaningSets();
  const id = useId();
  return <section className="philologist-desk" aria-label="The philologist’s desk">
    <p className="desk-introduction">Read the sounds before the history. Family names, family colours, and descent are kept out of this desk until you reveal the record.</p>
    <div className="desk-choices">
      <p>First language <strong>{ctx.overview.varieties[first]?.name ?? "An unrecorded language"}</strong></p>
      <label htmlFor={`${id}-language`}>Compare with
        <select id={`${id}-language`} value={other ?? ""} disabled={!choices.length} onChange={(event) => setChosen(Number(event.currentTarget.value))}>
          {!choices.length ? <option value="">No other living language</option> : null}
          {choices.map((variety) => <option key={variety.id} value={variety.id}>{variety.name}</option>)}
        </select>
      </label>
      <Pending reading={sets} label="Reading the meaning sets…">
        <MeaningChoice reading={sets} value={meaningSet} onChange={setMeaningSet} id={`${id}-meanings`} />
      </Pending>
    </div>
    {other === null ? <p className="muted">The desk needs another living language to compare.</p> : <Pending reading={ctx.engine.correspondences(ctx.generation, first, other, meaningSet)} label="Aligning the current forms…">
      <DeskComparison key={`${ctx.version}:${ctx.generation}:${first}:${other}:${meaningSet}`} first={first} second={other} meanings={meaningSet} ctx={ctx} onSaveNote={onSaveNote} onStartNote={onStartNote} />
    </Pending>}
  </section>;
}

function MeaningChoice({ reading, value, onChange, id }: { reading: Read<MeaningSet[]>; value: string; onChange: (value: string) => void; id: string }) {
  const sets = use(reading);
  return <label htmlFor={id}>Meanings
    <select id={id} value={value} onChange={(event) => onChange(event.currentTarget.value)}>
      {sets.map((set) => <option key={set.id} value={set.id}>{set.label}</option>)}
    </select>
  </label>;
}

function DeskComparison({ first, second, meanings, ctx, onSaveNote, onStartNote }: DeskProps & { second: number; meanings: string }) {
  const comparison = use(ctx.engine.correspondences(ctx.generation, first, second, meanings));
  const [selected, setSelected] = useState<Segment[]>([]);
  const [revealed, setRevealed] = useState(false);
  const [draft, setDraft] = useState<NotebookNote | null>(null);
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const editor = useRef<HTMLTextAreaElement>(null);
  const noteButton = useRef<HTMLButtonElement>(null);
  useEffect(() => { if (draft) editor.current?.focus(); }, [draft?.id]);
  const a = ctx.overview.varieties[first].name;
  const b = ctx.overview.varieties[second].name;
  const note = () => {
    onStartNote();
    const hypothesis = makeNote(ctx.overview, { kind: "language", variety: first }, a);
    hypothesis.title = `${a} and ${b}: a hypothesis`;
    hypothesis.body = `Comparing ${a} with ${b}, ${meanings}, in year ${ctx.generation * YEARS}.\n\nThe recurring sound pairs ${selected.map((pair) => `${pair.a ?? "–"} ↔ ${pair.b ?? "–"}`).join(", ")} may point to a shared history. This is a hypothesis from the current forms${revealed ? "; the record has been revealed" : ", made before revealing the record"}.`;
    setDraft(hypothesis);
    setNotice(null);
  };
  const discard = () => { setDraft(null); setNotice(null); noteButton.current?.focus(); };
  const keep = async () => {
    if (!draft || saving) return;
    if (!draft.title.trim()) { setNotice("Give this hypothesis a title."); return; }
    setSaving(true);
    const failure = await onSaveNote({ ...draft, title: draft.title.trim() });
    setSaving(false);
    setNotice(failure ?? `Hypothesis kept in the margin of ${a}.`);
    if (!failure) { setDraft(null); noteButton.current?.focus(); }
  };
  return <>
    <div className="desk-actions">
      <button type="button" onClick={() => setRevealed(true)} disabled={revealed}>{revealed ? "Record revealed" : "Reveal the record"}</button>
      <button type="button" ref={noteButton} onClick={note} disabled={!selected.length || draft !== null}>Note a hypothesis</button>
      <span className="muted small">{selected.length ? `${selected.length} sound ${selected.length === 1 ? "pair" : "pairs"} selected` : "Select recurring pairs to carry into a note."}</span>
    </div>
    {draft ? <form className="margin-editor desk-hypothesis" onSubmit={(event) => { event.preventDefault(); void keep(); }}
      onKeyDown={(event) => { if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); if (!saving) discard(); } }}>
      <p className="margin-date">A hypothesis on {a} · year {draft.generation * YEARS}</p>
      <input aria-label="Hypothesis title" value={draft.title} disabled={saving} onChange={(event) => setDraft({ ...draft, title: event.currentTarget.value })} />
      <textarea ref={editor} aria-label="Hypothesis body" rows={4} value={draft.body} disabled={saving} onChange={(event) => setDraft({ ...draft, body: event.currentTarget.value })} />
      <div className="margin-actions"><button type="submit" disabled={saving}>{saving ? "Keeping…" : "Keep in the margin"}</button><button type="button" className="link" disabled={saving} onClick={discard}>Discard</button></div>
    </form> : null}
    {notice ? <p className="desk-note-status" role="status">{notice}</p> : null}
    <div className="desk-reading">
      <aside className="desk-correspondences" aria-label="Recurring correspondences">
        <h3>Recurring sound pairs</h3>
        <p className="muted small">Support counts words, not repeated sounds within one word. Select a pair to find it in the alignments.</p>
        {comparison.regular.length ? <ul>{comparison.regular.map(({ pair, support }) => {
          const chosen = selected.some((item) => samePair(item, pair));
          return <li key={JSON.stringify(pair)}><button type="button" aria-pressed={chosen} onClick={() => setSelected(chosen ? selected.filter((item) => !samePair(item, pair)) : [...selected, pair])}>
            <span className="ipa">{pair.a ?? "–"} ↔ {pair.b ?? "–"}</span><span>{support} {support === 1 ? "word" : "words"}</span>
          </button></li>;
        })}</ul> : <p className="muted small">No recurring sound pairs in these meanings. A larger meaning set may offer more evidence.</p>}
        {selected.length ? <button type="button" className="link" onClick={() => setSelected([])}>Clear selection</button> : null}
        {comparison.grammar.length ? <details className="desk-grammar"><summary>Grammatical evidence</summary><ul>{comparison.grammar.map((row, index) => <li key={index}><strong>{row.category}</strong><span className="ipa">{row.a || "–"} ↔ {row.b || "–"}</span><span className="muted">{row.regular ? "Recurring support" : "No recurring support"}</span></li>)}</ul></details> : null}
      </aside>
      <div className="desk-evidence">
        {revealed ? <Pending reading={ctx.engine.truth(ctx.generation, first, second, meanings)} label="Reading the independent record…">
          <RecordedComparison comparison={comparison} selected={selected} ctx={ctx} />
        </Pending> : <>
          <p className="desk-record-state">The record is covered. The method reads only current sounds and grammatical evidence, never the family tree.</p>
          <AlignedRows comparison={comparison} selected={selected} ctx={ctx} />
        </>}
      </div>
    </div>
  </>;
}

function RecordedComparison({ comparison, selected, ctx }: { comparison: Correspondences; selected: Segment[]; ctx: ChapterContext }) {
  const truth = use(ctx.engine.truth(ctx.generation, comparison.a, comparison.b, comparison.meanings));
  return <>
    <div className="desk-record-summary">
      <h3>The recorded history</h3>
      <p>{truth.ancestor === null ? "No common ancestor is recorded for these languages." : <>Their common ancestor is {ctx.link({ kind: "language", variety: truth.ancestor }, ctx.overview.varieties[truth.ancestor]?.name ?? "an unrecorded language")}{truth.splitAt === null ? "; their parting year is not recorded" : `; they parted in year ${truth.splitAt * YEARS}`}.</>}</p>
      <p className="muted small">A root history and recurring sounds are different kinds of evidence. Where they differ, read the contact or the missing support, not a grade.</p>
    </div>
    <AlignedRows comparison={comparison} selected={selected} truth={truth} ctx={ctx} />
  </>;
}

function AlignedRows({ comparison, selected, truth, ctx }: { comparison: Correspondences; selected: Segment[]; truth?: Truth; ctx: ChapterContext }) {
  const records = new Map(truth?.rows.map((row) => [row.concept, row]));
  const a = ctx.overview.varieties[comparison.a].name;
  const b = ctx.overview.varieties[comparison.b].name;
  return <div className="desk-rows">
    {!comparison.rows.length ? <p className="muted">No current word pairs are recorded for these meanings.</p> : null}
    {comparison.rows.map((row) => {
      const record = records.get(row.concept);
      const differs = record !== undefined && record.sameRoot !== row.cognate;
      const highlighted = row.alignment.some((pair) => selected.some((item) => samePair(item, pair)));
      return <article key={row.concept} className="desk-word-row" data-selected={highlighted || undefined} data-differs={differs || undefined} aria-label={row.gloss}>
        <header><h4>{row.gloss}</h4><span className="desk-method">{row.cognate ? "Method: looks like a shared root" : "Method: no regular support for a shared root"}</span></header>
        <div className={truth ? "desk-word-evidence revealed" : "desk-word-evidence"}>
          <div className="desk-word-forms">
            <div className="desk-alignment-scroll"><div className="desk-alignment" style={{ "--segments": Math.max(1, row.alignment.length) } as CSSProperties} role="group" aria-label={`Aligned sounds for ${row.gloss}`}>
              <span className="desk-language">{a}<span className="word">{row.a.spelled}</span></span>
              {row.alignment.map((pair, index) => <span key={`a-${index}`} className="ipa desk-segment" data-selected={selected.some((item) => samePair(item, pair)) || undefined} title={pair.a === null ? "Gap" : undefined}>{pair.a ?? "–"}</span>)}
              {!row.alignment.length ? <span className="ipa desk-segment">–</span> : null}
              <span className="desk-language">{b}<span className="word">{row.b.spelled}</span></span>
              {row.alignment.map((pair, index) => <span key={`b-${index}`} className="ipa desk-segment" data-selected={selected.some((item) => samePair(item, pair)) || undefined} title={pair.b === null ? "Gap" : undefined}>{pair.b ?? "–"}</span>)}
              {!row.alignment.length ? <span className="ipa desk-segment">–</span> : null}
            </div></div>
            <p className="desk-support muted small">{Math.round(row.regularity * 100)}% of aligned pairs recur. A dash (–) is a gap.</p>
          </div>
          {truth ? <div className="desk-row-record">
            {record ? <>
              <p><strong>{record.sameRoot ? "Same root" : "Different roots"}</strong>.</p>
              {record.borrowed ? <p>Borrowed into {ctx.link({ kind: "language", variety: record.borrowed.into }, ctx.overview.varieties[record.borrowed.into]?.name ?? "an unrecorded language")} from {ctx.link({ kind: "language", variety: record.borrowed.from }, ctx.overview.varieties[record.borrowed.from]?.name ?? "an unrecorded language")}.<LoanCauseText cause={record.borrowed.cause} ctx={ctx} /></p> : null}
              {differs ? <p className="desk-difference">{record.borrowed ? "Borrowing connects the histories of these forms; recurring sounds alone do not distinguish borrowing from descent." : record.sameRoot ? "The root is shared, but these current forms lack enough regular support for the method to recognise it." : "These sounds have recurring support, but no shared root in the record. A recurring pair alone does not establish a word’s ancestry."}</p> : null}
            </> : <p className="muted">No root history is recorded for this row.</p>}
          </div> : null}
        </div>
      </article>;
    })}
  </div>;
}

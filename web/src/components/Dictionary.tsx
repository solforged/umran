import { Fragment, use, useEffect, useMemo, useRef, type MouseEvent, type ReactNode } from "react";
import type { ReadEngine, LexiconForm, LexiconRow } from "../model";

type OriginFilter = "all" | "inherited" | "derived" | "coined" | "borrowed" | "kept";
export interface DictionaryView {
  query: string;
  field: string;
  origin: OriginFilter;
  concept: string | null;
}
export const INITIAL_DICTIONARY: DictionaryView = { query: "", field: "all", origin: "all", concept: null };

/// Every meaning's dominant word, with matching current competitors.
export function Dictionary({
  engine,
  version,
  generation,
  variety,
  view,
  onView,
  onConcept,
  renderOrigin,
  renderEvidence,
  evidenceBelow = false,
  rowId,
}: {
  engine: ReadEngine;
  version: number;
  generation: number;
  variety: number;
  view: DictionaryView;
  onView: (view: DictionaryView) => void;
  onConcept: (concept: string) => void;
  renderOrigin?: (form: LexiconForm) => ReactNode;
  renderEvidence?: (row: LexiconRow) => ReactNode;
  /// Complete editions give the full word history the ledger's whole measure.
  evidenceBelow?: boolean;
  rowId?: (row: LexiconRow) => string;
}) {
  const { query, field, origin, concept } = view;
  const filter = (patch: Partial<DictionaryView>) => onView({ ...view, concept: null, ...patch });
  const open = (concept: string) => { onView({ ...view, concept }); onConcept(concept); };
  const tableRef = useRef<HTMLTableSectionElement>(null);

  const rows = use(engine.lexicon(generation, variety));
  const fields = useMemo(() => [...new Set(rows.map((r) => r.field))].sort(), [rows]);
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return rows.flatMap((row) => {
      if (field !== "all" && row.field !== field) return [];
      if (q === "" && origin === "all") return [{ row, dominantMatched: false, matchingForms: [] }];
      const meaningMatches = q === "" || [row.gloss, row.concept].some((s) => s.toLowerCase().includes(q));
      const matches = (form: LexiconForm) =>
        (origin === "all" || form.origin.kind === origin) &&
        (meaningMatches || [form.spelled, form.said ?? "", form.ipa].some((s) => s.toLowerCase().includes(q)));
      const dominantMatched = matches(row);
      const matchingForms = row.competingForms.filter(matches);
      return dominantMatched || matchingForms.length ? [{ row, dominantMatched, matchingForms }] : [];
    });
  }, [rows, query, field, origin]);

  useEffect(() => {
    tableRef.current?.querySelector<HTMLElement>('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [concept]);

  const move = (delta: number) => {
    if (shown.length === 0) return;
    const at = shown.findIndex(({ row }) => row.concept === concept);
    const next = at < 0 ? 0 : Math.min(shown.length - 1, Math.max(0, at + delta));
    onView({ ...view, concept: shown[next].row.concept });
  };

  return (
    <div className="dictionary" data-lexicon-total={rows.length} data-lexicon-shown={shown.length}>
      <div className="filters">
        <input
          type="search"
          placeholder="Search meaning, word, or IPA"
          aria-label="Search the lexicon"
          value={query}
          onChange={(e) => filter({ query: e.target.value })}
        />
        <select aria-label="Semantic field" value={field} onChange={(e) => filter({ field: e.target.value })}>
          <option value="all">All fields</option>
          {fields.map((f) => (
            <option key={f} value={f}>
              {f}
            </option>
          ))}
        </select>
        <select aria-label="Origin" value={origin} onChange={(e) => filter({ origin: e.target.value as OriginFilter })}>
          <option value="all">Any origin</option>
          <option value="inherited">Inherited</option>
          <option value="derived">Built from another word</option>
          <option value="coined">Coined</option>
          <option value="borrowed">Borrowed</option>
          <option value="kept">Kept through a shift</option>
        </select>
      </div>
      <div
        className="table-wrap"
        tabIndex={0}
        role="group"
        aria-label="Words; use arrow keys or j and k to move, Enter to read"
        onKeyDown={(e) => {
          if (e.key === "ArrowDown" || e.key === "j") {
            e.preventDefault();
            move(1);
          } else if (e.key === "ArrowUp" || e.key === "k") {
            e.preventDefault();
            move(-1);
          } else if (e.key === "Enter" && e.target === e.currentTarget && concept !== null && shown.some(({ row }) => row.concept === concept)) {
            e.preventDefault();
            open(concept);
          }
        }}
      >
        <table>
          <thead>
            <tr>
              <th>Meaning</th>
              <th>Word</th>
              <th>IPA</th>
              <th>Origin</th>
              <th title="Sound changes the word has undergone">Changes</th>
            </tr>
          </thead>
          <tbody ref={tableRef}>
            {shown.map(({ row, dominantMatched, matchingForms }) => (
              <Row key={row.concept} id={rowId?.(row)} row={row} dominantMatched={dominantMatched} matchingForms={matchingForms} selected={row.concept === concept} onSelect={() => open(row.concept)} renderOrigin={renderOrigin} renderEvidence={renderEvidence} evidenceBelow={evidenceBelow} />
            ))}
          </tbody>
        </table>
        {shown.length === 0 ? <p className="muted pad">No words match.</p> : null}
      </div>
    </div>
  );
}

function Row({ id, row, dominantMatched, matchingForms, selected, onSelect, renderOrigin, renderEvidence, evidenceBelow }: {
  id?: string;
  row: LexiconRow;
  dominantMatched: boolean;
  matchingForms: LexiconForm[];
  selected: boolean;
  onSelect: () => void;
  renderOrigin?: (form: LexiconForm) => ReactNode;
  renderEvidence?: (row: LexiconRow) => ReactNode;
  evidenceBelow: boolean;
}) {
  const select = (e: MouseEvent<HTMLTableRowElement>) => {
    if ((e.target as HTMLElement).closest("button, a, details, input, select")) return;
    onSelect();
  };
  return <>
    {[row, ...matchingForms].map((form, i) => {
      const dominant = i === 0;
      return <Fragment key={i}><tr id={dominant ? id : undefined} data-concept={row.concept} data-matched-form={dominant ? undefined : form.spelled} aria-selected={dominant ? selected : undefined} className={selected ? "selected" : ""} onClick={select}>
        <td>{dominant ? <>
          <button type="button" className="link" onClick={(e) => { e.stopPropagation(); onSelect(); }}>{row.gloss}</button>
          {row.competitors > 0 ? <span className="badge" title="Other words compete for this meaning">+{row.competitors}</span> : null}
        </> : <span className="muted small">Matched competing word</span>}</td>
        <td>
          <span className="word">{form.spelled}</span>
          {form.said !== null ? <> <span className="muted">· said</span> <span className="word">{form.said}</span></> : null}
          {dominant && dominantMatched ? <span className="badge">Matched</span> : null}
        </td>
        <td className="ipa">/{form.ipa}/</td>
        <td>
          {renderOrigin ? renderOrigin(form) : <span className={`origin origin-${form.origin.kind}`}>
            {form.origin.kind === "borrowed"
              ? `from ${form.origin.from}`
              : form.origin.kind === "kept"
                ? `kept from ${form.origin.from}`
                : form.origin.kind === "derived"
                  ? `from ${form.origin.from}`
                  : form.origin.kind}
          </span>}
          {dominant && !evidenceBelow ? renderEvidence?.(row) : null}
        </td>
        <td className="num">{form.changes || ""}</td>
      </tr>
      {dominant && evidenceBelow && renderEvidence ? <tr className="dictionary-evidence"><td colSpan={5}>{renderEvidence(row)}</td></tr> : null}
      </Fragment>;
    })}
  </>;
}


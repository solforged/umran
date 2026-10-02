import { useEffect, useMemo, useRef } from "react";
import type { Engine, LexiconRow } from "../model";

type OriginFilter = "all" | "inherited" | "derived" | "coined" | "borrowed" | "kept";
export interface DictionaryView {
  query: string;
  field: string;
  origin: OriginFilter;
  concept: string | null;
}
export const INITIAL_DICTIONARY: DictionaryView = { query: "", field: "all", origin: "all", concept: null };

/// Every concept's current word in one language.
export function Dictionary({
  engine,
  version,
  generation,
  variety,
  view,
  onView,
  onConcept,
}: {
  engine: Engine;
  version: number;
  generation: number;
  variety: number;
  view: DictionaryView;
  onView: (view: DictionaryView) => void;
  onConcept: (concept: string) => void;
}) {
  const { query, field, origin, concept } = view;
  const filter = (patch: Partial<DictionaryView>) => onView({ ...view, concept: null, ...patch });
  const open = (concept: string) => { onView({ ...view, concept }); onConcept(concept); };
  const tableRef = useRef<HTMLTableSectionElement>(null);

  const rows = useMemo(
    () => engine.lexicon(generation, variety),
    // `version` changes whenever the history does.
    [engine, generation, variety, version],
  );
  const fields = useMemo(() => [...new Set(rows.map((r) => r.field))].sort(), [rows]);
  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return rows.filter(
      (r) =>
        (field === "all" || r.field === field) &&
        (origin === "all" || r.origin.kind === origin) &&
        (q === "" || [r.gloss, r.spelled, r.said ?? "", r.ipa, r.concept].some((s) => s.toLowerCase().includes(q))),
    );
  }, [rows, query, field, origin]);

  useEffect(() => {
    tableRef.current?.querySelector<HTMLElement>('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [concept]);

  const move = (delta: number) => {
    if (shown.length === 0) return;
    const at = shown.findIndex((r) => r.concept === concept);
    const next = at < 0 ? 0 : Math.min(shown.length - 1, Math.max(0, at + delta));
    onView({ ...view, concept: shown[next].concept });
  };

  return (
    <div className="dictionary">
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
          } else if (e.key === "Enter" && e.target === e.currentTarget && concept !== null && shown.some((r) => r.concept === concept)) {
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
            {shown.map((r) => (
              <Row key={r.concept} row={r} selected={r.concept === concept} onSelect={() => open(r.concept)} />
            ))}
          </tbody>
        </table>
        {shown.length === 0 ? <p className="muted pad">No words match.</p> : null}
      </div>
    </div>
  );
}

function Row({ row, selected, onSelect }: { row: LexiconRow; selected: boolean; onSelect: () => void }) {
  return (
    <tr aria-selected={selected} className={selected ? "selected" : ""} onClick={onSelect}>
      <td>
        <button type="button" className="link" onClick={(e) => { e.stopPropagation(); onSelect(); }}>{row.gloss}</button>
        {row.competitors > 0 ? <span className="badge" title="Other words compete for this meaning">+{row.competitors}</span> : null}
      </td>
      <td>
        <span className="word">{row.spelled}</span>
        {row.said !== null ? <> <span className="muted">· said</span> <span className="word">{row.said}</span></> : null}
      </td>
      <td className="ipa">/{row.ipa}/</td>
      <td>
        <span className={`origin origin-${row.origin.kind}`}>
          {row.origin.kind === "borrowed"
            ? `from ${row.origin.from}`
            : row.origin.kind === "kept"
              ? `kept from ${row.origin.from}`
              : row.origin.kind === "derived"
                ? `from ${row.origin.from}`
                : row.origin.kind}
        </span>
      </td>
      <td className="num">{row.changes || ""}</td>
    </tr>
  );
}

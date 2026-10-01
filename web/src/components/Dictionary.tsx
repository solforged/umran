import { useEffect, useMemo, useRef, useState } from "react";
import type { Engine, LexiconRow } from "../model";

type OriginFilter = "all" | "inherited" | "derived" | "coined" | "borrowed" | "kept";

/// Every concept's current word in one language.
export function Dictionary({
  engine,
  version,
  generation,
  variety,
  concept,
  onConcept,
}: {
  engine: Engine;
  version: number;
  generation: number;
  variety: number;
  concept: string | null;
  onConcept: (concept: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [field, setField] = useState("all");
  const [origin, setOrigin] = useState<OriginFilter>("all");
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
        (q === "" || [r.gloss, r.spelled, r.ipa, r.concept].some((s) => s.toLowerCase().includes(q))),
    );
  }, [rows, query, field, origin]);

  useEffect(() => {
    tableRef.current?.querySelector<HTMLElement>('[aria-selected="true"]')?.scrollIntoView({ block: "nearest" });
  }, [concept]);

  const move = (delta: number) => {
    if (shown.length === 0) return;
    const at = shown.findIndex((r) => r.concept === concept);
    const next = at < 0 ? 0 : Math.min(shown.length - 1, Math.max(0, at + delta));
    onConcept(shown[next].concept);
  };

  return (
    <div className="dictionary">
      <div className="filters">
        <input
          type="search"
          placeholder="Search meaning, word, or IPA"
          aria-label="Search the lexicon"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <select aria-label="Semantic field" value={field} onChange={(e) => setField(e.target.value)}>
          <option value="all">All fields</option>
          {fields.map((f) => (
            <option key={f} value={f}>
              {f}
            </option>
          ))}
        </select>
        <select aria-label="Origin" value={origin} onChange={(e) => setOrigin(e.target.value as OriginFilter)}>
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
        aria-label="Words; use arrow keys or j and k to move"
        onKeyDown={(e) => {
          if (e.key === "ArrowDown" || e.key === "j") {
            e.preventDefault();
            move(1);
          } else if (e.key === "ArrowUp" || e.key === "k") {
            e.preventDefault();
            move(-1);
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
              <Row key={r.concept} row={r} selected={r.concept === concept} onSelect={() => onConcept(r.concept)} />
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
        {row.gloss}
        {row.competitors > 0 ? <span className="badge" title="Other words compete for this meaning">+{row.competitors}</span> : null}
      </td>
      <td className="word">{row.spelled}</td>
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

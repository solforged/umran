import { useEffect, useRef } from "react";
import type { Annal } from "../model";
import { YEARS } from "../model";

/// The world's history as a chronicle: entries grouped by year, with the
/// chosen language's sound changes as marginal notes. The newest entry is
/// written out as if by hand.
export function Chronicle({
  annals,
  variety,
  onScrub,
}: {
  annals: Annal[];
  /// Whose sound changes to include.
  variety: number;
  onScrub: (generation: number) => void;
}) {
  const shown = annals.filter((a) => a.kind !== "law" || a.variety === variety);
  const years = new Map<number, Annal[]>();
  for (const annal of shown) {
    const list = years.get(annal.generation) ?? [];
    list.push(annal);
    years.set(annal.generation, list);
  }
  const newest = shown.at(-1);
  const end = useRef<HTMLDivElement>(null);
  useEffect(() => {
    end.current?.scrollIntoView({ block: "nearest" });
  }, [shown.length]);

  return (
    <div role="tabpanel">
      <h2>Chronicle</h2>
      {shown.length === 0 ? <p className="muted">Nothing has happened yet.</p> : null}
      <ol className="annals">
        {[...years.entries()].map(([generation, entries]) => (
          <li key={generation}>
            <p className="year">
              <button type="button" className="link" onClick={() => onScrub(generation)} title={`View generation ${generation}`}>
                {generation === 0 ? "In the beginning" : `Year ${generation * YEARS}`}
              </button>
            </p>
            {entries.map((annal, i) => (
              <p
                key={`${annal.kind}-${i}-${annal.text}`}
                className={`entry entry-${annal.kind}${annal === newest && generation > 0 ? " fresh" : ""}`}
              >
                {annal.text}
              </p>
            ))}
          </li>
        ))}
      </ol>
      <div ref={end} />
    </div>
  );
}

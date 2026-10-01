import type { ReactNode } from "react";
import type { Annal, TellingView, Variety } from "../model";
import { year, yearHeading } from "../words";

/// One line of the chronicle: written, or struck out from a telling set
/// aside.
interface Line {
  annal: Annal;
  telling: TellingView | null;
  /// The first struck line of its telling, which carries the margin note.
  opens: boolean;
}

/// The annalist's text, with words of the language (marked *thus*) set
/// in the language's style.
export function Told({ text }: { text: string }) {
  const parts: ReactNode[] = text.split(/\*([^*]+)\*/).map((part, i) =>
    i % 2 === 1 ? (
      <i key={i} className="word">
        {part}
      </i>
    ) : (
      part
    ),
  );
  return <>{parts}</>;
}

/// The linguist's notes on an entry, kept small and to the side; words of
/// the language are marked *thus*, as in the text.
function Apparatus({ notes }: { notes: string[] }) {
  if (notes.length === 0) return null;
  return (
    <ul className="apparatus" aria-label="Notes">
      {notes.map((note) => (
        <li key={note}>
          <Told text={note} />
        </li>
      ))}
    </ul>
  );
}

/// The world's history as a chronicle: entries grouped by year, with the
/// chosen language's sound changes told through its words. Nothing written is
/// erased: what was undone or told otherwise stays, struck through. The
/// newest entry is written out as if by hand.
export function Chronicle({
  annals,
  tellings,
  varieties,
  variety,
  onScrub,
  onRestore,
}: {
  annals: Annal[];
  tellings: TellingView[];
  varieties: Variety[];
  /// Whose sound changes to include, with its ancestors' before it forked.
  variety: number;
  onScrub: (generation: number) => void;
  onRestore: (telling: number) => void;
}) {
  // The chosen tongue and its ancestors, each up to where the next forked.
  const lineage: [number, number][] = [];
  for (let v: number | null = variety, until = Infinity; v !== null; ) {
    lineage.push([v, until]);
    until = varieties[v].forkedAt ?? 0;
    v = varieties[v].parent;
  }
  const relevant = (a: Annal) =>
    a.kind !== "law" || lineage.some(([v, until]) => a.variety === v && a.generation <= until);
  const written: Line[] = annals.filter(relevant).map((annal) => ({ annal, telling: null, opens: false }));
  const struck: Line[] = tellings.flatMap((telling) =>
    telling.struck.filter(relevant).map((annal, i) => ({ annal, telling, opens: i === 0 })),
  );
  // Struck lines follow what was written in the same year.
  const lines = [...written, ...struck].sort((a, b) => a.annal.generation - b.annal.generation);
  const years = new Map<number, Line[]>();
  for (const line of lines) {
    const list = years.get(line.annal.generation) ?? [];
    list.push(line);
    years.set(line.annal.generation, list);
  }
  const newest = written.at(-1)?.annal;

  return (
    <div className="chronicle">
      {lines.length === 0 ? <p className="muted">Nothing has happened yet.</p> : null}
      <ol className="annals">
        {[...years.entries()].map(([generation, entries]) => (
          <li key={generation}>
            <p className="year">
              <button
                type="button"
                className="link"
                onClick={() => onScrub(generation)}
                title={`Read the book as it stood in ${year(generation)}`}
              >
                {yearHeading(generation)}
              </button>
            </p>
            {entries.map(({ annal, telling, opens }, i) =>
              telling ? (
                <div key={`t${telling.index}-${i}`} className={`entry entry-${annal.kind} struck`}>
                  {opens ? (
                    <p className="struck-note">
                      {telling.why === "undone" ? "Struck out" : "In another telling"}
                      {" · "}
                      <button type="button" className="link" onClick={() => onRestore(telling.index)}>
                        tell it this way
                      </button>
                    </p>
                  ) : null}
                  <del>
                    <Told text={annal.text} />
                  </del>
                  <Apparatus notes={annal.notes} />
                </div>
              ) : (
                <div
                  key={`${annal.kind}-${i}-${annal.text}`}
                  className={`entry entry-${annal.kind}${annal === newest && generation > 0 ? " fresh" : ""}`}
                >
                  <p>
                    <Told text={annal.text} />
                  </p>
                  <Apparatus notes={annal.notes} />
                </div>
              ),
            )}
          </li>
        ))}
      </ol>
    </div>
  );
}

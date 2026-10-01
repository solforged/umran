import type { Annal, TellingView } from "../model";
import { YEARS } from "../model";

/// One line of the chronicle: written, or struck out from a telling set
/// aside.
interface Line {
  annal: Annal;
  telling: TellingView | null;
  /// The first struck line of its telling, which carries the margin note.
  opens: boolean;
}

/// The world's history as a chronicle: entries grouped by year, with the
/// chosen language's sound changes as marginal notes. Nothing written is
/// erased: what was undone or told otherwise stays, struck through. The
/// newest entry is written out as if by hand.
export function Chronicle({
  annals,
  tellings,
  variety,
  onScrub,
  onRestore,
}: {
  annals: Annal[];
  tellings: TellingView[];
  /// Whose sound changes to include.
  variety: number;
  onScrub: (generation: number) => void;
  onRestore: (telling: number) => void;
}) {
  const relevant = (a: Annal) => a.kind !== "law" || a.variety === variety;
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
              <button type="button" className="link" onClick={() => onScrub(generation)} title={`Read the book as it stood in year ${generation * YEARS}`}>
                {generation === 0 ? "In the beginning" : `Year ${generation * YEARS}`}
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
                  <del>{annal.text}</del>
                </div>
              ) : (
                <p
                  key={`${annal.kind}-${i}-${annal.text}`}
                  className={`entry entry-${annal.kind}${annal === newest && generation > 0 ? " fresh" : ""}`}
                >
                  {annal.text}
                </p>
              ),
            )}
          </li>
        ))}
      </ol>
    </div>
  );
}

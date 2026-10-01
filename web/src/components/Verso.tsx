import { useEffect, useRef } from "react";
import type { Overview } from "../model";
import type { DialogKind } from "./ActionDialog";
import { Chronicle } from "./Chronicle";

/// The left page: who the book is about, what has happened to them, and an
/// open line for what happens next.
export function Verso({
  overview,
  selected,
  atPresent,
  onSelect,
  onScrub,
  onRestore,
  onDialog,
  onStep,
  onNextEvent,
}: {
  overview: Overview;
  selected: number;
  atPresent: boolean;
  onSelect: (community: number) => void;
  onScrub: (generation: number) => void;
  onRestore: (telling: number) => void;
  onDialog: (kind: DialogKind) => void;
  onStep: () => void;
  onNextEvent: () => void;
}) {
  const { communities, varieties } = overview;
  const current = communities[selected];
  const hand = (community: number) => `hand-${varieties[communities[community].variety].family % 5}`;
  const many = communities.length > 1;
  const end = useRef<HTMLDivElement>(null);
  // Keep the newest entry and the open line in view as the story grows.
  useEffect(() => {
    end.current?.scrollIntoView({ block: "nearest" });
  }, [overview.annals.length]);

  return (
    <section className="page verso" aria-label="Chronicle">
      <p className="cast">
        Here is written what befell{" "}
        {communities.map((c, i) => (
          <span key={c.id}>
            {i === 0 ? "the " : i === communities.length - 1 ? " and the " : ", the "}
            <button
              type="button"
              className={`link person ${hand(c.id)}${c.id === selected ? " chosen" : ""}`}
              aria-pressed={c.id === selected}
              title={`${c.name}, “${c.meaning}”: read their tongue`}
              onClick={() => onSelect(c.id)}
            >
              {c.name}
            </button>
          </span>
        ))}
        .
      </p>

      <Chronicle
        annals={overview.annals}
        tellings={overview.tellings}
        variety={current.variety}
        onScrub={onScrub}
        onRestore={onRestore}
      />

      <div className="and-then">
        <p className="and-then-lead">And then…</p>
        {atPresent ? null : (
          <p className="muted small">Writing here begins another telling from this year; the later years stay, struck through.</p>
        )}
        <ul>
          <li>
            <button type="button" className="link" onClick={onStep}>
              a generation passes
            </button>
          </li>
          <li>
            <button type="button" className="link" onClick={onNextEvent} disabled={!atPresent}>
              years pass until something happens
            </button>
          </li>
          <li>
            <button type="button" className="link" onClick={() => onDialog("split")}>
              some of the <span className={hand(selected)}>{current.name}</span> go their own way
            </button>
          </li>
          {many ? (
            <>
              <li>
                <button type="button" className="link" onClick={() => onDialog("connect")}>
                  the <span className={hand(selected)}>{current.name}</span> come to know another people
                </button>
              </li>
              <li>
                <button type="button" className="link" onClick={() => onDialog("shift")}>
                  the <span className={hand(selected)}>{current.name}</span> take up another tongue
                </button>
              </li>
            </>
          ) : null}
          <li>
            <button type="button" className="link" onClick={() => onDialog("found")}>
              a new people arrives
            </button>
          </li>
        </ul>
        <div ref={end} />
      </div>
    </section>
  );
}

import { useEffect, useRef, useState } from "react";
import type { Overview } from "../model";
import { YEARS } from "../model";
import { PACES } from "../words";
import type { DialogKind } from "./ActionDialog";
import { Chronicle } from "./Chronicle";

/// The left page: who the book is about, what has befallen them, and an
/// open line for what comes next. Time passes here too, so the book needs
/// no toolbar.
export function Verso({
  overview,
  selected,
  atPresent,
  canStrike,
  onSelect,
  onScrub,
  onRestore,
  onDialog,
  onRun,
  onTick,
  onStop,
  onNextEvent,
  onStrike,
}: {
  overview: Overview;
  selected: number;
  atPresent: boolean;
  canStrike: boolean;
  onSelect: (community: number) => void;
  onScrub: (generation: number) => void;
  onRestore: (telling: number) => void;
  onDialog: (kind: DialogKind) => void;
  /// Let `generations` pass, beginning another telling if reading the past.
  onRun: (generations: number) => void;
  /// One generation at the present, while the years pass on their own.
  onTick: () => boolean;
  /// The years stopped passing; a chance to save.
  onStop: () => void;
  onNextEvent: () => void;
  onStrike: () => void;
}) {
  const { communities, varieties } = overview;
  const current = communities[selected];
  const hand = (community: number) => `hand-${varieties[communities[community].variety].family % 5}`;
  const many = communities.length > 1;
  const [passing, setPassing] = useState(false);
  const [pace, setPace] = useState(4);
  const [until, setUntil] = useState("");
  const end = useRef<HTMLDivElement>(null);
  const tick = useRef(onTick);
  tick.current = onTick;
  const stop = useRef(onStop);
  stop.current = onStop;

  // Keep the newest entry and the open line in view as the story grows.
  useEffect(() => {
    end.current?.scrollIntoView({ block: "nearest" });
  }, [overview.annals.length, overview.generation]);

  useEffect(() => {
    if (!passing) return;
    const timer = window.setInterval(() => {
      if (!tick.current()) setPassing(false);
    }, 1000 / pace);
    return () => {
      window.clearInterval(timer);
      stop.current();
    };
  }, [passing, pace]);

  // Turning back to an earlier year stops the years passing.
  useEffect(() => {
    if (!atPresent) setPassing(false);
  }, [atPresent]);

  const target = Math.ceil(Number(until) / YEARS) - overview.generation;
  const people = <span className={hand(selected)}>{current.name}</span>;

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
        varieties={overview.varieties}
        variety={current.variety}
        onScrub={onScrub}
        onRestore={onRestore}
      />

      <div className="and-then">
        <p className="and-then-lead">And then…</p>
        {atPresent ? null : (
          <p className="muted small">
            Writing here begins another telling from this year; the later years stay, struck through.
          </p>
        )}
        <ul>
          {passing ? (
            <li>
              the years are passing{" "}
              <button type="button" className="link" onClick={() => setPassing(false)}>
                (stay)
              </button>
            </li>
          ) : (
            <>
              <li>
                <button type="button" className="link" onClick={() => onRun(1)}>
                  a generation passes
                </button>
              </li>
              <li>
                <button type="button" className="link" disabled={!atPresent} onClick={() => setPassing(true)}>
                  the years pass
                </button>{" "}
                <select className="inline" aria-label="How quickly" value={pace} onChange={(e) => setPace(Number(e.target.value))}>
                  {PACES.map(([speed, word]) => (
                    <option key={speed} value={speed}>
                      {word}
                    </option>
                  ))}
                </select>
              </li>
              <li>
                <button type="button" className="link" onClick={onNextEvent} disabled={!atPresent}>
                  years pass until something happens
                </button>
              </li>
              <li>
                {/* Years are multiples of 25, so every ordinal ends in "th". */}
                <form
                  className="inline-form"
                  onSubmit={(e) => {
                    e.preventDefault();
                    if (target > 0) onRun(target);
                    setUntil("");
                  }}
                >
                  <button type="submit" className="link" disabled={!(target > 0)}>
                    years pass until the
                  </button>{" "}
                  <input
                    type="number"
                    className="inline"
                    min={(overview.generation + 1) * YEARS}
                    step={YEARS}
                    placeholder={String((overview.generation + 4) * YEARS)}
                    aria-label="Until which year"
                    value={until}
                    onChange={(e) => setUntil(e.target.value)}
                  />
                  th year
                </form>
              </li>
              <li>
                <button type="button" className="link" onClick={() => onDialog("split")}>
                  some of the {people} go their own way
                </button>
              </li>
              {many ? (
                <>
                  <li>
                    <button type="button" className="link" onClick={() => onDialog("connect")}>
                      the {people} come to know another people
                    </button>
                  </li>
                  <li>
                    <button type="button" className="link" onClick={() => onDialog("shift")}>
                      the {people} take up another tongue
                    </button>
                  </li>
                </>
              ) : null}
              <li>
                <button type="button" className="link" onClick={() => onDialog("found")}>
                  a new people arrives
                </button>
              </li>
            </>
          )}
        </ul>
        {canStrike && !passing ? (
          <p className="strike">
            <button type="button" className="link" onClick={onStrike}>
              Or strike out what was last written.
            </button>
          </p>
        ) : null}
        <div ref={end} />
      </div>
    </section>
  );
}

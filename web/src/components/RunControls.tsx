import { useEffect, useRef, useState } from "react";
import { YEARS } from "../model";

const SPEEDS = [1, 4, 16];
/// Longest wait for something to happen before giving up.
const EVENT_LIMIT = 400;

/// Ways to let time pass: one step, continuous play, until the next split
/// or shift, or to a chosen year.
export function RunControls({
  generation,
  atPresent,
  onRun,
  onTick,
  onNextEvent,
  onStop,
}: {
  generation: number;
  atPresent: boolean;
  /// Run `generations`, branching from the viewed past if need be.
  onRun: (generations: number) => void;
  /// Run one generation at the present while playing.
  onTick: () => boolean;
  onNextEvent: (limit: number) => void;
  /// Playing stopped; a chance to save.
  onStop: () => void;
}) {
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(4);
  const [year, setYear] = useState("");
  const tick = useRef(onTick);
  tick.current = onTick;
  const stop = useRef(onStop);
  stop.current = onStop;

  useEffect(() => {
    if (!playing) return;
    const timer = window.setInterval(() => {
      if (!tick.current()) setPlaying(false);
    }, 1000 / speed);
    return () => {
      window.clearInterval(timer);
      stop.current();
    };
  }, [playing, speed]);

  // Scrubbing into the past pauses play.
  useEffect(() => {
    if (!atPresent) setPlaying(false);
  }, [atPresent]);

  const target = Number(year);
  const runTo = Math.ceil(target / YEARS) - generation;

  return (
    <div className="row run-controls">
      <button type="button" onClick={() => onRun(1)} disabled={playing} title="Run one generation (25 years)">
        Step
      </button>
      <button
        type="button"
        className="primary"
        disabled={!atPresent}
        title={atPresent ? "Let time pass" : "Return to the present to play"}
        onClick={() => setPlaying((p) => !p)}
      >
        {playing ? "Pause" : "Play"}
      </button>
      <select aria-label="Speed" value={speed} onChange={(e) => setSpeed(Number(e.target.value))}>
        {SPEEDS.map((s) => (
          <option key={s} value={s}>
            {s} gen/s
          </option>
        ))}
      </select>
      <button
        type="button"
        disabled={!atPresent || playing}
        title={atPresent ? "Run until a community splits or shifts language" : "Return to the present first"}
        onClick={() => onNextEvent(EVENT_LIMIT)}
      >
        Next event
      </button>
      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          if (runTo > 0) onRun(runTo);
          setYear("");
        }}
      >
        <input
          type="number"
          min={(generation + 1) * YEARS}
          step={YEARS}
          placeholder="year"
          aria-label="Run to year"
          className="year"
          value={year}
          onChange={(e) => setYear(e.target.value)}
        />
        <button type="submit" disabled={playing || !(runTo > 0)}>
          Run to
        </button>
      </form>
    </div>
  );
}

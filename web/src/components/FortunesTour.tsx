import { useEffect, useRef } from "react";
import type { Annal, Move, Overview } from "../model";
import { YEARS } from "../model";
import { individualAnnals } from "../history";
import { isQuiet } from "../eras";
import { Told } from "./Told";
import "./FortunesTour.css";

export interface FortuneStep {
  generation: number;
  annal: Annal | null;
  moves: Move[];
  regions: number[];
}

/// Headlines naming this people, plus journeys with no corresponding entry.
/// Route lines come only from recorded legs, never from a guessed shortcut.
export function fortuneSteps(overview: Overview, people: number): FortuneStep[] {
  const annals = individualAnnals(overview.annals);
  const named = annals.filter((annal) => (annal.peoples.includes(people) ||
    annal.names.some((name) => name.subject.kind === "people" && name.subject.id === people)) && !isQuiet(annal, annals));
  const journeys = overview.moves.filter((move) => move.community === people);
  const claimed = new Set<Move>();
  const steps: FortuneStep[] = named.map((annal) => {
    const moves = ["migration", "split", "settlement"].includes(annal.kind)
      ? overview.moves.filter((move) => !claimed.has(move) && move.generation === annal.generation && annal.peoples.includes(move.community) &&
        (annal.lands.length === 0 || annal.lands.includes(move.from) || annal.lands.includes(move.to))) : [];
    moves.forEach((move) => claimed.add(move));
    return { generation: annal.generation, annal, moves,
      regions: [...new Set([...annal.lands, ...moves.flatMap((move) => [move.from, ...move.path, move.to])])] };
  });
  for (const move of journeys) {
    if (!claimed.has(move)) steps.push({ generation: move.generation, annal: null, moves: [move],
      regions: [...new Set([move.from, ...move.path, move.to])] });
  }
  return steps.sort((a, b) => a.generation - b.generation);
}

export function FortunesTour({ name, steps, index, busy, error, onStep, onDone }: {
  name: string; steps: FortuneStep[]; index: number; busy: boolean; error: string | null;
  onStep: (index: number) => void; onDone: () => void;
}) {
  const panel = useRef<HTMLElement>(null);
  const done = useRef(onDone);
  done.current = onDone;
  useEffect(() => {
    panel.current?.focus({ preventScroll: true });
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault(); event.stopImmediatePropagation(); done.current();
    };
    const keepReading = (event: MouseEvent | KeyboardEvent) => {
      if (!(event.target instanceof Element) || event.target.closest(".fortunes-tour")) return;
      if (event.target.closest(".map-viewpoint")) {
        if (event instanceof KeyboardEvent && event.key === "Tab") return;
        event.preventDefault(); event.stopImmediatePropagation(); return;
      }
      if (event instanceof KeyboardEvent && event.key !== "Enter" && event.key !== " ") return;
      if (!event.target.closest("[data-map-feature], svg[role=application]")) return;
      event.preventDefault(); event.stopImmediatePropagation();
    };
    window.addEventListener("keydown", escape, true);
    window.addEventListener("click", keepReading, true);
    window.addEventListener("keydown", keepReading, true);
    return () => {
      window.removeEventListener("keydown", escape, true);
      window.removeEventListener("click", keepReading, true);
      window.removeEventListener("keydown", keepReading, true);
    };
  }, []);
  const step = steps[index];
  return <section ref={panel} className="fortunes-tour" role="dialog" aria-label={`The fortunes of ${name}`} tabIndex={-1} data-map-furniture>
    <header><span>The fortunes of <i>{name}</i></span><small>{index + 1} of {steps.length}</small></header>
    <div className="fortunes-entry" aria-live="polite" aria-busy={busy}>
      <h3>Year {step.generation * YEARS}</h3>
      {step.annal ? <p><Told text={step.annal.text} /></p> : <p>Recorded {step.moves[0].kind === "split" ? "split-off journey" : "migration"}.</p>}
      {busy ? <p className="muted small">Opening the recorded year…</p> : null}
      {error ? <p className="notice error" role="alert">{error}</p> : null}
    </div>
    <footer><button type="button" disabled={busy || index === 0} onClick={() => onStep(index - 1)}>Previous</button>
      <button type="button" disabled={busy || index === steps.length - 1} onClick={() => onStep(index + 1)}>Next</button>
      <button type="button" onClick={onDone}>Done</button></footer>
  </section>;
}

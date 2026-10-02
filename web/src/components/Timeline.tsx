import { useEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";
import type { Annal } from "../model";
import { YEARS } from "../model";
import { EVENT_KIND } from "../lore";
import "../timeline.css";

export type MarkKind = "found" | "split" | "shift" | "conquest" | "rose" | "fell" | "faith" | "craft";

export interface TimelineMark {
  generation: number;
  kind: MarkKind;
  label: string;
  annal: Annal;
}

/// One generation's sound changes, kept on the quiet track.
export interface SoundMark {
  generation: number;
  count: number;
  label: string;
}

const PRECEDENCE: Record<MarkKind, number> = {
  found: 0, split: 1, shift: 2, conquest: 3, rose: 4, fell: 5, faith: 6, craft: 7,
};
const RULER_STEPS = [100, 200, 500, 1000, 2000, 5000, 10000];

function isMarkKind(kind: Annal["kind"]): kind is MarkKind {
  return Object.hasOwn(PRECEDENCE, kind);
}

/// Extract turning points without changing the annals; sort by generation,
/// then precedence. Sound changes are counted separately per generation.
export function marksFromAnnals(annals: Annal[]): { marks: TimelineMark[]; sounds: SoundMark[] } {
  const marks: TimelineMark[] = [];
  const counts = new Map<number, number>();
  for (const annal of annals) {
    const { generation, kind } = annal;
    if (isMarkKind(kind)) {
      marks.push({ generation, kind, label: `${EVENT_KIND[kind].name}, year ${generation * YEARS}`, annal });
    } else if (kind === "law") {
      counts.set(generation, (counts.get(generation) ?? 0) + 1);
    }
  }
  marks.sort((a, b) => a.generation - b.generation || PRECEDENCE[a.kind] - PRECEDENCE[b.kind]);
  const sounds = [...counts].sort(([a], [b]) => a - b).map(([generation, count]) => ({
    generation,
    count,
    label: `${count === 1 ? "A sound change" : `${count} sound changes`}, year ${generation * YEARS}`,
  }));
  return { marks, sounds };
}

/// Use the smallest available year step that leaves at least 56px between
/// ruler ticks. The present need not coincide with a tick.
export function rulerTicks(latest: number, width: number): { years: number[]; step: number } {
  if (latest === 0) return { years: [0], step: 100 };
  const end = latest * YEARS;
  const step = RULER_STEPS.find((candidate) => candidate / end * width >= 56) ?? 10000;
  const years: number[] = [];
  for (let year = 0; year <= end; year += step) years.push(year);
  return { years, step };
}

function shape(kind: MarkKind): ReactNode {
  switch (kind) {
    case "found": return <circle cx="5" cy="5" r="4" />;
    case "split": return <rect x="2" y="2" width="6" height="6" transform="rotate(45 5 5)" />;
    case "shift": return <path d="M5 1 9 9H1Z" />;
    case "conquest": return <rect x="1" y="1" width="8" height="8" />;
    case "rose":
    case "fell": return <path d="M5 1 9 4V9H1V4Z" />;
    case "faith": return <path d="M5 0 6.5 3.5 10 5 6.5 6.5 5 10 3.5 6.5 0 5 3.5 3.5Z" />;
    case "craft": return <path d="M2.5 1H7.5L10 5 7.5 9H2.5L0 5Z" />;
  }
}

/// A controlled generation slider. Mark activation belongs to onOpen, so
/// the caller can seek and open its annal together without a second scrub.
export function Timeline({ marks, sounds, latest, viewed, onScrub, onOpen }: {
  marks: TimelineMark[];
  sounds: SoundMark[];
  latest: number;
  viewed: number;
  onScrub: (generation: number) => void;
  onOpen: (mark: TimelineMark) => void;
}): ReactNode {
  const root = useRef<HTMLDivElement>(null);
  const pointer = useRef<number | null>(null);
  const scrubbed = useRef(viewed);
  const [width, setWidth] = useState(600);
  useEffect(() => { scrubbed.current = viewed; }, [viewed]);
  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => setWidth(entry.contentRect.width));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const groups = useMemo(() => {
    const generations = new Map<number, TimelineMark[]>();
    for (const mark of marks) {
      const group = generations.get(mark.generation);
      if (group) group.push(mark);
      else generations.set(mark.generation, [mark]);
    }
    for (const group of generations.values()) group.sort((a, b) => PRECEDENCE[a.kind] - PRECEDENCE[b.kind]);
    return [...generations];
  }, [marks]);
  const ticks = useMemo(() => rulerTicks(latest, width), [latest, width]);
  const left = (generation: number) => `${generation / Math.max(latest, 1) * 100}%`;
  const scrub = (generation: number) => {
    const next = Math.max(0, Math.min(latest, generation));
    if (next === scrubbed.current) return;
    scrubbed.current = next;
    onScrub(next);
  };
  const scrubPointer = (event: PointerEvent<HTMLDivElement>) => {
    const bounds = event.currentTarget.getBoundingClientRect();
    if (bounds.width > 0) scrub(Math.round((event.clientX - bounds.left) / bounds.width * latest));
  };
  const down = (event: PointerEvent<HTMLDivElement>) => {
    if (latest === 0 || event.button !== 0 || pointer.current !== null) return;
    event.preventDefault();
    event.currentTarget.focus();
    pointer.current = event.pointerId;
    event.currentTarget.setPointerCapture(event.pointerId);
    scrubPointer(event);
  };
  const up = (event: PointerEvent<HTMLDivElement>) => {
    if (pointer.current !== event.pointerId) return;
    pointer.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  };
  const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target !== event.currentTarget) return;
    let next: number;
    switch (event.key) {
      case "ArrowLeft": next = scrubbed.current - 1; break;
      case "ArrowRight": next = scrubbed.current + 1; break;
      case "PageUp": next = scrubbed.current + 10; break;
      case "PageDown": next = scrubbed.current - 10; break;
      case "Home": next = 0; break;
      case "End": next = latest; break;
      default: return;
    }
    event.preventDefault();
    scrub(next);
  };

  return <div
    ref={root}
    className="timeline"
    role="slider"
    tabIndex={0}
    aria-label="Timeline"
    aria-valuemin={0}
    aria-valuemax={latest}
    aria-valuenow={viewed}
    aria-valuetext={`year ${viewed * YEARS}`}
    onPointerDown={down}
    onPointerMove={(event) => { if (latest > 0 && pointer.current === event.pointerId) scrubPointer(event); }}
    onPointerUp={up}
    onPointerCancel={up}
    onLostPointerCapture={() => { pointer.current = null; }}
    onKeyDown={keyDown}
  >
    <div className="timeline-track main">
      {latest === 0 ? null : groups.map(([generation, group]) => {
        const visible = group.length > 4 ? group.slice(0, 3) : group;
        const ahead = generation > viewed ? " ahead" : "";
        return <div key={generation}>
          {visible.map((mark, stack) => <button
            type="button"
            key={stack}
            className={`mark mark-${mark.kind}${ahead}`}
            style={{ left: left(generation), "--stack": stack } as CSSProperties}
            title={mark.label}
            aria-label={mark.label}
            onPointerDown={(event) => event.stopPropagation()}
            onClick={(event) => { event.stopPropagation(); onOpen(mark); }}
          >
            <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">{shape(mark.kind)}</svg>
          </button>)}
          {group.length > 4 ? <span
            className={`mark mark-more${ahead}`}
            style={{ left: left(generation), "--stack": 3 } as CSSProperties}
            title={group.slice(3).map((mark) => mark.label).join("\n")}
            onPointerDown={(event) => event.stopPropagation()}
          >+{group.length - 3}</span> : null}
        </div>;
      })}
    </div>
    <div className="timeline-track sounds">
      {latest === 0 ? null : sounds.map((sound) => <span
        key={sound.generation}
        className={`sound${sound.generation > viewed ? " ahead" : ""}`}
        style={{ left: left(sound.generation), "--count": sound.count } as CSSProperties}
        title={sound.label}
      />)}
    </div>
    <div className="timeline-ruler">
      {ticks.years.map((year) => <span className="timeline-tick" key={year} style={{ left: left(year / YEARS) }}>
        <small>{year}</small>
      </span>)}
    </div>
    <div className="timeline-cursor" style={{ left: left(viewed) }} aria-hidden="true" />
  </div>;
}

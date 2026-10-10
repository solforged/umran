import { useEffect, useRef, useState, type CSSProperties, type PointerEvent, type ReactNode } from "react";

export type SheetDetent = "peek" | "half" | "full";
const DETENTS: SheetDetent[] = ["peek", "half", "full"];

/** A card on a compact desk; ordinary grid children on a wider one. */
export function Sheet({ detent, onDetent, label, children }: {
  detent: SheetDetent;
  onDetent: (detent: SheetDetent) => void;
  label: string;
  children: ReactNode;
}) {
  const sheet = useRef<HTMLDivElement>(null);
  const drag = useRef<{ pointer: number; y: number; height: number; heights: number[]; moved: boolean } | null>(null);
  const suppressClick = useRef(false);
  const [dragHeight, setDragHeight] = useState<number | null>(null);

  useEffect(() => {
    const lower = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || !window.matchMedia("(max-width: 599px)").matches
        || document.querySelector("dialog:modal, .popover-panel")) return;
      if (detent !== "peek") {
        event.preventDefault();
        onDetent(DETENTS[DETENTS.indexOf(detent) - 1]);
      }
    };
    document.addEventListener("keydown", lower);
    return () => document.removeEventListener("keydown", lower);
  }, [detent, onDetent]);

  const start = (event: PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0 || !sheet.current) return;
    const element = sheet.current;
    const map = element.closest(".stage")?.querySelector(".stage-map");
    if (!map) return;
    const style = getComputedStyle(element);
    const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
    const peek = parseFloat(style.getPropertyValue("--sheet-peek")) * rem;
    const full = Math.max(peek, map.getBoundingClientRect().height - parseFloat(style.getPropertyValue("--sheet-ceiling")) * rem);
    drag.current = { pointer: event.pointerId, y: event.clientY, height: element.getBoundingClientRect().height,
      heights: [Math.min(peek, full), Math.min(window.innerHeight / 2, full), full], moved: false };
    suppressClick.current = false;
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const move = (event: PointerEvent<HTMLButtonElement>) => {
    const current = drag.current;
    if (!current || current.pointer !== event.pointerId) return;
    const delta = current.y - event.clientY;
    if (Math.abs(delta) > 4) current.moved = true;
    if (current.moved) setDragHeight(Math.max(current.heights[0], Math.min(current.heights[2], current.height + delta)));
  };
  const finish = (event: PointerEvent<HTMLButtonElement>, cancelled = false) => {
    const current = drag.current;
    if (!current || current.pointer !== event.pointerId) return;
    drag.current = null;
    if (current.moved) {
      suppressClick.current = true;
      if (!cancelled) {
        const height = Math.max(current.heights[0], Math.min(current.heights[2], current.height + current.y - event.clientY));
        const nearest = current.heights.reduce((best, value, index) =>
          Math.abs(value - height) < Math.abs(current.heights[best] - height) ? index : best, 0);
        onDetent(DETENTS[nearest]);
      }
    }
    setDragHeight(null);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  };

  return <div ref={sheet} className="bottom-sheet" data-detent={detent} data-dragging={dragHeight !== null || undefined}
    style={dragHeight === null ? undefined : { "--sheet-drag-height": `${dragHeight}px` } as CSSProperties}>
    <button type="button" className="sheet-handle" aria-label={label} aria-expanded={detent !== "peek"}
      onPointerDown={start} onPointerMove={move} onPointerUp={(event) => finish(event)}
      onPointerCancel={(event) => finish(event, true)}
      onClick={() => {
        if (suppressClick.current) { suppressClick.current = false; return; }
        onDetent(DETENTS[(DETENTS.indexOf(detent) + 1) % DETENTS.length]);
      }}><span aria-hidden="true" /></button>
    <div className="sheet-content">{children}</div>
  </div>;
}

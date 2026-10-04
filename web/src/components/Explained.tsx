import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { TERMS, type Term } from "../lore";

/// A linguist's term: clicking it floats a short explanation under it.
export function Explained({ term, children }: { term: Term; children: ReactNode }) {
  const [at, setAt] = useState<CSSProperties | null>(null);
  const button = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!at) return;
    const close = () => setAt(null);
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    const press = (e: PointerEvent) => {
      if (!button.current?.contains(e.target as Node)) close();
    };
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    window.addEventListener("keydown", key);
    window.addEventListener("pointerdown", press);
    return () => {
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
      window.removeEventListener("keydown", key);
      window.removeEventListener("pointerdown", press);
    };
  }, [at]);
  const toggle = () => {
    if (at || !button.current) return setAt(null);
    const r = button.current.getBoundingClientRect();
    const width = Math.min(320, window.innerWidth - 16);
    const left = Math.max(8, Math.min(r.left, window.innerWidth - width - 8));
    const below = r.bottom + 160 < window.innerHeight;
    setAt(below ? { left, top: r.bottom + 4 } : { left, top: r.top - 4, transform: "translateY(-100%)" });
  };
  return <>
    <button ref={button} type="button" className="term" aria-expanded={at !== null} onClick={toggle}>{children}</button>
    {at ? <span className="term-note" role="note" style={at}>{TERMS[term]}</span> : null}
  </>;
}

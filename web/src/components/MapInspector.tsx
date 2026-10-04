import { useEffect, useId, useLayoutEffect, useRef, useState, type RefObject } from "react";
import { createPortal } from "react-dom";
import type { ClimateView, LakeNamesView, Overview, RiverNamesView, Subject, WorldMap } from "../model";
import { YEARS } from "../model";
import { featureKey, gazette, type MapFeature } from "../gazetteer";

export interface MapInspection { feature: MapFeature; at: [number, number] }

const RELATION = { selected: "Selected speech", holders: "Holders", exonym: "Elsewhere", former: "Former holders", city: "The city" };

/// One HTML apparatus card, shared by chart and globe. The map's existing
/// clicks still open cards; this independent capture keeps the place pinned.
export function MapInspector({ container, inspection, map, overview, generation, selectedVariety, riverNames, lakeNames, climate, known, onOpen, pinnable = true }: {
  container: RefObject<HTMLElement | null>;
  inspection: MapInspection | null;
  map: WorldMap;
  overview: Overview;
  generation: number;
  selectedVariety?: number;
  riverNames?: RiverNamesView[];
  lakeNames?: LakeNamesView[];
  climate?: ClimateView;
  known?: ReadonlySet<number> | null;
  /// Founding has no Pedia, so no Open action is offered there.
  onOpen?: (subject: Subject) => void;
  /// Founding's click already fills its own panel; there the card is hover-only.
  pinnable?: boolean;
}) {
  const id = useId();
  const panel = useRef<HTMLDivElement>(null);
  const [host, setHost] = useState<HTMLElement | null>(null);
  const [shown, setShown] = useState<MapInspection | null>(null);
  const [pinned, setPinned] = useState(false);
  const pinnedRef = useRef(false);
  const shownRef = useRef<MapInspection | null>(null);
  const pinnableRef = useRef(pinnable);
  pinnableRef.current = pinnable;
  const latest = useRef(inspection);
  latest.current = inspection;
  const reveal = useRef(0);
  const leaving = useRef(0);
  const awaitingKey = useRef<string | null>(null);
  const [position, setPosition] = useState({ left: 0, top: 0, maxHeight: 0 });

  useEffect(() => {
    setHost(container.current?.querySelector<HTMLElement>(".mapview") ?? null);
    pinnedRef.current = false; shownRef.current = null;
    setPinned(false); setShown(null);
  }, [container, map]);

  useEffect(() => {
    if (pinnedRef.current) return;
    cancelAnimationFrame(leaving.current);
    if (!inspection) {
      clearTimeout(reveal.current);
      reveal.current = 0; awaitingKey.current = null;
      // Enter on a neighbouring cell cancels this before the next paint.
      leaving.current = requestAnimationFrame(() => {
        shownRef.current = null; setShown(null);
      });
      return;
    }
    const key = featureKey(inspection.feature);
    if (shownRef.current && featureKey(shownRef.current.feature) === key) {
      shownRef.current = inspection; setShown(inspection);
      return;
    }
    if (awaitingKey.current === key) return;
    clearTimeout(reveal.current);
    awaitingKey.current = key;
    shownRef.current = null; setShown(null);
    reveal.current = window.setTimeout(() => {
      reveal.current = 0; awaitingKey.current = null;
      if (!latest.current || featureKey(latest.current.feature) !== key || pinnedRef.current) return;
      shownRef.current = latest.current; setShown(latest.current);
    }, 120);
  }, [inspection]);

  useEffect(() => {
    const close = () => {
      clearTimeout(reveal.current);
      reveal.current = 0; awaitingKey.current = null;
      cancelAnimationFrame(leaving.current);
      pinnedRef.current = false; shownRef.current = null;
      setPinned(false); setShown(null);
    };
    let press: [number, number] | null = null;
    const outside = (event: PointerEvent) => {
      press = [event.clientX, event.clientY];
      if (!(event.target instanceof Element) || panel.current?.contains(event.target)) return;
      const feature = event.target.closest("[data-map-feature]");
      if (!pinnedRef.current || !feature || !host?.contains(feature)) close();
    };
    const pin = (event: MouseEvent | KeyboardEvent) => {
      if (!(event.target instanceof Element) || panel.current?.contains(event.target)) return;
      const element = event.target.closest("[data-map-feature]");
      if (!element || !host?.contains(element)) return;
      if (!pinnableRef.current) { close(); return; }
      if (event instanceof MouseEvent && event.detail !== 0 && press && Math.hypot(event.clientX - press[0], event.clientY - press[1]) >= 4) return;
      const feature = JSON.parse(element.getAttribute("data-map-feature")!) as MapFeature;
      const box = element.getBoundingClientRect();
      const reading: MapInspection = { feature, at: event instanceof MouseEvent && event.detail !== 0 ? [event.clientX, event.clientY] : [box.x + box.width / 2, box.y + box.height / 2] };
      clearTimeout(reveal.current);
      reveal.current = 0; awaitingKey.current = null;
      cancelAnimationFrame(leaving.current);
      pinnedRef.current = true; shownRef.current = reading;
      setPinned(true); setShown(reading);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Enter" || event.key === " ") { pin(event); return; }
      if (event.key !== "Escape" || !shownRef.current) return;
      event.preventDefault(); event.stopPropagation(); close();
    };
    document.addEventListener("pointerdown", outside, true);
    document.addEventListener("click", pin, true);
    document.addEventListener("keydown", escape, true);
    return () => {
      document.removeEventListener("pointerdown", outside, true);
      document.removeEventListener("click", pin, true);
      document.removeEventListener("keydown", escape, true);
      clearTimeout(reveal.current);
      cancelAnimationFrame(leaving.current);
    };
  }, [host]);

  const reading = shown ? gazette(shown.feature, map, overview, generation, { selectedVariety, riverNames, lakeNames, climate, known }) : null;
  if (reading && !onOpen) reading.open = null;
  useLayoutEffect(() => {
    if (!host || !shown || !panel.current) return;
    const measure = () => {
      const bounds = host.getBoundingClientRect();
      const card = panel.current!.getBoundingClientRect();
      const x = shown.at[0] - bounds.left, y = shown.at[1] - bounds.top;
      const left = x + 14 + card.width <= bounds.width - 8 ? x + 14 : x - card.width - 14;
      const top = y + 14 + card.height <= bounds.height - 8 ? y + 14 : y - card.height - 14;
      setPosition({ left: Math.max(8, Math.min(left, bounds.width - card.width - 8)), top: Math.max(8, Math.min(top, bounds.height - card.height - 8)), maxHeight: Math.max(0, bounds.height - 16) });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(host); observer.observe(panel.current);
    window.addEventListener("scroll", measure, true);
    return () => { observer.disconnect(); window.removeEventListener("scroll", measure, true); };
  }, [host, shown, generation, selectedVariety, known, overview]);

  if (!host || !reading) return null;
  return createPortal(<div ref={panel} className="map-inspector" role="dialog" aria-labelledby={id} data-pinned={pinned}
    data-feature={featureKey(reading.feature)} style={{ left: position.left, top: position.top, maxHeight: position.maxHeight || undefined }}
    onPointerDown={(event) => event.stopPropagation()} onClick={(event) => event.stopPropagation()}
    onWheel={(event) => event.stopPropagation()}
    onPointerEnter={() => cancelAnimationFrame(leaving.current)}
    onPointerLeave={() => { if (!pinnedRef.current) { shownRef.current = null; setShown(null); } }}>
    <header><h3 id={id}>{reading.title}</h3><button type="button" className="link inspector-close" aria-label="Close place inspector"
      onClick={() => { pinnedRef.current = false; shownRef.current = null; setPinned(false); setShown(null); }}>×</button></header>
    {reading.subtitle ? <p className="inspector-subtitle">{reading.subtitle}</p> : null}
    {reading.sayings.length ? <ul className="inspector-sayings">{reading.sayings.map((saying, index) => <li key={`${saying.variety}:${saying.relation}:${index}`} data-relation={saying.relation}>
      <div className="inspector-language">{saying.language}<small>{RELATION[saying.relation]}{saying.relation === "former" && saying.since !== undefined ? ` · ${saying.since * YEARS}` : ""}</small></div>
      <span className="inspector-spelled">{saying.spelled}</span>{saying.ipa ? <> <span className="ipa">/{saying.ipa}/</span></> : null}
      {saying.meaning ? <span className="inspector-meaning">“{saying.meaning}”</span> : null}
    </li>)}</ul> : null}
    {reading.facts.length ? <dl className="inspector-facts">{reading.facts.map((fact) => <div key={fact.label}><dt>{fact.label}</dt><dd>{fact.value}</dd></div>)}</dl> : null}
    <footer>{reading.open ? <button type="button" className="link" onClick={() => { onOpen?.(reading.open!.subject); pinnedRef.current = false; shownRef.current = null; setPinned(false); setShown(null); }}>{reading.open.label} →</button> : null}
      {!pinned && pinnable ? <small>Click the place to keep it open</small> : null}</footer>
  </div>, host);
}

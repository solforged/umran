import { Globe2, Map as MapIcon } from "lucide-react";
import { useId, useLayoutEffect, useRef, useState, type RefObject } from "react";
import type { MapProjection } from "../cartography";

const KEY = "umran.projection";

// A world's reading place wins over the older, general preference.
export function useMapProjection(container: RefObject<HTMLElement | null>, initial?: MapProjection): [MapProjection, (projection: MapProjection) => void] {
  const [saved] = useState<MapProjection | null>(() => {
    if (initial) return initial;
    try {
      const choice = localStorage.getItem(KEY);
      return choice === "globe" || choice === "chart" ? choice : null;
    } catch { return null; }
  });
  const [projection, setProjection] = useState<MapProjection>(saved ?? "chart");
  const chosen = useRef(saved !== null);
  useLayoutEffect(() => {
    const element = container.current;
    if (!element || chosen.current) return;
    const measure = () => {
      if (chosen.current) return;
      const { width, height } = element.getBoundingClientRect();
      if (!width || !height) return;
      setProjection(height > width ? "globe" : "chart");
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    measure();
    return () => observer.disconnect();
  }, [container]);
  return [projection, (next) => {
    chosen.current = true;
    setProjection(next);
    try { localStorage.setItem(KEY, next); }
    catch { /* The choice still applies when storage is unavailable. */ }
  }];
}

export function MapProjectionSwitch({ value, onChange }: {
  value: MapProjection;
  onChange: (projection: MapProjection) => void;
}) {
  const name = useId();
  return <div className="map-projection-switch" role="group" aria-label="View of the world">
    <label title="Read the world as a flat chart">
      <input type="radio" name={name} value="chart" checked={value === "chart"} onChange={() => onChange("chart")} />
      <span><MapIcon size={14} aria-hidden="true" /> Chart</span>
    </label>
    <label title="Turn the same world as a globe">
      <input type="radio" name={name} value="globe" checked={value === "globe"} onChange={() => onChange("globe")} />
      <span><Globe2 size={14} aria-hidden="true" /> Globe</span>
    </label>
  </div>;
}

import { Globe2, Map as MapIcon } from "lucide-react";
import { useId, useState } from "react";
import type { MapProjection } from "../cartography";

const KEY = "umran.projection";

// A reading preference, independent of the world's seed and history.
export function useMapProjection(): [MapProjection, (projection: MapProjection) => void] {
  const [projection, setProjection] = useState<MapProjection>(() => {
    try { return localStorage.getItem(KEY) === "globe" ? "globe" : "chart"; }
    catch { return "chart"; }
  });
  return [projection, (next) => {
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

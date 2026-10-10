import { useEffect, useMemo, useState } from "react";
import { hue } from "../lore";
import type { Forming, WorldMap } from "../model";
import { createCartography, sharedBorder } from "../cartography";

/// How long each drift frame stays before the next, and how long the
/// supercontinent and the finished coasts are held.
const FRAME_MS = 420;
const HOLD_MS = 1100;

/// The coasts forming: the supercontinent, the engine's ten plate-drift
/// steps, then the drawn coasts. Every frame is the generator's own state.
export function FormingChart({ map, forming, onDone }: { map: WorldMap; forming: Forming; onDone: () => void }) {
  const [frame, setFrame] = useState(0);
  const last = forming.frames.length - 1;
  const cartography = useMemo(() => createCartography(map), [map]);
  const regions = useMemo(() => map.regions.map((r) => cartography.region(r)), [map, cartography]);
  const drawn = useMemo(() => {
    const { land, plate } = forming.frames[frame];
    const plates = new Map<string, string[]>();
    let coast = "";
    let edges = "";
    for (const r of map.regions) {
      if (land[r.id] === "1") {
        const parts = plates.get(plate[r.id]) ?? [];
        parts.push(regions[r.id]);
        plates.set(plate[r.id], parts);
      }
      for (const n of r.neighbours) {
        if (n < r.id) continue;
        if (land[r.id] !== land[n]) coast += cartography.line(sharedBorder(map, r.id, n));
        else if (plate[r.id] !== plate[n]) edges += cartography.line(sharedBorder(map, r.id, n));
      }
    }
    return { plates: [...plates].map(([id, parts]) => ({ id, d: parts.join("") })), coast, edges };
  }, [forming, frame, map, regions, cartography]);

  useEffect(() => {
    const timer = window.setTimeout(() => frame < last ? setFrame(frame + 1) : onDone(),
      frame === 0 || frame === last ? HOLD_MS : FRAME_MS);
    return () => window.clearTimeout(timer);
  }, [frame, last, onDone]);

  const caption = frame === 0 ? "One land, before the plates moved"
    : frame === last ? "The coasts as drawn"
    : `The plates drift · age ${frame} of ${last - 1}`;
  return <div className="mapview forming-chart projection-chart" aria-label={caption} role="img">
    <svg viewBox={`0 0 ${map.width} ${map.height}`}>
      <g key={frame} className="forming-frame">
        {drawn.plates.map((p) => <path key={p.id} className="forming-plate" d={p.d} style={{ fill: hue(parseInt(p.id, 36)) }} />)}
        <path className="forming-edges" d={drawn.edges} />
        <path className="forming-coast" d={drawn.coast} />
      </g>
    </svg>
    <p className="forming-caption" aria-live="polite">{caption}</p>
    <button type="button" className="link forming-skip" onClick={onDone}>Skip</button>
  </div>;
}

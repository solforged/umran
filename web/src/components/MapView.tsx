import { useEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { Maximize, Minus, Plus } from "lucide-react";
import type { Community, Overview, WordMap, WorldMap } from "../model";
import { YEARS } from "../model";
import { hue, TERRAIN_NAME } from "../lore";

/// Gap between stacked labels of peoples sharing a region, in map units.
const LINE = 0.3;
/// Generations over which a route fades to its faintest.
const ROUTE_FADE = 40;
/// Colours of the isogloss view: land whose people underwent the change,
/// and peopled land that did not.
const CHANGED = "hsl(24 75% 50%)";
const UNCHANGED = "hsl(210 12% 55%)";
/// Farthest apart two corners can be and still be one corner of two
/// bordering lands, in map units.
const SAME_POINT = 1e-3;
/// Closest zoom, as a share of the map's width.
const CLOSEST = 1 / 8;
/// How long the view takes to glide to something, in milliseconds.
const GLIDE = 450;
/// Pointer travel, in pixels, before a press becomes a drag.
const DRAG_START = 4;

/// What colours the land: peoples by language family, every people's word
/// for one meaning by root, or who underwent one sound change.
export type Tint =
  | { kind: "peoples" }
  | { kind: "words"; words: WordMap }
  | { kind: "change"; had: ReadonlySet<number> };

/// The part of the map in view: left, top, width, height, in map units.
type Box = [number, number, number, number];

/// Living peoples by every region they hold, not just their heart land.
export function peoplesByRegion(overview: Overview): Map<number, Community[]> {
  const out = new Map<number, Community[]>();
  for (const c of overview.communities) {
    if (c.ended !== null) continue;
    for (const region of c.lands) {
      const here = out.get(region);
      if (here) here.push(c);
      else out.set(region, [c]);
    }
  }
  return out;
}

/// A gently curved path from `a` to `b`, stopping short of `b` so its
/// arrowhead does not cover the label there.
function route([ax, ay]: [number, number], [bx, by]: [number, number]): string {
  const [dx, dy] = [bx - ax, by - ay];
  const length = Math.hypot(dx, dy) || 1;
  const end: [number, number] = [bx - (dx / length) * 0.18, by - (dy / length) * 0.18];
  const bend: [number, number] = [ax + dx / 2 - dy * 0.2, ay + dy / 2 + dx * 0.2];
  return `M${ax},${ay} Q${bend[0]},${bend[1]} ${end[0]},${end[1]}`;
}

/// The world's map: every people where it lives, the dealings between
/// them, the roads they took, and the names of their lands. With
/// `zoomable`, wheel and drag move the view, and `focus` glides it to a
/// point when that point is out of the middle of the view.
export function MapView({
  map,
  overview,
  generation,
  tint,
  names = true,
  routes = true,
  contacts = true,
  chosen,
  lands,
  beacons = [],
  focus = null,
  zoomable = false,
  onPeople,
  onLand,
}: {
  map: WorldMap;
  overview: Overview;
  generation: number;
  tint: Tint;
  names?: boolean;
  routes?: boolean;
  contacts?: boolean;
  /// Peoples drawn as chosen.
  chosen: ReadonlySet<number>;
  /// Lands drawn outlined.
  lands: ReadonlySet<number>;
  /// Peoples marked with a pulse, for something that just happened.
  beacons?: readonly number[];
  /// A point to bring into view, in map units.
  focus?: [number, number] | null;
  zoomable?: boolean;
  onPeople: (community: number) => void;
  onLand: (region: number) => void;
}) {
  const full: Box = useMemo(() => [0, 0, map.width, map.height], [map]);
  const [box, setBox] = useState<Box>(full);
  const svg = useRef<SVGSVGElement>(null);
  const drag = useRef<{ x: number; y: number; scale: number; box: Box; moved: boolean } | null>(null);
  const glide = useRef(0);

  // Keep the view within the map, and no closer than `CLOSEST`.
  const clamp = ([x, y, w]: Box): Box => {
    const scale = Math.min(1, Math.max(CLOSEST, w / map.width));
    const [nw, nh] = [map.width * scale, map.height * scale];
    return [Math.min(Math.max(x, 0), map.width - nw), Math.min(Math.max(y, 0), map.height - nh), nw, nh];
  };

  const glideTo = (target: Box) => {
    cancelAnimationFrame(glide.current);
    const from = box;
    const start = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / GLIDE);
      const ease = 1 - (1 - t) ** 3;
      setBox(from.map((v, i) => v + (target[i] - v) * ease) as Box);
      if (t < 1) glide.current = requestAnimationFrame(step);
    };
    glide.current = requestAnimationFrame(step);
  };

  // Bring the focus into view if it lies outside the middle of the view.
  const [fx, fy] = focus ?? [NaN, NaN];
  useEffect(() => {
    if (!zoomable || Number.isNaN(fx)) return;
    const [x, y, w, h] = box;
    const inside = fx > x + w * 0.2 && fx < x + w * 0.8 && fy > y + h * 0.2 && fy < y + h * 0.8;
    if (!inside) glideTo(clamp([fx - w / 2, fy - h / 2, w, h]));
    // Only a new focus moves the view, never the view itself.
  }, [fx, fy, zoomable]);

  useEffect(() => () => cancelAnimationFrame(glide.current), []);

  /// The map point under a pointer.
  const pointAt = (clientX: number, clientY: number): [number, number] => {
    const ctm = svg.current?.getScreenCTM();
    if (!ctm) return [0, 0];
    const p = new DOMPoint(clientX, clientY).matrixTransform(ctm.inverse());
    return [p.x, p.y];
  };

  const zoom = (factor: number, [px, py]: [number, number]) => {
    cancelAnimationFrame(glide.current);
    setBox(([x, y, w, h]) => {
      const scale = Math.min(1, Math.max(CLOSEST, (w * factor) / map.width)) / (w / map.width);
      return clamp([px - (px - x) * scale, py - (py - y) * scale, w * scale, h * scale]);
    });
  };

  // Wheel zoom needs a listener that may prevent scrolling the page.
  useEffect(() => {
    const element = svg.current;
    if (!zoomable || !element) return;
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      zoom(Math.exp(e.deltaY * 0.0015), pointAt(e.clientX, e.clientY));
    };
    element.addEventListener("wheel", wheel, { passive: false });
    return () => element.removeEventListener("wheel", wheel);
  });

  const down = (e: PointerEvent<SVGSVGElement>) => {
    if (!zoomable || e.button !== 0) return;
    const ctm = svg.current?.getScreenCTM();
    drag.current = { x: e.clientX, y: e.clientY, scale: ctm ? 1 / ctm.a : 0, box, moved: false };
  };
  const move = (e: PointerEvent<SVGSVGElement>) => {
    const d = drag.current;
    if (!d) return;
    const [dx, dy] = [e.clientX - d.x, e.clientY - d.y];
    if (!d.moved && Math.hypot(dx, dy) < DRAG_START) return;
    if (!d.moved) {
      d.moved = true;
      cancelAnimationFrame(glide.current);
      svg.current?.setPointerCapture(e.pointerId);
    }
    const [x, y, w, h] = d.box;
    setBox(clamp([x - dx * d.scale, y - dy * d.scale, w, h]));
  };
  const up = () => {
    // A drag ends without a click on whatever it ended over.
    if (drag.current?.moved) window.setTimeout(() => (drag.current = null));
    else drag.current = null;
  };
  const dragged = () => drag.current?.moved === true;

  const byRegion = useMemo(() => peoplesByRegion(overview), [overview]);
  const hearts = useMemo(
    () => new Map([...byRegion].map(([region, here]) => [region, here.filter((c) => c.region === region)])),
    [byRegion],
  );
  // Peoples sharing a region stack their labels around its centre.
  const at = useMemo(() => {
    const out = new Map<number, [number, number]>();
    for (const [region, here] of hearts) {
      const [x, y] = map.regions[region].site;
      here.forEach((c, i) => out.set(c.id, [x, y + (i - (here.length - 1) / 2) * LINE]));
    }
    return out;
  }, [hearts, map]);
  const placeOf = useMemo(() => new Map(overview.places.map((p) => [p.region, p.names])), [overview.places]);
  const nameOf = (region: number): string | null => placeOf.get(region)?.at(-1)?.spelled ?? null;
  const family = (c: Community) => overview.varieties[c.variety].family;
  const wordBy = useMemo(
    () => new Map(tint.kind === "words" ? tint.words.words.map((w) => [w.community, w]) : []),
    [tint],
  );

  // The edge two bordering lands share, for drawing isoglosses on.
  const borders = useMemo(() => {
    const out: { a: number; b: number; ends: [number, number][] }[] = [];
    for (const r of map.regions) {
      if (r.terrain === "sea") continue;
      for (const n of r.neighbours) {
        const other = map.regions[n];
        if (n < r.id || other.terrain === "sea") continue;
        const ends = r.outline.filter(([x, y]) =>
          other.outline.some(([u, v]) => Math.abs(x - u) < SAME_POINT && Math.abs(y - v) < SAME_POINT),
        );
        if (ends.length >= 2) out.push({ a: r.id, b: n, ends: ends.slice(0, 2) });
      }
    }
    return out;
  }, [map]);

  // A region takes the colour of its largest people: its family, the root
  // of that people's word, or whether that people underwent the change.
  const largestOn = (region: number): Community | null => {
    const here = byRegion.get(region);
    return here ? here.reduce((a, b) => (b.size > a.size ? b : a)) : null;
  };
  const colourOf = (region: number): string | null => {
    const largest = largestOn(region);
    if (!largest) return null;
    switch (tint.kind) {
      case "peoples":
        return hue(family(largest));
      case "change":
        return tint.had.has(largest.id) ? CHANGED : UNCHANGED;
      case "words": {
        const word = wordBy.get(largest.id);
        return word ? hue(word.group) : null;
      }
    }
  };
  // Where the change stopped: borders between peopled lands whose peoples
  // differ in having it.
  const isoglosses =
    tint.kind === "change"
      ? borders.filter(({ a, b }) => {
          const [pa, pb] = [largestOn(a), largestOn(b)];
          return pa && pb && tint.had.has(pa.id) !== tint.had.has(pb.id);
        })
      : [];
  // Peoples on the same land need no line between them.
  const dealings = contacts
    ? overview.contacts.filter((k) => {
        const [a, b] = [overview.communities[k.a], overview.communities[k.b]];
        return a.ended === null && b.ended === null && a.region !== b.region;
      })
    : [];

  // Labels grow more slowly than the land as the view closes in.
  const label = Math.sqrt(box[2] / map.width);

  return (
    <div className={zoomable ? "mapview zoomable" : "mapview"}>
      <svg
        ref={svg}
        viewBox={box.join(" ")}
        role="img"
        aria-label={`Map of ${map.regions.length} lands in year ${generation * YEARS}`}
        style={{ "--label": label } as CSSProperties}
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        onPointerCancel={up}
      >
        <defs>
          <marker
            id="route-head"
            viewBox="0 0 10 10"
            refX="6"
            refY="5"
            markerWidth="5"
            markerHeight="5"
            orient="auto-start-reverse"
          >
            <path className="route-head" d="M0,0 L10,5 L0,10 z" />
          </marker>
        </defs>
        <g className="lands">
          {map.regions.map((r) => {
            const colour = colourOf(r.id);
            const points = r.outline.map(([x, y]) => `${x},${y}`).join(" ");
            const sea = r.terrain === "sea";
            return (
              <g key={r.id} onClick={sea ? undefined : () => dragged() || onLand(r.id)}>
                <title>{sea ? "Sea" : (nameOf(r.id) ?? `Unnamed ${TERRAIN_NAME[r.terrain].toLowerCase()}`)}</title>
                <polygon
                  className={`land terrain-${r.terrain}${sea ? "" : " open"}${lands.has(r.id) ? " shown" : ""}`}
                  points={points}
                />
                {colour ? <polygon className="claim" points={points} style={{ fill: colour }} /> : null}
              </g>
            );
          })}
        </g>
        <g className="isoglosses">
          {isoglosses.map(({ a, b, ends: [[x1, y1], [x2, y2]] }) => (
            <line key={`${a}-${b}`} className="isogloss" x1={x1} y1={y1} x2={x2} y2={y2} />
          ))}
        </g>
        {names ? (
          <g className="place-names" aria-hidden="true">
            {overview.places.map((p) => {
              const [x, y] = map.regions[p.region].site;
              const here = hearts.get(p.region)?.length ?? 0;
              const top = here > 0 ? y - (here / 2) * LINE * label - 0.06 * label : y;
              return (
                <text key={p.region} x={x} y={top} className={here > 0 ? "place-name" : "place-name left"}>
                  {p.names.at(-1)!.spelled}
                </text>
              );
            })}
          </g>
        ) : null}
        {routes ? (
          <g className="routes">
            {overview.moves.map((m, i) => {
              const mover = overview.communities[m.community];
              if (mover.ended !== null) return null;
              const age = generation - m.generation;
              const mine = chosen.has(m.community);
              return (
                <path
                  key={i}
                  className={`route${m.overseas ? " overseas" : ""}${mine ? " chosen" : ""}`}
                  d={route(map.regions[m.from].site, map.regions[m.to].site)}
                  markerEnd="url(#route-head)"
                  style={{
                    stroke: hue(family(mover)),
                    strokeOpacity: mine ? 1 : 0.3 + 0.6 * Math.max(0, 1 - age / ROUTE_FADE),
                  }}
                >
                  <title>
                    {`${mover.name} ${m.kind === "split" ? "went out" : "moved"} to ${nameOf(m.to) ?? "new land"}${m.overseas ? " by sea" : ""}, year ${m.generation * YEARS}`}
                  </title>
                </path>
              );
            })}
          </g>
        ) : null}
        <g className="dealings">
          {dealings.map((k, i) => {
            const [ax, ay] = at.get(k.a)!;
            const [bx, by] = at.get(k.b)!;
            return (
              <line
                key={i}
                className={`dealing dealing-${k.kind}`}
                x1={ax}
                y1={ay}
                x2={bx}
                y2={by}
                style={{ strokeOpacity: 0.35 + 0.65 * k.intensity }}
              />
            );
          })}
        </g>
        <g className="beacons" aria-hidden="true">
          {beacons.map((id) => {
            const point = at.get(id);
            if (!point) return null;
            return <circle key={`${id}-${generation}`} className="beacon" cx={point[0]} cy={point[1]} r={0.5} />;
          })}
        </g>
        <g className="peoples">
          {overview.communities.map((c) => {
            if (c.ended !== null) return null;
            const [x, y] = at.get(c.id)!;
            const word = wordBy.get(c.id);
            const text = tint.kind === "words" ? (word?.spelled ?? "—") : c.name;
            return (
              <g
                key={c.id}
                className={chosen.has(c.id) ? "people chosen" : "people"}
                role="button"
                tabIndex={0}
                aria-label={tint.kind === "words" ? `${c.name}: ${text}` : c.name}
                onClick={() => dragged() || onPeople(c.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    onPeople(c.id);
                  }
                }}
              >
                <title>
                  {tint.kind === "words" && word
                    ? `${c.name}: ${word.spelled} /${word.ipa}/`
                    : `${c.name}, ${Math.round(c.size).toLocaleString()} souls`}
                </title>
                <text
                  x={x}
                  y={y}
                  className={`hand-${family(c) % 5}`}
                  style={{ fill: tint.kind === "words" && word ? hue(word.group) : hue(family(c)) }}
                >
                  {text}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
      {zoomable ? (
        <div className="map-zoom">
          <button type="button" title="Closer" onClick={() => zoom(0.7, [box[0] + box[2] / 2, box[1] + box[3] / 2])}>
            <Plus size={16} />
          </button>
          <button type="button" title="Farther" onClick={() => zoom(1 / 0.7, [box[0] + box[2] / 2, box[1] + box[3] / 2])}>
            <Minus size={16} />
          </button>
          <button type="button" title="The whole map" onClick={() => glideTo(full)}>
            <Maximize size={16} />
          </button>
        </div>
      ) : null}
    </div>
  );
}

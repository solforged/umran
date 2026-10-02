import { useEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent } from "react";
import { Maximize, Minus, Plus } from "lucide-react";
import type { Community, Craft, Overview, WordMap, WorldMap } from "../model";
import type { ShelfPeople } from "../shelf";
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

/// What colours the land: language families, word roots, a sound change,
/// faiths, or the holders of one craft.
export type Tint =
  | { kind: "peoples" }
  | { kind: "words"; words: WordMap }
  | { kind: "change"; had: ReadonlySet<number> }
  | { kind: "faiths" }
  | { kind: "crafts"; craft: Craft };

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

/// Rings of ripple lines off the coast, outermost first: how far each
/// reaches into the sea, in map units, and how dark its line is.
const RIPPLES: [number, number][] = [
  [0.42, 0.1],
  [0.3, 0.12],
  [0.19, 0.16],
  [0.1, 0.25],
];
/// Where the compass lines radiate from, as shares of the map's width and
/// height, and how many lines each sends out.
const ROSES: [number, number][] = [
  [0.18, 0.36],
  [0.7, 0.5],
];
const RHUMBS = 32;

/// A small seeded stream for scattering the chart's marks. It is drawing
/// only, so it keeps clear of the engine's streams.
function scatter(seed: number): () => number {
  let s = (Math.imul(seed + 1, 2654435761) >>> 0) || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return (s >>> 0) / 2 ** 32;
  };
}

/// Whether a point lies inside an outline.
function inside(outline: [number, number][], [x, y]: [number, number]): boolean {
  let within = false;
  for (let i = 0, j = outline.length - 1; i < outline.length; j = i++) {
    const [xi, yi] = outline[i];
    const [xj, yj] = outline[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) within = !within;
  }
  return within;
}

/// What an old chart draws besides the lands' colours: compass lines over
/// the sea, ripples off the coast, an inked coastline, and small marks for
/// each land's terrain (styled in chart.css). It depends on the map alone,
/// so it is built once per map.
function chartDress(map: WorldMap) {
  const land = map.regions.filter((r) => r.terrain !== "sea");
  const coast = land.map((r) => `M${r.outline.map(([x, y]) => `${x},${y}`).join("L")}Z`).join("");
  const reach = Math.hypot(map.width, map.height);
  const rhumbs = ["", "", ""];
  for (const [sx, sy] of ROSES) {
    const [cx, cy] = [sx * map.width, sy * map.height];
    for (let i = 0; i < RHUMBS; i++) {
      const a = (i * 2 * Math.PI) / RHUMBS;
      rhumbs[i % 4 === 0 ? 0 : i % 2 === 0 ? 1 : 2] += `M${cx},${cy}L${cx + Math.cos(a) * reach},${cy + Math.sin(a) * reach}`;
    }
  }
  // Marks by kind; each kind is one path, so the chart stays light.
  const marks = { peak: "", shade: "", hill: "", tree: "", sand: "", grass: "", field: "" };
  for (const r of land) {
    const next = scatter(r.id);
    const [cx, cy] = r.site;
    const spots = (count: number, place: (x: number, y: number) => void) => {
      for (let placed = 0, tries = 0; placed < count && tries < 60; tries++) {
        const [x, y] = [cx + (next() - 0.5) * 0.9, cy + (next() - 0.5) * 0.8];
        // Keep the whole mark inside, not just its foot.
        if (inside(r.outline, [x, y]) && inside(r.outline, [x + 0.13, y]) && inside(r.outline, [x - 0.13, y - 0.17])) {
          place(x, y);
          placed++;
        }
      }
    };
    switch (r.terrain) {
      case "mountains":
        spots(3, (x, y) => {
          const s = 0.13 + next() * 0.05;
          marks.peak += `M${x - s},${y}L${x},${y - s * 1.5}L${x + s},${y}`;
          marks.shade += `M${x + s * 0.15},${y - s * 1.2}L${x + s * 0.65},${y}`;
        });
        break;
      case "hills":
        spots(3, (x, y) => (marks.hill += `M${x - 0.11},${y}Q${x},${y - 0.16} ${x + 0.11},${y}`));
        break;
      case "forest":
        spots(6, (x, y) => {
          marks.tree += `M${x - 0.045},${y - 0.07}a.045,.045 0 1,0 .09,0a.045,.045 0 1,0 -.09,0M${x},${y - 0.025}V${y + 0.03}`;
        });
        break;
      case "desert":
        spots(14, (x, y) => (marks.sand += `M${x - 0.008},${y}a.008,.008 0 1,0 .016,0a.008,.008 0 1,0 -.016,0`));
        break;
      case "steppe":
        spots(6, (x, y) => (marks.grass += `M${x - 0.04},${y}l.02,-.05M${x},${y}v-.06M${x + 0.04},${y}l-.02,-.05`));
        break;
      case "plains":
        spots(3, (x, y) => (marks.field += `M${x - 0.06},${y}h.12`));
        break;
    }
  }
  return {
    under: (
      <g className="chart-dress" aria-hidden="true">
        <g className="chart-rhumbs">
          {rhumbs.map((d, i) => (
            <path key={i} className={`rhumb rhumb-${i}`} d={d} />
          ))}
        </g>
        <g className="chart-ripples">
          {RIPPLES.map(([width, dark]) => (
            <g key={width}>
              <path className="ripple" d={coast} style={{ strokeWidth: width, strokeOpacity: dark }} />
              <path className="ripple-gap" d={coast} style={{ strokeWidth: width - 0.035 }} />
            </g>
          ))}
        </g>
        <path className="chart-coast" d={coast} />
      </g>
    ),
    over: (
      <g className="chart-dress chart-marks" aria-hidden="true">
        {Object.entries(marks).map(([kind, d]) => (d ? <path key={kind} className={`mark-${kind}`} d={d} /> : null))}
      </g>
    ),
  };
}

/// A compass rose for the chart's corner, north in the rubric.
const ROSE = (
  <svg className="chart-rose" viewBox="-1 -1.15 2 2.2" aria-hidden="true">
    <circle r={0.82} className="rose-ring" />
    <circle r={0.72} className="rose-ring thin" />
    {Array.from({ length: 8 }, (_, i) => {
      const a = (i * Math.PI) / 4;
      const long = i % 2 === 0 ? 0.92 : 0.5;
      const [x, y, px, py] = [Math.sin(a) * long, -Math.cos(a) * long, Math.cos(a) * 0.1, Math.sin(a) * 0.1];
      return (
        <g key={i}>
          <path className={i === 0 ? "rose-point north" : "rose-point"} d={`M0,0L${px},${py}L${x},${y}Z`} />
          <path className="rose-point light" d={`M0,0L${-px},${-py}L${x},${y}Z`} />
        </g>
      );
    })}
    <text y={-0.98} className="rose-north">
      N
    </text>
  </svg>
);

/// How many peoples a miniature names; the rest only colour their lands.
const NAMED = 6;

/// A world drawn small, for the shelf: the chart's dress, each people's
/// lands in its family's colour, and the largest peoples named in their
/// hands. A map without regions is a blank sheet of compass lines.
export function Miniature({ map, peoples = [] }: { map: WorldMap; peoples?: readonly ShelfPeople[] }) {
  const dress = useMemo(() => chartDress(map), [map]);
  const points = useMemo(() => map.regions.map((r) => r.outline.map(([x, y]) => `${x},${y}`).join(" ")), [map]);
  // Largest first, so where two peoples share a land the largest colours it.
  const holder = useMemo(() => {
    const out = new Map<number, number>();
    for (const p of peoples) for (const land of p.lands) if (!out.has(land)) out.set(land, p.family);
    return out;
  }, [peoples]);
  const named = peoples.slice(0, NAMED);
  const shape = (r: WorldMap["regions"][number]) => {
    const family = holder.get(r.id);
    return (
      <g key={r.id}>
        <polygon className={`land terrain-${r.terrain}`} points={points[r.id]} />
        {family === undefined ? null : <polygon className="claim" points={points[r.id]} style={{ fill: hue(family) }} />}
      </g>
    );
  };
  return (
    <div className="mapview miniature">
      <svg viewBox={`0 0 ${map.width} ${map.height}`} aria-hidden="true">
        <g className="lands">
          {map.regions.filter((r) => r.terrain === "sea").map(shape)}
          {dress.under}
          {map.regions.filter((r) => r.terrain !== "sea").map(shape)}
        </g>
        {dress.over}
        <g className="peoples">
          {named.map((p, i) => {
            const [x, y] = map.regions[p.region].site;
            const above = named.slice(0, i).filter((q) => q.region === p.region).length;
            return (
              <g key={i} className="people">
                <text x={x} y={y + above * LINE * 2} className={`hand-${p.family % 5}`} style={{ fill: hue(p.family) }}>
                  {p.name}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
      {ROSE}
    </div>
  );
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
  states = false,
  chosen,
  lands,
  beacons = [],
  focus = null,
  known = null,
  zoomable = false,
  onPeople,
  onLand,
  onContinent,
  onState,
  onReligion,
  onCraft,
}: {
  map: WorldMap;
  overview: Overview;
  generation: number;
  tint: Tint;
  names?: boolean;
  routes?: boolean;
  contacts?: boolean;
  states?: boolean;
  /// Peoples drawn as chosen.
  chosen: ReadonlySet<number>;
  /// Lands drawn outlined.
  lands: ReadonlySet<number>;
  /// Peoples marked with a pulse, for something that just happened.
  beacons?: readonly number[];
  /// A point to bring into view, in map units.
  focus?: [number, number] | null;
  /// Lands one language knows by name; the rest of the land is veiled,
  /// with the peoples and names on it.
  known?: Set<number> | null;
  zoomable?: boolean;
  onPeople: (community: number) => void;
  onLand: (region: number) => void;
  onContinent?: (landmass: number) => void;
  onState?: (state: number) => void;
  onReligion?: (religion: number) => void;
  onCraft?: (craft: Craft) => void;
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
  const dress = useMemo(() => chartDress(map), [map]);
  // Seas first, so the chart can ink the coast between them and the lands.
  const [seas, grounds] = useMemo(
    () => [map.regions.filter((r) => r.terrain === "sea"), map.regions.filter((r) => r.terrain !== "sea")],
    [map],
  );
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

  // Leave out shared edges within each realm, retaining coasts and map edges.
  const realms = useMemo(() => {
    if (!states) return [];
    const same = ([x, y]: [number, number], [u, v]: [number, number]) =>
      Math.abs(x - u) < SAME_POINT && Math.abs(y - v) < SAME_POINT;
    return overview.states.filter((s) => s.fell === null).map((state) => {
      const held = new Set(state.lands);
      const edges: string[] = [];
      for (const id of state.lands) {
        const r = map.regions[id];
        for (let i = 0; i < r.outline.length; i++) {
          const a = r.outline[i];
          const b = r.outline[(i + 1) % r.outline.length];
          if (same(a, b)) continue;
          const inside = r.neighbours.some((n) => held.has(n) &&
            map.regions[n].outline.some((p) => same(a, p)) &&
            map.regions[n].outline.some((p) => same(b, p)));
          if (!inside) edges.push(`M${a.join(",")}L${b.join(",")}`);
        }
      }
      return { state, border: edges.join(" ") };
    });
  }, [states, overview.states, map]);

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
      case "faiths":
        return largest.faith === null ? UNCHANGED : hue(largest.faith);
      case "crafts":
        return largest.crafts.includes(tint.craft) ? CHANGED : UNCHANGED;
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
  // Rule is always drawn; other dealings only for the chosen peoples, since
  // every dealing at once tangles the chart. Peoples on the same land need
  // no line between them.
  const dealings = contacts
    ? overview.contacts.filter((k) => {
        const [a, b] = [overview.communities[k.a], overview.communities[k.b]];
        if (a.ended !== null || b.ended !== null || a.region === b.region) return false;
        if (hidden(a.region) || hidden(b.region)) return false;
        return k.kind === "rule" || chosen.has(k.a) || chosen.has(k.b);
      })
    : [];
  // Every faith's holy places, side by side where several faiths revere one
  // land, and the roads their pilgrims walk, drawn under the faiths layer.
  const faiths = tint.kind === "faiths";
  const shrineMarks = faiths
    ? overview.religions.flatMap((religion) => religion.shrines.map((shrine) => ({ religion, shrine })))
      .map((mark, i, all) => ({ ...mark, offset: all.slice(0, i).filter((m) => m.shrine.region === mark.shrine.region).length }))
    : [];
  const pilgrimRoads = faiths
    ? overview.religions.flatMap((religion) => religion.pilgrims
      .filter((p) => !p.path.some(hidden))
      .map((p) => ({ religion, p })))
    : [];
  // Labels grow more slowly than the land as the view closes in.
  const label = Math.sqrt(box[2] / map.width);
  // Land a veiling language does not know.
  function hidden(region: number): boolean {
    return known !== null && !known.has(region);
  }

  // Where names would overprint each other, the larger people keeps its name
  // and the smaller shows as a dot until the view closes in, as a chart
  // letters only what room allows. Capitals are lettered first, and chosen
  // peoples always keep theirs. Glyphs in the map's hands run up to about
  // 0.63 em wide.
  const crowded = new Set<number>();
  const lettered: [number, number, number, number][] = [];
  // Where each capital's mark sits: below the names of the peoples there.
  const capitalAt = new Map<number, [number, number]>();
  for (const { state } of realms) {
    const [x, y] = map.regions[state.capital].site;
    const below = y + ((hearts.get(state.capital)?.length ?? 0) / 2 * LINE + 0.2) * label;
    capitalAt.set(state.id, [x, below]);
    const em = 0.21 * label;
    const half = state.name.length * em * 0.33;
    const middle = below + 0.27 * label;
    lettered.push([x - half, middle - em / 2, x + half, middle + em / 2]);
  }
  const byClaim = overview.communities
    .filter((c) => c.ended === null && !hidden(c.region))
    .sort((a, b) => Number(chosen.has(b.id)) - Number(chosen.has(a.id)) || b.size - a.size);
  for (const c of byClaim) {
    const [x, y] = at.get(c.id)!;
    const em = (chosen.has(c.id) ? 0.3 : 0.24) * label;
    const text = tint.kind === "words" ? (wordBy.get(c.id)?.spelled ?? "—") : c.name;
    const half = text.length * em * 0.33;
    const extent: [number, number, number, number] = [x - half, y - em / 2, x + half, y + em / 2];
    const overprints = lettered.some(([x0, y0, x1, y1]) => x0 < extent[2] && extent[0] < x1 && y0 < extent[3] && extent[1] < y1);
    if (overprints && !chosen.has(c.id)) crowded.add(c.id);
    else lettered.push(extent);
  }

  const shape = (r: WorldMap["regions"][number]) => {
    const sea = r.terrain === "sea";
    const veiled = !sea && hidden(r.id);
    const colour = veiled ? null : colourOf(r.id);
    const points = r.outline.map(([x, y]) => `${x},${y}`).join(" ");
    return (
      <g key={r.id} onClick={sea ? undefined : () => dragged() || onLand(r.id)}>
        <title>{sea ? "Sea" : veiled ? "Unknown land" : (nameOf(r.id) ?? `Unnamed ${TERRAIN_NAME[r.terrain].toLowerCase()}`)}</title>
        <polygon
          className={`land terrain-${r.terrain}${sea ? "" : " open"}${lands.has(r.id) ? " shown" : ""}`}
          points={points}
        />
        {colour ? <polygon className="claim" points={points} style={{ fill: colour }} /> : null}
      </g>
    );
  };

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
          {seas.map(shape)}
          {dress.under}
          {grounds.map(shape)}
        </g>
        {dress.over}
        {known ? (
          <g className="veil" aria-hidden="true">
            {grounds.filter((r) => hidden(r.id)).map((r) => (
              <polygon key={r.id} points={r.outline.map(([x, y]) => `${x},${y}`).join(" ")} />
            ))}
          </g>
        ) : null}
        <g className="continent-names">
          {overview.continents.map((c) => {
            const mass = map.landmasses[c.landmass];
            if (!c.name || !mass || mass.regions.every(hidden)) return null;
            const [x, y] = map.regions[mass.anchor].site;
            return (
              <text
                key={c.landmass}
                x={x}
                y={y}
                className="continent-name"
                role={onContinent ? "button" : undefined}
                tabIndex={onContinent ? 0 : undefined}
                onClick={() => dragged() || onContinent?.(c.landmass)}
                onKeyDown={(e) => {
                  if (onContinent && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onContinent(c.landmass);
                  }
                }}
              >
                <title>{`${c.name.name}, “${c.name.meaning}”`}</title>
                {c.name.name}
              </text>
            );
          })}
        </g>
        <g className="isoglosses">
          {isoglosses.map(({ a, b, ends: [[x1, y1], [x2, y2]] }) => (
            <line key={`${a}-${b}`} className="isogloss" x1={x1} y1={y1} x2={x2} y2={y2} />
          ))}
        </g>
        <g className="state-borders" aria-hidden="true">
          {realms.map(({ state, border }) => (
            <path key={state.id} d={border} style={{ stroke: hue(state.id) }} />
          ))}
        </g>
        {names ? (
          <g className="place-names" aria-hidden="true">
            {overview.places.map((p) => {
              if (hidden(p.region)) return null;
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
              if (mover.ended !== null || hidden(m.from) || hidden(m.to)) return null;
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
        {pilgrimRoads.length > 0 ? (
          <g className="pilgrim-roads">
            {pilgrimRoads.map(({ religion, p }) => (
              <polyline
                key={`${religion.id}-${p.people}-${p.to}`}
                className="pilgrim-road"
                points={p.path.map((r) => map.regions[r].site.join(",")).join(" ")}
                style={{ stroke: hue(religion.id) }}
              >
                <title>{`Pilgrims of ${religion.name}, the ${overview.communities[p.people].name}, since year ${p.since * YEARS}`}</title>
              </polyline>
            ))}
          </g>
        ) : null}
        <g className="beacons" aria-hidden="true">
          {[...new Set(beacons)].map((id) => {
            const point = at.get(id);
            if (!point) return null;
            return <circle key={`${id}-${generation}`} className="beacon" cx={point[0]} cy={point[1]} r={0.5} />;
          })}
        </g>
        <g className="peoples">
          {overview.communities.map((c) => {
            if (c.ended !== null || hidden(c.region)) return null;
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
                {crowded.has(c.id) ? (
                  <circle className="people-dot" cx={x} cy={y} r={0.06 * label}
                    style={{ fill: tint.kind === "words" && word ? hue(word.group) : hue(family(c)) }} />
                ) : (
                  <text
                    x={x}
                    y={y}
                    className={`hand-${family(c) % 5}`}
                    style={{ fill: tint.kind === "words" && word ? hue(word.group) : hue(family(c)) }}
                  >
                    {text}
                  </text>
                )}
              </g>
            );
          })}
        </g>
        <g className="state-capitals">
          {realms.map(({ state }) => {
            const [x, below] = capitalAt.get(state.id)!;
            return (
              <g
                key={state.id}
                className="state-capital"
                transform={`translate(${x} ${below}) scale(${label})`}
                style={{ "--realm": hue(state.id) } as CSSProperties}
                role={onState ? "button" : undefined}
                tabIndex={onState ? 0 : undefined}
                aria-label={`${state.name}, capital city, ${Math.round(state.city).toLocaleString()} souls`}
                onClick={() => dragged() || onState?.(state.id)}
                onKeyDown={(e) => {
                  if (onState && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onState(state.id);
                  }
                }}
              >
                <title>{`${state.name}: ${Math.round(state.city).toLocaleString()} in its capital city`}</title>
                <path className="city-marker" d="M-.14,.04V-.08H-.08V-.16H.02V-.04H.08V-.11H.14V.04Z" />
                <text y={0.27} className={`hand-${family(overview.communities[state.rulers]) % 5}`}>{state.name}</text>
              </g>
            );
          })}
        </g>
        <g className="founding-markers">
          {tint.kind === "faiths" ? overview.religions.map((religion, i, religions) => {
            const [x, y] = map.regions[religion.land].site;
            const offset = religions.slice(0, i).filter((r) => r.land === religion.land).length;
            return (
              <g
                key={religion.id}
                className="founding-marker"
                transform={`translate(${x - (0.22 + offset * 0.24) * label} ${y - 0.22 * label}) scale(${label})`}
                style={{ "--mark": hue(religion.id) } as CSSProperties}
                role={onReligion ? "button" : undefined}
                tabIndex={onReligion ? 0 : undefined}
                aria-label={`${religion.name}, founded here`}
                onClick={() => dragged() || onReligion?.(religion.id)}
                onKeyDown={(e) => {
                  if (onReligion && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onReligion(religion.id);
                  }
                }}
              >
                <title>{`${religion.name}, founded here`}</title>
                <path d="M0,-.12L.09,0L0,.12L-.09,0Z" />
              </g>
            );
          }) : null}
          {shrineMarks.map(({ religion, shrine, offset }) => {
            const [x, y] = map.regions[shrine.region].site;
            return (
              <g
                key={`shrine-${religion.id}-${shrine.region}`}
                className="founding-marker shrine-marker"
                transform={`translate(${x + (0.24 + offset * 0.24) * label} ${y + 0.2 * label}) scale(${label})`}
                style={{ "--mark": hue(religion.id) } as CSSProperties}
                role={onReligion ? "button" : undefined}
                tabIndex={onReligion ? 0 : undefined}
                aria-label={`${shrine.name.name}, holy to ${religion.name}`}
                onClick={() => dragged() || onReligion?.(religion.id)}
                onKeyDown={(e) => {
                  if (onReligion && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onReligion(religion.id);
                  }
                }}
              >
                <title>{`${shrine.name.name}, holy to ${religion.name}`}</title>
                <path d="M0,-.13L.035,-.035L.13,0L.035,.035L0,.13L-.035,.035L-.13,0L-.035,-.035Z" />
              </g>
            );
          })}
          {tint.kind === "crafts" ? overview.crafts.find((c) => c.id === tint.craft)?.inventors.map((id) => {
            const people = overview.communities[id];
            const [x, y] = map.regions[people.region].site;
            return (
              <g
                key={id}
                className="founding-marker"
                transform={`translate(${x + 0.22 * label} ${y - 0.22 * label}) scale(${label})`}
                style={{ "--mark": CHANGED } as CSSProperties}
                role={onCraft ? "button" : undefined}
                tabIndex={onCraft ? 0 : undefined}
                aria-label={`${people.name}, inventor of ${tint.craft}`}
                onClick={() => dragged() || onCraft?.(tint.craft)}
                onKeyDown={(e) => {
                  if (onCraft && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onCraft(tint.craft);
                  }
                }}
              >
                <title>{`${people.name}, inventor of ${tint.craft}`}</title>
                <path d="M-.1,-.1H.1V.1H-.1Z" />
              </g>
            );
          }) : null}
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
      {ROSE}
    </div>
  );
}

import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent, type RefObject, type SetStateAction } from "react";
import { LocateFixed, Maximize, Minus, Plus } from "lucide-react";
import type { ClimateView, Community, Craft, EthosAxis, Overview, River, RiverNamesView, SettlementPreview, WordMap, WorldMap } from "../model";
import type { ShelfPeople } from "../shelf";
import { reducedMotion } from "../motion";
import { YEARS } from "../model";
import { hue, TERRAIN_NAME } from "../lore";

export interface MapMotionReading {
  overview: Overview;
  fills: Map<number, { owners: string; fill: string }>;
}

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
/// Land lettering is legible below 6 km/pixel; tongues need below 1.5.
/// Physical scale keeps the policy steady across maps of different sizes.
const LAND_KM_PER_PIXEL = 6;
const TONGUE_KM_PER_PIXEL = 1.5;
/// Breathing room between the measured, screen-space lettering boxes.
const LABEL_GAP = 2;

/// What colours the land: language families, word roots, a sound change,
/// faiths, the holders of one craft, or one leaning of temper.
export type Tint =
  | { kind: "peoples" }
  | { kind: "words"; words: WordMap }
  | { kind: "change"; had: ReadonlySet<number> }
  | { kind: "faiths" }
  | { kind: "crafts"; craft: Craft }
  | { kind: "temper"; axis: EthosAxis }
  | { kind: "weather" };

/// A leaning from -1 to 1 as a colour: the isogloss view's blue toward the
/// low end, its orange toward the high, grey between. Peoples seldom lean
/// past a half, so colour is full there.
function leaning(value: number): string {
  const strength = Math.min(1, Math.abs(value) / 0.5);
  return `hsl(${value < 0 ? 210 : 24} ${12 + 63 * strength}% ${55 - 5 * strength}%)`;
}

/// The part of the map in view: left, top, width, height, in map units.
export type MapCamera = [number, number, number, number];
type Box = MapCamera;

/// A tributary stops at its confluence; only the main course reaches the coast.
export function riverPoints(map: WorldMap, river: River): [number, number][] {
  const points = river.course.map((id) => map.regions[id].site);
  const end = map.regions[river.course.at(-1)!];
  if (river.joinAt !== null) {
    points.push(map.regions[river.joinAt].site);
  } else {
    const coast = end.outline.filter(([x, y]) => map.regions[river.mouth].outline.some(([u, v]) =>
      Math.abs(x - u) < SAME_POINT && Math.abs(y - v) < SAME_POINT));
    if (coast.length >= 2) points.push([(coast[0][0] + coast[1][0]) / 2, (coast[0][1] + coast[1][1]) / 2]);
  }
  return points;
}

export function riverLength(map: WorldMap, river: River): number {
  const points = riverPoints(map, river);
  return points.slice(1).reduce((km, point, i) =>
    km + Math.hypot(point[0] - points[i][0], point[1] - points[i][1]) * map.kmPerUnit, 0);
}

/// Use a real attested form in the selected speech, then the mouth's speech.
export function riverName(view: RiverNamesView, variety: number | undefined) {
  if (variety === undefined) return null;
  return [...view.names].reverse().find((name) => name.variety === variety)
    ?? view.exonyms.find((name) => name.variety === variety) ?? null;
}

function riverDress(map: WorldMap) {
  const curve = (points: [number, number][]) => {
    let d = `M${points[0].join(",")}`;
    for (let i = 0; i < points.length - 1; i++) {
      const a = points[Math.max(0, i - 1)], b = points[i], c = points[i + 1], e = points[Math.min(points.length - 1, i + 2)];
      d += `C${b[0] + (c[0] - a[0]) / 6},${b[1] + (c[1] - a[1]) / 6} ${c[0] - (e[0] - b[0]) / 6},${c[1] - (e[1] - b[1]) / 6} ${c.join(",")}`;
    }
    return d;
  };
  const largest = Math.max(1, ...map.rivers.map((river) => river.catchment.length));
  return map.rivers.map((river) => {
    const points = riverPoints(map, river);
    let longest = 0, start = points[0], end = points.at(-1)!;
    // Retain the longest run whose intermediate points stay near its chord.
    for (let i = 0; i < points.length - 1; i++) {
      for (let j = i + 1; j < points.length; j++) {
        const a = points[i], b = points[j], dx = b[0] - a[0], dy = b[1] - a[1], length = Math.hypot(dx, dy);
        if (length <= longest || !length) continue;
        if (points.slice(i + 1, j).some((p) => Math.abs(dx * (p[1] - a[1]) - dy * (p[0] - a[0])) / length > 0.18)) continue;
        longest = length; start = a; end = b;
      }
    }
    if (end[0] < start[0]) [start, end] = [end, start];
    return { river, d: curve(points), label: `M${start.join(",")}L${end.join(",")}`,
      tier: river.catchment.length >= largest * 2 / 3 ? 3 : river.catchment.length >= largest / 3 ? 2 : 1 };
  });
}

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
  const same = ([x, y]: [number, number], [u, v]: [number, number]) => Math.abs(x - u) < SAME_POINT && Math.abs(y - v) < SAME_POINT;
  const coast = land.flatMap((r) => r.outline.flatMap((a, i) => {
    const b = r.outline[(i + 1) % r.outline.length];
    return r.neighbours.some((n) => map.regions[n].terrain !== "sea" &&
      map.regions[n].outline.some((p) => same(a, p)) && map.regions[n].outline.some((p) => same(b, p)))
      ? [] : [`M${a.join(",")}L${b.join(",")}`];
  })).join("");
  const reach = Math.hypot(map.width, map.height);
  const rhumbs = ["", "", ""];
  for (const [sx, sy] of ROSES) {
    const [cx, cy] = [sx * map.width, sy * map.height];
    for (let i = 0; i < RHUMBS; i++) {
      const a = (i * 2 * Math.PI) / RHUMBS;
      rhumbs[i % 4 === 0 ? 0 : i % 2 === 0 ? 1 : 2] += `M${cx},${cy}L${cx + Math.cos(a) * reach},${cy + Math.sin(a) * reach}`;
    }
  }
  // Keep marks by land so a large chart can cull what lies beyond the view.
  const marked: { id: number; paths: Record<string, string> }[] = [];
  for (const r of land) {
    const marks = { peak: "", shade: "", hill: "", tree: "", sand: "", grass: "", field: "" };
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
    marked.push({ id: r.id, paths: marks });
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
        {marked.map(({ id, paths }) => <g key={id} data-chart-region={id}>
          {Object.entries(paths).map(([kind, d]) => d ? <path key={kind} className={`mark-${kind}`} d={d} /> : null)}
        </g>)}
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
  climate,
  riverNames = [],
  selectedVariety,
  names = true,
  routes = true,
  contacts = true,
  states = false,
  chosen,
  lands,
  focus = null,
  known = null,
  lens = null,
  zoomable = false,
  settlement = null,
  animateChanges = false,
  motionMemory,
  camera,
  onCamera,
  onPeople,
  onLand,
  onContinent,
  onState,
  onReligion,
  onCraft,
  onRiver,
}: {
  map: WorldMap;
  overview: Overview;
  generation: number;
  tint: Tint;
  climate?: ClimateView;
  riverNames?: RiverNamesView[];
  selectedVariety?: number;
  names?: boolean;
  routes?: boolean;
  contacts?: boolean;
  states?: boolean;
  /// Peoples drawn as chosen.
  chosen: ReadonlySet<number>;
  /// Lands drawn outlined.
  lands: ReadonlySet<number>;
  /// A point to bring into view, in map units.
  focus?: [number, number] | null;
  /// Lands one language knows by name; the rest of the land is veiled,
  /// with the peoples and names on it.
  known?: Set<number> | null;
  /// Read the chart in a people's tongue and within its known lands.
  lens?: { people: number } | null;
  zoomable?: boolean;
  settlement?: SettlementPreview | null;
  animateChanges?: boolean;
  motionMemory?: RefObject<MapMotionReading | null>;
  camera?: MapCamera;
  onCamera?: (camera: MapCamera) => void;
  onPeople: (community: number) => void;
  onLand: (region: number) => void;
  onContinent?: (landmass: number) => void;
  onState?: (state: number) => void;
  onReligion?: (religion: number) => void;
  onCraft?: (craft: Craft) => void;
  onRiver?: (id: number) => void;
}) {
  const reader = lens ? overview.communities.find((c) => c.id === lens.people) : undefined;
  const readerVariety = reader ? overview.varieties[reader.variety] : undefined;
  selectedVariety = readerVariety?.id ?? selectedVariety;
  const readerLands = useMemo(() => readerVariety
    ? new Set(readerVariety.knownLands.map((land) => land.region)) : null, [readerVariety]);
  known = readerLands ?? known;
  const full: Box = useMemo(() => [0, 0, map.width, map.height], [map]);
  const view = useRef<Box>(camera ?? full);
  const drawnMap = useRef(map);
  if (drawnMap.current !== map) { drawnMap.current = map; view.current = camera ?? full; }
  let box = view.current;
  const root = useRef<HTMLDivElement>(null);
  const scaleText = useRef<HTMLSpanElement>(null);
  const scaleRule = useRef<HTMLSpanElement>(null);
  const labelLayers = useRef<SVGElement[]>([]);
  const lettering = useRef<SVGTextElement[]>([]);
  const frame = useRef(0);
  const [labelWidth, setLabelWidth] = useState(() => window.innerWidth);
  const bounds = useMemo(() => map.regions.map((region) => {
    const xs = region.outline.map((p) => p[0]), ys = region.outline.map((p) => p[1]);
    return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
  }), [map]);
  const culled = useRef<{ node: SVGElement; bounds: number[]; hidden: boolean }[]>([]);
  const drawCamera = () => {
    const element = svg.current, chart = root.current;
    if (!element || !chart) return;
    const value = view.current, ratio = value[2] / map.width;
    element.setAttribute("viewBox", value.join(" "));
    const pixelsPerUnit = element.getScreenCTM()?.a ?? 1;
    for (const layer of labelLayers.current) {
      layer.style.setProperty("--label", String(Math.sqrt(ratio)));
      if (layer.classList.contains("peoples")) layer.style.setProperty("--people-label-min",
        zoomable && window.innerWidth <= 700 ? `${12 / pixelsPerUnit}px` : "0px");
    }
    const level = ratio > 0.6 ? "zoom-far" : ratio < 0.25 ? "zoom-close" : "zoom-mid";
    if (!chart.classList.contains(level)) {
      chart.classList.remove("zoom-far", "zoom-mid", "zoom-close");
      chart.classList.add(level);
    }
    if (map.regions.length > 500) {
      const [x, y, w, h] = value;
      for (const item of culled.current) {
        const [left, top, right, bottom] = item.bounds;
        const hidden = ratio <= 0.6 && (right < x - 0.2 || left > x + w + 0.2 || bottom < y - 0.2 || top > y + h + 0.2);
        if (hidden !== item.hidden) { item.node.style.display = hidden ? "none" : ""; item.hidden = hidden; }
      }
    }
    const kmPerPixel = map.kmPerUnit / pixelsPerUnit;
    chart.style.setProperty("--small-label-min", `${10 / pixelsPerUnit}px`);
    chart.style.setProperty("--state-weight", String(1 + 2 * Math.min(1, (1 - ratio) / 0.6)));
    const occupied: DOMRect[] = [];
    const named = new Set<string>();
    // Measure the actual font, not character-count estimates. Visibility
    // preserves geometry, so a suppressed label can return on the next frame.
    for (const text of lettering.current) text.style.visibility = "";
    const measured = lettering.current.map((text) => ({ text, rect: text.getBoundingClientRect() }));
    const chartBounds = chart.getBoundingClientRect();
    for (const { text, rect } of measured) {
      const kind = text.dataset.labelKind;
      const people = text.dataset.people;
      const allowed = kind === "tongue"
        ? tint.kind === "peoples" && kmPerPixel < TONGUE_KM_PER_PIXEL && named.has(people!)
        : kind === "land" || kind === "river" ? kmPerPixel < LAND_KM_PER_PIXEL : true;
      const inView = rect.right > chartBounds.left && rect.left < chartBounds.right &&
        rect.bottom > chartBounds.top && rect.top < chartBounds.bottom;
      const collides = occupied.some((other) => other.left < rect.right + LABEL_GAP && rect.left < other.right + LABEL_GAP &&
        other.top < rect.bottom + LABEL_GAP && rect.top < other.bottom + LABEL_GAP);
      const visible = allowed && inView && rect.width > 0 && !collides;
      text.style.visibility = visible ? "" : "hidden";
      if (visible) { occupied.push(rect); if (kind === "people") named.add(people!); }
      if (kind === "people") {
        const dot = text.parentElement?.querySelector<SVGCircleElement>(".people-dot");
        if (dot) dot.style.visibility = visible ? "hidden" : "";
      }
    }
    const km = [50, 100, 200, 500, 1000].filter((length) => length <= value[2] * map.kmPerUnit * 0.15).at(-1) ?? 50;
    scaleRule.current?.style.setProperty("--scale-width", `${km / map.kmPerUnit * pixelsPerUnit}px`);
    const caption = `${km.toLocaleString()} km`;
    if (scaleText.current && scaleText.current.textContent !== caption) scaleText.current.textContent = caption;
    const closer = chart.querySelector<HTMLButtonElement>('[aria-label="Zoom in"]');
    const farther = chart.querySelector<HTMLButtonElement>('[aria-label="Zoom out"]');
    if (closer) closer.disabled = ratio <= CLOSEST + 0.0001;
    if (farther) farther.disabled = ratio >= 0.9999;
  };
  const setBox = (next: SetStateAction<Box>) => {
    box = typeof next === "function" ? next(view.current) : next;
    view.current = box;
    if (onCamera) onCamera(box);
    cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(drawCamera);
  };
  useLayoutEffect(() => {
    if (camera) view.current = camera;
    culled.current = [...(root.current?.querySelectorAll<SVGElement>("[data-chart-region]") ?? [])].map((node) => ({
      node, bounds: bounds[Number(node.dataset.chartRegion)], hidden: node.style.display === "none",
    }));
    labelLayers.current = [...(root.current?.querySelectorAll<SVGElement>(".peoples, .place-names, .river-names, .state-capitals, .founding-markers") ?? [])];
    lettering.current = [...(root.current?.querySelectorAll<SVGTextElement>("[data-label-kind]") ?? [])]
      .sort((a, b) => Number(b.dataset.labelPriority) - Number(a.dataset.labelPriority));
    drawCamera();
  });
  useEffect(() => {
    const refresh = () => { setLabelWidth(window.innerWidth); drawCamera(); };
    const observer = new ResizeObserver(refresh);
    if (root.current) observer.observe(root.current);
    document.fonts.addEventListener("loadingdone", refresh);
    return () => {
      observer.disconnect(); document.fonts.removeEventListener("loadingdone", refresh);
      cancelAnimationFrame(frame.current);
    };
  }, [map, tint]);
  const routeHead = useId();
  const svg = useRef<SVGSVGElement>(null);
  const localMotion = useRef<MapMotionReading | null>(null);
  const drag = useRef<{ x: number; y: number; scale: number; box: Box; moved: boolean } | null>(null);
  const glide = useRef(0);
  const pointers = useRef(new Map<number, [number, number]>());
  const pinch = useRef<{ distance: number; middle: [number, number]; anchor: [number, number]; scale: number; box: Box } | null>(null);
  const suppressClick = useRef(false);

  // Keep the view within the map, and no closer than `CLOSEST`.
  const clamp = ([x, y, w]: Box): Box => {
    const scale = Math.min(1, Math.max(CLOSEST, w / map.width));
    const [nw, nh] = [map.width * scale, map.height * scale];
    return [Math.min(Math.max(x, 0), map.width - nw), Math.min(Math.max(y, 0), map.height - nh), nw, nh];
  };

  const glideTo = (target: Box) => {
    cancelAnimationFrame(glide.current);
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) { setBox(target); return; }
    const from = view.current;
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
    const [x, y, w, h] = view.current;
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

  const startDrag = (x: number, y: number, moved = false) => {
    const ctm = svg.current?.getScreenCTM();
    drag.current = { x, y, scale: ctm ? 1 / ctm.a : 0, box, moved };
  };
  const down = (e: PointerEvent<SVGSVGElement>) => {
    if (!zoomable || e.button !== 0) return;
    if (pointers.current.size === 0) suppressClick.current = false;
    pointers.current.set(e.pointerId, [e.clientX, e.clientY]);
    if (pointers.current.size === 1) startDrag(e.clientX, e.clientY);
    if (pointers.current.size === 2) {
      const [a, b] = [...pointers.current.values()];
      const middle: [number, number] = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
      pinch.current = { distance: Math.hypot(a[0] - b[0], a[1] - b[1]), middle,
        anchor: pointAt(...middle), scale: drag.current?.scale ?? 1, box };
      suppressClick.current = true;
      cancelAnimationFrame(glide.current);
      for (const id of pointers.current.keys()) svg.current?.setPointerCapture(id);
    }
  };
  const move = (e: PointerEvent<SVGSVGElement>) => {
    if (!pointers.current.has(e.pointerId)) return;
    pointers.current.set(e.pointerId, [e.clientX, e.clientY]);
    const p = pinch.current;
    if (p && pointers.current.size >= 2) {
      const [a, b] = [...pointers.current.values()];
      const distance = Math.max(1, Math.hypot(a[0] - b[0], a[1] - b[1]));
      const [x, y, w, h] = p.box;
      const factor = Math.min(1, Math.max(CLOSEST, w * p.distance / distance / map.width)) / (w / map.width);
      const dx = ((a[0] + b[0]) / 2 - p.middle[0]) * p.scale * factor;
      const dy = ((a[1] + b[1]) / 2 - p.middle[1]) * p.scale * factor;
      setBox(clamp([p.anchor[0] - (p.anchor[0] - x) * factor - dx,
        p.anchor[1] - (p.anchor[1] - y) * factor - dy, w * factor, h * factor]));
      return;
    }
    const d = drag.current;
    if (!d) return;
    const [dx, dy] = [e.clientX - d.x, e.clientY - d.y];
    if (!d.moved && Math.hypot(dx, dy) < DRAG_START) return;
    d.moved = true;
    suppressClick.current = true;
    cancelAnimationFrame(glide.current);
    svg.current?.setPointerCapture(e.pointerId);
    const [x, y, w, h] = d.box;
    setBox(clamp([x - dx * d.scale, y - dy * d.scale, w, h]));
  };
  const up = (e: PointerEvent<SVGSVGElement>) => {
    pointers.current.delete(e.pointerId);
    if (svg.current?.hasPointerCapture(e.pointerId)) svg.current.releasePointerCapture(e.pointerId);
    pinch.current = null;
    const remaining = [...pointers.current.values()][0];
    if (remaining) startDrag(...remaining, true);
    else drag.current = null;
  };
  const dragged = () => suppressClick.current;
  const selectedRegions = lands.size > 0 ? [...lands] : [...new Set(overview.communities.filter((c) => chosen.has(c.id)).flatMap((c) => c.lands))];
  const fitSelection = () => {
    const points = selectedRegions.flatMap((id) => map.regions[id]?.outline ?? []);
    if (points.length === 0) return;
    const xs = points.map((p) => p[0]), ys = points.map((p) => p[1]);
    const left = Math.min(...xs), right = Math.max(...xs), top = Math.min(...ys), bottom = Math.max(...ys);
    const scale = Math.max((right - left) / map.width, (bottom - top) / map.height) * 1.3;
    const w = map.width * Math.min(1, Math.max(CLOSEST, scale));
    const h = w * map.height / map.width;
    glideTo(clamp([(left + right - w) / 2, (top + bottom - h) / 2, w, h]));
  };

  const byRegion = useMemo(() => peoplesByRegion(overview), [overview]);
  const dress = useMemo(() => chartDress(map), [map]);
  const riverPaths = useMemo(() => riverDress(map), [map]);
  const namesByRiver = useMemo(() => new Map(riverNames.map((view) => [view.river, view])), [riverNames]);
  const namedRiver = (river: River) => {
    const view = namesByRiver.get(river.id);
    if (!view) return null;
    let main = river;
    while (main.joins !== null) main = map.rivers[main.joins];
    const atMouth = byRegion.get(main.course.at(-1)!);
    const mouthVariety = atMouth?.reduce((a, b) => a.size >= b.size ? a : b).variety;
    return riverName(view, selectedVariety) ?? riverName(view, mouthVariety);
  };
  const grounds = useMemo(() => map.regions.filter((r) => r.terrain !== "sea"), [map]);
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
  const placeOf = useMemo(() => new Map(overview.places.map((p) => [p.region, p])), [overview.places]);
  const speechLands = useMemo(() => new Map((readerVariety?.knownLands ?? []).map((land) => [land.region, land.spelled])), [readerVariety]);
  const nameOf = (region: number): string | null => {
    const place = placeOf.get(region);
    return speechLands.get(region) ?? place?.exonyms.find((name) => name.variety === selectedVariety)?.spelled ??
      place?.names.findLast((name) => name.variety === selectedVariety)?.spelled ?? place?.names.at(-1)?.spelled ?? null;
  };
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
    if (tint.kind === "weather") {
      const zone = climate?.zones.find((z) => z.id === map.regions[region].climateZone);
      if (!zone) return null;
      const strength = Math.max(Math.abs(zone.wetness), Math.abs(zone.warmth));
      if (strength < 0.04) return null;
      const ink = zone.wetness < -0.04 ? "#b28a43" : zone.wetness > 0.04 ? "#658174" : zone.warmth < -0.04 ? "#667f96" : "#b28a43";
      return `color-mix(in srgb, ${ink} ${Math.min(85, 30 + strength * 80)}%, var(--paper))`;
    }
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
      case "temper":
        return leaning(largest.ethos[tint.axis]);
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
  // Rule is always drawn; other dealings only for at most two chosen
  // peoples, since a wider selection tangles the chart. Peoples on the
  // same land need no line between them.
  const dealings = contacts
    ? overview.contacts.filter((k) => {
        const [a, b] = [overview.communities[k.a], overview.communities[k.b]];
        if (a.ended !== null || b.ended !== null || a.region === b.region) return false;
        if (hidden(a.region) || hidden(b.region)) return false;
        return k.kind === "rule" || (chosen.size <= 2 && (chosen.has(k.a) || chosen.has(k.b)));
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
  const label = Math.max(Math.sqrt(box[2] / map.width),
    zoomable && labelWidth <= 700 ? 12 * box[2] / (labelWidth * 0.24) : 0);
  // Land a veiling language does not know.
  function hidden(region: number): boolean {
    return known !== null && !known.has(region);
  }

  // Capitals sit below their co-resident peoples. All lettering competes
  // in drawCamera, with people's names first, largest to smallest.
  // Where each capital's mark sits: below the names of the peoples there.
  const capitalAt = new Map<number, [number, number]>();
  for (const { state } of realms) {
    const [x, y] = map.regions[state.capital].site;
    const below = y + ((hearts.get(state.capital)?.length ?? 0) / 2 * LINE + 0.2) * label;
    capitalAt.set(state.id, [x, below]);
  }

  const possible = new Set(settlement?.options.filter((o) => o.reason === null).map((o) => o.region));
  const remains = new Set(settlement?.plan?.remaining.lands);
  const arrives = new Set(settlement?.plan?.arriving.lands);
  const shape = (r: WorldMap["regions"][number]) => {
    const sea = r.terrain === "sea";
    const veiled = !sea && hidden(r.id);
    const colour = veiled ? null : colourOf(r.id);
    const points = r.outline.map(([x, y]) => `${x},${y}`).join(" ");
    return (
      <g key={r.id} data-chart-region={r.id} onClick={sea ? undefined : () => dragged() || onLand(r.id)}>
        <title>{sea ? "Sea" : veiled ? "Unknown land" : (nameOf(r.id) ?? `Unnamed ${TERRAIN_NAME[r.terrain].toLowerCase()}`)}</title>
        <polygon
          data-region={r.id}
          className={`land terrain-${r.terrain}${sea ? "" : " open"}${lands.has(r.id) ? " shown" : ""}${reader?.region === r.id ? " lens-heart" : ""}`}
          points={points}
        />
        {colour ? <polygon className={tint.kind === "weather" ? "weather-wash" : "claim"} data-region={r.id} points={points} style={{ fill: colour }} /> : null}
        {!sea && settlement ? <polygon points={points} className={`settlement-land${possible.has(r.id) ? " possible" : ""}${remains.has(r.id) ? " remaining" : ""}${arrives.has(r.id) ? " arriving" : ""}`} /> : null}
      </g>
    );
  };
  // Snapshot only at a reading change. Camera and selection renders never
  // animate ownership, and no inferred journey is drawn.
  useLayoutEffect(() => {
    const node = svg.current;
    if (!node) return;
    const memory = motionMemory ?? localMotion;
    const before = memory.current;
    if (before?.overview === overview) return;
    const owners = new Map<number, number[]>();
    for (const people of overview.communities) {
      if (people.ended !== null) continue;
      for (const land of people.lands) {
        const held = owners.get(land) ?? [];
        held.push(people.id); owners.set(land, held);
      }
    }
    const fills: MapMotionReading["fills"] = new Map();
    node.querySelectorAll<SVGPolygonElement>(".claim[data-region]").forEach((claim) => {
      const region = Number(claim.dataset.region);
      fills.set(region, { owners: (owners.get(region) ?? []).join(","), fill: claim.style.fill });
    });
    memory.current = { overview, fills };
    const step = before && overview.generation === before.overview.generation + 1 &&
      (overview.telling === before.overview.telling || overview.mutation !== before.overview.mutation);
    const intervention = before && overview.generation === before.overview.generation &&
      overview.mutation !== before.overview.mutation && overview.point.action !== before.overview.point.action;
    if (!before || !animateChanges || reducedMotion() || (!step && !intervention)) return;
    const changed: Element[] = [];
    const ghosts: Element[] = [];
    const layer = node.querySelector(".lands");
    for (const region of new Set([...before.fills.keys(), ...fills.keys()])) {
      const old = before.fills.get(region);
      const next = fills.get(region);
      if (old?.owners === next?.owners) continue;
      const claim = node.querySelector<SVGPolygonElement>(`.claim[data-region="${region}"]`);
      if (next && claim) { claim.classList.add("map-holding-gained"); changed.push(claim); }
      if (old && layer) {
        const ground = node.querySelector<SVGPolygonElement>(`.land[data-region="${region}"]`);
        if (!ground) continue;
        const ghost = document.createElementNS("http://www.w3.org/2000/svg", "polygon");
        ghost.setAttribute("points", ground.getAttribute("points")!);
        ghost.setAttribute("class", "claim map-holding-lost");
        ghost.style.fill = old.fill;
        layer.append(ghost); ghosts.push(ghost);
      }
    }
    const priorStates = new Set(before.overview.states.filter((state) => state.fell === null).map((state) => state.id));
    node.querySelectorAll<SVGPathElement>(".state-borders [data-state]").forEach((border) => {
      if (priorStates.has(Number(border.dataset.state))) return;
      border.style.setProperty("--border-length", String(border.getTotalLength()));
      border.classList.add("map-border-new"); changed.push(border);
    });
    const clear = () => {
      changed.forEach((element) => element.classList.remove("map-holding-gained", "map-border-new"));
      ghosts.forEach((element) => element.remove());
    };
    const timer = window.setTimeout(clear, 400);
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    const instant = () => { if (media.matches) clear(); };
    media.addEventListener("change", instant);
    return () => { window.clearTimeout(timer); clear(); media.removeEventListener("change", instant); };
  }, [overview, animateChanges, motionMemory]);

  return (
    <div ref={root} className={`mapview${zoomable ? " zoomable" : ""} ${box[2] / map.width > 0.6 ? "zoom-far" : box[2] / map.width < 0.25 ? "zoom-close" : "zoom-mid"}`}>
      <svg
        ref={svg}
        viewBox={box.join(" ")}
        role="group"
        tabIndex={zoomable ? 0 : undefined}
        onKeyDown={(e) => {
          if (!zoomable || e.target !== e.currentTarget) return;
          const [x, y, w, h] = box;
          if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
            e.preventDefault(); cancelAnimationFrame(glide.current);
            setBox(clamp([x + (e.key === "ArrowLeft" ? -w / 8 : e.key === "ArrowRight" ? w / 8 : 0),
              y + (e.key === "ArrowUp" ? -h / 8 : e.key === "ArrowDown" ? h / 8 : 0), w, h]));
          } else if (["+", "=", "-", "Home", "f"].includes(e.key)) {
            e.preventDefault();
            if (e.key === "Home") glideTo(full);
            else if (e.key === "f") fitSelection();
            else zoom(e.key === "-" ? 1 / 0.7 : 0.7, [x + w / 2, y + h / 2]);
          }
        }}
        aria-label={`Map of ${map.regions.length} lands in year ${generation * YEARS}`}
        style={{ "--label": label } as CSSProperties}
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        onPointerCancel={up}
        onPointerLeave={(e) => { if (!svg.current?.hasPointerCapture(e.pointerId)) up(e); }}
      >
        <defs>
          <marker
            id={routeHead}
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
          <rect className="terrain-sea" width={map.width} height={map.height} />
          {dress.under}
          {grounds.map(shape)}
        </g>
        {dress.over}
        <g className="chart-rivers">
          {riverPaths.map(({ river, d, tier }) => {
            if (river.course.every(hidden)) return null;
            const name = namedRiver(river);
            const failed = climate?.rivers.find((flow) => flow.id === river.id)?.flowing === false;
            return <g key={river.id} className={`chart-river river-tier-${tier}${failed ? " failed" : ""}`}
              role={onRiver ? "button" : undefined} tabIndex={onRiver ? 0 : undefined}
              aria-label={name?.spelled ?? "Unnamed river"}
              onClick={() => dragged() || onRiver?.(river.id)}
              onKeyDown={(e) => { if (onRiver && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); onRiver(river.id); } }}>
              <title>{`${name?.spelled ?? "Unnamed river"}${failed ? ", flow has failed" : ""}`}</title>
              <path className="river-ink" d={d} />
              <path className="river-hit" d={d} />
            </g>;
          })}
        </g>
        {tint.kind === "weather" ? <g className="zone-borders" aria-hidden="true">
          {borders.filter(({ a, b }) => map.regions[a].climateZone !== map.regions[b].climateZone).map(({ a, b, ends }) =>
            <path key={`${a}-${b}`} d={`M${ends[0].join(",")}L${ends[1].join(",")}`} />)}
        </g> : null}
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
                data-label-kind="continent" data-label-priority={0}
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
            <path key={state.id} data-state={state.id} d={border} style={{ stroke: hue(state.id) }} />
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
                <text key={p.region} x={x} y={top} className={here > 0 ? "place-name" : "place-name left"}
                  data-label-kind="land" data-label-priority={2}>
                  {nameOf(p.region)}
                </text>
              );
            })}
          </g>
        ) : null}
        {names ? <g className="river-names" aria-hidden="true">
          {riverPaths.map(({ river, label: line }) => {
            const name = namedRiver(river);
            if (!name || river.course.every(hidden)) return null;
            const id = `${routeHead}-river-${river.id}`;
            return <g key={river.id}>
              <defs><path id={id} d={line} /></defs>
              <text className={`hand-${overview.varieties[name.variety].family % 5}`} dy={-0.06}
                data-label-kind="river" data-label-priority={1}>
                <textPath href={`#${id}`} startOffset="50%" textAnchor="middle">{name.spelled}</textPath>
              </text>
            </g>;
          })}
        </g> : null}
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
                  d={m.path.map((r, step) => `${step ? "L" : "M"}${map.regions[r].site.join(",")}`).join(" ")}
                  markerEnd={`url(#${routeHead})`}
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
        {settlement?.plan ? <g className="settlement-journeys" aria-hidden="true">
          {settlement.plan.routes.map((r) => <polyline key={r.from} className={r.by_sea ? "sea-journey" : ""}
            points={r.path.map((region) => map.regions[region].site.join(",")).join(" ")}
            markerEnd={`url(#${routeHead})`} />)}
        </g> : null}
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
        <g className="peoples">
          {overview.communities.map((c) => {
            if (c.ended !== null || hidden(c.region)) return null;
            const [x, y] = at.get(c.id)!;
            const word = wordBy.get(c.id);
            const localName = selectedVariety === undefined ? c.name : c.exonyms.find((name) =>
              name.by === reader?.id || overview.communities[name.by]?.variety === selectedVariety)?.name ?? c.name;
            const text = tint.kind === "words" ? (word?.spelled ?? "—") : localName;
            return (
              <g
                key={c.id}
                className={chosen.has(c.id) ? "people chosen" : "people"}
                role="button"
                tabIndex={0}
                aria-label={tint.kind === "words" ? `${localName}: ${text}` : localName}
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
                    ? `${localName}: ${word.spelled} /${word.ipa}/`
                    : `${localName}, ${Math.round(c.size).toLocaleString()} souls`}
                </title>
                <circle className="people-dot" cx={x} cy={y} r={0.06 * label}
                  style={{ fill: tint.kind === "words" && word ? hue(word.group) : hue(family(c)) }} />
                <text x={x} y={y} className={`hand-${family(c) % 5}`}
                  data-label-kind="people" data-people={c.id} data-label-priority={100 + c.size}
                  style={{ fill: tint.kind === "words" && word ? hue(word.group) : hue(family(c)) }}>
                  {text}
                </text>
                {overview.varieties[c.variety].name !== text && tint.kind === "peoples" ?
                  <text x={x} y={y + 0.24 * label} className="tongue-name"
                    data-label-kind="tongue" data-people={c.id} data-label-priority={4}>
                    {overview.varieties[c.variety].name}
                  </text> : null}
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
                <text y={0.27} className={`hand-${family(overview.communities[state.rulers]) % 5}`}
                  data-label-kind="state" data-label-priority={3}>{state.name}</text>
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
        <div className="map-zoom" role="group" aria-label="Map controls">
          {selectedRegions.length > 0 ? <button type="button" title="Fit the selected lands (F)" aria-label="Fit the selected lands" onClick={fitSelection}><LocateFixed size={16} /></button> : null}
          <button type="button" title="Closer (+)" aria-label="Zoom in" disabled={box[2] <= map.width * CLOSEST + 0.001} onClick={() => zoom(0.7, [box[0] + box[2] / 2, box[1] + box[3] / 2])}>
            <Plus size={16} />
          </button>
          <button type="button" title="Farther (−)" aria-label="Zoom out" disabled={box[2] >= map.width - 0.001} onClick={() => zoom(1 / 0.7, [box[0] + box[2] / 2, box[1] + box[3] / 2])}>
            <Minus size={16} />
          </button>
          <button type="button" title="The whole map (Home)" aria-label="The whole map" onClick={() => glideTo(full)}>
            <Maximize size={16} />
          </button>
        </div>
      ) : null}
      {zoomable ? <div className="chart-scale">
        <span ref={scaleRule} className="scale-rule"><span ref={scaleText} /></span>
        <span className="chart-help">Scroll to zoom · drag to explore</span>
      </div> : null}
      {ROSE}
    </div>
  );
}

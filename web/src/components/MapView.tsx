import { useEffect, useId, useLayoutEffect, useMemo, useRef, type CSSProperties, type PointerEvent, type RefObject, type SetStateAction } from "react";
import { LocateFixed, Maximize, Minus, Plus } from "lucide-react";
import type { ClimateView, Community, Craft, EthosAxis, FoundingPreview, Lake, LakeNamesView, Overview, River, RiverNamesView, SettlementPreview, WordMap, WorldMap } from "../model";
import type { ShelfPeople } from "../shelf";
import { reducedMotion } from "../motion";
import { YEARS } from "../model";
import { hue, TERRAIN_NAME } from "../lore";
import { chartPoint, createCartography, geographicPoint, lakeOutline, riverPoints, sharedBorder, type Cartography, type GlobeRotation, type MapPoint, type MapProjection } from "../cartography";
import { MapProjectionSwitch } from "./MapProjectionSwitch";

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

/// Use a real attested form in the selected speech, then the mouth's speech.
export function riverName(view: Pick<RiverNamesView, "names" | "exonyms">, variety: number | undefined) {
  if (variety === undefined) return null;
  return [...view.names].reverse().find((name) => name.variety === variety)
    ?? view.exonyms.find((name) => name.variety === variety) ?? null;
}

function riverDress(map: WorldMap) {
  const largest = Math.max(1, ...map.rivers.map((river) => river.catchment.length));
  return map.rivers.map((river) => ({
    river,
    points: riverPoints(map, river),
    tier: river.catchment.length >= largest * 2 / 3 ? 3 : river.catchment.length >= largest / 3 ? 2 : 1,
  }));
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

interface ChartDress {
  coast: MapPoint[][];
  marked: { id: number; spots: { kind: string; site: MapPoint; size: number }[] }[];
}

type PathDrawer = (cartography: Cartography) => string;
type PathBinder = (draw: PathDrawer, hide?: "parent" | "grandparent") => { ref: (node: SVGPathElement | null) => void; d: string };

/// Cache immutable geographic anchors and shared coast edges once per map.
/// The small ink glyphs remain lettering; their anchors follow the projection.
function chartDress(map: WorldMap): ChartDress {
  const land = map.regions.filter((r) => r.terrain !== "sea");
  const coast = land.flatMap((r) => r.neighbours
    .filter((id) => map.regions[id].terrain === "sea")
    .map((id) => sharedBorder(map, r.id, id)));
  const marked: ChartDress["marked"] = [];
  for (const r of land) {
    const next = scatter(r.id);
    const [cx, cy] = r.site;
    const size = Math.sqrt(r.areaKm2) / map.kmPerUnit * 1.4;
    const outline = r.boundary.map((point) => {
      const [x, y] = chartPoint(map, point);
      return [x + Math.round((cx - x) / map.width) * map.width, y] as MapPoint;
    });
    const spots: ChartDress["marked"][number]["spots"] = [];
    const count = r.terrain === "desert" ? 14 : r.terrain === "forest" || r.terrain === "steppe" ? 6 : 3;
    for (let placed = 0, tries = 0; placed < count && tries < 60; tries++) {
      const site: MapPoint = [cx + (next() - 0.5) * 0.9 * size, cy + (next() - 0.5) * 0.8 * size];
      if (!inside(outline, site) || !inside(outline, [site[0] + 0.13 * size, site[1]]) ||
        !inside(outline, [site[0] - 0.13 * size, site[1] - 0.17 * size])) continue;
      spots.push({ kind: r.terrain, site, size: r.terrain === "mountains" ? size * (0.18 + next() * 0.07) : size });
      placed++;
    }
    marked.push({ id: r.id, spots });
  }
  return { coast, marked };
}

function dressUnder(dress: ChartDress, cartography: Cartography, bind?: PathBinder) {
  const drawCoast = (view: Cartography) => dress.coast.map((ends) => view.line(ends)).join("");
  const coast = bind ? "" : drawCoast(cartography);
  const coastPath = () => bind?.(drawCoast) ?? { d: coast };
  return <g className="chart-dress" aria-hidden="true">
    <path className="chart-graticule" {...(bind?.((view) => view.graticule) ?? { d: cartography.graticule })} />
    <g className="chart-ripples">
      {RIPPLES.map(([width, dark]) => <g key={width}>
        <path className="ripple" {...coastPath()} style={{ strokeWidth: width, strokeOpacity: dark }} />
        <path className="ripple-gap" {...coastPath()} style={{ strokeWidth: width - 0.035 }} />
      </g>)}
    </g>
    <path className="chart-coast" {...coastPath()} />
  </g>;
}

function dressOver(dress: ChartDress, cartography: Cartography, bind?: (site: MapPoint, scale: number) => (node: SVGGElement | null) => void) {
  return <g className="chart-dress chart-marks" aria-hidden="true">
    {dress.marked.map(({ id, spots }) => <g key={id} data-chart-region={id}>
      {spots.map(({ kind, site, size }, i) => {
        const at = cartography.point(site);
        if (!at && !bind) return null;
        return <g key={i} ref={bind?.(site, size)} style={{ display: at ? undefined : "none" }}
          transform={`translate(${(at ?? [0, 0]).join(" ")}) scale(${size})`}>
          {kind === "mountains" ? <g>
            <path className="mark-peak" d="M-1,0L0,-1.5L1,0" />
            <path className="mark-shade" d="M.15,-1.2L.65,0" />
          </g> : kind === "hills" ? <path className="mark-hill" d="M-.11,0Q0,-.16 .11,0" />
            : kind === "forest" ? <path className="mark-tree" d="M-.045,-.07a.045,.045 0 1,0 .09,0a.045,.045 0 1,0 -.09,0M0,-.025V.03" />
              : kind === "desert" ? <path className="mark-sand" d="M-.008,0a.008,.008 0 1,0 .016,0a.008,.008 0 1,0 -.016,0" />
                : kind === "steppe" ? <path className="mark-grass" d="M-.04,0l.02,-.05M0,0v-.06M.04,0l-.02,-.05" />
                  : <path className="mark-field" d="M-.06,0h.12" />}
        </g>;
      })}
    </g>)}
  </g>;
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
  const cartography = useMemo(() => createCartography(map), [map]);
  const paths = useMemo(() => map.regions.map((r) => cartography.region(r)), [map, cartography]);
  // Largest first, so where two peoples share a land the largest colours it.
  const holder = useMemo(() => {
    const out = new Map<number, number>();
    for (const p of peoples) for (const land of p.lands) if (!out.has(land)) out.set(land, p.family);
    return out;
  }, [peoples]);
  const named = peoples.slice(0, NAMED);
  // Shelf lettering occupies the same share of every world's sheet.
  const label = map.width / 9;
  const shape = (r: WorldMap["regions"][number]) => {
    const family = holder.get(r.id);
    return (
      <g key={r.id}>
        <path className={`land terrain-${r.terrain}`} d={paths[r.id]} />
        {family === undefined ? null : <path className="claim" d={paths[r.id]} style={{ fill: hue(family) }} />}
      </g>
    );
  };
  return (
    <div className="mapview miniature projection-chart">
      <svg viewBox={`0 0 ${map.width} ${map.height}`} style={{ "--label": label } as CSSProperties} aria-hidden="true">
        <g className="lands">
          {map.regions.filter((r) => r.terrain === "sea").map(shape)}
          {dressUnder(dress, cartography)}
          {map.regions.filter((r) => r.terrain !== "sea").map(shape)}
        </g>
        {dressOver(dress, cartography)}
        <g className="peoples">
          {named.map((p, i) => {
            const [x, y] = cartography.point(map.regions[p.region].site)!;
            const above = named.slice(0, i).filter((q) => q.region === p.region).length;
            return (
              <g key={i} className="people">
                <text x={x} y={y + above * LINE * label} className={`hand-${p.family % 5}`} style={{ fill: hue(p.family) }}>
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
  lakeNames = [],
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
  projection = "chart",
  onProjection,
  rotation,
  onRotation,
  reach,
  onPeople,
  onLand,
  onContinent,
  onState,
  onReligion,
  onCraft,
  onRiver,
  onLake,
}: {
  map: WorldMap;
  overview: Overview;
  generation: number;
  tint: Tint;
  climate?: ClimateView;
  riverNames?: RiverNamesView[];
  lakeNames?: LakeNamesView[];
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
  projection?: MapProjection;
  onProjection?: (projection: MapProjection) => void;
  rotation?: GlobeRotation;
  onRotation?: (rotation: GlobeRotation) => void;
  reach?: { preview: FoundingPreview; people: number };
  onPeople: (community: number) => void;
  onLand: (region: number) => void;
  onContinent?: (landmass: number) => void;
  onState?: (state: number) => void;
  onReligion?: (religion: number) => void;
  onCraft?: (craft: Craft) => void;
  onLake?: (lake: number) => void;
  onRiver?: (id: number) => void;
}) {
  const climateZones = useMemo(() => new Map(climate?.zones.map((zone) => [zone.id, zone])), [climate]);
  const climateRivers = useMemo(() => new Map(climate?.rivers.map((river) => [river.id, river])), [climate]);
  const reader = lens ? overview.communities.find((c) => c.id === lens.people) : undefined;
  const readerVariety = reader ? overview.varieties[reader.variety] : undefined;
  selectedVariety = readerVariety?.id ?? selectedVariety;
  const readerLands = useMemo(() => readerVariety
    ? new Set(readerVariety.knownLands.map((land) => land.region)) : null, [readerVariety]);
  known = readerLands ?? known;
  const turning = useRef<GlobeRotation>(rotation ?? [0, 20]);
  const cartography = useMemo(() => createCartography(map, projection, turning.current), [map, projection]);
  const { width, height } = cartography;
  // At closest, show roughly three mean region widths in either projection.
  const closest = Math.min(1, 3 * Math.sqrt((projection === "globe" ? Math.PI : 1 / Math.PI) / Math.max(1, map.regions.length)));
  const full: Box = useMemo(() => [0, 0, width, height], [width, height]);
  const chartView = useRef<Box>(camera ?? [0, 0, map.width, map.height]);
  const globeView = useRef<Box>([0, 0, map.height, map.height]);
  const view = projection === "chart" ? chartView : globeView;
  const drawnMap = useRef(map);
  if (drawnMap.current !== map) {
    drawnMap.current = map;
    chartView.current = camera ?? [0, 0, map.width, map.height];
    globeView.current = [0, 0, map.height, map.height];
  }
  let box = view.current;
  const root = useRef<HTMLDivElement>(null);
  const scaleText = useRef<HTMLSpanElement>(null);
  const scaleRule = useRef<HTMLSpanElement>(null);
  const labelLayers = useRef<SVGElement[]>([]);
  const labelAnchors = useRef<SVGElement[]>([]);
  const lettering = useRef<SVGTextElement[]>([]);
  const paths = useRef(new Map<SVGPathElement, { draw: PathDrawer; d: string; hide: HTMLElement | SVGElement | null; revision: number }>());
  const drawnPaths = useMemo(() => new WeakMap<PathDrawer, { revision: number; d: string }>(), [cartography]);
  const drawRevision = useRef(0);
  const projectedAnchors = useRef(new Map<SVGElement, { site: MapPoint; scale?: number; visible: boolean; revision: number }>());
  // The same semantic zoom policy as workbench.css, classified once per
  // render rather than asking the DOM for visibility on every globe turn.
  const projectionLayers = useRef<{ mask: number; paths: typeof paths.current; anchors: typeof projectedAnchors.current; labels: SVGElement[] }[]>([]);
  const layerMask = (node: SVGElement): number => {
    if (node.closest(".place-names, .river-names, .lake-names") ||
      node.closest(".chart-marks") && node.querySelector(".mark-tree, .mark-grass, .mark-field, .mark-sand")) return 4;
    if (node.closest(".chart-river:not(.river-tier-3), .routes, .dealings, .pilgrim-roads, .founding-markers")) return 6;
    if (node.closest(".continent-names")) return 3;
    return 7;
  };
  const pendingTurn = useRef(false);
  const moving = useRef(false);
  const settleTimer = useRef(0);
  const rotationChanged = useRef(false);
  const callbacks = useRef({ onRotation, onCamera });
  callbacks.current = { onRotation, onCamera };
  const bindPath: PathBinder = (draw, hide) => {
    let drawn = drawnPaths.get(draw);
    if (!drawn || drawn.revision !== drawRevision.current) {
      drawn = { revision: drawRevision.current, d: draw(cartography) };
      drawnPaths.set(draw, drawn);
    }
    const d = drawn.d;
    let attached: SVGPathElement | null = null;
    return { d, ref: (node: SVGPathElement | null) => {
      if (attached) paths.current.delete(attached);
      attached = node;
      if (node) {
        const hidden = hide === "parent" ? node.parentElement : hide === "grandparent" ? node.parentElement?.parentElement ?? null : null;
        if (hidden) hidden.style.display = d ? "" : "none";
        paths.current.set(node, { draw, d, hide: hidden, revision: drawRevision.current });
      }
    } };
  };
  const bindAnchor = (site: MapPoint, scale?: number) => {
    let attached: SVGElement | null = null;
    return (node: SVGElement | null) => {
      if (attached) projectedAnchors.current.delete(attached);
      attached = node;
      if (node) projectedAnchors.current.set(node, { site, scale, visible: node.style.display !== "none", revision: -1 });
    };
  };
  // Hidden hemisphere anchors still have DOM and handlers, so they can appear
  // on the next turn without any React reconciliation.
  const pointFor = (site: MapPoint): MapPoint => cartography.point(site) ?? [0, 0];
  const frame = useRef(0);
  const bounds = useMemo(() => map.regions.map((region) => {
    const sites = region.boundary.map((point) => chartPoint(map, point));
    const xs = sites.map((p) => p[0]), ys = sites.map((p) => p[1]);
    if (Math.max(...xs) - Math.min(...xs) > map.width / 2 || Math.abs(region.center[1]) > 80)
      return [0, 0, map.width, map.height];
    return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
  }), [map]);
  const culled = useRef<{ node: SVGElement; bounds: number[]; hidden: boolean }[]>([]);
  const drawCamera = () => {
    const element = svg.current, chart = root.current;
    if (!element || !chart) return;
    const value = view.current, ratio = value[2] / width;
    const nextBox = value.join(" ");
    if (element.getAttribute("viewBox") !== nextBox) element.setAttribute("viewBox", nextBox);
    // Read the scale before moving paths; measuring afterward forces layout
    // of the whole changed globe on every turn.
    const pixelsPerUnit = element.getScreenCTM()?.a ?? 1;
    const visibleMask = ratio > 0.6 ? 1 : ratio < 0.25 ? 4 : 2;
    if (pendingTurn.current) {
      cartography.rotate(turning.current);
      ++drawRevision.current;
      chart.dataset.longitude = String(turning.current[0]);
      chart.dataset.latitude = String(turning.current[1]);
      pendingTurn.current = false;
    }
    const revision = drawRevision.current;
    for (const layer of projectionLayers.current) {
      if (!(layer.mask & visibleMask)) continue;
      // Hidden layers retain their old revision and catch up exactly once
      // when zoom reveals them, even if the rotation has since stopped.
      layer.paths.forEach((binding, node) => {
        if (binding.revision === revision) return;
        let drawn = drawnPaths.get(binding.draw);
        if (!drawn) {
          drawn = { revision, d: binding.draw(cartography) };
          drawnPaths.set(binding.draw, drawn);
        } else if (drawn.revision !== revision) {
          drawn.d = binding.draw(cartography);
          drawn.revision = revision;
        }
        if (drawn.d !== binding.d) {
          node.setAttribute("d", drawn.d);
          if (binding.hide) binding.hide.style.display = drawn.d ? "" : "none";
          binding.d = drawn.d;
        }
        binding.revision = revision;
      });
      layer.anchors.forEach((binding, node) => {
        if (binding.revision === revision) return;
        binding.revision = revision;
        const point = cartography.point(binding.site);
        const visible = point !== null;
        if (visible !== binding.visible) { node.style.display = visible ? "" : "none"; binding.visible = visible; }
        if (!point) return;
        if (binding.scale !== undefined) {
          node.setAttribute("transform", `translate(${point[0]} ${point[1]}) scale(${binding.scale})`);
        } else {
          node.dataset.labelX = String(point[0]);
          node.dataset.labelY = String(point[1]);
        }
      });
    }
    const label = Math.max(Math.sqrt(ratio), 14 / (pixelsPerUnit * 0.24));
    element.style.setProperty("--label", String(label));
    for (const layer of labelLayers.current) {
      layer.style.setProperty("--label", String(label));
      if (layer.classList.contains("peoples")) layer.style.setProperty("--people-label-min", `${14 / pixelsPerUnit}px`);
    }
    // Camera frames need only relayout lettering, not rerender mesh geometry.
    for (const layer of projectionLayers.current) {
      if (!(layer.mask & visibleMask)) continue;
      for (const anchor of layer.labels) {
        const x = Number(anchor.dataset.labelX) + Number(anchor.dataset.labelDx ?? 0) * label;
        const y = Number(anchor.dataset.labelY) + Number(anchor.dataset.labelDy ?? 0) * label;
        if (anchor.tagName === "g" || anchor.dataset.labelScale !== undefined) {
          anchor.setAttribute("transform", `translate(${x} ${y})${anchor.dataset.labelScale === undefined ? "" : ` scale(${label})`}`);
        } else {
          anchor.setAttribute("x", String(x));
          anchor.setAttribute("y", String(y));
        }
      }
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
        const hidden = projection === "chart" && ratio <= 0.6 &&
          (right < x - 0.2 || left > x + w + 0.2 || bottom < y - 0.2 || top > y + h + 0.2);
        if (hidden !== item.hidden) { item.node.style.display = hidden ? "none" : ""; item.hidden = hidden; }
      }
    }
    const kmPerPixel = (projection === "globe" ? 2 * map.radiusKm / height : map.kmPerUnit) / pixelsPerUnit;
    chart.style.setProperty("--small-label-min", `${10 / pixelsPerUnit}px`);
    chart.style.setProperty("--state-weight", String(1 + 2 * Math.min(1, (1 - ratio) / 0.6)));
    if (!moving.current) {
    const occupied: DOMRect[] = [];
    const named = new Set<string>();
    // Measure the actual font, not character-count estimates. Visibility
    // preserves geometry, so a suppressed label can return on the next frame.
    for (const text of lettering.current) {
      text.style.visibility = "";
      if (text.dataset.labelKind === "river") text.setAttribute("dy", String(-0.06 * label));
    }
    const measured = lettering.current.map((text) => ({ text, rect: text.getBoundingClientRect() }));
    // Family hands have different ascenders. Place the land's name above
    // their measured ink rather than assuming one font's line height.
    const peopleTops = new Map<string, number>();
    for (const { text, rect } of measured) {
      if (text.dataset.labelKind !== "people") continue;
      const region = text.parentElement?.dataset.region;
      if (region !== undefined) peopleTops.set(region, Math.min(peopleTops.get(region) ?? Infinity, rect.top));
    }
    for (const item of measured) {
      if (item.text.dataset.labelKind !== "land") continue;
      const top = peopleTops.get(item.text.dataset.region ?? "");
      if (top === undefined) continue;
      const y = Number(item.text.getAttribute("y")) + (top - LABEL_GAP * 2 - item.rect.bottom) / pixelsPerUnit;
      item.text.setAttribute("y", String(y));
      item.rect = item.text.getBoundingClientRect();
    }
    const chartBounds = chart.getBoundingClientRect();
    for (const item of measured) {
      const { text } = item;
      let { rect } = item;
      const kind = text.dataset.labelKind;
      const people = text.dataset.people;
      const allowed = kind === "tongue"
        ? tint.kind === "peoples" && kmPerPixel < TONGUE_KM_PER_PIXEL && named.has(people!)
        : kind === "land" || kind === "river" || kind === "lake" ? kmPerPixel < LAND_KM_PER_PIXEL : true;
      const inView = rect.right > chartBounds.left && rect.left < chartBounds.right &&
        rect.bottom > chartBounds.top && rect.top < chartBounds.bottom;
      // Water names sit below their centroid. If the land's lettering reaches
      // there, move below its measured ink instead of hiding the lake's name.
      if (kind === "lake" && allowed && rect.width > 0) {
        for (let attempt = 0; attempt < occupied.length; attempt++) {
          const overlap = occupied.filter((other) => other.left < rect.right + LABEL_GAP && rect.left < other.right + LABEL_GAP &&
            other.top < rect.bottom + LABEL_GAP && rect.top < other.bottom + LABEL_GAP);
          if (!overlap.length) break;
          const y = Number(text.getAttribute("y")) + (Math.max(...overlap.map((other) => other.bottom)) + LABEL_GAP * 2 - rect.top) / pixelsPerUnit;
          text.setAttribute("y", String(y));
          rect = text.getBoundingClientRect();
        }
      }
      const collides = occupied.some((other) => other.left < rect.right + LABEL_GAP && rect.left < other.right + LABEL_GAP &&
        other.top < rect.bottom + LABEL_GAP && rect.top < other.bottom + LABEL_GAP);
      const visible = allowed && inView && rect.width > 0 && !collides;
      text.style.visibility = visible ? "" : "hidden";
      if (visible) { occupied.push(rect); if (kind === "people") named.add(people!); }
      if (kind === "people") {
        const dot = text.parentElement?.querySelector<SVGCircleElement>(".people-dot");
        if (dot) {
          dot.style.visibility = visible ? "hidden" : "";
          dot.setAttribute("r", String(Math.max(0.06 * Math.sqrt(ratio), 2.5 / pixelsPerUnit)));
        }
      }
    }
    }
    const km = [50, 100, 200, 500, 1000].filter((length) => length <= value[2] * map.kmPerUnit * 0.15).at(-1) ?? 50;
    if (projection === "chart") scaleRule.current?.style.setProperty("--scale-width", `${km / map.kmPerUnit * pixelsPerUnit}px`);
    else scaleRule.current?.style.removeProperty("--scale-width");
    const caption = projection === "globe"
      ? `Circumference ${Math.round(2 * Math.PI * map.radiusKm).toLocaleString()} km`
      : `${km.toLocaleString()} km at the equator`;
    if (scaleText.current && scaleText.current.textContent !== caption) scaleText.current.textContent = caption;
    const closer = chart.querySelector<HTMLButtonElement>('[aria-label="Zoom in"]');
    const farther = chart.querySelector<HTMLButtonElement>('[aria-label="Zoom out"]');
    if (closer) closer.disabled = ratio <= closest + 0.0001;
    if (farther) farther.disabled = ratio >= 0.9999;
  };
  const drawCurrent = useRef(drawCamera);
  drawCurrent.current = drawCamera;
  const scheduleDraw = () => {
    if (frame.current) return;
    frame.current = requestAnimationFrame(() => { frame.current = 0; drawCurrent.current(); });
  };
  const settle = () => {
    window.clearTimeout(settleTimer.current);
    moving.current = false;
    scheduleDraw();
    if (rotationChanged.current) {
      rotationChanged.current = false;
      callbacks.current.onRotation?.([...turning.current]);
    }
  };
  const moveCamera = () => {
    moving.current = true;
    window.clearTimeout(settleTimer.current);
    settleTimer.current = window.setTimeout(() => {
      if (!drag.current?.moved && !pinch.current && !glide.current) settle();
    }, 120);
    scheduleDraw();
  };
  const setBox = (next: SetStateAction<Box>) => {
    box = typeof next === "function" ? next(view.current) : next;
    view.current = box;
    moveCamera();
    if (projection === "chart") callbacks.current.onCamera?.(box);
  };
  useLayoutEffect(() => {
    if (projection === "chart" && camera) view.current = camera;
    culled.current = [...(root.current?.querySelectorAll<SVGElement>("[data-chart-region]") ?? [])].map((node) => ({
      node, bounds: bounds[Number(node.dataset.chartRegion)], hidden: node.style.display === "none",
    }));
    labelLayers.current = [...(root.current?.querySelectorAll<SVGElement>(".peoples, .place-names, .river-names, .lake-names, .continent-names, .state-capitals, .founding-markers") ?? [])];
    labelAnchors.current = [...(root.current?.querySelectorAll<SVGElement>("[data-label-x]") ?? [])];
    lettering.current = [...(root.current?.querySelectorAll<SVGTextElement>("[data-label-kind]") ?? [])]
      .sort((a, b) => Number(b.dataset.labelPriority) - Number(a.dataset.labelPriority));
    const layers = new Map<number, typeof projectionLayers.current[number]>();
    const layerFor = (node: SVGElement) => {
      const mask = layerMask(node);
      let layer = layers.get(mask);
      if (!layer) { layer = { mask, paths: new Map(), anchors: new Map(), labels: [] }; layers.set(mask, layer); }
      return layer;
    };
    paths.current.forEach((binding, node) => layerFor(node).paths.set(node, binding));
    projectedAnchors.current.forEach((binding, node) => layerFor(node).anchors.set(node, binding));
    for (const node of labelAnchors.current) layerFor(node).labels.push(node);
    projectionLayers.current = [...layers.values()];
    drawCamera();
  });
  useEffect(() => {
    const refresh = () => drawCurrent.current();
    const observer = new ResizeObserver(refresh);
    if (root.current) observer.observe(root.current);
    document.fonts.addEventListener("loadingdone", refresh);
    return () => {
      observer.disconnect(); document.fonts.removeEventListener("loadingdone", refresh);
      cancelAnimationFrame(frame.current); frame.current = 0;
      window.clearTimeout(settleTimer.current);
    };
  }, [map, projection]);
  useLayoutEffect(() => {
    if (!rotation || (rotation[0] === turning.current[0] && rotation[1] === turning.current[1])) return;
    turning.current = [...rotation];
    pendingTurn.current = true;
    scheduleDraw();
  }, [rotation?.[0], rotation?.[1], cartography]);
  const routeHead = useId();
  const surfaceClip = useId();
  const svg = useRef<SVGSVGElement>(null);
  const localMotion = useRef<MapMotionReading | null>(null);
  const drag = useRef<{ x: number; y: number; scale: number; box: Box; rotation: GlobeRotation; moved: boolean } | null>(null);
  const glide = useRef(0);
  const pointers = useRef(new Map<number, [number, number]>());
  const pinch = useRef<{ distance: number; middle: [number, number]; anchor: [number, number]; scale: number; box: Box } | null>(null);
  const suppressClick = useRef(false);

  // Keep the view within the projection's bounds and local zoom limit.
  const clamp = ([x, y, w]: Box): Box => {
    const scale = Math.min(1, Math.max(closest, w / width));
    const [nw, nh] = [width * scale, height * scale];
    return [Math.min(Math.max(x, 0), width - nw), Math.min(Math.max(y, 0), height - nh), nw, nh];
  };

  const glideTo = (target: Box) => {
    cancelAnimationFrame(glide.current);
    glide.current = 0;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) { setBox(target); settle(); return; }
    const from = view.current;
    const start = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / GLIDE);
      const ease = 1 - (1 - t) ** 3;
      setBox(from.map((v, i) => v + (target[i] - v) * ease) as Box);
      if (t < 1) glide.current = requestAnimationFrame(step);
      else { glide.current = 0; settle(); }
    };
    glide.current = requestAnimationFrame(step);
  };

  const turn = ([longitude, latitude]: GlobeRotation) => {
    turning.current = [((longitude + 180) % 360 + 360) % 360 - 180, Math.max(-90, Math.min(90, latitude))];
    pendingTurn.current = true;
    rotationChanged.current = true;
    moveCamera();
  };
  const turnTo = (target: GlobeRotation, targetBox?: Box) => {
    cancelAnimationFrame(glide.current);
    glide.current = 0;
    if (reducedMotion()) {
      turn(target);
      if (targetBox) setBox(targetBox);
      settle();
      return;
    }
    const from = turning.current;
    const fromBox = view.current;
    const longitude = ((target[0] - from[0] + 180) % 360 + 360) % 360 - 180;
    const start = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / GLIDE);
      const ease = 1 - (1 - t) ** 3;
      turn([from[0] + longitude * ease, from[1] + (target[1] - from[1]) * ease]);
      if (targetBox) setBox(fromBox.map((value, i) => value + (targetBox[i] - value) * ease) as Box);
      if (t < 1) glide.current = requestAnimationFrame(step);
      else { glide.current = 0; settle(); }
    };
    glide.current = requestAnimationFrame(step);
  };

  // Bring the focus into view if it lies outside the middle of the view.
  const [fx, fy] = focus ?? [NaN, NaN];
  useEffect(() => {
    if (!zoomable || Number.isNaN(fx)) return;
    const [x, y, w, h] = view.current;
    if (projection === "globe") {
      turnTo(geographicPoint(map, [fx, fy]), [(width - w) / 2, (height - h) / 2, w, h]);
      return;
    }
    const inside = fx > x + w * 0.2 && fx < x + w * 0.8 && fy > y + h * 0.2 && fy < y + h * 0.8;
    if (!inside) glideTo(clamp([fx - w / 2, fy - h / 2, w, h]));
    // Only a new focus or map moves the view, never the view itself.
  }, [fx, fy, zoomable, projection, map]);

  useEffect(() => {
    pointers.current.clear();
    drag.current = null;
    pinch.current = null;
    return () => { cancelAnimationFrame(glide.current); glide.current = 0; moving.current = false; };
  }, [projection, map]);

  /// The map point under a pointer.
  const pointAt = (clientX: number, clientY: number): [number, number] => {
    const ctm = svg.current?.getScreenCTM();
    if (!ctm) return [0, 0];
    const p = new DOMPoint(clientX, clientY).matrixTransform(ctm.inverse());
    return [p.x, p.y];
  };

  const zoom = (factor: number, [px, py]: [number, number]) => {
    cancelAnimationFrame(glide.current);
    glide.current = 0;
    setBox(([x, y, w, h]) => {
      const scale = Math.min(1, Math.max(closest, (w * factor) / width)) / (w / width);
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
    drag.current = { x, y, scale: ctm ? 1 / ctm.a : 0, box: view.current, rotation: [...turning.current], moved };
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
      glide.current = 0;
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
      const factor = Math.min(1, Math.max(closest, w * p.distance / distance / width)) / (w / width);
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
    glide.current = 0;
    svg.current?.setPointerCapture(e.pointerId);
    if (projection === "globe") {
      const degrees = d.scale / (height / 2) * 180 / Math.PI;
      turn([d.rotation[0] - dx * degrees, d.rotation[1] + dy * degrees]);
      return;
    }
    const [x, y, w, h] = d.box;
    setBox(clamp([x - dx * d.scale, y - dy * d.scale, w, h]));
  };
  const up = (e: PointerEvent<SVGSVGElement>) => {
    if (!pointers.current.has(e.pointerId)) return;
    pointers.current.delete(e.pointerId);
    if (svg.current?.hasPointerCapture(e.pointerId)) svg.current.releasePointerCapture(e.pointerId);
    pinch.current = null;
    const remaining = [...pointers.current.values()][0];
    if (remaining) startDrag(...remaining, true);
    else { drag.current = null; if (!glide.current) settle(); }
  };
  const dragged = () => suppressClick.current;
  const selectedRegions = lands.size > 0 ? [...lands] : [...new Set(overview.communities.filter((c) => chosen.has(c.id)).flatMap((c) => c.lands))];
  const fitSelection = () => {
    const regions = selectedRegions.map((id) => map.regions[id]).filter((region) => region !== undefined);
    if (regions.length === 0) return;
    if (projection === "globe") {
      const vectors = regions.map(({ center: [longitude, latitude] }) => {
        const lon = longitude * Math.PI / 180, lat = latitude * Math.PI / 180;
        return [Math.cos(lat) * Math.cos(lon), Math.cos(lat) * Math.sin(lon), Math.sin(lat)];
      });
      const center = vectors.reduce((sum, vector) => sum.map((value, i) => value + vector[i]), [0, 0, 0]);
      const magnitude = Math.hypot(...center);
      const normal = magnitude < 1e-8 ? vectors[0] : center.map((value) => value / magnitude);
      const target: GlobeRotation = [Math.atan2(normal[1], normal[0]) * 180 / Math.PI, Math.asin(normal[2]) * 180 / Math.PI];
      let angle = 0;
      for (const region of regions) for (const [longitude, latitude] of region.boundary) {
        const lon = longitude * Math.PI / 180, lat = latitude * Math.PI / 180;
        const dot = normal[0] * Math.cos(lat) * Math.cos(lon) + normal[1] * Math.cos(lat) * Math.sin(lon) + normal[2] * Math.sin(lat);
        angle = Math.max(angle, Math.acos(Math.max(-1, Math.min(1, dot))));
      }
      const scale = Math.min(1, Math.max(closest, angle >= Math.PI / 2 ? 1 : Math.sin(angle) * 1.3));
      const w = width * scale, h = height * scale;
      turnTo(target, [(width - w) / 2, (height - h) / 2, w, h]);
      return;
    }
    const points = regions.flatMap((region) => region.boundary.map((point) => {
      const [x, y] = chartPoint(map, point);
      // The other seam fragment stays at the opposite edge of the chart.
      return [regions.length === 1 ? x + Math.round((region.site[0] - x) / map.width) * map.width : x, y] as MapPoint;
    }));
    const xs = points.map((p) => p[0]), ys = points.map((p) => p[1]);
    const left = Math.min(...xs), right = Math.max(...xs), top = Math.min(...ys), bottom = Math.max(...ys);
    const scale = Math.max((right - left) / map.width, (bottom - top) / map.height) * 1.3;
    const w = map.width * Math.min(1, Math.max(closest, scale));
    const h = w * map.height / map.width;
    glideTo(clamp([(left + right - w) / 2, (top + bottom - h) / 2, w, h]));
  };

  const byRegion = useMemo(() => peoplesByRegion(overview), [overview]);
  const dress = useMemo(() => chartDress(map), [map]);
  const riverPaths = useMemo(() => riverDress(map), [map]);
  const recordedRoutes = useMemo(() => overview.moves.map((move) => move.path.map((id) => map.regions[id].site)), [overview.moves, map]);
  const settlementRoutes = useMemo(() => settlement?.plan?.routes.map((route) => route.path.map((id) => map.regions[id].site)) ?? [], [settlement?.plan, map]);
  const reachEvidence = useMemo(() => {
    if (!reach) return null;
    const founder = overview.communities.find((community) => community.id === reach.people);
    if (!founder || (known && !known.has(founder.region))) return null;
    const encounters = reach.preview.pairs
      .filter((pair) => (pair.a === founder.id || pair.b === founder.id) && pair.reach !== "apart")
      .flatMap((pair) => {
        const other = overview.communities.find((community) => community.id === (pair.a === founder.id ? pair.b : pair.a));
        if (!other || (known && !known.has(other.region))) return [];
        const effort = pair.reach === "sea" ? pair.voyage : pair.walk;
        return [{
          pair,
          other,
          points: [map.regions[founder.region].site, map.regions[other.region].site] as MapPoint[],
          evidence: `${founder.name} and ${other.name}: ${pair.reach === "sea" ? "would need boats" : "may meet on foot"}${effort === null ? "" : ` · ${Math.round(effort).toLocaleString()} effort-km`}`,
        }];
      });
    return { founder, encounters };
  }, [reach, overview.communities, map, known]);
  const letterScale = Math.max(Math.sqrt(box[2] / width),
    14 / ((svg.current?.getScreenCTM()?.a ?? (root.current?.clientWidth ?? window.innerWidth) / width) * 0.24));
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
  const lakePaths = useMemo(() => map.lakes.map((lake) => ({ lake, ...lakeOutline(map, lake) })), [map]);
  const namesByLake = useMemo(() => new Map(lakeNames.map((view) => [view.lake, view])), [lakeNames]);
  const namedLake = (lake: Lake) => {
    const view = namesByLake.get(lake.id);
    if (!view) return null;
    const shores = lake.regions.flatMap((id) => byRegion.get(id) ?? []);
    const shoreVariety = shores.length ? shores.reduce((a, b) => a.size >= b.size ? a : b).variety : undefined;
    return riverName(view, selectedVariety) ?? riverName(view, shoreVariety);
  };
  const grounds = useMemo(() => map.regions.filter((r) => r.terrain !== "sea"), [map]);
  const hearts = useMemo(
    () => new Map([...byRegion].map(([region, here]) => [region, here.filter((c) => c.region === region)])),
    [byRegion],
  );
  // Peoples sharing a region stack their labels around its centre.
  const at = useMemo(() => {
    const out = new Map<number, { point: MapPoint; offset: number }>();
    for (const [region, here] of hearts) {
      const point = map.regions[region].site;
      here.forEach((c, i) => out.set(c.id, { point, offset: (i - (here.length - 1) / 2) * LINE }));
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
        const ends = sharedBorder(map, r.id, n);
        if (ends.length >= 2) out.push({ a: r.id, b: n, ends });
      }
    }
    return out;
  }, [map]);

  // A closed sphere has no map edges; retain only each realm's outer borders.
  const realms = useMemo(() => {
    if (!states) return [];
    return overview.states.filter((s) => s.fell === null).map((state) => {
      const held = new Set(state.lands);
      const edges = state.lands.flatMap((id) => map.regions[id].neighbours
        .filter((neighbour) => !held.has(neighbour))
        .map((neighbour) => sharedBorder(map, id, neighbour)));
      return { state, edges };
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
      const zone = climateZones.get(map.regions[region].climateZone!);
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
  const contactPaths = useMemo(() => overview.contacts.map((k) => ({
    k,
    points: [map.regions[overview.communities[k.a].region].site, map.regions[overview.communities[k.b].region].site] as MapPoint[],
  })), [overview.contacts, overview.communities, map]);
  const dealings = contacts
    ? contactPaths.filter(({ k }) => {
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
  const pilgrimRoads = useMemo(() => faiths
    ? overview.religions.flatMap((religion) => religion.pilgrims
      .filter((p) => !p.path.some(hidden))
      .map((p) => ({ religion, p, points: p.path.map((id) => map.regions[id].site) })))
    : [], [faiths, overview.religions, map, known]);
  // Labels grow more slowly than the land as the view closes in.
  const label = letterScale;
  // Land a veiling language does not know.
  function hidden(region: number): boolean {
    return known !== null && !known.has(region);
  }


  const possible = new Set(settlement?.options.filter((o) => o.reason === null).map((o) => o.region));
  const remains = new Set(settlement?.plan?.remaining.lands);
  const arrives = new Set(settlement?.plan?.arriving.lands);
  const shape = (r: WorldMap["regions"][number]) => {
    const sea = r.terrain === "sea";
    const veiled = !sea && hidden(r.id);
    const colour = veiled ? null : colourOf(r.id);
    return (
      <g key={r.id} data-chart-region={r.id} onClick={sea ? undefined : () => dragged() || onLand(r.id)}>
        <title>{sea ? "Sea" : veiled ? "Unknown land" : (nameOf(r.id) ?? `Unnamed ${TERRAIN_NAME[r.terrain].toLowerCase()}`)}</title>
        <path
          data-region={r.id}
          className={`land terrain-${r.terrain}${sea ? "" : " open"}${lands.has(r.id) ? " shown" : ""}${reader?.region === r.id ? " lens-heart" : ""}`}
          {...bindPath((view) => view.region(r))}
        />
        {colour ? <path className={tint.kind === "weather" ? "weather-wash" : "claim"} data-region={r.id} {...bindPath((view) => view.region(r))} style={{ fill: colour }} /> : null}
        {!sea && settlement ? <path {...bindPath((view) => view.region(r))} className={`settlement-land${possible.has(r.id) ? " possible" : ""}${remains.has(r.id) ? " remaining" : ""}${arrives.has(r.id) ? " arriving" : ""}`} /> : null}
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
    for (const region of map.regions) {
      if (region.terrain === "sea" || hidden(region.id)) continue;
      const fill = colourOf(region.id);
      if (fill) fills.set(region.id, { owners: (owners.get(region.id) ?? []).join(","), fill });
    }
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
      const claim = node.querySelector<SVGPathElement>(`.claim[data-region="${region}"]`);
      if (next && claim) { claim.classList.add("map-holding-gained"); changed.push(claim); }
      if (old && layer) {
        const ground = node.querySelector<SVGPathElement>(`.land[data-region="${region}"]`);
        if (!ground) continue;
        const ghost = document.createElementNS("http://www.w3.org/2000/svg", "path");
        ghost.setAttribute("d", ground.getAttribute("d")!);
        ghost.setAttribute("class", "claim map-holding-lost");
        ghost.style.fill = old.fill;
        const binding = { draw: (view: Cartography) => view.region(map.regions[region]), d: ground.getAttribute("d")!, hide: null, revision: drawRevision.current };
        paths.current.set(ghost, binding);
        projectionLayers.current.find((layer) => layer.mask === 7)?.paths.set(ghost, binding);
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
      ghosts.forEach((element) => {
        paths.current.delete(element as SVGPathElement);
        for (const layer of projectionLayers.current) layer.paths.delete(element as SVGPathElement);
        element.remove();
      });
    };
    const timer = window.setTimeout(clear, 400);
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    const instant = () => { if (media.matches) clear(); };
    media.addEventListener("change", instant);
    return () => { window.clearTimeout(timer); clear(); media.removeEventListener("change", instant); };
  }, [overview, animateChanges, motionMemory, cartography, known, tint]);

  return (
    <div ref={root} className={`mapview projection-${projection}${zoomable ? " zoomable" : ""} ${box[2] / width > 0.6 ? "zoom-far" : box[2] / width < 0.25 ? "zoom-close" : "zoom-mid"}`}
      data-longitude={turning.current[0]} data-latitude={turning.current[1]}>
      <svg
        ref={svg}
        viewBox={box.join(" ")}
        role="group"
        tabIndex={zoomable ? 0 : undefined}
        onKeyDown={(e) => {
          if (!zoomable || e.target !== e.currentTarget) return;
          const [x, y, w, h] = view.current;
          if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
            e.preventDefault(); cancelAnimationFrame(glide.current); glide.current = 0;
            if (projection === "globe") {
              turn([turning.current[0] + (e.key === "ArrowLeft" ? -15 : e.key === "ArrowRight" ? 15 : 0),
                turning.current[1] + (e.key === "ArrowUp" ? 15 : e.key === "ArrowDown" ? -15 : 0)]);
              return;
            }
            setBox(clamp([x + (e.key === "ArrowLeft" ? -w / 8 : e.key === "ArrowRight" ? w / 8 : 0),
              y + (e.key === "ArrowUp" ? -h / 8 : e.key === "ArrowDown" ? h / 8 : 0), w, h]));
          } else if (["+", "=", "-", "Home", "f", "F"].includes(e.key)) {
            e.preventDefault();
            if (e.key === "Home") {
              if (projection === "globe") turnTo([0, 20], full);
              else glideTo(full);
            } else if (e.key.toLowerCase() === "f") fitSelection();
            else zoom(e.key === "-" ? 1 / 0.7 : 0.7, [x + w / 2, y + h / 2]);
          }
        }}
        aria-label={`${projection === "globe" ? "Globe" : "Chart"} of the world in year ${generation * YEARS}`}
        style={{ "--label": label } as CSSProperties}
        onPointerDown={down}
        onPointerMove={move}
        onPointerUp={up}
        onPointerCancel={up}
        onPointerLeave={(e) => { if (!svg.current?.hasPointerCapture(e.pointerId)) up(e); }}
      >
        <defs>
          <clipPath id={surfaceClip}><path d={cartography.sphere} /></clipPath>
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
        <g className="map-surface" clipPath={`url(#${surfaceClip})`}>
        <g className="lands">
          <path className="terrain-sea" d={cartography.sphere} />
          {dressUnder(dress, cartography, bindPath)}
          {grounds.map(shape)}
        </g>
        {dressOver(dress, cartography, bindAnchor)}
        <g className="chart-rivers">
          {riverPaths.map(({ river, points, tier }) => {
            if (river.course.every(hidden)) return null;
            const name = namedRiver(river);
            const failed = climateRivers.get(river.id)?.flowing === false;
            return <g key={river.id} className={`chart-river river-tier-${tier}${failed ? " failed" : ""}`}
              role={onRiver ? "button" : undefined} tabIndex={onRiver ? 0 : undefined}
              aria-label={name?.spelled ?? "Unnamed river"}
              onClick={() => dragged() || onRiver?.(river.id)}
              onKeyDown={(e) => { if (onRiver && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); onRiver(river.id); } }}>
              <title>{`${name?.spelled ?? "Unnamed river"}${failed ? ", flow has failed" : ""}`}</title>
              <path className="river-ink" {...bindPath((view) => view.line(points), "parent")} />
              <path className="river-hit" {...bindPath((view) => view.line(points))} />
            </g>;
          })}
        </g>
        <g className="chart-lakes">
          {lakePaths.map(({ lake, shore, ripple }) => {
            if (lake.regions.every(hidden)) return null;
            const name = namedLake(lake);
            return <g key={lake.id} className="chart-lake" data-lake={lake.id}
              role={onLake ? "button" : undefined} tabIndex={onLake ? 0 : undefined}
              aria-label={name?.spelled ?? "Unnamed lake"}
              onClick={() => dragged() || onLake?.(lake.id)}
              onKeyDown={(e) => { if (onLake && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); onLake(lake.id); } }}>
              <title>{name?.spelled ?? "Unnamed lake"}</title>
              <path className="lake-shore" {...bindPath((view) => view.area(shore), "parent")} />
              <path className="lake-ripple" {...bindPath((view) => view.line(ripple))} pointerEvents="none" />
            </g>;
          })}
        </g>
        {tint.kind === "weather" ? <g className="zone-borders" aria-hidden="true">
          {borders.filter(({ a, b }) => map.regions[a].climateZone !== map.regions[b].climateZone).map(({ a, b, ends }) =>
            <path key={`${a}-${b}`} {...bindPath((view) => view.line(ends))} />)}
        </g> : null}
        {known ? (
          <g className="veil" aria-hidden="true">
            {grounds.filter((r) => hidden(r.id)).map((r) => (
              <path key={r.id} {...bindPath((view) => view.region(r))} />
            ))}
          </g>
        ) : null}
        {reachEvidence ? <g className="reach-overlay" role="group"
          aria-label={`Possible first encounters of the ${reachEvidence.founder.name}`}>
          <path className="reach-homeland" {...bindPath((view) => view.region(map.regions[reachEvidence.founder.region]))} pointerEvents="none" />
          {reachEvidence.encounters.map(({ pair, other, points, evidence }) => {
            const point = pointFor(points[0]);
            const together = other.region === reachEvidence.founder.region;
            return <g key={`${pair.a}:${pair.b}`}>
              <path className="reach-neighbour" {...bindPath((view) => view.region(map.regions[other.region]))} pointerEvents="none" />
              <path className={`reach-line ${pair.reach === "sea" ? "reach-sea" : "reach-foot"}`}
                {...(together ? { d: "M0,0c-.3,-.4 .3,-.4 0,0", ref: bindAnchor(points[0]) } : bindPath((view) => view.line(points)))}
                data-people={other.id} aria-label={evidence}
                data-label-x={together ? point[0] : undefined} data-label-y={together ? point[1] : undefined}
                data-label-scale={together ? true : undefined}
                transform={together ? `translate(${point.join(" ")}) scale(${label})` : undefined}>
                <title>{evidence}</title>
              </path>
            </g>;
          })}
        </g> : null}
        <g className="continent-names">
          {overview.continents.map((c) => {
            const mass = map.landmasses[c.landmass];
            if (!c.name || !mass || mass.regions.every(hidden)) return null;
            const point = pointFor(map.regions[mass.anchor].site);
            const [x, y] = point;
            return (
              <text
                key={c.landmass}
                ref={bindAnchor(map.regions[mass.anchor].site)}
                data-label-x={x} data-label-y={y}
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
          {isoglosses.map(({ a, b, ends }) => (
            <path key={`${a}-${b}`} className="isogloss" {...bindPath((view) => view.line(ends))} />
          ))}
        </g>
        <g className="state-borders" aria-hidden="true">
          {realms.map(({ state, edges }) => (
            <path key={state.id} data-state={state.id} {...bindPath((view) => edges.map((ends) => view.line(ends)).join(""))} style={{ stroke: hue(state.id) }} />
          ))}
        </g>
        {names ? (
          <g className="place-names" aria-hidden="true">
            {overview.places.map((p) => {
              if (hidden(p.region)) return null;
              const point = pointFor(map.regions[p.region].site);
              const [x, y] = point;
              const here = hearts.get(p.region)?.length ?? 0;
              const top = here > 0 ? y - (here / 2) * LINE * label - 0.06 * label : y;
              return (
                <text key={p.region} x={x} y={top} className={here > 0 ? "place-name" : "place-name left"}
                  ref={bindAnchor(map.regions[p.region].site)}
                  data-region={p.region}
                  data-label-x={x} data-label-y={y} data-label-dy={here > 0 ? -(here / 2) * LINE - 0.06 : 0}
                  data-label-kind="land" data-label-priority={2}>
                  {nameOf(p.region)}
                </text>
              );
            })}
          </g>
        ) : null}
        {names ? <g className="river-names" aria-hidden="true">
          {riverPaths.map(({ river, points }) => {
            const name = namedRiver(river);
            if (!name || river.course.every(hidden)) return null;
            const id = `${routeHead}-river-${river.id}`;
            return <g key={river.id}>
              <defs><path id={id} {...bindPath((view) => view.labelLine(points), "grandparent")} /></defs>
              <text className={`hand-${overview.varieties[name.variety].family % 5}`} dy={-0.06 * label}
                data-label-kind="river" data-label-priority={1}>
                <textPath href={`#${id}`} startOffset="50%" textAnchor="middle">{name.spelled}</textPath>
              </text>
            </g>;
          })}
        </g> : null}
        {names ? <g className="lake-names">
          {lakePaths.map(({ lake, center }) => {
            const name = namedLake(lake);
            if (!name || lake.regions.every(hidden)) return null;
            const [x, y] = pointFor(center);
            return <text key={lake.id} ref={bindAnchor(center)} x={x} y={y + 0.3 * label}
              data-lake={lake.id}
              data-label-x={x} data-label-y={y} data-label-dy={0.3} data-label-kind="lake" data-label-priority={1}
              className={`hand-${overview.varieties[name.variety].family % 5}`}
              onClick={() => dragged() || onLake?.(lake.id)}>{name.spelled}</text>;
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
                <g key={i}>
                <path
                  className={`route${m.overseas ? " overseas" : ""}${mine ? " chosen" : ""}`}
                  {...bindPath((view) => view.line(recordedRoutes[i]))}
                  style={{
                    stroke: hue(family(mover)),
                    strokeOpacity: mine ? 1 : 0.3 + 0.6 * Math.max(0, 1 - age / ROUTE_FADE),
                  }}
                >
                  <title>
                    {`${mover.name} ${m.kind === "split" ? "went out" : "moved"} to ${nameOf(m.to) ?? "new land"}${m.overseas ? " by sea" : ""}, year ${m.generation * YEARS}`}
                  </title>
                </path>
                  <path className="route-terminal" {...bindPath((view) => view.endLine(recordedRoutes[i]))}
                    markerEnd={`url(#${routeHead})`} pointerEvents="none"
                    fill="none" stroke="transparent" strokeWidth={mine ? 3 : 1.5} vectorEffect="non-scaling-stroke" />
                </g>
              );
            })}
          </g>
        ) : null}
        {settlement?.plan ? <g className="settlement-journeys" aria-hidden="true">
          {settlement.plan.routes.map((r, i) => <g key={r.from}>
            <path className={r.by_sea ? "sea-journey" : ""} {...bindPath((view) => view.line(settlementRoutes[i]))} />
            <path className="route-terminal" {...bindPath((view) => view.endLine(settlementRoutes[i]))}
              markerEnd={`url(#${routeHead})`} stroke="transparent" vectorEffect="non-scaling-stroke" />
          </g>)}
        </g> : null}
        <g className="dealings">
          {dealings.map(({ k, points }, i) => {
            return (
              <path
                key={i}
                className={`dealing dealing-${k.kind}`}
                {...bindPath((view) => view.line(points))}
                style={{ strokeOpacity: 0.35 + 0.65 * k.intensity }}
              />
            );
          })}
        </g>
        {pilgrimRoads.length > 0 ? (
          <g className="pilgrim-roads">
            {pilgrimRoads.map(({ religion, p, points }) => (
              <path
                key={`${religion.id}-${p.people}-${p.to}`}
                className="pilgrim-road"
                {...bindPath((view) => view.line(points))}
                style={{ stroke: hue(religion.id) }}
              >
                <title>{`Pilgrims of ${religion.name}, the ${overview.communities[p.people].name}, since year ${p.since * YEARS}`}</title>
              </path>
            ))}
          </g>
        ) : null}
        <g className="peoples">
          {overview.communities.map((c) => {
            if (c.ended !== null || hidden(c.region)) return null;
            const point = at.get(c.id);
            if (!point) return null;
            const { point: site, offset } = point;
            const [x, y] = pointFor(site);
            const word = wordBy.get(c.id);
            const localName = selectedVariety === undefined ? c.name : c.exonyms.find((name) =>
              name.by === reader?.id || overview.communities[name.by]?.variety === selectedVariety)?.name ?? c.name;
            const text = tint.kind === "words" ? (word?.spelled ?? "—") : localName;
            return (
              <g
                key={c.id}
                ref={bindAnchor(site)}
                className={chosen.has(c.id) ? "people chosen" : "people"}
                data-region={c.region}
                data-label-x={x} data-label-y={y} data-label-dy={offset}
                transform={`translate(${x} ${y + offset * label})`}
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
                <circle className="people-dot" cx={0} cy={0} r={0.06 * label}
                  style={{ fill: tint.kind === "words" && word ? hue(word.group) : hue(family(c)) }} />
                <text x={0} y={0} className={`hand-${family(c) % 5}`}
                  data-label-kind="people" data-people={c.id} data-label-priority={100 + c.size}
                  style={{ fill: tint.kind === "words" && word ? hue(word.group) : hue(family(c)) }}>
                  {text}
                </text>
                {overview.varieties[c.variety].name !== text && tint.kind === "peoples" ?
                  <text x={0} y={0.24 * label} className="tongue-name"
                    data-label-x={0} data-label-y={0} data-label-dy={0.24}
                    data-label-kind="tongue" data-people={c.id} data-label-priority={4}>
                    {overview.varieties[c.variety].name}
                  </text> : null}
              </g>
            );
          })}
        </g>
        <g className="state-capitals">
          {realms.map(({ state }) => {
            if (hidden(state.capital)) return null;
            const point = pointFor(map.regions[state.capital].site);
            const [x, y] = point;
            const offset = (hearts.get(state.capital)?.length ?? 0) / 2 * LINE + 0.2;
            return (
              <g
                key={state.id}
                ref={bindAnchor(map.regions[state.capital].site)}
                className="state-capital"
                data-label-x={x} data-label-y={y} data-label-dy={offset} data-label-scale
                transform={`translate(${x} ${y + offset * label}) scale(${label})`}
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
            if (hidden(religion.land)) return null;
            const point = pointFor(map.regions[religion.land].site);
            const [x, y] = point;
            const offset = religions.slice(0, i).filter((r) => r.land === religion.land).length;
            return (
              <g
                key={religion.id}
                ref={bindAnchor(map.regions[religion.land].site)}
                className="founding-marker"
                data-label-x={x} data-label-y={y} data-label-dx={-(0.22 + offset * 0.24)} data-label-dy={-0.22} data-label-scale
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
            if (hidden(shrine.region)) return null;
            const point = pointFor(map.regions[shrine.region].site);
            const [x, y] = point;
            return (
              <g
                key={`shrine-${religion.id}-${shrine.region}`}
                ref={bindAnchor(map.regions[shrine.region].site)}
                className="founding-marker shrine-marker"
                data-label-x={x} data-label-y={y} data-label-dx={0.24 + offset * 0.24} data-label-dy={0.2} data-label-scale
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
            if (hidden(people.region)) return null;
            const point = pointFor(map.regions[people.region].site);
            const [x, y] = point;
            return (
              <g
                key={id}
                ref={bindAnchor(map.regions[people.region].site)}
                className="founding-marker"
                data-label-x={x} data-label-y={y} data-label-dx={0.22} data-label-dy={-0.22} data-label-scale
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
        </g>
        {projection === "globe" ? <path className="globe-limb" d={cartography.sphere} aria-hidden="true" /> : null}
      </svg>
      {zoomable ? (
        <div className="map-zoom" role="group" aria-label={projection === "globe" ? "Globe controls" : "Chart controls"}>
          {selectedRegions.length > 0 ? <button type="button" title="Fit the selected lands (F)" aria-label="Fit the selected lands" onClick={fitSelection}><LocateFixed size={16} /></button> : null}
          <button type="button" title="Closer (+)" aria-label="Zoom in" disabled={box[2] <= width * closest + 0.001} onClick={() => zoom(0.7, [view.current[0] + view.current[2] / 2, view.current[1] + view.current[3] / 2])}>
            <Plus size={16} />
          </button>
          <button type="button" title="Farther (−)" aria-label="Zoom out" disabled={box[2] >= width - 0.001} onClick={() => zoom(1 / 0.7, [view.current[0] + view.current[2] / 2, view.current[1] + view.current[3] / 2])}>
            <Minus size={16} />
          </button>
          <button type="button" title={projection === "globe" ? "The whole globe (Home)" : "The whole chart (Home)"}
            aria-label={projection === "globe" ? "The whole globe" : "The whole chart"}
            onClick={() => projection === "globe" ? turnTo([0, 20], full) : glideTo(full)}>
            <Maximize size={16} />
          </button>
        </div>
      ) : null}
      {onProjection ? <div className="map-projection-control">
        <MapProjectionSwitch value={projection} onChange={onProjection} />
      </div> : null}
      {zoomable ? <div className="chart-scale">
        <span ref={scaleRule} className="scale-rule"><span ref={scaleText} /></span>
        <span className="chart-help">{projection === "globe" ? "Drag to turn · scroll to zoom" : "Scroll to zoom · drag to explore"}</span>
      </div> : null}
      {ROSE}
    </div>
  );
}

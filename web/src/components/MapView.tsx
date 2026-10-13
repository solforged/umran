import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type PointerEvent, type ReactNode, type RefObject, type SetStateAction } from "react";
import { geoAzimuthalEquidistant } from "d3-geo";
import { LocateFixed, Maximize, Minus, Plus } from "lucide-react";
import type { ClimateView, Community, Craft, EthosAxis, FoundingPreview, Lake, LakeNamesView, Overview, River, RiverNamesView, SettlementPreview, WordMap, WorldMap } from "../model";
import type { ShelfPeople } from "../shelf";
import { reducedMotion } from "../motion";
import { YEARS } from "../model";
import { continentLabel, ETHOS_POLES, hue, landCardLabel, landLabel, LEG_BY, TERRAIN_NAME } from "../lore";
import { chartPoint, createCartography, geographicPoint, lakeOutline, riverPoints, riverRoutePoints, sharedBorder, type Cartography, type GlobeRotation, type MapPoint, type MapProjection } from "../cartography";
import { MapProjectionSwitch } from "./MapProjectionSwitch";
import type { MapFeature } from "../gazetteer";

export interface MapMotionReading {
  overview: Overview;
  fills: Map<number, { owners: string; fill: string }>;
}

/// Gap between stacked labels of peoples sharing a region, in map units.
const LINE = 0.3;
/// Generations over which a route fades to its faintest.
const ROUTE_FADE = 40;
/// A passage keeps regional context even when it names only one small land.
const STORY_SPAN_KM = 1800;
/// Colours of the isogloss view: land whose people underwent the change,
/// and peopled land that did not.
const CHANGED = "hsl(24 75% 50%)";
const UNCHANGED = "hsl(210 12% 55%)";
/// How long the view takes to glide to something, in milliseconds.
const GLIDE = 450;
/// Pointer travel, in pixels, before a press becomes a drag.
const DRAG_START = 4;
/// One physical scale governs lettering, geometry layers, and projection work.
const WORLD_KM_PER_PIXEL = 20;
const LAND_KM_PER_PIXEL = 6;
const TONGUE_KM_PER_PIXEL = 1.5;
const LAND_LETTERING_LAYER = 8;
function chartScale(kmPerPixel: number) {
  const land = kmPerPixel < LAND_KM_PER_PIXEL;
  const level = kmPerPixel >= WORLD_KM_PER_PIXEL ? "zoom-far" : land ? "zoom-close" : "zoom-mid";
  return {
    level,
    land,
    tongues: kmPerPixel < TONGUE_KM_PER_PIXEL,
    mask: (level === "zoom-far" ? 1 : land ? 4 : 2) | (land ? LAND_LETTERING_LAYER : 0),
    landPixels: Math.min(20, 13 + 3 * Math.max(0, Math.log2(LAND_KM_PER_PIXEL / Math.max(0.01, kmPerPixel)))),
  };
}
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

export interface MapStory {
  regions: number[];
  route: { from: number; to: number; generation: number }[];
  label?: string;
}

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
          data-mark-size={size}
          data-mark-extent={kind === "mountains" ? 2 : kind === "hills" ? 0.22 : kind === "forest" ? 0.09 : kind === "desert" ? 0.016 : 0.12}
          data-mark-pixels={kind === "mountains" ? 14 : kind === "desert" ? 1.5 : 5}
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

/// Rectangles that share any ink, in the SVG's own units.
function clash(a: DOMRect, b: DOMRect): boolean {
  return a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
}

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
  // Neighbouring peoples' names would print over one another at this size.
  // Largest first, a name that would cover one already lettered is left
  // off, as the full chart does, and the next largest is tried instead.
  const named = peoples.slice(0, NAMED * 2);
  const lettering = useRef<SVGGElement>(null);
  useLayoutEffect(() => {
    const placed: DOMRect[] = [];
    // An undrawn miniature (display: none) has no ink to measure.
    if (!lettering.current?.getClientRects().length) return;
    for (const text of lettering.current.querySelectorAll("text")) {
      text.style.visibility = "";
      const box = text.getBBox();
      if (placed.length >= NAMED || placed.some((other) => clash(box, other))) text.style.visibility = "hidden";
      else placed.push(box);
    }
  }, [peoples, map]);
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
        <g className="peoples" ref={lettering}>
          {named.map((p, i) => {
            const [x, y] = cartography.point(map.regions[p.region].site)!;
            const above = named.slice(0, i).filter((q) => q.region === p.region).length;
            return (
              <g key={i} className="people">
                <text x={x} y={y + above * LINE * label} style={{ fill: hue(p.family) }}>
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
  story = null,
  zoomable = false,
  settlement = null,
  animateChanges = false,
  motionMemory,
  camera,
  globeCamera,
  frameOnOpen = false,
  onView,
  onCamera,
  projection = "chart",
  onProjection,
  viewpointControl,
  rotation,
  onRotation,
  reach,
  onPeople,
  onLand,
  onClearLand,
  onContinent,
  onState,
  onReligion,
  onCraft,
  onRiver,
  onLake,
  onCity,
  onInspect,
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
  /// A chosen people, with names and known lands as independent readings.
  lens?: { people: number; names?: boolean; known?: boolean } | null;
  /// A recorded passage, framed temporarily without losing the reader's view.
  story?: MapStory | null;
  zoomable?: boolean;
  settlement?: SettlementPreview | null;
  animateChanges?: boolean;
  motionMemory?: RefObject<MapMotionReading | null>;
  camera?: MapCamera;
  onCamera?: (camera: MapCamera) => void;
  globeCamera?: MapCamera;
  /// Frame living peoples once on each projection when no camera was saved.
  frameOnOpen?: boolean;
  /// A settled reader camera, never the temporary story camera.
  onView?: (camera: MapCamera, projection: MapProjection, rotation: GlobeRotation) => void;
  projection?: MapProjection;
  onProjection?: (projection: MapProjection) => void;
  viewpointControl?: ReactNode;
  rotation?: GlobeRotation;
  onRotation?: (rotation: GlobeRotation) => void;
  reach?: { preview: FoundingPreview; people: number };
  onPeople: (community: number) => void;
  onLand: (region: number) => void;
  /// Founding clears its chosen homeland alongside the map's roving land cursor.
  onClearLand?: () => void;
  onContinent?: (landmass: number) => void;
  onState?: (state: number) => void;
  onReligion?: (religion: number) => void;
  onCraft?: (craft: Craft) => void;
  onLake?: (lake: number) => void;
  onRiver?: (id: number) => void;
  onCity?: (city: number) => void;
  onInspect?: (feature: MapFeature | null, at: [number, number]) => void;
}) {
  const climateZones = useMemo(() => new Map(climate?.zones.map((zone) => [zone.id, zone])), [climate]);
  const climateRivers = useMemo(() => new Map(climate?.rivers.map((river) => [river.id, river])), [climate]);
  const reader = lens ? overview.communities.find((c) => c.id === lens.people) : undefined;
  const readerVariety = reader ? overview.varieties[reader.variety] : undefined;
  const namesVariety = lens?.names === false ? undefined : readerVariety;
  selectedVariety = namesVariety?.id ?? selectedVariety;
  const readerLands = useMemo(() => readerVariety && lens?.known !== false
    ? new Set(readerVariety.knownLands.map((land) => land.region)) : null, [readerVariety, lens?.known]);
  known = readerLands ?? known;
  const turning = useRef<GlobeRotation>(rotation ?? [0, 20]);
  const cartography = useMemo(() => createCartography(map, projection, turning.current), [map, projection]);
  const { width, height } = cartography;
  // At closest, show roughly three mean region widths in either projection.
  const closest = Math.min(1, 3 * Math.sqrt((projection === "globe" ? Math.PI : 1 / Math.PI) / Math.max(1, map.regions.length)));
  const full: Box = useMemo(() => [0, 0, width, height], [width, height]);
  const chartView = useRef<Box>(camera ?? [0, 0, map.width, map.height]);
  const globeView = useRef<Box>(globeCamera ?? [0, 0, map.height, map.height]);
  const view = projection === "chart" ? chartView : globeView;
  const readerCamera = useRef<{ chart: Box; globe: Box; rotation: GlobeRotation } | null>(null);
  const followingStory = useRef(story !== null);
  followingStory.current = story !== null;
  const framed = useRef({ chart: camera !== undefined, globe: globeCamera !== undefined });
  const drawnMap = useRef(map);
  if (drawnMap.current !== map) {
    drawnMap.current = map;
    chartView.current = camera ?? [0, 0, map.width, map.height];
    globeView.current = globeCamera ?? [0, 0, map.height, map.height];
    framed.current = { chart: camera !== undefined, globe: globeCamera !== undefined };
  }
  let box = view.current;
  const root = useRef<HTMLDivElement>(null);
  const [landCursor, setLandCursor] = useState<number | null>(null);
  const [landFocused, setLandFocused] = useState(false);
  const landNavigation = zoomable || onClearLand !== undefined;
  const scaleText = useRef<HTMLSpanElement>(null);
  const scaleRule = useRef<HTMLSpanElement>(null);
  const zoomIn = useRef<HTMLButtonElement>(null);
  const zoomOut = useRef<HTMLButtonElement>(null);
  const labelLayers = useRef<SVGElement[]>([]);
  const labelAnchors = useRef<SVGElement[]>([]);
  const lettering = useRef<SVGTextElement[]>([]);
  const terrainMarks = useRef<{ scale: SVGTransform; size: number; extent: number; pixels: number }[]>([]);
  const largestPeoples = useMemo(() => new Set(overview.communities.filter((people) => people.ended === null)
    .sort((a, b) => b.size - a.size).slice(0, NAMED).map((people) => String(people.id))), [overview.communities]);
  /// Region sites by map-unit cell, for asking which region holds a point,
  /// and each continent's candidate sites for its name; a chart's only.
  const siteGrid = useRef<{ cartography: Cartography; cells: Map<number, [number, number, number][]>; candidates: Map<number, [number, number, number][]> } | null>(null);
  const paths = useRef(new Map<SVGPathElement, { draw: PathDrawer; d: string; hide: HTMLElement | SVGElement | null; revision: number }>());
  const drawnPaths = useMemo(() => new WeakMap<PathDrawer, { revision: number; d: string }>(), [cartography]);
  const drawRevision = useRef(0);
  const projectedAnchors = useRef(new Map<SVGElement, {
    site: MapPoint; translation?: SVGTransform; visible: boolean; revision: number;
  }>());
  // The same physical scale controls visible ink and hidden projection work.
  const projectionLayers = useRef<{ mask: number; paths: typeof paths.current; anchors: typeof projectedAnchors.current; labels: SVGElement[] }[]>([]);
  const layerMask = (node: SVGElement): number => {
    if (node.closest(".place-names, .river-names, .lake-names")) return LAND_LETTERING_LAYER;
    if (node.closest(".chart-marks") && node.querySelector(".mark-tree, .mark-grass, .mark-field, .mark-sand")) return 4;
    if (node.closest(".chart-river:not(.river-tier-3), .routes, .dealings, .pilgrim-roads, .founding-markers")) return 6;
    if (node.closest(".continent-names")) return 3;
    return 7;
  };
  const pendingTurn = useRef(false);
  const moving = useRef(false);
  const settleTimer = useRef(0);
  const rotationChanged = useRef(false);
  const callbacks = useRef({ onRotation, onCamera, onView });
  callbacks.current = { onRotation, onCamera, onView };
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
      if (node) projectedAnchors.current.set(node, {
        site, visible: node.style.display !== "none", revision: -1,
        translation: scale === undefined ? undefined : (node as SVGGraphicsElement).transform.baseVal.getItem(0),
      });
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
    // Both camera dimensions scale together. Read the old screen scale before
    // changing viewBox, then derive the new one without a second SVG layout.
    const pixelsPerUnit = (element.getScreenCTM()?.a ?? 1) * element.viewBox.baseVal.width / value[2];
    const nextBox = value.join(" ");
    if (element.getAttribute("viewBox") !== nextBox) element.setAttribute("viewBox", nextBox);
    const kmPerPixel = (projection === "globe" ? 2 * map.radiusKm / height : map.kmPerUnit) / pixelsPerUnit;
    const policy = chartScale(kmPerPixel);
    const landLettering = policy.land, visibleMask = policy.mask;
    chart.dataset.kmPerPixel = String(kmPerPixel);
    chart.style.setProperty("--small-label-min", `${policy.landPixels / pixelsPerUnit}px`);
    for (const mark of terrainMarks.current) {
      const size = Math.min(mark.size, mark.pixels / (mark.extent * pixelsPerUnit));
      mark.scale.setScale(size, size);
    }
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
        if (binding.translation) {
          binding.translation.setTranslate(point[0], point[1]);
        } else {
          node.dataset.labelX = String(point[0]);
          node.dataset.labelY = String(point[1]);
        }
      });
    }
    const label = Math.max(Math.sqrt(ratio), 14 / (pixelsPerUnit * 0.24));
    for (const layer of labelLayers.current) {
      layer.style.setProperty("--label", String(label));
      layer.style.setProperty("--small-label-min", `${policy.landPixels / pixelsPerUnit}px`);
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
    const level = policy.level;
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
    const named = new Set<string>();
    const chartBounds = chart.getBoundingClientRect();
    const matrix = element.getScreenCTM()!;
    // Namesakes belong to this reading: use the spellings and land anchors
    // in the viewport, not historical names or another tongue's exonyms.
    const landNamesakes = new Map<string, number[]>();
    if (landLettering) for (const text of lettering.current) {
      const name = text.dataset.landName;
      if (!name || text.style.display === "none") continue;
      const x = matrix.a * Number(text.getAttribute("x")) + matrix.e;
      const y = matrix.d * Number(text.getAttribute("y")) + matrix.f;
      if (x < chartBounds.left || x > chartBounds.right || y < chartBounds.top || y > chartBounds.bottom) continue;
      const peers = landNamesakes.get(name);
      if (peers) peers.push(Number(text.dataset.region));
      else landNamesakes.set(name, [Number(text.dataset.region)]);
    }
    // Keep names at their geographic anchors. Priority decides which fit;
    // zoom reveals the rest without sending leaders across the chart.
    for (const text of lettering.current) {
      text.style.visibility = "";
      const name = text.dataset.landName;
      if (name !== undefined) {
        const peers = landNamesakes.get(name);
        const reading = peers && peers.length > 1
          ? landCardLabel(overview, map, Number(text.dataset.region), name, peers) : name;
        if (text.textContent !== reading) text.textContent = reading;
      }
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
    // Overlaid furniture (the cartouche, tools, caption, scale) holds its
    // ground too: the map's own and its container's direct children.
    const occupied: DOMRect[] = [...chart.children, ...chart.parentElement?.children ?? []]
      .filter((node) => node.hasAttribute("data-map-furniture"))
      .map((node) => node.getBoundingClientRect()).filter((rect) => rect.width > 0);
    const surface = new DOMRect(matrix.e, matrix.f, width * pixelsPerUnit, height * pixelsPerUnit);
    const overlaps = (rect: DOMRect) => occupied.some((other) =>
      other.left < rect.right + LABEL_GAP && rect.left < other.right + LABEL_GAP &&
      other.top < rect.bottom + LABEL_GAP && rect.top < other.bottom + LABEL_GAP);
    const fits = (rect: DOMRect) => {
      if (rect.left < Math.max(surface.left, chartBounds.left) + LABEL_GAP ||
        rect.right > Math.min(surface.right, chartBounds.right) - LABEL_GAP ||
        rect.top < Math.max(surface.top, chartBounds.top) + LABEL_GAP ||
        rect.bottom > Math.min(surface.bottom, chartBounds.bottom) - LABEL_GAP) return false;
      if (projection === "globe") {
        const radius = surface.width / 2 - LABEL_GAP;
        const x = surface.x + surface.width / 2, y = surface.y + surface.height / 2;
        if ([rect.left, rect.right].some((left) => [rect.top, rect.bottom].some((top) =>
          (left - x) ** 2 + (top - y) ** 2 > radius ** 2))) return false;
      }
      return !overlaps(rect);
    };
    // Which region holds a point: the nearest region site, since regions are
    // their sites' cells. Indexed in map units, a mean region width per cell,
    // so a pan never moves a cell boundary; a globe reindexes each frame,
    // as it turns, once a continent asks. A chart keeps the index and each
    // continent's candidate sites until its cartography changes.
    const cell = width * Math.sqrt((projection === "globe" ? Math.PI : 1 / Math.PI) / Math.max(1, map.regions.length));
    const kept = projection === "chart" && siteGrid.current?.cartography === cartography ? siteGrid.current : null;
    let cells = kept?.cells ?? null;
    const candidateSites = kept?.candidates ?? new Map<number, [number, number, number][]>();
    const regionAt = (x: number, y: number) => {
      if (!cells) {
        const index = new Map<number, [number, number, number][]>();
        map.regions.forEach((region, id) => {
          const point = cartography.point(region.site);
          if (!point) return;
          const key = Math.floor(point[0] / cell) * 1e5 + Math.floor(point[1] / cell);
          const list = index.get(key);
          if (list) list.push([point[0], point[1], id]); else index.set(key, [[point[0], point[1], id]]);
        });
        cells = index;
        if (projection === "chart") siteGrid.current = { cartography, cells, candidates: candidateSites };
      }
      const key = Math.floor(x / cell) * 1e5 + Math.floor(y / cell);
      let nearest = -1, best = Infinity;
      for (const dx of [-1e5, 0, 1e5]) for (const dy of [-1, 0, 1]) {
        for (const [sx, sy, id] of cells.get(key + dx + dy) ?? []) {
          const d = (sx - x) ** 2 + (sy - y) ** 2;
          if (d < best) { best = d; nearest = id; }
        }
      }
      return nearest;
    };
    for (const item of measured) {
      const { text } = item;
      const { rect } = item;
      const kind = text.dataset.labelKind;
      if (kind === "continent") {
        // A continent's name goes where its own land has room, nearest the
        // engine's anchor, and smaller on a small land. Where none fits, it
        // waits for zoom rather than lettering across the sea. Its last
        // placement is tried first, so a pan does not send it hopping.
        const landmass = Number(text.dataset.landmass);
        const mass = map.landmasses[landmass];
        const anchor = cartography.point(map.regions[mass.anchor].site);
        let placed = false;
        if (anchor && rect.width > 0) {
          let sites = candidateSites.get(landmass);
          if (!sites) {
            const all: [number, number, number][] = [];
            for (const region of mass.regions) {
              const point = cartography.point(map.regions[region].site);
              if (point) all.push([point[0], point[1], region]);
            }
            all.sort((a, b) => (a[0] - anchor[0]) ** 2 + (a[1] - anchor[1]) ** 2 - (b[0] - anchor[0]) ** 2 - (b[1] - anchor[1]) ** 2);
            const step = Math.max(1, Math.ceil(all.length / 60));
            sites = all.filter((_, index) => index % step === 0);
            candidateSites.set(landmass, sites);
          }
          const previous = sites.find((site) => String(site[2]) === text.dataset.placed);
          const order = previous ? [previous, ...sites.filter((site) => site !== previous)] : sites;
          // The heading was measured at the anchor at its last size.
          const shown = Number(text.dataset.fit ?? 1);
          const ax = matrix.a * anchor[0] + matrix.e, ay = matrix.d * anchor[1] + matrix.f;
          for (const fit of [1, 0.8, 0.64]) {
            const scale = fit / shown;
            for (const [x, y, site] of order) {
              const sx = matrix.a * x + matrix.e, sy = matrix.d * y + matrix.f;
              const box = new DOMRect(sx + (rect.left - ax) * scale, sy + (rect.top - ay) * scale, rect.width * scale, rect.height * scale);
              if (!fits(box)) continue;
              const middle = (box.top + box.height / 2 - matrix.f) / matrix.d;
              let covered = 0;
              for (let sample = 0; sample < 9; sample++) {
                const region = regionAt((box.left + box.width * (sample + 0.5) / 9 - matrix.e) / matrix.a, middle);
                if (map.regions[region]?.landmass === landmass) covered++;
              }
              if (covered < 8) continue;
              text.setAttribute("x", String(x));
              text.setAttribute("y", String(y));
              if (fit !== shown) {
                text.style.setProperty("--fit", String(fit));
                text.dataset.fit = String(fit);
              }
              text.dataset.placed = String(site);
              occupied.push(box);
              placed = true;
              break;
            }
            if (placed) break;
          }
        }
        text.style.visibility = placed ? "" : "hidden";
        continue;
      }
      const people = text.dataset.people;
      const allowed = kind === "tongue"
        ? tint.kind === "peoples" && policy.tongues && named.has(people!)
        : kind === "land" || kind === "river" || kind === "lake" ? policy.land
        : kind === "people" ? level !== "zoom-far" || largestPeoples.has(people!)
        : kind === "state" ? level !== "zoom-far" : true;
      const inView = rect.right > chartBounds.left && rect.left < chartBounds.right &&
        rect.bottom > chartBounds.top && rect.top < chartBounds.bottom;
      const visible = allowed && inView && rect.width > 0 && fits(rect);
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
    const km = [50, 100, 200, 500, 1000].filter((length) => length <= value[2] * map.kmPerUnit * 0.15).at(-1) ?? 50;
    if (projection === "chart") scaleRule.current?.style.setProperty("--scale-width", `${km / map.kmPerUnit * pixelsPerUnit}px`);
    else scaleRule.current?.style.removeProperty("--scale-width");
    const caption = projection === "globe"
      ? `Circumference ${Math.round(2 * Math.PI * map.radiusKm).toLocaleString()} km`
      : `${km.toLocaleString()} km at the equator`;
    if (scaleText.current && scaleText.current.textContent !== caption) scaleText.current.textContent = caption;
    const closer = zoomIn.current, farther = zoomOut.current;
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
      if (!followingStory.current) callbacks.current.onRotation?.([...turning.current]);
    }
    if (!followingStory.current) callbacks.current.onView?.([...view.current], projection, [...turning.current]);
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
    if (projection === "chart" && !followingStory.current) callbacks.current.onCamera?.(box);
  };
  useLayoutEffect(() => {
    if (followingStory.current) return;
    if (camera) chartView.current = camera;
    if (globeCamera) globeView.current = globeCamera;
    scheduleDraw();
  }, [camera, globeCamera, projection]);
  useLayoutEffect(() => {
    terrainMarks.current = [...(root.current?.querySelectorAll<SVGGElement>("[data-mark-size]") ?? [])].map((node) => ({
      scale: node.transform.baseVal.getItem(1), size: Number(node.dataset.markSize),
      extent: Number(node.dataset.markExtent), pixels: Number(node.dataset.markPixels),
    }));
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
    if (followingStory.current || !rotation || (rotation[0] === turning.current[0] && rotation[1] === turning.current[1])) return;
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

  const showLandCursor = (region: number) => {
    setLandCursor(region);
    const [x, y, w, h] = view.current;
    const land = map.regions[region];
    if (projection === "globe") {
      turnTo(land.center, [(width - w) / 2, (height - h) / 2, w, h]);
    } else {
      const [px, py] = land.site;
      if (px < x + w * 0.1 || px > x + w * 0.9 || py < y + h * 0.1 || py > y + h * 0.9)
        glideTo(clamp([px - w / 2, py - h / 2, w, h]));
    }
  };
  const initialLand = () => {
    const selected = [...lands].find((id) => map.regions[id]?.terrain !== "sea");
    if (selected !== undefined) return selected;
    const [x, y, w, h] = view.current;
    let closest: number | null = null, distance = Infinity;
    for (const region of map.regions) {
      if (region.terrain === "sea") continue;
      const point = cartography.point(region.site);
      if (!point) continue;
      const d = (point[0] - x - w / 2) ** 2 + (point[1] - y - h / 2) ** 2;
      if (d < distance) { distance = d; closest = region.id; }
    }
    return closest;
  };
  const stepLandCursor = (key: string) => {
    if (landCursor === null) {
      const first = initialLand();
      if (first !== null) showLandCursor(first);
      return;
    }
    const current = map.regions[landCursor];
    const dx = key === "ArrowLeft" ? -1 : key === "ArrowRight" ? 1 : 0;
    const dy = key === "ArrowUp" ? -1 : key === "ArrowDown" ? 1 : 0;
    // A cursor-centred tangent chart keeps the globe's screen directions,
    // but can also reach land beyond the horizon of an isolated island.
    const around = projection === "globe"
      ? geoAzimuthalEquidistant().rotate([-current.center[0], -current.center[1]]).scale(1).translate([0, 0])
      : null;
    let next = current.id, best = Infinity;
    let across = current.id, acrossBest = Infinity;
    for (const region of map.regions) {
      if (region.id === current.id || region.terrain === "sea") continue;
      const point = around?.(region.center);
      let x = point ? point[0] : region.site[0] - current.site[0];
      const y = point ? point[1] : region.site[1] - current.site[1];
      if (!around) x -= Math.round(x / map.width) * map.width;
      const along = x * dx + y * dy;
      if (along <= 1e-9) continue;
      const distance = x * x + y * y;
      if (distance < acrossBest) { acrossBest = distance; across = region.id; }
      const sideways = Math.abs(x * dy - y * dx);
      if (along >= sideways && distance < best) { best = distance; next = region.id; }
    }
    // At a coast or on an island, use the nearest forward land even when
    // none lies within the arrow's 90-degree cone.
    if (next === current.id) next = across;
    if (next !== current.id) showLandCursor(next);
  };

  // Bring the focus into view if it lies outside the middle of the view.
  const [fx, fy] = focus ?? [NaN, NaN];
  const firstFocus = useRef(true);
  useEffect(() => {
    if (firstFocus.current) {
      firstFocus.current = false;
      if (frameOnOpen) return;
    }
    if (!zoomable || story || Number.isNaN(fx)) return;
    const [x, y, w, h] = view.current;
    if (projection === "globe") {
      turnTo(geographicPoint(map, [fx, fy]), [(width - w) / 2, (height - h) / 2, w, h]);
      return;
    }
    const inside = fx > x + w * 0.2 && fx < x + w * 0.8 && fy > y + h * 0.2 && fy < y + h * 0.8;
    if (!inside) glideTo(clamp([fx - w / 2, fy - h / 2, w, h]));
    // Only a new focus or map moves the view, never the view itself.
  }, [fx, fy, zoomable, map]);

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
  const fitRegions = (ids: readonly number[], immediate = false, minimumSpanKm = 0) => {
    const regions = ids.map((id) => map.regions[id]).filter((region) => region !== undefined);
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
      const minimumScale = Math.sin(Math.min(Math.PI / 2, minimumSpanKm / (2 * map.radiusKm)));
      const scale = Math.min(1, Math.max(closest, minimumScale, angle >= Math.PI / 2 ? 1 : Math.sin(angle) * 1.3));
      const w = width * scale, h = height * scale;
      const targetBox: Box = [(width - w) / 2, (height - h) / 2, w, h];
      if (immediate) { turn(target); setBox(targetBox); drawCurrent.current(); settle(); }
      else turnTo(target, targetBox);
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
    const minimumScale = minimumSpanKm / (map.width * map.kmPerUnit);
    const w = map.width * Math.min(1, Math.max(closest, minimumScale, scale));
    const h = w * map.height / map.width;
    const targetBox = clamp([(left + right - w) / 2, (top + bottom - h) / 2, w, h]);
    if (immediate) { setBox(targetBox); drawCurrent.current(); settle(); }
    else glideTo(targetBox);
  };
  useLayoutEffect(() => {
    if (!frameOnOpen || framed.current[projection] || story) return;
    framed.current[projection] = true;
    fitRegions([...new Set(overview.communities.filter((people) => people.ended === null).flatMap((people) => people.lands))], true);
  }, [map, projection, frameOnOpen]);
  const storyKey = story ? JSON.stringify(story) : null;
  useEffect(() => {
    if (story) {
      readerCamera.current ??= { chart: [...chartView.current], globe: [...globeView.current], rotation: [...turning.current] };
      fitRegions(story.regions, false, STORY_SPAN_KM);
    } else if (readerCamera.current) {
      const saved = readerCamera.current;
      readerCamera.current = null;
      chartView.current = saved.chart;
      globeView.current = saved.globe;
      if (projection === "globe") turnTo(saved.rotation, saved.globe);
      else {
        turning.current = saved.rotation;
        pendingTurn.current = true;
        glideTo(saved.chart);
      }
    }
    // The passage, not a new overview or a camera frame, moves the chart.
  }, [storyKey, projection, map]);

  const byRegion = useMemo(() => peoplesByRegion(overview), [overview]);
  const dress = useMemo(() => chartDress(map), [map]);
  const riverPaths = useMemo(() => riverDress(map), [map]);
  const recordedRoutes = useMemo(() => overview.moves.map((move) => {
    let start = 0;
    const legs = move.legs?.length ? move.legs : [{ from: move.from, to: move.to, by: move.bySea ? "sea" as const : "land" as const, km: 0 }];
    return legs.map((leg) => {
      start = move.path.indexOf(leg.from, start);
      const end = move.path.indexOf(leg.to, start + 1);
      const path = move.path.slice(start, end + 1);
      start = end;
      return { ...leg, points: leg.by === "river" ? riverRoutePoints(map, path) : path.map((id) => map.regions[id].site) };
    });
  }), [overview.moves, map]);
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
  const speechLands = useMemo(() => new Map((namesVariety?.knownLands ?? []).map((land) => [land.region, land.spelled])), [namesVariety]);
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
  const key = useMemo((): { title: string; entries: { label: string; colour: string }[]; note?: string } => {
    switch (tint.kind) {
      case "peoples": {
        const families = [...new Set(overview.communities.filter((people) => people.ended === null)
          .map((people) => overview.varieties[people.variety].family))];
        return { title: "Language families", entries: families.map((id) => ({
          label: `${overview.varieties.find((variety) => variety.id === id)?.name ?? "Unnamed"} family`,
          colour: hue(id),
        })) };
      }
      case "faiths":
        return { title: "Faiths", entries: [
          ...overview.religions.filter((religion) => religion.followers.length > 0).map((religion) => ({ label: religion.name, colour: hue(religion.id) })),
          { label: "Their own gods", colour: UNCHANGED },
        ] };
      case "crafts":
        return { title: overview.crafts.find((craft) => craft.id === tint.craft)?.name ?? tint.craft, entries: [
          { label: "Hold this craft", colour: CHANGED }, { label: "Without it", colour: UNCHANGED },
        ], note: "Squares mark its inventors." };
      case "temper":
        return { title: "Temper", entries: [
          { label: ETHOS_POLES[tint.axis][0], colour: leaning(-0.5) },
          { label: "Between", colour: leaning(0) },
          { label: ETHOS_POLES[tint.axis][1], colour: leaning(0.5) },
        ], note: "Deeper colour means a stronger leaning." };
      case "weather":
        return { title: "Weather", entries: [
          { label: "Drier / warmer", colour: "#b28a43" }, { label: "Wetter", colour: "#658174" },
          { label: "Colder", colour: "#667f96" }, { label: "Near usual", colour: "var(--paper)" },
        ] };
      case "change":
        return { title: "Sound change", entries: [
          { label: "Underwent it", colour: CHANGED }, { label: "Did not", colour: UNCHANGED },
        ], note: "The heavy line marks where it stopped." };
      case "words": {
        const roots = new Map<number, string>();
        for (const word of tint.words.words) if (!roots.has(word.group)) roots.set(word.group, word.spelled);
        return { title: `Words for “${tint.words.gloss}”`, entries: [...roots].map(([id, label]) => ({ label, colour: hue(id) })),
          note: "Words in one colour descend from one root." };
      }
    }
  }, [tint, overview.communities, overview.varieties, overview.religions, overview.crafts]);

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
  const inspectionFrame = useRef(0);
  const inspectionCallback = useRef(onInspect);
  inspectionCallback.current = onInspect;
  useEffect(() => () => cancelAnimationFrame(inspectionFrame.current), []);
  useLayoutEffect(() => {
    if (!landFocused || !svg.current) return;
    if (landCursor === null) {
      inspectionCallback.current?.(null, [0, 0]);
      return;
    }
    const point = cartography.point(map.regions[landCursor].site);
    const matrix = svg.current.getScreenCTM();
    const bounds = svg.current.getBoundingClientRect();
    const at: [number, number] = point && matrix
      ? [matrix.a * point[0] + matrix.e, matrix.d * point[1] + matrix.f]
      : [bounds.x + bounds.width / 2, bounds.y + bounds.height / 2];
    inspectionCallback.current?.({ kind: "land", region: landCursor }, at);
  }, [landCursor, landFocused, map, cartography]);
  const selectLand = (region: number) => {
    if (dragged()) return;
    if (landNavigation) setLandCursor(region);
    onLand(region);
  };
  const inspectEvents = (feature: MapFeature) => ({
    "data-map-feature": JSON.stringify(feature),
    onPointerEnter: (event: PointerEvent<SVGElement>) => {
      cancelAnimationFrame(inspectionFrame.current); inspectionFrame.current = 0;
      if (!drag.current?.moved && !moving.current) inspectionCallback.current?.(feature, [event.clientX, event.clientY]);
    },
    onPointerMove: (event: PointerEvent<SVGElement>) => {
      if (inspectionFrame.current || drag.current?.moved || moving.current) return;
      const at: [number, number] = [event.clientX, event.clientY];
      inspectionFrame.current = requestAnimationFrame(() => {
        inspectionFrame.current = 0;
        if (!drag.current?.moved && !moving.current) inspectionCallback.current?.(feature, at);
      });
    },
    onPointerLeave: (event: PointerEvent<SVGElement>) => {
      cancelAnimationFrame(inspectionFrame.current); inspectionFrame.current = 0;
      inspectionCallback.current?.(null, [event.clientX, event.clientY]);
    },
    onFocus: (event: { currentTarget: SVGElement }) => {
      const box = event.currentTarget.getBoundingClientRect();
      inspectionCallback.current?.(feature, [box.x + box.width / 2, box.y + box.height / 2]);
    },
    onBlur: (event: { currentTarget: SVGElement }) => {
      const box = event.currentTarget.getBoundingClientRect();
      inspectionCallback.current?.(null, [box.x + box.width / 2, box.y + box.height / 2]);
    },
  });


  const possible = new Set(settlement?.options.filter((o) => o.reason === null).map((o) => o.region));
  const remains = new Set(settlement?.plan?.remaining.lands);
  const arrives = new Set(settlement?.plan?.arriving.lands);
  const shape = (r: WorldMap["regions"][number]) => {
    const sea = r.terrain === "sea";
    const veiled = !sea && hidden(r.id);
    const colour = veiled ? null : colourOf(r.id);
    return (
      <g key={r.id} data-chart-region={r.id} onClick={sea ? undefined : () => selectLand(r.id)}
        {...inspectEvents({ kind: "land", region: r.id })}
        aria-label={sea ? "Sea" : veiled ? "Unknown land" : (nameOf(r.id) ?? `Unnamed ${TERRAIN_NAME[r.terrain].toLowerCase()}`)}>
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
  const cursorName = landCursor === null ? "" : `${landLabel(overview, map, landCursor)}, land ${landCursor + 1}`;

  return (
    <div ref={root} className={`mapview projection-${projection}${zoomable ? " zoomable" : ""}`}
      data-longitude={turning.current[0]} data-latitude={turning.current[1]}>
      <svg
        ref={svg}
        viewBox={box.join(" ")}
        role={landNavigation ? "application" : "group"}
        tabIndex={landNavigation ? 0 : undefined}
        data-map-feature={!landFocused || landCursor === null ? undefined : JSON.stringify({ kind: "land", region: landCursor })}
        onFocus={(event) => {
          if (!landNavigation || event.target !== event.currentTarget) return;
          setLandFocused(true);
          const region = landCursor ?? initialLand();
          if (region !== null) showLandCursor(region);
        }}
        onBlur={(event) => {
          if (event.target !== event.currentTarget) return;
          setLandFocused(false);
          inspectionCallback.current?.(null, [0, 0]);
        }}
        onKeyDown={(e) => {
          if (!landNavigation || e.target !== e.currentTarget) return;
          if (landNavigation) {
            if (!e.altKey && landCursor !== null && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
              e.preventDefault(); stepLandCursor(e.key); return;
            }
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              if (landCursor !== null) onLand(landCursor);
              else {
                const region = initialLand();
                if (region !== null) showLandCursor(region);
              }
              return;
            }
            if (e.key === "Escape") {
              e.preventDefault(); setLandCursor(null); onClearLand?.(); return;
            }
          }
          const [x, y, w, h] = view.current;
          if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
            e.preventDefault(); cancelAnimationFrame(glide.current); glide.current = 0;
            if (landCursor !== null) setLandCursor(null);
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
            } else if (e.key.toLowerCase() === "f") fitRegions(selectedRegions);
            else zoom(e.key === "-" ? 1 / 0.7 : 0.7, [x + w / 2, y + h / 2]);
          }
        }}
        aria-label={`${projection === "globe" ? "Globe" : "Chart"}${cursorName ? `: ${cursorName}` : onClearLand ? ": Choose a homeland" : ` of the world in year ${generation * YEARS}`}`}
        aria-description={landNavigation ? `Arrow keys read nearby lands, across water too. Enter ${onClearLand ? "selects a homeland" : "opens the land's card"}. Escape clears the land cursor. Alt and arrow keys pan; after Escape, arrows pan and Enter reads lands again.` : undefined}
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
              role={onRiver || onInspect ? "button" : undefined} tabIndex={!onClearLand && (onRiver || onInspect) ? 0 : undefined}
              aria-label={name?.spelled ?? "Unnamed river"}
              {...inspectEvents({ kind: "river", river: river.id, region: river.mouth })}
              onClick={() => dragged() || onRiver?.(river.id)}
              onKeyDown={(e) => { if (onRiver && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); onRiver(river.id); } }}>
              <path className="river-ink" {...bindPath((view) => view.line(points), "parent")} />
              <path className="river-hit" {...bindPath((view) => view.line(points))} />
            </g>;
          })}
        </g>
        <g className="chart-lakes">
          {lakePaths.map(({ lake, fills, shores }) => {
            if (lake.regions.every(hidden)) return null;
            const name = namedLake(lake);
            return <g key={lake.id} className="chart-lake" data-lake={lake.id}
              role={onLake || onInspect ? "button" : undefined} tabIndex={!onClearLand && (onLake || onInspect) ? 0 : undefined}
              aria-label={name?.spelled ?? "Unnamed lake"}
              {...inspectEvents({ kind: "lake", lake: lake.id, region: lake.regions.find((region) => !hidden(region))! })}
              onClick={() => dragged() || onLake?.(lake.id)}
              onKeyDown={(e) => { if (onLake && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); onLake(lake.id); } }}>
              <path className="lake-water" {...bindPath((view) => fills.map((fill) => view.area(fill)).join(""), "parent")} />
              <path className="lake-shore" {...bindPath((view) => shores.map((shore) => view.line(shore)).join(""))} pointerEvents="none" />
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
                data-label-kind="continent" data-label-priority={3} data-landmass={c.landmass}
                role={onContinent ? "button" : undefined}
                tabIndex={onContinent ? 0 : undefined}
                aria-label={`${continentLabel(overview, map, c.landmass)}, “${c.name.meaning}”`}
                {...inspectEvents({ kind: "continent", landmass: c.landmass, region: mass.anchor })}
                onClick={() => dragged() || onContinent?.(c.landmass)}
                onKeyDown={(e) => {
                  if (onContinent && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onContinent(c.landmass);
                  }
                }}
              >
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
                  data-land-name={nameOf(p.region) ?? undefined}
                  {...inspectEvents({ kind: "land", region: p.region })}
                  onClick={() => selectLand(p.region)}
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
              <text dy={-0.06 * label}
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
              {...inspectEvents({ kind: "lake", lake: lake.id, region: lake.regions.find((region) => !hidden(region))! })}
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
                {recordedRoutes[i].map((leg, index) => <path
                  key={index}
                  className={`route route-${leg.by}${mine ? " chosen" : ""}`}
                  data-route={i} data-leg={index}
                  {...bindPath((view) => view.line(leg.points))}
                  style={{
                    stroke: leg.by === "river" ? "var(--ink)" : hue(family(mover)),
                    strokeOpacity: mine ? 1 : 0.3 + 0.6 * Math.max(0, 1 - age / ROUTE_FADE),
                  }}
                >
                  <title>
                    {`${mover.name} ${m.kind === "split" ? "went out" : "moved"} to ${nameOf(m.to) ?? "new land"}, ${LEG_BY[leg.by]}, year ${m.generation * YEARS}`}
                  </title>
                </path>)}
                  <path className="route-terminal" {...bindPath((view) => view.endLine(recordedRoutes[i].at(-1)?.points ?? []))}
                    markerEnd={`url(#${routeHead})`} pointerEvents="none"
                    fill="none" stroke="transparent" strokeWidth={mine ? 3 : 1.5} vectorEffect="non-scaling-stroke" />
                </g>
              );
            })}
          </g>
        ) : null}
        {story ? <g className="story-routes" role="group" aria-label={story.label ?? "Recorded journey"}>
          {story.route.map((leg, i) => {
            const from = map.regions[leg.from], to = map.regions[leg.to];
            if (!from || !to) return null;
            const points = [from.site, to.site];
            return <g key={`${leg.from}:${leg.to}:${leg.generation}:${i}`}>
              <path className="story-route" {...bindPath((view) => view.line(points))}>
                <title>{`Year ${leg.generation * YEARS} · ${story.label ?? "Recorded journey"}`}</title>
              </path>
              <path className="route-terminal" {...bindPath((view) => view.endLine(points))}
                markerEnd={`url(#${routeHead})`} fill="none" stroke="transparent" vectorEffect="non-scaling-stroke" />
              <text ref={bindAnchor(to.site)} x={pointFor(to.site)[0]} y={pointFor(to.site)[1]}
                data-label-x={pointFor(to.site)[0]} data-label-y={pointFor(to.site)[1]} data-label-dy={0.4}
                className="story-route-date" data-label-kind="story" data-label-priority={1e13}>{leg.generation * YEARS}</text>
            </g>;
          })}
        </g> : null}
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
                tabIndex={onClearLand ? undefined : 0}
                aria-label={tint.kind === "words" ? `${localName}: ${text}` : localName}
                {...inspectEvents({ kind: "people", community: c.id, region: c.region })}
                onClick={() => dragged() || onPeople(c.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    onPeople(c.id);
                  }
                }}
              >
                <title>{localName}</title>
                <circle className="people-dot" cx={0} cy={0} r={0.06 * label}
                  style={{ fill: tint.kind === "words" && word ? hue(word.group) : hue(family(c)) }} />
                <text x={0} y={0}
                  data-label-kind="people" data-people={c.id}
                  data-label-priority={100 + c.size * (overview.states.some((state) => state.fell === null && state.rulers === c.id) ? 2 : 1)}
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
                role={onState || onInspect ? "button" : undefined}
                tabIndex={onState || onInspect ? 0 : undefined}
                aria-label={`${state.name}, capital city, ${Math.round(state.city).toLocaleString()} souls`}
                {...inspectEvents({ kind: "state", state: state.id, region: state.capital })}
                onClick={() => dragged() || onState?.(state.id)}
                onKeyDown={(e) => {
                  if (onState && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onState(state.id);
                  }
                }}
              >
                <path className="city-marker" d="M-.14,.04V-.08H-.08V-.16H.02V-.04H.08V-.11H.14V.04Z" />
                <text y={0.27}
                  data-label-kind="state" data-label-priority={1e12 + state.city}>{state.name}</text>
              </g>
            );
          })}
        </g>
        <g className="cities">
          {overview.cities.map((city) => {
            const state = overview.states.find((s) => s.id === city.state);
            if (city.since > generation || (state?.fell !== null && state?.fell !== undefined && state.fell <= generation) ||
              hidden(city.region) || realms.some(({ state }) => state.capital === city.region)) return null;
            const [x, y] = pointFor(map.regions[city.region].site);
            const offset = (hearts.get(city.region)?.length ?? 0) / 2 * LINE + 0.2;
            const radius = 0.055 + Math.cbrt(Math.max(0, city.size)) * 0.003;
            return <g key={city.id} className="city" ref={bindAnchor(map.regions[city.region].site)}
              data-region={city.region} data-label-x={x} data-label-y={y} data-label-dy={offset} data-label-scale
              transform={`translate(${x} ${y + offset * label}) scale(${label})`}
              role="button" tabIndex={0} aria-label={`${city.name.name}, city, ${Math.round(city.size).toLocaleString()} souls`}
              {...inspectEvents({ kind: "city", city: city.id, region: city.region })}
              onClick={() => dragged() || onCity?.(city.id)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") { event.preventDefault(); onCity?.(city.id); }
              }}>
              <circle className="city-hit" r={Math.max(0.18, radius + 0.06)} />
              <circle className="city-ring" r={radius} />
              <circle className="city-heart" r={0.025} />
            </g>;
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
                role={onReligion || onInspect ? "button" : undefined}
                tabIndex={onReligion || onInspect ? 0 : undefined}
                aria-label={`${religion.name}, founded here`}
                {...inspectEvents({ kind: "shrine", religion: religion.id, region: religion.land })}
                onClick={() => dragged() || onReligion?.(religion.id)}
                onKeyDown={(e) => {
                  if (onReligion && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onReligion(religion.id);
                  }
                }}
              >
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
                role={onReligion || onInspect ? "button" : undefined}
                tabIndex={onReligion || onInspect ? 0 : undefined}
                aria-label={`${shrine.name.name}, holy to ${religion.name}`}
                {...inspectEvents({ kind: "shrine", religion: religion.id, region: shrine.region })}
                onClick={() => dragged() || onReligion?.(religion.id)}
                onKeyDown={(e) => {
                  if (onReligion && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onReligion(religion.id);
                  }
                }}
              >
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
                role={onCraft || onInspect ? "button" : undefined}
                tabIndex={onCraft || onInspect ? 0 : undefined}
                aria-label={`${people.name}, inventor of ${tint.craft}`}
                {...inspectEvents({ kind: "people", community: people.id, region: people.region })}
                onClick={() => dragged() || onCraft?.(tint.craft)}
                onKeyDown={(e) => {
                  if (onCraft && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    onCraft(tint.craft);
                  }
                }}
              >
                <path d="M-.1,-.1H.1V.1H-.1Z" />
              </g>
            );
          }) : null}
        </g>
        {landNavigation && landFocused && landCursor !== null ? <path className="land-cursor" data-cursor-region={landCursor}
          {...bindPath((view) => view.region(map.regions[landCursor]))} pointerEvents="none" /> : null}
        </g>
        {projection === "globe" ? <path className="globe-limb" d={cartography.sphere} aria-hidden="true" /> : null}
        {projection === "chart" ? <g className="chart-neatline" aria-hidden="true" pointerEvents="none">
          <path className="chart-edge-paper" d={`M0,0H${width}M0,${height}H${width}`} />
          <path className="chart-edge-rule" d={`M0,0H${width}M0,${height}H${width}`} />
          <path className="chart-edge-ticks" d={Array.from({ length: 37 }, (_, i) => {
            const x = width * i / 36, tick = height * (i % 3 === 0 ? 0.012 : 0.006);
            return `M${x},0v${tick}M${x},${height}v${-tick}`;
          }).join("")} />
        </g> : null}
      </svg>
      {landNavigation && landFocused ? <span className="sr-only" role="status">{cursorName}</span> : null}
      {zoomable ? (
        <div className="map-zoom" role="group" data-map-furniture aria-label={projection === "globe" ? "Globe controls" : "Chart controls"}>
          {selectedRegions.length > 0 ? <button type="button" title="Fit the selected lands (F)" aria-label="Fit the selected lands" onClick={() => fitRegions(selectedRegions)}><LocateFixed size={16} /></button> : null}
          {/* drawCamera owns disabled state alongside the imperative camera.
              Stale React disabled props would suppress a re-enabled button. */}
          <button ref={zoomIn} type="button" title="Closer (+)" aria-label="Zoom in" onClick={() => zoom(0.7, [view.current[0] + view.current[2] / 2, view.current[1] + view.current[3] / 2])}>
            <Plus size={16} />
          </button>
          <button ref={zoomOut} type="button" title="Farther (−)" aria-label="Zoom out" onClick={() => zoom(1 / 0.7, [view.current[0] + view.current[2] / 2, view.current[1] + view.current[3] / 2])}>
            <Minus size={16} />
          </button>
          <button type="button" title={projection === "globe" ? "The whole globe (Home)" : "The whole chart (Home)"}
            aria-label={projection === "globe" ? "The whole globe" : "The whole chart"}
            onClick={() => projection === "globe" ? turnTo([0, 20], full) : glideTo(full)}>
            <Maximize size={16} />
          </button>
        </div>
      ) : null}
      {onProjection || viewpointControl ? <div className="map-projection-control" data-map-furniture>
        {onProjection ? <MapProjectionSwitch value={projection} onChange={onProjection} /> : null}
        {viewpointControl}
      </div> : null}
      {zoomable ? <aside className="chart-legend" data-map-furniture aria-label="Chart key">
        <h3>{key.title}</h3>
        <ul className="chart-legend-colours">
          {key.entries.map((entry, index) => <li key={index}>
            <span className="legend-wash" style={{ background: entry.colour }} aria-hidden="true" />
            <span>{entry.label}</span>
          </li>)}
        </ul>
        {key.note ? <p>{key.note}</p> : null}
        <ul className="chart-legend-overlays">
          {names ? <li><span className="legend-letter" aria-hidden="true">Aa</span> Land and water names</li> : null}
          {routes ? <li><span className="legend-road" aria-hidden="true" /> Roads · dashed at sea</li> : null}
          {contacts ? <li><span className="legend-dealing" aria-hidden="true" /> Rule and chosen dealings</li> : null}
          {states ? <li><span className="legend-border" aria-hidden="true" /> State borders</li> : null}
          {faiths ? <li><span className="legend-shrine" aria-hidden="true">◇</span> Holy places and pilgrims' roads</li> : null}
          {known ? <li><span className="legend-veil" aria-hidden="true" /> Unknown lands veiled</li> : null}
          {story ? <li><span className="legend-story" aria-hidden="true" /> Recorded journey · years</li> : null}
          {reach ? <li><span className="legend-road" aria-hidden="true" /> Reach on foot or by sea</li> : null}
          {settlement ? <li><span className="legend-border" aria-hidden="true" /> Proposed settlement</li> : null}
        </ul>
        <p className="chart-legend-zoom">Closer views add the smaller lettering and marks.</p>
      </aside> : null}
      {zoomable ? <div className="chart-scale" data-map-furniture>
        <span ref={scaleRule} className="scale-rule"><span ref={scaleText} /></span>
        <span className="chart-help">{landFocused ? "Arrows read lands · Alt + arrows pan" : projection === "globe" ? "Drag to turn · scroll to zoom" : "Scroll to zoom · drag to explore"}</span>
      </div> : null}
      {ROSE}
    </div>
  );
}

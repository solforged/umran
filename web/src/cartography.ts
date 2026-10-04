import {
  geoDistance,
  geoEquirectangular,
  geoGraticule10,
  geoInterpolate,
  geoOrthographic,
  geoPath,
  geoStream,
} from "d3-geo";
import type { LineString, Polygon } from "geojson";
import type { Lake, Region, River, WorldMap } from "./model";

export type MapProjection = "chart" | "globe";
/** Geographic longitude and latitude at the centre of the globe, in degrees. */
export type GlobeRotation = [number, number];
/** A point in the world's original equirectangular drawing coordinates. */
export type MapPoint = [number, number];

export interface Cartography {
  readonly width: number;
  readonly height: number;
  /** Update the globe in place; source geometry and projected cache entries survive. */
  rotate(rotation: GlobeRotation): void;
  point(site: MapPoint): MapPoint | null;
  region(region: Region): string;
  /** Point arrays and their coordinates are immutable cache keys. */
  line(points: readonly MapPoint[]): string;
  /** Smooth source-chart outlines, clipped by the same spherical projection as lands. */
  area(points: readonly MapPoint[]): string;
  /** Longest uninterrupted visible line segment, oriented left to right. */
  labelLine(points: readonly MapPoint[]): string;
  /** Final visible segment in source order; empty if the destination is hidden. */
  endLine(points: readonly MapPoint[]): string;
  readonly graticule: string;
  readonly sphere: string;
}

type Vector = [number, number, number];
const RADIANS = Math.PI / 180;
const SAME_VERTEX_SQUARED = 1e-18;
const SPHERE = { type: "Sphere" } as const;
const GRATICULE = geoGraticule10();
const planarPath = geoPath().digits(6);

// Geometry belongs to an immutable world and survives projection/rotation
// changes. Weak keys release it when that world or a transient route unloads.
interface MapGeometry {
  regions: WeakMap<Region, Polygon>;
  vectors: WeakMap<Region, Vector[]>;
  lines: WeakMap<readonly MapPoint[], LineString>;
  areas: WeakMap<readonly MapPoint[], Polygon>;
  points: WeakMap<MapPoint, { location: MapPoint; normal: Vector }>;
  borders: Map<number, Map<number, MapPoint[]>>;
  rivers: WeakMap<River, MapPoint[]>;
}
const mapGeometry = new WeakMap<WorldMap, MapGeometry>();

function geometryFor(map: WorldMap): MapGeometry {
  let geometry = mapGeometry.get(map);
  if (!geometry) {
    geometry = {
      regions: new WeakMap(),
      vectors: new WeakMap(),
      lines: new WeakMap(),
      areas: new WeakMap(),
      points: new WeakMap(),
      borders: new Map(),
      rivers: new WeakMap(),
    };
    mapGeometry.set(map, geometry);
  }
  return geometry;
}

export function chartPoint(map: WorldMap, geographic: MapPoint): MapPoint {
  return [
    (geographic[0] + 180) / 360 * map.width,
    (90 - geographic[1]) / 180 * map.height,
  ];
}

export function geographicPoint(map: WorldMap, site: MapPoint): MapPoint {
  return [site[0] / map.width * 360 - 180, 90 - site[1] / map.height * 180];
}

function vector(point: MapPoint): Vector {
  const longitude = point[0] * RADIANS, latitude = point[1] * RADIANS;
  const radius = Math.cos(latitude);
  return [radius * Math.cos(longitude), radius * Math.sin(longitude), Math.sin(latitude)];
}

function geographic(vector: Vector): MapPoint {
  return [
    Math.atan2(vector[1], vector[0]) / RADIANS,
    Math.atan2(vector[2], Math.hypot(vector[0], vector[1])) / RADIANS,
  ];
}

function dot(a: Vector, b: Vector): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

function sameVertex(a: Vector, b: Vector): boolean {
  return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2 < SAME_VERTEX_SQUARED;
}

function vectorsFor(map: WorldMap, region: Region): Vector[] {
  const cache = geometryFor(map).vectors;
  let vectors = cache.get(region);
  if (!vectors) {
    vectors = region.boundary.map(vector);
    cache.set(region, vectors);
  }
  return vectors;
}

function polygonGeometry(points: readonly MapPoint[]): Polygon {
  const ring = points.slice();
  if (ring.length) ring.push(ring[0]);
  return { type: "Polygon", coordinates: [ring] };
}

function lineGeometry(map: WorldMap, points: readonly MapPoint[]): LineString {
  const cache = geometryFor(map).lines;
  let line = cache.get(points);
  if (line) return line;
  const coordinates: MapPoint[] = [];
  let previous: Vector | undefined;
  for (const point of points) {
    const location = geographicPoint(map, point), next = vector(location);
    if (previous && dot(previous, next) < 0
      && (previous[0] + next[0]) ** 2 + (previous[1] + next[1]) ** 2 + (previous[2] + next[2]) ** 2 < 1e-24) {
      // Exactly antipodal endpoints have no unique great circle. Choose the
      // northward arc (the Greenwich arc at the poles), independently of view
      // rotation or route direction, before d3 clips the resulting great circle.
      const axis: Vector = Math.abs(previous[2]) < 1 - 1e-12 ? [0, 0, 1] : [1, 0, 0];
      const along = dot(previous, axis);
      coordinates.push(geographic([
        axis[0] - along * previous[0],
        axis[1] - along * previous[1],
        axis[2] - along * previous[2],
      ]));
    }
    coordinates.push(location);
    previous = next;
  }
  line = { type: "LineString", coordinates };
  cache.set(points, line);
  return line;
}

/** A lake's inferred shore is one rounded hull, not separate inset land tiles.
 * Unwrap around its first land before taking the hull so seam lakes stay local.
 * Sample a closed cubic B-spline once in the source plane; both projections
 * then draw and horizon-clip the very same smooth geographic shoreline. */
export function lakeOutline(map: WorldMap, lake: Lake): { shore: MapPoint[]; ripple: MapPoint[]; center: MapPoint } {
  const origin = map.regions[lake.regions[0]].site[0];
  const vertices = lake.regions.flatMap((id) => {
    const region = map.regions[id];
    return region.boundary.map((point): MapPoint => {
      const [x, y] = chartPoint(map, geoInterpolate(region.center, point)(0.75) as MapPoint);
      return [origin + ((x - origin + map.width * 1.5) % map.width) - map.width / 2, y];
    });
  }).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  const cross = (a: MapPoint, b: MapPoint, c: MapPoint) =>
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
  const half = (points: MapPoint[]) => {
    const out: MapPoint[] = [];
    for (const point of points) {
      while (out.length > 1 && cross(out[out.length - 2], out[out.length - 1], point) <= 0) out.pop();
      out.push(point);
    }
    out.pop();
    return out;
  };
  const hull = [...half(vertices), ...half([...vertices].reverse())];
  const shore: MapPoint[] = [];
  for (let i = 0; i < hull.length; i++) {
    const a = hull[(i + hull.length - 1) % hull.length], b = hull[i];
    const c = hull[(i + 1) % hull.length], d = hull[(i + 2) % hull.length];
    for (let step = 0; step < 8; step++) {
      const t = step / 8, t2 = t * t, t3 = t2 * t;
      const weights = [(1 - 3 * t + 3 * t2 - t3) / 6, (4 - 6 * t2 + 3 * t3) / 6,
        (1 + 3 * t + 3 * t2 - 3 * t3) / 6, t3 / 6];
      shore.push([a[0] * weights[0] + b[0] * weights[1] + c[0] * weights[2] + d[0] * weights[3],
        a[1] * weights[0] + b[1] * weights[1] + c[1] * weights[2] + d[1] * weights[3]]);
    }
  }
  let area = 0, x = 0, y = 0;
  for (let i = 0; i < shore.length; i++) {
    const a = shore[i], b = shore[(i + 1) % shore.length], weight = a[0] * b[1] - b[0] * a[1];
    area += weight; x += (a[0] + b[0]) * weight; y += (a[1] + b[1]) * weight;
  }
  const center: MapPoint = area ? [x / (3 * area), y / (3 * area)] : map.regions[lake.regions[0]].site;
  const ripple = shore.map(([x, y]): MapPoint => [center[0] + (x - center[0]) * 0.84, center[1] + (y - center[1]) * 0.84]);
  ripple.push(ripple[0]);
  return { shore, ripple, center };
}

/** Map geometry and supplied point arrays must not be mutated after use. */
export function createCartography(
  map: WorldMap,
  mode: MapProjection = "chart",
  rotation: GlobeRotation = [0, 20],
): Cartography {
  const globe = mode === "globe";
  const width = globe ? map.height : map.width, height = map.height;
  const geometry = geometryFor(map);
  const projection = globe
    ? geoOrthographic().scale(height / 2).translate([width / 2, height / 2])
      .rotate([-rotation[0], -rotation[1]]).clipAngle(90)
    : geoEquirectangular().scale(map.width / (2 * Math.PI)).translate([width / 2, height / 2])
      .clipExtent([[0, 0], [width, height]]);
  projection.precision(height / 2000);
  const path = geoPath(projection).digits(6);
  let viewCenter = vector(rotation);
  let revision = 0;
  const paths = new WeakMap<Polygon | LineString, { revision: number; d: string }>();
  const continuousPaths = new WeakMap<LineString, { revision: number; label: string; end: string }>();
  const points = new WeakMap<MapPoint, { revision: number; at: MapPoint | null }>();
  const project = (geometry: Polygon | LineString): string => {
    let result = paths.get(geometry);
    if (!result) {
      result = { revision: -1, d: "" };
      paths.set(geometry, result);
    }
    if (result.revision !== revision) {
      result.d = path(geometry) ?? "";
      result.revision = revision;
    }
    return result.d;
  };
  const continuous = (line: LineString): { label: string; end: string } => {
    let cached = continuousPaths.get(line);
    if (cached?.revision === revision) return cached;
    let longest: MapPoint[] = [], last: MapPoint[] = [], segment: MapPoint[] = [];
    let longestLength = 0, length = 0;
    geoStream(line, projection.stream({
      lineStart() { segment = []; length = 0; },
      point(x, y) {
        const previous = segment.at(-1);
        if (previous) length += Math.hypot(x - previous[0], y - previous[1]);
        segment.push([x, y]);
      },
      lineEnd() {
        last = length > 0 ? segment : [];
        if (length > longestLength) { longest = segment; longestLength = length; }
      },
      polygonStart() {},
      polygonEnd() {},
      sphere() {},
    }));
    const [longitude, latitude] = line.coordinates.at(-1)!;
    // A clipped limb is not the destination. Serialize before possibly
    // reversing the same segment for a readable label.
    const destinationVisible = !globe || dot(viewCenter, vector([longitude, latitude])) >= -1e-12;
    const end = destinationVisible && last.length > 1
      ? planarPath({ type: "LineString", coordinates: last }) ?? "" : "";
    if (longest.length && longest[0][0] > longest.at(-1)![0]) longest.reverse();
    const label = longestLength > 0 ? planarPath({ type: "LineString", coordinates: longest }) ?? "" : "";
    if (!cached) {
      cached = { revision, label, end };
      continuousPaths.set(line, cached);
    } else {
      cached.revision = revision;
      cached.label = label;
      cached.end = end;
    }
    return cached;
  };
  let graticuleRevision = -1, graticule = "";
  // The chart boundary and orthographic limb do not move when the globe turns.
  const sphere = path(SPHERE) ?? "";

  return {
    width,
    height,
    rotate(next) {
      if (!globe || (next[0] === rotation[0] && next[1] === rotation[1])) return;
      rotation = next;
      viewCenter = vector(next);
      projection.rotate([-next[0], -next[1]]);
      revision++;
    },
    point(site) {
      let projected = points.get(site);
      if (!projected) {
        projected = { revision: -1, at: null };
        points.set(site, projected);
      }
      if (projected.revision !== revision) {
        let source = geometry.points.get(site);
        if (!source) {
          const location = geographicPoint(map, site);
          source = { location, normal: vector(location) };
          geometry.points.set(site, source);
        }
        // Calling a d3 projection directly does not run its spherical clipper.
        projected.at = globe && dot(viewCenter, source.normal) < -1e-12 ? null : projection(source.location);
        projected.revision = revision;
      }
      return projected.at;
    },
    region(region) {
      let polygon = geometry.regions.get(region);
      if (!polygon) {
        // Canonical clockwise winding also encloses small seam/pole cells;
        // chart-coordinate sorting would reverse or tear their interiors.
        polygon = polygonGeometry(region.boundary);
        geometry.regions.set(region, polygon);
      }
      return project(polygon);
    },
    line(points) {
      return points.length < 2 ? "" : project(lineGeometry(map, points));
    },
    area(points) {
      if (points.length < 3) return "";
      let polygon = geometry.areas.get(points);
      if (!polygon) {
        polygon = polygonGeometry(points.map((point) => geographicPoint(map, point)));
        geometry.areas.set(points, polygon);
      }
      return project(polygon);
    },
    labelLine(points) {
      return points.length < 2 ? "" : continuous(lineGeometry(map, points)).label;
    },
    endLine(points) {
      return points.length < 2 ? "" : continuous(lineGeometry(map, points)).end;
    },
    get graticule() {
      if (graticuleRevision !== revision) {
        graticule = path(GRATICULE) ?? "";
        graticuleRevision = revision;
      }
      return graticule;
    },
    sphere,
  };
}

/** Cached source-chart edge; callers must treat its points as immutable. */
export function sharedBorder(map: WorldMap, a: number, b: number): MapPoint[] {
  const first = Math.min(a, b), second = Math.max(a, b);
  const left = map.regions[first], right = map.regions[second];
  if (!left || !right || first === second || !left.neighbours.includes(second)) return [];
  const borders = geometryFor(map).borders;
  const cached = borders.get(first)?.get(second);
  if (cached) return cached;
  const l = vectorsFor(map, left), r = vectorsFor(map, right);
  const matches = l.map((point) => r.findIndex((other) => sameVertex(point, other)));
  const shared = matches.map((match, i) => {
    const next = matches[(i + 1) % l.length];
    return match !== -1 && next !== -1 &&
      (next === (match + 1) % r.length || next === (match + r.length - 1) % r.length);
  });
  let start = -1, count = 0;
  // A refined edge can cross the canonical ring's first vertex. Start at its
  // corner, not at the first matching vertex, and retain every shore bend.
  for (let i = 0; i < shared.length; i++) {
    if (!shared[i] || shared[(i + shared.length - 1) % shared.length]) continue;
    let length = 1;
    while (length < shared.length && shared[(i + length) % shared.length]) length++;
    if (length > count) { start = i; count = length; }
  }
  if (start === -1) throw new Error(`Neighbouring regions ${first} and ${second} have no shared spherical edge`);
  const edge = Array.from({ length: count + 1 }, (_, i) => chartPoint(map, left.boundary[(start + i) % l.length]));
  let neighbours = borders.get(first);
  if (!neighbours) { neighbours = new Map(); borders.set(first, neighbours); }
  neighbours.set(second, edge);
  return edge;
}

/** Cached source-chart course, ending at the actual confluence or coastline. */
export function riverPoints(map: WorldMap, river: River): MapPoint[] {
  const cache = geometryFor(map).rivers;
  const cached = cache.get(river);
  if (cached) return cached;
  const points = river.course.map((id) => chartPoint(map, map.regions[id].center));
  if (river.course.length) {
    if (river.joinAt !== null) {
      points.push(chartPoint(map, map.regions[river.joinAt].center));
    } else {
      const edge = sharedBorder(map, river.course.at(-1)!, river.mouth);
      if (edge.length < 2) throw new Error(`River ${river.id} does not end at an adjacent coastal region`);
      const shore = edge.map((point) => geographicPoint(map, point));
      const lengths = shore.slice(1).map((point, i) => geoDistance(shore[i], point));
      let remaining = lengths.reduce((length, segment) => length + segment, 0) / 2;
      for (let i = 0; i < lengths.length; i++) {
        if (remaining <= lengths[i]) {
          const point = geoInterpolate(shore[i], shore[i + 1])(lengths[i] > 0 ? remaining / lengths[i] : 0);
          points.push(chartPoint(map, point));
          break;
        }
        remaining -= lengths[i];
      }
    }
  }
  cache.set(river, points);
  return points;
}

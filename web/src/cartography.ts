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

interface SphericalBounds {
  vertices: Vector[];
  center: Vector;
  /** A cap smaller than a hemisphere contains the cell's great-circle edges. */
  limb: number;
  /** Edges whose maximum orthographic sagitta is below d3's precision. */
  direct: boolean;
}

interface RegionGeometry extends SphericalBounds {
  polygon: Polygon;
}

// Geometry belongs to an immutable world and survives projection/rotation
// changes. Weak keys release it when that world or a transient route unloads.
interface MapGeometry {
  regions: WeakMap<Region, RegionGeometry>;
  vectors: WeakMap<Region, Vector[]>;
  lines: WeakMap<readonly MapPoint[], LineString>;
  areas: WeakMap<readonly MapPoint[], Polygon>;
  lineBounds: WeakMap<LineString, SphericalBounds>;
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
      lineBounds: new WeakMap(),
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

function sphericalBounds(vertices: Vector[], center: Vector, closed: boolean): SphericalBounds {
  let radiusCos = 1, direct = vertices.length >= (closed ? 3 : 2);
  for (let i = 0; i < vertices.length; i++) {
    radiusCos = Math.min(radiusCos, dot(center, vertices[i]));
    // precision / globe radius = (height / 2000) / (height / 2).
    // 1 - cos(edge / 2) <= .001 means no adaptive midpoint is needed.
    if ((closed || i + 1 < vertices.length) && dot(vertices[i], vertices[(i + 1) % vertices.length]) < 0.996002) direct = false;
  }
  return {
    vertices, center,
    limb: radiusCos > 0 ? Math.sqrt(Math.max(0, 1 - radiusCos * radiusCos)) + 1e-12 : Infinity,
    direct,
  };
}

function regionGeometry(map: WorldMap, region: Region): RegionGeometry {
  const cache = geometryFor(map).regions;
  let geometry = cache.get(region);
  if (geometry) return geometry;
  geometry = {
    polygon: polygonGeometry(region.boundary),
    ...sphericalBounds(vectorsFor(map, region), vector(region.center), true),
  };
  cache.set(region, geometry);
  return geometry;
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

/** Follow the water's cells, including bays and islands, rather than their hull.
 * Only exposed edges retreat into their own cell. Shared edges stay identical
 * and disappear under the fill. All sampling is spherical: longitude is not a
 * distance near a pole, and a chart-space spline can flatten a polar shore.
 * The small ID-keyed variation is ink geometry, not engine randomness. */
export function lakeOutline(map: WorldMap, lake: Lake): { fills: MapPoint[][]; shores: MapPoint[][]; center: MapPoint } {
  const members = new Set(lake.regions);
  const fills: MapPoint[][] = [], shores: MapPoint[][] = [];
  const variation = (region: number, edge: number, step: number) => {
    let hash = Math.imul(lake.id + 1, 0x45d9f3b) ^ Math.imul(region + 1, 0x27d4eb2d)
      ^ Math.imul(edge + 1, 0x165667b1) ^ Math.imul(step + 1, 0x85ebca6b);
    hash = Math.imul(hash ^ (hash >>> 16), 0x45d9f3b);
    return ((hash ^ (hash >>> 16)) >>> 0) / 4294967296;
  };
  for (const id of lake.regions) {
    const region = map.regions[id], vertices = vectorsFor(map, region);
    const neighbours = region.neighbours.filter((other) => members.has(other))
      .map((other) => vectorsFor(map, map.regions[other]));
    const shared = vertices.map((vertex, edge) => neighbours.some((boundary) => boundary.some((point, i) =>
      sameVertex(vertex, point) && (
        sameVertex(vertices[(edge + 1) % vertices.length], boundary[(i + 1) % boundary.length])
        || sameVertex(vertices[(edge + 1) % vertices.length], boundary[(i + boundary.length - 1) % boundary.length])
      ))));
    const inset = vertices.map((_, edge) => shared[edge] || shared[(edge + vertices.length - 1) % vertices.length]
      ? 0 : 0.14 + variation(id, edge, 0) * 0.14);
    const fill: MapPoint[] = [];
    for (let edge = 0; edge < vertices.length; edge++) {
      const next = (edge + 1) % vertices.length;
      const a = region.boundary[edge], b = region.boundary[next];
      if (shared[edge]) {
        fill.push(chartPoint(map, a));
        continue;
      }
      const along = geoInterpolate(a, b), shore: MapPoint[] = [];
      // Limit the retreat by edge length as well as distance to the cell
      // centre, so finely refined boundaries acquire coves, not long teeth.
      const edgeLength = geoDistance(a, b);
      for (let step = 0; step <= 6; step++) {
        const t = step / 6, point = along(inset[edge] + t * (1 - inset[edge] - inset[next])) as MapPoint;
        const inward = geoDistance(point, region.center);
        const depth = Math.min(0.065, edgeLength / Math.max(inward, 1e-12) * 0.1)
          * Math.sin(Math.PI * t) * (0.45 + 0.55 * variation(id, edge, step));
        shore.push(chartPoint(map, geoInterpolate(point, region.center)(depth) as MapPoint));
      }
      if (inset[next]) {
        const start = along(1 - inset[next]);
        const end = geoInterpolate(b, region.boundary[(next + 1) % vertices.length])(inset[next]);
        for (let step = 1; step <= 4; step++) {
          const t = step / 4;
          shore.push(chartPoint(map, geoInterpolate(geoInterpolate(start, b)(t), geoInterpolate(b, end)(t))(t) as MapPoint));
        }
      }
      fill.push(...shore.slice(0, -1));
      shores.push(shore);
    }
    fills.push(fill);
  }
  // Keep the lettering anchored in an actual member cell, even for a
  // crescent-shaped lake whose arithmetic centroid would lie on dry land.
  const center = map.regions[lake.regions[0]].site;
  return { fills, shores, center };
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
  let east: Vector = [-Math.sin(rotation[0] * RADIANS), Math.cos(rotation[0] * RADIANS), 0];
  let north: Vector = [
    -Math.sin(rotation[1] * RADIANS) * Math.cos(rotation[0] * RADIANS),
    -Math.sin(rotation[1] * RADIANS) * Math.sin(rotation[0] * RADIANS),
    Math.cos(rotation[1] * RADIANS),
  ];
  let revision = 0;
  const paths = new WeakMap<Polygon | LineString, { revision: number; d: string }>();
  const continuousPaths = new WeakMap<LineString, { revision: number; label: string; end: string }>();
  const points = new WeakMap<MapPoint, { revision: number; at: MapPoint | null }>();
  const project = (geometry: Polygon | LineString, bounds?: SphericalBounds): string => {
    let result = paths.get(geometry);
    if (!result) {
      result = { revision: -1, d: "" };
      paths.set(geometry, result);
    }
    if (result.revision !== revision) {
      const facing = globe && bounds ? dot(viewCenter, bounds.center) : 0;
      if (globe && bounds && facing < -bounds.limb) {
        result.d = "";
      } else if (globe && bounds?.direct && facing > bounds.limb) {
        // Match geoPath's six-digit serialization without the spherical
        // clipper or trigonometry at every vertex. Larger and limb-crossing
        // edges retain d3's exact clipping and adaptive great-circle sampling.
        let d = "";
        for (const vertex of bounds.vertices) {
          const x = Math.round((width / 2 + height / 2 * dot(east, vertex)) * 1e6) / 1e6;
          const y = Math.round((height / 2 - height / 2 * dot(north, vertex)) * 1e6) / 1e6;
          d += `${d ? "L" : "M"}${x},${y}`;
        }
        result.d = d && geometry.type === "Polygon" ? `${d}Z` : d;
      } else {
        result.d = path(geometry) ?? "";
      }
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
      const longitude = next[0] * RADIANS, latitude = next[1] * RADIANS;
      const sinLongitude = Math.sin(longitude), cosLongitude = Math.cos(longitude);
      const sinLatitude = Math.sin(latitude), cosLatitude = Math.cos(latitude);
      east = [-sinLongitude, cosLongitude, 0];
      north = [-sinLatitude * cosLongitude, -sinLatitude * sinLongitude, cosLatitude];
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
        projected.at = !globe ? projection(source.location) : dot(viewCenter, source.normal) < -1e-12 ? null : [
          width / 2 + height / 2 * dot(east, source.normal),
          height / 2 - height / 2 * dot(north, source.normal),
        ];
        projected.revision = revision;
      }
      return projected.at;
    },
    region(region) {
      const source = regionGeometry(map, region);
      return project(source.polygon, source);
    },
    line(points) {
      if (points.length < 2) return "";
      const line = lineGeometry(map, points);
      if (!globe) return project(line);
      let bounds = geometry.lineBounds.get(line);
      if (!bounds) {
        const vertices = line.coordinates.map((point) => vector(point as MapPoint));
        bounds = sphericalBounds(vertices, vertices[0], false);
        geometry.lineBounds.set(line, bounds);
      }
      return project(line, bounds);
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

/** Samples per course leg when a river is smoothed through its cell centres. */
const RIVER_SMOOTHING = 4;

/// A centripetal Catmull-Rom curve through the course's unit vectors, so a
/// river bends through each land it crosses instead of turning at its centre.
/// Done on the sphere, it needs no seam or pole cases; the source and the
/// mouth stay exactly where the engine put them.
function smoothCourse(map: WorldMap, points: MapPoint[]): MapPoint[] {
  if (points.length < 3) return points;
  const knots = points.map((point) => vector(geographicPoint(map, point)));
  const out: MapPoint[] = [points[0]];
  const at = (i: number) => knots[Math.min(Math.max(i, 0), knots.length - 1)];
  const chord = (a: Vector, b: Vector) => Math.sqrt(Math.sqrt((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2));
  for (let i = 0; i + 1 < knots.length; i++) {
    const p0 = at(i - 1), p1 = at(i), p2 = at(i + 1), p3 = at(i + 2);
    const t0 = 0, t1 = t0 + Math.max(chord(p0, p1), 1e-9), t2 = t1 + Math.max(chord(p1, p2), 1e-9), t3 = t2 + Math.max(chord(p2, p3), 1e-9);
    for (let k = 1; k <= RIVER_SMOOTHING; k++) {
      if (k === RIVER_SMOOTHING) { out.push(points[i + 1]); break; }
      const t = t1 + (t2 - t1) * k / RIVER_SMOOTHING;
      const mix = (a: Vector, b: Vector, ta: number, tb: number): Vector => {
        const wb = (t - ta) / (tb - ta), wa = 1 - wb;
        return [a[0] * wa + b[0] * wb, a[1] * wa + b[1] * wb, a[2] * wa + b[2] * wb];
      };
      const a1 = mix(p0, p1, t0, t1), a2 = mix(p1, p2, t1, t2), a3 = mix(p2, p3, t2, t3);
      const b1 = mix(a1, a2, t0, t2), b2 = mix(a2, a3, t1, t3);
      const c = mix(b1, b2, t1, t2);
      const length = Math.hypot(c[0], c[1], c[2]);
      out.push(chartPoint(map, geographic([c[0] / length, c[1] / length, c[2] / length])));
    }
  }
  return out;
}

/** Cached source-chart course, smoothed through the lands it crosses. V4 and
 * later rivers follow the engine's channel through shared borders, ending at
 * their confluence, lake, or coast; legacy courses run centre to centre and
 * are extended to the coast here. */
export function riverPoints(map: WorldMap, river: River): MapPoint[] {
  const cache = geometryFor(map).rivers;
  const cached = cache.get(river);
  if (cached) return cached;
  const points = river.channel.length >= 2
    ? river.channel.map((point) => chartPoint(map, geographic(point)))
    : river.course.map((id) => chartPoint(map, map.regions[id].center));
  if (river.channel.length < 2 && river.course.length) {
    if (river.joinAt !== null) {
      points.push(chartPoint(map, map.regions[river.joinAt].center));
    } else if (river.course.at(-1) !== river.mouth) {
      const edge = sharedBorder(map, river.course.at(-1)!, river.mouth);
      if (edge.length < 2) { cache.set(river, points); return points; }
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
  const course = smoothCourse(map, points);
  cache.set(river, course);
  return course;
}

/** Follow the same river ink as the chart, in either direction. A recorded
 * river leg can cross a confluence; join its channel sections in path order. */
export function riverRoutePoints(map: WorldMap, path: readonly number[]): MapPoint[] {
  if (!path.length) return [];
  const points: MapPoint[] = [map.regions[path[0]].site];
  for (let edge = 1; edge < path.length; edge++) {
    const from = path[edge - 1], to = path[edge];
    const river = map.rivers.find((river) => {
      const a = river.course.indexOf(from), b = river.course.indexOf(to);
      return a >= 0 && b >= 0 && Math.abs(a - b) === 1 ||
        (river.course.at(-1) === from && (river.joinAt ?? river.mouth) === to) ||
        (river.course.at(-1) === to && (river.joinAt ?? river.mouth) === from);
    });
    if (!river) {
      // Old maps may record a navigable valley without a drawn channel.
      points.push(map.regions[to].site);
      continue;
    }
    const channel = riverPoints(map, river);
    let a = 0, b = 0, fromDistance = Infinity, toDistance = Infinity;
    for (let i = 0; i < channel.length; i++) {
      const point = geographicPoint(map, channel[i]);
      const da = geoDistance(map.regions[from].center, point);
      const db = geoDistance(map.regions[to].center, point);
      if (da < fromDistance) { fromDistance = da; a = i; }
      if (db < toDistance) { toDistance = db; b = i; }
    }
    const direction = a <= b ? 1 : -1;
    for (let i = a; ; i += direction) {
      points.push(channel[i]);
      if (i === b) break;
    }
  }
  points.push(map.regions[path.at(-1)!].site);
  return points;
}

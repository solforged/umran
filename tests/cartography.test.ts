import { describe, expect, test } from "bun:test";
import { geoCircle, geoOrthographic, geoPath } from "d3-geo";
import {
  chartPoint,
  createCartography,
  geographicPoint,
  lakeOutline,
  riverPoints,
  sharedBorder,
  type MapPoint,
} from "../web/src/cartography";
import type { Region, River, WorldMap } from "../web/src/model";

function world(cells: { center: MapPoint; boundary?: MapPoint[]; neighbours?: number[]; terrain?: Region["terrain"] }[] = []): WorldMap {
  const map: WorldMap = {
    size: "small", radiusKm: 18000 / Math.PI, width: 360, height: 180, kmPerUnit: 100,
    geography: "continental-v4",
    regions: [], landmasses: [], rivers: [], lakes: [], climateZones: [],
  };
  map.regions = cells.map(({ center, boundary = [], neighbours = [], terrain = "plains" }, id) => ({
    id, center, boundary, neighbours, terrain,
    site: chartPoint(map, center),
    areaKm2: 1, elevation: 0, moisture: 0, warmth: 0, climateZone: null,
    coastal: false, island: false, landmass: null,
  }));
  return map;
}

// Inspect rendered geometry, not SVG serialization or d3's sampling density.
function segments(path: string): MapPoint[][] {
  expect(path).not.toMatch(/NaN|Infinity/);
  const lines: MapPoint[][] = [];
  for (const command of path.match(/[MLZ][^MLZ]*/gi) ?? []) {
    if (command[0].toUpperCase() === "Z") continue;
    if (command[0].toUpperCase() === "M") lines.push([]);
    const coordinates = command.slice(1).trim().split(/[,\s]+/).map(Number);
    for (let i = 0; i < coordinates.length; i += 2) {
      expect(Number.isFinite(coordinates[i]) && Number.isFinite(coordinates[i + 1])).toBe(true);
      lines.at(-1)!.push([coordinates[i], coordinates[i + 1]]);
    }
  }
  return lines;
}

function bounds(lines: MapPoint[][]): [number, number, number, number] {
  const points = lines.flat();
  return [
    Math.min(...points.map(([x]) => x)), Math.min(...points.map(([, y]) => y)),
    Math.max(...points.map(([x]) => x)), Math.max(...points.map(([, y]) => y)),
  ];
}

function contains(rings: MapPoint[][], [x, y]: MapPoint): boolean {
  let inside = false;
  for (const ring of rings) {
    for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
      const [xi, yi] = ring[i], [xj, yj] = ring[j];
      if ((yi > y) !== (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi) inside = !inside;
    }
  }
  return inside;
}

function pathLength(lines: MapPoint[][]): number {
  let length = 0;
  for (const line of lines) {
    for (let i = 1; i < line.length; i++) {
      length += Math.hypot(line[i][0] - line[i - 1][0], line[i][1] - line[i - 1][1]);
    }
  }
  return length;
}

describe("spherical chart geometry", () => {
  test("a seam cell fills both chart edges without covering the intervening world", () => {
    const map = world([{
      center: [180, 0], boundary: [[170, -10], [170, 10], [-170, 10], [-170, -10]],
    }]);
    const rings = segments(createCartography(map, "chart").region(map.regions[0]));
    expect(rings).toHaveLength(2);
    expect(contains(rings, [355, 90])).toBe(true);
    expect(contains(rings, [5, 90])).toBe(true);
    expect(contains(rings, [180, 90])).toBe(false);
    for (const ring of rings) expect(bounds([ring])[2] - bounds([ring])[0]).toBeLessThan(11);
  });

  test.each([1, -1])("a canonical pole cell encloses only its own pole (hemisphere %i)", (hemisphere) => {
    const boundary: MapPoint[] = [[-120, 80], [120, 80], [0, 80]];
    if (hemisphere < 0) {
      boundary.reverse();
      for (const point of boundary) point[1] *= -1;
    }
    const map = world([{ center: [0, 90 * hemisphere], boundary }]);
    const rings = segments(createCartography(map, "chart").region(map.regions[0]));
    expect(contains(rings, [180, hemisphere > 0 ? 2 : 178])).toBe(true);
    expect(contains(rings, [180, 90])).toBe(false);
    expect(contains(rings, [180, hemisphere > 0 ? 178 : 2])).toBe(false);
    const [left, top, right, bottom] = bounds(rings);
    expect(left).toBeCloseTo(0, 5);
    expect(right).toBeCloseTo(360, 5);
    expect(top).toBeGreaterThanOrEqual(0);
    expect(bottom).toBeLessThanOrEqual(180);
    expect(bottom - top).toBeLessThanOrEqual(10.000001);
  });

  test("a geodesic route crosses the seam in two short strokes, not a world-spanning chord", () => {
    const map = world();
    const route: MapPoint[] = [[350, 90], [10, 90]];
    const strokes = segments(createCartography(map, "chart").line(route));
    expect(strokes).toHaveLength(2);
    expect(pathLength(strokes)).toBeCloseTo(20, 5);
    expect(bounds(strokes)[0]).toBeCloseTo(0, 5);
    expect(bounds(strokes)[2]).toBeCloseTo(360, 5);
    for (const stroke of strokes) expect(bounds([stroke])[2] - bounds([stroke])[0]).toBeLessThanOrEqual(10);
  });

  test("a high-latitude route bows toward the pole along its great circle", () => {
    const map = world();
    const route: MapPoint[] = [[120, 30], [240, 30]];
    const cartography = createCartography(map, "chart");
    const strokes = segments(cartography.line(route));
    const midpointLatitude = Math.atan2(Math.sin(Math.PI / 3), Math.cos(Math.PI / 3) * Math.cos(Math.PI / 3)) * 180 / Math.PI;
    expect(bounds(strokes)[1]).toBeCloseTo(90 - midpointLatitude, 5);
    expect(bounds(strokes)[3]).toBeCloseTo(30, 5);
    expect(pathLength(strokes)).toBeGreaterThan(120);
    expect(bounds(segments(cartography.labelLine(route)))[1]).toBeCloseTo(90 - midpointLatitude, 5);
  });

  test("source-coordinate caches stay local to their world's drawing scale", () => {
    const map = world();
    const larger = { ...world(), width: 720, height: 360, radiusKm: map.radiusKm * 2 };
    const route: MapPoint[] = [[180, 90], [200, 90]];
    expect(chartPoint(larger, [90, 45])).toEqual([540, 90]);
    expect(geographicPoint(larger, [540, 90])).toEqual([90, 45]);
    const first = createCartography(map, "chart");
    const second = createCartography(larger, "chart");
    expect(bounds(segments(first.line(route)))).toEqual([180, 90, 200, 90]);
    const [left, , right] = bounds(segments(second.line(route)));
    expect(left).toBeCloseTo(180, 5);
    expect(right).toBeCloseTo(200, 5);
  });
});

describe("globe visibility and rotation", () => {
  test("labels use the rotated hemisphere, including seam and pole view centres", () => {
    const map = world();
    const seam = createCartography(map, "globe", [180, 30]);
    const center = seam.point(chartPoint(map, [-180, 30]))!;
    expect(center[0]).toBeCloseTo(90, 5);
    expect(center[1]).toBeCloseTo(90, 5);
    expect(seam.point(chartPoint(map, [0, -30]))).toBeNull();
    const north = createCartography(map, "globe", [73, 90]);
    expect(north.point(chartPoint(map, [0, -10]))).toBeNull();
    const pole = north.point(chartPoint(map, [-120, 90]))!;
    expect(pole[0]).toBeCloseTo(90, 5);
    expect(pole[1]).toBeCloseTo(90, 5);
  });

  test("a cell with a hidden centre still paints its visible horizon sliver", () => {
    const map = world([{
      center: [100, 0], boundary: [[80, -15], [80, 15], [120, 15], [120, -15]],
    }, {
      center: [180, 0], boundary: [[170, -10], [170, 10], [-170, 10], [-170, -10]],
    }]);
    const front = createCartography(map, "globe", [0, 0]);
    expect(front.point(map.regions[0].site)).toBeNull();
    const rings = segments(front.region(map.regions[0]));
    expect(contains(rings, front.point(chartPoint(map, [85, 0]))!)).toBe(true);
    expect(contains(rings, front.point(chartPoint(map, [70, 0]))!)).toBe(false);
    for (const [x, y] of rings.flat()) expect(Math.hypot(x - 90, y - 90)).toBeLessThanOrEqual(90.000001);
    expect(front.region(map.regions[1])).toBe("");
    const back = createCartography(map, "globe", [180, 0]);
    expect(contains(segments(back.region(map.regions[1])), [90, 90])).toBe(true);
    expect(front.region(map.regions[1])).toBe("");
  });

  test("turning an existing globe refreshes hidden land, anchors, labels and route terminals", () => {
    const map = world([{ center: [180, 0], boundary: [[170, -10], [170, 10], [-170, 10], [-170, -10]] }]);
    const globe = createCartography(map, "globe", [0, 0]);
    const route: MapPoint[] = [chartPoint(map, [60, 0]), map.regions[0].site];
    const front = segments(globe.line(route));
    const frontGrid = segments(globe.graticule);
    expect(globe.point(map.regions[0].site)).toBeNull();
    expect(globe.region(map.regions[0])).toBe("");
    expect(globe.endLine(route)).toBe("");
    globe.rotate([180, 0]);
    const point = globe.point(map.regions[0].site)!;
    expect(point[0]).toBeCloseTo(90, 6);
    expect(point[1]).toBeCloseTo(90, 6);
    expect(contains(segments(globe.region(map.regions[0])), [90, 90])).toBe(true);
    expect(segments(globe.endLine(route)).at(-1)!.at(-1)).toEqual([90, 90]);
    const label = segments(globe.labelLine(route))[0];
    expect(label[0][0]).toBeLessThan(label.at(-1)![0]);
    globe.rotate([137, 17]);
    expect(segments(globe.graticule)).not.toEqual(frontGrid);
    globe.rotate([0, 0]);
    expect(globe.point(map.regions[0].site)).toBeNull();
    expect(globe.region(map.regions[0])).toBe("");
    expect(globe.endLine(route)).toBe("");
    expect(segments(globe.line(route))).toEqual(front);
    expect(segments(globe.graticule)).toEqual(frontGrid);
  });

  test("routes stop at the limb instead of folding hidden endpoints onto the front", () => {
    const cartography = createCartography(world(), "globe", [0, 0]);
    const route: MapPoint[] = [[180, 90], [300, 90]];
    expect(cartography.point(route[1])).toBeNull();
    const strokes = segments(cartography.line(route));
    expect(strokes).toHaveLength(1);
    expect(bounds(strokes)).toEqual([90, 90, 180, 90]);
    expect(pathLength(strokes)).toBeCloseTo(90, 5);
    const hidden: MapPoint[] = [[300, 90], [340, 90]];
    expect(cartography.line(hidden)).toBe("");
    expect(cartography.labelLine(hidden)).toBe("");
  });

  test("antipodal routes choose a reversible northward arc and polar routes use Greenwich", () => {
    const map = world();
    const route: MapPoint[] = [[180, 90], [360, 90]];
    for (const points of [route, route.toReversed()]) {
      const chart = segments(createCartography(map, "chart").line(points));
      expect(bounds(chart)[1]).toBeCloseTo(0, 5);
      expect(bounds(chart)[3]).toBeCloseTo(90, 5);
      const globe = segments(createCartography(map, "globe", [0, 0]).line(points));
      expect(bounds(globe)).toEqual([90, 0, 90, 90]);
      expect(pathLength(globe)).toBeCloseTo(90, 5);
    }
    const poles: MapPoint[] = [[180, 0], [180, 180]];
    const meridian = segments(createCartography(map, "globe", [0, 0]).line(poles));
    expect(bounds(meridian)).toEqual([90, 0, 90, 180]);
    expect(pathLength(meridian)).toBeCloseTo(180, 5);
  });

  test("cached caps and direct vertices match d3 across the limb, seam and poles", () => {
    const cells = [-179, -90, -1, 0, 89, 179].flatMap((longitude) =>
      [-89, -45, 0, 45, 89].flatMap((latitude) =>
        [0.1, 2, 25].map((radius) => {
          const center: MapPoint = [longitude, latitude];
          const boundary = geoCircle().center(center).radius(radius).precision(60)().coordinates[0].slice(0, -1) as MapPoint[];
          return { center, boundary };
        })));
    const map = world(cells);
    const globe = createCartography(map, "globe");
    for (const longitude of [-180, -90, -0.1, 0, 89.9, 180]) {
      for (const latitude of [-90, -45, 0, 45, 90]) {
        const rotation: MapPoint = [longitude, latitude];
        globe.rotate(rotation);
        const projection = geoOrthographic().scale(map.height / 2).translate([map.height / 2, map.height / 2])
          .rotate([-longitude, -latitude]).clipAngle(90).precision(map.height / 2000);
        const path = geoPath(projection).digits(6);
        for (const region of map.regions) {
          const expected = path({ type: "Polygon", coordinates: [[...region.boundary, region.boundary[0]]] }) ?? "";
          expect(globe.region(region)).toBe(expected);
          // A second read shares the same projected result, including empty cells.
          expect(globe.region(region)).toBe(expected);
          const point = globe.point(region.site);
          if (point) {
            const reference = projection(region.center)!;
            expect(point[0]).toBeCloseTo(reference[0], 10);
            expect(point[1]).toBeCloseTo(reference[1], 10);
          }
        }
      }
    }
  });

  test("cached coastal and river caps preserve clipped and resampled d3 lines", () => {
    const map = world();
    const globe = createCartography(map, "globe");
    const courses: MapPoint[][] = [
      [[-1, 0], [1, 1]], [[178, 10], [-179, 11]], [[90, -2], [90, 2]],
      [[88, 0], [89.9, 0]], [[-40, 0], [40, 0]], [[0, -80], [0, 0], [0, 80]],
      [[-120, -30], [0, 60], [120, -30]],
    ];
    const lines = courses.map((course) => course.map((point) => chartPoint(map, point)));
    for (const rotation of [[0, 0], [0, 90], [0, -90], [180, 30], [89.9, 0], [-90, 20], [0, 0]] as MapPoint[]) {
      globe.rotate(rotation);
      const path = geoPath(geoOrthographic().scale(map.height / 2).translate([map.height / 2, map.height / 2])
        .rotate([-rotation[0], -rotation[1]]).clipAngle(90).precision(map.height / 2000)).digits(6);
      for (const points of lines) {
        const expected = path({ type: "LineString", coordinates: points.map((point) => geographicPoint(map, point)) }) ?? "";
        expect(globe.line(points)).toBe(expected);
        expect(globe.line(points)).toBe(expected);
      }
    }
  });
});

describe("continuous readable label paths", () => {
  test("a seam-crossing river label takes its longest side and reads left to right in either direction", () => {
    const cartography = createCartography(world(), "chart");
    const route: MapPoint[] = [[350, 90], [10, 90], [60, 90]];
    for (const points of [route, route.toReversed()]) {
      const labels = segments(cartography.labelLine(points));
      expect(labels).toHaveLength(1);
      expect(labels[0][0][0]).toBeCloseTo(0, 5);
      expect(labels[0].at(-1)![0]).toBeCloseTo(60, 5);
      expect(pathLength(labels)).toBeCloseTo(60, 5);
    }
  });

  test("a globe label never bridges a hidden reach between visible stretches", () => {
    const cartography = createCartography(world(), "globe", [0, 0]);
    const route: MapPoint[] = [[180, 90], [300, 90], [360, 90], [60, 90], [150, 90]];
    const strokes = segments(cartography.line(route));
    expect(strokes).toHaveLength(2);
    const label = segments(cartography.labelLine(route));
    expect(label).toHaveLength(1);
    expect(bounds(label)).toEqual([90, 90, 180, 90]);
    expect(label[0][0][0]).toBeLessThan(label[0].at(-1)![0]);
    expect(pathLength(label)).toBeCloseTo(90, 5);
  });
});

describe("route terminal markers", () => {
  test("seam-crossing routes expose only their destination-side stroke to markerEnd", () => {
    const cartography = createCartography(world(), "chart");
    const route: MapPoint[] = [[350, 90], [10, 90], [60, 90]];
    const forward = segments(cartography.endLine(route));
    expect(forward).toHaveLength(1);
    expect(forward[0][0]).toEqual([0, 90]);
    expect(forward[0].at(-1)).toEqual([60, 90]);
    const reverse = segments(cartography.endLine(route.toReversed()));
    expect(reverse).toHaveLength(1);
    expect(reverse[0][0]).toEqual([360, 90]);
    expect(reverse[0].at(-1)).toEqual([350, 90]);
  });

  test("readable label orientation never reverses the actual route destination", () => {
    const cartography = createCartography(world(), "chart");
    const route: MapPoint[] = [[220, 90], [180, 90], [140, 90]];
    const label = segments(cartography.labelLine(route));
    expect(label[0][0]).toEqual([140, 90]);
    expect(label[0].at(-1)).toEqual([220, 90]);
    const terminal = segments(cartography.endLine(route));
    expect(terminal).toHaveLength(1);
    expect(terminal[0][0]).toEqual([220, 90]);
    expect(terminal[0].at(-1)).toEqual([140, 90]);
  });

  test("a hidden destination has no false limb arrow; a returning route marks its actual final reach", () => {
    const cartography = createCartography(world(), "globe", [0, 0]);
    const leaving: MapPoint[] = [[180, 90], [300, 90]];
    expect(pathLength(segments(cartography.line(leaving)))).toBeCloseTo(90, 5);
    expect(cartography.endLine(leaving)).toBe("");
    const returning: MapPoint[] = [[180, 90], [300, 90], [360, 90], [60, 90], [150, 90]];
    const terminal = segments(cartography.endLine(returning));
    expect(terminal).toHaveLength(1);
    expect(terminal[0][0]).toEqual([0, 90]);
    expect(terminal[0].at(-1)).toEqual([45, 90]);
  });
});

describe("canonical coastal borders and river courses", () => {
  test("shared vertices at opposite chart edges describe one spherical border", () => {
    const map = world([
      { center: [175, 0], boundary: [[170, 0], [180, 10], [180, -10]], neighbours: [1] },
      { center: [-175, 0], boundary: [[-180, -10], [-180, 10], [-170, 0]], neighbours: [0] },
      { center: [0, 0] },
    ]);
    const border = sharedBorder(map, 0, 1);
    expect(border.map((point) => geographicPoint(map, point))).toEqual([[180, 10], [180, -10]]);
    expect(sharedBorder(map, 0, 2)).toEqual([]);
    const stroke = segments(createCartography(map, "globe", [180, 0]).line(border));
    expect(bounds(stroke)[0]).toBeCloseTo(90, 5);
    expect(bounds(stroke)[2]).toBeCloseTo(90, 5);
    expect(pathLength(stroke)).toBeCloseTo(180 * Math.sin(Math.PI / 18), 5);
  });

  test("an ambiguous tributary ends at its actual confluence, not the first adjacent parent reach", () => {
    const tributary: River = { id: 0, course: [0], channel: [], mouth: 1, catchment: [0], joins: 1, joinAt: 3, lengthKm: 500 };
    const parent: River = { id: 1, course: [2, 3], channel: [], mouth: 1, catchment: [0, 2, 3], joins: null, joinAt: null, lengthKm: 1000 };
    const map = world([
      { center: [0, 0], neighbours: [2, 3] }, { center: [90, 0], terrain: "sea" },
      { center: [20, 0] }, { center: [3, 4] },
    ]);
    map.rivers = [tributary, parent];
    expect(riverPoints(map, tributary)).toEqual([[180, 90], [183, 86]]);
  });

  test("a closed-basin river ends in its terminal lake cell without seeking a coastline", () => {
    const river: River = { id: 0, course: [0, 1, 2], channel: [], mouth: 2, catchment: [0, 1, 2], joins: null, joinAt: null, lengthKm: 500 };
    const map = world([
      { center: [-10, 0], neighbours: [1] },
      { center: [0, 5], neighbours: [0, 2] },
      { center: [10, 0], neighbours: [1] },
    ]);
    map.rivers = [river];
    map.lakes = [{ id: 0, regions: [1, 2], outlet: null }];
    const points = riverPoints(map, river);
    expect(points).toHaveLength(9);
    expect(points[0]).toEqual(chartPoint(map, map.regions[0].center));
    expect(points.at(-1)).toEqual(chartPoint(map, map.regions[2].center));
    expect(riverPoints(map, river)).toBe(points);
    for (const projection of ["chart", "globe"] as const) {
      expect(segments(createCartography(map, projection).line(points))).toHaveLength(1);
    }
  });

  test("a sea outlet ends at the actual shared coast, not the sea centre or centre-to-centre midpoint", () => {
    const river: River = { id: 0, course: [0], channel: [], mouth: 1, catchment: [0], joins: null, joinAt: null, lengthKm: 1000 };
    const map = world([
      { center: [0, 0], boundary: [[-10, -10], [-10, 10], [10, 10], [10, -10]], neighbours: [1] },
      { center: [40, 0], boundary: [[10, -10], [10, 10], [50, 10], [50, -10]], neighbours: [0], terrain: "sea" },
    ]);
    const points = riverPoints(map, river);
    expect(points).toHaveLength(2);
    const outlet = geographicPoint(map, points[1]);
    expect(outlet[0]).toBeCloseTo(10, 5);
    expect(outlet[1]).toBeCloseTo(0, 5);
  });

  test("a shore wrapping the canonical ring keeps every bend and its outlet on the coast", () => {
    const map = world([
      { center: [-5, 0], boundary: [[0, 17], [0, 0], [10, 0], [20, -10], [-10, -10], [-10, 20], [0, 20]], neighbours: [1] },
      { center: [15, 10], boundary: [[10, 0], [0, 0], [0, 17], [0, 20], [20, 20], [20, 0]], neighbours: [0], terrain: "sea" },
    ]);
    const border = sharedBorder(map, 0, 1);
    expect(border.map((point) => geographicPoint(map, point))).toEqual([[0, 20], [0, 17], [0, 0], [10, 0]]);
    const river: River = { id: 0, course: [0], channel: [], mouth: 1, catchment: [0], joins: null, joinAt: null, lengthKm: 1000 };
    const outlet = geographicPoint(map, riverPoints(map, river).at(-1)!);
    expect(outlet[0]).toBeCloseTo(0, 6);
    expect(outlet[1]).toBeCloseTo(5, 6);
    const shore = segments(createCartography(map).line(border))[0];
    expect(shore.some(([x, y]) => Math.abs(x - 180) < 1e-6 && Math.abs(y - 90) < 1e-6)).toBe(true);
  });

  test("a high-latitude seam outlet uses the shared great-circle midpoint", () => {
    const river: River = { id: 0, course: [0], channel: [], mouth: 1, catchment: [0], joins: null, joinAt: null, lengthKm: 1038 };
    const map = world([
      { center: [180, 50], boundary: [[170, 60], [-170, 60], [180, 40]], neighbours: [1] },
      { center: [180, 70], boundary: [[-170, 60], [170, 60], [180, 80]], neighbours: [0], terrain: "sea" },
    ]);
    const points = riverPoints(map, river);
    const outlet = geographicPoint(map, points.at(-1)!);
    const latitude = Math.atan2(Math.sin(Math.PI / 3), Math.cos(Math.PI / 3) * Math.cos(Math.PI / 18)) * 180 / Math.PI;
    expect(Math.abs(outlet[0])).toBeCloseTo(180, 5);
    expect(outlet[1]).toBeCloseTo(latitude, 5);
    expect(outlet[1]).toBeGreaterThan(60);
    const globe = createCartography(map, "globe", [180, 60]);
    expect(bounds(segments(globe.line(points)))[0]).toBeCloseTo(90, 5);
    expect(bounds(segments(globe.line(points)))[2]).toBeCloseTo(90, 5);
  });
});

describe("region-following lake shores", () => {
  test("a bent lake retains its dry bay and has no ink on internal cell borders", () => {
    const cell = (x: number, y: number, neighbours: number[]) => ({
      center: [x + 5, y + 5] as MapPoint, neighbours,
      boundary: [[x, y], [x, y + 10], [x + 10, y + 10], [x + 10, y]] as MapPoint[],
    });
    const map = world([cell(0, 0, [1]), cell(10, 0, [0, 2]), cell(10, 10, [1])]);
    const lake = { id: 7, regions: [0, 1, 2], outlet: null };
    const outline = lakeOutline(map, lake), chart = createCartography(map);
    expect(outline.fills).toHaveLength(3);
    expect(outline.shores).toHaveLength(8);
    const water = outline.fills.map((fill) => segments(chart.area(fill)));
    for (const region of map.regions) expect(water.some((rings) => contains(rings, region.site))).toBe(true);
    expect(water.some((rings) => contains(rings, chartPoint(map, [5, 15])))).toBe(false);
    expect(lakeOutline(map, lake)).toEqual(outline);
    expect(lakeOutline(map, { ...lake, id: 8 }).fills).not.toEqual(outline.fills);
    expect(lake.regions).toEqual([0, 1, 2]);
  });

  test.each([[179, 30], [30, 81], [-50, -86]] as MapPoint[])(
    "seam and polar lakes keep their spherical footprint at %j", (longitude, latitude) => {
      const center: MapPoint = [longitude, latitude];
      const boundary = geoCircle().center(center).radius(6).precision(24)().coordinates[0].slice(0, -1) as MapPoint[];
      const map = world([{ center, boundary }]);
      const outline = lakeOutline(map, { id: 1, regions: [0], outlet: null });
      const globe = createCartography(map, "globe", center);
      const rings = segments(globe.area(outline.fills[0]));
      const [left, top, right, bottom] = bounds(rings);
      expect(contains(rings, [90, 90])).toBe(true);
      expect((right - left) / (bottom - top)).toBeGreaterThan(0.85);
      expect((right - left) / (bottom - top)).toBeLessThan(1.15);
      expect(right - left).toBeGreaterThan(17);
      const chart = createCartography(map);
      expect(contains(segments(chart.area(outline.fills[0])), map.regions[0].site)).toBe(true);
      for (const shore of outline.shores) segments(chart.line(shore));
    },
  );
});

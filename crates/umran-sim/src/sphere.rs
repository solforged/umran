//! Closed dual of a subdivided icosahedron. IDs follow fixed face/edge order;
//! no geometry lookup, hashing order, chart seam, or pole affects topology.

use crate::math;
use std::collections::BTreeMap;

pub(crate) type Point = [f64; 3];

pub(crate) fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn unit(a: Point) -> Point {
    let norm = dot(a, a).sqrt();
    [a[0] / norm, a[1] / norm, a[2] / norm]
}

pub(crate) fn tangent(p: Point) -> (Point, Point) {
    let axis = if p[2].abs() < 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let east = unit(cross(axis, p));
    (east, cross(p, east))
}

pub(crate) fn angle(a: Point, b: Point) -> f64 {
    let c = cross(a, b);
    math::atan2(dot(c, c).sqrt(), dot(a, b))
}

/// Solid angle of a minor spherical triangle, robust even for tiny cells.
fn triangle_area(a: Point, b: Point, c: Point) -> f64 {
    2.0 * math::atan2(
        dot(a, cross(b, c)).abs(),
        1.0 + dot(a, b) + dot(b, c) + dot(c, a),
    )
}

pub(crate) fn area(position: Point, boundary: &[Point]) -> f64 {
    boundary
        .iter()
        .enumerate()
        .map(|(i, &a)| triangle_area(position, a, boundary[(i + 1) % boundary.len()]))
        .sum()
}

/// Exact first surface moment of a clockwise geodesic polygon. Normalizing
/// the area-weighted sum locates a landmass without a chart-seam discontinuity.
pub(crate) fn moment(boundary: &[Point]) -> Point {
    let mut sum = [0.0; 3];
    for (i, &a) in boundary.iter().enumerate() {
        let b = boundary[(i + 1) % boundary.len()];
        let normal = cross(a, b);
        let norm = dot(normal, normal).sqrt();
        let weight = -0.5 * angle(a, b) / norm;
        for j in 0..3 {
            sum[j] += weight * normal[j];
        }
    }
    sum
}

pub(crate) struct Cell {
    pub position: Point,
    pub boundary: Vec<Point>,
    pub neighbours: Vec<usize>,
}

pub(crate) struct Border {
    pub a: usize,
    pub b: usize,
    pub midpoint: Point,
    faces: [usize; 2],
}

pub(crate) struct Mesh {
    pub cells: Vec<Cell>,
    pub borders: Vec<Border>,
    faces: Vec<[usize; 3]>,
    corners: Vec<Point>,
}

fn edge(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

fn midpoint(
    vertices: &mut Vec<Point>,
    edges: &mut BTreeMap<(usize, usize), usize>,
    a: usize,
    b: usize,
) -> usize {
    *edges.entry(edge(a, b)).or_insert_with(|| {
        let id = vertices.len();
        vertices.push(unit(add(vertices[a], vertices[b])));
        id
    })
}

/// O(N log N) construction and O(N) storage, with exactly 10*4^level+2 cells.
/// Circumcentres of each outward triangle are the shared dual vertices.
pub(crate) fn mesh(level: u32) -> Mesh {
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let mut vertices = vec![
        [-1.0, phi, 0.0],
        [1.0, phi, 0.0],
        [-1.0, -phi, 0.0],
        [1.0, -phi, 0.0],
        [0.0, -1.0, phi],
        [0.0, 1.0, phi],
        [0.0, -1.0, -phi],
        [0.0, 1.0, -phi],
        [phi, 0.0, -1.0],
        [phi, 0.0, 1.0],
        [-phi, 0.0, -1.0],
        [-phi, 0.0, 1.0],
    ];
    for p in &mut vertices {
        *p = unit(*p);
    }
    let mut faces = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..level {
        let mut edges = BTreeMap::new();
        let mut next = Vec::with_capacity(faces.len() * 4);
        for [a, b, c] in faces {
            let ab = midpoint(&mut vertices, &mut edges, a, b);
            let bc = midpoint(&mut vertices, &mut edges, b, c);
            let ca = midpoint(&mut vertices, &mut edges, c, a);
            next.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        faces = next;
    }
    let mut incident: Vec<_> = (0..vertices.len()).map(|_| Vec::with_capacity(6)).collect();
    let mut edge_faces = BTreeMap::<(usize, usize), [usize; 2]>::new();
    let mut dual = Vec::with_capacity(faces.len());
    for (f, &[a, b, c]) in faces.iter().enumerate() {
        let normal = unit(cross(
            sub(vertices[b], vertices[a]),
            sub(vertices[c], vertices[a]),
        ));
        debug_assert!(dot(normal, vertices[a]) > 0.0);
        dual.push(normal);
        for r in [a, b, c] {
            incident[r].push(f);
        }
        for (u, v) in [(a, b), (b, c), (c, a)] {
            edge_faces
                .entry(edge(u, v))
                .and_modify(|faces| {
                    assert_eq!(faces[1], usize::MAX, "at most two faces meet an edge");
                    faces[1] = f;
                })
                .or_insert([f, usize::MAX]);
        }
    }
    let mut neighbours: Vec<_> = (0..vertices.len()).map(|_| Vec::with_capacity(6)).collect();
    let borders = edge_faces
        .into_iter()
        .map(|((a, b), faces)| {
            assert_ne!(
                faces[1],
                usize::MAX,
                "a closed mesh edge has two incident faces"
            );
            neighbours[a].push(b);
            neighbours[b].push(a);
            Border {
                a,
                b,
                midpoint: unit(add(dual[faces[0]], dual[faces[1]])),
                faces,
            }
        })
        .collect();
    let cells = vertices
        .into_iter()
        .zip(incident)
        .zip(neighbours)
        .map(|((position, faces), mut neighbours)| {
            let (east, north) = tangent(position);
            let mut ring: Vec<_> = faces
                .into_iter()
                .map(|f| {
                    let p = dual[f];
                    (math::atan2(dot(p, north), dot(p, east)), f)
                })
                .collect();
            // Descending tangent angle is clockwise when viewed from outside.
            ring.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            neighbours.sort_unstable();
            Cell {
                position,
                boundary: ring.into_iter().map(|(_, f)| dual[f]).collect(),
                neighbours,
            }
        })
        .collect();
    Mesh {
        cells,
        borders,
        faces,
        corners: dual,
    }
}

/// A minor-arc interpolation, using the same portable trigonometry as areas.
fn interpolate(a: Point, b: Point, t: f64) -> Point {
    if t == 0.0 {
        return a;
    }
    if t == 1.0 {
        return b;
    }
    let arc = angle(a, b);
    let left = math::sin((1.0 - t) * arc);
    let right = math::sin(t * arc);
    unit(std::array::from_fn(|i| left * a[i] + right * b[i]))
}

/// Shared chains can wrap the unclosed ring's first vertex. No temporary
/// polygon or allocation is needed to find their half-arclength point.
pub(crate) fn shared_midpoint(a: &[Point], b: &[Point]) -> Option<Point> {
    let start = (0..a.len())
        .find(|&i| b.contains(&a[i]) && !b.contains(&a[(i + a.len() - 1) % a.len()]))?;
    let point = |offset: usize| a[(start + offset) % a.len()];
    let count = (0..a.len()).take_while(|&i| b.contains(&point(i))).count();
    if count < 2 {
        return None;
    }
    if count == 2 {
        return Some(unit(add(point(0), point(1))));
    }
    let length: f64 = (1..count).map(|i| angle(point(i - 1), point(i))).sum();
    let mut remaining = length * 0.5;
    for i in 1..count {
        let segment = angle(point(i - 1), point(i));
        if remaining <= segment || i == count - 1 {
            return Some(interpolate(
                point(i - 1),
                point(i),
                (remaining / segment).clamp(0.0, 1.0),
            ));
        }
        remaining -= segment;
    }
    unreachable!("a shared border has a final segment")
}

struct ShoreFace {
    node: Point,
    /// Ordered edge numbers in the primal face and one eight-segment shore.
    curve: Option<([usize; 2], [Point; 9])>,
}

impl ShoreFace {
    /// A spoke runs from the common three-cell junction to a centre-edge
    /// crossing. Coast spokes are halves of the same smooth spherical curve.
    fn spoke(&self, edge: usize, crossing: Point) -> ([Point; 5], usize) {
        if let Some((edges, curve)) = &self.curve {
            if edge == edges[0] {
                return (std::array::from_fn(|i| curve[4 - i]), 5);
            }
            if edge == edges[1] {
                return (std::array::from_fn(|i| curve[4 + i]), 5);
            }
        }
        ([self.node, crossing, crossing, crossing, crossing], 2)
    }

    fn separates_centres(&self, positions: [Point; 3], crossings: [Point; 3]) -> bool {
        (0..3).all(|e| {
            let (spoke, count) = self.spoke(e, crossings[e]);
            spoke[..count].windows(2).all(|pair| {
                let normal = cross(pair[0], pair[1]);
                dot(positions[e], normal) < 0.0 && dot(positions[(e + 1) % 3], normal) > 0.0
            })
        })
    }
}

/// Replace only the geometry touching a coast, inside the original primal
/// triangles. Centre-edge crossings stay strictly inside each edge; the three
/// spokes partition each face. Every segment separates its two owning centres,
/// making each resulting cell a clockwise star polygon. This preserves the
/// closed tiling, adjacency, islands and straits without display-only noise.
///
/// O(N log N) time for sorted border lookups, O(N) temporary storage. Coast
/// borders have eight segments; affected same-terrain borders have two.
pub(crate) fn refine_coasts(mesh: &mut Mesh, shore: &[f64], detail: impl Fn(Point) -> f64) {
    let border_id = |a, b| {
        mesh.borders
            .binary_search_by_key(&edge(a, b), |border| (border.a, border.b))
            .unwrap()
    };
    let crossings: Vec<_> = mesh
        .borders
        .iter()
        .map(|border| {
            let a = mesh.cells[border.a].position;
            let b = mesh.cells[border.b].position;
            let mixed = (shore[border.a] > 0.0) != (shore[border.b] > 0.0);
            let t = if mixed {
                (shore[border.a] / (shore[border.a] - shore[border.b])
                    + 0.06 * detail(unit(add(a, b))))
                .clamp(0.20, 0.80)
            } else {
                0.5
            };
            interpolate(a, b, t)
        })
        .collect();
    let mut incident = vec![[usize::MAX; 6]; mesh.cells.len()];
    let mut shapes = Vec::with_capacity(mesh.faces.len());
    for (f, &face) in mesh.faces.iter().enumerate() {
        let old_corner = mesh.corners[f];
        for &r in &face {
            let corner = mesh.cells[r]
                .boundary
                .iter()
                .position(|&p| p == old_corner)
                .unwrap();
            incident[r][corner] = f;
        }
        let edges = std::array::from_fn::<_, 3, _>(|e| border_id(face[e], face[(e + 1) % 3]));
        let points = edges.map(|e| crossings[e]);
        let mut coast_edges = [0; 2];
        let mut count = 0;
        for e in 0..3 {
            if (shore[face[e]] > 0.0) != (shore[face[(e + 1) % 3]] > 0.0) {
                coast_edges[count] = e;
                count += 1;
            }
        }
        let shape = if count == 0 {
            ShoreFace {
                node: old_corner,
                curve: None,
            }
        } else {
            debug_assert_eq!(count, 2, "a binary face has zero or two shore crossings");
            let a = points[coast_edges[0]];
            let b = points[coast_edges[1]];
            let middle = std::array::from_fn::<_, 3, _>(|i| 0.5 * (a[i] + b[i]));
            let mut strength = (0.6 + 0.3 * detail(old_corner)).clamp(0.2, 0.9);
            let mut result = None;
            // This local curvature limiter protects skinny clipped triangles;
            // it does not retry terrain or invent a different connectivity.
            for attempt in 0..=16 {
                if attempt == 16 {
                    strength = 0.0;
                }
                let control = std::array::from_fn::<_, 3, _>(|i| {
                    middle[i] + strength * (old_corner[i] - middle[i])
                });
                let curve = std::array::from_fn(|j| {
                    if j == 0 {
                        return a;
                    }
                    if j == 8 {
                        return b;
                    }
                    let t = j as f64 / 8.0;
                    unit(std::array::from_fn(|i| {
                        (1.0 - t) * (1.0 - t) * a[i]
                            + 2.0 * t * (1.0 - t) * control[i]
                            + t * t * b[i]
                    }))
                });
                let candidate = ShoreFace {
                    node: curve[4],
                    curve: Some((coast_edges, curve)),
                };
                if candidate.separates_centres(face.map(|r| mesh.cells[r].position), points) {
                    result = Some(candidate);
                    break;
                }
                strength *= 0.5;
            }
            result.expect("straight clipped-face spokes separate their centres")
        };
        shapes.push(shape);
    }
    for (r, cell) in mesh.cells.iter_mut().enumerate() {
        if incident[r][..cell.boundary.len()]
            .iter()
            .all(|&f| shapes[f].curve.is_none())
        {
            continue;
        }
        let coastal = cell
            .neighbours
            .iter()
            .filter(|&&n| (shore[r] > 0.0) != (shore[n] > 0.0))
            .count();
        let mut boundary = Vec::with_capacity(cell.boundary.len() * 2 + coastal * 6);
        for i in 0..cell.boundary.len() {
            let f = incident[r][i];
            let g = incident[r][(i + 1) % cell.boundary.len()];
            if shapes[f].curve.is_none() && shapes[g].curve.is_none() {
                boundary.push(shapes[f].node);
                continue;
            }
            let other = mesh.faces[f]
                .iter()
                .copied()
                .find(|&n| n != r && mesh.faces[g].contains(&n))
                .unwrap();
            let crossing = crossings[border_id(r, other)];
            let face_edge = |face: [usize; 3]| {
                (0..3)
                    .find(|&e| edge(face[e], face[(e + 1) % 3]) == edge(r, other))
                    .unwrap()
            };
            let (first, first_len) = shapes[f].spoke(face_edge(mesh.faces[f]), crossing);
            let (second, second_len) = shapes[g].spoke(face_edge(mesh.faces[g]), crossing);
            // Include the start junction and crossing exactly once; the next
            // border supplies the end junction.
            boundary.extend_from_slice(&first[..first_len]);
            boundary.extend(second[1..second_len - 1].iter().rev().copied());
        }
        cell.boundary = boundary;
    }
    for border in &mut mesh.borders {
        if shapes[border.faces[0]].curve.is_some() || shapes[border.faces[1]].curve.is_some() {
            border.midpoint = shared_midpoint(
                &mesh.cells[border.a].boundary,
                &mesh.cells[border.b].boundary,
            )
            .expect("neighbouring cells retain their shared chain");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn dual_tiles_a_closed_sphere_with_clockwise_shared_edges() {
        for level in [3, 4, 5] {
            let mesh = mesh(level);
            let n = mesh.cells.len();
            assert_eq!(n, 10 * 4_usize.pow(level) + 2);
            assert_eq!(mesh.borders.len(), 3 * n - 6);
            assert_eq!(
                mesh.cells
                    .iter()
                    .filter(|c| c.neighbours.len() == 5)
                    .count(),
                12
            );
            let total: f64 = mesh
                .cells
                .iter()
                .map(|c| area(c.position, &c.boundary))
                .sum();
            assert!((total - 4.0 * PI).abs() < 1e-10);
            let total_moment = mesh
                .cells
                .iter()
                .fold([0.0; 3], |sum, c| add(sum, moment(&c.boundary)));
            assert!(dot(total_moment, total_moment) < 1e-22);
            for (id, cell) in mesh.cells.iter().enumerate() {
                assert!((dot(cell.position, cell.position) - 1.0).abs() < 1e-14);
                assert_eq!(cell.boundary.len(), cell.neighbours.len());
                assert!((5..=6).contains(&cell.boundary.len()));
                let surface_moment = moment(&cell.boundary);
                assert!(dot(surface_moment, cell.position) > 0.0);
                assert!(
                    dot(surface_moment, surface_moment).sqrt()
                        <= area(cell.position, &cell.boundary)
                );
                for (i, &a) in cell.boundary.iter().enumerate() {
                    let b = cell.boundary[(i + 1) % cell.boundary.len()];
                    assert!((dot(a, a) - 1.0).abs() < 1e-14);
                    assert!(dot(cell.position, cross(a, b)) < 0.0);
                    let owners = cell
                        .neighbours
                        .iter()
                        .filter(|&&other| {
                            let ring = &mesh.cells[other].boundary;
                            ring.iter()
                                .enumerate()
                                .any(|(j, &p)| p == b && ring[(j + 1) % ring.len()] == a)
                        })
                        .count();
                    assert_eq!(
                        owners, 1,
                        "cell {id} has exactly one neighbour across each edge"
                    );
                }
                for &other in &cell.neighbours {
                    assert!(mesh.cells[other].neighbours.contains(&id));
                }
            }
            for pole in [[0.0, 0.0, 1.0], [0.0, 0.0, -1.0]] {
                let cell = mesh
                    .cells
                    .iter()
                    .find(|c| dot(c.position, pole) > 1.0 - 1e-14)
                    .unwrap();
                assert!(cell.boundary.iter().all(|&p| dot(p, pole) < 1.0));
            }
        }
    }

    fn assert_refined_tiling(mesh: &Mesh) {
        let total: f64 = mesh
            .cells
            .iter()
            .map(|c| area(c.position, &c.boundary))
            .sum();
        assert!(
            (total - 4.0 * PI).abs() < 1e-10,
            "closed spherical coverage"
        );
        let total_moment = mesh
            .cells
            .iter()
            .fold([0.0; 3], |sum, c| add(sum, moment(&c.boundary)));
        assert!(dot(total_moment, total_moment) < 1e-22);
        for (id, cell) in mesh.cells.iter().enumerate() {
            let surface = area(cell.position, &cell.boundary);
            assert!(surface > 0.0);
            let centre_moment = moment(&cell.boundary);
            assert!(dot(centre_moment, cell.position) > 0.0);
            assert!(dot(centre_moment, centre_moment).sqrt() <= surface + 1e-14);
            // A clockwise fan around an unchanged interior centre proves the
            // ring is simple; exact reversed edges prove there are no cracks.
            for (i, &a) in cell.boundary.iter().enumerate() {
                let b = cell.boundary[(i + 1) % cell.boundary.len()];
                assert!((dot(a, a) - 1.0).abs() < 1e-14);
                assert!(dot(cell.position, cross(a, b)) < 0.0, "cell {id} folds");
                let owners = cell
                    .neighbours
                    .iter()
                    .filter(|&&n| {
                        let ring = &mesh.cells[n].boundary;
                        ring.iter()
                            .enumerate()
                            .any(|(j, &p)| p == b && ring[(j + 1) % ring.len()] == a)
                    })
                    .count();
                assert_eq!(owners, 1, "one opposite owner for every segment");
            }
        }
        for border in &mesh.borders {
            let a = &mesh.cells[border.a].boundary;
            let b = &mesh.cells[border.b].boundary;
            assert_eq!(shared_midpoint(a, b), Some(border.midpoint));
        }
    }

    #[test]
    fn refined_shores_tile_across_seams_poles_and_thin_clipped_faces() {
        for level in [3, 4, 5] {
            for field in 0..4 {
                let mut mesh = mesh(level);
                let centres: Vec<_> = mesh.cells.iter().map(|c| c.position).collect();
                let neighbours: Vec<_> = mesh.cells.iter().map(|c| c.neighbours.clone()).collect();
                let shore: Vec<_> = mesh
                    .cells
                    .iter()
                    .map(|c| {
                        let p = c.position;
                        let value = match field {
                            0 => p[0] + 0.003, // meridian near both poles and the seam
                            1 => p[2] - 0.93,  // small polar cap
                            2 => {
                                p[0] * 0.3 + p[1] * 0.7 + p[2] * 0.2 + 0.1 * math::sin(29.0 * p[2])
                            }
                            _ => {
                                if p[0] > 0.0 {
                                    1e-12
                                } else {
                                    -1.0
                                }
                            } // extreme clipping
                        };
                        if value == 0.0 { 1e-12 } else { value }
                    })
                    .collect();
                refine_coasts(&mut mesh, &shore, |p| math::sin(113.0 * p[0] + 71.0 * p[2]));
                assert_eq!(mesh.cells.len(), centres.len());
                for (id, cell) in mesh.cells.iter().enumerate() {
                    assert_eq!(cell.position, centres[id]);
                    assert_eq!(cell.neighbours, neighbours[id]);
                }
                assert_refined_tiling(&mesh);
                for border in &mesh.borders {
                    if (shore[border.a] > 0.0) != (shore[border.b] > 0.0) {
                        let a = &mesh.cells[border.a].boundary;
                        let b = &mesh.cells[border.b].boundary;
                        assert!(a.iter().filter(|p| b.contains(p)).count() > 2);
                    }
                }
            }
        }
    }

    #[test]
    fn continental_fields_keep_coherent_land_and_valid_refined_geometry() {
        for (level, radius) in [(3, 800.0), (4, 1_600.0), (4, 3_200.0), (5, 6_371.0)] {
            for seed in [0, 7, 31] {
                let mut mesh = mesh(level);
                let (terrain, elevation, moisture) =
                    crate::continental::surface(seed, &mut mesh, radius);
                assert_refined_tiling(&mesh);
                let land_area: f64 = mesh
                    .cells
                    .iter()
                    .enumerate()
                    .filter(|(r, _)| terrain[*r].is_land())
                    .map(|(_, c)| area(c.position, &c.boundary))
                    .sum();
                assert!(
                    (0.20..0.55).contains(&(land_area / (4.0 * PI))),
                    "seed {seed}"
                );
                assert!(elevation.iter().all(|e| (0.0..=1.0).contains(e)));
                assert!(moisture.iter().all(|m| (0.0..=1.0).contains(m)));
                let land_edges = mesh
                    .borders
                    .iter()
                    .filter(|b| terrain[b.a].is_land() && terrain[b.b].is_land())
                    .count();
                let shore_edges = mesh
                    .borders
                    .iter()
                    .filter(|b| terrain[b.a].is_land() != terrain[b.b].is_land())
                    .count();
                assert!(
                    land_edges > shore_edges,
                    "coherent crust, not isolated noise"
                );
                // Wet maritime lands and drier interiors must coexist; a
                // uniform moisture field would erase the climate mechanism.
                let land = terrain.iter().enumerate().filter(|(_, t)| t.is_land());
                let driest = land.clone().map(|(r, _)| moisture[r]).fold(1.0, f64::min);
                let wettest = land.map(|(r, _)| moisture[r]).fold(0.0, f64::max);
                assert!(wettest - driest > 0.20);
            }
        }
    }

    #[test]
    fn shared_chain_midpoint_wraps_the_ring_and_uses_arc_length() {
        let point = |radians: f64| [math::cos(radians), math::sin(radians), 0.0];
        let a = point(0.0);
        let b = point(0.2);
        let c = point(0.9);
        let d = point(1.0);
        let north = [0.0, 0.0, 1.0];
        let south = [0.0, 0.0, -1.0];
        let first = [c, d, north, a, b];
        let second = [south, d, c, b, a];
        let actual = shared_midpoint(&first, &second).unwrap();
        assert!(angle(actual, point(0.5)) < 1e-14);
        assert!(angle(actual, shared_midpoint(&second, &first).unwrap()) < 1e-14);
        assert_eq!(
            shared_midpoint(&[a, b, north], &[b, a, south]),
            Some(unit(add(a, b)))
        );
        assert_eq!(shared_midpoint(&[a, b, north], &[c, d, south]), None);
    }

    #[test]
    fn geodesics_handle_coincident_quarter_circle_and_antipodal_points() {
        let a = [1.0, 0.0, 0.0];
        assert_eq!(angle(a, a), 0.0);
        assert!((angle(a, [0.0, 1.0, 0.0]) - PI / 2.0).abs() < 1e-15);
        assert!((angle(a, [-1.0, 0.0, 0.0]) - PI).abs() < 1e-15);
    }
}

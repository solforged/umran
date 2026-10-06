//! ContinentalV6: a short, resampled rigid-plate history on a fixed sphere.
//! All state below is generation scratch, never mutable world or replay state.

use super::{Field, Wind, move_on_sphere, prevailing_wind};
use crate::geography::{Terrain, coast_distances, random_direction};
use crate::math;
use crate::rng::{index, key, stream};
use crate::sphere::{self, Point, cross, dot, tangent, unit};
use rand::Rng;
use std::f64::consts::PI;
use std::sync::LazyLock;

const LEVEL: u32 = 5;
const STEPS: u8 = 10;
const OCEAN: f64 = 7.0;

type Surface = (Vec<Terrain>, Vec<f64>, Vec<f64>);

#[derive(Clone, Copy)]
struct Crust {
    plate: usize,
    continental: bool,
    thickness: f64,
    age: u8,
    uplift: f64,
    last_uplift: u8,
    margin: bool,
    hotspot: bool,
}

#[derive(Clone, Copy)]
struct Rotation {
    axis: Point,
    sin: f64,
    cos: f64,
}

impl Rotation {
    fn new(velocity: Point) -> Self {
        let angle = dot(velocity, velocity).sqrt();
        Self {
            axis: if angle > 0.0 {
                velocity.map(|v| v / angle)
            } else {
                [0.0, 0.0, 1.0]
            },
            sin: math::sin(angle),
            cos: math::cos(angle),
        }
    }

    fn apply(self, p: Point, inverse: bool) -> Point {
        let s = if inverse { -self.sin } else { self.sin };
        let along = dot(self.axis, p) * (1.0 - self.cos);
        let around = cross(self.axis, p);
        std::array::from_fn(|i| p[i] * self.cos + around[i] * s + self.axis[i] * along)
    }
}

struct Plate {
    centre: Point,
    velocity: Point,
    weight: f64,
    group: usize,
    birth_age: u8,
}

/// The convex icosphere's Delaunay graph is a deterministic spatial index.
/// Greedy dot-product ascent ends at the nearest spherical Voronoi cell.
/// A local inverse rotation starts at its destination, not a global scan.
pub(crate) fn nearest(mesh: &sphere::Mesh, p: Point, start: usize) -> usize {
    let mut at = start;
    loop {
        let mut best = at;
        let mut score = dot(mesh.cells[at].position, p);
        for &n in &mesh.cells[at].neighbours {
            let next = dot(mesh.cells[n].position, p);
            if next.total_cmp(&score).then(best.cmp(&n)).is_gt() {
                best = n;
                score = next;
            }
        }
        if best == at {
            return at;
        }
        at = best;
    }
}

fn simulation_mesh() -> &'static sphere::Mesh {
    static MESH: LazyLock<sphere::Mesh> = LazyLock::new(|| sphere::mesh(LEVEL));
    &MESH
}

pub(crate) struct Evidence {
    pub collision: Vec<f64>,
    pub margin: Vec<bool>,
    pub hotspot: Vec<bool>,
    pub sutures: usize,
    pub boundaries: Vec<[Point; 2]>,
    pub initial_coast: Vec<[Point; 2]>,
}

struct History {
    crust: Vec<Crust>,
    initial: Vec<bool>,
    sutures: usize,
    detail: Field,
    wet: Field,
    land_share: f64,
}

impl History {
    fn new(seed: u64) -> Self {
        let mesh = simulation_mesh();
        let mut rng = stream(seed, &[key("continental-v6 supercontinent")]);
        let centre = random_direction(&mut rng);
        let outline = Field::new(&mut rng, 0.85);
        let warp = Field::new(&mut rng, 0.65);
        let (east, north) = tangent(centre);
        let crust_share = rng.gen_range(0.38..0.42);
        let mut field: Vec<_> = mesh
            .cells
            .iter()
            .map(|c| {
                let p = c.position;
                let warped = unit(std::array::from_fn(|i| p[i] + 0.28 * warp.at(p) * east[i]));
                dot(warped, centre) + 0.26 * outline.at(warped)
            })
            .collect();
        let areas: Vec<_> = mesh
            .cells
            .iter()
            .map(|c| sphere::area(c.position, &c.boundary))
            .collect();
        let level = sea_level(&field, &areas, crust_share);
        for value in &mut field {
            *value -= level;
        }
        let mut initial = vec![false; mesh.cells.len()];
        let root = nearest(mesh, centre, 0);
        let mut frontier = vec![root];
        initial[root] = true;
        while let Some(r) = frontier.pop() {
            for &n in &mesh.cells[r].neighbours {
                if !initial[n] && field[n] > 0.0 {
                    initial[n] = true;
                    frontier.push(n);
                }
            }
        }

        let mut rng = stream(seed, &[key("continental-v6 plate partition")]);
        let count = 9 + index(&mut rng, 4);
        let turn = rng.gen_range(-PI..PI);
        let expansion = stream(seed, &[key("continental-v6 rift motion")]).gen_range(0.13..0.15);
        let mut age_rng = stream(seed, &[key("continental-v6 lithosphere age")]);
        let mut plates: Vec<Plate> = Vec::with_capacity(count);
        // Four principal rift provinces and smaller marginal plates. A power
        // law in their additive Voronoi weights makes their areas unequal.
        for id in 0..count {
            let p = if id < 4 {
                let bearing = turn + id as f64 * (2.0 * PI / 3.0);
                let direction = std::array::from_fn(|i| {
                    east[i] * math::cos(bearing) + north[i] * math::sin(bearing)
                });
                move_on_sphere(
                    centre,
                    direction,
                    if id == 0 {
                        0.20
                    } else {
                        rng.gen_range(0.85..1.10)
                    },
                )
            } else if id < 6 {
                let bearing = turn + rng.gen_range(-PI..PI);
                let direction = std::array::from_fn(|i| {
                    east[i] * math::cos(bearing) + north[i] * math::sin(bearing)
                });
                move_on_sphere(
                    centre,
                    direction,
                    rng.gen_range(if id == 4 { 1.25..1.65 } else { 1.60..1.90 }),
                )
            } else {
                // A fixed candidate set, never rejection sampling a map.
                (0..48)
                    .map(|candidate| {
                        let p = random_direction(&mut rng);
                        let score = plates
                            .iter()
                            .map(|plate| 1.0 - dot(p, plate.centre))
                            .fold(f64::INFINITY, f64::min);
                        (candidate, p, score)
                    })
                    .max_by(|a, b| a.2.total_cmp(&b.2).then(b.0.cmp(&a.0)))
                    .unwrap()
                    .1
            };
            let spin = rng.gen_range(-0.008..0.008);
            let pole = cross(centre, p);
            plates.push(Plate {
                centre: p,
                velocity: std::array::from_fn(|i| pole[i] * expansion + p[i] * spin),
                weight: 0.13 / ((id + 1) * (id + 1)) as f64,
                group: id,
                birth_age: age_rng.gen_range(8..21),
            });
        }
        let boundary_warp = Field::new(&mut rng, 1.7);
        let owners: Vec<_> = mesh
            .cells
            .iter()
            .map(|c| {
                let p = c.position;
                let q = unit(std::array::from_fn(|i| {
                    p[i] + 0.12 * boundary_warp.at(p) * north[i] + 0.08 * outline.at(p) * east[i]
                }));
                plates
                    .iter()
                    .enumerate()
                    .max_by(|(a, pa), (b, pb)| {
                        (dot(q, pa.centre) + pa.weight)
                            .total_cmp(&(dot(q, pb.centre) + pb.weight))
                            .then(b.cmp(a))
                    })
                    .unwrap()
                    .0
            })
            .collect();
        let mut fraction = vec![(0.0, 0.0); count];
        for (r, &owner) in owners.iter().enumerate() {
            fraction[owner].1 += areas[r];
            if initial[r] {
                fraction[owner].0 += areas[r];
            }
        }
        for (plate, &(land, area)) in plates.iter_mut().zip(&fraction) {
            let drag = 1.0 - 0.22 * land / area.max(1e-12);
            plate.velocity = plate.velocity.map(|v| v * drag);
        }
        // The dominant rift follows one inherited weak suture. A second
        // crosses the interior; both erode and travel with their own crust.
        let old_pole = random_direction(&mut rng);
        let old_pole = unit(std::array::from_fn(|i| {
            old_pole[i] - centre[i] * dot(old_pole, centre)
        }));
        let mut crust: Vec<_> = owners
            .iter()
            .enumerate()
            .map(|(r, &plate)| {
                let continental = initial[r];
                let p = mesh.cells[r].position;
                let interior = dot(p, old_pole).abs();
                let rift = (dot(p, plates[0].centre) + plates[0].weight
                    - dot(p, plates[1].centre)
                    - plates[1].weight)
                    .abs();
                let old_range = if continental {
                    0.40 * math::exp64(-interior * interior / 0.008)
                        + 0.12 * math::exp64(-rift * rift / 0.004)
                } else {
                    0.0
                };
                Crust {
                    plate,
                    continental,
                    thickness: if continental {
                        // Marginal provinces inherit stretched, shallower roots.
                        25.0 + 12.0 * field[r].max(0.0).sqrt()
                            - 6.0 * (1.0 - fraction[plate].0 / fraction[plate].1.max(1e-12))
                    } else {
                        OCEAN
                    },
                    age: plates[plate].birth_age,
                    uplift: old_range,
                    last_uplift: 0,
                    margin: old_range > 0.035,
                    hotspot: false,
                }
            })
            .collect();
        let mut rng = stream(seed, &[key("continental-v6 mantle hotspots")]);
        let mut hotspots: Vec<Point> = Vec::with_capacity(3);
        for _ in 0..3 {
            let cost = |p| {
                (dot(p, centre) + 0.45).abs()
                    + hotspots
                        .iter()
                        .map(|&h| (dot(p, h) - 0.25).max(0.0))
                        .sum::<f64>()
            };
            let hotspot = (0..24)
                .map(|id| (id, random_direction(&mut rng)))
                .min_by(|a, b| cost(a.1).total_cmp(&cost(b.1)).then(a.0.cmp(&b.0)))
                .unwrap()
                .1;
            hotspots.push(hotspot);
        }
        let volcanism = Field::new(
            &mut stream(seed, &[key("continental-v6 arc volcanism")]),
            5.0,
        );
        let volcanic: Vec<_> = mesh
            .cells
            .iter()
            .map(|c| volcanism.at(c.position))
            .collect();
        let mut next = crust.clone();
        let mut sutures = 2;
        let mut pairs = vec![false; count * count];
        let mut uplift = vec![0.0_f64; crust.len()];
        let mut rotations = vec![Rotation::new([0.0; 3]); count];
        for step in 1..=STEPS {
            // A marginal fragment returns as the spreading system reorganizes.
            // Its later overlap is resolved by the same collision and welding
            // rules, never by choosing a desired final continental outline.
            if step == 6 && plates[4].group == 4 {
                let returning = plates[4].velocity.map(|v| -1.3 * v);
                for plate in &mut plates {
                    if plate.group == 4 {
                        plate.velocity = returning;
                    }
                }
            }
            for (rotation, plate) in rotations.iter_mut().zip(&plates) {
                *rotation = Rotation::new(plate.velocity);
            }
            pairs.fill(false);
            uplift.fill(0.0);
            for (r, cell) in mesh.cells.iter().enumerate() {
                let mut chosen: Option<Crust> = None;
                for (id, rotation) in rotations.iter().enumerate() {
                    let source = nearest(mesh, rotation.apply(cell.position, true), r);
                    let incoming = crust[source];
                    if incoming.plate != id {
                        continue;
                    }
                    chosen = Some(match chosen {
                        None => incoming,
                        Some(mut held) => {
                            if incoming.continental && held.continental {
                                let a = &plates[held.plate];
                                let b = &plates[id];
                                let normal =
                                    unit(std::array::from_fn(|i| b.centre[i] - a.centre[i]));
                                let relative = cross(
                                    std::array::from_fn(|i| a.velocity[i] - b.velocity[i]),
                                    cell.position,
                                );
                                if a.group != b.group && dot(relative, normal) > 0.008 {
                                    pairs[held.plate.min(id) * count + held.plate.max(id)] = true;
                                    held.thickness += 0.30 * (incoming.thickness - 22.0).max(0.0);
                                    held.uplift += 0.22;
                                    held.last_uplift = step;
                                    held.margin = true;
                                }
                                held
                            } else if incoming.continental
                                || !held.continental
                                    && plates[id].birth_age < plates[held.plate].birth_age
                            {
                                incoming
                            } else {
                                held
                            }
                        }
                    });
                }
                let mut arrived = chosen.unwrap_or_else(|| {
                    let plate = plates
                        .iter()
                        .enumerate()
                        .max_by(|(a, pa), (b, pb)| {
                            dot(cell.position, pa.centre)
                                .total_cmp(&dot(cell.position, pb.centre))
                                .then(b.cmp(a))
                        })
                        .unwrap()
                        .0;
                    Crust {
                        plate,
                        continental: false,
                        thickness: OCEAN,
                        age: 0,
                        uplift: 0.0,
                        last_uplift: step,
                        margin: false,
                        hotspot: false,
                    }
                });
                if chosen.is_some() {
                    arrived.age = arrived.age.saturating_add(1);
                }
                arrived.uplift *= 0.95;
                next[r] = arrived;
            }
            // Convergent contacts leave a trench on the subducting side and
            // relief on the overriding side. Arc magmatism has separated vents.
            for border in &mesh.borders {
                let (a, b) = (border.a, border.b);
                let (ca, cb) = (next[a], next[b]);
                let (pa, pb) = (&plates[ca.plate], &plates[cb.plate]);
                if pa.group == pb.group {
                    continue;
                }
                let normal = unit(std::array::from_fn(|i| pb.centre[i] - pa.centre[i]));
                let relative = cross(
                    std::array::from_fn(|i| pa.velocity[i] - pb.velocity[i]),
                    border.midpoint,
                );
                let compression = dot(relative, normal);
                if compression < -0.02 {
                    // Stretching lowers the conjugate continental shelves.
                    // The opened gap itself receives zero-age ocean above.
                    for r in [a, b] {
                        if next[r].continental {
                            next[r].thickness = (next[r].thickness - 0.20).max(20.0);
                        }
                    }
                }
                if compression <= 0.012 {
                    continue;
                }
                if ca.continental && cb.continental {
                    pairs[ca.plate.min(cb.plate) * count + ca.plate.max(cb.plate)] = true;
                    for r in [a, b] {
                        uplift[r] = uplift[r].max(0.30);
                    }
                } else {
                    let over = if ca.continental
                        || !cb.continental && (pa.birth_age, ca.plate) < (pb.birth_age, cb.plate)
                    {
                        a
                    } else {
                        b
                    };
                    let under = if over == a { b } else { a };
                    next[under].thickness = (next[under].thickness - 0.5).max(4.0);
                    if next[over].continental {
                        uplift[over] = uplift[over].max(0.20 + compression);
                    } else {
                        let magma = (30.0 * (volcanic[over] - 0.12).max(0.0)).min(8.0);
                        next[over].thickness = (next[over].thickness + magma).min(34.0);
                        if magma > 0.0 {
                            next[over].age = 0;
                            uplift[over] = uplift[over].max(0.13);
                        }
                    }
                }
            }
            for (r, &amount) in uplift.iter().enumerate() {
                if amount == 0.0 {
                    continue;
                }
                for (at, strength) in std::iter::once((r, amount))
                    .chain(mesh.cells[r].neighbours.iter().map(|&n| (n, amount * 0.55)))
                {
                    if next[at].continental || at == r {
                        next[at].uplift = next[at].uplift.max(strength);
                        next[at].last_uplift = step;
                        next[at].margin = true;
                    }
                }
            }
            for a in 0..count {
                for b in a + 1..count {
                    if !pairs[a * count + b] || plates[a].group == plates[b].group {
                        continue;
                    }
                    let (ga, gb) = (plates[a].group, plates[b].group);
                    let velocity = std::array::from_fn(|i| {
                        (plates[a].velocity[i] + plates[b].velocity[i]) * 0.5
                    });
                    for plate in &mut plates {
                        if plate.group == ga || plate.group == gb {
                            plate.group = ga.min(gb);
                            plate.velocity = velocity;
                        }
                    }
                    sutures += 1;
                }
            }
            for &hotspot in &hotspots {
                let r = nearest(mesh, hotspot, 0);
                if !next[r].continental {
                    next[r].thickness = next[r].thickness.max(42.0);
                    next[r].age = 0;
                    next[r].hotspot = true;
                    next[r].uplift = next[r].uplift.max(0.08);
                    next[r].last_uplift = step;
                    for &n in &mesh.cells[r].neighbours {
                        if !next[n].continental {
                            next[n].thickness = next[n].thickness.max(33.0);
                            next[n].age = 0;
                            next[n].hotspot = true;
                        }
                    }
                }
            }
            for (plate, &rotation) in plates.iter_mut().zip(&rotations) {
                plate.centre = rotation.apply(plate.centre, false);
            }
            std::mem::swap(&mut crust, &mut next);
        }
        Self {
            crust,
            initial,
            sutures,
            detail: Field::new(
                &mut stream(seed, &[key("continental-v6 coastal detail")]),
                8.0,
            ),
            wet: Field::new(&mut stream(seed, &[key("continental-v6 moisture")]), 1.0),
            land_share: stream(seed, &[key("continental-v6 sea level")]).gen_range(0.296..0.308),
        }
    }

    fn elevation(&self, r: usize) -> f64 {
        let c = self.crust[r];
        let erosion = math::exp64(-0.045 * f64::from(STEPS - c.last_uplift));
        let isostasy = if c.continental {
            (c.thickness - 26.0) * 0.016
        } else {
            (c.thickness - 32.0) * 0.035 - 0.025 * f64::from(c.age).sqrt()
        };
        isostasy + c.uplift * erosion + 0.012 * self.detail.at(simulation_mesh().cells[r].position)
    }
}

fn sea_level(values: &[f64], areas: &[f64], fraction: f64) -> f64 {
    let mut rank: Vec<_> = (0..values.len()).collect();
    rank.sort_by(|&a, &b| values[b].total_cmp(&values[a]).then(a.cmp(&b)));
    let target = 4.0 * PI * fraction;
    let mut area = 0.0;
    for pair in rank.windows(2) {
        area += areas[pair[0]];
        if area >= target {
            return (values[pair[0]] + values[pair[1]]) * 0.5;
        }
    }
    values[*rank.last().unwrap()]
}

pub(crate) fn surface(seed: u64, mesh: &mut sphere::Mesh, radius_km: f32) -> Surface {
    let history = History::new(seed);
    draw(&history, mesh, radius_km)
}

pub(crate) fn diagnosed_surface(
    seed: u64,
    mesh: &mut sphere::Mesh,
    radius_km: f32,
) -> (Surface, Evidence) {
    let history = History::new(seed);
    let surface = draw(&history, mesh, radius_km);
    let sim = simulation_mesh();
    let segment = |border: &sphere::Border| {
        let mut points = sim.cells[border.a]
            .boundary
            .iter()
            .copied()
            .filter(|p| sim.cells[border.b].boundary.contains(p));
        [points.next().unwrap(), points.next().unwrap()]
    };
    let evidence = Evidence {
        collision: history.crust[..mesh.cells.len()]
            .iter()
            .map(|c| c.uplift)
            .collect(),
        margin: history.crust[..mesh.cells.len()]
            .iter()
            .map(|c| c.margin)
            .collect(),
        hotspot: history.crust[..mesh.cells.len()]
            .iter()
            .map(|c| c.hotspot)
            .collect(),
        sutures: history.sutures,
        boundaries: sim
            .borders
            .iter()
            .filter(|b| history.crust[b.a].plate != history.crust[b.b].plate)
            .map(segment)
            .collect(),
        initial_coast: sim
            .borders
            .iter()
            .filter(|b| history.initial[b.a] != history.initial[b.b])
            .map(segment)
            .collect(),
    };
    (surface, evidence)
}

fn draw(history: &History, mesh: &mut sphere::Mesh, radius_km: f32) -> Surface {
    // Icosahedral subdivisions append points; all regional centres are an
    // exact subset of the fixed simulation grid, without another resampling.
    let mut values: Vec<_> = (0..mesh.cells.len())
        .map(|r| history.elevation(r))
        .collect();
    let areas: Vec<_> = mesh
        .cells
        .iter()
        .map(|c| sphere::area(c.position, &c.boundary))
        .collect();
    let level = sea_level(&values, &areas, history.land_share);
    // A sea one region wide with land all round is a basin below the map's
    // resolution: rivers and lakes carry inland water at that scale. It
    // rises to its lowest neighbour. Seas of two or more regions remain.
    let holes: Vec<_> = (0..values.len())
        .filter(|&r| {
            values[r] <= level && mesh.cells[r].neighbours.iter().all(|&n| values[n] > level)
        })
        .collect();
    for r in holes {
        values[r] = mesh.cells[r]
            .neighbours
            .iter()
            .map(|&n| values[n])
            .fold(f64::INFINITY, f64::min);
    }
    let shore: Vec<_> = values
        .iter()
        .map(|v| (v - level).clamp(-0.015, 0.015))
        .collect();
    let mut terrain: Vec<_> = shore
        .iter()
        .map(|&v| {
            if v > 0.0 {
                Terrain::Plains
            } else {
                Terrain::Sea
            }
        })
        .collect();
    let elevation: Vec<_> = values
        .iter()
        .map(|v| (0.15 + v - level).clamp(0.0, 1.0))
        .collect();
    sphere::refine_coasts(mesh, &shore, |p| 0.20 * history.detail.at(p));
    let coast_distance = coast_distances(&mesh.cells, &terrain, radius_km);
    let moisture: Vec<_> = mesh
        .cells
        .iter()
        .enumerate()
        .map(|(r, cell)| {
            let p = cell.position;
            let latitude = p[2].abs();
            let equatorial = math::exp64(-latitude * latitude / 0.045);
            let subtropical = math::exp64(-((latitude - 0.48) / 0.17) * ((latitude - 0.48) / 0.17));
            let wind = prevailing_wind(p, Wind::GeographicEast);
            let mut shadow: f64 = 0.0;
            let mut uplift: f64 = 0.0;
            for &n in &cell.neighbours {
                if -dot(mesh.cells[n].position, wind) > 0.0 {
                    let gradient = elevation[n] - elevation[r];
                    shadow = shadow.max(gradient);
                    uplift = uplift.max(-gradient);
                }
            }
            (0.42 + 0.29 * math::exp64(-coast_distance[r] / 750.0) + 0.22 * equatorial
                - 0.23 * subtropical
                - 0.14 * elevation[r]
                - 0.38 * shadow
                + 0.20 * uplift
                + 0.24 * history.wet.at(p))
            .clamp(0.0, 1.0)
        })
        .collect();
    let mut land: Vec<_> = (0..terrain.len())
        .filter(|&r| terrain[r].is_land())
        .collect();
    land.sort_by(|&a, &b| elevation[a].total_cmp(&elevation[b]).then(a.cmp(&b)));
    let mountains = land.len() * 10 / 100;
    let hills = land.len() * 17 / 100;
    for (rank, &r) in land.iter().enumerate() {
        terrain[r] = if rank >= land.len() - mountains {
            Terrain::Mountains
        } else if rank >= land.len() - mountains - hills {
            Terrain::Hills
        } else if moisture[r] < 0.30 {
            Terrain::Desert
        } else if moisture[r] < 0.46 {
            Terrain::Steppe
        } else if moisture[r] > 0.65 {
            Terrain::Forest
        } else {
            Terrain::Plains
        };
    }
    (terrain, elevation, moisture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spatial_index_matches_exhaustive_nearest_and_rotations_are_rigid() {
        let mesh = simulation_mesh();
        assert_eq!(mesh.cells.len(), 10_242);
        let mut rng = stream(7, &[key("continental-v6 spatial index test")]);
        for _ in 0..256 {
            let p = random_direction(&mut rng);
            let at = nearest(mesh, p, index(&mut rng, mesh.cells.len()));
            let brute = mesh
                .cells
                .iter()
                .enumerate()
                .max_by(|(a, ca), (b, cb)| {
                    dot(ca.position, p)
                        .total_cmp(&dot(cb.position, p))
                        .then(b.cmp(a))
                })
                .unwrap()
                .0;
            assert_eq!(at, brute);
            let rotation = Rotation::new(random_direction(&mut rng).map(|v| v * 0.15));
            let moved = rotation.apply(p, false);
            assert!((dot(moved, moved) - 1.0).abs() < 1e-12);
            let restored = rotation.apply(moved, true);
            assert!(sphere::angle(p, restored) < 1e-12);
        }
        for level in [3, 4] {
            let region_mesh = sphere::mesh(level);
            for (region, simulation) in region_mesh.cells.iter().zip(&mesh.cells) {
                assert_eq!(region.position, simulation.position);
            }
        }
    }

    #[test]
    fn history_preserves_a_supercontinents_crust_and_uplift_ages() {
        let mesh = simulation_mesh();
        let mut ratios = Vec::new();
        for seed in 0..16 {
            let history = History::new(seed);
            let mut initial_area = 0.0;
            let mut final_area = 0.0;
            for (r, cell) in mesh.cells.iter().enumerate() {
                let area = sphere::area(cell.position, &cell.boundary);
                if history.initial[r] {
                    initial_area += area;
                }
                if history.crust[r].continental {
                    final_area += area;
                }
                assert!(history.crust[r].last_uplift <= STEPS);
                assert!(history.elevation(r).is_finite());
            }
            assert!((0.35..=0.421).contains(&(initial_area / (4.0 * PI))));
            let ratio = final_area / initial_area;
            assert!(
                (0.8..=1.2).contains(&ratio),
                "seed {seed}: continental area ratio {ratio}"
            );
            ratios.push(ratio);
            let root = history.initial.iter().position(|&v| v).unwrap();
            let mut seen = vec![false; mesh.cells.len()];
            seen[root] = true;
            let mut stack = vec![root];
            while let Some(r) = stack.pop() {
                for &n in &mesh.cells[r].neighbours {
                    if history.initial[n] && !seen[n] {
                        seen[n] = true;
                        stack.push(n);
                    }
                }
            }
            assert_eq!(seen, history.initial);
        }
        println!("V6 continental-area retention, seeds 0..16: {ratios:?}");
    }

    #[test]
    fn sampled_surface_retains_ranked_relief_and_is_seed_repeatable() {
        let mut a = sphere::mesh(3);
        let mut b = sphere::mesh(3);
        let first = surface(21, &mut a, 800.0);
        let second = surface(21, &mut b, 800.0);
        assert_eq!(first, second);
        let land = first.0.iter().filter(|t| t.is_land()).count();
        assert_eq!(
            first.0.iter().filter(|&&t| t == Terrain::Mountains).count(),
            land * 10 / 100
        );
        assert_eq!(
            first.0.iter().filter(|&&t| t == Terrain::Hills).count(),
            land * 17 / 100
        );
        for (a, b) in a.cells.iter().zip(&b.cells) {
            assert_eq!(a.boundary, b.boundary);
        }
    }
}

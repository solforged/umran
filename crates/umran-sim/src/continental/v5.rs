//! ContinentalV5: area-balanced cratons with plate-margin relief.
//! Kept separate from the immutable V2–V4 fields and purpose streams.

use super::{Plate, Wind, move_on_sphere, prevailing_wind};
use crate::geography::{Terrain, coast_distances, random_direction};
use crate::math;
use crate::rng::{index, key, stream};
use crate::sphere::{self, Point, cross, dot, tangent};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::f64::consts::PI;

// Amplitudes fall faster than frequency^-1.2. These are a few broad bays,
// not near-equal noise at every scale. Their squared mean normalizes area.
const FREQUENCIES: [f64; 5] = [2.0, 3.0, 5.0, 8.0, 13.0];
const AMPLITUDES: [f64; 5] = [0.13, 0.075, 0.035, 0.017, 0.008];
const ISLAND_CRUST_SHARE: f64 = 0.065;
const MAX_CONTINENTS: usize = 6;

struct Crust {
    centre: Point,
    along: Point,
    across: Point,
    radius: f64,
    stretch: f64,
    share: f64,
    outline: [f64; 5],
    normalization: f64,
    phases: [f64; 5],
}

impl Crust {
    fn thickness(&self, p: Point) -> f64 {
        let facing = dot(p, self.centre).clamp(-1.0, 1.0);
        if facing < -0.5 {
            return -2.0;
        }
        // Lambert azimuthal equal-area projection: a disk of radius r has
        // solid angle pi*r*r. The elliptic transform has determinant one.
        let scale = (2.0 / (1.0 + facing)).sqrt();
        let x = scale * dot(p, self.along) / self.stretch;
        let y = scale * dot(p, self.across) * self.stretch;
        let bearing = math::atan2(y, x);
        let mut coast = 1.0;
        for (i, frequency) in FREQUENCIES.iter().enumerate() {
            coast += self.outline[i] * math::sin(frequency * bearing + self.phases[i]);
        }
        1.0 - (x * x + y * y).sqrt() * self.normalization / (self.radius * coast)
    }
}

struct Field([(Point, f64); 5]);

impl Field {
    fn new(rng: &mut ChaCha8Rng) -> Self {
        Self(std::array::from_fn(|_| {
            (random_direction(rng), rng.gen_range(-PI..PI))
        }))
    }

    fn at(&self, p: Point, scale: f64) -> f64 {
        self.0
            .iter()
            .enumerate()
            .map(|(i, &(axis, phase))| {
                AMPLITUDES[i] * math::sin(dot(p, axis) * FREQUENCIES[i] * scale + phase)
            })
            .sum::<f64>()
            / 0.266
    }
}

pub(super) struct Model {
    crust: Vec<Crust>,
    plates: Vec<Plate>,
    continental: Vec<bool>,
    relief: Field,
    wet: Field,
    detail: Field,
    islands: Field,
    land_share: f64,
}

impl Model {
    pub(super) fn new(seed: u64, small: bool) -> Self {
        let mut rng = stream(seed, &[key("continental-v5 cratons")]);
        let land_share: f64 = rng.gen_range(0.285..0.315);
        let mut outline_rng = stream(seed, &[key("continental-v5 continental structure")]);
        // Coarse worlds need enough cells per body to resolve their gulfs.
        let count =
            [3, 4, 4, 4, 5, 5, 5, 6][index(&mut outline_rng, 8)].min(if small { 4 } else { 6 });
        let mut weights = match count {
            3 => [0.55, 0.30, 0.15, 0.0, 0.0, 0.0],
            4 if small => [0.48, 0.28, 0.14, 0.10, 0.0, 0.0],
            4 => [0.51, 0.28, 0.14, 0.07, 0.0, 0.0],
            5 => [0.47, 0.26, 0.13, 0.07, 0.07, 0.0],
            6 => [0.40, 0.27, 0.13, 0.067, 0.067, 0.066],
            _ => unreachable!(),
        };
        let shift = rng.gen_range(-0.006..0.006);
        weights[0] += shift;
        weights[1] -= shift;
        let mut crust: Vec<Crust> = Vec::with_capacity(count);
        for (i, weight) in weights.into_iter().take(count).enumerate() {
            let centre = if i == 0 {
                random_direction(&mut rng)
            } else if i == 1 {
                let (east, north) = tangent(crust[0].centre);
                let turn = rng.gen_range(-PI..PI);
                let direction =
                    std::array::from_fn(|j| east[j] * math::cos(turn) + north[j] * math::sin(turn));
                move_on_sphere(crust[0].centre, direction, rng.gen_range(1.85..2.15))
            } else {
                // Place inherited cratons in the remaining ocean, accounting
                // for unequal sizes, rather than assembling chains of lobes.
                (0..96)
                    .map(|_| random_direction(&mut rng))
                    .max_by(|&a, &b| {
                        let clearance = |p| {
                            crust
                                .iter()
                                .map(|c| sphere::angle(p, c.centre) - c.radius * c.stretch * 1.3)
                                .fold(f64::INFINITY, f64::min)
                        };
                        clearance(a).total_cmp(&clearance(b))
                    })
                    .unwrap()
            };
            let (east, north) = tangent(centre);
            let turn = rng.gen_range(-PI..PI);
            let along =
                std::array::from_fn(|j| east[j] * math::cos(turn) + north[j] * math::sin(turn));
            // These broad second/third harmonics form gulfs and peninsulas.
            // Fine coast detail remains a separate, steeply falling spectrum.
            let outline = [
                outline_rng.gen_range(0.19..0.25),
                outline_rng.gen_range(if small { 0.60..0.64 } else { 0.525..0.575 }),
                0.025,
                0.010,
                0.004,
            ];
            let aspect: f64 = if small && i == 0 {
                outline_rng.gen_range(2.8..3.0)
            } else if i == 0 || small {
                outline_rng.gen_range(2.6..3.0)
            } else if i == 1 {
                outline_rng.gen_range(2.0..2.8)
            } else {
                outline_rng.gen_range(1.8..2.7)
            };
            crust.push(Crust {
                centre,
                along,
                across: cross(centre, along),
                radius: (4.0 * land_share * weight).sqrt(),
                stretch: aspect.sqrt(),
                share: weight,
                normalization: (1.0 + 0.5 * outline.iter().map(|a| a * a).sum::<f64>()).sqrt(),
                outline,
                phases: std::array::from_fn(|_| rng.gen_range(-PI..PI)),
            });
        }
        let mut rng = stream(seed, &[key("continental-v5 plate domains")]);
        let mut plates = Vec::with_capacity(count * 2 + 6);
        let mut continental = Vec::with_capacity(count * 2 + 6);
        for c in &crust {
            // Two inherited domains under each body allow interior sutures,
            // while oceanic neighbours create coastal subduction belts.
            for offset in [-0.22, 0.22] {
                let speed = rng.gen_range(0.45..0.9);
                plates.push(Plate {
                    centre: move_on_sphere(c.centre, c.along, c.radius * offset),
                    rotation: random_direction(&mut rng).map(|v| v * speed),
                    weight: 0.12,
                });
                continental.push(true);
            }
        }
        for _ in 0..6 {
            let centre = (0..48)
                .map(|_| random_direction(&mut rng))
                .max_by(|&a, &b| {
                    let clearance = |p| {
                        plates
                            .iter()
                            .map(|plate| 1.0 - dot(p, plate.centre))
                            .fold(f64::INFINITY, f64::min)
                    };
                    clearance(a).total_cmp(&clearance(b))
                })
                .unwrap();
            let speed = rng.gen_range(0.45..0.9);
            plates.push(Plate {
                centre,
                rotation: random_direction(&mut rng).map(|v| v * speed),
                weight: 0.0,
            });
            continental.push(false);
        }
        Self {
            crust,
            plates,
            continental,
            relief: Field::new(&mut stream(seed, &[key("continental-v5 erosion")])),
            wet: Field::new(&mut stream(seed, &[key("continental-v5 moisture")])),
            detail: Field::new(&mut stream(seed, &[key("continental-v5 shore detail")])),
            islands: Field::new(&mut stream(seed, &[key("continental-v5 shelf islands")])),
            land_share,
        }
    }

    fn crust(&self, p: Point) -> (usize, f64) {
        self.crust
            .iter()
            .enumerate()
            .map(|(id, c)| (id, c.thickness(p)))
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
            .unwrap()
    }

    pub(super) fn stress(&self, p: Point) -> (f64, f64, bool) {
        let mut first = (0, f64::NEG_INFINITY);
        let mut second = (0, f64::NEG_INFINITY);
        for (id, plate) in self.plates.iter().enumerate() {
            let score = dot(p, plate.centre) + plate.weight;
            if score > first.1 {
                second = first;
                first = (id, score);
            } else if score > second.1 {
                second = (id, score);
            }
        }
        let a = &self.plates[first.0];
        let b = &self.plates[second.0];
        let difference: Point = std::array::from_fn(|i| b.centre[i] - a.centre[i]);
        let length = dot(difference, difference).sqrt();
        let normal = difference.map(|v| v / length);
        let relative = cross(std::array::from_fn(|i| a.rotation[i] - b.rotation[i]), p);
        let convergence = dot(relative, normal);
        let distance = (first.1 - second.1) / length;
        let band = math::exp64(-(distance / 0.09) * (distance / 0.09));
        (
            band * convergence.max(0.0),
            band * (-convergence).max(0.0),
            !self.continental[first.0] && !self.continental[second.0],
        )
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Emergence {
    thickness: f64,
    region: usize,
}

impl Eq for Emergence {}

impl Ord for Emergence {
    fn cmp(&self, other: &Self) -> Ordering {
        self.thickness
            .total_cmp(&other.thickness)
            .then(other.region.cmp(&self.region))
    }
}

impl PartialOrd for Emergence {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Grow each inherited body continuously from its thickest crust. Area
/// budgets preserve unequal sizes; flooding between provinces leaves straits.
fn continents(
    mesh: &sphere::Mesh,
    model: &Model,
    owners: &[usize],
    area: &[f64],
    shore: &mut [f64],
) -> Vec<Terrain> {
    let mut roots = [None; MAX_CONTINENTS];
    for (r, &owner) in owners.iter().enumerate() {
        if roots[owner].is_none_or(|previous| shore[r] > shore[previous]) {
            roots[owner] = Some(r);
        }
    }
    let mut terrain = vec![Terrain::Sea; mesh.cells.len()];
    let mut queued = vec![false; mesh.cells.len()];
    let mut frontier = BinaryHeap::new();
    for root in roots.iter().take(model.crust.len()) {
        let region = root.expect("each craton has a crust province");
        queued[region] = true;
        frontier.push(Emergence {
            thickness: shore[region],
            region,
        });
    }
    let target = 4.0 * PI * model.land_share * (1.0 - ISLAND_CRUST_SHARE);
    let mut occupied = [0.0; MAX_CONTINENTS];
    let mut level = [0.0; MAX_CONTINENTS];
    let mut filled = [false; MAX_CONTINENTS];
    let mut remaining = model.crust.len();
    while let Some(Emergence { region: r, .. }) = frontier.pop() {
        let owner = owners[r];
        if filled[owner]
            || mesh.cells[r]
                .neighbours
                .iter()
                .any(|&n| terrain[n].is_land() && owners[n] != owner)
        {
            continue;
        }
        if occupied[owner] + area[r] * 0.5 > target * model.crust[owner].share {
            filled[owner] = true;
            remaining -= 1;
            if remaining == 0 {
                break;
            }
            continue;
        }
        terrain[r] = Terrain::Plains;
        occupied[owner] += area[r];
        level[owner] = shore[r];
        for &n in &mesh.cells[r].neighbours {
            if !queued[n] && owners[n] == owner {
                queued[n] = true;
                frontier.push(Emergence {
                    thickness: shore[n],
                    region: n,
                });
            }
        }
    }
    for (r, value) in shore.iter_mut().enumerate() {
        *value -= level[owners[r]];
        *value = if terrain[r].is_land() {
            value.max(1e-12)
        } else {
            value.min(-1e-12)
        };
    }
    terrain
}

/// Emergent shelf fragments and volcanic arcs have their own crust budget.
/// Flooded straits separate them from continents and neighbouring islands.
fn islands(
    mesh: &sphere::Mesh,
    model: &Model,
    radius_km: f32,
    area: &[f64],
    oceanic_collision: &mut [f64],
    terrain: &mut [Terrain],
    shore: &mut [f64],
) {
    let mut candidates = Vec::new();
    for (r, cell) in mesh.cells.iter().enumerate() {
        if terrain[r].is_land() || cell.neighbours.iter().any(|&n| terrain[n].is_land()) {
            oceanic_collision[r] = 0.0;
            continue;
        }
        let shelf = cell.neighbours.iter().any(|&n| {
            mesh.cells[n]
                .neighbours
                .iter()
                .any(|&next| terrain[next].is_land())
        });
        let arc = oceanic_collision[r] >= 0.10;
        if shelf || arc {
            oceanic_collision[r] = oceanic_collision[r].max(if shelf { 0.25 } else { 0.0 })
                * (1.0 + 0.35 * model.islands.at(cell.position, 3.0));
            candidates.push(r);
        } else {
            oceanic_collision[r] = 0.0;
        }
    }
    candidates.sort_by(|&a, &b| {
        oceanic_collision[b]
            .total_cmp(&oceanic_collision[a])
            .then(a.cmp(&b))
    });
    let budget = 4.0 * PI * model.land_share * ISLAND_CRUST_SHARE;
    let radius = f64::from(radius_km);
    let cutoff = if radius_km < 1_000.0 {
        125_000.0
    } else {
        500_000.0
    };
    let minimum = if radius_km < 1_000.0 {
        2
    } else if radius_km > 6_000.0 {
        15
    } else {
        6
    };
    // Stay well below the physical continent threshold, including the small
    // deformation introduced by coast refinement.
    let count = minimum.max((budget * radius * radius / (0.45 * cutoff)) as usize + 1);
    let per_island = budget / (count + 2) as f64;
    let mut owners: Vec<_> = terrain
        .iter()
        .map(|t| if t.is_land() { 0 } else { usize::MAX })
        .collect();
    let mut fringe = VecDeque::new();
    let mut occupied = 0.0;
    let mut id = 0;
    for start in candidates {
        if occupied + area[start] * 0.5 > budget {
            break;
        }
        if owners[start] != usize::MAX
            || mesh.cells[start]
                .neighbours
                .iter()
                .any(|&n| owners[n] != usize::MAX)
        {
            continue;
        }
        id += 1;
        owners[start] = id;
        terrain[start] = Terrain::Plains;
        shore[start] = -shore[start];
        occupied += area[start];
        let mut grown = area[start];
        fringe.clear();
        fringe.push_back(start);
        while let Some(r) = fringe.pop_front() {
            for &n in &mesh.cells[r].neighbours {
                if oceanic_collision[n] <= 0.0
                    || owners[n] != usize::MAX
                    || grown + area[n] * 0.5 > per_island
                    || occupied + area[n] * 0.5 > budget
                    || mesh.cells[n]
                        .neighbours
                        .iter()
                        .any(|&next| owners[next] != usize::MAX && owners[next] != id)
                {
                    continue;
                }
                owners[n] = id;
                terrain[n] = Terrain::Plains;
                shore[n] = -shore[n];
                occupied += area[n];
                grown += area[n];
                fringe.push_back(n);
            }
        }
    }
}

pub(crate) fn surface(
    seed: u64,
    mesh: &mut sphere::Mesh,
    radius_km: f32,
) -> (Vec<Terrain>, Vec<f64>, Vec<f64>) {
    let model = Model::new(seed, radius_km < 1_000.0);
    let mut shore = Vec::with_capacity(mesh.cells.len());
    let mut elevation = Vec::with_capacity(mesh.cells.len());
    let mut area = Vec::with_capacity(mesh.cells.len());
    let mut oceanic_collision = Vec::with_capacity(mesh.cells.len());
    let mut owners = Vec::with_capacity(mesh.cells.len());
    for cell in &mesh.cells {
        let p = cell.position;
        let (owner, crust) = model.crust(p);
        owners.push(owner);
        let (collision, rift, oceanic) = model.stress(p);
        oceanic_collision.push(if oceanic { collision } else { 0.0 });
        shore.push(crust - 0.025 * rift);
        elevation.push(
            (0.14 + 0.07 * crust.max(0.0) + 0.72 * collision - 0.08 * rift
                + 0.035 * model.relief.at(p, 2.0))
            .clamp(0.0, 1.0),
        );
        area.push(sphere::area(p, &cell.boundary));
    }
    let mut terrain = continents(mesh, &model, &owners, &area, &mut shore);
    islands(
        mesh,
        &model,
        radius_km,
        &area,
        &mut oceanic_collision,
        &mut terrain,
        &mut shore,
    );
    sphere::refine_coasts(mesh, &shore, |p| model.detail.at(p, 9.0));
    let coast_distance = coast_distances(&mesh.cells, &terrain, radius_km);
    // Preserve V3's climate law and geographic wind; only input geography and
    // the new purpose-keyed moisture field differ. Drainage stays downstream.
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
                + 0.24 * model.wet.at(p, 1.0))
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

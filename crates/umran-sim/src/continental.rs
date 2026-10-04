//! ContinentalV2: assembled crust, divergent rifts, convergent belts and
//! maritime moisture on the sphere. A static tectonic analogue, not a plate
//! time simulation. Every field and stream is independent of SphericalV1.

use crate::geography::{Terrain, coast_distances, random_direction};
use crate::math;
use crate::rng::{index, key, stream};
use crate::sphere::{self, Point, cross, dot, tangent, unit};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::f64::consts::PI;

struct Wave {
    direction: Point,
    frequency: f64,
    phase: f64,
    amplitude: f64,
}

struct Field([Wave; 8]);

impl Field {
    fn new(rng: &mut ChaCha8Rng, scale: f64) -> Self {
        Self(std::array::from_fn(|i| Wave {
            direction: random_direction(rng),
            frequency: scale * [2.0, 3.7, 6.3, 11.0, 19.0, 37.0, 71.0, 137.0][i],
            phase: rng.gen_range(-PI..PI),
            amplitude: [0.28, 0.23, 0.18, 0.12, 0.08, 0.05, 0.035, 0.025][i],
        }))
    }

    fn at(&self, p: Point) -> f64 {
        self.0
            .iter()
            .map(|w| w.amplitude * math::sin(dot(p, w.direction) * w.frequency + w.phase))
            .sum()
    }
}

struct Craton {
    centre: Point,
    along: Point,
    across: Point,
    length: f64,
    width: f64,
    thickness: f64,
}

impl Craton {
    fn at(&self, p: Point) -> f64 {
        let facing = dot(p, self.centre);
        if facing <= 0.0 {
            return 0.0;
        }
        // Chord coordinates avoid a trigonometric transform for every lobe.
        // The facing term prevents an antipodal copy of the same continent.
        let x = dot(p, self.along) / self.length;
        let y = dot(p, self.across) / self.width;
        self.thickness * math::exp64(-1.7 * (x * x + y * y)) * facing
    }
}

struct Plate {
    centre: Point,
    rotation: Point,
    weight: f64,
}

struct Tectonics {
    crust: Vec<Craton>,
    plates: Vec<Plate>,
    coast: Field,
    relief: Field,
    wet: Field,
    detail: Field,
    land_share: f64,
}

fn move_on_sphere(p: Point, direction: Point, radians: f64) -> Point {
    let (s, c) = (math::sin(radians), math::cos(radians));
    unit(std::array::from_fn(|i| p[i] * c + direction[i] * s))
}

impl Tectonics {
    fn new(seed: u64) -> Self {
        let mut rng = stream(seed, &[key("continental-v2 crust assembly")]);
        let assemblies = 2 + index(&mut rng, 3);
        let mut crust = Vec::with_capacity(92);
        // Unequal branching assemblies may collide, overlap or lie far apart.
        // There is deliberately no farthest-point placement or forced gap.
        for assembly in 0..assemblies {
            let root = random_direction(&mut rng);
            let scale = if assembly == 0 {
                rng.gen_range(0.65..0.95)
            } else {
                rng.gen_range(0.30..0.70)
            };
            let branches = 2 + index(&mut rng, 3);
            let bearing = rng.gen_range(-PI..PI);
            for branch in 0..branches {
                let mut at = root;
                let mut turn = bearing + branch as f64 * rng.gen_range(1.1..2.4);
                let pieces = 2 + index(&mut rng, 4);
                let mut width = scale * rng.gen_range(0.36..0.65);
                for piece in 0..pieces {
                    let (east, north) = tangent(at);
                    let (s, c) = (math::sin(turn), math::cos(turn));
                    let along = std::array::from_fn(|i| east[i] * c + north[i] * s);
                    crust.push(Craton {
                        centre: at,
                        along,
                        across: cross(at, along),
                        length: width * rng.gen_range(1.1..1.9),
                        width,
                        thickness: rng.gen_range(0.65..1.15) / branches as f64,
                    });
                    at = move_on_sphere(at, along, width * rng.gen_range(0.8..1.5));
                    turn += rng.gen_range(-0.9..0.9);
                    width *= if piece == 0 {
                        0.9
                    } else {
                        rng.gen_range(0.65..0.95)
                    };
                }
            }
        }
        // A few inherited microcontinental fragments, not a uniform island grid.
        for _ in 0..(5 + index(&mut rng, 8)) {
            let centre = random_direction(&mut rng);
            let (along, across) = tangent(centre);
            crust.push(Craton {
                centre,
                along,
                across,
                length: rng.gen_range(0.07..0.20),
                width: rng.gen_range(0.045..0.12),
                thickness: rng.gen_range(0.20..0.55),
            });
        }
        let land_share = rng.gen_range(0.30..0.43);
        let mut rng = stream(seed, &[key("continental-v2 plates")]);
        let count = 10 + index(&mut rng, 6);
        let plates = (0..count)
            .map(|_| Plate {
                centre: random_direction(&mut rng),
                rotation: {
                    let axis = random_direction(&mut rng);
                    let speed = rng.gen_range(0.35..1.0);
                    axis.map(|v| v * speed)
                },
                weight: rng.gen_range(-0.12..0.12),
            })
            .collect();
        Self {
            crust,
            plates,
            coast: Field::new(
                &mut stream(seed, &[key("continental-v2 coastal relief")]),
                1.0,
            ),
            relief: Field::new(&mut stream(seed, &[key("continental-v2 erosion")]), 1.7),
            wet: Field::new(&mut stream(seed, &[key("continental-v2 moisture")]), 1.0),
            detail: Field::new(
                &mut stream(seed, &[key("continental-v2 shore detail")]),
                9.0,
            ),
            land_share,
        }
    }

    /// Thin boundary bands are compressive or extensional according to the
    /// relative surface velocities of the two closest weighted plate domains.
    fn stress(&self, p: Point) -> (f64, f64) {
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
        let scale = dot(difference, difference).sqrt();
        let normal = difference.map(|v| v / scale);
        let relative = cross(std::array::from_fn(|i| a.rotation[i] - b.rotation[i]), p);
        let convergence = dot(relative, normal);
        let distance = (first.1 - second.1) / scale;
        let band = math::exp64(-(distance / 0.065) * (distance / 0.065));
        (band * convergence.max(0.0), band * (-convergence).max(0.0))
    }
}

/// Which tangent the zonal wind follows. ContinentalV2 recorded the mesh's
/// arbitrary tangent basis, which flips near 64 degrees of latitude; its
/// worlds keep it so they replay unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wind {
    MeshTangent,
    GeographicEast,
}

/// Easterlies below 30 degrees of latitude, westerlies above.
fn prevailing_wind(p: Point, basis: Wind) -> Point {
    let east = match basis {
        Wind::MeshTangent => tangent(p).0,
        // At either exact pole, use the east of the longitude-zero meridian.
        Wind::GeographicEast if p[0] == 0.0 && p[1] == 0.0 => [0.0, 1.0, 0.0],
        Wind::GeographicEast => unit([-p[1], p[0], 0.0]),
    };
    if p[2].abs() < 0.50 {
        east.map(|v| -v)
    } else {
        east
    }
}

/// O(N log N) time (sea-level ranking and coast-distance heap), O(N) storage;
/// the bounded crust/plate fields add O(N). Geometry refinement has the same
/// bounds and never changes the number or identity of the simulation cells.
pub(crate) fn surface(
    seed: u64,
    mesh: &mut sphere::Mesh,
    radius_km: f32,
    wind: Wind,
) -> (Vec<Terrain>, Vec<f64>, Vec<f64>) {
    let model = Tectonics::new(seed);
    let mut shore = Vec::with_capacity(mesh.cells.len());
    let mut elevation = Vec::with_capacity(mesh.cells.len());
    for cell in &mesh.cells {
        let p = cell.position;
        let crust: f64 = model.crust.iter().map(|c| c.at(p)).sum();
        let (collision, rift) = model.stress(p);
        let coastal = model.coast.at(p);
        // Rift valleys cut crust; convergence adds narrow marginal arcs and
        // collisional sutures rather than a mountain at every body's centre.
        shore.push(crust + 0.105 * coastal + 0.07 * collision - 0.12 * rift);
        elevation.push(
            (0.10 + 0.26 * crust + 0.58 * collision - 0.15 * rift + 0.12 * model.relief.at(p))
                .clamp(0.0, 1.0),
        );
    }
    // A seeded sea-level budget prevents all-ocean/all-land draws without
    // forcing a count of continents or retrying a world. Ties are by region ID.
    let mut by_height: Vec<_> = (0..shore.len()).collect();
    by_height.sort_by(|&a, &b| shore[a].total_cmp(&shore[b]).then(a.cmp(&b)));
    let sea_count = ((1.0 - model.land_share) * shore.len() as f64) as usize;
    let level = (shore[by_height[sea_count - 1]] + shore[by_height[sea_count]]) * 0.5;
    let mut terrain = vec![Terrain::Sea; mesh.cells.len()];
    for &r in &by_height[sea_count..] {
        terrain[r] = Terrain::Plains;
    }
    for (r, value) in shore.iter_mut().enumerate() {
        *value -= level;
        // The ranked tie policy also owns the sign used by coast interpolation.
        if terrain[r].is_land() {
            *value = (*value).max(1e-12);
        } else {
            *value = (*value).min(-1e-12);
        }
    }
    sphere::refine_coasts(mesh, &shore, |p| model.detail.at(p));
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
            let wind = prevailing_wind(p, wind);
            let mut shadow: f64 = 0.0;
            let mut uplift: f64 = 0.0;
            for &n in &cell.neighbours {
                let upwind = -dot(mesh.cells[n].position, wind);
                if upwind > 0.0 {
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
                + 0.24 * model.wet.at(p))
            .clamp(0.0, 1.0)
        })
        .collect();
    // Rank only the relief: there are substantial lowlands on every draw,
    // while climate classes use real thresholds, not fixed global quotas.
    let mut land: Vec<_> = (0..terrain.len())
        .filter(|&r| terrain[r].is_land())
        .collect();
    land.sort_by(|&a, &b| elevation[a].total_cmp(&elevation[b]).then(a.cmp(&b)));
    let mountains = land.len() * 10 / 100;
    let hills = land.len() * 17 / 100;
    let lowland_end = land.len() - mountains - hills;
    for (rank, &r) in land.iter().enumerate() {
        terrain[r] = if rank >= land.len() - mountains {
            Terrain::Mountains
        } else if rank >= lowland_end {
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
    const WIND: Wind = Wind::GeographicEast;

    fn position(longitude: f64, latitude: f64) -> Point {
        let (longitude, latitude) = (longitude.to_radians(), latitude.to_radians());
        [
            math::cos(latitude) * math::cos(longitude),
            math::cos(latitude) * math::sin(longitude),
            math::sin(latitude),
        ]
    }

    #[test]
    fn upwind_stays_zonal_across_the_mesh_basis_switch() {
        for longitude in [-170.0, -90.0, 0.0, 45.0, 120.0, 180.0] {
            for hemisphere in [-1.0, 1.0] {
                let below = prevailing_wind(position(longitude, hemisphere * 64.0), WIND);
                let above = prevailing_wind(position(longitude, hemisphere * 65.0), WIND);
                assert!(dot(below, above) > 1.0 - 1e-12);
                for latitude in [64.0, 65.0, 80.0, 89.999] {
                    let p = position(longitude, hemisphere * latitude);
                    let wind = prevailing_wind(p, WIND);
                    assert_eq!(wind[2], 0.0, "zonal wind has no northward component");
                    assert!(dot(wind, p).abs() < 1e-12);
                    assert!((dot(wind, wind) - 1.0).abs() < 1e-12);
                    let west = position(longitude - 1.0, hemisphere * latitude);
                    let east = position(longitude + 1.0, hemisphere * latitude);
                    assert!(-dot(west, wind) > 0.0, "western relief is upwind");
                    assert!(-dot(east, wind) < 0.0, "eastern relief is downwind");
                }
            }
        }
    }

    #[test]
    fn exact_poles_have_a_fixed_wind_and_tropics_keep_easterlies() {
        for pole in [-1.0, 1.0] {
            assert_eq!(prevailing_wind([0.0, 0.0, pole], WIND), [0.0, 1.0, 0.0]);
        }
        for latitude in [-20.0, 0.0, 20.0] {
            let wind = prevailing_wind(position(0.0, latitude), WIND);
            assert!(-dot(position(1.0, latitude), wind) > 0.0);
            assert!(-dot(position(-1.0, latitude), wind) < 0.0);
        }
    }
}

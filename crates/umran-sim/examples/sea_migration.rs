//! Compare physical sea opportunity on the same seeds, before demography or fleets.
//! cargo run --release -p umran-sim --example sea_migration -- [seeds]
use umran_sim::{GeographyVersion, LandmassKind, MapSize, Params, World};

#[derive(Default)]
struct Reach {
    shores: usize,
    destinations: usize,
    overseas: usize,
}

#[derive(Default)]
struct Geography {
    maps: usize,
    land: usize,
    shores: usize,
    bodies: usize,
    islands: usize,
    walks: usize,
    sea: [Reach; 4],
    cheapest: f32,
}

fn main() {
    let seeds: u64 = std::env::args()
        .nth(1)
        .map_or(40, |s| s.parse().expect("seeds"));
    let sizes = [
        MapSize::Small,
        MapSize::Medium,
        MapSize::Large,
        MapSize::Vast,
    ];
    let params = Params::default();
    let reaches = [params.migration_reach, 700.0, 800.0, params.colony_reach];
    for version in [
        GeographyVersion::ContinentalV4,
        GeographyVersion::ContinentalV5,
    ] {
        let mut counts: [Geography; 4] = std::array::from_fn(|_| Geography {
            cheapest: f32::INFINITY,
            ..Geography::default()
        });
        for seed in 0..seeds {
            let size = seed as usize % sizes.len();
            let world = World::with_geography(seed, params.clone(), sizes[size], version);
            let map = &world.map;
            let count = &mut counts[size];
            count.maps += 1;
            count.land += map.regions.iter().filter(|r| r.terrain.is_land()).count();
            count.bodies += map.landmasses.len();
            count.islands += map
                .landmasses
                .iter()
                .filter(|l| l.kind == LandmassKind::Island)
                .count();
            for shore in (0..map.regions.len()).filter(|&r| map.coastal(r)) {
                count.shores += 1;
                let walking = map.walking_row(shore, params.colony_reach);
                for &(to, effort) in walking.iter() {
                    if to as usize != shore && effort <= params.migration_reach {
                        count.walks += 1;
                    }
                }
                let mut shortest = f32::INFINITY;
                for &(to, effort) in map.voyage_row(shore, params.colony_reach).iter() {
                    let walk = walking
                        .binary_search_by_key(&to, |&(r, _)| r)
                        .ok()
                        .map_or(f32::INFINITY, |i| walking[i].1);
                    if effort >= walk {
                        continue;
                    }
                    shortest = shortest.min(effort);
                    for (reach, sea) in reaches.iter().zip(&mut count.sea) {
                        if effort <= *reach {
                            sea.destinations += 1;
                            sea.overseas += usize::from(map.overseas(shore, to as usize));
                        }
                    }
                }
                count.cheapest = count.cheapest.min(shortest);
                for (reach, sea) in reaches.iter().zip(&mut count.sea) {
                    sea.shores += usize::from(shortest <= *reach);
                }
            }
        }
        println!(
            "{version:?}: {seeds} matched maps; sea means boat-shortest coast-to-coast routes, without fleet/contact/capacity gates"
        );
        for (size, count) in sizes.iter().zip(&counts) {
            println!(
                "{size:?}: {} maps; radius {:.0}; colony/radius {:.3}; land {}; shores {} ({:.1}%); bodies {}; islands {}; walk destinations/shore at {:.0}: {:.2}; minimum boat-shortest {:.1}",
                count.maps,
                size.radius_km(),
                params.colony_reach / size.radius_km(),
                count.land,
                count.shores,
                100.0 * count.shores as f64 / count.land.max(1) as f64,
                count.bodies,
                count.islands,
                params.migration_reach,
                count.walks as f64 / count.shores.max(1) as f64,
                count.cheapest,
            );
            for (reach, sea) in reaches.iter().zip(&count.sea) {
                println!(
                    "  sea at {reach:.0}: eligible shores {} ({:.1}%); destinations/shore {:.2}; overseas {}/{} ({:.1}%)",
                    sea.shores,
                    100.0 * sea.shores as f64 / count.shores.max(1) as f64,
                    sea.destinations as f64 / count.shores.max(1) as f64,
                    sea.overseas,
                    sea.destinations,
                    100.0 * sea.overseas as f64 / sea.destinations.max(1) as f64,
                );
            }
        }
    }
}

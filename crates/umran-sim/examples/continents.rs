//! Compare immutable geography versions by physical area, not cell counts.
//! cargo run --release -p umran-sim --example continents -- [seeds] [size|all]
//! cargo run --release -p umran-sim --example continents -- dynamics [seeds] [generations]
//! cargo run --release -p umran-sim --example continents -- sites [seed]

use umran_sim::geography::{GeographyVersion, MapSize, continent_diagnostics};
use umran_sim::{ContactKind, Map, Params, SoundProfile, Terrain, World, WorldEvent};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "dynamics") {
        dynamics(
            args.get(2).map_or(12, |n| n.parse().expect("seed count")),
            args.get(3).map_or(160, |n| n.parse().expect("generations")),
        );
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "sites") {
        sites(args.get(2).map_or(21, |n| n.parse().expect("world seed")));
        return;
    }
    let seeds: u64 = args.get(1).map_or(40, |n| n.parse().expect("seed count"));
    let sizes = match args.get(2).map(String::as_str).unwrap_or("all") {
        "small" => vec![MapSize::Small],
        "medium" => vec![MapSize::Medium],
        "large" => vec![MapSize::Large],
        "vast" => vec![MapSize::Vast],
        "all" => vec![
            MapSize::Small,
            MapSize::Medium,
            MapSize::Large,
            MapSize::Vast,
        ],
        _ => panic!("size must be small, medium, large, vast, or all"),
    };
    println!(
        "size,seed,version,land_pct,continents,top1_pct,top2_pct,top3_pct,top4_pct,top5_pct,island_pct,compactness,mountain_pct,island_supported_pct,mountain_collision,lowland_collision,coastal_mountain_pct,islands,continental_straits"
    );
    let mut failures = [0; 6];
    let mut worlds = 0;
    for size in sizes {
        for seed in 0..seeds {
            for (label, version) in [
                ("v3", GeographyVersion::ContinentalV3),
                ("v5", GeographyVersion::ContinentalV5),
            ] {
                let d = continent_diagnostics(seed, size, version);
                let compactness = d
                    .continents
                    .iter()
                    .map(|c| format!("{:.3}", c.1))
                    .collect::<Vec<_>>()
                    .join(";");
                println!(
                    "{size:?},{seed},{label},{:.3},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{compactness},{:.3},{:.3},{:.4},{:.4},{:.3},{},{}",
                    d.land_share * 100.0,
                    d.continents.len(),
                    d.top_shares[0] * 100.0,
                    d.top_shares[1] * 100.0,
                    d.top_shares[2] * 100.0,
                    d.top_shares[3] * 100.0,
                    d.top_shares[4] * 100.0,
                    d.island_share * 100.0,
                    d.mountain_share * 100.0,
                    d.supported_island_share * 100.0,
                    d.mountain_collision,
                    d.lowland_collision,
                    d.coastal_mountain_share * 100.0,
                    d.island_count,
                    d.continental_straits
                );
                if version == GeographyVersion::ContinentalV5 {
                    worlds += 1;
                    let count_band = if size == MapSize::Small { 2..=5 } else { 3..=6 };
                    let minimum_islands = match size {
                        MapSize::Small => 2,
                        MapSize::Medium | MapSize::Large => 6,
                        MapSize::Vast => 15,
                    };
                    let mut compact: Vec<_> = d.continents.iter().map(|c| c.1).collect();
                    compact.sort_by(f64::total_cmp);
                    let holds = [
                        (0.26..=0.34).contains(&d.land_share),
                        count_band.contains(&d.continents.len()),
                        (0.35..=0.60).contains(&d.top_shares[0])
                            && (0.15..=0.35).contains(&d.top_shares[1])
                            && d.top_shares[..3].windows(2).all(|p| p[0] - p[1] >= 0.10),
                        (0.03..=0.08).contains(&d.island_share)
                            && d.supported_island_share >= 0.5
                            && d.island_count >= minimum_islands,
                        !compact.is_empty()
                            && compact.iter().all(|&c| (0.07..=0.38).contains(&c))
                            && (0.12..=0.30).contains(
                                &((compact[(compact.len() - 1) / 2] + compact[compact.len() / 2])
                                    * 0.5),
                            )
                            && d.continents[0].1 <= 0.25,
                        d.mountain_collision > 2.0 * d.lowland_collision,
                    ];
                    for (failed, hold) in failures.iter_mut().zip(holds) {
                        *failed += usize::from(!hold);
                    }
                }
            }
        }
    }
    eprintln!(
        "V5 worlds={worlds}; target failures (land,count,shares,islands,compactness,stress)={failures:?}"
    );
}

fn dynamics(seeds: u64, generations: u32) {
    let kinds = [
        ContactKind::Neighbours,
        ContactKind::Trade,
        ContactKind::Rule,
        ContactKind::Religion,
        ContactKind::Intermarriage,
    ];
    let profiles = SoundProfile::presets();
    println!(
        "seed,version,living,neighbours,trade,rule,religion,intermarriage,states_founded,shifts,met_neighbours,met_trade,met_rule,met_religion,met_intermarriage"
    );
    let mut sums = [[0.0; 13]; 2];
    for seed in 0..seeds {
        for (version, geography) in [
            GeographyVersion::ContinentalV3,
            GeographyVersion::ContinentalV5,
        ]
        .into_iter()
        .enumerate()
        {
            let mut world =
                World::with_geography(seed, Params::default(), MapSize::Medium, geography);
            // The existing calibrate geography recipe: six independently
            // placed founders, no authored contact or forced shared homeland.
            for i in 0..6 {
                world.found(&profiles[i % profiles.len()], 0.25 + i as f32 * 0.12, 0.5);
            }
            world.run(generations);
            let mut row = [0; 13];
            row[0] = world.living().count();
            for (i, kind) in kinds.iter().enumerate() {
                row[1 + i] = world.contacts.iter().filter(|c| c.kind == *kind).count();
                row[8 + i] = world
                    .events
                    .iter()
                    .filter(
                        |(_, event)| matches!(event, WorldEvent::Met { kind: k, .. } if k == kind),
                    )
                    .count();
            }
            row[6] = world.states.len();
            row[7] = world
                .events
                .iter()
                .filter(|(_, event)| matches!(event, WorldEvent::Shift { .. }))
                .count();
            print!("{seed},{}", ["v3", "v5"][version]);
            for (sum, value) in sums[version].iter_mut().zip(row) {
                *sum += value as f64;
                print!(",{value}");
            }
            println!();
        }
    }
    for (version, sum) in sums.into_iter().enumerate() {
        print!("mean,{}", ["v3", "v5"][version]);
        for value in sum {
            print!(",{:.3}", value / seeds as f64);
        }
        println!();
    }
}

/// Find nearby, lake-free river and coastal plains for the authored showcase.
fn sites(seed: u64) {
    use std::collections::VecDeque;

    let map = Map::generate(seed, MapSize::Medium);
    let largest = map
        .landmasses
        .iter()
        .enumerate()
        .max_by_key(|(_, m)| m.regions.len())
        .unwrap()
        .0;
    let plain = |r: usize| {
        map.regions[r].terrain == Terrain::Plains
            && map.regions[r].landmass == Some(largest)
            && map.lake_regions[r].is_none()
    };
    println!(
        "seed={seed} geography={:?} lakes={} largest_landmass={largest}",
        map.geography,
        map.lakes.len()
    );
    let mut parent = vec![usize::MAX; map.regions.len()];
    let mut queue = VecDeque::new();
    for (river, course) in map.rivers.iter().enumerate() {
        for &start in &course.course {
            if !plain(start) || map.coastal(start) {
                continue;
            }
            parent.fill(usize::MAX);
            parent[start] = start;
            queue.clear();
            queue.push_back((start, 0));
            while let Some((r, depth)) = queue.pop_front() {
                if depth == 4 {
                    continue;
                }
                for &next in &map.regions[r].neighbours {
                    if parent[next] != usize::MAX || !plain(next) {
                        continue;
                    }
                    parent[next] = r;
                    queue.push_back((next, depth + 1));
                    if depth >= 1 && map.coastal(next) {
                        let mut path = vec![next];
                        let mut at = next;
                        while at != start {
                            at = parent[at];
                            path.push(at);
                        }
                        path.reverse();
                        let middle = path[(path.len() - 1) / 2];
                        println!("river={river} sites=[{start},{middle},{next}] path={path:?}");
                    }
                }
            }
        }
    }
}

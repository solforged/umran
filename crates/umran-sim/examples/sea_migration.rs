//! Diagnose physical sea reach independently of craft acquisition and demography.
//! cargo run --release -p umran-sim --example sea_migration -- [seeds]
use umran_sim::{MapSize, Params, World};

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
    let reaches = [600.0, 1200.0, 1800.0, 2400.0];
    let mut shores = [0usize; 4];
    let mut eligible = [[0usize; 4]; 4];
    let mut minima = [f32::INFINITY; 4];
    let mut overseas = [f32::INFINITY; 4];
    for seed in 0..seeds {
        let size = seed as usize % sizes.len();
        let world = World::with_map(seed, Params::default(), sizes[size]);
        for shore in (0..world.map.regions.len()).filter(|&r| world.map.coastal(r)) {
            shores[size] += 1;
            let walking = world.map.walking_row(shore, 2400.0);
            let mut shortest = f32::INFINITY;
            for &(to, effort) in world.map.voyage_row(shore, 2400.0).iter() {
                minima[size] = minima[size].min(effort);
                if world.map.overseas(shore, to as usize) {
                    overseas[size] = overseas[size].min(effort);
                }
                let walk = walking
                    .binary_search_by_key(&to, |&(r, _)| r)
                    .ok()
                    .map_or(f32::INFINITY, |i| walking[i].1);
                if effort < walk {
                    shortest = shortest.min(effort);
                }
            }
            for (i, reach) in reaches.iter().enumerate() {
                eligible[size][i] += usize::from(shortest <= *reach);
            }
        }
    }
    println!(
        "{seeds} maps; shores with at least one sea journey cheaper than walking at {reaches:?} effort-km:"
    );
    for i in 0..sizes.len() {
        println!(
            "{:?}: {} shores; {:?}; minimum voyage {:.1}; minimum overseas {:.1}",
            sizes[i], shores[i], eligible[i], minima[i], overseas[i]
        );
    }
}

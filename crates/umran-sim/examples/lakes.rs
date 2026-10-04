//! Lake basins, channel geometry, and surviving hydronyms over 4,000 years.
#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
use umran_sim::{MapSize, Naming, Params, SoundProfile, World};

#[derive(Default)]
pub struct Report {
    pub worlds: u64,
    pub lakes: usize,
    pub closed: usize,
    pub named: usize,
    pub lake_area: f64,
    pub land_area: f64,
}

pub fn report(size: MapSize, seeds: u64, generations: u32) -> Report {
    let mut report = Report {
        worlds: seeds,
        ..Report::default()
    };
    for seed in 0..seeds {
        let mut world = World::with_map(seed, Params::default(), size);
        for (i, profile) in ["familiar", "germanic", "polynesian", "semitic", "iranian"]
            .iter()
            .enumerate()
        {
            let region = world.map.lakes.get(i).map(|lake| lake.regions[0]);
            world.found_seeded(
                &Naming::People,
                &SoundProfile::by_id(profile).unwrap(),
                seed * 10 + i as u64,
                0.5,
                0.5,
                region,
                None,
                None,
            );
        }
        let geometry = world.map.clone();
        world.run(generations);
        assert_eq!(world.map.lakes, geometry.lakes);
        assert_eq!(world.map.rivers, geometry.rivers);
        assert_eq!(world.map.regions, geometry.regions);
        summarize(&world, &format!("size={size:?} seed={seed}"), &mut report);
    }
    println!(
        "band size={size:?} worlds={} years={} lakes={} closed={} named={} lakes_per_world={:.3} endorheic_share={:.5} named_share={:.5} lake_region_area_share={:.5}",
        report.worlds,
        generations * 25,
        report.lakes,
        report.closed,
        report.named,
        report.lakes as f64 / report.worlds as f64,
        report.closed as f64 / report.lakes as f64,
        report.named as f64 / report.lakes as f64,
        report.lake_area / report.land_area
    );
    report
}

pub fn report_sample() -> Report {
    let mut report = Report {
        worlds: 1,
        ..Report::default()
    };
    let history = sample::sample();
    summarize(history.latest(), "sample=21", &mut report);
    report
}

fn summarize(world: &World, label: &str, report: &mut Report) {
    let map = &world.map;
    let closed = map.lakes.iter().filter(|lake| lake.spill.is_none()).count();
    let named = world
        .lake_names
        .iter()
        .filter(|names| !names.is_empty())
        .count();
    let points: usize = map.rivers.iter().map(|river| river.channel.len()).sum();
    report.lakes += map.lakes.len();
    report.closed += closed;
    report.named += named;
    report.lake_area += map
        .lakes
        .iter()
        .flat_map(|lake| &lake.regions)
        .map(|&r| f64::from(map.regions[r].area_km2))
        .sum::<f64>();
    report.land_area += map
        .regions
        .iter()
        .filter(|r| r.terrain.is_land())
        .map(|r| f64::from(r.area_km2))
        .sum::<f64>();
    println!(
        "{label} years={} lakes={} closed={closed} named={named} rivers={} channel_points={points}",
        world.generation * 25,
        map.lakes.len(),
        map.rivers.len()
    );
    for (id, lake) in map.lakes.iter().enumerate().take(3) {
        let name = world.lake_names[id]
            .last()
            .map(|record| world.varieties[record.variety].title(&record.name.form));
        println!(
            "  lake={id} regions={:?} surface={:.4} outlet={:?} name={name:?} attestations={}",
            lake.regions,
            lake.surface,
            lake.outlet,
            world.lake_names[id].len()
        );
    }
}

#[cfg(not(test))]
fn main() {
    let seeds = std::env::args()
        .nth(1)
        .map_or(4, |n| n.parse().expect("seeds"));
    for size in [
        MapSize::Small,
        MapSize::Medium,
        MapSize::Large,
        MapSize::Vast,
    ] {
        report(size, seeds, 160);
    }
    report_sample();
}

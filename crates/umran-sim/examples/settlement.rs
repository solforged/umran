//! Four settlement cohorts on Medium continental-v6, readings through year 4000.
//!
//! cargo run --release -p umran-sim --example settlement -- [seeds=40] [cohort=all]
//! Cohorts: three-free, three-grouped, six-free, six-grouped.
//! Timings include only World::step, not founding, surveying, or printing.

use rand::RngCore;
use std::time::Instant;
use umran_sim::rng::{key, stream};
use umran_sim::{
    GeographyVersion, Livelihood, MapSize, Naming, Params, SoundProfile, World, WorldEvent,
};

const YEARS: [u32; 5] = [0, 500, 1000, 2000, 4000];

// Equivalent to Bench::founding_sites on an empty year-zero world: walking
// effort order, the 800-effort-km cap, then two, one, or no intervening lands.
fn founding_sites(world: &World, anchor: usize, count: usize) -> Vec<usize> {
    let mut nearby: Vec<_> = world
        .map
        .walking_row(anchor, world.params.settle_apart)
        .iter()
        .map(|&(r, effort)| (r as usize, effort))
        .filter(|&(r, effort)| {
            effort.is_finite() && Livelihood::ALL.iter().any(|&way| world.feeds(r, way) > 0.0)
        })
        .collect();
    nearby.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut steps = vec![u8::MAX; world.map.regions.len()];
    let mut sites = Vec::with_capacity(count);
    for gap in (0..=2).rev() {
        for &(r, _) in &nearby {
            if sites.len() == count {
                break;
            }
            if steps[r] > gap || (r == anchor && sites.is_empty()) {
                sites.push(r);
                steps[r] = 0;
                let mut frontier = vec![r];
                for step in 1..=2 {
                    let mut next = Vec::new();
                    for r in frontier {
                        for &n in &world.map.regions[r].neighbours {
                            if steps[n] > step {
                                steps[n] = step;
                                next.push(n);
                            }
                        }
                    }
                    frontier = next;
                }
            }
        }
    }
    assert_eq!(
        sites.len(),
        count,
        "seed {}: short founding group",
        world.seed
    );
    sites
}

fn survey(world: &World, habitable: &[bool], step_seconds: f64) -> [f64; 5] {
    let mut occupied = vec![false; world.map.regions.len()];
    for c in world.living() {
        for (r, n) in world.presence(c) {
            occupied[r] |= n > 0.0;
        }
    }
    let mut landmasses = vec![false; world.map.landmasses.len()];
    let mut area = 0.0;
    let mut settled = 0.0;
    for (r, region) in world.map.regions.iter().enumerate() {
        if habitable[r] {
            area += f64::from(region.area_km2);
            if occupied[r] {
                settled += f64::from(region.area_km2);
            }
        }
        if occupied[r]
            && let Some(body) = region.landmass
        {
            landmasses[body] = true;
        }
    }
    [
        occupied.iter().filter(|&&x| x).count() as f64,
        100.0 * settled / area,
        world.living().count() as f64,
        landmasses.iter().filter(|&&x| x).count() as f64,
        step_seconds,
    ]
}

fn median(values: &mut [u32]) -> f64 {
    values.sort_unstable();
    (f64::from(values[(values.len() - 1) / 2]) + f64::from(values[values.len() / 2])) / 2.0
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u64 = args.next().map_or(40, |s| s.parse().expect("seeds"));
    assert!(seeds > 0);
    let selected = args.next().unwrap_or_else(|| "all".into());
    let cohorts = [
        ("three-free", 3, false),
        ("three-grouped", 3, true),
        ("six-free", 6, false),
        ("six-grouped", 6, true),
    ];
    assert!(selected == "all" || cohorts.iter().any(|c| c.0 == selected));
    let profiles = SoundProfile::presets();
    println!("Medium continental-v6; seeds 0..{seeds}; default Params; step-only wall time.");
    println!(
        "| Cohort | Year | Occupied regions | Habitable area settled | Living peoples | Landmasses | Step total (s) |"
    );
    println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: |");
    for (label, founders, grouped) in cohorts {
        if selected != "all" && selected != label {
            continue;
        }
        let mut sums = [[0.0; 5]; YEARS.len()];
        let mut first_spreads = Vec::new();
        let mut censored = 0;
        let mut events = [0.0; 5];
        let mut per_land_splits = 0.0;
        for seed in 0..seeds {
            let mut world = World::with_geography(
                seed,
                Params::default(),
                MapSize::Medium,
                GeographyVersion::ContinentalV6,
            );
            let habitable: Vec<_> = (0..world.map.regions.len())
                .map(|r| Livelihood::ALL.iter().any(|&way| world.feeds(r, way) > 0.0))
                .collect();
            let sites = if grouped {
                let mut probe = world.clone();
                let c = probe.found(&profiles[0], 0.25, 0.5);
                founding_sites(&world, probe.communities[c].home(), founders)
            } else {
                Vec::new()
            };
            for i in 0..founders {
                let profile = &profiles[i % profiles.len()];
                let power = 0.25 + i as f32 * 0.12;
                if grouped {
                    let variety_seed = stream(seed, &[key("found"), i as u64]).next_u64();
                    world.found_seeded(
                        &Naming::People,
                        profile,
                        variety_seed,
                        power,
                        0.5,
                        Some(sites[i]),
                        None,
                        None,
                    );
                } else {
                    world.found(profile, power, 0.5);
                }
            }
            let farmers: Vec<_> = world
                .living()
                .filter(|&c| world.communities[c].livelihood == Livelihood::Farming)
                .collect();
            let mut elapsed = 0.0;
            for (reading, year) in YEARS.into_iter().enumerate() {
                while world.generation * 25 < year {
                    let start = Instant::now();
                    world.step();
                    elapsed += start.elapsed().as_secs_f64();
                }
                for (sum, value) in sums[reading]
                    .iter_mut()
                    .zip(survey(&world, &habitable, elapsed))
                {
                    *sum += value;
                }
            }
            for farmer in farmers {
                let first = world.events.iter().find_map(|(g, e)| {
                    matches!(e, WorldEvent::Spread { community, .. } if *community == farmer)
                        .then_some(g * 25)
                });
                censored += usize::from(first.is_none());
                first_spreads.push(first.unwrap_or(4025));
            }
            let mut splits = 0;
            for (_, event) in &world.events {
                match event {
                    WorldEvent::Spread { .. } => events[0] += 1.0,
                    WorldEvent::Split { by_sea, .. } => {
                        events[1] += 1.0;
                        events[2] += f64::from(*by_sea);
                        splits += 1;
                    }
                    WorldEvent::Migrated { .. } => events[3] += 1.0,
                    WorldEvent::HardTimes { .. } => events[4] += 1.0,
                    _ => {}
                }
            }
            per_land_splits += splits as f64 / survey(&world, &habitable, elapsed)[0];
        }
        for (year, row) in YEARS.into_iter().zip(sums) {
            let [occupied, area, living, bodies, time] = row.map(|v| v / seeds as f64);
            println!(
                "| {label} | {year} | {occupied:.2} | {area:.2}% | {living:.2} | {bodies:.2} | {time:.3} |"
            );
        }
        let [spread, splits, colonies, migrations, hardship] = events.map(|v| v / seeds as f64);
        println!(
            "{label}: farming founders={}, first Spread median={:.1} years (unspread={censored}, censored at 4025); mean spread={spread:.2}, splits={splits:.2}, sea colonies={colonies:.2}, migrations={migrations:.2}, hardship={hardship:.2}, splits/final occupied land={:.3}",
            first_spreads.len(),
            median(&mut first_spreads),
            per_land_splits / seeds as f64
        );
    }
}

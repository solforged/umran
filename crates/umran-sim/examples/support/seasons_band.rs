use std::collections::BTreeSet;
use umran_sim::seasons::{SeasonalHazard, WetSeason};
use umran_sim::{Params, SoundProfile, World, WorldEvent};

#[derive(Default)]
pub struct Report {
    pub counts: [usize; 3],
    pub samples: usize,
    pub lands: usize,
    pub hit_lands: usize,
    pub quiet_land_years: usize,
    /// Severe hazards on inhabited land, the recorded ones.
    pub recorded: usize,
    pub migrations: usize,
    pub adoptions: usize,
    pub monsoon: usize,
    pub floodplains: usize,
}

impl Report {
    pub fn add(&mut self, world: &World) {
        let lands = world
            .map
            .regions
            .iter()
            .filter(|r| r.terrain.is_land())
            .count();
        self.lands += lands;
        let samples = lands * world.generation as usize;
        self.samples += samples;
        // Rates come from every sampled hazard; the record holds only severe
        // hazards on inhabited land, the ones a response can cite.
        let counts = world.climate.seasons.struck.map(|n| n as usize);
        let mut hits = BTreeSet::new();
        for (_, event) in &world.events {
            if let WorldEvent::SeasonalHazard {
                region,
                hazard,
                severity,
                ref peoples,
            } = *event
            {
                assert!(world.map.regions[region].terrain.is_land());
                assert!((0.4..=1.0).contains(&severity));
                assert!(!peoples.is_empty(), "only inhabited land is recorded");
                if hazard == SeasonalHazard::Flood {
                    assert!(world.climate.seasons.profiles[region].floods);
                    assert!(world.map.river_regions[region].is_some());
                }
                hits.insert(region);
                self.recorded += 1;
            }
        }
        self.hit_lands += hits.len();
        self.quiet_land_years += samples - counts.iter().sum::<usize>();
        for (total, count) in self.counts.iter_mut().zip(counts) {
            *total += count;
        }
        self.monsoon += world
            .climate
            .seasons
            .profiles
            .iter()
            .filter(|p| p.wet == WetSeason::Monsoon)
            .count();
        self.floodplains += world
            .climate
            .seasons
            .profiles
            .iter()
            .filter(|p| p.floods)
            .count();
        for (&response, cause) in &world.causes {
            if matches!(
                world.events[cause.event].1,
                WorldEvent::SeasonalHazard { .. }
            ) {
                assert!(cause.event < response);
                self.migrations += usize::from(matches!(
                    world.events[response].1,
                    WorldEvent::Migrated { .. }
                ));
                self.adoptions += usize::from(matches!(
                    world.events[response].1,
                    WorldEvent::Adopted { .. }
                ));
            }
        }
    }

    pub fn print(&self, label: &str) {
        println!(
            "{label}: sampled region-years={} drought={} ({:.2}%) hard-winter={} ({:.2}%) flood={} ({:.2}%) quiet={:.2}%",
            self.samples,
            self.counts[0],
            self.rate(0) * 100.0,
            self.counts[1],
            self.rate(1) * 100.0,
            self.counts[2],
            self.rate(2) * 100.0,
            self.quiet_land_years as f64 * 100.0 / self.samples as f64
        );
        println!(
            "  recorded={} inhabited lands hit={}/{} monsoon={} floodplains={} seasonal causes: migration={} livelihood={}",
            self.recorded,
            self.hit_lands,
            self.lands,
            self.monsoon,
            self.floodplains,
            self.migrations,
            self.adoptions
        );
    }

    pub fn rate(&self, kind: usize) -> f64 {
        self.counts[kind] as f64 / self.samples as f64
    }
}

pub fn history(seed: u64) -> World {
    let mut world = World::new(
        seed,
        Params {
            seasons_enabled: true,
            climate_enabled: true,
            growth_rate: Params::default().growth_rate,
            migration_rate: Params::default().migration_rate,
            adoption_rate: Params::default().adoption_rate,
            ..Params::static_society()
        },
    );
    // Include farming near the pastoral margin, not only the richest plain
    // selected by automatic founding. Failed rains can change the better
    // livelihood here; the other founders still use ordinary settlement.
    let marginal = (0..world.map.regions.len())
        .filter(|&r| {
            let farming = world.feeds(r, umran_sim::Livelihood::Farming);
            let herding = world.feeds(r, umran_sim::Livelihood::Herding);
            farming > 2_000.0 && (0.8..1.4).contains(&(herding / farming))
        })
        .max_by(|&a, &b| {
            world
                .feeds(a, umran_sim::Livelihood::Farming)
                .total_cmp(&world.feeds(b, umran_sim::Livelihood::Farming))
        });
    world.found_seeded(
        &umran_sim::Naming::People,
        &SoundProfile::by_id("germanic").unwrap(),
        seed + 100,
        0.5,
        0.5,
        marginal,
        Some(umran_sim::Livelihood::Farming),
        None,
    );
    for profile in ["semitic", "polynesian"] {
        world.found(&SoundProfile::by_id(profile).unwrap(), 0.5, 0.5);
    }
    world.run(160);
    world
}

use umran_sim::{Params, SoundProfile, World, WorldEvent};

#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

#[derive(Debug, Default)]
struct Counts {
    faiths: usize,
    founding_tenets: usize,
    drifts: usize,
    disputes: usize,
    taboos: usize,
}

impl Counts {
    fn add(&mut self, world: &World) {
        self.faiths += world.religions.len();
        for (_, event) in &world.events {
            match event {
                WorldEvent::TenetAdopted { previous: None, .. } => self.founding_tenets += 1,
                WorldEvent::TenetAdopted { .. } => self.drifts += 1,
                WorldEvent::TenetDisputed { .. } => self.disputes += 1,
                WorldEvent::TabooReplaced { .. } => self.taboos += 1,
                _ => {}
            }
        }
    }
}

#[test]
#[ignore = "40 six-people worlds over 4,000 years plus the full sample recipe"]
fn doctrine_band_40_seeds_4000_years() {
    let start = std::time::Instant::now();
    let presets = SoundProfile::presets();
    let mut counts = Counts::default();
    for seed in 0..40u64 {
        let mut world = World::new(seed, Params::default());
        for i in 0..6 {
            world.found(&presets[(seed as usize + i) % presets.len()], 0.5, 0.5);
        }
        world.run(160);
        counts.add(&world);
        for religion in &world.religions {
            for position in religion.doctrine.positions() {
                assert!((-1.0..=1.0).contains(&position.stance));
            }
        }
    }
    let mut sample_counts = Counts::default();
    sample_counts.add(sample::sample().latest());
    println!(
        "doctrine band: 40 seeds × 4,000 years; {counts:?}; sample {sample_counts:?}; elapsed {:.2}s",
        start.elapsed().as_secs_f64()
    );
    // Revision 44: 19 faiths, 102 founding positions, no announced drift, 2
    // disputes at schism, 4 taboo replacements. Teaching is stable; it moves
    // at schisms, not by slow drift.
    assert!((10..=40).contains(&counts.faiths), "{counts:?}");
    assert!(counts.founding_tenets <= 6 * counts.faiths, "{counts:?}");
    assert!(counts.drifts <= counts.faiths, "{counts:?}");
    assert!((1..=20).contains(&counts.disputes), "{counts:?}");
    assert!(counts.taboos <= 4 * counts.faiths, "{counts:?}");
}

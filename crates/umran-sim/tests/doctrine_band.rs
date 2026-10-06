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
#[ignore = "160 six-people worlds over 4,000 years plus the full sample recipe"]
fn doctrine_band_160_seeds_4000_years() {
    let start = std::time::Instant::now();
    let presets = SoundProfile::presets();
    let mut counts = Counts::default();
    for seed in 0..160u64 {
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
        "doctrine band: 160 seeds × 4,000 years; {counts:?}; sample {sample_counts:?}; elapsed {:.2}s",
        start.elapsed().as_secs_f64()
    );
    // V5 seeds 0–39: 26 faiths, 138 founding positions, 3 disputes,
    // 9 taboos, no drift; schisms are too rare to test over 40 seeds.
    // V6 seeds 0–159: 179 faiths, 996 founding positions, 13 disputes,
    // 99 taboos, and no drift. The faith band changes from 10–40 over 40
    // seeds to 80–240 over 160. Keep the positive 1–20 dispute band.
    assert!((80..=240).contains(&counts.faiths), "{counts:?}");
    assert!(counts.founding_tenets <= 6 * counts.faiths, "{counts:?}");
    assert!(counts.drifts <= counts.faiths, "{counts:?}");
    assert!((1..=20).contains(&counts.disputes), "{counts:?}");
    assert!(counts.taboos <= 4 * counts.faiths, "{counts:?}");
}

//! cargo run --release -p umran-sim --example ethos -- [seed] [generations]
//! cargo run --release -p umran-sim --example ethos -- --band [seeds] [generations]
//! Band compares enabled ethos with the neutral baseline, for both launch setups.
use umran_sim::{
    Axis, ContactKind, Ethos, LanguageDesign, Naming, Params, Revelation, SoundProfile, World,
    WorldEvent,
};

fn founders(seed: u64, sample: bool, neutral: bool) -> World {
    let mut world = World::new(
        seed,
        Params {
            ethos_enabled: !neutral,
            ..Params::default()
        },
    );
    // The authored contacts start among neighbours on one farming plain.
    let home = world
        .map
        .regions
        .iter()
        .position(|r| r.terrain == umran_sim::Terrain::Plains)
        .unwrap();
    if sample {
        for (preset, language_seed, naming, power) in [
            (
                "germanic",
                31,
                Naming::Place {
                    place: "river".into(),
                },
                0.7,
            ),
            ("semitic", 52, Naming::People, 0.5),
            (
                "polynesian",
                73,
                Naming::Place {
                    place: "sea".into(),
                },
                0.4,
            ),
        ] {
            world.found_seeded(
                &naming,
                &LanguageDesign::preset(preset, language_seed)
                    .unwrap()
                    .profile(),
                language_seed,
                power,
                0.5,
                Some(home),
                None,
                None,
            );
        }
    } else {
        let presets = SoundProfile::presets();
        let n = presets.len() as u64;
        for (i, power, openness) in [(0, 0.5, 0.4), (1, 0.4, 0.6), (2, 0.85, 0.3)] {
            world.found_seeded(
                &Naming::People,
                &presets[((((seed % n) * 3 + i) * 7) % n) as usize],
                seed.wrapping_add(i),
                power,
                openness,
                Some(home),
                None,
                None,
            );
        }
        world.connect(0, 1, 0.5, ContactKind::Trade).unwrap();
        world.connect(2, 1, 0.8, ContactKind::Rule).unwrap();
        world.connect(2, 0, 0.3, ContactKind::Neighbours).unwrap();
    }
    world
}

/// An older sample-inspired sequence, with founders placed together.
fn sample_action(world: &mut World) {
    if world.generation == 100 {
        assert!(
            world.communities[0].living() && world.communities[2].living(),
            "sample founders ended, seed {}",
            world.seed
        );
        world.connect(0, 2, 0.8, ContactKind::Rule).unwrap();
    }
    if world.generation == 130 {
        assert!(
            world.communities[0].living() && world.communities[2].living(),
            "sample founders ended, seed {}",
            world.seed
        );
        world.found_religion(2, Revelation::Proclaimed);
        assert_ne!(
            world.communities[2].variety, world.communities[0].variety,
            "sample shift invalid"
        );
        world.shift(2, 0);
    }
}

fn show(world: &World) {
    println!(
        "year {}: martial open pious hierarchical roving seaward",
        world.generation * 25
    );
    for c in world.living() {
        let e = world.communities[c].ethos;
        println!(
            "  {c:>2} {:<18} {:+.3} {:+.3} {:+.3} {:+.3} {:+.3} {:+.3}",
            world.community_name(c),
            e.martial,
            e.open,
            e.pious,
            e.hierarchical,
            e.roving,
            e.seaward
        );
    }
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "--band") {
        let seeds: u64 = args.get(1).map_or(40, |s| s.parse().expect("seeds"));
        let generations: u32 = args.get(2).map_or(160, |s| s.parse().expect("generations"));
        assert!(seeds > 0 && generations >= 130);
        for sample in [false, true] {
            for neutral in [true, false] {
                let mut band = Band::default();
                for seed in 0..seeds {
                    let mut world = founders(seed, sample, neutral);
                    for _ in 0..generations {
                        world.step();
                        if sample {
                            sample_action(&mut world);
                        }
                    }
                    band.record(&world);
                }
                band.print(
                    if sample { "sample" } else { "three-founder" },
                    neutral,
                    generations,
                );
            }
        }
        return;
    }
    let seed = args.first().map_or(42, |s| s.parse().expect("seed"));
    let generations = args.get(1).map_or(160, |s| s.parse().expect("generations"));
    let mut world = founders(seed, false, false);
    println!("Ethos study: seed {seed}, {generations} generations; default three-founder setup");
    show(&world);
    for _ in 0..generations {
        let before = world.events.len();
        world.step();
        for (g, event) in &world.events[before..] {
            match event {
                WorldEvent::Temper {
                    community,
                    axis,
                    pole,
                    entered,
                    cause,
                } => println!(
                    "  year {}: people {community} {} {} {} ({})",
                    g * 25,
                    axis.id(),
                    if *entered { "entered" } else { "left" },
                    pole.id(),
                    cause.id()
                ),
                WorldEvent::HardTimes { .. }
                | WorldEvent::Conquered { .. }
                | WorldEvent::Parted {
                    kind: ContactKind::Rule,
                    ..
                }
                | WorldEvent::Learnt { .. }
                | WorldEvent::Revealed { .. }
                | WorldEvent::Converted { .. }
                | WorldEvent::Split { .. } => println!("  year {}: {event:?}", g * 25),
                _ => {}
            }
        }
        if world.generation.is_multiple_of(20) || world.generation == generations {
            show(&world);
        }
    }
}

#[derive(Default)]
struct Moments {
    n: u64,
    mean: f64,
    m2: f64,
}
impl Moments {
    fn add(&mut self, value: f64) {
        self.n += 1;
        let delta = value - self.mean;
        self.mean += delta / self.n as f64;
        self.m2 += delta * (value - self.mean);
    }
    fn sd(&self) -> f64 {
        (self.m2 / self.n.max(1) as f64).sqrt()
    }
}

#[derive(Default)]
struct Band {
    pooled: [Moments; 6],
    within: [Moments; 6],
    counts: Vec<[usize; 4]>,
}
impl Band {
    fn record(&mut self, world: &World) {
        let mut within: [Moments; 6] = Default::default();
        for c in world.living() {
            let ethos: Ethos = world.communities[c].ethos;
            for (i, axis) in Axis::ALL.into_iter().enumerate() {
                let value = f64::from(ethos.get(axis));
                self.pooled[i].add(value);
                within[i].add(value);
            }
        }
        for (i, axis) in within.iter().enumerate() {
            self.within[i].add(axis.sd());
        }
        let mut counts = [0; 4];
        for (_, event) in &world.events {
            let i = match event {
                WorldEvent::Temper { .. } => 0,
                WorldEvent::Conquered { .. } => 1,
                WorldEvent::Migrated { .. } => 2,
                WorldEvent::Schism { .. } => 3,
                _ => continue,
            };
            counts[i] += 1;
        }
        self.counts.push(counts);
    }

    fn print(&self, setup: &str, neutral: bool, generations: u32) {
        println!(
            "BAND {setup} {}: {} seeds x {} years; {} living peoples pooled",
            if neutral { "baseline-neutral" } else { "ethos" },
            self.counts.len(),
            generations * 25,
            self.pooled[0].n
        );
        for (i, axis) in Axis::ALL.into_iter().enumerate() {
            println!(
                "  {}: pooled sd {:.4}; mean within-world sd {:.4}",
                axis.id(),
                self.pooled[i].sd(),
                self.within[i].mean
            );
        }
        for (i, label) in ["temper", "conquests", "migrations", "schisms"]
            .into_iter()
            .enumerate()
        {
            let mut values: Vec<_> = self.counts.iter().map(|counts| counts[i]).collect();
            values.sort_unstable();
            let n = values.len();
            let total: usize = values.iter().sum();
            let median = (values[(n - 1) / 2] + values[n / 2]) as f64 / 2.0;
            println!(
                "  {label}: total {total}; mean {:.3}; min {}; median {median:.1}; max {}",
                total as f64 / n as f64,
                values[0],
                values[n - 1]
            );
        }
    }
}

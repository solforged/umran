use std::time::Instant;
use umran_sim::{Event, Params, SoundProfile, World, WorldEvent};

#[path = "../../../umran-web/examples/support/sample.rs"]
mod sample;

#[derive(Debug, Default)]
pub struct Stats {
    pub seeds: u64,
    pub gained: usize,
    pub lost: usize,
    pub tonal: usize,
    pub coda: usize,
    pub register: usize,
    pub sample_gains: usize,
    pub sample_tonal: usize,
}

pub fn run(seeds: u64) -> Stats {
    let start = Instant::now();
    let mut stats = Stats {
        seeds,
        ..Stats::default()
    };
    let profiles = ["base", "germanic", "semitic", "indic", "polynesian"];
    for seed in 0..seeds {
        let id = profiles[seed as usize % profiles.len()];
        let profile = if id == "base" {
            SoundProfile::base()
        } else {
            SoundProfile::by_id(id).unwrap()
        };
        let mut world = World::solo(seed, &profile, Params::default());
        world.run(160);
        let variety = &world.varieties[0];
        let gained = world.events.iter().any(|(_, event)| {
            matches!(
                event,
                WorldEvent::Tone {
                    variety: 0,
                    gained: true,
                    ..
                }
            )
        });
        let lost = world.events.iter().any(|(_, event)| {
            matches!(
                event,
                WorldEvent::Tone {
                    variety: 0,
                    gained: false,
                    ..
                }
            )
        });
        stats.gained += usize::from(gained);
        stats.lost += usize::from(lost);
        stats.tonal += usize::from(variety.tones() > 0);
        stats.coda += usize::from(variety.laws.iter().any(|(_, id)| *id == "coda-tonogenesis"));
        stats.register += usize::from(
            variety
                .laws
                .iter()
                .any(|(_, id)| *id == "register-tonogenesis"),
        );
        println!(
            "seed {seed:2} {id:10}: tones={} gained={gained} lost={lost}",
            variety.tones()
        );
        if seed < 5 {
            for word in &variety.lexicon.lexemes {
                if let Some((index, entry)) = word.log.iter().enumerate().find(|(_, entry)| {
                    matches!(
                        entry.event,
                        Event::SoundLaw {
                            law: "coda-tonogenesis" | "register-tonogenesis",
                            ..
                        }
                    )
                }) {
                    let Event::SoundLaw { before, law } = &entry.event else {
                        unreachable!()
                    };
                    let after = word.log[index + 1..]
                        .iter()
                        .find_map(|entry| match &entry.event {
                            Event::SoundLaw { before, .. } => Some(before),
                            _ => None,
                        })
                        .unwrap_or(&word.form);
                    println!(
                        "  year {} {law}: {} /{}/ > /{}/",
                        entry.generation * 25,
                        word.first_sense.gloss,
                        before.ipa(),
                        after.ipa()
                    );
                    break;
                }
            }
        }
    }
    let history = sample::sample();
    let world = history.latest();
    stats.sample_gains = world
        .events
        .iter()
        .filter(|(_, event)| matches!(event, WorldEvent::Tone { gained: true, .. }))
        .count();
    stats.sample_tonal = world
        .varieties
        .iter()
        .filter(|variety| variety.tones() > 0)
        .count();
    println!(
        "sample: years={} tonal={} gains={}",
        world.generation * 25,
        stats.sample_tonal,
        stats.sample_gains
    );
    println!(
        "tone cohort: seeds={} years=4000 gained={} lost={} tonal={} coda={} register={} elapsed={:.2}s",
        stats.seeds,
        stats.gained,
        stats.lost,
        stats.tonal,
        stats.coda,
        stats.register,
        start.elapsed().as_secs_f64()
    );
    stats
}

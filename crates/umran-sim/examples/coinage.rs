//! Native coinages, translated compounds, and opacity after 4,000 years.
//! cargo run --release -p umran-sim --example coinage -- [seeds]
use umran_sim::{ContactKind, Craft, Naming, Params, Revelation, SoundProfile, World, WorldEvent};

#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

fn counts(world: &World) -> [usize; 4] {
    let mut out = [0; 4];
    for (_, event) in &world.events {
        let WorldEvent::Coined { variety, word, .. } = event else {
            continue;
        };
        let c = world.varieties[*variety]
            .lexicon
            .get(*word)
            .coined
            .as_ref()
            .unwrap();
        out[c.kind as usize] += 1;
        out[3] += usize::from(c.opaque_since.is_some());
    }
    out
}

fn main() {
    let seeds: u64 = std::env::args()
        .nth(1)
        .map_or(40, |s| s.parse().expect("seeds"));
    assert!(seeds > 0);
    println!("{seeds} seeds per profile, two peoples in trade contact; 4,000 years.");
    println!(
        "Default coinage rate; static society; ordinary sound change, competition and borrowing."
    );
    println!(
        "Crafts and a faith are authored at year 25. Counts are new events, not inherited copies."
    );
    println!("profile,compound,derived,calque,opaque (means per world)");
    for profile in ["germanic", "semitic", "polynesian"] {
        let mut totals = [0; 4];
        for seed in 0..seeds {
            let mut world = World::solo(
                seed,
                &SoundProfile::by_id(profile).unwrap(),
                Params {
                    coinage_rate: Params::default().coinage_rate,
                    ..Params::static_society()
                },
            );
            world.found_seeded(
                &Naming::People,
                &SoundProfile::by_id("germanic").unwrap(),
                seed + 200,
                0.8,
                0.8,
                Some(world.communities[0].home()),
                None,
                None,
            );
            world.connect(0, 1, 0.8, ContactKind::Trade).unwrap();
            world.generation = 1;
            for c in 0..2 {
                for craft in Craft::ALL {
                    world.learn(c, craft, None);
                }
                world.found_religion(c, Revelation::Proclaimed);
            }
            world.run(159);
            for (sum, value) in totals.iter_mut().zip(counts(&world)) {
                *sum += value;
            }
            if seed == 0 {
                for (generation, event) in world
                    .events
                    .iter()
                    .filter(|(_, e)| matches!(e, WorldEvent::Coined { .. }))
                    .take(4)
                {
                    let WorldEvent::Coined { variety, word, .. } = event else {
                        unreachable!()
                    };
                    let v = &world.varieties[*variety];
                    let w = v.lexicon.get(*word);
                    let c = w.coined.as_ref().unwrap();
                    println!(
                        "  {profile} seed 0 year {}: {} '{}' {:?}, from {} -> now {}; opaque since {:?}",
                        generation * 25,
                        v.spell(w.form_at(*generation)),
                        w.first_sense.id,
                        c.kind,
                        c.parts
                            .iter()
                            .map(|p| format!("{} '{}'", v.spell(&p.form), p.concept.id))
                            .collect::<Vec<_>>()
                            .join(" + "),
                        v.spell(&w.form),
                        c.opaque_since.map(|g| g * 25)
                    );
                }
            }
        }
        println!(
            "{profile},{:.2},{:.2},{:.2},{:.2}",
            totals[0] as f32 / seeds as f32,
            totals[1] as f32 / seeds as f32,
            totals[2] as f32 / seeds as f32,
            totals[3] as f32 / seeds as f32
        );
    }
    let history = sample::sample();
    let world = history.latest();
    println!(
        "Workbench sample, year {}: compound/derived/calque/opaque {:?}",
        world.generation * 25,
        counts(world)
    );
}

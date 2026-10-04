use umran_sim::{ContactKind, Craft, Naming, Params, Revelation, SoundProfile, World, WorldEvent};

#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

#[test]
#[ignore = "40 seeds × 4000 years across three profiles, plus the workbench sample"]
fn productive_coinage_band() {
    let started = std::time::Instant::now();
    let mut totals = [0usize; 4];
    for profile in ["germanic", "semitic", "polynesian"] {
        let mut counts = [0usize; 4];
        for seed in 0..40 {
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
            for (_, event) in &world.events {
                if let WorldEvent::Coined { variety, word, .. } = event {
                    let c = world.varieties[*variety]
                        .lexicon
                        .get(*word)
                        .coined
                        .as_ref()
                        .unwrap();
                    counts[c.kind as usize] += 1;
                    counts[3] += usize::from(c.opaque_since.is_some());
                }
            }
        }
        println!(
            "{profile}: 40 seeds × 4000 years: compounds={} derived={} calques={} opaque={}",
            counts[0], counts[1], counts[2], counts[3]
        );
        assert!((80..3200).contains(&(counts[0] + counts[1])));
        assert!((20..2400).contains(&counts[0]));
        assert!((20..2400).contains(&counts[1]));
        assert!(counts[3] > 20 && counts[3] <= counts[0] + counts[1] + counts[2]);
        for (total, count) in totals.iter_mut().zip(counts) {
            *total += count;
        }
    }
    assert!(
        (1..500).contains(&totals[2]),
        "calques outside band: {totals:?}"
    );
    let history = sample::sample();
    let world = history.latest();
    let coined = world
        .events
        .iter()
        .filter(|(_, e)| matches!(e, WorldEvent::Coined { .. }))
        .count();
    println!(
        "workbench sample: year={} coinage events={coined}; profile totals={totals:?}; elapsed={:.1}s",
        world.generation * 25,
        started.elapsed().as_secs_f32()
    );
    assert!((1..3000).contains(&coined));
}

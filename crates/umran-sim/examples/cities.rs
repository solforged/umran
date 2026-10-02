//! cargo run --release -p umran-sim --example cities -- [seed] [generations]
//! cargo run --release -p umran-sim --example cities -- --band [seeds] [generations]
//! The detailed case starts with an authored, populous farming state. The band
//! uses the audit's three 1,000-person founders and otherwise default rates.
use std::collections::BTreeSet;
use umran_sim::{ContactKind, Event, Livelihood, Params, SoundProfile, Terrain, World, WorldEvent};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--band") {
        let seeds = args.get(1).map_or(40, |s| s.parse().expect("seeds"));
        let generations = args.get(2).map_or(160, |s| s.parse().expect("generations"));
        band(seeds, generations);
        return;
    }
    let seed = args.first().map_or(42, |s| s.parse().expect("seed"));
    let generations = args.get(1).map_or(80, |s| s.parse().expect("generations"));
    let mut world = World::new(
        seed,
        Params {
            ethos_enabled: !args.iter().any(|a| a == "--neutral"),
            ..Params::default()
        },
    );
    let capital = world
        .map
        .regions
        .iter()
        .position(|r| r.terrain == Terrain::Plains)
        .unwrap();
    for (preset, size) in [
        ("germanic", 80_000.0),
        ("familiar", 70_000.0),
        ("semitic", 50_000.0),
    ] {
        let c = world.found_seeded(
            &umran_sim::Naming::People,
            &SoundProfile::by_id(preset).unwrap(),
            seed + world.communities.len() as u64,
            0.5,
            0.6,
            Some(capital),
            Some(Livelihood::Farming),
            None,
        );
        world.communities[c].size = size;
    }
    world.connect(0, 1, 0.8, ContactKind::Rule).unwrap();
    world.connect(0, 2, 0.8, ContactKind::Rule).unwrap();
    println!("City study: seed {seed}, {generations} generations; authored mixed farming capital");
    for _ in 0..generations {
        let events = world.events.len();
        world.step();
        let mut changed = false;
        for (generation, event) in &world.events[events..] {
            match *event {
                WorldEvent::City { city } => {
                    changed = true;
                    println!("year {}: city {city} crosses 10,000", generation * 25);
                }
                WorldEvent::Koine {
                    city,
                    community,
                    variety,
                } => {
                    changed = true;
                    let v = &world.varieties[variety];
                    println!(
                        "year {}: city {city} townsfolk {} form {} (variety {variety}, parent {})",
                        generation * 25,
                        world.community_name(community),
                        world.language_title(variety),
                        v.parent.unwrap().variety
                    );
                    println!("  contributors: {:?}", v.koine_of);
                    for word in v
                        .lexicon
                        .living()
                        .filter(|l| {
                            l.log.iter().any(|e| {
                                matches!(
                                    e.event,
                                    Event::SoundLaw {
                                        law: "koine-levelling",
                                        ..
                                    }
                                )
                            })
                        })
                        .take(6)
                    {
                        let before = word
                            .log
                            .iter()
                            .find_map(|e| match &e.event {
                                Event::SoundLaw {
                                    law: "koine-levelling",
                                    before,
                                } => Some(before),
                                _ => None,
                            })
                            .unwrap();
                        println!(
                            "  {}: /{}/ → /{}/",
                            word.first_sense.id,
                            before.ipa(),
                            word.form.ipa()
                        );
                    }
                }
                WorldEvent::Standard { state } => println!(
                    "year {}: state {state} selects variety {} as standard",
                    generation * 25,
                    world.standard_variety(state)
                ),
                _ => {}
            }
        }
        if changed || world.generation.is_multiple_of(10) {
            for (i, city) in world.cities.iter().enumerate() {
                println!(
                    "year {}: city {i}, people {:.0}, makeup {:?}, townsfolk {:?}, state standing {}",
                    world.generation * 25,
                    world.city_size(i),
                    city.makeup,
                    city.townsfolk,
                    world.states[city.state].standing()
                );
            }
        }
        for (to, v) in world.varieties.iter().enumerate() {
            for &(g, law, from) in v.waves.iter().filter(|(g, _, _)| *g == world.generation) {
                if !world.varieties[from].koine_of.is_empty() {
                    println!(
                        "year {}: city speech {from} sends wave {law} to variety {to}",
                        g * 25
                    );
                }
            }
        }
    }
}

fn band(seeds: u64, generations: u32) {
    assert!(seeds > 0, "at least one seed");
    let presets = SoundProfile::presets();
    let mut counts = Vec::new();
    let mut standards = 0;
    let mut worlds_with_standard = 0;
    for seed in 0..seeds {
        let pick = |i: u64| &presets[((seed * 3 + i) as usize * 7) % presets.len()];
        let mut world = World::new(
            seed,
            Params {
                ethos_enabled: !std::env::args().any(|a| a == "--neutral"),
                ..Params::default()
            },
        );
        let hill = world.found(pick(0), 0.5, 0.4);
        let home = world.communities[hill].home();
        let coast = world.found_seeded(
            &umran_sim::Naming::People,
            pick(1),
            seed.wrapping_add(1),
            0.4,
            0.6,
            Some(home),
            None,
            None,
        );
        let empire = world.found_seeded(
            &umran_sim::Naming::People,
            pick(2),
            seed.wrapping_add(2),
            0.85,
            0.3,
            Some(home),
            None,
            None,
        );
        world.connect(hill, coast, 0.5, ContactKind::Trade).unwrap();
        world
            .connect(empire, coast, 0.8, ContactKind::Rule)
            .unwrap();
        world
            .connect(empire, hill, 0.3, ContactKind::Neighbours)
            .unwrap();
        let mut adopted = BTreeSet::new();
        for _ in 0..generations {
            let events = world.events.len();
            world.step();
            for (_, event) in &world.events[events..] {
                if let WorldEvent::Standard { state } = *event {
                    let v = world.standard_variety(state);
                    if !world.varieties[v].koine_of.is_empty() {
                        adopted.insert(v);
                    }
                }
            }
        }
        let koines = world
            .varieties
            .iter()
            .filter(|v| !v.koine_of.is_empty())
            .count();
        println!(
            "seed {seed}: cities {}, koines {koines}, koines selected as standards {}",
            world.cities.len(),
            adopted.len()
        );
        counts.push(koines);
        standards += adopted.len();
        worlds_with_standard += usize::from(!adopted.is_empty());
    }
    counts.sort_unstable();
    let worlds = counts.iter().filter(|&&n| n > 0).count();
    let total: usize = counts.iter().sum();
    let median = (counts[(counts.len() - 1) / 2] + counts[counts.len() / 2]) as f32 / 2.0;
    println!(
        "BAND: {seeds} seeds × {} years; worlds with koine {worlds}/{seeds} ({:.1}%); median koines/world {median:.1}; total {total}; koines selected as standard {standards}/{total} ({:.1}%); worlds with koine standard {worlds_with_standard}/{seeds}",
        generations * 25,
        worlds as f32 * 100.0 / seeds as f32,
        standards as f32 * 100.0 / total.max(1) as f32
    );
}

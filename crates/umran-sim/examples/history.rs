//! Three peoples over a long span with automatic growth, splits, and
//! language shift: hill farmers, a coastal people, and an empire that rules
//! the coast.
//!
//! cargo run --release -p umran-sim --example history -- [seed] [generations]

use umran_sim::compare::intelligibility;
use umran_sim::{ContactKind, Params, SoundProfile, World, WorldEvent};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = args.first().map_or(42, |s| s.parse().expect("seed"));
    let generations: u32 = args.get(1).map_or(200, |s| s.parse().expect("generations"));

    let profile = |id: &str| SoundProfile::by_id(id).unwrap();
    let mut world = World::new(seed, Params::default());
    let hill = world.found(&profile("familiar"), 0.5, 0.4);
    let coast = world.found(&profile("polynesian"), 0.4, 0.6);
    let empire = world.found(&profile("iranian"), 0.85, 0.3);
    world.connect(hill, coast, 0.5, ContactKind::Trade);
    world.connect(empire, coast, 0.8, ContactKind::Rule);
    world.connect(empire, hill, 0.3, ContactKind::Neighbours);
    world.run(generations);

    println!(
        "seed {seed} · {generations} generations (about {} years)\n",
        generations * 25
    );
    println!("Events");
    let name = |c: usize| world.community_name(c);
    let place = |r: usize, g: u32| world.place_at(r, g).unwrap_or_else(|| format!("land {r}"));
    for (generation, event) in &world.events {
        match event {
            WorldEvent::Found { community } => {
                println!("  gen {generation:>3}  {} founded", name(*community))
            }
            WorldEvent::Split {
                community,
                daughter,
                to,
                ..
            } => {
                println!(
                    "  gen {generation:>3}  {} splits; {} founded in {}",
                    name(*community),
                    name(*daughter),
                    place(*to, *generation)
                )
            }
            WorldEvent::Migrated {
                community,
                from,
                to,
            } => println!(
                "  gen {generation:>3}  {} leave {} for {}{}",
                name(*community),
                place(*from, generation.saturating_sub(1)),
                place(*to, *generation),
                if world.map.overseas(*from, *to) {
                    " by sea"
                } else {
                    ""
                }
            ),
            WorldEvent::Shift {
                community,
                toward,
                variety,
                ..
            } => println!(
                "  gen {generation:>3}  {} shifts to {}'s language (variety {variety})",
                name(*community),
                name(*toward)
            ),
            WorldEvent::Met { a, b, kind } => {
                println!(
                    "  gen {generation:>3}  {} and {} meet ({kind:?})",
                    name(*a),
                    name(*b)
                )
            }
            WorldEvent::Parted { a, b, kind } => {
                println!(
                    "  gen {generation:>3}  {} and {} part ({kind:?})",
                    name(*a),
                    name(*b)
                )
            }
            WorldEvent::Conquered { ruler, ruled } => {
                println!(
                    "  gen {generation:>3}  {} conquers {}",
                    name(*ruler),
                    name(*ruled)
                )
            }
            WorldEvent::Spread { community, to } => println!(
                "  gen {generation:>3}  {} spread into {}",
                name(*community),
                place(*to, *generation)
            ),
            WorldEvent::Displaced {
                community,
                region,
                by,
            } => println!(
                "  gen {generation:>3}  {} crowded off {} by {}",
                name(*community),
                place(*region, *generation),
                name(*by)
            ),
            WorldEvent::HardTimes {
                region,
                kind,
                share,
            } => println!(
                "  gen {generation:>3}  {kind:?} in {} kills {:.0}%",
                place(*region, *generation),
                share * 100.0
            ),
            WorldEvent::Adopted {
                community,
                livelihood,
                from,
            } => println!(
                "  gen {generation:>3}  {} become {}{}",
                name(*community),
                livelihood.label(),
                from.map_or(String::new(), |f| format!(", taught by {}", name(f)))
            ),
            WorldEvent::Ended { community, into } => println!(
                "  gen {generation:>3}  {} {}",
                name(*community),
                into.map_or("die out".to_string(), |i| format!("merge into {}", name(i)))
            ),
            WorldEvent::Rose { state } => {
                let s = &world.states[*state];
                println!(
                    "  gen {generation:>3}  {} raise {} ({:?}, purism {:.2})",
                    name(s.rulers),
                    s.name.meaning,
                    s.how,
                    s.purism
                )
            }
            WorldEvent::Fell { state } => {
                let s = &world.states[*state];
                println!(
                    "  gen {generation:>3}  {} falls: {:?}",
                    s.name.meaning,
                    s.fell.map(|(_, how)| how)
                )
            }
            WorldEvent::Standard { state } => println!(
                "  gen {generation:>3}  {} takes its court speech as its standard",
                world.states[*state].name.meaning
            ),
        }
    }

    println!("\nCommunities now");
    for (i, c) in world.communities.iter().enumerate() {
        let mut lineage = vec![c.variety];
        while let Some(fork) = world.varieties[*lineage.last().unwrap()].parent {
            lineage.push(fork.variety);
        }
        let lineage: Vec<String> = lineage.iter().map(|v| format!("v{v}")).collect();
        let meaning = format!("\"{}\"", c.name.meaning);
        let language = &world.varieties[c.variety].name;
        println!(
            "  {:<16} {meaning:<34} size {:>5.0} prestige {:.2}  speaks {} ({}, \"{}\")",
            world.community_name(i),
            c.size,
            c.prestige,
            lineage.join(" ← "),
            world.language_title(c.variety),
            language.meaning,
        );
    }

    println!("\nPlaces");
    for (r, names) in world.places.iter().enumerate() {
        if names.is_empty() {
            continue;
        }
        let told: Vec<String> = names
            .iter()
            .map(|p| {
                format!(
                    "{} ({:?} g{}, v{})",
                    world.varieties[p.variety].title(&p.name.form),
                    p.origin,
                    p.since,
                    p.variety
                )
            })
            .collect();
        println!(
            "  {r:>3} {:<10} \"{}\": {}",
            format!("{:?}", world.map.regions[r].terrain),
            names[0].name.meaning,
            told.join(" → ")
        );
    }

    let spoken: Vec<usize> = (0..world.varieties.len())
        .filter(|&v| world.spoken()[v])
        .collect();
    println!("\nIntelligibility between spoken varieties (core vocabulary)");
    print!("        ");
    for v in &spoken {
        print!("{:>6}", format!("v{v}"));
    }
    println!();
    for a in &spoken {
        print!("  {:>5} ", format!("v{a}"));
        for b in &spoken {
            let score = intelligibility(&world.varieties[*a].lexicon, &world.varieties[*b].lexicon);
            print!("{score:>6.2}");
        }
        println!();
    }
    let extinct = world.varieties.len() - spoken.len();
    println!(
        "\n{} varieties in all, {extinct} no longer spoken",
        world.varieties.len()
    );
}

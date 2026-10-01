//! Three peoples over a long span with automatic growth, splits, and
//! language shift: hill farmers, a coastal people, and an empire that rules
//! the coast.
//!
//! cargo run --release -p langgen-sim --example history -- [seed] [generations]

use langgen_sim::compare::intelligibility;
use langgen_sim::{ContactKind, Params, SoundProfile, World, WorldEvent};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = args.first().map_or(42, |s| s.parse().expect("seed"));
    let generations: u32 = args.get(1).map_or(200, |s| s.parse().expect("generations"));

    let profile = |id: &str| SoundProfile::by_id(id).unwrap();
    let mut world = World::new(seed, Params::default());
    let hill = world.found("Hill", &profile("neutral"), 0.5, 0.4);
    let coast = world.found("Coast", &profile("kuo-toa"), 0.4, 0.6);
    let empire = world.found("Empire", &profile("illithid"), 0.85, 0.3);
    world.connect(hill, coast, 0.5, ContactKind::Trade);
    world.connect(empire, coast, 0.8, ContactKind::Rule);
    world.connect(empire, hill, 0.3, ContactKind::Neighbours);
    world.run(generations);

    println!(
        "seed {seed} · {generations} generations (about {} years)\n",
        generations * 25
    );
    println!("Events");
    let name = |c: usize| world.communities[c].name.clone();
    for (generation, event) in &world.events {
        match event {
            WorldEvent::Split {
                community,
                daughter,
            } => {
                println!(
                    "  gen {generation:>3}  {} splits; {} founded",
                    name(*community),
                    name(*daughter)
                )
            }
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
        }
    }

    println!("\nCommunities now");
    for c in &world.communities {
        let mut lineage = vec![c.variety];
        while let Some(fork) = world.varieties[*lineage.last().unwrap()].parent {
            lineage.push(fork.variety);
        }
        let lineage: Vec<String> = lineage.iter().map(|v| format!("v{v}")).collect();
        println!(
            "  {:<16} size {:>5.0} prestige {:.2}  speaks {}",
            c.name,
            c.size,
            c.prestige,
            lineage.join(" ← ")
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

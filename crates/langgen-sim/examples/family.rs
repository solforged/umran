//! A proto-language splits; one daughter is then ruled by outsiders and
//! borrows from them. An automated comparative method then compares the
//! daughters without seeing lineage, and is graded against it.
//!
//! cargo run --release -p langgen-sim --example family -- [seed] [proto profile] [outsider profile] [generations]

use langgen_sim::compare::{Pair, Settings, compare, regular_correspondences};
use langgen_sim::{CATALOG, CONCEPTS, ContactKind, Origin, Params, SoundProfile, World, catalog};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize, default: &str| args.get(i).cloned().unwrap_or_else(|| default.into());
    let seed: u64 = arg(0, "42").parse().expect("seed");
    let proto = SoundProfile::by_id(&arg(1, "familiar")).expect("proto profile");
    let outsider = SoundProfile::by_id(&arg(2, "iranian")).expect("outsider profile");
    let generations: u32 = arg(3, "40").parse().expect("generations");

    let mut world = World::new(seed, Params::static_society());
    let west = world.found(&proto, 0.5, 0.5);
    let outsiders = world.found(&outsider, 0.9, 0.3);
    world.run(5);
    let east = world.split(west, None, 0.0);
    world.connect(outsiders, east, 0.8, ContactKind::Rule);
    world.run(generations);

    let (wv, ev) = (
        world.communities[west].variety,
        world.communities[east].variety,
    );
    let laws = catalog();
    let label = |id: &str| {
        laws.iter()
            .find(|l| l.id == id)
            .map_or(id.to_string(), |l| l.label.to_string())
    };
    for (name, v) in [("West", wv), ("East", ev)] {
        println!("{name} sound laws since the split:");
        for (g, id) in &world.varieties[v].laws {
            if *g > 5 {
                println!("  gen {g:>3}  {}", label(id));
            }
        }
    }

    let concepts: Vec<_> = CONCEPTS.iter().collect();
    let result = compare(
        &world.varieties[wv].lexicon,
        &world.varieties[ev].lexicon,
        &concepts,
    );
    let truly: Vec<bool> = result
        .rows
        .iter()
        .map(|r| world.cognate(wv, ev, r.concept))
        .collect();
    let truth = regular_correspondences(
        result
            .rows
            .iter()
            .zip(&truly)
            .filter(|(_, t)| **t)
            .map(|(r, _)| r.alignment.as_slice()),
        &Settings::default(),
    );
    let show = |p: &Pair| {
        let s = |x: Option<langgen_sim::PhonemeId>| x.map_or("∅", |p| CATALOG.get(p).ipa());
        format!("{}:{}", s(p.0), s(p.1))
    };
    let changed = |list: &[(Pair, u32)]| -> Vec<Pair> {
        list.iter()
            .map(|(p, _)| *p)
            .filter(|p| p.0 != p.1)
            .collect()
    };
    let (found, real) = (changed(&result.regular), changed(&truth));
    println!("\nRegular correspondences West:East that are not identities");
    println!(
        "  recovered  {}",
        found.iter().map(show).collect::<Vec<_>>().join("  ")
    );
    println!(
        "  true       {}",
        real.iter().map(show).collect::<Vec<_>>().join("  ")
    );

    let east_lexicon = &world.varieties[ev].lexicon;
    let is_outside_loan = |c: &langgen_sim::Concept| {
        east_lexicon
            .word_for(c)
            .is_some_and(|l| matches!(l.origin, Origin::Borrowed { from, .. } if from == world.communities[outsiders].variety))
    };
    let (mut tp, mut fp, mut fn_, mut tn, mut loans, mut loans_flagged) = (0, 0, 0, 0, 0, 0);
    for (row, &cognate) in result.rows.iter().zip(&truly) {
        match (cognate, row.cognate) {
            (true, true) => tp += 1,
            (true, false) => fn_ += 1,
            (false, true) => fp += 1,
            (false, false) => tn += 1,
        }
        if is_outside_loan(row.concept) {
            loans += 1;
            loans_flagged += usize::from(!row.cognate);
        }
    }
    println!("\nCognate judgments over {} concepts", result.rows.len());
    println!("  true cognates kept {tp}, wrongly flagged {fn_}");
    println!("  non-cognates flagged {tn}, missed {fp}");
    println!("  East's loans from the outsiders flagged {loans_flagged} of {loans}");

    println!("\nSample of flagged pairs (West ~ East)");
    for row in result.rows.iter().filter(|r| !r.cognate).take(12) {
        let why = if is_outside_loan(row.concept) {
            "loan from outsiders"
        } else if world.cognate(wv, ev, row.concept) {
            "cognate (false alarm)"
        } else {
            "replaced"
        };
        println!(
            "  {:<10} {:>8} ~ {:<8} {:.2}  {why}",
            row.concept.id,
            row.a.ipa(),
            row.b.ipa(),
            row.regularity
        );
    }
}

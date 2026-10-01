//! Runs one variety through generations and prints what happened.
//!
//! cargo run -p langgen-sim --example drift -- [seed] [profile] [generations] [flavor...]
//! e.g. cargo run -p langgen-sim --example drift -- 42 neutral 40 pie-like

use langgen_sim::{
    CATALOG, Event, Flavor, Form, Lexeme, Origin, Params, SoundProfile, World, catalog,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = args
        .first()
        .map_or(42, |s| s.parse().expect("seed must be a number"));
    let profile_id = args.get(1).map_or("germanic", String::as_str);
    let generations: u32 = args
        .get(2)
        .map_or(40, |s| s.parse().expect("generations must be a number"));
    let mut profile = SoundProfile::by_id(profile_id).unwrap_or_else(|| {
        let ids: Vec<_> = SoundProfile::presets().into_iter().map(|p| p.id).collect();
        panic!(
            "unknown profile {profile_id}; try one of {}",
            ids.join(", ")
        )
    });
    for id in args.iter().skip(3) {
        let flavor = Flavor::by_id(id).unwrap_or_else(|| {
            let ids: Vec<_> = Flavor::examples().into_iter().map(|f| f.id).collect();
            panic!("unknown flavor {id}; try one of {}", ids.join(", "))
        });
        profile = profile.flavored(&flavor);
    }

    let mut sim = World::solo(seed, &profile, Params::default());
    let founding = sim.varieties[0].clone();
    sim.run(generations);
    let variety = &sim.varieties[0];
    let lexicon = &variety.lexicon;

    println!(
        "{} · seed {seed} · {generations} generations (about {} years)",
        profile.name,
        generations * 25
    );
    let ipa = |ids: Vec<langgen_sim::PhonemeId>| -> String {
        ids.iter()
            .map(|id| CATALOG.get(*id).ipa())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let (c0, v0) = founding.inventory();
    let (c1, v1) = variety.inventory();
    println!("consonants  {}  →  {}", ipa(c0), ipa(c1));
    println!("vowels      {}  →  {}", ipa(v0), ipa(v1));

    println!("\nSound laws");
    let laws = catalog();
    for &(generation, id) in &variety.laws {
        let law = laws.iter().find(|l| l.id == id).unwrap();
        let changed: Vec<(&Lexeme, &Form, &Form)> = lexicon
            .lexemes
            .iter()
            .filter_map(|l| {
                let at = l
                    .log
                    .iter()
                    .position(|e| matches!(&e.event, Event::SoundLaw { law, .. } if *law == id))?;
                let Event::SoundLaw { before, .. } = &l.log[at].event else {
                    unreachable!()
                };
                let after = l.log[at + 1..]
                    .iter()
                    .find_map(|e| match &e.event {
                        Event::SoundLaw { before, .. } => Some(before),
                        _ => None,
                    })
                    .unwrap_or(&l.form);
                Some((l, before, after))
            })
            .collect();
        let examples: Vec<String> = changed
            .iter()
            .take(4)
            .map(|(l, before, after)| {
                format!("{} → {} '{}'", before.ipa(), after.ipa(), l.first_sense.id)
            })
            .collect();
        println!(
            "  gen {generation:>3}  {:<28} {:>3} words  {}",
            law.label,
            changed.len(),
            examples.join(", ")
        );
        if law.label.len() > 28 {
            println!("           ({id})");
        }
    }

    println!("\nReplacements (concept: founding word → today's dominant word)");
    let mut replaced = 0;
    for slot in &lexicon.slots {
        let Some(now) = slot.dominant().map(|id| lexicon.get(id)) else {
            continue;
        };
        if lexicon.keeps_founding_word(slot.concept) {
            continue;
        }
        replaced += 1;
        let old = founding.lexicon.word_for(slot.concept).unwrap();
        let how = match now.origin {
            Origin::Founding => format!("extended from '{}'", now.first_sense.gloss),
            Origin::Expressive => format!("new root, gen {}", now.born),
            Origin::Derived { relation, .. } => {
                format!("derived ({}), gen {}", relation.label(), now.born)
            }
            Origin::Borrowed { from, .. } => {
                format!("borrowed from variety {from}, gen {}", now.born)
            }
        };
        let rank = slot
            .concept
            .stability
            .map_or("culture".to_string(), |r| format!("rank {r}"));
        println!(
            "  {:<16} {:<8} {:>10} → {:<10} {how}",
            slot.concept.id,
            rank,
            founding.spell(&old.form),
            variety.spell(&now.form)
        );
    }
    println!("  {replaced} concepts replaced");

    println!("\nCore words, most stable first (founding → now)");
    let rows: Vec<String> = lexicon
        .slots
        .iter()
        .filter(|s| s.concept.stability.is_some_and(|r| r <= 40))
        .map(|s| {
            let then = founding.lexicon.word_for(s.concept).unwrap();
            let now = lexicon.word_for(s.concept).unwrap();
            format!(
                "{:<9} {:>7} → {:<7}",
                s.concept.id,
                founding.spell(&then.form),
                variety.spell(&now.form)
            )
        })
        .collect();
    for row in rows.chunks(3) {
        println!("  {}", row.join("   "));
    }

    let living = lexicon.living().count();
    let obsolete = lexicon.lexemes.len() - living;
    let contested = lexicon
        .slots
        .iter()
        .filter(|s| s.variants.len() > 1)
        .count();
    println!(
        "\nCore retention {:.0}% · {living} living words, {obsolete} obsolete · {contested} concepts contested now",
        lexicon.core_retention() * 100.0
    );
}

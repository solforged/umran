//! Averages drift statistics over many seeds, for tuning `Params`.
//!
//! cargo run --release -p langgen-sim --example calibrate -- [seeds] [generations]

use langgen_sim::{Lexicon, Params, Sim, SoundProfile};
use std::collections::HashMap;

fn kept(lexicon: &Lexicon, ranks: std::ops::RangeInclusive<u8>) -> f32 {
    let slots: Vec<_> = lexicon
        .slots
        .iter()
        .filter(|s| s.concept.stability.is_some_and(|r| ranks.contains(&r)))
        .collect();
    let kept = slots
        .iter()
        .filter(|s| lexicon.keeps_founding_word(s.concept))
        .count();
    kept as f32 / slots.len() as f32
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: usize = args.next().map_or(200, |s| s.parse().expect("seeds"));
    let generations: u32 = args.next().map_or(40, |s| s.parse().expect("generations"));

    let mut sums: HashMap<&str, f32> = HashMap::new();
    let profiles = SoundProfile::examples();
    for n in 0..seeds {
        let profile = &profiles[n % profiles.len()];
        let mut sim = Sim::new(n as u64, profile, Params::default());
        sim.run(generations);
        let lexicon = &sim.variety.lexicon;
        let culture: Vec<_> = lexicon
            .slots
            .iter()
            .filter(|s| s.concept.stability.is_none())
            .collect();
        let culture_kept = culture
            .iter()
            .filter(|s| lexicon.keeps_founding_word(s.concept))
            .count() as f32
            / culture.len() as f32;
        let mut forms: HashMap<&langgen_sim::Form, usize> = HashMap::new();
        for slot in &lexicon.slots {
            if let Some(id) = slot.dominant() {
                *forms.entry(&lexicon.get(id).form).or_default() += 1;
            }
        }
        let distinct_ids: std::collections::HashSet<_> =
            lexicon.slots.iter().filter_map(|s| s.dominant()).collect();
        let mut add = |k, v: f32| *sums.entry(k).or_default() += v;
        add("core retention", lexicon.core_retention());
        add("ranks 1-20 kept", kept(lexicon, 1..=20));
        add("ranks 81-100 kept", kept(lexicon, 81..=100));
        add("culture kept", culture_kept);
        add("sound laws", sim.variety.laws.len() as f32);
        add(
            "contested concepts",
            lexicon
                .slots
                .iter()
                .filter(|s| s.variants.len() > 1)
                .count() as f32,
        );
        add(
            "polysemous dominant words",
            (lexicon.slots.len() - distinct_ids.len()) as f32,
        );
        add(
            "homophones among other words",
            forms.values().map(|&n| n.saturating_sub(1)).sum::<usize>() as f32
                - (lexicon.slots.len() - distinct_ids.len()) as f32,
        );
    }
    println!("{seeds} seeds × {generations} generations, profiles in rotation");
    let mut rows: Vec<_> = sums.into_iter().collect();
    rows.sort_by_key(|(k, _)| *k);
    for (k, v) in rows {
        println!("  {k:<30} {:.3}", v / seeds as f32);
    }
}

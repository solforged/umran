//! Prints a newly founded variety: inventory, then every root by field.
//!
//! cargo run -p langgen-sim --example found -- [seed] [profile]

use langgen_sim::{CATALOG, Form, SoundProfile, Variety};
use std::collections::HashMap;

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args
        .next()
        .map_or(42, |s| s.parse().expect("seed must be a number"));
    let profile_id = args.next().unwrap_or_else(|| "elvish".into());
    let profile = SoundProfile::by_id(&profile_id).unwrap_or_else(|| {
        let ids: Vec<_> = SoundProfile::examples().into_iter().map(|p| p.id).collect();
        panic!(
            "unknown profile {profile_id}; try one of {}",
            ids.join(", ")
        )
    });
    let variety = Variety::found(seed, &profile);

    let ipa = |ids: &[langgen_sim::PhonemeId]| -> Vec<&str> {
        ids.iter().map(|id| CATALOG.get(*id).ipa()).collect()
    };
    println!("{} · seed {seed}", profile.name);
    let (consonants, vowels) = variety.inventory();
    println!("consonants  {}", ipa(&consonants).join(" "));
    println!("vowels      {}", ipa(&vowels).join(" "));

    let lexicon = &variety.lexicon;
    let mut words: Vec<_> = lexicon
        .slots
        .iter()
        .filter_map(|s| Some((s.concept, &lexicon.word_for(s.concept)?.form)))
        .collect();
    words.sort_by_key(|(concept, _)| concept.field);
    let mut field = None;
    for &(concept, form) in &words {
        if field != Some(concept.field) {
            field = Some(concept.field);
            println!("\n{:?}", concept.field);
        }
        println!(
            "  {:<12} /{}/{:<width$} {}",
            variety.spell(form),
            form.ipa(),
            "",
            concept.gloss,
            width = 8usize.saturating_sub(form.ipa().chars().count()),
        );
    }

    let mut by_form: HashMap<&Form, Vec<&str>> = HashMap::new();
    for &(concept, form) in &words {
        by_form.entry(form).or_default().push(concept.id);
    }
    let mut homophones: Vec<_> = by_form
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(form, ids)| format!("{} = {}", variety.spell(form), ids.join(", ")))
        .collect();
    homophones.sort();
    println!(
        "\n{} words, {} homophone sets",
        words.len(),
        homophones.len()
    );
    for h in homophones {
        println!("  {h}");
    }
}

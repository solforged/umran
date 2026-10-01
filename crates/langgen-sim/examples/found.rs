//! Prints a newly founded variety: inventory, then every root by field.
//!
//! cargo run -p langgen-sim --example found -- [seed] [preset]

use langgen_sim::{CATALOG, Preset, Variety};
use std::collections::HashMap;

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args
        .next()
        .map_or(42, |s| s.parse().expect("seed must be a number"));
    let preset_id = args.next().unwrap_or_else(|| "elvish".into());
    let preset = Preset::by_id(&preset_id).unwrap_or_else(|| {
        let ids: Vec<_> = Preset::all().into_iter().map(|p| p.id).collect();
        panic!("unknown preset {preset_id}; try one of {}", ids.join(", "))
    });
    let variety = Variety::found(seed, &preset);

    let ipa = |ids: &[langgen_sim::PhonemeId]| -> Vec<&str> {
        ids.iter().map(|id| CATALOG.get(*id).ipa()).collect()
    };
    println!("{} · seed {seed}", preset.name);
    println!(
        "consonants  {}",
        ipa(&variety.inventory.consonants).join(" ")
    );
    println!("vowels      {}", ipa(&variety.inventory.vowels).join(" "));

    let mut words: Vec<_> = variety.words.iter().collect();
    words.sort_by_key(|w| w.concept.field);
    let mut field = None;
    for word in words {
        if field != Some(word.concept.field) {
            field = Some(word.concept.field);
            println!("\n{:?}", word.concept.field);
        }
        println!(
            "  {:<12} /{}/{:<width$} {}",
            variety.spell(&word.form),
            word.form.ipa(),
            "",
            word.concept.gloss,
            width = 8usize.saturating_sub(word.form.ipa().chars().count()),
        );
    }

    let mut by_form: HashMap<&langgen_sim::Form, Vec<&str>> = HashMap::new();
    for word in &variety.words {
        by_form.entry(&word.form).or_default().push(word.concept.id);
    }
    let mut homophones: Vec<_> = by_form
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(form, ids)| format!("{} = {}", variety.spell(form), ids.join(", ")))
        .collect();
    homophones.sort();
    println!(
        "\n{} words, {} homophone sets",
        variety.words.len(),
        homophones.len()
    );
    for h in homophones {
        println!("  {h}");
    }
}

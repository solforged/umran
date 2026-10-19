//! Founds a language from the familiar preset with an author's own roots,
//! runs a hundred generations, and prints each root's year-2500 form and
//! law count beside a drawn word of the same field.
//!
//! cargo run --release -p umran-sim --example roots -- [seed]

use umran_sim::concepts::by_id;
use umran_sim::lexicon::Event;
use umran_sim::{
    CATALOG, CONCEPTS, Form, LanguageDesign, Lexeme, Naming, Params, Root, Sound, World,
};

const ROOTS: [(&str, &str); 10] = [
    ("water", "nara"),
    ("fire", "pata"),
    ("stone", "gora"),
    ("chief", "kaːr"),
    ("song", "ʃaːn"),
    ("wolf", "waga"),
    ("death", "mora"),
    ("peace", "peli"),
    ("dream", "juna"),
    ("moon", "meːn"),
];

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .map_or(1319325112, |s| s.parse().expect("seed must be a number"));
    let mut design = LanguageDesign::preset("familiar", seed).expect("familiar preset");
    for (concept, ipa) in ROOTS {
        for seg in Form::from_ipa(ipa).expect("catalog IPA").segs {
            let ipa = CATALOG.get(seg.phone).ipa();
            if !design.sounds.iter().any(|s| s.ipa == ipa) {
                design.sounds.push(Sound {
                    ipa: ipa.into(),
                    favoured: false,
                });
            }
        }
        design.roots.push(Root {
            concept: concept.into(),
            ipa: ipa.into(),
        });
    }
    design.validate().expect("the roots fit the design");

    let mut world = World::new(seed, Params::default());
    world.found_seeded(
        &Naming::People,
        &design.profile(),
        seed,
        0.5,
        0.5,
        None,
        None,
        None,
    );
    let generations = 100;
    world.run(generations);
    let variety = &world.varieties[0];
    let laws = |word: &Lexeme| {
        word.log
            .iter()
            .filter(|e| matches!(e.event, Event::SoundLaw { .. }))
            .count()
    };
    let describe = |word: &Lexeme| {
        let note = match word.obsolete {
            Some(g) => format!(", obsolete since {g}"),
            None => String::new(),
        };
        format!(
            "{:<8} /{}/{:<width$} {} laws{note}",
            variety.spell(&word.form),
            word.form.ipa_stressed(variety.stress()),
            "",
            laws(word),
            width = 8usize.saturating_sub(word.form.ipa().chars().count()),
        )
    };
    println!(
        "seed {seed}, {} laws over {generations} generations (year {})",
        variety.laws.len(),
        generations * 25
    );
    println!(
        "{:<8} {:<9} {:<30}   {:<8} year 2500",
        "meaning", "root", "year 2500", "drawn"
    );
    let position = |id: &str| CONCEPTS.iter().position(|c| c.id == id).unwrap_or(0);
    for (concept, ipa) in ROOTS {
        let concept = by_id(concept).expect("catalog concept");
        let word = variety
            .lexicon
            .lexemes
            .iter()
            .find(|l| l.authored && l.first_sense.id == concept.id)
            .expect("the root is in the lexicon");
        // The nearest drawn word of the same field after it in the catalog.
        let drawn = || {
            variety.lexicon.lexemes.iter().filter(|l| {
                !l.authored
                    && l.born == 0
                    && l.obsolete.is_none()
                    && l.first_sense.field == concept.field
            })
        };
        let neighbour = drawn()
            .find(|l| position(l.first_sense.id) > position(concept.id))
            .or_else(|| drawn().next())
            .expect("a drawn word of the field");
        println!(
            "{:<8} /{ipa}/{:<width$} {:<30}   {:<8} {}",
            concept.id,
            "",
            describe(word),
            neighbour.first_sense.id,
            describe(neighbour),
            width = 7usize.saturating_sub(ipa.chars().count()),
        );
    }
}

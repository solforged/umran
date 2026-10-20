//! The sample world's three families with the author's own meanings.
//! cargo run --release -p umran-sim --example meanings
use umran_sim::{
    Action, CATALOG, Chronicle, Class, ContactKind, Craft, Derivation, Field, LanguageDesign,
    MapSize, Meaning, Naming, Tier,
};

fn show(history: &Chronicle) {
    let world = history.latest();
    println!("\nyear {}", world.generation * 25);
    for (v, variety) in world.varieties.iter().enumerate() {
        let words: Vec<_> = ["hearth", "anvil", "vow"]
            .into_iter()
            .map(|id| {
                match world
                    .concepts
                    .by_id(id)
                    .and_then(|c| variety.lexicon.word_for(c))
                {
                    Some(word) => format!(
                        "{id}={} /{}/ ({:?})",
                        variety.spell(&word.form),
                        word.form.ipa_stressed(variety.stress()),
                        word.origin
                    ),
                    None => format!("{id}=—"),
                }
            })
            .collect();
        println!("  {v} {}: {}", world.language_title(v), words.join("; "));
    }
}

fn main() {
    let mut history = Chronicle::new(21, MapSize::Medium);
    for (id, field, tier, base, relation) in [
        (
            "hearth",
            Field::House,
            Tier::Everyday,
            "fire",
            umran_sim::Relation::Place,
        ),
        (
            "anvil",
            Field::BasicActions,
            Tier::Specialized,
            "iron",
            umran_sim::Relation::Instrument,
        ),
    ] {
        history
            .act(Action::Meaning {
                meaning: Meaning {
                    id: id.into(),
                    gloss: id.into(),
                    field,
                    class: Class::Entity,
                    countable: true,
                    tier,
                    from: Some(Derivation {
                        base: base.into(),
                        relation,
                    }),
                },
            })
            .unwrap();
    }
    for (preset, seed, naming, power, region) in [
        (
            "germanic",
            31,
            Naming::Place {
                place: "river".into(),
            },
            0.7,
            448,
        ),
        ("semitic", 52, Naming::People, 0.5, 1722),
        (
            "polynesian",
            73,
            Naming::Place {
                place: "sea".into(),
            },
            0.4,
            33,
        ),
    ] {
        history
            .act(Action::Found {
                naming,
                design: LanguageDesign::preset(preset, seed).unwrap(),
                seed,
                power,
                openness: 0.5,
                region: Some(region),
                livelihood: None,
                ethos: None,
            })
            .unwrap();
    }
    history
        .act(Action::State {
            community: 2,
            capital: None,
        })
        .unwrap();
    history
        .act(Action::Craft {
            community: 2,
            craft: Craft::Writing,
        })
        .unwrap();
    show(&history);
    history.act(Action::Run { generations: 48 }).unwrap();
    history
        .act(Action::Meaning {
            meaning: Meaning {
                id: "vow".into(),
                gloss: "vow".into(),
                field: Field::Law,
                class: Class::Event,
                countable: false,
                tier: Tier::Everyday,
                from: None,
            },
        })
        .unwrap();
    let world = history.latest();
    let variety = world.communities[world.living().next().unwrap()].variety;
    let (consonants, vowels) = world.varieties[variety].inventory();
    let (c, v) = (
        CATALOG.get(consonants[0]).ipa(),
        CATALOG.get(vowels[0]).ipa(),
    );
    history
        .act(Action::Word {
            variety,
            concept: "vow".into(),
            ipa: format!("{c}{v}{c}{v}"),
        })
        .unwrap();
    show(&history);
    history.act(Action::Run { generations: 32 }).unwrap();
    show(&history);
    history.act(Action::Run { generations: 20 }).unwrap();
    history
        .act(Action::Connect {
            a: 0,
            b: 2,
            intensity: 0.8,
            contact: ContactKind::Rule,
        })
        .unwrap();
    history.act(Action::Religion { community: 2 }).unwrap();
    history
        .act(Action::Shift {
            community: 2,
            toward: 0,
        })
        .unwrap();
    history.act(Action::Run { generations: 60 }).unwrap();
    show(&history);
}

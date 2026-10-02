//! Stress rules and length-related changes in several founding palettes.
//! cargo run --release -p umran-sim --example stress -- [seed] [generations]
use umran_sim::{Event, Params, SoundProfile, World, catalog};

fn main() {
    let mut args = std::env::args().skip(1);
    let seed = args.next().map_or(7, |s| s.parse().expect("seed"));
    let generations = args.next().map_or(160, |s| s.parse().expect("generations"));
    let laws = catalog();
    for preset in ["germanic", "polynesian", "semitic", "finnic"] {
        let mut world = World::solo(
            seed,
            &SoundProfile::by_id(preset).unwrap(),
            Params::default(),
        );
        println!(
            "\n{preset}, seed {seed}: founding stress {}",
            world.varieties[0].stress().id()
        );
        for generation in 1..=generations {
            let old: Vec<_> = world.varieties.iter().map(|v| v.stress()).collect();
            world.run(1);
            for (id, variety) in world.varieties.iter().enumerate() {
                let mut stress = old.get(id).copied().unwrap_or(variety.stress());
                for &(g, law_id) in &variety.laws {
                    if g != generation {
                        continue;
                    }
                    let Some(law) = laws.iter().find(|l| l.id == law_id) else {
                        continue;
                    };
                    let after_stress = law.stress.unwrap_or(stress);
                    if matches!(
                        law_id,
                        "unstressed-reduction"
                            | "unstressed-syncope"
                            | "unstressed-apocope"
                            | "stressed-open-lengthening"
                            | "verner-voicing"
                            | "cluster-gemination"
                            | "j-gemination"
                            | "degemination"
                            | "romance-lenition"
                            | "compensatory-lengthening"
                            | "initial-stress"
                            | "penult-stress"
                    ) {
                        let example = variety.lexicon.lexemes.iter().find_map(|word| {
                            let at = word.log.iter().position(|e| e.generation == g && matches!(e.event, Event::SoundLaw { law, .. } if law == law_id))?;
                            let Event::SoundLaw { before, .. } = &word.log[at].event else { return None };
                            let after = word.log[at + 1..].iter().find_map(|e| match &e.event { Event::SoundLaw { before, .. } => Some(before), _ => None }).unwrap_or(&word.form);
                            Some(format!("{}: /{}/ > /{}/", word.first_sense.gloss, before.ipa_stressed(stress), after.ipa_stressed(after_stress)))
                        }).unwrap_or_default();
                        println!(
                            "year {:4}, language {id}, stress {}: {} — {example}",
                            g * 25,
                            after_stress.id(),
                            law.label
                        );
                    }
                    stress = after_stress;
                }
            }
        }
    }
}

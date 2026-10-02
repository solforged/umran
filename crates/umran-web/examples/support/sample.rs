use umran_sim::{Action, Chronicle, ContactKind, Craft, LanguageDesign, MapSize, Naming};

/// The actions in web/src/sample.ts, including its fixed map and language seeds.
pub fn sample() -> Chronicle {
    let mut history = Chronicle::new(21, MapSize::Medium);
    for (preset, seed, naming, power, region) in [
        (
            "germanic",
            31,
            Naming::Place {
                place: "river".into(),
            },
            0.7,
            73,
        ),
        ("semitic", 52, Naming::People, 0.5, 99),
        (
            "polynesian",
            73,
            Naming::Place {
                place: "sea".into(),
            },
            0.4,
            76,
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
    for action in [
        Action::State {
            community: 2,
            capital: None,
        },
        Action::Craft {
            community: 2,
            craft: Craft::Writing,
        },
        Action::Run { generations: 100 },
        Action::Connect {
            a: 0,
            b: 2,
            intensity: 0.8,
            contact: ContactKind::Rule,
        },
        Action::Religion { community: 2 },
        Action::Shift {
            community: 2,
            toward: 0,
        },
        Action::Run { generations: 60 },
    ] {
        history.act(action).unwrap();
    }
    history
}

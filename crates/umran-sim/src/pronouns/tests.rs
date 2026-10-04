use super::*;
use crate::{Ethos, Livelihood, MinimalWord, Params, Rise, SoundProfile, StressRule};

fn word<'a>(v: &'a Variety, cell: &str) -> &'a crate::Lexeme {
    v.lexicon.word_for(by_id(cell).unwrap()).unwrap()
}

fn put(v: &mut Variety, cell: &str, ipa: &str) {
    let id = word(v, cell).id;
    v.lexicon.get_mut(id).form = Form::from_ipa(ipa).unwrap();
}

#[test]
fn founding_cells_are_distinct_and_use_the_founding_sounds() {
    for seed in 0..12 {
        for profile in ["germanic", "semitic", "polynesian", "finnic"] {
            let v = Variety::found(
                seed,
                &SoundProfile::by_id(profile).unwrap(),
                Livelihood::Farming,
                Ethos::default(),
            );
            for (_, _, cell) in CELLS {
                let form = &word(&v, cell).form;
                assert!(form.vowel_count() > 0);
                assert!(!clashes(&v, cell, form));
                assert!(
                    form.phones()
                        .all(|p| v.founding_inventory.consonants.contains(&p)
                            || v.founding_inventory.vowels.contains(&p))
                );
            }
        }
    }
}

#[test]
fn renewal_copies_a_noun_and_regular_laws_leave_retired_forms_alone() {
    let mut world = World::solo(7, &SoundProfile::base(), Params::static_society());
    world.params.pronoun_rate = 1.0;
    let v = &mut world.varieties[0];
    v.minimal = MinimalWord::Syllable;
    v.profile.stress = Some(StressRule::Initial);
    for (_, _, cell) in CELLS {
        put(v, cell, "kata");
    }
    let old = word(v, "1sg").id;
    world.generation = 1;
    world.evolve_pronouns(&[true]);
    let v = &world.varieties[0];
    let renewed = word(v, "1sg");
    let Origin::Renewed { base, with: None } = renewed.origin else {
        panic!("renewed from noun")
    };
    assert_eq!(renewed.form, v.lexicon.get(base).form);
    assert_eq!(v.lexicon.get(old).obsolete, Some(1));
    assert!(!clashes(v, "1sg", &renewed.form));
    assert!(matches!(
        v.pronouns.events[0].event,
        NoticeKind::Renewed { merger: true, .. }
    ));
    let before = renewed.form.clone();
    let law = crate::catalog()
        .into_iter()
        .find(|l| l.id == "apocope")
        .unwrap();
    let expected = law.apply(&before, v.minimal, v.stress());
    world.generation = 2;
    world.apply_law(0, &law);
    assert_eq!(word(&world.varieties[0], "1sg").form, expected);
    assert_eq!(world.varieties[0].lexicon.get(old).form.ipa(), "kata");
    for (_, _, cell) in CELLS {
        assert!(word(&world.varieties[0], cell).form.vowel_count() > 0);
    }
}

#[test]
fn regular_laws_reach_all_six_cells_and_obey_the_size_floor() {
    let mut world = World::solo(7, &SoundProfile::base(), Params::static_society());
    let v = &mut world.varieties[0];
    v.minimal = MinimalWord::TwoSyllables;
    for (_, _, cell) in CELLS {
        put(v, cell, "hakata");
    }
    let law = crate::catalog()
        .into_iter()
        .find(|l| l.id == "h-loss")
        .unwrap();
    world.apply_law(0, &law);
    for (_, _, cell) in CELLS {
        assert_eq!(word(&world.varieties[0], cell).form.ipa(), "akata");
    }
    let law = crate::catalog()
        .into_iter()
        .find(|l| l.id == "apocope")
        .unwrap();
    world.apply_law(0, &law);
    world.apply_law(0, &law);
    for (_, _, cell) in CELLS {
        assert_eq!(word(&world.varieties[0], cell).form.vowel_count(), 2);
    }
}

#[test]
fn court_address_needs_an_old_standing_state_and_records_its_cause() {
    let mut world = World::solo(8, &SoundProfile::base(), Params::static_society());
    world.params.pronoun_rate = 1.0;
    world.polite_pronouns();
    assert!(!world.varieties[0].pronouns.polite);
    let state = world.raise_state(0, None, Rise::Proclaimed);
    world.generation = 7;
    world.polite_pronouns();
    assert!(!world.varieties[0].pronouns.polite);
    world.generation = 8;
    world.polite_pronouns();
    let v = &world.varieties[0];
    assert_eq!(word(v, "2sg").form, word(v, "2pl").form);
    assert!(v.pronouns.polite);
    assert!(!clashes(v, "2sg", &word(v, "2sg").form));
    let notice = v.pronouns.events.last().unwrap();
    assert!(matches!(notice.event, NoticeKind::Polite { state: s } if s == state));
    let cause = notice.cause.unwrap();
    assert_eq!(cause.mechanism, Mechanism::Court);
    assert!(
        matches!(world.events[cause.event].1, crate::WorldEvent::Rose { state: s } if s == state)
    );
    let mut fallen = World::solo(8, &SoundProfile::base(), Params::static_society());
    fallen.params.pronoun_rate = 1.0;
    let state = fallen.raise_state(0, None, Rise::Proclaimed);
    fallen.states[state].fell = Some((1, crate::Fall::Collapsed));
    fallen.generation = 8;
    fallen.polite_pronouns();
    assert!(!fallen.varieties[0].pronouns.polite);
}

#[test]
fn borrowing_requires_intense_sustained_intimate_contact() {
    let mut world = World::new(2, Params::static_society());
    world.params.pronoun_rate = 100.0;
    let a = world.found(&SoundProfile::by_id("germanic").unwrap(), 0.8, 1.0);
    let b = world.found(&SoundProfile::by_id("polynesian").unwrap(), 0.4, 1.0);
    world.communities[b].lands = world.communities[a].lands.clone();
    world
        .connect(a, b, 1.0, ContactKind::Intermarriage)
        .unwrap();
    for (_, _, cell) in CELLS {
        put(&mut world.varieties[0], cell, "takama");
        put(&mut world.varieties[1], cell, "nulu");
    }
    world.generation = 8;
    for (kind, intensity, duration, openness) in [
        (ContactKind::Trade, 1.0, 8, 1.0),
        (ContactKind::Neighbours, 1.0, 8, 1.0),
        (ContactKind::Intermarriage, 0.89, 8, 1.0),
        (ContactKind::Intermarriage, 1.0, 7, 1.0),
        (ContactKind::Intermarriage, 1.0, 8, 0.74),
    ] {
        let mut denied = world.clone();
        denied.contacts[0].kind = kind;
        denied.contacts[0].intensity = intensity;
        for v in 0..2 {
            denied.varieties[v]
                .grammar
                .contact_generations
                .insert(1 - v, duration);
            denied.communities[v].openness = openness;
        }
        denied.borrow_pronouns();
        assert!(
            denied
                .varieties
                .iter()
                .all(|v| v.pronouns.events.is_empty())
        );
    }
    for v in 0..2 {
        world.varieties[v]
            .grammar
            .contact_generations
            .insert(1 - v, 8);
    }
    world.borrow_pronouns();
    for v in &world.varieties {
        let notice = v.pronouns.events.last().expect("loan at forced rate");
        assert!(matches!(notice.event, NoticeKind::Borrowed { .. }));
        assert_eq!(notice.cause.unwrap().mechanism, Mechanism::Contact);
        assert!(matches!(
            word(v, notice.cell).origin,
            Origin::Borrowed { .. }
        ));
    }
}

#[test]
fn static_society_keeps_pronoun_replacement_off_and_daughters_inherit() {
    let mut world = World::solo(0, &SoundProfile::base(), Params::static_society());
    world.raise_state(0, None, Rise::Proclaimed);
    let original: Vec<_> = CELLS
        .iter()
        .map(|(_, _, c)| word(&world.varieties[0], c).id)
        .collect();
    world.run(16);
    assert!(world.varieties[0].pronouns.events.is_empty());
    assert!(!world.varieties[0].pronouns.polite);
    assert_eq!(
        original,
        CELLS
            .iter()
            .map(|(_, _, c)| word(&world.varieties[0], c).id)
            .collect::<Vec<_>>()
    );
    let daughter = world.varieties[0].fork(0, 16);
    for (_, _, c) in CELLS {
        assert_eq!(word(&daughter, c), word(&world.varieties[0], c));
    }
}

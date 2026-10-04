use super::*;
use crate::{Env, Matcher, Params, Rewrite, SoundChange, Variety, World};
use rand::RngCore;

fn form(ipa: &str) -> Form {
    Form::from_ipa(ipa).unwrap()
}
fn word(v: &Variety, concept: &str) -> LexemeId {
    v.lexicon.word_for(by_id(concept).unwrap()).unwrap().id
}
fn world(choice: ClassChoice) -> World {
    let mut profile = SoundProfile::base();
    profile.grammar.classes = Some(choice);
    World::solo(7, &profile, Params::static_society())
}
fn fixture() -> World {
    let mut world = world(ClassChoice::None);
    let v = &mut world.varieties[0];
    let target = word(v, "this");
    v.lexicon.get_mut(target).form = form("tak");
    v.minimal = MinimalWord::Syllable;
    v.profile.stress = Some(StressRule::Initial);
    v.gender.establish(
        ClassChoice::Sex,
        ["a", "i", "u"]
            .into_iter()
            .map(|s| (form(s), None))
            .collect(),
        Side::Suffix,
        &v.morphology,
        v.stress(),
        &v.lexicon,
        0,
    );
    world
}
fn vowel_merger() -> Law {
    Law {
        id: "test-class-vowel-merger",
        label: "Class vowels merge",
        rules: vec![SoundChange {
            id: "test-class-vowel-merger".into(),
            target: Matcher::Phone(crate::CATALOG.id_by_ipa("i").unwrap()),
            result: Rewrite::Phone(crate::CATALOG.id_by_ipa("a").unwrap()),
            left: Env::Any,
            right: Env::WordEdge,
        }],
        commonness: 1.0,
        stress: None,
    }
}

#[test]
fn founding_assigns_all_nouns_but_not_verbs_and_preserves_semantic_cores() {
    for choice in [
        ClassChoice::None,
        ClassChoice::Sex,
        ClassChoice::Animacy,
        ClassChoice::Many,
    ] {
        let world = world(choice);
        let v = &world.varieties[0];
        let founding_seed = stream(7, &[key("found"), 0]).next_u64();
        assert_eq!(v.gender.active_count(), choice.draw_count(founding_seed));
        for slot in &v.lexicon.slots {
            for variant in &slot.variants {
                assert_eq!(
                    v.gender.class_of(variant.lexeme).is_some(),
                    choice != ClassChoice::None && slot.concept.noun_semantics().is_some(),
                    "{} {choice:?}",
                    slot.concept.id
                );
            }
        }
        if choice == ClassChoice::Sex {
            assert_ne!(
                v.gender.class_of(word(v, "father")),
                v.gender.class_of(word(v, "mother"))
            );
        }
        if matches!(choice, ClassChoice::Animacy | ClassChoice::Many)
            && v.gender.active_count() >= 3
        {
            assert_eq!(
                v.gender.class_of(word(v, "child")),
                v.gender.class_of(word(v, "person"))
            );
            assert_eq!(
                v.gender.class_of(word(v, "dog")),
                v.gender.class_of(word(v, "fish"))
            );
            assert_ne!(
                v.gender.class_of(word(v, "dog")),
                v.gender.class_of(word(v, "child"))
            );
        }
        let assigned = v
            .lexicon
            .living()
            .filter(|w| v.gender.class_of(w.id).is_some())
            .count();
        assert_eq!(
            v.gender.classes.iter().map(|c| c.members).sum::<usize>(),
            assigned
        );
    }
}

#[test]
fn new_words_loans_and_new_noun_senses_receive_native_classes() {
    let mut world = fixture();
    let v = &mut world.varieties[0];
    let mother = by_id("mother").unwrap();
    let native = v.lexicon.coin(form("maka"), Origin::Expressive, mother, 1);
    let loan = v.lexicon.coin(
        form("naku"),
        Origin::Borrowed {
            from: 8,
            source: LexemeId(1),
            cause: crate::LoanCause::Unrecorded,
        },
        mother,
        1,
    );
    v.lexicon.slot_mut(mother).introduce(native, 0.25);
    v.lexicon.slot_mut(mother).introduce(loan, 0.25);
    let verb = word(v, "see");
    v.lexicon
        .slot_mut(by_id("person").unwrap())
        .introduce(verb, 0.25);
    v.gender.sync(&v.lexicon);
    assert_eq!(v.gender.class_of(native), Some(1), "semantic female core");
    assert_eq!(
        v.gender.class_of(loan),
        Some(2),
        "loan ending -u, not donor or semantic gender"
    );
    assert!(
        v.gender.class_of(verb).is_some(),
        "new noun sense gains a class"
    );
    let assigned = v.gender.class_of(native);
    v.lexicon.get_mut(native).form = form("maku");
    v.gender.sync(&v.lexicon);
    assert_eq!(
        v.gender.class_of(native),
        assigned,
        "established class is not reassigned every tick"
    );
}

#[test]
fn regular_erosion_merges_classes_records_trigger_and_then_loses_agreement() {
    let mut world = fixture();
    world.generation = 1;
    world.apply_law(0, &vowel_merger());
    let v = &world.varieties[0];
    assert_eq!(v.gender.active_count(), 2);
    assert_eq!(v.gender.classes[1].merged_into, Some(0));
    assert_eq!(v.gender.class_of(word(v, "mother")), Some(0));
    assert_eq!(v.gender.classes[1].agreement_at(0).ipa(), "taki");
    assert_eq!(
        v.gender.events[0].cause,
        Some(SoundCause {
            law: "test-class-vowel-merger",
            generation: 1
        })
    );
    let retired = v.gender.classes[1].clone();
    world.generation = 2;
    let law = crate::catalog()
        .into_iter()
        .find(|l| l.id == "apocope")
        .unwrap();
    world.apply_law(0, &law);
    let v = &world.varieties[0];
    assert_eq!(v.gender.active_count(), 0);
    assert_eq!(v.gender.class_of(word(v, "mother")), None);
    assert_eq!(v.gender.classes[1], retired, "merged forms freeze");
    assert_eq!(v.gender.events.last().unwrap().change, ClassChange::Lost);
    assert!(v.gender.classes.iter().all(|c| c.members == 0));
}

#[test]
fn umlaut_keeps_class_contrast_after_marker_loss_and_size_floor_still_holds() {
    let mut world = fixture();
    for (generation, id) in [(1, "umlaut"), (2, "apocope")] {
        world.generation = generation;
        let law = crate::catalog().into_iter().find(|l| l.id == id).unwrap();
        world.apply_law(0, &law);
    }
    let v = &world.varieties[0];
    assert_eq!(v.gender.active_count(), 2);
    assert!(
        v.gender
            .classes
            .iter()
            .filter(|c| c.retired.is_none())
            .all(|c| c.marker.segs.is_empty())
    );
    // The catalog's [a] is a front vowel, so it umlauts the stem of taka as
    // well as taki; the merged taka/taki stay apart from taku.
    assert_eq!(v.gender.classes[0].agreement.ipa(), "tek");
    assert_eq!(v.gender.classes[1].retired, Some(2));
    assert_eq!(v.gender.classes[2].agreement.ipa(), "tak");
    let mut protected = fixture();
    protected.varieties[0].minimal = MinimalWord::TwoSyllables;
    let before = protected.varieties[0].gender.clone();
    let law = crate::catalog()
        .into_iter()
        .find(|l| l.id == "apocope")
        .unwrap();
    protected.apply_law(0, &law);
    assert_eq!(protected.varieties[0].gender, before);
}

#[test]
fn emergence_is_seeded_static_rate_is_inert_and_daughters_inherit() {
    let mut world = world(ClassChoice::None);
    let v = &mut world.varieties[0];
    let before = v.gender.clone();
    v.gender.evolve(
        3,
        0,
        1,
        0.0,
        &v.profile,
        &v.morphology,
        v.stress(),
        &v.lexicon,
    );
    assert_eq!(v.gender, before);
    for generation in 1..100 {
        v.gender.evolve(
            3,
            0,
            generation,
            1.0,
            &v.profile,
            &v.morphology,
            v.stress(),
            &v.lexicon,
        );
        if v.gender.active_count() > 0 {
            break;
        }
    }
    assert!(v.gender.active_count() >= 2);
    assert!(v.gender.classes.iter().all(|c| c.source.is_some()));
    let mut replay = before;
    for generation in 1..=v.gender.classes[0].born {
        replay.evolve(
            3,
            0,
            generation,
            1.0,
            &v.profile,
            &v.morphology,
            v.stress(),
            &v.lexicon,
        );
    }
    assert_eq!(v.gender, replay);
    assert_eq!(v.fork(0, 50).gender, v.gender);
}

#[test]
fn renewed_system_does_not_assign_new_nouns_to_dead_classes() {
    let mut world = fixture();
    world.generation = 1;
    let law = crate::catalog()
        .into_iter()
        .find(|l| l.id == "apocope")
        .unwrap();
    world.apply_law(0, &law);
    let v = &mut world.varieties[0];
    v.gender.establish(
        ClassChoice::Sex,
        ["pa", "ti", "ku"]
            .into_iter()
            .map(|s| (form(s), None))
            .collect(),
        Side::Suffix,
        &v.morphology,
        v.stress(),
        &v.lexicon,
        40,
    );
    assert_eq!(v.gender.class_of(word(v, "mother")), Some(4));
    assert_eq!(v.gender.class_of(word(v, "father")), Some(3));
}

#[test]
fn each_basis_draws_binary_three_four_and_larger_systems() {
    for basis in [ClassChoice::Sex, ClassChoice::Animacy, ClassChoice::Many] {
        let mut sizes = [false; 9];
        for seed in 0..1000 {
            sizes[basis.draw_count(seed)] = true;
        }
        assert!(sizes[2..].iter().all(|present| *present), "{basis:?}");
    }
    assert_eq!(ClassChoice::None.draw_count(0), 0);
}

#[test]
fn binary_animacy_includes_people_and_sex_keeps_parent_cores_apart() {
    for basis in [ClassChoice::Sex, ClassChoice::Animacy] {
        let seed = (0..1000).find(|&seed| basis.draw_count(seed) == 2).unwrap();
        let mut profile = SoundProfile::base();
        profile.grammar.classes = Some(basis);
        let v = &Variety::found(
            seed,
            &profile,
            crate::Livelihood::Farming,
            crate::Ethos::default(),
        );
        assert_eq!(v.gender.active_count(), 2);
        if basis == ClassChoice::Sex {
            assert_ne!(
                v.gender.class_of(word(v, "father")),
                v.gender.class_of(word(v, "mother"))
            );
        } else {
            let animate = v.gender.class_of(word(v, "person"));
            assert_eq!(animate, v.gender.class_of(word(v, "dog")));
            assert_eq!(animate, v.gender.class_of(word(v, "mother")));
            // Native non-animate meanings use the inanimate semantic remainder.
            assert_eq!(v.gender.classes[1].core, Core::Remainder);
            assert_ne!(animate, v.gender.class_of(word(v, "stone")));
            assert_ne!(animate, v.gender.class_of(word(v, "water")));
        }
    }
}

use super::*;
use crate::{Params, SoundProfile, World};

fn world(order: PossessorOrder, genitive: GrammarChoice) -> World {
    let mut profile = SoundProfile::base();
    profile.grammar = GrammarPrior::fixed(GrammarDesign {
        possessor: Some(order),
        genitive: Some(genitive),
        ..GrammarDesign::default()
    });
    profile.stress = Some(StressRule::Initial);
    World::solo(9, &profile, Params::static_society())
}
fn forced() -> Params {
    Params {
        possessive_case_rate: 1.0,
        possessive_fusion_rate: 1.0,
        possessive_drop_rate: 1.0,
        ..Params::static_society()
    }
}
fn evolve(world: &mut World, generation: u32) {
    let v = &mut world.varieties[0];
    let stress = v.stress();
    v.grammar.evolve(
        9,
        0,
        generation,
        &mut v.lexicon,
        &v.morphology,
        stress,
        &forced(),
    );
    world.generation = generation;
}

#[test]
fn genitive_eligibility_split_preserves_every_nonpronoun() {
    for concept in crate::concepts::CONCEPTS {
        if crate::pronouns::is_pronoun(concept) {
            assert!(concept.categories.genitive);
            assert!(!concept.categories.plural);
        } else {
            assert_eq!(
                concept.categories.genitive, concept.categories.plural,
                "{}",
                concept.id
            );
        }
    }
}

#[test]
fn founding_and_static_replay_have_no_possessive_cycle() {
    let mut a = world(PossessorOrder::Before, GrammarChoice::Suffix);
    let mut b = a.clone();
    for _ in 0..40 {
        a.run(1);
        b.run(1);
        let av = &a.varieties[0];
        assert_eq!(av.grammar, b.varieties[0].grammar);
        assert_eq!(av.lexicon, b.varieties[0].lexicon);
        assert_eq!(av.grammar.summary.possessive, PronounCase::None);
        assert_eq!(av.grammar.possessor_agreement().stage, AgreementStage::None);
        assert!(av.grammar.possessor_agreement().events.is_empty());
        for (_, _, cell) in av.pronouns.cells() {
            let w = av.lexicon.word_for(by_id(cell).unwrap()).unwrap();
            assert!(!w.paradigms.iter().any(|p| p.category == Category::Genitive));
        }
    }
    let mut json = serde_json::to_value(GrammarDesign::default()).unwrap();
    json.as_object_mut().unwrap().remove("genitive");
    let design: GrammarDesign = serde_json::from_value(json).unwrap();
    assert_eq!(design.genitive, None);
}

#[test]
fn possessor_cycle_attaches_on_both_sides_and_drops_after_fusion() {
    for (order, side) in [
        (PossessorOrder::Before, Side::Prefix),
        (PossessorOrder::After, Side::Suffix),
    ] {
        let mut w = world(order, GrammarChoice::None);
        assert_eq!(w.varieties[0].grammar.summary.possessive, PronounCase::None);
        evolve(&mut w, 1);
        let v = &w.varieties[0];
        let state = v.grammar.possessor_agreement();
        assert_eq!((state.stage, state.side), (AgreementStage::Clitic, side));
        let noun = &v.lexicon.word_for(by_id("house").unwrap()).unwrap().form;
        let clitic = &state.cell("1sg").unwrap().form;
        let expected = match side {
            Side::Prefix => v.morphology.join_with_edge(clitic, noun),
            Side::Suffix => v.morphology.join_with_edge(noun, clitic),
        };
        assert_eq!(
            v.grammar
                .attach_possessor(noun, clitic, &v.morphology, v.stress()),
            expected
        );
        evolve(&mut w, 8);
        assert_eq!(
            w.varieties[0].grammar.possessor_agreement().stage,
            AgreementStage::Clitic
        );
        evolve(&mut w, 9);
        assert_eq!(
            w.varieties[0].grammar.possessor_agreement().stage,
            AgreementStage::Affix
        );
        assert!(!w.varieties[0].grammar.possessor_agreement().pro_drop);
        evolve(&mut w, 10);
        let state = w.varieties[0].grammar.possessor_agreement();
        assert!(state.pro_drop);
        assert_eq!(
            state.events.iter().map(|e| e.kind).collect::<Vec<_>>(),
            [
                AgreementKind::Clitics,
                AgreementKind::Affixes,
                AgreementKind::ProDrop
            ]
        );
    }
}

#[test]
fn worn_possessive_affixes_return_the_pronoun_and_wait_to_restart() {
    let mut w = world(PossessorOrder::Before, GrammarChoice::None);
    evolve(&mut w, 1);
    evolve(&mut w, 9);
    evolve(&mut w, 10);
    let v = &mut w.varieties[0];
    for (cell, ipa) in v
        .grammar
        .possessor_agreement
        .cells
        .iter_mut()
        .take(4)
        .zip(["ma", "mi", "pa", "pi"])
    {
        cell.form = Form::from_ipa(ipa).unwrap();
    }
    let law = Law {
        id: "possessive-test-raising".into(),
        label: "Test raising".into(),
        source: None,
        commonness: 1.0,
        stress: None,
        rules: vec![crate::SoundChange {
            id: "possessive-test-raising".into(),
            target: crate::Matcher::Phone(crate::CATALOG.id_by_ipa("a").unwrap()),
            result: crate::Rewrite::Phone(crate::CATALOG.id_by_ipa("i").unwrap()),
            left: crate::Env::Any,
            right: crate::Env::Any,
        }],
    };
    let stress = v.stress();
    v.grammar.wear_agreement(&law, stress, 20);
    v.grammar.refresh(&v.lexicon, stress, 20);
    let state = v.grammar.possessor_agreement();
    assert_eq!(state.stage, AgreementStage::None);
    assert!(!state.pro_drop);
    assert_eq!(state.events.last().unwrap().kind, AgreementKind::Loss);
    assert!(state.cells.iter().all(|c| c.retired == Some(20)));
    assert!(state.cells.iter().any(|c| {
        c.history
            .iter()
            .any(|h| matches!(h.event, GrammarEvent::SoundLaw { .. }))
    }));
    evolve(&mut w, 31);
    assert_eq!(
        w.varieties[0].grammar.possessor_agreement().stage,
        AgreementStage::None
    );
    evolve(&mut w, 32);
    assert_eq!(
        w.varieties[0].grammar.possessor_agreement().stage,
        AgreementStage::Clitic
    );
}

#[test]
fn possessive_paradigm_recruits_fuses_ahead_and_retains_relics_without_touching_objects() {
    let mut w = world(PossessorOrder::Before, GrammarChoice::Particle);
    let v = &mut w.varieties[0];
    let stress = v.stress();
    let objects = |v: &crate::Variety| {
        v.lexicon
            .living()
            .map(|w| {
                w.paradigms
                    .iter()
                    .find(|p| p.category == Category::Object)
                    .cloned()
            })
            .collect::<Vec<_>>()
    };
    let before = objects(v);
    v.grammar
        .evolve_possessive_case(9, 0, 1, &mut v.lexicon, &v.morphology, stress, 1.0);
    assert_eq!(v.grammar.summary.possessive, PronounCase::LikeNouns);
    v.grammar
        .evolve_possessive_case(9, 0, 2, &mut v.lexicon, &v.morphology, stress, 1.0);
    v.grammar.refresh(&v.lexicon, stress, 2);
    assert_eq!(v.grammar.summary.possessive, PronounCase::Ahead);
    assert_eq!(objects(v), before);
    for word in &mut v.lexicon.lexemes {
        if crate::pronouns::is_pronoun(word.first_sense) {
            continue;
        }
        for r in word
            .paradigms
            .iter_mut()
            .filter(|p| p.category == Category::Genitive)
            .flat_map(|p| &mut p.realizations)
        {
            r.form = Some(word.form.clone());
        }
    }
    v.grammar.refresh(&v.lexicon, stress, 3);
    v.grammar.sync(&mut v.lexicon, &v.morphology, stress, 3);
    assert_eq!(v.grammar.summary.possessive, PronounCase::Relic);
    assert_eq!(
        v.grammar
            .events
            .iter()
            .filter(|e| e.category == Category::Genitive
                && matches!(e.event, NoticeKind::PronounRelic { .. }))
            .count(),
        1
    );
}

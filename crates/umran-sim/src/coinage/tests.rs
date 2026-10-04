use super::*;
use crate::{Craft, Params, Relation, SoundProfile};

fn world(seed: u64) -> World {
    World::solo(
        seed,
        &SoundProfile::base(),
        Params {
            coinage_rate: Params::default().coinage_rate,
            sound_change_rate: 0.0,
            innovation_rate: 0.0,
            loan_rate: 0.0,
            ..Params::static_society()
        },
    )
}

fn set_form(world: &mut World, v: usize, concept: &str, ipa: &str) -> LexemeId {
    let lexicon = &mut world.varieties[v].lexicon;
    let id = lexicon.slot(by_id(concept).unwrap()).dominant().unwrap();
    lexicon.get_mut(id).form = Form::from_ipa(ipa).unwrap();
    id
}

#[test]
fn compound_order_and_local_parts_are_preserved_without_rebuilding() {
    for head_first in [false, true] {
        let mut world = world(4);
        world.generation = 3;
        world.varieties[0].morphology.names.head_first = head_first;
        set_form(&mut world, 0, "fish", "pata");
        set_form(&mut world, 0, "cloth", "kuma");
        let target = by_id("net").unwrap();
        let build = compound(
            &world,
            0,
            by_id("fish").unwrap(),
            by_id("cloth").unwrap(),
            None,
        )
        .unwrap();
        let id = world.install_coinage(0, target, build);
        world.varieties[0]
            .lexicon
            .slot_mut(target)
            .introduce(id, 1.0);
        observe_opacity(&mut world.varieties[0].lexicon, 3);
        let word = world.varieties[0].lexicon.get(id);
        assert_eq!(
            word.form.ipa(),
            if head_first { "kumapata" } else { "patakuma" }
        );
        assert_eq!(word.coined.as_ref().unwrap().opaque_since, None);
        assert_eq!(word.coined.as_ref().unwrap().parts[0].form.ipa(), "pata");
        set_form(&mut world, 0, "fish", "sina");
        observe_opacity(&mut world.varieties[0].lexicon, 7);
        let word = world.varieties[0].lexicon.get(id);
        assert_eq!(
            word.form.ipa(),
            if head_first { "kumapata" } else { "patakuma" }
        );
        assert_eq!(word.coined.as_ref().unwrap().opaque_since, Some(7));
        set_form(&mut world, 0, "fish", "pata");
        observe_opacity(&mut world.varieties[0].lexicon, 8);
        assert_eq!(
            world.varieties[0]
                .lexicon
                .get(id)
                .coined
                .as_ref()
                .unwrap()
                .opaque_since,
            Some(7)
        );
    }
}

#[test]
fn sound_laws_change_the_lexicalized_whole_and_retired_words_stop() {
    let mut world = world(9);
    world.generation = 1;
    world.varieties[0].morphology.names.head_first = false;
    set_form(&mut world, 0, "fish", "tap");
    set_form(&mut world, 0, "cloth", "ata");
    let target = by_id("net").unwrap();
    let build = compound(
        &world,
        0,
        by_id("fish").unwrap(),
        by_id("cloth").unwrap(),
        None,
    )
    .unwrap();
    let id = world.install_coinage(0, target, build);
    world.varieties[0]
        .lexicon
        .slot_mut(target)
        .introduce(id, 1.0);
    let law = crate::catalog()
        .into_iter()
        .find(|l| l.id == "intervocalic-voicing")
        .unwrap();
    let before = world.varieties[0].lexicon.get(id).form.clone();
    let expected = law.apply(
        &before,
        world.varieties[0].minimal,
        world.varieties[0].stress(),
    );
    assert_ne!(before, expected);
    world.generation = 4;
    world.apply_law(0, &law);
    assert_eq!(world.varieties[0].lexicon.get(id).form, expected);
    assert_eq!(world.varieties[0].lexicon.get(id).form_at(1), &before);
    assert_eq!(
        world.varieties[0]
            .lexicon
            .get(id)
            .coined
            .as_ref()
            .unwrap()
            .opaque_since,
        Some(4)
    );
    let word = world.varieties[0].lexicon.get_mut(id);
    word.obsolete = Some(5);
    let retired = word.clone();
    world.generation = 6;
    set_form(&mut world, 0, "fish", "sina");
    world.apply_law(0, &law);
    observe_opacity(&mut world.varieties[0].lexicon, 6);
    assert_eq!(world.varieties[0].lexicon.get(id), &retired);
}

#[test]
fn calques_translate_donor_parts_in_recipient_order_and_require_each_part() {
    let mut world = world(14);
    let recipient = world.found(&SoundProfile::base(), 0.5, 0.5);
    let v = world.communities[recipient].variety;
    world.generation = 2;
    set_form(&mut world, 0, "word", "pata");
    set_form(&mut world, 0, "god", "kuma");
    let target = by_id("oath").unwrap(); // No native recipe: only a donor can supply this analysis.
    let source = compound(
        &world,
        0,
        by_id("word").unwrap(),
        by_id("god").unwrap(),
        None,
    )
    .unwrap();
    let source = world.install_coinage(0, target, source);
    world.varieties[0].lexicon.slot_mut(target).variants.clear();
    world.varieties[0]
        .lexicon
        .slot_mut(target)
        .introduce(source, 1.0);
    set_form(&mut world, v, "word", "sina");
    let god = set_form(&mut world, v, "god", "telu");
    world.varieties[v].morphology.names.head_first = true;
    let build = world.plan_coinage(v, target, Some(0)).unwrap();
    assert_eq!(build.kind, CoinageKind::Calque);
    assert_eq!(build.form.ipa(), "telusina");
    let id = world.install_coinage(recipient, target, build);
    let record = world.varieties[v].lexicon.get(id).coined.as_ref().unwrap();
    assert_eq!(record.from, Some(0));
    assert_eq!(record.parts[0].form.ipa(), "sina");
    assert_ne!(world.root_of(0, source), world.root_of(v, id));
    world.varieties[v].lexicon.get_mut(id).obsolete = Some(3);
    world.varieties[v]
        .lexicon
        .slot_mut(by_id("god").unwrap())
        .variants
        .clear();
    assert!(world.plan_coinage(v, target, Some(0)).is_none());
    world.varieties[v]
        .lexicon
        .slot_mut(by_id("god").unwrap())
        .introduce(god, 1.0);
    assert!(world.plan_coinage(v, target, Some(0)).is_some());
    world.varieties[0]
        .lexicon
        .get_mut(source)
        .coined
        .as_mut()
        .unwrap()
        .opaque_since = Some(3);
    assert!(world.plan_coinage(v, target, Some(0)).is_none());
}

#[test]
fn derivation_follows_true_root_through_a_prefixed_base() {
    let mut world = World::solo(
        2,
        &SoundProfile::by_id("semitic").unwrap(),
        Params {
            coinage_rate: 1.0,
            ..Params::static_society()
        },
    );
    let fish = set_form(&mut world, 0, "fish", "katab");
    let fishing = set_form(&mut world, 0, "fishing", "maktab");
    world.varieties[0].lexicon.get_mut(fishing).origin = Origin::Derived {
        base: fish,
        relation: Relation::Action,
    };
    let build = world
        .derived_coinage(0, by_id("net").unwrap(), true)
        .unwrap();
    let morphology = &world.varieties[0].morphology;
    let expected = morphology
        .derive(
            &Form::from_ipa("maktab").unwrap(),
            Morphology::skeleton(&Form::from_ipa("katab").unwrap()),
            Relation::Instrument,
        )
        .unwrap();
    assert_eq!(build.form, expected);
    assert_ne!(
        build.form,
        morphology
            .derive(
                &Form::from_ipa("maktab").unwrap(),
                None,
                Relation::Instrument
            )
            .unwrap()
    );
}

#[test]
fn need_causes_precede_coinages_and_unknown_ideas_stay_unknown() {
    let mut world = world(12);
    world.generation = 8;
    assert!(
        world.varieties[0]
            .lexicon
            .word_for(by_id("iron").unwrap())
            .is_none()
    );
    world.params.coinage_rate = 1.0;
    world.coin_words(0);
    assert!(
        world.varieties[0]
            .lexicon
            .word_for(by_id("iron").unwrap())
            .is_none()
    );
    world.learn(0, Craft::Metalworking, None);
    let cause = world.events.len() - 1;
    let target = by_id("iron").unwrap();
    let build = world.plan_coinage(0, target, None).unwrap();
    let id = world.install_coinage(0, target, build);
    let event = world.events.len() - 1;
    assert_eq!(
        world.causes[&event],
        crate::Cause {
            event: cause,
            mechanism: crate::Mechanism::WordNeed
        }
    );
    assert!(matches!(
        world.events[cause].1,
        WorldEvent::Learnt {
            craft: Craft::Metalworking,
            ..
        }
    ));
    assert_eq!(world.varieties[0].lexicon.get(id).born, 8);
    world.learn_words();
    assert!(
        world.varieties[0]
            .lexicon
            .word_for(by_id("iron").unwrap())
            .is_some()
    );
    assert!(
        world.varieties[0]
            .lexicon
            .word_for(by_id("book").unwrap())
            .is_none()
    );
}

#[test]
fn static_society_disables_productive_coinage_and_competitors_are_bounded() {
    let mut world = world(13);
    world.params.coinage_rate = 0.0;
    let before = world.varieties[0].lexicon.clone();
    world.coin_words(0);
    assert_eq!(world.varieties[0].lexicon, before);
    world.params.coinage_rate = 1.0;
    for generation in 1..5 {
        world.generation = generation;
        world.coin_words(0);
    }
    assert!(
        world.varieties[0]
            .lexicon
            .slots
            .iter()
            .all(|s| s.variants.len() <= crate::world::MAX_VARIANTS)
    );
    for word in world.varieties[0]
        .lexicon
        .living()
        .filter(|l| l.coined.is_some())
    {
        assert!(!word.form.segs.is_empty());
        assert!(word.form.vowel_count() > 0);
        assert!(world.varieties[0].lexicon.senses(word.id).next().is_some());
    }
    assert!(world.varieties[0].lexicon.living().any(|l| {
        l.coined
            .as_ref()
            .is_some_and(|c| c.kind == CoinageKind::Compound)
    }));
    assert!(world.varieties[0].lexicon.living().any(|l| {
        l.coined
            .as_ref()
            .is_some_and(|c| c.kind == CoinageKind::Derived)
    }));
}

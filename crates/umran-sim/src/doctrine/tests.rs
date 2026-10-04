use super::*;
use crate::{Params, Revelation, SoundProfile};

fn faithful(seed: u64) -> (World, usize) {
    let mut world = World::solo(seed, &SoundProfile::base(), Params::static_society());
    let faith = world.found_religion(0, Revelation::Proclaimed);
    (world, faith)
}

#[test]
fn founders_bias_doctrine_without_fixing_every_faith() {
    let high = Ethos {
        pious: 1.0,
        open: -1.0,
        hierarchical: 1.0,
        ..Ethos::default()
    };
    let low = Ethos {
        pious: -1.0,
        open: 1.0,
        hierarchical: -1.0,
        ..Ethos::default()
    };
    for seed in 0..80 {
        let a = Doctrine::found(seed, 0, high, true, false);
        let b = Doctrine::found(seed, 0, low, true, false);
        assert!(a.stance(Tenet::SacredLanguage) > b.stance(Tenet::SacredLanguage));
        assert!(a.stance(Tenet::Hierarchy) > b.stance(Tenet::Hierarchy));
        assert!(
            a.positions()
                .chain(b.positions())
                .all(|p| (-1.0..=1.0).contains(&p.stance))
        );
    }
}

#[test]
fn static_society_keeps_doctrine_and_language_inert() {
    let (mut world, _) = faithful(3);
    let before = world.clone();
    for _ in 0..20 {
        world.change_doctrine();
    }
    // World has no PartialEq; both sides are clones, so even hash maps
    // print in the same order.
    assert_eq!(format!("{world:?}"), format!("{before:?}"));
}

#[test]
fn drift_is_slow_bounded_and_replayable() {
    let (mut world, faith) = faithful(4);
    world.params.doctrine_enabled = true;
    let mut replay = world.clone();
    for generation in 1..=160 {
        let old = world.religions[faith].doctrine.stances;
        world.generation = generation;
        replay.generation = generation;
        world.change_doctrine();
        replay.change_doctrine();
        for (a, b) in old.into_iter().zip(world.religions[faith].doctrine.stances) {
            assert!((a - b).abs() <= 0.0141);
            assert!((-1.0..=1.0).contains(&b));
        }
    }
    assert_eq!(format!("{world:?}"), format!("{replay:?}"));
    world.communities[0].ended = Some(world.generation);
    let before = world.religions[faith].doctrine.clone();
    world.change_doctrine();
    assert_eq!(world.religions[faith].doctrine, before);
}

#[test]
fn schism_changes_only_the_most_contested_tenet_and_records_it() {
    let (mut world, parent) = faithful(8);
    world.params.doctrine_enabled = true;
    world.religions[parent].doctrine.stances = [0.0; 6];
    world.communities[0].ethos = Ethos {
        hierarchical: 1.0,
        ..Ethos::default()
    };
    let branch = world.schism(0, SchismCause::Distance).unwrap();
    assert_eq!(world.religions[branch].disputed, Some(Tenet::Hierarchy));
    for tenet in Tenet::ALL {
        assert_eq!(world.religions[parent].doctrine.stance(tenet), 0.0);
        assert_eq!(
            world.religions[branch].doctrine.stance(tenet),
            if tenet == Tenet::Hierarchy { 0.8 } else { 0.0 }
        );
    }
    let (event, dispute) = world
        .events
        .iter()
        .enumerate()
        .find_map(|(i, (_, e))| match e {
            WorldEvent::TenetDisputed {
                tenet,
                previous,
                stance,
                ..
            } => Some((i, (*tenet, *previous, *stance))),
            _ => None,
        })
        .unwrap();
    assert_eq!(dispute, (Tenet::Hierarchy, 0.0, 0.8));
    let cause = world.causes[&event];
    assert!(
        matches!(world.events[cause.event].1, WorldEvent::Schism { religion, .. } if religion == branch)
    );
}

#[test]
fn written_sacred_requirement_and_reform_change_register_not_sacred_words() {
    let (mut world, parent) = faithful(13);
    world.params.doctrine_enabled = true;
    world.religions[parent].scripture = true;
    world.religions[parent].translates = false;
    world.religions[parent].doctrine.stances[Tenet::SacredLanguage as usize] = 0.9;
    world.learn(0, Craft::Writing, None);
    world.doctrine_register(0, parent);
    let v = world.communities[0].variety;
    let sacred = world.religions[parent].sacred;
    let frozen = world.varieties[sacred].clone();
    assert!(world.diglossic(v));
    assert_eq!(world.varieties[v].high, Some(sacred));
    let branch = world.schism(0, SchismCause::Reform).unwrap();
    assert_eq!(
        world.religions[branch].disputed,
        Some(Tenet::SacredLanguage)
    );
    assert!(world.religions[branch].translates);
    assert!(!world.diglossic(v));
    assert_eq!(
        format!("{:?}", world.varieties[sacred]),
        format!("{frozen:?}")
    );
}

#[test]
fn taboo_replacement_preserves_other_senses_and_retires_only_unused_words() {
    let (mut world, faith) = faithful(5);
    world.params.doctrine_enabled = true;
    world.adopt_doctrine(faith, None);
    world.religions[faith].doctrine.taboo = "blood";
    let v = world.communities[0].variety;
    let blood = by_id("blood").unwrap();
    let red = by_id("red").unwrap();
    let old = world.varieties[v].lexicon.slot(blood).dominant().unwrap();
    let old_form = world.varieties[v].lexicon.get(old).form.clone();
    world.varieties[v].lexicon.slot_mut(red).introduce(old, 0.8);
    let replacement = world.replace_taboo(0, faith).unwrap();
    assert_eq!(
        world.varieties[v].lexicon.slot(blood).dominant(),
        Some(replacement)
    );
    assert!(!world.varieties[v].lexicon.slot(blood).has(old));
    assert!(world.varieties[v].lexicon.slot(red).has(old));
    assert_eq!(world.varieties[v].lexicon.get(old).obsolete, None);
    assert_eq!(world.varieties[v].lexicon.get(old).form, old_form);
    let next = world.replace_taboo(0, faith).unwrap();
    assert_ne!(world.varieties[v].lexicon.get(next).form, old_form);
    assert_eq!(
        world.varieties[v].lexicon.get(replacement).obsolete,
        Some(world.generation)
    );
    let cause = world.causes[&(world.events.len() - 1)];
    assert!(matches!(
        world.events[cause.event].1,
        WorldEvent::TenetAdopted {
            tenet: Tenet::Purity,
            ..
        }
    ));
}

#[test]
fn sacred_vocabulary_records_faith_and_never_duplicates_a_source() {
    let (mut world, faith) = faithful(19);
    let other = world.found(&SoundProfile::by_id("polynesian").unwrap(), 0.5, 0.5);
    world.convert(other, faith, None);
    world.religions[faith].translates = false;
    world.religions[faith].doctrine.stances[Tenet::SacredLanguage as usize] = 1.0;
    let sacred = world.religions[faith].sacred;
    let frozen = world.varieties[sacred].clone();
    for generation in 1..=160 {
        world.generation = generation;
        world.doctrine_words(other, faith);
    }
    let lexicon = &world.variety_of(other).lexicon;
    let mut borrowed = 0;
    for concept in crate::CONCEPTS
        .iter()
        .filter(|c| need(c) == Some(Need::Faith))
    {
        let words: Vec<_> = lexicon.slot(concept).variants.iter().filter_map(|variant| {
            let word = lexicon.get(variant.lexeme);
            matches!(word.origin, Origin::Borrowed { from, cause: LoanCause::Faith { religion, recipient, .. }, .. }
                if from == sacred && religion == faith && recipient == other).then_some(word)
        }).collect();
        assert!(words.len() <= 1);
        borrowed += words.len();
    }
    assert!(borrowed >= 3, "religious loans: {borrowed}");
    assert_eq!(
        format!("{:?}", world.varieties[sacred]),
        format!("{frozen:?}")
    );
}

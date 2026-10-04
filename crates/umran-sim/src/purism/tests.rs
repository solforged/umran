use super::*;
use crate::schisms::SchismCause;
use crate::{ContactKind, Craft, LoanCause, Params, Revelation, Rise, SoundProfile};

fn realm(fixed: bool, rate: f32) -> (World, usize, usize) {
    let mut world = World::solo(
        31,
        &SoundProfile::base(),
        Params {
            purism_rate: rate,
            sound_change_rate: 0.0,
            innovation_rate: 0.0,
            loan_rate: 0.0,
            ..Params::static_society()
        },
    );
    let donor = world.found(&SoundProfile::by_id("semitic").unwrap(), 0.9, 0.5);
    let state = world.raise_state(0, None, Rise::Proclaimed);
    world.adopt_standard(state);
    world.learn(0, Craft::Writing, None);
    let religion = world.found_religion(0, Revelation::Proclaimed);
    world.religions[religion].scripture = true;
    world.religions[religion].translates = false;
    world.generation = 4;
    let from = world.communities[donor].variety;
    for concept in CONCEPTS.iter().filter(|c| c.stability.is_some()).take(24) {
        let source = world.varieties[from].lexicon.word_for(concept).unwrap();
        let origin = Origin::Borrowed {
            from,
            source: source.id,
            cause: LoanCause::Coinage {
                donor,
                recipient: 0,
            },
        };
        let form = source.form.clone();
        let lexicon = &mut world.varieties[0].lexicon;
        let loan = lexicon.coin(form, origin, concept, world.generation);
        lexicon.slot_mut(concept).variants.clear();
        lexicon.slot_mut(concept).introduce(loan, 1.0);
    }
    world.retire(0);
    if fixed {
        world.fix(state, Fixing::Age);
    }
    (world, state, religion)
}

fn revival(world: &mut World) -> usize {
    world.generation += 1;
    let event = world.events.len();
    world.schism(0, SchismCause::Reform).unwrap();
    assert!(matches!(
        world.events[event].1,
        WorldEvent::Schism {
            cause: SchismCause::Reform,
            ..
        }
    ));
    event
}

#[test]
fn revival_does_not_reform_a_sacred_high_form() {
    let (mut world, _, _) = realm(false, 1.0);
    // Freeze the speech after its loans arrived, so a missing guard would
    // have borrowed words to replace rather than passing for lack of targets.
    let religion = world.found_religion(0, Revelation::Proclaimed);
    world.religions[religion].scripture = true;
    world.religions[religion].translates = false;
    let sacred = world.religions[religion].sacred;
    world.write_sacred(0, sacred);
    assert!(!world.spoken()[sacred]);
    assert!(world.varieties[sacred].lexicon.slots.iter().any(|slot| {
        slot.dominant().is_some_and(|id| {
            matches!(
                world.varieties[sacred].lexicon.get(id).origin,
                Origin::Borrowed { .. }
            )
        })
    }));
    let original = world.varieties[sacred].lexicon.clone();
    revival(&mut world);
    assert!(!world.purist_pressures.is_empty());
    world.purify_high_forms();
    assert_eq!(world.varieties[sacred].lexicon, original);
    assert!(world.varieties[sacred].purism.is_empty());
}

#[test]
fn written_revival_revives_archaisms_only_in_high_form_and_retains_causes() {
    let (mut world, state, _) = realm(true, 1.0);
    let high = world.states[state].classical.unwrap().variety;
    let trigger = revival(&mut world);
    let vernacular = world.varieties[0].lexicon.clone();
    let old_high = world.varieties[high].lexicon.clone();
    world.purify_high_forms();
    let episode = &world.varieties[high].purism[0];
    assert_eq!(
        episode.cause,
        Cause {
            event: trigger,
            mechanism: Mechanism::ReligiousRevival
        }
    );
    assert_eq!(
        world.varieties[0].lexicon, vernacular,
        "reform never edits everyday words"
    );
    for word in &episode.replaced {
        let lexicon = &world.varieties[high].lexicon;
        let concept = by_id(word.concept).unwrap();
        assert_eq!(
            lexicon.slot(concept).variants,
            vec![Variant {
                lexeme: word.native,
                weight: 1.0
            }]
        );
        assert!(native(lexicon, word.native));
        assert!(
            old_high.get(word.native).obsolete.is_some(),
            "old native word was revived"
        );
        assert_eq!(
            lexicon.get(word.native).form,
            old_high.get(word.native).form
        );
        assert!(
            lexicon
                .get(word.native)
                .log
                .iter()
                .any(|e| matches!(e.event, Event::Obsolete))
        );
        assert!(lexicon.get(word.native).obsolete.is_none());
        assert_eq!(lexicon.get(word.loan).obsolete, Some(world.generation));
        assert_eq!(vernacular.slot(concept).dominant(), Some(word.loan));
    }
    let reform = world
        .events
        .iter()
        .position(|(_, e)| matches!(e, WorldEvent::PuristReform { .. }))
        .unwrap();
    assert_eq!(world.causes[&reform].event, trigger);
    for (event, (_, e)) in world.events.iter().enumerate() {
        if matches!(e, WorldEvent::PuristReplacement { .. }) {
            assert_eq!(
                world.causes[&event],
                Cause {
                    event: reform,
                    mechanism: Mechanism::PuristNorm
                }
            );
        }
    }
    let frozen = world.varieties[high].lexicon.clone();
    let law = world
        .law_catalog()
        .iter()
        .find(|law| {
            let v = &world.varieties[0];
            v.lexicon.living().any(|w| {
                law.changes(
                    &w.form,
                    &law.apply(&w.form, v.minimal, v.stress()),
                    v.stress(),
                )
            })
        })
        .unwrap()
        .clone();
    world.generation += 1;
    world.apply_law(0, &law);
    assert_ne!(world.varieties[0].lexicon, vernacular);
    assert_eq!(
        world.varieties[high].lexicon, frozen,
        "prescribed written form stays fixed"
    );
    world.write_vernacular(0, crate::Vernacular::Standard { state });
    assert!(
        !world.diglossic(0),
        "existing vernacularization still works"
    );
}

#[test]
fn reform_of_written_standard_separates_speech_before_replacing_words() {
    let (mut world, state, _) = realm(false, 1.0);
    revival(&mut world);
    let speech = world.varieties[0].lexicon.clone();
    world.purify_high_forms();
    let high = world.states[state].classical.unwrap();
    assert_eq!(high.how, Fixing::Purism);
    assert_eq!(world.varieties[0].high, Some(high.variety));
    assert_eq!(world.varieties[0].written, None);
    assert_eq!(world.varieties[0].lexicon, speech);
    assert_eq!(world.pace(0), 1.0);
    assert!(world.varieties[high.variety].lexicon != speech);
}

#[test]
fn static_society_and_unwritten_revival_do_not_reform() {
    let (mut static_world, state, _) = realm(true, 0.0);
    let high = static_world.states[state].classical.unwrap().variety;
    revival(&mut static_world);
    let original = static_world.varieties[high].lexicon.clone();
    static_world.purify_high_forms();
    assert_eq!(static_world.varieties[high].lexicon, original);
    assert!(static_world.purist_pressures.is_empty());

    let (mut oral, state, religion) = realm(true, 1.0);
    oral.religions[religion].scripture = false;
    revival(&mut oral);
    oral.purify_high_forms();
    assert!(
        oral.varieties[oral.states[state].classical.unwrap().variety]
            .purism
            .is_empty()
    );
    assert!(oral.purist_pressures.is_empty());
}

#[test]
fn missing_standard_waits_for_writing_but_stale_movements_expire() {
    let (mut world, state, _) = realm(false, 1.0);
    world.varieties[0].written = None;
    revival(&mut world);
    // Reform converts may bring writing: explicitly keep this speech unwritten.
    world.varieties[0].written = None;
    world.purify_high_forms();
    assert!(world.states[state].classical.is_none());
    assert_eq!(world.purist_pressures.len(), 1);
    world.generation += PRESSURE_SPAN + 1;
    world.begin_writing(0);
    world.purify_high_forms();
    assert!(world.states[state].classical.is_none());
    assert!(world.purist_pressures.is_empty());
}

#[test]
fn no_native_attestation_coins_without_laundering_a_foreign_root() {
    let (mut world, state, _) = realm(true, 1.0);
    let high = world.states[state].classical.unwrap().variety;
    let concept = by_id("water").unwrap();
    let v = &mut world.varieties[high];
    let source = v.lexicon.word_for(concept).unwrap().id;
    // Treat every earlier word for this meaning as foreign as well. A local
    // derivative of a loan is not a native archaism.
    let foreign = Origin::Borrowed {
        from: 1,
        source,
        cause: LoanCause::Coinage {
            donor: 1,
            recipient: 0,
        },
    };
    for word in &mut v.lexicon.lexemes {
        if word.first_sense.id == concept.id {
            word.origin = foreign;
        }
    }
    let derivative = v.lexicon.coin(
        Form::from_ipa("papata").unwrap(),
        Origin::Renewed {
            base: source,
            with: None,
        },
        concept,
        3,
    );
    v.lexicon.get_mut(derivative).obsolete = Some(4);
    let slot = CONCEPTS.iter().position(|c| c.id == concept.id).unwrap();
    let id = native_substitute(v, slot, 5, &mut stream(8, &[key("purism test")]));
    assert!(native(&v.lexicon, id));
    assert_ne!(id, derivative);
    assert_eq!(v.lexicon.get(id).born, 5);
    assert!(v.lexicon.get(id).form.vowel_count() > 0);
}

#[test]
fn foreign_prestige_cause_is_recorded_at_the_actual_state_rise() {
    let mut world = World::new(
        21,
        Params {
            state_rate: 100.0,
            purism_rate: 1.0,
            ..Params::static_society()
        },
    );
    let foreign = world.found(&SoundProfile::base(), 0.9, 0.5);
    let home = world.communities[foreign].home();
    let local = world.found_seeded(
        &crate::Naming::People,
        &SoundProfile::base(),
        8,
        0.2,
        0.5,
        Some(home),
        Some(crate::Livelihood::Farming),
        None,
    );
    world.communities[local].size = 60_000.0;
    let rival = world.raise_state(foreign, None, Rise::Proclaimed);
    world
        .connect(foreign, local, 1.0, ContactKind::Neighbours)
        .unwrap();
    world.rise_states();
    let state = world.rules(local).unwrap();
    let cause = world.purist_pressures[0].cause;
    assert_eq!(
        cause,
        Cause {
            event: world.triggers.states[&state],
            mechanism: Mechanism::ForeignPrestige
        }
    );
    assert_eq!(
        world.causes[&cause.event].event,
        world.triggers.states[&rival]
    );
    // Related varieties cannot become a foreign-language antagonist by power alone.
    world.purist_pressures.clear();
    world.communities[foreign].variety = world.communities[local].variety;
    world.purist_rival(state, rival);
    assert!(world.purist_pressures.is_empty());
}

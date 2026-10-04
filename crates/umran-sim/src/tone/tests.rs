use super::*;
use crate::{MinimalWord, Params, SoundProfile, StressRule, catalog};

fn form(ipa: &str) -> Form {
    Form::from_ipa(ipa).unwrap()
}
fn law(id: &str) -> Law {
    catalog().into_iter().find(|law| law.id == id).unwrap()
}
fn apply(id: &str, ipa: &str) -> Form {
    law(id).apply(&form(ipa), MinimalWord::Syllable, StressRule::Initial)
}

#[test]
fn coda_loss_preserves_contrasts_and_respects_word_size() {
    assert_eq!(apply("coda-tonogenesis", "kaʔ").ipa(), "ka˧˥");
    assert_eq!(apply("coda-tonogenesis", "kah").ipa(), "ka˥˩");
    assert_eq!(apply("coda-tonogenesis", "kas").ipa(), "ka˥˩");
    assert_eq!(apply("coda-tonogenesis", "kaska").ipa(), "kaska");
    assert_eq!(apply("coda-tonogenesis", "kans").ipa(), "kans");
    assert_eq!(
        law("coda-tonogenesis").apply(&form("kas"), MinimalWord::Heavy, StressRule::Initial),
        form("kas")
    );
    assert_eq!(
        law("coda-tonogenesis")
            .apply(&form("kaːs"), MinimalWord::Heavy, StressRule::Initial)
            .ipa(),
        "kaː˥˩"
    );
    assert_eq!(
        apply("unstressed-apocope", "ta˩").ipa(),
        "ta˩",
        "last vowel and its tone survive"
    );
}

#[test]
fn register_split_reads_old_voicing_and_keeps_contours() {
    assert_eq!(apply("register-tonogenesis", "bada").ipa(), "pa˩ta˩");
    assert_eq!(apply("register-tonogenesis", "pata").ipa(), "pa˥ta˥");
    assert_eq!(
        apply("register-tonogenesis", "ma").ipa(),
        "ma",
        "sonorants do not trigger an obstruent register split"
    );
    assert_eq!(apply("register-tonogenesis", "ba˧˥").ipa(), "pa˩˧");
    assert_eq!(apply("register-tonogenesis", "pa˧˥").ipa(), "pa˧˥");
    assert_eq!(apply("register-tonogenesis", "ba˥˩").ipa(), "pa˧˩");
    assert_eq!(
        apply("register-tonogenesis", "pa˩").ipa(),
        "pa˩",
        "recurrence must not erase the register left by old voicing"
    );
    let split = law("register-tonogenesis");
    let profile = SoundProfile::base();
    assert!(
        split
            .assess(
                [form("pata")].iter(),
                &profile.inventory,
                MinimalWord::Syllable,
                StressRule::Initial
            )
            .is_none()
    );
    assert!(
        split
            .assess(
                [form("bada"), form("pata")].iter(),
                &profile.inventory,
                MinimalWord::Syllable,
                StressRule::Initial
            )
            .is_some()
    );
}

#[test]
fn pitch_survives_quality_length_and_boundary_changes() {
    assert_eq!(apply("umlaut", "ka˧˥ti").ipa(), "ke˧˥ti");
    let mut before = form("ka˧˥tas");
    before.boundaries = vec![2];
    before.stress = Some(1);
    let after = law("coda-tonogenesis").apply(&before, MinimalWord::Syllable, StressRule::Free);
    assert_eq!(after.ipa(), "ka˧˥ta˥˩");
    assert_eq!(after.boundaries, vec![2]);
    assert_eq!(after.stress, Some(1));
    assert_eq!(after.ipa_stressed(StressRule::Free), "ka˧˥ˈta˥˩");
    assert_eq!(apply("tone-merger", "ka˧˥ta˥˩pa˩˧").ipa(), "ka˥ta˩pa˩");
    assert_eq!(apply("tone-loss", "ka˥ta˩").ipa(), "kata");
    assert_eq!(apply("tone-loss", "ka").ipa(), "ka");
    for tone in Tone::ALL {
        let ipa = format!("kaː{}", tone.ipa());
        assert_eq!(form(&ipa).ipa(), ipa);
    }
    assert!(Form::from_ipa("k˥a").is_none());
    assert!(Form::from_ipa("ka˧").is_none());
    let old: Form =
        serde_json::from_str(r#"{"segs":[{"phone":"k"},{"phone":"a","long":true}]}"#).unwrap();
    assert_eq!(old, form("kaː"));
    let saved = serde_json::to_string(&after).unwrap();
    assert_eq!(serde_json::from_str::<Form>(&saved).unwrap(), after);
}

#[test]
fn loans_follow_recipient_pitch_and_comparison_hears_tone() {
    let profile = SoundProfile::base();
    for (native, foreign, expected) in [
        ("ka", "ka˧˥", "ka"),
        ("ka˧˥", "ka˧˥", "ka˧˥"),
        ("ka˥", "ka˧˥", "ka˥"),
    ] {
        let recipient = [form(native), form(native)];
        let adapter = crate::adapt::Adapter::new(recipient.iter(), &profile.inventory);
        let borrowed = adapter.adapt(&form(foreign), 1.0, &mut crate::rng::stream(0, &[]));
        assert_eq!(borrowed.ipa(), expected);
    }
    assert_eq!(crate::compare::similarity(&form("ka˥"), &form("ka˥")), 1.0);
    assert!(crate::compare::similarity(&form("ka˥"), &form("ka˩")) < 1.0);
    assert!(crate::compare::similarity(&form("ka˥"), &form("ka")) < 1.0);
}

#[test]
fn regular_tone_laws_reach_living_words_grammar_and_names_not_obsolete_words() {
    let mut world = World::solo(8, &SoundProfile::base(), Params::static_society());
    let variety = &mut world.varieties[0];
    variety.minimal = MinimalWord::Syllable;
    for word in &mut variety.lexicon.lexemes {
        word.form = form("kas");
        for paradigm in &mut word.paradigms {
            for realization in &mut paradigm.realizations {
                if let Some(f) = &mut realization.form {
                    *f = form("takas");
                    realization.edge = 2;
                }
            }
        }
    }
    variety.lexicon.lexemes[0].obsolete = Some(0);
    for marker in &mut variety.grammar.markers {
        marker.form = form("kas");
    }
    variety.name.form = form("kas");
    world.communities[0].name.form = form("kas");
    world.generation = 3;
    world.apply_law(0, &law("coda-tonogenesis"));
    let variety = &world.varieties[0];
    assert_eq!(variety.tones(), 1);
    assert_eq!(
        variety.tonal,
        Some(Tonal {
            since: 3,
            lost: None
        })
    );
    assert!(
        variety
            .lexicon
            .living()
            .all(|word| word.form.ipa() == "ka˥˩")
    );
    assert_eq!(variety.lexicon.lexemes[0].form.ipa(), "kas");
    assert_eq!(variety.name.form.ipa(), "ka˥˩");
    assert_eq!(world.communities[0].name.form.ipa(), "ka˥˩");
    assert!(
        variety
            .grammar
            .forms(&variety.lexicon)
            .all(|(form, _)| form.ipa().ends_with("a˥˩"))
    );
    let daughter = variety.fork(0, 4);
    assert_eq!(daughter.tonal, variety.tonal);
    world.generation = 8;
    world.apply_law(0, &law("tone-loss"));
    assert_eq!(world.varieties[0].tones(), 0);
    assert_eq!(
        world.varieties[0].tonal,
        Some(Tonal {
            since: 3,
            lost: Some(8)
        })
    );
    assert!(matches!(
        world.events.last().unwrap().1,
        WorldEvent::Tone {
            gained: false,
            law: Some("tone-loss"),
            ..
        }
    ));
}

#[test]
fn tone_waves_record_contact_cause_and_static_society_stays_inert() {
    let mut world = World::solo(
        9,
        &SoundProfile::base(),
        Params {
            sound_change_rate: 0.0,
            innovation_rate: 0.0,
            loan_rate: 0.0,
            ..Params::static_society()
        },
    );
    for word in &mut world.varieties[0].lexicon.lexemes {
        word.form = form("kas");
    }
    world.varieties[0].minimal = MinimalWord::Syllable;
    world.run(4);
    assert_eq!(world.varieties[0].tones(), 0);
    let c = world.split(0, None, 1.0);
    world.communities[c].lands = world.communities[0].lands.clone();
    world
        .connect(0, c, 1.0, crate::ContactKind::Intermarriage)
        .unwrap();
    let cause = world.contacts.last().unwrap().cause.unwrap().event;
    let v = world.communities[c].variety;
    world.apply_law(0, &law("coda-tonogenesis"));
    world.params.wave_rate = 1000.0;
    world.step();
    assert!(world.varieties[v].tones() > 0);
    assert!(
        world.varieties[v]
            .waves
            .iter()
            .any(|&(_, id, from)| id == "coda-tonogenesis" && from == 0)
    );
    let event = world.events.iter().position(|(_, event)| matches!(event, WorldEvent::Tone { variety, gained: true, .. } if *variety == v)).unwrap();
    assert_eq!(world.causes.get(&event).unwrap().event, cause);
}

use super::*;
use crate::{Ethos, Livelihood, Origin, Params, Variety, World};

fn profile(choice: GrammarChoice) -> SoundProfile {
    let mut profile = SoundProfile::base();
    profile.grammar = GrammarPrior::fixed(GrammarDesign {
        object: Some(choice),
        ..GrammarDesign::default()
    });
    profile.stress = Some(StressRule::Initial);
    profile
}
fn variety(choice: GrammarChoice) -> Variety {
    Variety::found(9, &profile(choice), Livelihood::Farming, Ethos::default())
}
fn object(word: &crate::Lexeme) -> &Realization {
    word.paradigms
        .iter()
        .find(|p| p.category == Category::Object)
        .unwrap()
        .realizations
        .iter()
        .filter(|r| r.retired.is_none() && r.share > 0.0)
        .max_by(|a, b| a.share.total_cmp(&b.share))
        .unwrap()
}
fn erase_noun_contrast(v: &mut Variety) {
    for word in &mut v.lexicon.lexemes {
        if crate::pronouns::is_pronoun(word.first_sense) {
            continue;
        }
        for r in word
            .paradigms
            .iter_mut()
            .filter(|p| p.category == Category::Object)
            .flat_map(|p| &mut p.realizations)
            .filter(|r| r.retired.is_none())
        {
            r.form = Some(word.form.clone());
        }
    }
}

#[test]
fn pronouns_gain_object_paradigms_from_replaced_founding_stems() {
    for choice in [
        GrammarChoice::Suffix,
        GrammarChoice::Prefix,
        GrammarChoice::Particle,
        GrammarChoice::None,
    ] {
        let v = variety(choice);
        for (_, _, cell) in v.pronouns.cells() {
            let word = v.lexicon.word_for(by_id(cell).unwrap()).unwrap();
            assert!(!word.first_sense.categories.plural);
            assert!(word.first_sense.categories.object);
            let r = object(word);
            let expected =
                v.grammar
                    .materialize(r.marker, &word.form, &v.morphology, v.stress(), 0, 1.0);
            assert_eq!(r, &expected, "{cell} {choice:?}");
            assert_eq!(
                v.grammar.audible(&word.form, r, v.stress()),
                choice != GrammarChoice::None
            );
            assert_eq!(word.paradigms.len(), 1);
        }
    }
}

#[test]
fn renewed_pronoun_gets_current_object_form_without_stale_realizations() {
    let mut world = World::solo(
        9,
        &profile(GrammarChoice::Suffix),
        Params {
            pronoun_rate: 1.0,
            ..Params::static_society()
        },
    );
    let old = world.varieties[0]
        .lexicon
        .word_for(by_id("1sg").unwrap())
        .unwrap()
        .id;
    world.varieties[0].lexicon.get_mut(old).form = Form::from_ipa("a").unwrap();
    world.generation = 1;
    world.evolve_pronouns(&[true]);
    let v = &mut world.varieties[0];
    v.sync_grammar(1);
    let word = v.lexicon.word_for(by_id("1sg").unwrap()).unwrap();
    assert_ne!(word.id, old);
    assert!(matches!(word.origin, Origin::Renewed { .. }));
    let r = object(word);
    assert_eq!(r.born, 1);
    assert_eq!(
        r,
        &v.grammar
            .materialize(r.marker, &word.form, &v.morphology, v.stress(), 1, 1.0)
    );
    assert!(
        v.lexicon
            .get(old)
            .paradigms
            .iter()
            .flat_map(|p| &p.realizations)
            .all(|r| r.retired == Some(1))
    );
}

#[test]
fn early_fusion_is_full_share_and_never_acquired_by_nouns() {
    let mut v = variety(GrammarChoice::Particle);
    let stress = v.stress();
    v.grammar.evolve(
        9,
        0,
        1,
        &mut v.lexicon,
        &v.morphology,
        stress,
        &Params {
            pronoun_fusion_rate: 1.0,
            ..Params::default()
        },
    );
    assert_eq!(v.grammar.pronoun_case(), PronounCase::Ahead);
    let marker = v
        .grammar
        .markers
        .iter()
        .find(|m| m.scope == MarkerScope::Pronouns)
        .unwrap()
        .id;
    for (_, _, cell) in v.pronouns.cells() {
        let r = object(v.lexicon.word_for(by_id(cell).unwrap()).unwrap());
        assert_eq!((r.marker, r.share), (marker, 1.0));
    }
    let noun = by_id("stone").unwrap();
    let id = v
        .lexicon
        .coin(Form::from_ipa("kapa").unwrap(), Origin::Founding, noun, 2);
    v.lexicon.slot_mut(noun).variants.clear();
    v.lexicon.slot_mut(noun).introduce(id, 1.0);
    v.sync_grammar(2);
    assert_ne!(object(v.lexicon.get(id)).marker, marker);
    assert!(v.grammar.has_object_marking());
    assert_eq!(
        v.grammar
            .marker(v.grammar.productive(Category::Object).unwrap())
            .kind,
        MarkerKind::Particle
    );
    assert_eq!(
        v.grammar
            .acquire_pattern(Category::Object, v.lexicon.get(id), &v.morphology, 2),
        v.grammar.productive(Category::Object)
    );
}

#[test]
fn relic_survives_noun_loss_and_records_once_per_loss_episode() {
    let mut v = variety(GrammarChoice::Suffix);
    v.grammar.pronoun_case_enabled = true;
    let stress = v.stress();
    let before = object(v.lexicon.word_for(by_id("1sg").unwrap()).unwrap()).clone();
    erase_noun_contrast(&mut v);
    v.grammar.refresh(&v.lexicon, stress, 3);
    v.sync_grammar(3);
    v.sync_grammar(4);
    assert_eq!(v.grammar.pronoun_case(), PronounCase::Relic);
    assert!(!v.grammar.has_object_marking());
    let after = object(v.lexicon.word_for(by_id("1sg").unwrap()).unwrap());
    assert_eq!(after.form, before.form);
    assert_eq!(after.born, before.born);
    let relic = v.grammar.marker(after.marker);
    assert_eq!(relic.scope, MarkerScope::Pronouns);
    assert_eq!(relic.born, 3);
    assert!(!relic.productive);
    let notices = |v: &Variety| {
        v.grammar
            .events
            .iter()
            .filter(|e| matches!(e.event, NoticeKind::PronounRelic { .. }))
            .count()
    };
    assert_eq!(notices(&v), 1);
    let marker = v.grammar.productive(Category::Object).unwrap();
    for word in &mut v.lexicon.lexemes {
        if crate::pronouns::is_pronoun(word.first_sense) {
            continue;
        }
        for p in word
            .paradigms
            .iter_mut()
            .filter(|p| p.category == Category::Object)
        {
            p.realizations =
                vec![
                    v.grammar
                        .materialize(marker, &word.form, &v.morphology, stress, 5, 1.0),
                ];
        }
    }
    v.grammar.refresh(&v.lexicon, stress, 5);
    assert!(v.grammar.has_object_marking());
    erase_noun_contrast(&mut v);
    v.grammar.refresh(&v.lexicon, stress, 6);
    assert_eq!(notices(&v), 2);
}

#[test]
fn static_society_disables_pronoun_case_fusion_and_retention() {
    let params = Params::static_society();
    assert!(!params.pronoun_case);
    assert_eq!(params.pronoun_case_retention, 0.0);
    let mut v = variety(GrammarChoice::Particle);
    let stress = v.stress();
    v.grammar.evolve(
        9,
        0,
        1,
        &mut v.lexicon,
        &v.morphology,
        stress,
        &Params {
            pronoun_fusion_rate: 1.0,
            ..params
        },
    );
    assert!(
        v.grammar
            .markers
            .iter()
            .all(|m| m.scope == MarkerScope::All)
    );
    let mut v = variety(GrammarChoice::Suffix);
    erase_noun_contrast(&mut v);
    v.grammar.refresh(&v.lexicon, stress, 1);
    v.sync_grammar(1);
    assert!(
        v.grammar
            .markers
            .iter()
            .all(|m| m.scope == MarkerScope::All)
    );
    assert!(!v.grammar.events.iter().any(|e| matches!(
        e.event,
        NoticeKind::PronounFusion { .. } | NoticeKind::PronounRelic { .. }
    )));
}

#[test]
fn transferred_pronoun_marker_preserves_scope_and_never_marks_nouns() {
    let mut v = variety(GrammarChoice::None);
    let stress = v.stress();
    let id = v.lexicon.word_for(by_id("1sg").unwrap()).unwrap().id;
    v.grammar.import_pair(
        &mut v.lexicon,
        id,
        1,
        ImportedPair {
            category: Category::Object,
            source_marker: 7,
            kind: MarkerKind::Bound,
            scope: MarkerScope::Pronouns,
            side: Side::Suffix,
            marker_form: Form::from_ipa("ta").unwrap(),
            source_marker_form: Form::from_ipa("ta").unwrap(),
            form: Some(Form::from_ipa("mita").unwrap()),
            edge: 2,
            source_form: Form::from_ipa("mita").unwrap(),
        },
        stress,
        1,
    );
    let marker = v.grammar.markers.last().unwrap().id;
    assert!(!v.grammar.marker(marker).productive);
    v.grammar
        .transfer_marker(marker, 1, &mut v.lexicon, &v.morphology, stress, 2);
    assert_eq!(v.grammar.marker(marker).scope, MarkerScope::Pronouns);
    assert!(!v.grammar.has_object_marking());
    assert!(
        v.lexicon
            .living()
            .filter(|word| !crate::pronouns::is_pronoun(word.first_sense))
            .all(|word| word
                .paradigms
                .iter()
                .flat_map(|p| &p.realizations)
                .all(|r| r.marker != marker))
    );
}

#[test]
fn retention_stream_skips_only_pronoun_object_analogy() {
    let mut v = variety(GrammarChoice::Suffix);
    // One eligible word makes the existing analogy selection deterministic.
    let concept = by_id("1sg").unwrap();
    v.lexicon = Lexicon::found(std::iter::empty());
    let id = v
        .lexicon
        .coin(Form::from_ipa("ka").unwrap(), Origin::Founding, concept, 0);
    v.lexicon.slot_mut(concept).introduce(id, 1.0);
    v.sync_grammar(0);
    let p = &mut v.lexicon.get_mut(id).paradigms[0];
    p.realizations[0].form = Some(Form::from_ipa("mi").unwrap());
    let seed = (0..100_000)
        .find(|&seed| {
            let mut rng = stream(seed, &[key("grammar analogy"), 0, 1, key("object")]);
            rng.r#gen::<f32>() < 0.002
        })
        .unwrap();
    let mut levelled = v.clone();
    let stress = v.stress();
    for (variety, retention) in [(&mut v, 1.0), (&mut levelled, 0.0)] {
        variety.grammar.evolve(
            seed,
            0,
            1,
            &mut variety.lexicon,
            &variety.morphology,
            stress,
            &Params {
                pronoun_case_retention: retention,
                pronoun_fusion_rate: 0.0,
                ..Params::default()
            },
        );
    }
    assert_eq!(object(v.lexicon.get(id)).form.as_ref().unwrap().ipa(), "mi");
    assert_ne!(
        object(levelled.lexicon.get(id))
            .form
            .as_ref()
            .unwrap()
            .ipa(),
        "mi"
    );
    assert!(
        !v.grammar
            .events
            .iter()
            .any(|e| matches!(e.event, NoticeKind::Analogy { .. }))
    );
    assert!(
        levelled
            .grammar
            .events
            .iter()
            .any(|e| matches!(e.event, NoticeKind::Analogy { .. }))
    );
}

#[test]
fn scoped_forms_follow_harmony_sound_laws_and_historical_surface() {
    let mut v = variety(GrammarChoice::Particle);
    let particle = v.grammar.productive(Category::Object).unwrap();
    v.grammar.markers[particle as usize].side = Side::Suffix;
    v.grammar.markers[particle as usize].form = Form::from_ipa("kɑ").unwrap();
    v.grammar.harmony = Some(crate::harmony::Feature::Backness);
    let id = v.lexicon.word_for(by_id("1sg").unwrap()).unwrap().id;
    v.lexicon.get_mut(id).form = Form::from_ipa("pa").unwrap();
    let stress = v.stress();
    v.grammar.fuse_scoped(
        particle,
        MarkerScope::Pronouns,
        &mut v.lexicon,
        &v.morphology,
        stress,
        1,
    );
    let before = object(v.lexicon.get(id)).clone();
    assert_eq!(before.form.as_ref().unwrap().ipa(), "paka");
    let apocope = crate::catalog()
        .into_iter()
        .find(|l| l.id == "apocope")
        .unwrap();
    v.grammar
        .apply_law(&mut v.lexicon, &apocope, MinimalWord::Syllable, stress, 2);
    let after = object(v.lexicon.get(id));
    assert_eq!(after.form.as_ref().unwrap().ipa(), "pak");
    assert_eq!(
        v.grammar.surface_at(&v.lexicon.get(id).form, after, 1),
        vec![before.form.unwrap()]
    );
}

#[test]
fn minority_bound_competitors_are_not_ahead_or_visible_object_forms() {
    let mut v = variety(GrammarChoice::Particle);
    let particle = v.grammar.productive(Category::Object).unwrap();
    let stress = v.stress();
    let bound = v
        .grammar
        .fuse(particle, &mut v.lexicon, &v.morphology, stress, 1);
    assert_eq!(v.grammar.pronoun_case(), PronounCase::LikeNouns);
    let cells: Vec<_> = v
        .pronouns
        .cells()
        .map(|(_, _, cell)| v.lexicon.word_for(by_id(cell).unwrap()).unwrap().id)
        .collect();
    for id in cells.iter().take(cells.len().div_ceil(2)) {
        for r in &mut v.lexicon.get_mut(*id).paradigms[0].realizations {
            r.share = if r.marker == bound { 0.6 } else { 0.4 };
        }
    }
    v.grammar.refresh(&v.lexicon, stress, 2);
    assert_eq!(v.grammar.pronoun_case(), PronounCase::Ahead);
    for id in cells {
        let word = v.lexicon.get_mut(id);
        for r in &mut word.paradigms[0].realizations {
            r.share = if r.marker == bound { 0.18 } else { 0.82 };
            if r.marker == particle {
                r.form = Some(word.form.clone());
            }
        }
    }
    v.grammar.refresh(&v.lexicon, stress, 3);
    assert_eq!(v.grammar.pronoun_case(), PronounCase::None);
    assert!(
        v.grammar
            .pronoun_object(v.lexicon.word_for(by_id("1sg").unwrap()).unwrap(), stress)
            .is_none()
    );
}

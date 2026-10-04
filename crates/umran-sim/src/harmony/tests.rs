use super::*;
use crate::grammar::{Category, GrammarChoice, GrammarDesign, GrammarPrior};
use crate::{Ethos, Livelihood, Params, SoundProfile};

fn form(ipa: &str) -> Form {
    Form::from_ipa(ipa).unwrap()
}

fn variety() -> Variety {
    let mut profile = SoundProfile::by_id("finnic").unwrap();
    profile.grammar = GrammarPrior::fixed(GrammarDesign {
        plural: GrammarChoice::Suffix,
        past: GrammarChoice::None,
        object: Some(GrammarChoice::None),
        future: Some(GrammarChoice::None),
        progressive: Some(GrammarChoice::None),
        genitive: Some(GrammarChoice::None),
        ..GrammarDesign::default()
    });
    let mut v = Variety::found(7, &profile, Livelihood::Farming, Ethos::default());
    for (i, word) in v.lexicon.lexemes.iter_mut().enumerate() {
        word.form = form(if i % 2 == 0 { "piti" } else { "pyty" });
        word.paradigms.clear();
    }
    v.grammar.markers[0].form = form("ki");
    v.sync_grammar(0);
    v
}

fn world() -> World {
    let mut w = World::solo(
        7,
        &SoundProfile::by_id("finnic").unwrap(),
        Params::static_society(),
    );
    w.varieties[0] = variety();
    w
}

#[test]
fn harmony_spans_the_word_and_preserves_prosody() {
    for (feature, before, after) in [
        (Feature::Backness, "punitøkɑ", "punitokɑ"),
        (Feature::Rounding, "pykiteli", "pykytøly"),
        (Feature::Atr, "pɪtekoku", "pɪtɛkɔkʊ"),
    ] {
        let mut f = form(before);
        f.boundaries = vec![2, 4];
        f.stress = Some(2);
        f.segs[1].long = true;
        let old = f.clone();
        assert_eq!(feature.apply(&mut f), Some(old.clone()));
        assert_eq!(
            f.phones().collect::<Vec<_>>(),
            form(after).phones().collect::<Vec<_>>()
        );
        assert_eq!(f.boundaries, old.boundaries);
        assert_eq!(f.stress, old.stress);
        assert!(f.segs[1].long);
        assert_eq!(f.vowel_count(), old.vowel_count());
        assert_eq!(feature.apply(&mut f), None);
    }
}

#[test]
fn affixes_follow_stems_on_both_sides_and_particles_stay_separate() {
    let base = form("pyty");
    let mut suffix = form("pytyki");
    Feature::Rounding.realize_bound(&mut suffix, 4, Side::Suffix, &base);
    assert_eq!(suffix.ipa(), "pytyky");
    let mut prefix = form("kipyty");
    Feature::Rounding.realize_bound(&mut prefix, 2, Side::Prefix, &base);
    assert_eq!(prefix.ipa(), "kypyty");
    let mut particle = form("ki");
    assert_eq!(Feature::Rounding.apply(&mut particle), None);
    assert_eq!(particle.ipa(), "ki");
    let alternants: Vec<_> = Feature::Rounding
        .alternants(&particle)
        .iter()
        .map(Form::ipa)
        .collect();
    assert_eq!(alternants, ["ki", "ky"]);
}

#[test]
fn emergence_requires_recent_assimilation_and_static_societies_disable_it() {
    let mut w = world();
    w.generation = 30;
    w.params.harmony_rate = 1.0;
    w.evolve_harmony(&[true]);
    assert!(w.varieties[0].harmony.is_none());
    w.varieties[0].laws.push((1, "rounding-assimilation"));
    w.evolve_harmony(&[true]);
    assert!(w.varieties[0].harmony.is_none());
    w.varieties[0].laws.push((29, "rounding-assimilation"));
    w.params.harmony_rate = Params::static_society().harmony_rate;
    w.evolve_harmony(&[true]);
    assert!(w.varieties[0].harmony.is_none());
    w.params.harmony_rate = 1.0;
    w.evolve_harmony(&[true]);
    let h = w.varieties[0].harmony.as_ref().unwrap();
    assert_eq!((h.feature, h.since, h.lost), (Feature::Rounding, 30, None));
    assert_eq!(
        w.varieties[0].harmony_events[0].trigger,
        Trigger::Assimilation {
            generation: 29,
            law: "rounding-assimilation"
        }
    );
}

#[test]
fn productive_forms_and_living_names_change_but_obsolete_forms_do_not() {
    let mut w = world();
    let v = &mut w.varieties[0];
    v.harmony = Some(Harmony::new(Feature::Rounding, 1, 7));
    v.lexicon.lexemes[0].form = form("pyti");
    v.lexicon.lexemes[0].obsolete = Some(0);
    v.lexicon.lexemes[1].form = form("pyti");
    v.name.form = form("pyti");
    v.exonyms.push((
        0,
        crate::Name {
            form: form("pyti"),
            ..Default::default()
        },
    ));
    let frozen = v.lexicon.lexemes[0].clone();
    w.generation = 1;
    w.varieties[0].sync_grammar(1);
    w.harmonize_names(0);
    let v = &w.varieties[0];
    assert_eq!(v.lexicon.lexemes[0].form, frozen.form);
    assert_eq!(v.lexicon.lexemes[0].log, frozen.log);
    assert_eq!(v.lexicon.lexemes[1].form.ipa(), "pyty");
    assert_eq!(v.name.form.ipa(), "pyty");
    assert_eq!(v.name.form_at(0).ipa(), "pyti");
    assert_eq!(v.exonyms[0].1.form.ipa(), "pyty");
    let noun = v
        .lexicon
        .word_for(crate::concepts::by_id("child").unwrap())
        .unwrap();
    let plural = &noun
        .paradigms
        .iter()
        .find(|p| p.category == Category::Plural)
        .unwrap()
        .realizations[0];
    assert!(!Feature::Rounding.disharmonic(plural.form.as_ref().unwrap()));
}

#[test]
fn post_gain_loans_can_remain_disharmonic_but_their_endings_agree() {
    let mut kept = 0;
    for seed in 0..40 {
        let mut v = variety();
        v.harmony = Some(Harmony::new(Feature::Rounding, 1, seed));
        v.sync_grammar(1);
        let concept = crate::concepts::by_id("child").unwrap();
        let id = v.lexicon.coin(
            form("pity"),
            Origin::Borrowed {
                from: 1,
                source: LexemeId(0),
                cause: crate::LoanCause::City {
                    city: 0,
                    community: 0,
                },
            },
            concept,
            2,
        );
        v.lexicon.slot_mut(concept).introduce(id, 1.0);
        v.sync_grammar(2);
        let exempt = v.harmony.as_ref().unwrap().disharmonic_loans.contains(&id);
        kept += usize::from(exempt);
        assert_eq!(
            v.lexicon.get(id).form.ipa(),
            if exempt { "pity" } else { "piti" }
        );
        let plural = &v
            .lexicon
            .get(id)
            .paradigms
            .iter()
            .find(|p| p.category == Category::Plural)
            .unwrap()
            .realizations[0];
        let surface = plural.form.as_ref().unwrap();
        assert_eq!(
            surface.segs.last().unwrap().phone,
            CATALOG.id_by_ipa(if exempt { "y" } else { "i" }).unwrap()
        );
        let before = v.lexicon.get(id).clone();
        v.sync_grammar(3);
        assert_eq!(&before, v.lexicon.get(id));
    }
    assert!((15..=35).contains(&kept), "exemptions {kept}/40");
}

#[test]
fn merger_stops_productivity_and_does_not_undo_old_forms() {
    let mut w = world();
    w.varieties[0].harmony = Some(Harmony::new(Feature::Rounding, 0, 7));
    w.varieties[0].sync_grammar(0);
    let law = crate::Law {
        id: "test-unround",
        label: "unround",
        commonness: 0.0,
        stress: None,
        rules: [("y", "i"), ("ø", "e")]
            .into_iter()
            .map(|(a, b)| crate::SoundChange {
                id: "test-unround".into(),
                target: crate::Matcher::Phone(CATALOG.id_by_ipa(a).unwrap()),
                result: crate::Rewrite::Phone(CATALOG.id_by_ipa(b).unwrap()),
                left: crate::Env::Any,
                right: crate::Env::Any,
            })
            .collect(),
    };
    w.generation = 2;
    w.apply_law(0, &law);
    assert_eq!(w.varieties[0].harmony.as_ref().unwrap().lost, Some(2));
    assert_eq!(w.varieties[0].grammar.harmony, None);
    assert_eq!(
        w.varieties[0].harmony_events[0].trigger,
        Trigger::ContrastMerger {
            law: "test-unround"
        }
    );
}

#[test]
fn contact_loss_needs_sustained_contact_with_a_nonharmonic_donor() {
    let mut w = world();
    w.params.harmony_rate = 1.0;
    w.varieties.push(variety());
    w.varieties[0].harmony = Some(Harmony::new(Feature::Rounding, 0, 7));
    w.varieties[1].harmony = Some(Harmony::new(Feature::Rounding, 0, 8));
    w.varieties[0].grammar.contact_generations.insert(1, 100);
    for g in 1..20 {
        w.generation = g;
        w.evolve_harmony(&[true, false]);
    }
    assert!(w.varieties[0].harmony.as_ref().unwrap().active());
    w.varieties[1].harmony = None;
    w.varieties[0].grammar.contact_generations.insert(1, 11);
    for g in 20..40 {
        w.generation = g;
        w.evolve_harmony(&[true, false]);
    }
    assert!(w.varieties[0].harmony.as_ref().unwrap().active());
    w.varieties[0].grammar.contact_generations.insert(1, 100);
    for g in 40..80 {
        w.generation = g;
        w.evolve_harmony(&[true, false]);
    }
    assert!(!w.varieties[0].harmony.as_ref().unwrap().active());
    assert!(matches!(
        w.varieties[0].harmony_events[0].trigger,
        Trigger::Contact { donor: 1, .. }
    ));
}

#[test]
fn rare_harmonic_vowels_survive_consonant_laws_below_established_threshold() {
    // Audit: 184 living words, only three /o/ words, yet /o ~ ø/ survives.
    // The old established() check lost harmony after degemination.
    let mut w = world();
    let v = &mut w.varieties[0];
    for word in &mut v.lexicon.lexemes {
        word.form = form("pøtø");
        word.paradigms.clear();
    }
    let concept = crate::concepts::by_id("child").unwrap();
    while v.lexicon.living().count() < 184 {
        v.lexicon.coin(form("pøtø"), Origin::Expressive, concept, 0);
    }
    for word in v.lexicon.lexemes.iter_mut().take(3) {
        word.form = form("poːtːo");
    }
    v.harmony = Some(Harmony::new(Feature::Backness, 0, 7));
    v.sync_grammar(0);
    assert!(!v.established().contains(&CATALOG.id_by_ipa("o").unwrap()));
    assert!(Feature::Backness.has_contrast(v));
    let law = w
        .law_catalog()
        .iter()
        .find(|l| l.id == "degemination")
        .unwrap()
        .clone();
    w.generation = 115;
    w.apply_law(0, &law);
    assert!(w.varieties[0].harmony.as_ref().unwrap().active());
    assert!(w.varieties[0].harmony_events.is_empty());
}

#[test]
fn lexical_attrition_is_not_credited_to_the_next_consonant_law() {
    let mut w = world();
    let v = &mut w.varieties[0];
    v.harmony = Some(Harmony::new(Feature::Rounding, 0, 7));
    v.sync_grammar(0);
    for word in &mut v.lexicon.lexemes {
        if word
            .form
            .phones()
            .any(|p| p == CATALOG.id_by_ipa("y").unwrap())
        {
            word.obsolete = Some(1);
        }
    }
    let law = w
        .law_catalog()
        .iter()
        .find(|l| l.id == "degemination")
        .unwrap()
        .clone();
    w.generation = 2;
    w.apply_law(0, &law);
    assert_eq!(
        w.varieties[0].harmony_events[0].trigger,
        Trigger::LexicalAttrition
    );
}

#[test]
fn collapse_of_productive_affixes_stops_harmony_with_stem_contrast_intact() {
    let mut w = world();
    let v = &mut w.varieties[0];
    // Stems end in /t/; only affix vowels are word-final.
    for (i, word) in v.lexicon.lexemes.iter_mut().enumerate() {
        word.form = form(if i % 2 == 0 { "pit" } else { "pyt" });
        word.paradigms.clear();
    }
    v.harmony = Some(Harmony::new(Feature::Rounding, 0, 7));
    v.sync_grammar(0);
    assert!(Feature::Rounding.has_affix_contrast(v));
    let law = crate::Law {
        id: "test-final-unround",
        label: "final vowels unround",
        commonness: 0.0,
        stress: None,
        rules: vec![crate::SoundChange {
            id: "test-final-unround".into(),
            target: crate::Matcher::Phone(CATALOG.id_by_ipa("y").unwrap()),
            result: crate::Rewrite::Phone(CATALOG.id_by_ipa("i").unwrap()),
            left: crate::Env::Any,
            right: crate::Env::WordEdge,
        }],
    };
    w.generation = 2;
    w.apply_law(0, &law);
    assert!(Feature::Rounding.has_contrast(&w.varieties[0]));
    assert!(!Feature::Rounding.has_affix_contrast(&w.varieties[0]));
    assert_eq!(w.varieties[0].harmony.as_ref().unwrap().lost, Some(2));
    assert_eq!(
        w.varieties[0].harmony_events[0].trigger,
        Trigger::ContrastMerger {
            law: "test-final-unround"
        }
    );
}

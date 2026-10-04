use super::*;
use crate::concepts::by_id;
use crate::grammar::{Paradigm, Side};
use crate::{
    CATALOG, Env, Form, Law, Livelihood, Matcher, MinimalWord, Params, Rewrite, SoundChange,
    SoundProfile, Variety,
};

fn fixture() -> (Grammar, Lexicon, Morphology) {
    let variety = Variety::found(
        52,
        &SoundProfile::by_id("semitic").unwrap(),
        Livelihood::Farming,
        Default::default(),
    );
    let mut grammar = Grammar::default();
    grammar.add_marker(
        Category::Plural,
        MarkerKind::Bound,
        Side::Suffix,
        Form::from_ipa("at").unwrap(),
        MarkerOrigin::Founding,
        true,
        0,
    );
    let template = Template::from_form(&Form::from_ipa("kutub").unwrap(), 1.0).unwrap();
    let pattern = grammar.add_marker(
        Category::Plural,
        MarkerKind::Pattern,
        Side::Suffix,
        template.melody(),
        MarkerOrigin::Founding,
        true,
        0,
    );
    grammar.markers[pattern as usize].template = Some(template);
    (
        grammar,
        Lexicon::found(std::iter::empty()),
        variety.morphology,
    )
}

fn coin(lexicon: &mut Lexicon, ipa: &str, origin: Origin, born: u32) -> crate::LexemeId {
    let sense = by_id("person").unwrap();
    let id = lexicon.coin(Form::from_ipa(ipa).unwrap(), origin, sense, born);
    lexicon.slot_mut(sense).introduce(id, 0.1);
    id
}

fn pattern_word(
    grammar: &Grammar,
    lexicon: &mut Lexicon,
    morphology: &Morphology,
    ipa: &str,
) -> crate::LexemeId {
    let id = coin(lexicon, ipa, Origin::Founding, 0);
    let r = grammar.materialize(
        1,
        &lexicon.get(id).form,
        morphology,
        StressRule::Initial,
        0,
        1.0,
    );
    lexicon.get_mut(id).paradigms.push(Paradigm {
        category: Category::Plural,
        realizations: vec![r],
    });
    id
}

fn vowel_law(left: Env, result: Rewrite) -> Law {
    Law {
        id: "test-melody",
        label: "Test melody",
        commonness: 1.0,
        stress: None,
        rules: vec![SoundChange {
            id: "test-melody".into(),
            target: Matcher::Phone(CATALOG.id_by_ipa("u").unwrap()),
            result,
            left,
            right: Env::Any,
        }],
    }
}

#[test]
fn roots_exclude_derivational_prefixes_and_unfitting_loans_take_affixes() {
    let (mut grammar, mut lexicon, mut morphology) = fixture();
    morphology.patterns = vec![(
        crate::Relation::Place,
        crate::morphology::Pattern(vec![
            Slot::Fixed(CATALOG.id_by_ipa("m").unwrap()),
            Slot::Fixed(CATALOG.id_by_ipa("a").unwrap()),
            Slot::Root(0),
            Slot::Root(1),
            Slot::Fixed(CATALOG.id_by_ipa("a").unwrap()),
            Slot::Root(2),
        ]),
    )];
    let derived = coin(&mut lexicon, "maktab", Origin::Founding, 0);
    let short = coin(&mut lexicon, "ka", Origin::Expressive, 1);
    let long = coin(&mut lexicon, "katabman", Origin::Expressive, 1);
    grammar.sync(&mut lexicon, &morphology, StressRule::Initial, 0);
    let r = &lexicon.get(derived).paradigms[0].realizations[0];
    assert_eq!(r.marker, 1);
    assert_eq!(r.form.as_ref().unwrap().ipa(), "kutub");
    for id in [short, long] {
        let r = &lexicon.get(id).paradigms[0].realizations[0];
        assert_eq!(r.marker, 0);
        assert!(r.form.as_ref().unwrap().ipa().ends_with("at"));
    }
    let before = lexicon.clone();
    grammar.sync(&mut lexicon, &morphology, StressRule::Initial, 12);
    assert_eq!(lexicon, before, "sync must not remint existing forms");
}

#[test]
fn unanimous_vowel_changes_update_productivity_but_conditioned_allomorphs_do_not() {
    let (mut grammar, mut lexicon, morphology) = fixture();
    let a = pattern_word(&grammar, &mut lexicon, &morphology, "katab");
    let b = pattern_word(&grammar, &mut lexicon, &morphology, "salad");
    let mut contextual = grammar.clone();
    let mut contextual_words = lexicon.clone();
    let law = vowel_law(
        Env::Matcher(Matcher::Phone(CATALOG.id_by_ipa("k").unwrap())),
        Rewrite::Phone(CATALOG.id_by_ipa("i").unwrap()),
    );
    contextual.apply_law(
        &mut contextual_words,
        &law,
        MinimalWord::Syllable,
        StressRule::Initial,
        1,
    );
    assert_eq!(
        contextual_words.get(a).paradigms[0].realizations[0]
            .form
            .as_ref()
            .unwrap()
            .ipa(),
        "kitub"
    );
    assert_eq!(
        contextual_words.get(b).paradigms[0].realizations[0]
            .form
            .as_ref()
            .unwrap()
            .ipa(),
        "sulud"
    );
    assert_eq!(
        contextual.marker(1).template.as_ref().unwrap().notation(),
        "C1uC2uC3"
    );
    let law = vowel_law(Env::Any, Rewrite::Phone(CATALOG.id_by_ipa("i").unwrap()));
    grammar.apply_law(
        &mut lexicon,
        &law,
        MinimalWord::Syllable,
        StressRule::Initial,
        2,
    );
    assert_eq!(
        grammar.marker(1).template.as_ref().unwrap().notation(),
        "C1iC2iC3"
    );
    assert!(
        matches!(&grammar.marker(1).history.last().unwrap().event, GrammarEvent::SoundLaw { before, .. } if before.ipa() == "uu")
    );
    let template = grammar.marker(1).template.as_ref().unwrap();
    assert_eq!(
        template
            .realize(
                &Form::from_ipa("namar").unwrap(),
                &morphology,
                StressRule::Initial
            )
            .unwrap()
            .ipa(),
        "nimir"
    );
    let r = &lexicon.get(a).paradigms[0].realizations[0];
    assert_eq!(
        grammar.surface_at(&lexicon.get(a).form, r, 0)[0].ipa(),
        "kutub"
    );
    assert_eq!(r.form.as_ref().unwrap().ipa(), "kitib");
}

#[test]
fn length_changes_generalize_and_retired_forms_stay_frozen() {
    let (mut grammar, mut lexicon, morphology) = fixture();
    let live = pattern_word(&grammar, &mut lexicon, &morphology, "katab");
    let old = pattern_word(&grammar, &mut lexicon, &morphology, "salad");
    let retired = &mut lexicon.get_mut(old).paradigms[0].realizations[0];
    retired.retired = Some(1);
    retired.share = 0.0;
    let before = retired.clone();
    grammar.apply_law(
        &mut lexicon,
        &vowel_law(Env::Any, Rewrite::Length(true)),
        MinimalWord::Syllable,
        StressRule::Initial,
        2,
    );
    assert_eq!(
        grammar.marker(1).template.as_ref().unwrap().notation(),
        "C1uːC2uːC3"
    );
    assert_eq!(
        lexicon.get(live).paradigms[0].realizations[0]
            .form
            .as_ref()
            .unwrap()
            .ipa(),
        "kuːtuːb"
    );
    assert_eq!(lexicon.get(old).paradigms[0].realizations[0], before);
}

#[test]
fn levelling_records_the_replaced_internal_form_and_static_rate_is_inert() {
    let (mut grammar, mut lexicon, morphology) = fixture();
    let id = pattern_word(&grammar, &mut lexicon, &morphology, "katab");
    grammar.refresh(&lexicon, StressRule::Initial, 0);
    let original = lexicon.clone();
    grammar.level_patterns(
        12,
        0,
        1,
        &mut lexicon,
        &morphology,
        StressRule::Initial,
        Params::static_society().pattern_analogy_rate,
    );
    assert_eq!(lexicon, original);
    grammar.level_patterns(
        12,
        0,
        2,
        &mut lexicon,
        &morphology,
        StressRule::Initial,
        1.0,
    );
    let r = &lexicon.get(id).paradigms[0].realizations[0];
    assert_eq!(r.marker, 0);
    assert_eq!(r.share, 1.0);
    assert!(
        matches!(&r.history.last().unwrap().event, GrammarEvent::Analogy { before } if before.ipa() == "kutub")
    );
    assert!(
        matches!(&grammar.events.last().unwrap().event, NoticeKind::Analogy { lexeme, before, after } if *lexeme == id && before.ipa() == "kutub" && after == r.form.as_ref().unwrap())
    );
}

#[test]
fn acquisition_uses_current_productivity_and_loans_favour_affixes() {
    let (mut grammar, mut lexicon, morphology) = fixture();
    grammar.summary.marker_shares = vec![0.2, 0.8];
    let id = coin(&mut lexicon, "katab", Origin::Expressive, 1);
    let native = lexicon.get(id).clone();
    let mut loan = native.clone();
    loan.origin = Origin::Borrowed {
        from: 1,
        source: crate::LexemeId(0),
        cause: crate::provenance::LoanCause::Unrecorded,
    };
    let mut native_patterns = 0;
    let mut loan_patterns = 0;
    for seed in 0..1000 {
        grammar.pattern_seed = seed;
        native_patterns += usize::from(
            grammar.acquire_pattern(Category::Plural, &native, &morphology, 1) == Some(1),
        );
        loan_patterns += usize::from(
            grammar.acquire_pattern(Category::Plural, &loan, &morphology, 1) == Some(1),
        );
    }
    assert!((550..730).contains(&native_patterns), "{native_patterns}");
    assert!((240..400).contains(&loan_patterns), "{loan_patterns}");
    grammar.summary.marker_shares = vec![1.0, 0.0];
    assert_eq!(
        grammar.acquire_pattern(Category::Plural, &native, &morphology, 1),
        Some(0)
    );
}

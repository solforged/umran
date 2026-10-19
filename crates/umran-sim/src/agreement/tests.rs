use super::*;
use crate::{Ethos, Livelihood, Params, Variety, World};

fn profile(order: WordOrder) -> SoundProfile {
    let mut profile = SoundProfile::base();
    profile.grammar = GrammarPrior::fixed(GrammarDesign {
        past: GrammarChoice::Suffix,
        order: Some(order),
        ..GrammarDesign::default()
    });
    profile.stress = Some(StressRule::Initial);
    profile.phonotactics.open_medial = false;
    profile
}
fn variety(order: WordOrder) -> Variety {
    Variety::found(9, &profile(order), Livelihood::Farming, Ethos::default())
}
/// Every transition open, so one `evolve` takes the next step of the cycle.
fn forced() -> Params {
    Params {
        agreement_rate: 1.0,
        agreement_fusion_rate: 1.0,
        pro_drop_rate: 1.0,
        ..Params::static_society()
    }
}
fn evolve(v: &mut Variety, generation: u32, params: &Params) {
    let stress = v.stress();
    v.grammar.evolve(
        9,
        0,
        generation,
        &mut v.lexicon,
        &v.morphology,
        stress,
        params,
    );
}
/// Cells a language lacks (a plain plural under clusivity) are passed over.
fn set_pronouns(v: &mut Variety, forms: &[(&str, &str)]) {
    for (cell, ipa) in forms {
        let Some(word) = v.lexicon.word_for(by_id(cell).unwrap()) else {
            continue;
        };
        let id = word.id;
        v.lexicon.get_mut(id).form = Form::from_ipa(ipa).unwrap();
    }
}
const PRONOUNS: [(&str, &str); 6] = [
    ("1sg", "kaːmi"),
    ("1pl", "miko"),
    ("2sg", "tunu"),
    ("2pl", "sata"),
    ("3sg", "pa"),
    ("3pl", "lupi"),
];
fn ipa(v: &Variety, cell: &str) -> String {
    v.grammar.agreement().cell(cell).unwrap().form.ipa()
}
fn kinds(v: &Variety) -> Vec<AgreementKind> {
    v.grammar
        .agreement()
        .events
        .iter()
        .map(|e| e.kind)
        .collect()
}

#[test]
fn cliticization_cuts_the_syllable_nearest_the_verb() {
    for (order, side, expected) in [
        (
            WordOrder::SVO,
            Side::Prefix,
            ["kaː", "mi", "tu", "sa", "pa", "lu"],
        ),
        (
            WordOrder::SOV,
            Side::Prefix,
            ["kaː", "mi", "tu", "sa", "pa", "lu"],
        ),
        (
            WordOrder::VSO,
            Side::Suffix,
            ["mi", "ko", "nu", "ta", "pa", "pi"],
        ),
    ] {
        let mut v = variety(order);
        set_pronouns(&mut v, &PRONOUNS);
        evolve(&mut v, 1, &forced());
        let agreement = v.grammar.agreement();
        assert_eq!(agreement.stage, AgreementStage::Clitic);
        assert_eq!(agreement.side, side);
        assert_eq!(agreement.since, Some(1));
        assert!(!agreement.pro_drop);
        assert_eq!(v.grammar.summary.agreement, AgreementStage::Clitic);
        assert_eq!(kinds(&v), [AgreementKind::Clitics]);
        for ((cell, _), expected) in PRONOUNS.iter().zip(expected) {
            let form = agreement.cell(cell).unwrap();
            assert_eq!(form.form.ipa(), expected, "{order:?} {cell}");
            assert_eq!(form.form.stress, None);
            assert_eq!(form.born, 1);
            assert_eq!(
                form.source,
                v.lexicon.word_for(by_id(cell).unwrap()).unwrap().id
            );
            assert_eq!(form.history.len(), 1);
            assert!(matches!(form.history[0].event, GrammarEvent::Created));
            assert!(agreement.distinct(form, v.stress()));
        }
        assert_eq!(agreement.cell("2sg-polite").unwrap().cell, "2pl");
        assert_eq!(agreement.living().count(), 6);
    }
}

#[test]
fn whole_pronouns_cliticize_when_their_syllables_would_merge() {
    let mut v = variety(WordOrder::SVO);
    set_pronouns(&mut v, &[("1sg", "kami"), ("1pl", "kamu")]);
    evolve(&mut v, 1, &forced());
    assert_eq!(v.grammar.agreement().stage, AgreementStage::Clitic);
    assert_eq!(ipa(&v, "1sg"), "kami");
    assert_eq!(ipa(&v, "1pl"), "kamu");
}

#[test]
fn short_or_merged_pronouns_and_static_society_never_cliticize() {
    let mut v = variety(WordOrder::SVO);
    set_pronouns(&mut v, &PRONOUNS);
    set_pronouns(&mut v, &[("3sg", "a")]);
    evolve(&mut v, 1, &forced());
    assert_eq!(v.grammar.agreement().stage, AgreementStage::None);
    let mut v = variety(WordOrder::SVO);
    set_pronouns(&mut v, &PRONOUNS);
    set_pronouns(&mut v, &[("3sg", "tunu")]);
    evolve(&mut v, 1, &forced());
    assert_eq!(v.grammar.agreement().stage, AgreementStage::None);
    assert!(kinds(&v).is_empty());
    let params = Params::static_society();
    assert_eq!(
        (
            params.agreement_rate,
            params.agreement_fusion_rate,
            params.pro_drop_rate
        ),
        (0.0, 0.0, 0.0)
    );
    let mut world = World::solo(9, &profile(WordOrder::SVO), params);
    set_pronouns(&mut world.varieties[0], &PRONOUNS);
    world.run(40);
    let agreement = world.varieties[0].grammar.agreement();
    assert_eq!(agreement.stage, AgreementStage::None);
    assert!(agreement.events.is_empty() && agreement.cells.is_empty());
}

#[test]
fn clitics_fuse_after_eight_generations_and_distinct_affixes_license_pro_drop() {
    let mut v = variety(WordOrder::SOV);
    set_pronouns(&mut v, &PRONOUNS);
    evolve(&mut v, 1, &forced());
    evolve(&mut v, 8, &forced());
    assert_eq!(v.grammar.agreement().stage, AgreementStage::Clitic);
    evolve(&mut v, 9, &forced());
    let agreement = v.grammar.agreement();
    assert_eq!(agreement.stage, AgreementStage::Affix);
    assert_eq!(agreement.since, Some(9));
    assert!(!agreement.pro_drop);
    assert_eq!(kinds(&v), [AgreementKind::Clitics, AgreementKind::Affixes]);
    // Three of six cells alike: half distinct, no pro-drop, no loss.
    let mut levelled = v.clone();
    let same = Form::from_ipa("ta").unwrap();
    for cell in ["1sg", "2sg", "3sg"] {
        let i = levelled
            .grammar
            .agreement
            .cells
            .iter()
            .position(|c| c.cell == cell)
            .unwrap();
        levelled.grammar.agreement.cells[i].form = same.clone();
    }
    evolve(&mut levelled, 10, &forced());
    assert_eq!(levelled.grammar.agreement().stage, AgreementStage::Affix);
    assert!(!levelled.grammar.agreement().pro_drop);
    evolve(&mut v, 10, &forced());
    let agreement = v.grammar.agreement();
    assert!(agreement.pro_drop);
    assert_eq!(agreement.stage, AgreementStage::Affix);
    assert_eq!(agreement.since, Some(9));
    assert!(v.grammar.summary.pro_drop);
    assert_eq!(
        kinds(&v),
        [
            AgreementKind::Clitics,
            AgreementKind::Affixes,
            AgreementKind::ProDrop
        ]
    );
    // A further forced generation has nothing left to draw for.
    evolve(&mut v, 11, &forced());
    assert_eq!(kinds(&v).len(), 3);
}

#[test]
fn worn_affixes_lose_the_contrast_and_renewed_pronouns_restart_after_twelve() {
    let mut v = variety(WordOrder::SVO);
    set_pronouns(&mut v, &PRONOUNS);
    evolve(&mut v, 1, &forced());
    evolve(&mut v, 9, &forced());
    evolve(&mut v, 10, &forced());
    assert!(v.grammar.agreement().pro_drop);
    let same = Form::from_ipa("ta").unwrap();
    for cell in ["1sg", "2sg", "3sg", "1pl"] {
        let i = v
            .grammar
            .agreement
            .cells
            .iter()
            .position(|c| c.cell == cell)
            .unwrap();
        v.grammar.agreement.cells[i].form = same.clone();
    }
    let stress = v.stress();
    v.grammar.refresh(&v.lexicon, stress, 20);
    let agreement = v.grammar.agreement();
    assert_eq!(agreement.stage, AgreementStage::None);
    assert!(!agreement.pro_drop);
    assert_eq!(agreement.since, Some(20));
    assert_eq!(agreement.living().count(), 0);
    assert_eq!(agreement.cells.len(), 6);
    for cell in &agreement.cells {
        assert_eq!(cell.retired, Some(20));
        assert!(matches!(
            cell.history.last().unwrap(),
            GrammarEntry {
                generation: 20,
                event: GrammarEvent::Retired
            }
        ));
    }
    assert_eq!(v.grammar.summary.agreement, AgreementStage::None);
    assert_eq!(kinds(&v).last(), Some(&AgreementKind::Loss));
    // Renewed pronouns wait twelve generations before cliticizing again.
    set_pronouns(&mut v, &[("1sg", "nami"), ("2sg", "roto")]);
    evolve(&mut v, 31, &forced());
    assert_eq!(v.grammar.agreement().stage, AgreementStage::None);
    evolve(&mut v, 32, &forced());
    let agreement = v.grammar.agreement();
    assert_eq!(agreement.stage, AgreementStage::Clitic);
    assert_eq!(agreement.since, Some(32));
    assert_eq!(agreement.living().count(), 6);
    assert_eq!(agreement.cells.len(), 12);
    assert_eq!(ipa(&v, "1sg"), "na");
    assert_eq!(ipa(&v, "2sg"), "ro");
    assert!(agreement.living().all(|c| c.born == 32));
    assert_eq!(
        kinds(&v),
        [
            AgreementKind::Clitics,
            AgreementKind::Affixes,
            AgreementKind::ProDrop,
            AgreementKind::Loss,
            AgreementKind::Clitics
        ]
    );
}

#[test]
fn agreement_forms_wear_by_sound_law_with_history_and_can_merge() {
    let mut v = variety(WordOrder::SVO);
    set_pronouns(&mut v, &PRONOUNS);
    set_pronouns(&mut v, &[("1sg", "mako")]);
    evolve(&mut v, 1, &forced());
    assert_eq!(ipa(&v, "1sg"), "ma");
    let law = Law {
        id: "test-raising",
        label: "Test raising",
        commonness: 1.0,
        stress: None,
        rules: vec![crate::SoundChange {
            id: "test-raising".into(),
            target: crate::Matcher::Phone(crate::CATALOG.id_by_ipa("a").unwrap()),
            result: crate::Rewrite::Phone(crate::CATALOG.id_by_ipa("i").unwrap()),
            left: crate::Env::Any,
            right: crate::Env::Any,
        }],
    };
    let stress = v.stress();
    v.grammar
        .apply_law(&mut v.lexicon, &law, MinimalWord::Syllable, stress, 2);
    let agreement = v.grammar.agreement();
    let cell = agreement.cell("1sg").unwrap();
    assert_eq!(cell.form.ipa(), "mi");
    assert_eq!(cell.form_at(1).ipa(), "ma");
    assert!(matches!(
        &cell.history[1],
        GrammarEntry {
            generation: 2,
            event: GrammarEvent::SoundLaw {
                law: "test-raising",
                before
            }
        } if before.ipa() == "ma"
    ));
    // It now sounds like the plural: neither tells its person apart.
    assert!(!agreement.distinct(cell, stress));
    assert!(!agreement.distinct(agreement.cell("1pl").unwrap(), stress));
    assert!(agreement.distinct(agreement.cell("2sg").unwrap(), stress));
    assert_eq!(agreement.stage, AgreementStage::Clitic);
    // A clitic the law passes by records nothing.
    let untouched = agreement.cell("2sg").unwrap();
    assert_eq!(
        (untouched.form.ipa(), untouched.history.len()),
        ("tu".into(), 1)
    );
}

#[test]
fn affixes_join_the_verb_outside_its_marking_and_take_harmony() {
    let mut v = variety(WordOrder::VSO);
    set_pronouns(&mut v, &PRONOUNS);
    set_pronouns(&mut v, &[("1sg", "ka")]);
    evolve(&mut v, 1, &forced());
    evolve(&mut v, 9, &forced());
    assert_eq!(v.grammar.agreement().side, Side::Suffix);
    let stress = v.stress();
    let affix = v.grammar.agreement().cell("1sg").unwrap().form.clone();
    let verb = Form::from_ipa("putum").unwrap();
    let plain = v
        .grammar
        .affix_agreement(&verb, &affix, &v.morphology, stress);
    assert_eq!(plain.ipa(), "putumka");
    v.grammar.harmony = Some(crate::harmony::Feature::Backness);
    let agreed = v
        .grammar
        .affix_agreement(&verb, &affix, &v.morphology, stress);
    assert_eq!(agreed.ipa(), "putumkɑ");
    assert_eq!(v.grammar.agreement().cell("1sg").unwrap().form, affix);
    let mut prefixing = variety(WordOrder::SVO);
    set_pronouns(&mut prefixing, &PRONOUNS);
    set_pronouns(&mut prefixing, &[("1sg", "ka")]);
    evolve(&mut prefixing, 1, &forced());
    prefixing.grammar.harmony = Some(crate::harmony::Feature::Backness);
    let joined = prefixing
        .grammar
        .affix_agreement(&verb, &affix, &prefixing.morphology, stress);
    assert_eq!(joined.ipa(), "kɑputum");
}

#[test]
fn daughters_inherit_agreement_and_its_draws_shift_no_other_grammar() {
    let mut v = variety(WordOrder::SVO);
    set_pronouns(&mut v, &PRONOUNS);
    let mut quiet = v.clone();
    evolve(&mut v, 1, &forced());
    evolve(&mut quiet, 1, &Params::static_society());
    assert_eq!(v.grammar.agreement().stage, AgreementStage::Clitic);
    assert_eq!(quiet.grammar.agreement().stage, AgreementStage::None);
    assert_eq!(v.lexicon, quiet.lexicon);
    assert_eq!(v.grammar.markers, quiet.grammar.markers);
    assert_eq!(v.grammar.events, quiet.grammar.events);
    let daughter = v.fork(0, 2);
    assert_eq!(daughter.grammar.agreement(), v.grammar.agreement());
    assert_eq!(daughter.grammar, v.grammar);
}

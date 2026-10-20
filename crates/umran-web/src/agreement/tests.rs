use super::*;
use umran_sim::Params;

fn forced() -> Params {
    Params {
        agreement_rate: 1.0,
        agreement_fusion_rate: 1.0,
        pro_drop_rate: 1.0,
        object_agreement_rate: 1.0,
        object_fusion_rate: 1.0,
        object_drop_rate: 1.0,
        ..Params::static_society()
    }
}

fn world(order: WordOrder, object: GrammarChoice) -> World {
    let mut world = tests::grammar_world(object, order);
    let v = &mut world.varieties[0];
    for (cell, ipa) in [
        ("1sg", "kami"),
        ("1pl", "miko"),
        ("1pl-inclusive", "miko"),
        ("1pl-exclusive", "nata"),
        ("2sg", "tunu"),
        ("2pl", "sata"),
        ("3sg", "pano"),
        ("3pl", "lupi"),
    ] {
        let Some(word) = v
            .lexicon
            .word_for(umran_sim::concepts::by_id(cell).unwrap())
        else {
            continue;
        };
        let id = word.id;
        v.lexicon.get_mut(id).form = Form::from_ipa(ipa).unwrap();
    }
    world
}

fn evolve(world: &mut World, generation: u32, params: &Params) {
    let v = &mut world.varieties[0];
    let stress = v.stress();
    v.grammar.evolve(
        7,
        0,
        generation,
        &mut v.lexicon,
        &v.morphology,
        stress,
        params,
    );
    world.generation = generation;
}

#[test]
fn clitics_replace_pronouns_in_all_orders_without_doubling_nouns() {
    for (order, gloss, second, ipa) in [
        (
            WordOrder::SOV,
            "1SG=2SG.OBJ=see-PAST",
            "2SG=1SG.OBJ=see-PAST",
            "ka=tu=pata",
        ),
        (
            WordOrder::SVO,
            "1SG=see-PAST=2SG.OBJ",
            "2SG=see-PAST=1SG.OBJ",
            "ka=pata=nu",
        ),
        (
            WordOrder::VSO,
            "see-PAST=1SG=2SG.OBJ",
            "see-PAST=2SG=1SG.OBJ",
            "pata=mi=nu",
        ),
    ] {
        let mut world = world(order, GrammarChoice::None);
        evolve(&mut world, 1, &forced());
        let v = &world.varieties[0];
        let sample = grammar_sample(v, 1).unwrap();
        let pronoun = sample.pronoun.unwrap();
        assert_eq!(pronoun.first.gloss, [gloss]);
        assert_eq!(pronoun.second.gloss, [second]);
        assert_eq!(pronoun.first.text.split_whitespace().count(), 1);
        assert_eq!(pronoun.first.text.matches('=').count(), 2);
        let noun = sample.future;
        assert!(!noun.text.contains('='));
        assert!(noun.gloss.iter().all(|g| !g.contains("3SG")));
        assert_eq!(noun.gloss.len(), 3);
        let subject = v.grammar.agreement().cell("1sg").unwrap();
        let object = v.grammar.object_agreement().cell("2sg").unwrap();
        let token = agree_token(
            v,
            &Form::from_ipa("pata").unwrap(),
            "see-PAST",
            Some((subject.cell, &subject.form)),
            Some((object.cell, &object.form)),
        );
        assert_eq!(token.ipa.replace('ˈ', ""), ipa);
        assert_eq!(token.gloss, gloss);
    }
}

#[test]
fn affixes_double_nouns_but_each_pronoun_stays_until_its_own_drop() {
    for (order, verb_gloss, noun_gloss) in [
        (
            WordOrder::SOV,
            "1SG-2SG.OBJ-see-PAST",
            "3PL-3SG.OBJ-see-FUT",
        ),
        (
            WordOrder::SVO,
            "1SG-see-PAST-2SG.OBJ",
            "3PL-see-FUT-3SG.OBJ",
        ),
        (
            WordOrder::VSO,
            "see-PAST-1SG-2SG.OBJ",
            "see-FUT-3PL-3SG.OBJ",
        ),
    ] {
        let mut world = world(order, GrammarChoice::None);
        evolve(&mut world, 1, &forced());
        evolve(&mut world, 9, &forced());
        let view = grammar_view(&world, 0);
        assert_eq!(view.agreement.stage, AgreementStage::Affix);
        assert_eq!(view.object_agreement.stage, AgreementStage::Affix);
        let sample = view.sample.unwrap();
        let pronoun = sample.pronoun.unwrap();
        assert_eq!(pronoun.first.gloss.len(), 3);
        assert!(pronoun.first.gloss.iter().any(|g| g == "1SG"));
        assert!(pronoun.first.gloss.iter().any(|g| g == "2SG"));
        assert!(pronoun.first.gloss.iter().any(|g| g == verb_gloss));
        assert!(sample.future.gloss.iter().any(|g| g == noun_gloss));
        assert_eq!(sample.future.gloss.len(), 3);
        assert!(!pronoun.first.text.contains('='));
        for object_first in [false, true] {
            let mut dropped = world.clone();
            evolve(
                &mut dropped,
                10,
                &Params {
                    pro_drop_rate: if object_first { 0.0 } else { 1.0 },
                    object_drop_rate: if object_first { 1.0 } else { 0.0 },
                    ..forced()
                },
            );
            let sample = grammar_sample(&dropped.varieties[0], 10).unwrap();
            let pronoun = sample.pronoun.unwrap();
            assert_eq!(pronoun.first.gloss.len(), 2);
            assert!(
                pronoun
                    .first
                    .gloss
                    .iter()
                    .any(|g| g == if object_first { "1SG" } else { "2SG" })
            );
            assert!(sample.future.gloss.iter().any(|g| g == noun_gloss));
            evolve(&mut dropped, 11, &forced());
            assert_eq!(
                grammar_pronoun_sample(&dropped.varieties[0], 11)
                    .unwrap()
                    .first
                    .gloss,
                [verb_gloss]
            );
        }
    }
}

#[test]
fn mixed_stages_wrap_the_affixed_verb_in_the_other_roles_clitic() {
    for (order, subject_affix, object_affix) in [
        (
            WordOrder::SOV,
            "2SG.OBJ=1SG-see-PAST",
            "1SG=2SG.OBJ-see-PAST",
        ),
        (
            WordOrder::SVO,
            "1SG-see-PAST=2SG.OBJ",
            "1SG=see-PAST-2SG.OBJ",
        ),
        (
            WordOrder::VSO,
            "see-PAST-1SG=2SG.OBJ",
            "see-PAST-2SG.OBJ=1SG",
        ),
    ] {
        for (subject_fuses, expected) in [(true, subject_affix), (false, object_affix)] {
            let mut world = world(order, GrammarChoice::None);
            evolve(&mut world, 1, &forced());
            evolve(
                &mut world,
                9,
                &Params {
                    agreement_fusion_rate: if subject_fuses { 1.0 } else { 0.0 },
                    object_fusion_rate: if subject_fuses { 0.0 } else { 1.0 },
                    ..forced()
                },
            );
            let sample = grammar_sample(&world.varieties[0], 9).unwrap();
            let pronoun = sample.pronoun.unwrap();
            assert_eq!(pronoun.first.gloss.len(), 2);
            assert!(pronoun.first.gloss.iter().any(|g| g == expected));
            assert!(
                pronoun
                    .first
                    .gloss
                    .iter()
                    .any(|g| g == if subject_fuses { "1SG" } else { "2SG" })
            );
            assert_eq!(pronoun.first.text.matches('=').count(), 1);
            assert!(!sample.future.text.contains('='));
            assert!(
                sample
                    .future
                    .gloss
                    .iter()
                    .any(
                        |g| g.contains(if subject_fuses { "3PL-" } else { "3SG.OBJ" })
                            || (subject_fuses && g.ends_with("-3PL"))
                    )
            );
        }
    }
}

#[test]
fn object_views_preserve_birth_sources_and_annals_identify_both_roles() {
    let mut world = world(WordOrder::SOV, GrammarChoice::Suffix);
    let v = &mut world.varieties[0];
    let id = v
        .lexicon
        .word_for(umran_sim::concepts::by_id("1sg").unwrap())
        .unwrap()
        .id;
    for r in v
        .lexicon
        .get_mut(id)
        .paradigms
        .iter_mut()
        .filter(|p| p.category == Category::Object)
        .flat_map(|p| &mut p.realizations)
    {
        r.form = Some(Form::from_ipa("nami").unwrap());
    }
    evolve(&mut world, 1, &forced());
    let born = serde_json::to_value(grammar_view(&world, 0)).unwrap();
    assert_eq!(born["agreement"]["role"], "subject");
    let objects = &born["objectAgreement"];
    assert_eq!(objects["role"], "object");
    let cell = objects["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["cell"] == "1sg")
        .unwrap();
    assert_eq!(cell["source"]["ipa"], "ˈnami");
    let source = cell["source"].clone();
    world.varieties[0].lexicon.get_mut(id).paradigms.clear();
    let later = serde_json::to_value(grammar_view(&world, 0)).unwrap();
    assert_eq!(later["objectAgreement"]["cells"][0]["source"], source);
    // Inherited events at the fork generation belong to the parent only.
    world.varieties.push(world.varieties[0].fork(0, 1));
    evolve(&mut world, 9, &forced());
    evolve(&mut world, 10, &forced());
    let entries: Vec<_> = annals(&world)
        .into_iter()
        .filter(|a| a.id.starts_with("agreement:") || a.id.starts_with("object-agreement:"))
        .collect();
    assert_eq!(entries.len(), 6);
    for entry in entries {
        assert_eq!(entry.variety, Some(0));
        let event = serde_json::to_value(entry.grammar.unwrap()).unwrap();
        assert_eq!(event["event"], "agreement");
        let role = if entry.id.starts_with("object-") {
            "object"
        } else {
            "subject"
        };
        assert_eq!(event["role"], role);
        assert_eq!(event["cells"].as_array().unwrap().len(), 6);
        assert!(["clitics", "affixes", "pro-drop"].contains(&event["kind"].as_str().unwrap()));
    }
}

use super::*;
use serde_json::{Value, json};
use umran_sim::{CATALOG, LanguageDesign};

fn meaning() -> Value {
    json!({"id":"hearth", "gloss":"fire hearth", "field":"house", "class":"entity", "countable":true, "tier":"everyday", "from":{"base":"fire", "relation":"place"}})
}

fn point(bench: &Bench) -> String {
    serde_json::to_string(&bench.chronicle.end()).unwrap()
}

fn value(text: String) -> Value {
    serde_json::from_str(&text).unwrap()
}

#[test]
fn meaning_checks_have_every_status_and_match_field_ids() {
    let mut bench = Bench::new(7, "small").unwrap();
    let beginning = point(&bench);
    for (field, id) in crate::desk::FIELDS {
        assert_eq!(serde_json::to_value(field).unwrap(), id);
    }
    let accepted = value(
        bench
            .check_meaning(&beginning, &meaning().to_string())
            .unwrap(),
    );
    assert_eq!(accepted["status"], "ok");
    assert!(
        accepted["suggestions"]
            .as_array()
            .unwrap()
            .contains(&json!("fire"))
    );
    assert!(accepted["suggestions"].as_array().unwrap().len() <= 3);
    for (field, replacement, status) in [
        ("id", json!("With space"), "bad-id"),
        ("id", json!("smith"), "taken"),
        ("id", json!("oath"), "taken"),
        ("gloss", json!(""), "bad-gloss"),
        ("class", json!("function"), "bad-class"),
        (
            "from",
            json!({"base":"absent", "relation":"place"}),
            "unknown-base",
        ),
        (
            "from",
            json!({"base":"1sg", "relation":"place"}),
            "unknown-base",
        ),
        (
            "from",
            json!({"base":"fire", "relation":"absent"}),
            "bad-relation",
        ),
    ] {
        let mut candidate = meaning();
        candidate[field] = replacement;
        let check = value(
            bench
                .check_meaning(&beginning, &candidate.to_string())
                .unwrap(),
        );
        assert_eq!(check["status"], status);
        assert!(check["message"].as_str().is_some_and(|s| !s.is_empty()));
    }
    bench
        .act(&json!({"kind":"meaning", "meaning":meaning()}).to_string())
        .unwrap();
    assert_eq!(
        value(
            bench
                .check_meaning(&point(&bench), &meaning().to_string())
                .unwrap()
        )["status"],
        "taken"
    );
    assert_eq!(
        value(
            bench
                .check_meaning(&beginning, &meaning().to_string())
                .unwrap()
        )["status"],
        "ok"
    );
}

#[test]
fn custom_roots_listings_word_checks_history_and_annals() {
    let mut bench = Bench::new(7, "small").unwrap();
    let design = LanguageDesign::preset("familiar", 7).unwrap();
    let ipa = design
        .sounds
        .iter()
        .find(|s| CATALOG.get(CATALOG.id_by_ipa(&s.ipa).unwrap()).is_vowel())
        .unwrap()
        .ipa
        .clone();
    let roots = value(
        Bench::check_roots(
            &serde_json::to_string(&design).unwrap(),
            &format!("hearth /{ipa}/"),
            &json!([meaning()]).to_string(),
        )
        .unwrap(),
    );
    assert!(matches!(
        roots["roots"][0]["status"].as_str(),
        Some("ok" | "warned")
    ));
    bench
        .act(&json!({"kind":"meaning", "meaning":meaning()}).to_string())
        .unwrap();
    bench.act(&json!({"kind":"found", "naming":{"kind":"people"}, "design":roots["design"], "seed":7, "power":0.5, "openness":0.5}).to_string()).unwrap();
    let concept = bench.chronicle.latest().concepts.by_id("hearth").unwrap();
    assert_eq!(
        bench.chronicle.latest().varieties[0]
            .lexicon
            .word_for(concept)
            .unwrap()
            .form
            .ipa(),
        ipa
    );
    let listing = value(bench.meanings(0).unwrap());
    let custom = listing
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "hearth")
        .unwrap();
    assert_eq!(custom["custom"], true);
    assert_eq!(custom["tier"], "everyday");
    assert!(
        value(Bench::concepts().unwrap())
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["custom"] == false)
    );
    let reading = point(&bench);
    let current = value(bench.check_word(&reading, 0, "hearth", &ipa).unwrap());
    assert_eq!(current["status"], "ok");
    assert!(current["spelled"].is_string());
    assert_eq!(current["current"]["form"], ipa);
    let (consonants, vowels) = bench.chronicle.latest().varieties[0].inventory();
    let outside = CATALOG
        .segments
        .iter()
        .find(|p| {
            !consonants
                .iter()
                .chain(&vowels)
                .any(|id| CATALOG.get(*id).ipa() == p.ipa())
        })
        .unwrap()
        .ipa();
    for (concept, form, status) in [
        ("absent", "a".into(), "unknown-meaning"),
        ("hearth", "a˥".into(), "bad-ipa"),
        ("hearth", "p".into(), "no-vowel"),
        ("hearth", format!("{outside}{ipa}"), "outside-inventory"),
        ("1sg", "a".into(), "pronoun"),
    ] {
        assert_eq!(
            value(bench.check_word(&reading, 0, concept, &form).unwrap())["status"],
            status
        );
    }
    assert_eq!(
        value(
            bench
                .check_word(&reading, usize::MAX, "hearth", &ipa)
                .unwrap()
        )["status"],
        "dead"
    );
    bench
        .act(
            &json!({"kind":"word", "variety":0, "concept":"hearth", "ipa":ipa.repeat(2)})
                .to_string(),
        )
        .unwrap();
    let word = value(bench.word(0, 0, "hearth").unwrap());
    let authored = word["variants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["origin"]["kind"] == "authored")
        .unwrap();
    assert!(
        authored["history"]
            .to_string()
            .contains("Given by the author in year 0")
    );
    let rows = value(bench.lexicon(0, 0).unwrap());
    assert_eq!(
        rows.as_array()
            .unwrap()
            .iter()
            .find(|r| r["concept"] == "hearth")
            .unwrap()["origin"]["kind"],
        "authored"
    );
    let annals = serde_json::to_value(crate::annals::annals(bench.chronicle.latest())).unwrap();
    for kind in ["meaning-defined", "word-authored"] {
        let entries: Vec<_> = annals
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["kind"] == kind)
            .collect();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0]["decision"].is_null());
    }
    let saved = bench.save().unwrap();
    let mut replay = Bench::load(&saved).unwrap();
    assert_eq!(
        bench.word(0, 0, "hearth").unwrap(),
        replay.word(0, 0, "hearth").unwrap()
    );
    bench.act(r#"{"kind":"run","generations":1}"#).unwrap();
    let mut later = meaning();
    later["id"] = json!("vow");
    later["gloss"] = json!("vow");
    later["class"] = json!("event");
    later["from"] = Value::Null;
    bench
        .act(&json!({"kind":"meaning", "meaning":later}).to_string())
        .unwrap();
    assert!(
        value(bench.meanings(0).unwrap())
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["id"] != "vow")
    );
    assert!(
        value(bench.meanings(1).unwrap())
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "vow")
    );
}

#[test]
fn authored_annal_keeps_the_given_form_after_same_year_sound_change() {
    let mut world = umran_sim::World::solo(
        7,
        &umran_sim::SoundProfile::base(),
        umran_sim::Params::static_society(),
    );
    let concept = world.concepts.by_id("water").unwrap();
    let form = world.varieties[0]
        .lexicon
        .word_for(concept)
        .unwrap()
        .form
        .clone();
    let word = world
        .give_word(
            0,
            &Root {
                concept: "water".into(),
                ipa: form.ipa(),
            },
        )
        .unwrap();
    // Reconstruct the log state left by a same-year law; the annal must read
    // the initial attestation, not the end-of-generation form.
    let lexeme = world.varieties[0].lexicon.get_mut(word);
    lexeme.log.push(umran_sim::Entry {
        generation: 0,
        event: umran_sim::Event::SoundLaw {
            law: "spirantization".into(),
            before: form.clone(),
        },
    });
    lexeme.form = umran_sim::Form::from_ipa("tatata").unwrap();
    let annals = serde_json::to_value(crate::annals::annals(&world)).unwrap();
    let annal = annals
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["kind"] == "word-authored")
        .unwrap();
    assert!(
        annal["text"]
            .as_str()
            .unwrap()
            .contains(&format!("/{}/", form.ipa()))
    );
}

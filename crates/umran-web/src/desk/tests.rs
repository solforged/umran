use super::*;
use serde_json::{Value, json};
use umran_sim::grammar::{Category, Grammar, Paradigm, Realization};
use umran_sim::{Entry, Event, LanguageDesign, LoanCause, Params, ReadingRef, SoundProfile};

fn form(ipa: &str) -> Form {
    Form::from_ipa(ipa).unwrap()
}

fn world() -> World {
    World::solo(5, &SoundProfile::base(), Params::static_society())
}

fn pair() -> World {
    let mut world = world();
    for id in SPECIMEN {
        let concept = by_id(id).unwrap();
        let word = world.varieties[0].lexicon.word_for(concept).unwrap().id;
        let word = world.varieties[0].lexicon.get_mut(word);
        word.form = form("papa");
        word.paradigms.clear();
    }
    world.varieties.push(world.varieties[0].fork(0, 0));
    world
}

fn loan(world: &mut World, into: usize, from: usize, meaning: &str) {
    let concept = by_id(meaning).unwrap();
    let donor = world.varieties[from].lexicon.word_for(concept).unwrap();
    let (source, heard) = (donor.id, donor.form.clone());
    let lexicon = &mut world.varieties[into].lexicon;
    let id = lexicon.coin(
        heard.clone(),
        Origin::Borrowed {
            from,
            source,
            cause: LoanCause::Unrecorded,
        },
        concept,
        world.generation,
    );
    lexicon.get_mut(id).log.push(Entry {
        generation: world.generation,
        event: Event::Borrowed {
            from,
            source: heard,
            cause: LoanCause::Unrecorded,
        },
    });
    lexicon.slot_mut(concept).variants.clear();
    lexicon.slot_mut(concept).introduce(id, 1.0);
}

fn keys(value: &Value, expected: &[&str]) {
    let actual: std::collections::BTreeSet<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(actual, expected.iter().copied().collect());
}

#[test]
fn blind_comparison_ignores_a_regular_looking_loan_and_lineage() {
    let mut world = pair();
    let before = serde_json::to_value(correspondences(&world, 0, 1, "specimen").unwrap()).unwrap();
    assert!(
        truth(&world, 0, 1, "specimen")
            .unwrap()
            .rows
            .iter()
            .all(|r| r.same_root)
    );
    loan(&mut world, 1, 0, "water");
    let after = serde_json::to_value(correspondences(&world, 0, 1, "specimen").unwrap()).unwrap();
    assert_eq!(
        before, after,
        "hidden loan records cannot change the blind read"
    );
    assert!(
        after["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["cognate"] == true)
    );
    let record = truth(&world, 0, 1, "specimen").unwrap();
    let water = record.rows.iter().find(|r| r.concept == "water").unwrap();
    assert!(!water.same_root, "a loan is not inherited from its donor");
    let borrowed = water.borrowed.as_ref().unwrap();
    assert_eq!((borrowed.into, borrowed.from), (1, 0));
    assert_eq!(record.ancestor, Some(0));
    assert_eq!(record.split_at, Some(0));
    world.varieties[1].parent = None;
    assert_eq!(
        after,
        serde_json::to_value(correspondences(&world, 0, 1, "specimen").unwrap()).unwrap()
    );
    assert_eq!(truth(&world, 0, 1, "specimen").unwrap().ancestor, None);
}

#[test]
fn desk_json_shapes_and_support_count_rows_not_positions() {
    let mut world = pair();
    loan(&mut world, 1, 0, "water");
    let compared =
        serde_json::to_value(correspondences(&world, 0, 1, "specimen").unwrap()).unwrap();
    keys(
        &compared,
        &[
            "a",
            "b",
            "generation",
            "meanings",
            "rows",
            "regular",
            "grammar",
        ],
    );
    for row in compared["rows"].as_array().unwrap() {
        keys(
            row,
            &[
                "concept",
                "gloss",
                "a",
                "b",
                "alignment",
                "regularity",
                "cognate",
            ],
        );
        keys(&row["a"], &["spelled", "ipa"]);
        keys(&row["b"], &["spelled", "ipa"]);
        assert!((0.0..=1.0).contains(&row["regularity"].as_f64().unwrap()));
        for segment in row["alignment"].as_array().unwrap() {
            keys(segment, &["a", "b"]);
        }
    }
    assert_eq!(compared["regular"].as_array().unwrap().len(), 2);
    for regular in compared["regular"].as_array().unwrap() {
        keys(regular, &["pair", "support"]);
        assert_eq!(
            regular["support"], 6,
            "p and a both repeat twice in every row"
        );
    }
    let truth = serde_json::to_value(truth(&world, 0, 1, "specimen").unwrap()).unwrap();
    keys(&truth, &["rows", "ancestor", "splitAt"]);
    for row in truth["rows"].as_array().unwrap() {
        keys(row, &["concept", "sameRoot", "borrowed"]);
    }
    keys(&truth["rows"][0]["borrowed"], &["into", "from", "cause"]);
    assert_eq!(
        truth["rows"][0]["borrowed"]["cause"],
        json!({"kind": "unrecorded"})
    );
}

#[test]
fn unrelated_length_mismatches_keep_gaps_but_not_cognate_votes() {
    let mut world = pair();
    world.varieties[1].parent = None;
    for (index, variety) in world.varieties.iter_mut().enumerate() {
        for meaning in SPECIMEN {
            let id = variety
                .lexicon
                .word_for(by_id(meaning).unwrap())
                .unwrap()
                .id;
            variety.lexicon.get_mut(id).form = form(match (index, meaning) {
                (0, "fire") => "ka",
                (0, _) => "ta",
                _ => "tatata",
            });
        }
    }
    let view = correspondences(&world, 0, 1, "specimen").unwrap();
    let method = compare::compare_varieties(
        &world.varieties[0],
        &world.varieties[1],
        &concepts("specimen").unwrap(),
    );
    assert!(
        view.regular
            .iter()
            .all(|r| r.pair.a.is_some() && r.pair.b.is_some())
    );
    let fire = view.rows.iter().find(|r| r.concept == "fire").unwrap();
    assert!(
        fire.alignment
            .iter()
            .any(|p| p.a.is_none() || p.b.is_none())
    );
    assert_eq!(fire.regularity, 0.5);
    assert!(!fire.cognate);
    for (shown, inferred) in view.rows.iter().zip(&method.rows) {
        assert_eq!(shown.concept, inferred.concept.id);
        assert_eq!(shown.regularity, inferred.regularity);
        assert_eq!(
            shown.cognate, inferred.cognate,
            "the facade must not invent its own threshold"
        );
    }
    assert!(
        truth(&world, 0, 1, "specimen")
            .unwrap()
            .rows
            .iter()
            .all(|r| !r.same_root)
    );
}

#[test]
fn alignment_gaps_and_missing_meanings_match_on_both_reads() {
    let mut world = pair();
    let night = by_id("night").unwrap();
    let id = world.varieties[1].lexicon.word_for(night).unwrap().id;
    world.varieties[1].lexicon.get_mut(id).form = form("pap");
    world.varieties[1]
        .lexicon
        .slot_mut(by_id("fire").unwrap())
        .variants
        .clear();
    let compared = correspondences(&world, 0, 1, "specimen").unwrap();
    let recorded = truth(&world, 0, 1, "specimen").unwrap();
    assert_eq!(
        compared.rows.iter().map(|r| r.concept).collect::<Vec<_>>(),
        recorded.rows.iter().map(|r| r.concept).collect::<Vec<_>>()
    );
    let night = compared.rows.iter().find(|r| r.concept == "night").unwrap();
    assert!(
        night
            .alignment
            .iter()
            .any(|s| s.a == Some("a") && s.b.is_none())
    );
    assert!(compared.rows.iter().all(|r| r.concept != "fire"));
}

#[test]
fn grammar_observations_are_the_methods_current_contrasts() {
    let mut world = pair();
    for variety in &mut world.varieties {
        variety.grammar = Grammar::default();
        for id in ["stone", "eye", "hand"] {
            let concept = by_id(id).unwrap();
            let id = variety.lexicon.word_for(concept).unwrap().id;
            variety.lexicon.get_mut(id).paradigms.push(Paradigm {
                category: Category::Plural,
                realizations: vec![Realization {
                    marker: 0,
                    form: Some(form("papapi")),
                    edge: 4,
                    share: 1.0,
                    born: 0,
                    retired: None,
                    history: Vec::new(),
                }],
            });
        }
    }
    // Bound forms carry their audible contrast themselves; marker ancestry is
    // deliberately not supplied to the method or this display fixture.
    let compared = correspondences(&world, 0, 1, "specimen").unwrap();
    let method = compare::compare_varieties(
        &world.varieties[0],
        &world.varieties[1],
        &concepts("specimen").unwrap(),
    );
    assert!(method.grammar.iter().any(|g| g.regular));
    assert!(!compared.grammar.is_empty());
    for observation in &compared.grammar {
        assert_eq!(observation.category, Category::Plural.id());
        assert_eq!(observation.a, observation.b);
        assert!(observation.regular);
        keys(
            &serde_json::to_value(observation).unwrap(),
            &["category", "a", "b", "regular"],
        );
    }
}

#[test]
fn meaning_sets_cover_catalog_fields_and_reject_unknown_inputs() {
    let world = pair();
    let sets = meaning_sets();
    assert_eq!((sets[0].id, sets[1].id), ("core", "specimen"));
    assert_eq!(
        concepts("core").unwrap().len(),
        CONCEPTS.iter().filter(|c| c.stability.is_some()).count()
    );
    assert_eq!(
        concepts("specimen")
            .unwrap()
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>(),
        SPECIMEN
    );
    let mut covered = HashSet::new();
    for set in &sets[2..] {
        assert!(!set.label.is_empty());
        for concept in concepts(set.id).unwrap() {
            assert!(covered.insert(concept.id));
        }
    }
    assert_eq!(covered.len(), CONCEPTS.len());
    assert!(correspondences(&world, 0, 99, "core").is_err());
    assert!(truth(&world, 99, 0, "core").is_err());
    assert!(correspondences(&world, 0, 1, "not-a-set").is_err());
    assert!(truth(&world, 0, 1, "not-a-set").is_err());
    for set in sets {
        keys(&serde_json::to_value(set).unwrap(), &["id", "label"]);
    }
}

#[test]
fn nearest_ancestor_split_and_inherited_loan_keep_their_recorded_owner() {
    let mut world = pair();
    world.generation = 2;
    loan(&mut world, 1, 0, "water");
    world.varieties.push(world.varieties[1].fork(1, 4));
    world.varieties.push(world.varieties[1].fork(1, 7));
    world.varieties.push(world.varieties[2].fork(2, 8));
    assert_eq!(ancestor(&world, 4, 3), (Some(1), Some(4)));
    assert_eq!(ancestor(&world, 3, 4), (Some(1), Some(4)));
    assert_eq!(ancestor(&world, 2, 4), (Some(2), Some(8)));
    assert_eq!(ancestor(&world, 4, 4), (Some(4), None));
    let truth = truth(&world, 4, 3, "specimen").unwrap();
    let water = truth.rows.iter().find(|r| r.concept == "water").unwrap();
    assert!(
        water.same_root,
        "both inherited the same loan from their common ancestor"
    );
    assert_eq!(water.borrowed.as_ref().unwrap().into, 1);
}

#[test]
fn branched_static_world_counts_only_new_laws_loans_and_descendants() {
    let mut shared = pair();
    shared.varieties[0].laws.push((0, "shared-law".into()));
    loan(&mut shared, 0, 1, "night");
    let mut left = shared.clone();
    let mut right = shared.clone();
    left.generation = 3;
    right.generation = 3;
    // Supplied ledgers isolate facade subtraction from law eligibility. Two
    // applications of the same law remain two occurrences but one exclusive id.
    left.varieties[0].laws.extend([
        (0, "left-law".into()),
        (3, "left-law".into()),
        (3, "both-law".into()),
    ]);
    right.varieties[0]
        .laws
        .extend([(0, "right-law".into()), (3, "both-law".into())]);
    loan(&mut left, 0, 1, "water");
    loan(&mut left, 0, 1, "fire");
    loan(&mut right, 0, 1, "hand");
    let id = right.varieties[0]
        .lexicon
        .word_for(by_id("eye").unwrap())
        .unwrap()
        .id;
    right.varieties[0].lexicon.get_mut(id).form = form("tata");
    left.varieties.push(left.varieties[0].fork(0, 3));
    left.varieties.push(left.varieties[2].fork(2, 3));
    right.varieties.push(right.varieties[1].fork(1, 3));
    let left_sides = speech_sides(&left, &shared, 2);
    let right_sides = speech_sides(&right, &shared, 2);
    let deltas: Vec<_> = left_sides
        .into_iter()
        .zip(right_sides)
        .enumerate()
        .map(|(id, (a, b))| language_delta(id, a, b))
        .collect();
    let delta = &deltas[0];
    assert_eq!(delta.only_left, ["left-law"]);
    assert_eq!(delta.only_right, ["right-law"]);
    assert_eq!(delta.left.laws.len(), 3);
    assert_eq!((delta.left.descendants, delta.right.descendants), (2, 1));
    assert_eq!(
        (deltas[1].left.descendants, deltas[1].right.descendants),
        (0, 1)
    );
    assert_eq!(
        (delta.loans_left[0].donor, delta.loans_left[0].count),
        (1, 2)
    );
    assert_eq!(
        (delta.loans_right[0].donor, delta.loans_right[0].count),
        (1, 1)
    );
    assert_eq!(
        delta
            .changed_words
            .iter()
            .map(|w| w.concept)
            .collect::<Vec<_>>(),
        ["eye"]
    );
    let json = serde_json::to_value(delta).unwrap();
    keys(
        &json,
        &[
            "variety",
            "left",
            "right",
            "onlyLeft",
            "onlyRight",
            "changedWords",
            "loansLeft",
            "loansRight",
        ],
    );
    keys(&json["left"], &["alive", "descendants", "laws", "specimen"]);
    keys(&json["left"]["laws"][0], &["id", "label", "generation"]);
    keys(
        &json["left"]["specimen"][0],
        &[
            "concept", "gloss", "spelled", "ipa", "was", "stress", "wasIpa",
        ],
    );
    keys(&json["loansLeft"][0], &["donor", "count"]);
    keys(
        &json["changedWords"][0],
        &["concept", "gloss", "left", "right"],
    );
    keys(&json["changedWords"][0]["left"], &["spelled", "ipa"]);
}

fn found() -> String {
    json!({
        "kind": "found", "naming": {"kind": "people"},
        "design": LanguageDesign::preset("familiar", 1).unwrap(),
        "seed": 4, "power": 0.5, "openness": 0.5
    })
    .to_string()
}

#[test]
fn telling_delta_uses_exact_divergence_and_existing_decision_views() {
    let mut bench = Bench::new(5, "small").unwrap();
    bench.act(&found()).unwrap();
    let point = bench.chronicle.end();
    let choices: Value =
        serde_json::from_str(&bench.law_choices(&to_json(&point).unwrap(), 0).unwrap()).unwrap();
    let law = choices
        .as_array()
        .unwrap()
        .iter()
        .find(|choice| choice["words"].as_u64().unwrap() > 0)
        .unwrap();
    bench
        .act(&json!({"kind": "law", "variety": 0, "law": law["id"]}).to_string())
        .unwrap();
    let left = bench.chronicle.active();
    bench
        .act_at(
            &to_json(&ReadingRef {
                telling: left,
                point,
            })
            .unwrap(),
            bench.mutation,
            &found(),
        )
        .unwrap();
    let right = bench.chronicle.active();
    let saved = bench.save().unwrap();
    let delta: Value = serde_json::from_str(&bench.speech_delta(left, right, 0).unwrap()).unwrap();
    keys(&delta, &["generation", "diverged", "decision", "languages"]);
    assert_eq!(delta["diverged"], 0);
    assert_eq!(
        delta["languages"].as_array().unwrap().len(),
        1,
        "post-divergence numeric ids are not paired"
    );
    assert_eq!(delta["languages"][0]["variety"], 0);
    assert_eq!(delta["languages"][0]["onlyLeft"], json!([law["id"]]));
    assert_eq!(delta["languages"][0]["onlyRight"], json!([]));
    // Each side branched with its own decision at the split; the side being
    // read names its own, and reading the other way round names the other's.
    let left_decisions: Value =
        serde_json::from_str(&bench.read(left, "").unwrap().decisions().unwrap()).unwrap();
    assert_eq!(
        delta["decision"],
        left_decisions.as_array().unwrap().last().unwrap().clone()
    );
    assert_eq!(delta["decision"]["index"], point.action);
    assert_eq!(delta["decision"]["kind"], "law");
    let mirrored: Value =
        serde_json::from_str(&bench.speech_delta(right, left, 0).unwrap()).unwrap();
    let right_decisions: Value =
        serde_json::from_str(&bench.read(right, "").unwrap().decisions().unwrap()).unwrap();
    assert_eq!(
        mirrored["decision"],
        right_decisions.as_array().unwrap().last().unwrap().clone()
    );
    assert_eq!(mirrored["decision"]["kind"], "found");
    assert_eq!(bench.save().unwrap(), saved, "reads cannot alter a telling");
    assert!(bench.speech_delta(left, right, 1).is_err());
    assert!(bench.speech_delta(left, 999, 0).is_err());
    let identical: Value =
        serde_json::from_str(&bench.speech_delta(left, left, 0).unwrap()).unwrap();
    assert!(identical["decision"].is_null());
    assert_eq!(identical["languages"][0]["onlyLeft"], json!([]));
    assert_eq!(identical["languages"][0]["changedWords"], json!([]));
}

#[test]
fn a_law_decided_mid_run_names_the_divergence_from_either_side() {
    let mut bench = Bench::new(5, "small").unwrap();
    bench.act(&found()).unwrap();
    bench.act(r#"{"kind":"run","generations":40}"#).unwrap();
    let parent = bench.chronicle.active();
    let point = umran_sim::HistoryPoint {
        action: 1,
        offset: 37,
    };
    let choices: Value =
        serde_json::from_str(&bench.law_choices(&to_json(&point).unwrap(), 0).unwrap()).unwrap();
    let law = choices
        .as_array()
        .unwrap()
        .iter()
        .find(|choice| choice["words"].as_u64().unwrap() > 0)
        .unwrap();
    bench
        .act_at(
            &to_json(&ReadingRef {
                telling: parent,
                point,
            })
            .unwrap(),
            bench.mutation,
            &json!({"kind": "law", "variety": 0, "law": law["id"]}).to_string(),
        )
        .unwrap();
    let child = bench.chronicle.active();
    for (left, right) in [(child, parent), (parent, child)] {
        let delta: Value =
            serde_json::from_str(&bench.speech_delta(left, right, 37).unwrap()).unwrap();
        assert_eq!(delta["diverged"], 37);
        assert_eq!(delta["decision"]["kind"], "law", "{left:?} vs {right:?}");
        assert_eq!(delta["decision"]["generation"], 37);
    }
}

#[test]
fn reading_before_divergence_and_then_the_tip_keeps_decisions_at_their_year() {
    let mut bench = Bench::new(5, "small").unwrap();
    bench.act(&found()).unwrap();
    bench.act(r#"{"kind":"run","generations":2}"#).unwrap();
    let point = bench.chronicle.end();
    bench.act(&found()).unwrap();
    let left = bench.chronicle.active();
    bench
        .act_at(
            &to_json(&ReadingRef {
                telling: left,
                point,
            })
            .unwrap(),
            bench.mutation,
            &found(),
        )
        .unwrap();
    let right = bench.chronicle.active();
    let before: Value = serde_json::from_str(&bench.speech_delta(left, right, 0).unwrap()).unwrap();
    assert_eq!(before["diverged"], 2);
    assert!(before["decision"].is_null());
    for language in before["languages"].as_array().unwrap() {
        assert_eq!(language["onlyLeft"], json!([]));
        assert_eq!(language["onlyRight"], json!([]));
        assert_eq!(language["changedWords"], json!([]));
        assert_eq!(language["left"]["descendants"], 0);
    }
    let at_tip: Value = serde_json::from_str(&bench.speech_delta(left, right, 2).unwrap()).unwrap();
    assert_eq!(at_tip["decision"]["generation"], 2);
    assert_eq!(at_tip["decision"]["index"], point.action + 1);
    assert_eq!(at_tip["decision"]["kind"], "found");
}

#[test]
fn absent_specimen_forms_are_null_and_descendant_loans_are_not_duplicated() {
    let shared = pair();
    let mut left = shared.clone();
    let mut right = shared.clone();
    left.varieties.push(left.varieties[0].fork(0, 0));
    loan(&mut left, 2, 1, "water");
    right.varieties[0]
        .lexicon
        .slot_mut(by_id("water").unwrap())
        .variants
        .clear();
    let mut a = speech_sides(&left, &shared, 2);
    let mut b = speech_sides(&right, &shared, 2);
    let delta = language_delta(0, a.remove(0), b.remove(0));
    assert_eq!(delta.left.descendants, 1);
    assert!(delta.loans_left.is_empty());
    assert_eq!(delta.changed_words.len(), 1);
    assert_eq!(delta.changed_words[0].concept, "water");
    assert!(delta.changed_words[0].left.is_some());
    assert!(delta.changed_words[0].right.is_none());
    let json = serde_json::to_value(delta).unwrap();
    assert!(json["changedWords"][0]["right"].is_null());
}

#[test]
fn fixed_readings_do_not_leak_later_desk_evidence() {
    let mut bench = Bench::new(5, "small").unwrap();
    bench.act(&found()).unwrap();
    let point = bench.chronicle.end();
    bench.act(&found()).unwrap();
    let mut reading = bench
        .read(bench.chronicle.active(), &to_json(&point).unwrap())
        .unwrap();
    assert!(reading.correspondences(0, 0, 1, "core").is_err());
    assert!(reading.truth(0, 0, 1, "core").is_err());
    let shown: Value =
        serde_json::from_str(&reading.correspondences(99, 0, 0, "specimen").unwrap()).unwrap();
    assert_eq!(shown["generation"], 0);
    assert_eq!(shown["meanings"], "specimen");
    let sets: Value = serde_json::from_str(&reading.meaning_sets().unwrap()).unwrap();
    assert_eq!(sets[0]["id"], "core");
}

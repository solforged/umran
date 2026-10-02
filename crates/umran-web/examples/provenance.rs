//! cargo run --release -p umran-web --example provenance
#[path = "support/sample.rs"]
mod sample;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use umran_sim::{Event, LoanCause, Origin};
use umran_web::Bench;

fn main() {
    let history = sample::sample();
    let world = history.latest();
    let mut bench = Bench::load(&serde_json::to_string(&history.recipe()).unwrap()).unwrap();
    let mut counts = BTreeMap::from([
        ("contact", 0),
        ("rule", 0),
        ("faith", 0),
        ("shift", 0),
        ("city", 0),
        ("coinage", 0),
        ("classical", 0),
        ("unrecorded", 0),
    ]);
    let mut seen = HashSet::new();
    for (v, variety) in world.varieties.iter().enumerate() {
        for word in &variety.lexicon.lexemes {
            let Origin::Borrowed { cause, .. } = word.origin else {
                continue;
            };
            if !seen.insert(world.root_of(v, word.id)) {
                continue;
            }
            let kind = match cause {
                LoanCause::Contact { .. } => "contact",
                LoanCause::Rule { .. } => "rule",
                LoanCause::Faith { .. } => "faith",
                LoanCause::Shift { .. } => "shift",
                LoanCause::City { .. } => "city",
                LoanCause::Coinage { .. } => "coinage",
                LoanCause::Classical { .. } => "classical",
                LoanCause::Unrecorded => "unrecorded",
            };
            *counts.get_mut(kind).unwrap() += 1;
        }
    }
    println!("SAMPLE seed 21, 4000 years; unique loan origins (including obsolete words)");
    for (kind, count) in counts {
        println!("{kind}: {count}");
    }
    for kind in ["faith", "shift", "contact"] {
        'trail: for (v, variety) in world.varieties.iter().enumerate() {
            for word in variety.lexicon.living() {
                if !matches!(word.origin, Origin::Borrowed { .. })
                    || !word
                        .log
                        .iter()
                        .any(|e| matches!(e.event, Event::SoundLaw { .. }))
                {
                    continue;
                }
                for concept in variety.lexicon.senses(word.id) {
                    let detail: Value =
                        serde_json::from_str(&bench.word(160, v, concept.id).unwrap()).unwrap();
                    let Some(variant) =
                        detail["variants"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|variant| {
                                variant["origin"]["cause"]["kind"] == kind
                                    && variant["ipa"] == word.form.ipa_stressed(variety.stress())
                            })
                    else {
                        continue;
                    };
                    let cause = &variant["origin"]["cause"];
                    println!(
                        "\nTRAIL variety {v} {} ({}) donor={} cause={} event={}",
                        variant["spelled"],
                        concept.id,
                        variant["origin"]["from"],
                        cause,
                        cause["event"]
                    );
                    for line in variant["history"].as_array().unwrap() {
                        println!(
                            "  year {}: {}",
                            line["generation"].as_u64().unwrap() * 25,
                            line["text"]
                        );
                    }
                    println!("  story: {}", bench.story(160, &serde_json::json!({"kind":"word", "variety":v, "concept":concept.id}).to_string()).unwrap());
                    break 'trail;
                }
            }
        }
    }
}

//! cargo run --release -p umran-web --example notation
#[path = "support/sample.rs"]
mod sample;
use serde_json::{Value, json};
use umran_sim::notation::parse;
use umran_web::Bench;

fn main() -> Result<(), String> {
    for source in [
        "[voiceless stop] > [fricative]",
        "[voiceless stop] > [voiced] / V _ V",
        "V > ∅ / _ #",
        "a > e / _ .. [front]",
        "k > tʃ / _ [front]",
        "s > [lengthen before] / _ C",
    ] {
        let report = parse("Example", source);
        assert!(report.law.is_some(), "{report:?}");
        println!("{:?}: {source}", report.rules[0].status);
    }
    let history = sample::sample();
    let mut bench =
        Bench::load(&serde_json::to_string(&history.recipe()).map_err(|e| e.to_string())?)?;
    bench.branch(48);
    for (title, source) in [
        (
            "Intervocalic voicing",
            "[voiceless stop] > [voiced] / V _ V",
        ),
        ("Final vowel loss", "V > ∅ / _ #"),
    ] {
        let law = parse(title, source).law.unwrap();
        let overview: Value =
            serde_json::from_str(&bench.overview(48)?).map_err(|e| e.to_string())?;
        let point = overview["point"].to_string();
        let mut chosen = None;
        for community in overview["communities"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["ended"].is_null())
        {
            let variety = community["variety"].as_u64().unwrap() as usize;
            let preview: Value = serde_json::from_str(&bench.law_preview(
                &point,
                variety,
                &json!({"authored": law}).to_string(),
            )?)
            .map_err(|e| e.to_string())?;
            if !preview["words"].as_array().unwrap().is_empty()
                || !preview["names"].as_array().unwrap().is_empty()
            {
                chosen = Some((variety, preview));
                break;
            }
        }
        let (variety, preview) =
            chosen.ok_or_else(|| format!("sample has no eligible language for {title}"))?;
        println!("\nYear 1200 — {title} — language {variety}");
        println!("meaning\tbefore\tafter");
        for row in preview["words"].as_array().unwrap() {
            println!(
                "{}\t{}\t{}",
                row["concept"].as_str().unwrap(),
                row["before"].as_str().unwrap(),
                row["after"].as_str().unwrap()
            );
        }
        for row in preview["names"].as_array().unwrap() {
            println!(
                "{} {}\t{}\t{}",
                row["kind"].as_str().unwrap(),
                row["subject"],
                row["before"].as_str().unwrap(),
                row["after"].as_str().unwrap()
            );
        }
        println!("Rules: {}", preview["rules"]);
        bench.act(&json!({"kind":"authored", "variety":variety, "law":law}).to_string())?;
        let after: Value = serde_json::from_str(&bench.overview(48)?).map_err(|e| e.to_string())?;
        let id = format!("authored:{}:48", umran_sim::notation::slug(title));
        let annal = after["annals"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| {
                a["laws"]
                    .as_array()
                    .is_some_and(|laws| laws.iter().any(|l| l == &id))
            })
            .unwrap();
        println!(
            "Annal: {} — {} (decision {})",
            annal["text"], annal["notes"], annal["decision"]
        );
    }
    Ok(())
}

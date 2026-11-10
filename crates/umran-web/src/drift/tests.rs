use super::*;
use serde_json::json;
use umran_sim::LanguageDesign;

fn found(preset: &str, region: usize) -> String {
    let design = LanguageDesign::preset(preset, 1).unwrap();
    json!({
        "kind": "found", "naming": {"kind": "people"}, "design": design,
        "seed": 3, "power": 0.5, "openness": 0.5, "region": region
    })
    .to_string()
}

/// Two peoples in trade and thirty generations, frozen at the last.
fn frozen() -> (Bench, Value) {
    let mut w = Bench::new(5, "medium").unwrap();
    let map = &w.chronicle.latest().map;
    let anchor = map
        .regions
        .iter()
        .enumerate()
        .filter(|(_, r)| r.terrain.is_land())
        .max_by_key(|&(r, _)| map.walking_row(r, 800.0).len())
        .unwrap()
        .0;
    let sites: Vec<usize> = serde_json::from_str(&w.founding_sites(anchor, 2).unwrap()).unwrap();
    w.act(&found("familiar", anchor)).unwrap();
    w.act(&found("polynesian", sites[1])).unwrap();
    w.act(r#"{"kind":"connect","a":0,"b":1,"intensity":0.6,"contact":"trade"}"#)
        .unwrap();
    w.act(r#"{"kind":"run","generations":30}"#).unwrap();
    let latest = w.latest();
    let file = serde_json::from_str(&w.frozen(latest).unwrap()).unwrap();
    (w, file)
}

fn drift(w: &Bench, file: &Value) -> Value {
    serde_json::from_str(&w.drift(&file.to_string()).unwrap()).unwrap()
}

#[test]
fn the_engine_that_froze_a_world_reads_it_the_same() {
    let (mut w, file) = frozen();
    let report = drift(&w, &file);
    assert_eq!(report["same"], true);
    assert_eq!(report["partsWays"], Value::Null);
    assert_eq!(report["was"], ENGINE_REVISION);
    assert_eq!(report["now"], ENGINE_REVISION);
    assert_eq!(report["generation"], 30);
    assert_eq!(report["peoples"]["changed"], json!([]));
    assert_eq!(report["languages"]["changed"], json!([]));
    assert_eq!(report["words"]["changed"], 0);
    assert_eq!(report["names"]["changed"], 0);
    assert!(report["words"]["total"].as_u64().unwrap() > 100);
    assert!(report["names"]["total"].as_u64().unwrap() > 0);

    // Another JSON tool may have rewritten the file: 1.0 as 1, keys reordered.
    fn rewritten(value: &mut Value) {
        match value {
            Value::Number(number) => {
                let float = number.as_f64().unwrap();
                if number.is_f64() && float.fract() == 0.0 && float.abs() < 1e15 {
                    *value = (float as i64).into();
                }
            }
            Value::Array(items) => items.iter_mut().for_each(rewritten),
            Value::Object(members) => members.values_mut().for_each(rewritten),
            _ => {}
        }
    }
    let mut passed_on = file.clone();
    rewritten(&mut passed_on);
    assert_ne!(
        passed_on, file,
        "the record has whole-number floats to rewrite"
    );
    assert_eq!(drift(&w, &passed_on)["same"], true);

    // The record is of its own point: years written since do not count.
    w.act(r#"{"kind":"run","generations":5}"#).unwrap();
    let later = drift(&w, &file);
    assert_eq!(later["same"], true);
    assert_eq!(later["generation"], 30);
}

#[test]
fn a_record_another_engine_wrote_says_what_reads_differently() {
    let (w, mut file) = frozen();
    let record = &mut file["frozen"];
    record["revision"] = (ENGINE_REVISION - 1).into();
    let people = record["communities"][1]["name"]
        .as_str()
        .unwrap()
        .to_owned();
    record["communities"][1]["name"] = "Elsewhere".into();
    let language = record["varieties"][0]["name"].as_str().unwrap().to_owned();
    record["varieties"][0]["name"] = "Otherwise".into();
    let row = record["varieties"][0]["lexicon"][0].clone();
    record["varieties"][0]["lexicon"][0]["spelled"] = "zzz".into();
    // A meaning the old engine had a word for and this one does not.
    let mut lost = row.clone();
    lost["concept"] = "no-such-meaning".into();
    lost["gloss"] = "a lost meaning".into();
    record["varieties"][0]["lexicon"]
        .as_array_mut()
        .unwrap()
        .push(lost);
    let annal = record["annals"].as_array().unwrap().len() / 2;
    let year = record["annals"][annal]["generation"].clone();
    record["annals"][annal]["text"] = "Something else happened.".into();
    let land = record["places"][0]["names"].as_array().unwrap().len() - 1;
    record["places"][0]["names"][land]["spelled"] = "Nowhere".into();

    let report = drift(&w, &file);
    assert_eq!(report["same"], false);
    assert_eq!(report["was"], ENGINE_REVISION - 1);
    assert_eq!(report["partsWays"], year);
    assert_eq!(
        report["peoples"]["changed"],
        json!([{ "of": "1", "was": "Elsewhere", "now": people }])
    );
    assert_eq!(
        report["languages"]["changed"],
        json!([{ "of": "0", "was": "Otherwise", "now": language }])
    );
    assert_eq!(report["words"]["changed"], 2);
    let examples = report["words"]["examples"].as_array().unwrap();
    assert_eq!(
        examples[0],
        json!({
            "of": format!("“{}” in Otherwise", row["gloss"].as_str().unwrap()),
            "was": format!("zzz /{}/", row["ipa"].as_str().unwrap()),
            "now": format!("{} /{}/", row["spelled"].as_str().unwrap(), row["ipa"].as_str().unwrap()),
        })
    );
    assert_eq!(examples[1]["of"], "“a lost meaning” in Otherwise");
    assert_eq!(examples[1]["now"], Value::Null);
    assert_eq!(report["names"]["changed"], 1);
    assert_eq!(report["names"]["examples"][0]["of"], "land");
    assert_eq!(report["names"]["examples"][0]["was"], "Nowhere");
}

#[test]
fn a_record_this_history_cannot_reach_is_refused_by_name() {
    let (w, mut file) = frozen();
    assert!(
        w.drift(&w.save().unwrap())
            .unwrap_err()
            .contains("no frozen record")
    );
    file["frozen"]["telling"] = 9.into();
    assert!(
        w.drift(&file.to_string())
            .unwrap_err()
            .contains("no longer has the point")
    );
    assert!(
        w.drift("not json")
            .unwrap_err()
            .starts_with("Not a frozen world")
    );
}

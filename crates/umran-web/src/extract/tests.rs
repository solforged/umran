use super::*;
use serde_json::{Value, json};
use umran_sim::{CONCEPTS, LanguageDesign};

fn found(preset: &str, region: usize) -> String {
    let design = LanguageDesign::preset(preset, 1).unwrap();
    json!({
        "kind": "found", "naming": {"kind": "people"}, "design": design,
        "seed": 3, "power": 0.5, "openness": 0.5, "region": region
    })
    .to_string()
}

/// Three languages: a founder, a daughter founded with its speech, and a
/// stranger in trade with the founder, after a run.
fn bench() -> Bench {
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
    let sites: Vec<usize> = serde_json::from_str(&w.founding_sites(anchor, 3).unwrap()).unwrap();
    w.act(&found("familiar", anchor)).unwrap();
    w.act(
        &json!({
            "kind": "found-related", "source": 0, "region": sites[1],
            "naming": {"kind": "people"}
        })
        .to_string(),
    )
    .unwrap();
    w.act(&found("polynesian", sites[2])).unwrap();
    w.act(r#"{"kind":"connect","a":0,"b":2,"intensity":0.6,"contact":"trade"}"#)
        .unwrap();
    w.act(r#"{"kind":"run","generations":30}"#).unwrap();
    w
}

fn table(w: &mut Bench, generation: u32, varieties: &[usize], concepts: &[&str]) -> Value {
    serde_json::from_str(
        &w.word_table(
            generation,
            &json!(varieties).to_string(),
            &json!(concepts).to_string(),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn word_table_shares_roots_by_descent_and_mints_fresh_ones_for_coinages() {
    let mut w = bench();
    let generation = w.latest();
    let all: Vec<&str> = CONCEPTS.iter().map(|c| c.id.as_ref()).collect();
    let table = table(&mut w, generation, &[0, 1, 2], &all);
    assert_eq!(table["generation"], generation);
    assert_eq!(table["varieties"].as_array().unwrap().len(), 3);
    assert_eq!(table["varieties"][1]["id"], 1);
    let roots = table["roots"].as_array().unwrap();
    let rows = table["rows"].as_array().unwrap();
    assert_eq!(rows.len(), CONCEPTS.len());
    let world = w.chronicle.latest();
    let mut shared = 0;
    let mut coined = 0;
    for row in rows {
        let cells = row["cells"].as_array().unwrap();
        assert_eq!(cells.len(), 3);
        let concept = world
            .concepts
            .by_id(row["concept"].as_str().unwrap())
            .unwrap();
        if let (Some(a), Some(b)) = (cells[0].as_object(), cells[1].as_object()) {
            // Root identity is the engine's own cognacy, not the origin label.
            assert_eq!(
                a["root"] == b["root"],
                world.cognate(0, 1, concept),
                "{}",
                row["concept"]
            );
            if a["root"] == b["root"] {
                let root = &roots[a["root"].as_u64().unwrap() as usize];
                assert_eq!(root["variety"], 0);
                assert_eq!(root["born"], 0);
                assert!(!root["spelled"].as_str().unwrap().is_empty());
                shared += 1;
            }
        }
        for (i, cell) in cells.iter().enumerate() {
            let Some(cell) = cell.as_object() else {
                continue;
            };
            assert_eq!(cell["authored"], false);
            assert!(cell["competitors"].as_u64().is_some());
            if cell["origin"]["kind"] != "coined" {
                continue;
            }
            let root = cell["root"].as_u64().unwrap() as usize;
            assert_eq!(roots[root]["variety"], i, "{}", row["concept"]);
            assert_eq!(roots[root]["born"], cell["origin"]["generation"]);
            assert!(
                cells
                    .iter()
                    .enumerate()
                    .all(|(j, other)| j == i || other["root"] != root),
                "{}",
                row["concept"]
            );
            coined += 1;
        }
    }
    assert!(shared > 20, "{shared} shared rows");
    assert!(coined > 0, "no coined word in {generation} years");
    // The stranger shares no root with the founder by descent.
    for row in rows {
        let cells = row["cells"].as_array().unwrap();
        if let (Some(a), Some(c)) = (cells[0].as_object(), cells[2].as_object()) {
            assert_ne!(a["root"], c["root"], "{}", row["concept"]);
        }
    }
}

#[test]
fn word_table_leaves_craft_gated_meanings_empty_and_names_bad_ids() {
    let mut w = bench();
    let table = table(&mut w, 0, &[2, 0], &["iron", "water"]);
    assert_eq!(table["varieties"][0]["id"], 2);
    assert_eq!(table["rows"][0]["concept"], "iron");
    assert!(table["rows"][0]["cells"][0].is_null());
    assert!(table["rows"][0]["cells"][1].is_null());
    assert!(table["rows"][1]["cells"][0].is_object());
    let error = w
        .word_table(0, "[0]", r#"["water","no-such-thing"]"#)
        .unwrap_err();
    assert!(error.contains("no-such-thing"), "{error}");
    assert!(w.word_table(0, "[0, 99]", r#"["water"]"#).is_err());
    assert!(w.word_table(0, "[0.5]", r#"["water"]"#).is_err());
    assert!(w.word_table(0, "nope", r#"["water"]"#).is_err());
}

#[test]
fn name_quarry_lists_held_lands_and_heard_peoples() {
    let mut w = bench();
    let generation = w.latest();
    let overview: Value = serde_json::from_str(&w.overview(generation).unwrap()).unwrap();
    let quarry: Value = serde_json::from_str(&w.name_quarry(generation, 0).unwrap()).unwrap();
    let world = w.chronicle.latest();
    assert_eq!(quarry["variety"], 0);
    assert!(!quarry["given"].as_array().unwrap().is_empty());
    let places = quarry["places"].as_array().unwrap();
    for &c in &world
        .living()
        .filter(|&c| world.communities[c].variety == 0)
        .collect::<Vec<_>>()
    {
        for &region in &world.communities[c].lands {
            let row = places
                .iter()
                .find(|p| p["feature"] == "land" && p["id"] == region)
                .unwrap_or_else(|| panic!("land {region}"));
            assert_eq!(row["name"], world.place_at(region, generation).unwrap());
            assert_ne!(row["origin"], "exonym");
        }
        let own = quarry["peoples"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["community"] == c)
            .unwrap();
        assert_eq!(own["name"], world.community_name(c));
    }
    let peoples = quarry["peoples"].as_array().unwrap();
    let mut heard = 0;
    for k in &world.contacts {
        for (by, other) in [(k.a, k.b), (k.b, k.a)] {
            if world.communities[by].variety != 0 || world.communities[other].variety == 0 {
                continue;
            }
            let row = peoples
                .iter()
                .find(|p| p["community"] == other)
                .unwrap_or_else(|| panic!("exonym for {other}"));
            assert_eq!(row["name"], world.exonym(other, by));
            assert_eq!(row["since"], k.since);
            assert!(!row["ipa"].as_str().unwrap().is_empty());
            heard += 1;
        }
    }
    assert!(heard > 0, "no exonym for a neighbour");
    // The overview lists the same exonyms, minus those said as the people
    // say their own name.
    for people in overview["communities"].as_array().unwrap() {
        for exonym in people["exonyms"].as_array().unwrap() {
            let by = exonym["by"].as_u64().unwrap() as usize;
            if world.communities[by].variety != 0 {
                continue;
            }
            let row = peoples
                .iter()
                .find(|p| p["community"] == people["id"])
                .unwrap_or_else(|| panic!("exonym for {}", people["id"]));
            assert_eq!(row["name"], exonym["name"]);
        }
    }
    assert!(w.name_quarry(generation, 99).is_err());
}

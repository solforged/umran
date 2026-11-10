use super::*;
use serde_json::{Value, json};
use umran_sim::LanguageDesign;

fn found(preset: &str, region: usize) -> String {
    let design = LanguageDesign::preset(preset, 1).unwrap();
    json!({
        "kind": "found", "naming": {"kind": "people"}, "design": design,
        "seed": 3, "power": 0.5, "openness": 0.5, "region": region
    })
    .to_string()
}

/// Two peoples in trade, a title, and thirty generations.
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
    let sites: Vec<usize> = serde_json::from_str(&w.founding_sites(anchor, 2).unwrap()).unwrap();
    w.act(&found("familiar", anchor)).unwrap();
    w.act(&found("polynesian", sites[1])).unwrap();
    w.act(r#"{"kind":"connect","a":0,"b":1,"intensity":0.6,"contact":"trade"}"#)
        .unwrap();
    w.act(r#"{"kind":"run","generations":30}"#).unwrap();
    w.set_title("The frozen sea");
    w
}

fn parsed(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn a_frozen_file_is_the_save_with_the_world_it_resolved_to() {
    let mut w = bench();
    let latest = w.latest();
    let file = parsed(&w.frozen(latest).unwrap());
    let frozen = &file["frozen"];

    // Without the record it is the save, member for member.
    let mut save = file.clone();
    save.as_object_mut().unwrap().remove("frozen");
    assert_eq!(save, parsed(&w.save().unwrap()));

    assert_eq!(frozen["version"], 1);
    assert_eq!(frozen["revision"], ENGINE_REVISION);
    assert_eq!(frozen["telling"], 0);
    assert_eq!(frozen["generation"], latest);
    assert_eq!(frozen["yearsPerGeneration"], 25);
    assert_eq!(frozen["point"], json!(w.chronicle.end()));

    // The record is what the workbench reads, under the same names.
    let overview = parsed(&w.overview(latest).unwrap());
    for member in [
        "communities",
        "contacts",
        "states",
        "cities",
        "religions",
        "continents",
        "crafts",
        "places",
        "moves",
        "annals",
    ] {
        assert_eq!(frozen[member], overview[member], "{member}");
    }
    let varieties = frozen["varieties"].as_array().unwrap();
    assert_eq!(
        varieties.len(),
        overview["varieties"].as_array().unwrap().len()
    );
    for (id, variety) in varieties.iter().enumerate() {
        let mut view = variety.clone();
        let own = view.as_object_mut().unwrap();
        let lexicon = own.remove("lexicon").unwrap();
        let quarry = own.remove("nameQuarry").unwrap();
        let phrasebook = own.remove("phrasebook").unwrap();
        assert_eq!(view, overview["varieties"][id]);
        assert_eq!(lexicon, parsed(&w.lexicon(latest, id).unwrap()));
        assert_eq!(quarry, parsed(&w.name_quarry(latest, id).unwrap()));
        assert_eq!(phrasebook, parsed(&w.phrasebook(latest, id).unwrap()));
        assert!(!lexicon.as_array().unwrap().is_empty());
    }
    let rivers = frozen["rivers"].as_array().unwrap();
    assert_eq!(rivers.len(), w.chronicle.latest().map.rivers.len());
    for (id, river) in rivers.iter().enumerate() {
        assert_eq!(*river, parsed(&w.river(latest, id).unwrap()));
    }
    for (id, lake) in frozen["lakes"].as_array().unwrap().iter().enumerate() {
        assert_eq!(*lake, parsed(&w.lake(latest, id).unwrap()));
    }

    // What belongs to a session reading the world is not part of the world.
    for session in [
        "mutation",
        "atTip",
        "latest",
        "timeline",
        "tellings",
        "savedRevision",
    ] {
        assert!(frozen.get(session).is_none(), "{session}");
    }
}

#[test]
fn a_frozen_file_opens_as_the_save_it_carries() {
    let mut w = bench();
    let latest = w.latest();
    let file = w.frozen(latest).unwrap();
    let mut opened = Bench::load(&file).unwrap();
    assert_eq!(opened.save().unwrap(), w.save().unwrap());
    assert_eq!(opened.title(), Some("The frozen sea"));
    assert_eq!(
        parsed(&opened.overview(latest).unwrap())["communities"],
        parsed(&file)["frozen"]["communities"]
    );
}

#[test]
fn an_earlier_reading_freezes_its_own_year_and_keeps_the_whole_history() {
    let mut w = bench();
    let latest = w.latest();
    let point = json!(w.chronicle.point_at(12)).to_string();
    let mut reading = w.read(0, &point).unwrap();
    let file = parsed(&reading.frozen(latest).unwrap());
    let frozen = &file["frozen"];
    assert_eq!(frozen["generation"], 12);
    assert_eq!(frozen["point"], parsed(&point));
    assert_eq!(
        frozen["communities"],
        parsed(&w.overview(12).unwrap())["communities"]
    );
    assert!(
        frozen["annals"]
            .as_array()
            .unwrap()
            .iter()
            .all(|annal| annal["generation"].as_u64().unwrap() <= 12)
    );
    // The recipe is not cut at the reading: the file reopens every year.
    assert_eq!(file["tellings"], parsed(&w.save().unwrap())["tellings"]);

    // Without a fixed reading, a year is frozen as the world stood in it.
    let earlier = parsed(&w.frozen(12).unwrap());
    assert_eq!(earlier["frozen"]["generation"], 12);
    assert_eq!(earlier["frozen"]["varieties"], frozen["varieties"]);
}

#[test]
fn a_frozen_record_names_the_engine_that_resolved_it_not_the_recipes_stamp() {
    let w = bench();
    let earlier = ENGINE_REVISION - 1;
    let mut old = parsed(&w.save().unwrap());
    old["revision"] = earlier.into();
    let mut reread = Bench::load(&old.to_string()).unwrap();
    let file = parsed(&reread.frozen(reread.latest()).unwrap());
    // The recipe was written under the earlier revision and stands unchanged.
    assert_eq!(file["revision"], earlier);
    // This engine read it, and the record is of that reading.
    assert_eq!(file["frozen"]["revision"], ENGINE_REVISION);
}

//! JSON-line driver for scripts/parity.ts. Only public Bench views are read.
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use umran_web::Bench;

fn entity_id(value: &Value, missing: &str) -> Result<usize, String> {
    let id = value.as_u64().ok_or(missing)?;
    let id = u32::try_from(id).map_err(|error| error.to_string())?;
    usize::try_from(id).map_err(|error| error.to_string())
}

fn snapshot(bench: &mut Bench, generation: u32) -> Result<Value, String> {
    let mut overview: Value =
        serde_json::from_str(&bench.overview(generation)?).map_err(|error| error.to_string())?;
    // These describe the present history, not the requested generation. In
    // particular, timeline labels use names from the present even in the past.
    let object = overview
        .as_object_mut()
        .ok_or("Overview is not an object")?;
    object.remove("latest");
    object.remove("timeline");
    object.remove("mutation");
    object.remove("point");
    object.remove("atTip");
    object.remove("tellings");
    let varieties = overview["varieties"]
        .as_array()
        .ok_or("Missing varieties")?;
    let mut lexicons = Vec::with_capacity(varieties.len());
    let mut competitors = Vec::new();
    for variety in varieties {
        let id = entity_id(&variety["id"], "Missing variety id")?;
        let rows: Value = serde_json::from_str(&bench.lexicon(generation, id)?)
            .map_err(|error| error.to_string())?;
        for row in rows.as_array().ok_or("Lexicon is not an array")? {
            if row["competitors"].as_u64().unwrap_or(0) > 0 {
                let concept = row["concept"].as_str().ok_or("Missing concept")?;
                let detail: Value = serde_json::from_str(&bench.word(generation, id, concept)?)
                    .map_err(|error| error.to_string())?;
                // The lexicon has every dominant form; word() supplies every
                // competing living form, its share, senses, and sound history.
                competitors.push(json!({
                    "variety": id,
                    "concept": concept,
                    "variants": detail["variants"],
                }));
            }
        }
        lexicons.push(json!({ "variety": id, "rows": rows }));
    }
    let map: Value = serde_json::from_str(&bench.map()?).map_err(|error| error.to_string())?;
    let climate: Value =
        serde_json::from_str(&bench.climate(generation)?).map_err(|error| error.to_string())?;
    let rivers = map["rivers"]
        .as_array()
        .ok_or("Missing rivers")?
        .iter()
        .map(|river| {
            let id = entity_id(&river["id"], "Missing river id")?;
            serde_json::from_str::<Value>(&bench.river(generation, id)?)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!({
        "overview": overview,
        "map": map,
        "lexicons": lexicons,
        "competitors": competitors,
        "climate": climate,
        "rivers": rivers,
    }))
}

fn act(bench: &mut Bench, mut action: Value) -> Result<(), String> {
    // A harness-only action descriptor resolves presets independently on each
    // architecture, rather than hiding design drift behind a WASM-made recipe.
    if let Some(preset) = action.get("preset").and_then(Value::as_str) {
        let seed = action["seed"].as_u64().ok_or("Missing design seed")?;
        let seed = u32::try_from(seed).map_err(|error| error.to_string())?;
        let design: Value = serde_json::from_str(&Bench::design(preset, seed)?)
            .map_err(|error| error.to_string())?;
        action
            .as_object_mut()
            .ok_or("Action is not an object")?
            .remove("preset");
        action["design"] = design;
    }
    bench.act(&action.to_string())
}

fn request(bench: &mut Option<Bench>, input: Value) -> Result<Value, String> {
    match input["kind"].as_str().ok_or("Missing request kind")? {
        "catalog" => serde_json::from_str(&Bench::catalog()?).map_err(|error| error.to_string()),
        "new" => {
            let seed = input["seed"].as_u64().ok_or("Missing world seed")?;
            let seed = u32::try_from(seed).map_err(|error| error.to_string())?;
            let map = input["map"].as_str().ok_or("Missing map size")?;
            *bench = Some(match input["geography"].as_str() {
                Some(geography) => Bench::with_geography(seed, map, geography)?,
                None => Bench::new(seed, map)?,
            });
            Ok(Value::Null)
        }
        "load" => {
            *bench = Some(Bench::load(
                input["recipe"].as_str().ok_or("Missing recipe")?,
            )?);
            Ok(Value::Null)
        }
        "act" => {
            act(
                bench.as_mut().ok_or("No world open")?,
                input["action"].clone(),
            )?;
            Ok(Value::Null)
        }
        "settlement" => {
            let preview = bench.as_ref().ok_or("No world open")?.settlement(
                &input["point"].to_string(),
                entity_id(&input["community"], "Missing community")?,
                input["intent"].as_str().ok_or("Missing intent")?,
                input["share"].as_f64().ok_or("Missing share")? as f32,
                input["destination"].as_i64().ok_or("Missing destination")? as i32,
            )?;
            serde_json::from_str(&preview).map_err(|e| e.to_string())
        }
        "founding-sites" => {
            let region = entity_id(&input["region"], "Missing anchor region")?;
            let count = usize::try_from(input["count"].as_u64().ok_or("Missing site count")?)
                .map_err(|error| error.to_string())?;
            let sites = bench
                .as_ref()
                .ok_or("No world open")?
                .founding_sites(region, count)?;
            serde_json::from_str(&sites).map_err(|e| e.to_string())
        }
        "decisions" => {
            let decisions = bench.as_ref().ok_or("No world open")?.decisions()?;
            serde_json::from_str(&decisions).map_err(|e| e.to_string())
        }
        "law-choices" => {
            let choices = bench.as_ref().ok_or("No world open")?.law_choices(
                &input["point"].to_string(),
                entity_id(&input["variety"], "Missing variety")?,
            )?;
            serde_json::from_str(&choices).map_err(|e| e.to_string())
        }
        "branch" => {
            let generation = input["generation"].as_u64().ok_or("Missing generation")?;
            let generation = u32::try_from(generation).map_err(|error| error.to_string())?;
            bench.as_mut().ok_or("No world open")?.branch(generation);
            Ok(Value::Null)
        }
        "save" => Ok(Value::String(
            bench.as_ref().ok_or("No world open")?.save()?,
        )),
        "snapshot" => {
            let generation = input["generation"].as_u64().ok_or("Missing generation")?;
            let generation = u32::try_from(generation).map_err(|error| error.to_string())?;
            snapshot(bench.as_mut().ok_or("No world open")?, generation)
        }
        other => Err(format!("Unknown request: {other}")),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut bench = None;
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let input = serde_json::from_str(&line?);
        let result = match input {
            Ok(input) => request(&mut bench, input),
            Err(error) => Err(error.to_string()),
        };
        let response = match result {
            Ok(value) => json!({ "ok": value }),
            Err(error) => json!({ "error": error }),
        };
        serde_json::to_writer(&mut stdout, &response)?;
        writeln!(stdout)?;
        stdout.flush()?;
    }
    Ok(())
}

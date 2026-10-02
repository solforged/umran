//! JSON-line driver for scripts/parity.ts. Only public Bench views are read.
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use umran_web::Bench;

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
    let varieties = overview["varieties"]
        .as_array()
        .ok_or("Missing varieties")?;
    let mut lexicons = Vec::with_capacity(varieties.len());
    let mut competitors = Vec::new();
    for variety in varieties {
        let id = variety["id"].as_u64().ok_or("Missing variety id")? as usize;
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
    let climate: Value = serde_json::from_str(&bench.climate(generation)?)
        .map_err(|error| error.to_string())?;
    let rivers = map["rivers"].as_array().ok_or("Missing rivers")?
        .iter().map(|river| {
            let id = river["id"].as_u64().ok_or("Missing river id")? as usize;
            serde_json::from_str::<Value>(&bench.river(generation, id)?)
                .map_err(|error| error.to_string())
        }).collect::<Result<Vec<_>, String>>()?;
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
        "new" => {
            let seed = input["seed"].as_u64().ok_or("Missing world seed")?;
            let seed = u32::try_from(seed).map_err(|error| error.to_string())?;
            *bench = Some(Bench::new(
                seed,
                input["map"].as_str().ok_or("Missing map size")?,
            )?);
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

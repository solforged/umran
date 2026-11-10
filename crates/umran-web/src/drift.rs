//! What this engine reads differently from a frozen record. A recipe replays
//! exactly only on the revision that wrote it, so a world saved under another
//! revision may have drifted: peoples renamed, words spelled otherwise, the
//! chronicle parting ways in some year. The record says how it read then; a
//! replay to the same point says how it reads now. This compares the two.
//!
//! Peoples and languages are matched by number. Numbers are given in order
//! of arising, so once two chronicles part ways the same number can be a
//! different people. That is still the honest comparison: the fourth people
//! the author read about is no longer the one they read about.
use crate::{Bench, ReadView, Workbench, fail, to_json};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use umran_sim::{ENGINE_REVISION, TellingId};
use wasm_bindgen::prelude::*;

/// How many examples a report carries of each kind.
const EXAMPLES: usize = 6;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Drift {
    /// The engine revision that wrote the record, and this one.
    was: u32,
    now: u32,
    generation: u32,
    /// Nothing in the record reads differently.
    same: bool,
    /// The generation of the first chronicle entry that differs.
    parts_ways: Option<u32>,
    peoples: Renamed,
    languages: Renamed,
    /// Lands, cities, states, and faiths.
    names: Changed,
    words: Changed,
}

/// Who is called something else, of how many.
#[derive(Serialize)]
struct Renamed {
    total: usize,
    changed: Vec<Example>,
}

#[derive(Serialize)]
struct Changed {
    total: usize,
    changed: usize,
    examples: Vec<Example>,
}

#[derive(Serialize)]
struct Example {
    /// What it is: a people's or language's number, a word's gloss with its
    /// language, or the kind of thing named.
    of: String,
    was: String,
    /// `None` when this engine has no such thing at all.
    now: Option<String>,
}

/// Whether two records say the same. Numbers are compared by value: a file
/// that has passed through another JSON tool may write `1.0` as `1`.
fn alike(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a == b || a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| alike(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, a)| b.get(key).is_some_and(|b| alike(a, b)))
        }
        _ => a == b,
    }
}

fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map_or(&[], Vec::as_slice)
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

fn renamed(was: &Value, now: &Value, key: &str) -> Renamed {
    let (old, new) = (list(was, key), list(now, key));
    Renamed {
        total: old.len(),
        changed: old
            .iter()
            .enumerate()
            .filter_map(|(id, entry)| {
                let other = new.get(id).map(|other| text(other, "name"));
                (other != Some(text(entry, "name"))).then(|| Example {
                    of: id.to_string(),
                    was: text(entry, "name").into(),
                    now: other.map(str::to_owned),
                })
            })
            .collect(),
    }
}

impl Changed {
    fn new() -> Self {
        Changed {
            total: 0,
            changed: 0,
            examples: Vec::new(),
        }
    }

    fn weigh(&mut self, of: impl FnOnce() -> String, was: &str, now: Option<&str>) {
        self.total += 1;
        if now == Some(was) {
            return;
        }
        self.changed += 1;
        if self.examples.len() < EXAMPLES {
            self.examples.push(Example {
                of: of(),
                was: was.into(),
                now: now.map(str::to_owned),
            });
        }
    }
}

/// Every name the record gives a land, city, state, or faith, against the
/// same thing's name now.
fn names(was: &Value, now: &Value) -> Changed {
    let mut out = Changed::new();
    let current = |place: &Value| -> Option<String> {
        list(place, "names")
            .last()
            .map(|name| text(name, "spelled").to_owned())
    };
    let lands: BTreeMap<u64, &Value> = list(now, "places")
        .iter()
        .filter_map(|place| Some((place["region"].as_u64()?, place)))
        .collect();
    for place in list(was, "places") {
        let Some(name) = current(place) else { continue };
        let other = place["region"]
            .as_u64()
            .and_then(|region| lands.get(&region))
            .and_then(|place| current(place));
        out.weigh(|| "land".into(), &name, other.as_deref());
    }
    let named = |entry: &Value| -> String {
        // A city's name is a name view; a state's and a faith's are strings.
        match &entry["name"] {
            Value::String(name) => name.clone(),
            view => text(view, "name").to_owned(),
        }
    };
    for (key, kind) in [
        ("cities", "city"),
        ("states", "state"),
        ("religions", "faith"),
    ] {
        let new = list(now, key);
        for (id, entry) in list(was, key).iter().enumerate() {
            let other = new.get(id).map(named);
            out.weigh(|| kind.into(), &named(entry), other.as_deref());
        }
    }
    out
}

/// Every word the record gives, by language and meaning, against this
/// engine's word for the same meaning in the language of the same number.
fn words(was: &Value, now: &Value) -> Changed {
    let mut out = Changed::new();
    let said = |row: &Value| format!("{} /{}/", text(row, "spelled"), text(row, "ipa"));
    let new = list(now, "varieties");
    for (id, variety) in list(was, "varieties").iter().enumerate() {
        let lexicon: BTreeMap<&str, &Value> = new
            .get(id)
            .map(|other| list(other, "lexicon"))
            .unwrap_or_default()
            .iter()
            .map(|row| (text(row, "concept"), row))
            .collect();
        for row in list(variety, "lexicon") {
            let other = lexicon.get(text(row, "concept")).map(|row| said(row));
            out.weigh(
                || format!("“{}” in {}", text(row, "gloss"), text(variety, "name")),
                &said(row),
                other.as_deref(),
            );
        }
    }
    out
}

impl Bench {
    /// What this engine reads differently from the frozen record in `file`,
    /// at the telling and point the record was read at.
    pub fn drift(&self, file: &str) -> Result<String, String> {
        let file: Value =
            serde_json::from_str(file).map_err(|e| format!("Not a frozen world: {e}"))?;
        let was = &file["frozen"];
        let revision = was["revision"]
            .as_u64()
            .and_then(|revision| u32::try_from(revision).ok())
            .ok_or("This file carries no frozen record.")?;
        let telling = was["telling"]
            .as_u64()
            .and_then(|telling| TellingId::try_from(telling).ok())
            .ok_or("The frozen record does not say which telling it read.")?;
        let mut reading = self
            .read(telling, &was["point"].to_string())
            .map_err(|_| "This history no longer has the point the record was read at.")?;
        let generation = reading.latest();
        let now: Value =
            serde_json::from_str(&reading.frozen(generation)?).map_err(|e| e.to_string())?;
        let now = &now["frozen"];
        let generation = now["generation"]
            .as_u64()
            .and_then(|generation| u32::try_from(generation).ok())
            .unwrap_or(generation);
        let (old, new) = (list(was, "annals"), list(now, "annals"));
        let parts_ways = old
            .iter()
            .zip(new)
            .find(|(a, b)| !alike(a, b))
            .map(|(a, _)| a)
            .or_else(|| old.get(new.len()).or_else(|| new.get(old.len())))
            .and_then(|annal| annal["generation"].as_u64())
            .and_then(|generation| u32::try_from(generation).ok());
        // The record's own stamps differ by design; the world is what counts.
        let world = |record: &Value| {
            let mut world = record.clone();
            if let Some(members) = world.as_object_mut() {
                members.remove("revision");
                members.remove("version");
            }
            world
        };
        to_json(&Drift {
            was: revision,
            now: ENGINE_REVISION,
            generation,
            same: alike(&world(was), &world(now)),
            parts_ways,
            peoples: renamed(was, now, "communities"),
            languages: renamed(was, now, "varieties"),
            names: names(was, now),
            words: words(was, now),
        })
    }
}

#[wasm_bindgen]
impl Workbench {
    pub fn drift(&self, file: &str) -> Result<String, JsValue> {
        self.bench.drift(file).map_err(fail)
    }
}

#[wasm_bindgen]
impl ReadView {
    pub fn drift(&self, file: &str) -> Result<String, JsValue> {
        self.bench.borrow().drift(file).map_err(fail)
    }
}

#[cfg(test)]
mod tests;

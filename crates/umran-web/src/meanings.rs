//! Reading-scoped author meaning and word checks.
use crate::{Bench, ReadView, Workbench, concept_view, entity_id, fail, to_json};
use serde::Serialize;
use umran_sim::meanings::WordError;
use umran_sim::{ConceptTable, HistoryPoint, Meaning, Root, RootError};
use wasm_bindgen::prelude::*;

pub(crate) fn table(json: &str) -> Result<ConceptTable, String> {
    let meanings: Vec<Meaning> =
        serde_json::from_str(json).map_err(|e| format!("Malformed meanings: {e}"))?;
    let mut table = ConceptTable::default();
    for meaning in meanings {
        table.append(meaning).map_err(|e| e.status().to_string())?;
    }
    Ok(table)
}

#[derive(Serialize)]
struct MeaningCheck<'a> {
    status: &'static str,
    message: &'static str,
    suggestions: Vec<&'a str>,
}

#[derive(Serialize)]
struct CurrentWord {
    form: String,
    spelled: String,
}

#[derive(Serialize)]
struct WordCheck {
    status: &'static str,
    detail: Option<String>,
    suggestions: Vec<String>,
    spelled: Option<String>,
    current: Option<CurrentWord>,
}

impl Bench {
    pub fn meanings(&mut self, generation: u32) -> Result<String, String> {
        let table = &self.world(generation).concepts;
        to_json(
            &table
                .iter()
                .map(|(id, c)| concept_view(id, c, table.is_custom(id)))
                .collect::<Vec<_>>(),
        )
    }

    pub fn check_meaning(&self, point: &str, meaning: &str) -> Result<String, String> {
        let point: HistoryPoint = serde_json::from_str(point).map_err(|e| e.to_string())?;
        let world = self.chronicle.world_at_point(point)?;
        let raw: serde_json::Value = serde_json::from_str(meaning).map_err(|e| e.to_string())?;
        let gloss = raw.get("gloss").and_then(|v| v.as_str()).unwrap_or("");
        let words: Vec<String> = gloss
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect();
        let suggestions = umran_sim::CONCEPTS
            .iter()
            .filter(|c| {
                c.gloss
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|word| words.iter().any(|w| w.eq_ignore_ascii_case(word)))
            })
            .take(3)
            .map(|c| c.id.as_ref())
            .collect();
        let status = if raw
            .get("class")
            .and_then(|v| v.as_str())
            .is_none_or(|c| !matches!(c, "entity" | "event" | "property"))
        {
            "bad-class"
        } else if raw
            .get("from")
            .filter(|v| !v.is_null())
            .is_some_and(|from| {
                from.get("relation").is_none_or(|relation| {
                    serde_json::from_value::<umran_sim::Relation>(relation.clone()).is_err()
                })
            })
        {
            "bad-relation"
        } else {
            let meaning: Meaning = serde_json::from_value(raw).map_err(|e| e.to_string())?;
            world
                .concepts
                .check(&meaning)
                .err()
                .map_or("ok", |e| e.status())
        };
        let message = match status {
            "ok" => "",
            "bad-id" => "A short name is 1 to 32 lowercase letters, digits, or hyphens.",
            "taken" => "A meaning by that short name exists already.",
            "bad-gloss" => "A meaning is glossed in 1 to 40 characters.",
            "bad-class" => "A meaning is a thing, an action, or a quality.",
            "unknown-base" => {
                "No meaning by that short name to build from; pronouns cannot be a base."
            }
            _ => "No such relation to the base.",
        };
        to_json(&MeaningCheck {
            status,
            message,
            suggestions,
        })
    }

    pub fn check_word(
        &self,
        point: &str,
        variety: usize,
        concept: &str,
        ipa: &str,
    ) -> Result<String, String> {
        let point: HistoryPoint = serde_json::from_str(point).map_err(|e| e.to_string())?;
        let world = self.chronicle.world_at_point(point)?;
        let root = Root {
            concept: concept.into(),
            ipa: ipa.into(),
        };
        let mut check = WordCheck {
            status: "ok",
            detail: None,
            suggestions: Vec::new(),
            spelled: None,
            current: None,
        };
        if let Some(speech) = world.varieties.get(variety) {
            check.current = world
                .concepts
                .by_id(concept)
                .and_then(|id| speech.lexicon.word_for(id))
                .map(|word| CurrentWord {
                    form: word.form.ipa_stressed(speech.stress()),
                    spelled: speech.spell(&word.form),
                });
        }
        match world.check_word(variety, &root) {
            Ok((_, form)) => check.spelled = Some(world.varieties[variety].spell(&form)),
            Err(WordError::Pronoun) => check.status = "pronoun",
            Err(WordError::Dead) => check.status = "dead",
            Err(WordError::Root(error)) => match error {
                RootError::UnknownConcept => {
                    check.status = "unknown-meaning";
                    check.suggestions = world
                        .concepts
                        .iter()
                        .filter(|(_, c)| {
                            !concept.is_empty()
                                && (c.id.contains(concept) || c.gloss.contains(concept))
                        })
                        .take(3)
                        .map(|(_, c)| c.id.to_string())
                        .collect();
                }
                RootError::BadIpa(detail) => {
                    check.status = "bad-ipa";
                    check.detail = detail;
                }
                RootError::NoVowel => check.status = "no-vowel",
                RootError::OutsideInventory(detail) => {
                    check.status = "outside-inventory";
                    check.detail = Some(detail);
                }
                RootError::Duplicate => {
                    unreachable!("word actions replace a slot rather than submit duplicate roots")
                }
            },
        }
        to_json(&check)
    }
}

#[wasm_bindgen]
impl Workbench {
    pub fn meanings(&mut self, generation: u32) -> Result<String, JsValue> {
        self.bench.meanings(generation).map_err(fail)
    }

    #[wasm_bindgen(js_name = checkMeaning)]
    pub fn check_meaning(&self, point: &str, meaning: &str) -> Result<String, JsValue> {
        self.bench.check_meaning(point, meaning).map_err(fail)
    }

    #[wasm_bindgen(js_name = checkWord)]
    pub fn check_word(
        &self,
        point: &str,
        variety: f64,
        concept: &str,
        ipa: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .check_word(point, entity_id(variety).map_err(fail)?, concept, ipa)
            .map_err(fail)
    }
}

#[wasm_bindgen]
impl ReadView {
    pub fn meanings(&mut self, generation: u32) -> Result<String, JsValue> {
        self.bench.borrow_mut().meanings(generation).map_err(fail)
    }

    #[wasm_bindgen(js_name = checkMeaning)]
    pub fn check_meaning(&self, point: &str, meaning: &str) -> Result<String, JsValue> {
        self.bench
            .borrow()
            .check_meaning(point, meaning)
            .map_err(fail)
    }

    #[wasm_bindgen(js_name = checkWord)]
    pub fn check_word(
        &self,
        point: &str,
        variety: f64,
        concept: &str,
        ipa: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow()
            .check_word(point, entity_id(variety).map_err(fail)?, concept, ipa)
            .map_err(fail)
    }
}

#[cfg(test)]
mod tests;

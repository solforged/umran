//! Read-only law inspection through the real engine application path.
use crate::{Bench, ReadView, Workbench, entity_id, fail, to_json};
use serde::{Deserialize, Serialize};
use umran_sim::notation::{self, AuthoredLaw};
use umran_sim::{Form, HistoryPoint, Law, StressRule, World};
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Choice {
    Catalog(String),
    Authored(AuthoredLaw),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Preview {
    title: String,
    rules: Vec<Rule>,
    words: Vec<Word>,
    names: Vec<Name>,
    unchanged_words: usize,
    unchanged_names: usize,
    stress: Option<Stress>,
}
#[derive(Serialize)]
struct Rule {
    source: String,
    changed: usize,
    unrealisable: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Word {
    concept: String,
    gloss: String,
    before: String,
    after: String,
    spelled_before: String,
    spelled_after: String,
}
#[derive(Serialize)]
struct Name {
    kind: &'static str,
    subject: Option<usize>,
    before: String,
    after: String,
}
#[derive(Serialize)]
struct Stress {
    before: StressRule,
    after: StressRule,
}

impl Bench {
    pub fn check_law(title: &str, source: &str) -> Result<String, String> {
        to_json(&notation::parse(title, source))
    }

    pub fn law_preview(&self, point: &str, variety: usize, law: &str) -> Result<String, String> {
        let point: HistoryPoint = serde_json::from_str(point).map_err(|e| e.to_string())?;
        let world = self.chronicle.world_at_point(point)?;
        if !world
            .communities
            .iter()
            .any(|c| c.living() && c.variety == variety)
        {
            return Err(format!("language {variety} has no living speakers"));
        }
        let choice: Choice = serde_json::from_str(law).map_err(|e| e.to_string())?;
        let law = match choice {
            Choice::Catalog(id) => world
                .law_catalog()
                .iter()
                .find(|law| law.id == id)
                .filter(|law| law.id != "koine-levelling")
                .cloned()
                .ok_or_else(|| format!("there is no portable sound law {id}"))?,
            Choice::Authored(law) => law.law(world.generation),
        };
        to_json(&preview(&world, variety, &law))
    }
}

fn preview(world: &World, variety: usize, law: &Law) -> Preview {
    let speech = &world.varieties[variety];
    let stress = speech.stress();
    let mut rules: Vec<_> = law
        .rule_sources()
        .into_iter()
        .map(|source| Rule {
            source,
            changed: 0,
            unrealisable: 0,
        })
        .collect();
    let mut count = |form: &Form| {
        let mut current = std::borrow::Cow::Borrowed(form);
        for (i, rule) in law.rules.iter().enumerate() {
            rules[i].unrealisable += rule.unrealisable(&current, stress);
            let next = rule.apply(&current, stress);
            if !speech.minimal.blocks(&current, &next) && *current != next {
                rules[i].changed += 1;
                current = std::borrow::Cow::Owned(next);
            }
        }
        if let Some(next) = law.stress {
            rules[law.rules.len()].changed +=
                usize::from(current.stressed_syllable(stress) != current.stressed_syllable(next));
        }
    };
    for (form, _) in speech.spoken_forms() {
        count(form);
    }
    world.visit_law_names(variety, |_, _, name| count(&name.form));

    // This exact path includes grammar, productive harmony, tone observation,
    // and Name::change. No stochastic selection or generation runs on the copy.
    let mut applied = world.clone();
    applied.apply_law(variety, law);
    let next = &applied.varieties[variety];
    let mut words = Vec::new();
    let mut unchanged_words = 0;
    for slot in &speech.lexicon.slots {
        for variant in &slot.variants {
            let word = speech.lexicon.get(variant.lexeme);
            if word.obsolete.is_some() {
                continue;
            }
            let after = &next.lexicon.get(word.id).form;
            if !law.changes(&word.form, after, stress) {
                unchanged_words += 1;
                continue;
            }
            let concept = &speech.lexicon.concepts[slot.concept];
            words.push(Word {
                concept: concept.id.to_string(),
                gloss: concept.gloss.to_string(),
                before: word.form.ipa_stressed(stress),
                after: after.ipa_stressed(next.stress()),
                spelled_before: speech.spell(&word.form),
                spelled_after: next.spell(after),
            });
        }
    }
    let mut originals = Vec::new();
    world.visit_law_names(variety, |kind, subject, name| {
        originals.push((kind, subject, name))
    });
    let mut names = Vec::new();
    let mut unchanged_names = 0;
    let mut i = 0;
    applied.visit_law_names(variety, |kind, subject, name| {
        let (old_kind, old_subject, before) = originals[i];
        i += 1;
        debug_assert_eq!((kind, subject), (old_kind, old_subject));
        if law.changes(&before.form, &name.form, stress) {
            names.push(Name {
                kind,
                subject,
                before: before.form.ipa_stressed(stress),
                after: name.form.ipa_stressed(next.stress()),
            });
        } else {
            unchanged_names += 1;
        }
    });
    names.sort_by_key(|n| (n.kind, n.subject));
    Preview {
        title: law.label.to_string(),
        rules,
        words,
        names,
        unchanged_words,
        unchanged_names,
        stress: law.stress.map(|after| Stress {
            before: stress,
            after,
        }),
    }
}

#[wasm_bindgen]
impl Workbench {
    #[wasm_bindgen(js_name = checkLaw)]
    pub fn check_law(title: &str, source: &str) -> Result<String, JsValue> {
        Bench::check_law(title, source).map_err(fail)
    }
    #[wasm_bindgen(js_name = lawPreview)]
    pub fn law_preview(&self, point: &str, variety: f64, law: &str) -> Result<String, JsValue> {
        self.bench
            .law_preview(point, entity_id(variety).map_err(fail)?, law)
            .map_err(fail)
    }
}
#[wasm_bindgen]
impl ReadView {
    #[wasm_bindgen(js_name = lawPreview)]
    pub fn law_preview(&self, point: &str, variety: f64, law: &str) -> Result<String, JsValue> {
        self.bench
            .borrow()
            .law_preview(point, entity_id(variety).map_err(fail)?, law)
            .map_err(fail)
    }
}

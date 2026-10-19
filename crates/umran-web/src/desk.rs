//! Blind comparison and its separate answer key, plus speech across tellings.
//! The blind path only renders `compare_varieties`; lineage is read in `truth`.
use crate::provenance::{LoanCauseView, cause_view};
use crate::{
    Bench, DecisionView, ReadView, SPECIMEN, SpecimenWord, Workbench, entity_id, fail, specimen,
    substrate_label, to_json,
};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use umran_sim::compare::{self, Pair};
use umran_sim::concepts::{Concept, Field, by_id};
use umran_sim::{CATALOG, CONCEPTS, Form, Lexeme, Origin, TellingId, Variety, World};
use wasm_bindgen::prelude::*;

pub(crate) const FIELDS: [(Field, &str); 23] = [
    (Field::PhysicalWorld, "physical-world"),
    (Field::Kinship, "kinship"),
    (Field::Animals, "animals"),
    (Field::Body, "body"),
    (Field::FoodDrink, "food-drink"),
    (Field::ClothingGrooming, "clothing-grooming"),
    (Field::House, "house"),
    (Field::Agriculture, "agriculture"),
    (Field::BasicActions, "basic-actions"),
    (Field::Motion, "motion"),
    (Field::Possession, "possession"),
    (Field::Spatial, "spatial"),
    (Field::Quantity, "quantity"),
    (Field::Time, "time"),
    (Field::SensePerception, "sense-perception"),
    (Field::Emotions, "emotions"),
    (Field::Cognition, "cognition"),
    (Field::Speech, "speech"),
    (Field::Social, "social"),
    (Field::Warfare, "warfare"),
    (Field::Law, "law"),
    (Field::Religion, "religion"),
    (Field::Function, "function"),
];

/// The kebab-case id a meaning set and the concept catalog use for a field.
pub(crate) fn field_id(field: Field) -> &'static str {
    FIELDS
        .iter()
        .find(|(f, _)| *f == field)
        .map(|(_, id)| *id)
        .expect("every field has a meaning set")
}

#[derive(Serialize)]
struct MeaningSet {
    id: &'static str,
    label: &'static str,
}

fn meaning_sets() -> Vec<MeaningSet> {
    [
        MeaningSet {
            id: "core",
            label: "Core words",
        },
        MeaningSet {
            id: "specimen",
            label: "Specimen",
        },
    ]
    .into_iter()
    .chain(FIELDS.iter().map(|&(field, id)| MeaningSet {
        id,
        label: field.label(),
    }))
    .collect()
}

fn concepts(meanings: &str) -> Result<Vec<&'static Concept>, String> {
    match meanings {
        "core" => Ok(CONCEPTS.iter().filter(|c| c.stability.is_some()).collect()),
        "specimen" => Ok(SPECIMEN
            .iter()
            .map(|id| by_id(id).expect("specimen concept"))
            .collect()),
        _ => {
            let field = FIELDS
                .iter()
                .find(|(_, id)| *id == meanings)
                .map(|(field, _)| *field)
                .ok_or("Unknown meaning set.")?;
            Ok(CONCEPTS.iter().filter(|c| c.field == field).collect())
        }
    }
}

fn speech(world: &World, variety: usize) -> Result<&Variety, String> {
    world
        .varieties
        .get(variety)
        .ok_or_else(|| "No such language variety.".into())
}

#[derive(Serialize, PartialEq, Eq)]
struct FormView {
    spelled: String,
    ipa: String,
}

fn form_view(variety: &Variety, form: &Form) -> FormView {
    FormView {
        spelled: variety.spell(form),
        ipa: form.ipa_stressed(variety.stress()),
    }
}

#[derive(Serialize)]
struct Segment {
    a: Option<&'static str>,
    b: Option<&'static str>,
}

fn segment(pair: Pair) -> Segment {
    Segment {
        a: pair.0.map(|p| CATALOG.get(p).ipa()),
        b: pair.1.map(|p| CATALOG.get(p).ipa()),
    }
}

#[derive(Serialize)]
struct CorrespondenceRow {
    concept: &'static str,
    gloss: &'static str,
    a: FormView,
    b: FormView,
    alignment: Vec<Segment>,
    regularity: f32,
    cognate: bool,
}

#[derive(Serialize)]
struct RegularPair {
    pair: Segment,
    support: usize,
}

#[derive(Serialize, PartialEq, Eq)]
struct GrammarObservation {
    category: &'static str,
    a: String,
    b: String,
    regular: bool,
}

#[derive(Serialize)]
struct Correspondences<'a> {
    a: usize,
    b: usize,
    generation: u32,
    meanings: &'a str,
    rows: Vec<CorrespondenceRow>,
    regular: Vec<RegularPair>,
    grammar: Vec<GrammarObservation>,
}

fn correspondences<'a>(
    world: &World,
    a: usize,
    b: usize,
    meanings: &'a str,
) -> Result<Correspondences<'a>, String> {
    let (va, vb) = (speech(world, a)?, speech(world, b)?);
    let comparison = compare::compare_varieties(va, vb, &concepts(meanings)?);
    // The method's support includes repeated positions and capped grammatical
    // evidence. The desk promises rows, so count each pair once per lexical row.
    // Gaps remain in the displayed alignment only: the method excludes them
    // from correspondence support and regularity, and owns the cognate verdict.
    let mut regular: Vec<_> = comparison
        .regular
        .iter()
        .map(|&(pair, _)| {
            (
                pair,
                comparison
                    .rows
                    .iter()
                    .filter(|row| row.alignment.contains(&pair))
                    .count(),
            )
        })
        .collect();
    regular.sort_by(|(a, na), (b, nb)| {
        nb.cmp(na).then_with(|| {
            (a.0.map(|p| p.0), a.1.map(|p| p.0)).cmp(&(b.0.map(|p| p.0), b.1.map(|p| p.0)))
        })
    });
    let mut grammar = Vec::new();
    for observation in comparison.grammar {
        let view = GrammarObservation {
            category: observation.category.id(),
            a: observation
                .alignment
                .iter()
                .filter_map(|p| p.0)
                .map(|p| CATALOG.get(p).ipa())
                .collect(),
            b: observation
                .alignment
                .iter()
                .filter_map(|p| p.1)
                .map(|p| CATALOG.get(p).ipa())
                .collect(),
            regular: observation.regular,
        };
        // The same audible contrast may occur on many words; retain differing
        // contrasts and differing support, not duplicate display observations.
        if !grammar.contains(&view) {
            grammar.push(view);
        }
    }
    Ok(Correspondences {
        a,
        b,
        generation: world.generation,
        meanings,
        rows: comparison
            .rows
            .into_iter()
            .map(|row| CorrespondenceRow {
                concept: row.concept.id,
                gloss: row.concept.gloss,
                a: form_view(va, &row.a),
                b: form_view(vb, &row.b),
                alignment: row.alignment.into_iter().map(segment).collect(),
                regularity: row.regularity,
                cognate: row.cognate,
            })
            .collect(),
        regular: regular
            .into_iter()
            .map(|(pair, support)| RegularPair {
                pair: segment(pair),
                support,
            })
            .collect(),
        grammar,
    })
}

#[derive(Serialize)]
struct Borrowed {
    into: usize,
    from: usize,
    cause: LoanCauseView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TruthRow {
    concept: &'static str,
    same_root: bool,
    borrowed: Option<Borrowed>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Truth {
    rows: Vec<TruthRow>,
    ancestor: Option<usize>,
    split_at: Option<u32>,
}

fn borrowed(world: &World, variety: usize, word: &Lexeme) -> Option<Borrowed> {
    let Origin::Borrowed { from, cause, .. } = word.origin else {
        return None;
    };
    Some(Borrowed {
        into: world.root_of(variety, word.id).0,
        from,
        cause: cause_view(world, cause, word.born),
    })
}

fn ancestor(world: &World, a: usize, b: usize) -> (Option<usize>, Option<u32>) {
    let mut path = Vec::new();
    let (mut at, mut split) = (a, None);
    loop {
        path.push((at, split));
        let Some(fork) = world.varieties[at].parent else {
            break;
        };
        at = fork.variety;
        split = Some(fork.generation);
    }
    let (mut at, mut split_b) = (b, None);
    loop {
        if let Some(&(_, split_a)) = path.iter().find(|(id, _)| *id == at) {
            return (
                Some(at),
                match (split_a, split_b) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                },
            );
        }
        let Some(fork) = world.varieties[at].parent else {
            return (None, None);
        };
        at = fork.variety;
        split_b = Some(fork.generation);
    }
}

fn truth(world: &World, a: usize, b: usize, meanings: &str) -> Result<Truth, String> {
    let (va, vb) = (speech(world, a)?, speech(world, b)?);
    let rows = concepts(meanings)?
        .into_iter()
        .filter_map(|concept| {
            let (wa, wb) = (va.lexicon.word_for(concept)?, vb.lexicon.word_for(concept)?);
            // One loan fits the contract: prefer a direct loan between the compared
            // varieties, otherwise the first side's record, then the second's.
            let loan = if matches!(wb.origin, Origin::Borrowed { from, .. } if from == a) {
                borrowed(world, b, wb)
            } else {
                borrowed(world, a, wa).or_else(|| borrowed(world, b, wb))
            };
            Some(TruthRow {
                concept: concept.id,
                same_root: world.root_of(a, wa.id) == world.root_of(b, wb.id),
                borrowed: loan,
            })
        })
        .collect();
    let (ancestor, split_at) = ancestor(world, a, b);
    Ok(Truth {
        rows,
        ancestor,
        split_at,
    })
}

#[derive(Serialize)]
struct AppliedLaw {
    id: &'static str,
    label: String,
    generation: u32,
}

#[derive(Serialize)]
struct SpeechSide {
    alive: bool,
    descendants: usize,
    laws: Vec<AppliedLaw>,
    specimen: Vec<SpecimenWord>,
}

#[derive(Serialize)]
struct ChangedWord {
    concept: &'static str,
    gloss: &'static str,
    left: Option<FormView>,
    right: Option<FormView>,
}

#[derive(Serialize)]
struct Loans {
    donor: usize,
    count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LanguageDelta {
    variety: usize,
    left: SpeechSide,
    right: SpeechSide,
    only_left: Vec<&'static str>,
    only_right: Vec<&'static str>,
    changed_words: Vec<ChangedWord>,
    loans_left: Vec<Loans>,
    loans_right: Vec<Loans>,
}

#[derive(Serialize)]
struct SpeechDelta {
    generation: u32,
    diverged: u32,
    decision: Option<DecisionView>,
    languages: Vec<LanguageDelta>,
}

fn speech_sides(world: &World, shared: &World, count: usize) -> Vec<(SpeechSide, Vec<Loans>)> {
    let after = world.generation >= shared.generation;
    let spoken = world.spoken();
    let mut descendants = vec![0; count];
    if after {
        for variety in &world.varieties[shared.varieties.len()..] {
            let mut parent = variety.parent;
            while let Some(fork) = parent {
                if fork.variety < count {
                    descendants[fork.variety] += 1;
                }
                parent = world.varieties[fork.variety].parent;
            }
        }
    }
    (0..count)
        .map(|id| {
            let variety = &world.varieties[id];
            let common = &shared.varieties[id];
            let laws = if after {
                variety
                    .laws
                    .iter()
                    .skip(common.laws.len())
                    .map(|&(generation, id)| AppliedLaw {
                        id,
                        label: world
                            .law_catalog()
                            .iter()
                            .find(|law| law.id == id)
                            .map_or_else(|| substrate_label(id), |law| law.label.to_string()),
                        generation,
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let mut loans = BTreeMap::new();
            if after {
                for word in variety
                    .lexicon
                    .lexemes
                    .iter()
                    .skip(common.lexicon.lexemes.len())
                {
                    if let Origin::Borrowed { from, .. } = word.origin {
                        *loans.entry(from).or_insert(0) += 1;
                    }
                }
            }
            (
                SpeechSide {
                    alive: spoken[id],
                    descendants: descendants[id],
                    laws,
                    specimen: specimen(variety, world.generation),
                },
                loans
                    .into_iter()
                    .map(|(donor, count)| Loans { donor, count })
                    .collect(),
            )
        })
        .collect()
}

fn language_delta(
    variety: usize,
    left: (SpeechSide, Vec<Loans>),
    right: (SpeechSide, Vec<Loans>),
) -> LanguageDelta {
    let (left, loans_left) = left;
    let (right, loans_right) = right;
    let exclusive = |a: &SpeechSide, b: &SpeechSide| {
        let other: HashSet<_> = b.laws.iter().map(|law| law.id).collect();
        let mut seen = HashSet::new();
        a.laws
            .iter()
            .filter_map(|law| (!other.contains(law.id) && seen.insert(law.id)).then_some(law.id))
            .collect()
    };
    let only_left = exclusive(&left, &right);
    let only_right = exclusive(&right, &left);
    let changed_words = SPECIMEN
        .iter()
        .filter_map(|id| {
            let a = left.specimen.iter().find(|word| word.concept == *id);
            let b = right.specimen.iter().find(|word| word.concept == *id);
            if a.map(|word| (&word.spelled, &word.ipa)) == b.map(|word| (&word.spelled, &word.ipa))
            {
                return None;
            }
            let form = |word: &SpecimenWord| FormView {
                spelled: word.spelled.clone(),
                ipa: word.ipa.clone(),
            };
            let concept = by_id(id).expect("specimen concept");
            Some(ChangedWord {
                concept: concept.id,
                gloss: concept.gloss,
                left: a.map(form),
                right: b.map(form),
            })
        })
        .collect();
    LanguageDelta {
        variety,
        left,
        right,
        only_left,
        only_right,
        changed_words,
        loans_left,
        loans_right,
    }
}

impl Bench {
    pub fn correspondences(
        &mut self,
        generation: u32,
        a: usize,
        b: usize,
        meanings: &str,
    ) -> Result<String, String> {
        to_json(&correspondences(self.world(generation), a, b, meanings)?)
    }

    pub fn truth(
        &mut self,
        generation: u32,
        a: usize,
        b: usize,
        meanings: &str,
    ) -> Result<String, String> {
        to_json(&truth(self.world(generation), a, b, meanings)?)
    }

    pub fn meaning_sets(&self) -> Result<String, String> {
        to_json(&meaning_sets())
    }

    pub fn speech_delta(
        &self,
        left: TellingId,
        right: TellingId,
        generation: u32,
    ) -> Result<String, String> {
        let a = self.scope(left, "")?;
        let b = self.scope(right, "")?;
        if generation > a.borrow().latest().min(b.borrow().latest()) {
            return Err("Choose a year already recorded in both tellings.".into());
        }
        let common = self.chronicle.common_reading(left, right)?;
        let shared = self.scope(common.telling, &to_json(&common.point)?)?;
        let mut shared = shared.borrow_mut();
        let shared = shared.world(generation);
        // Read the sides separately: comparing a telling to itself returns the
        // same cached RefCell, which must never be mutably borrowed twice.
        let mut a = a.borrow_mut();
        let world = a.world(generation);
        let count = shared.varieties.len().min(world.varieties.len());
        let left_sides = speech_sides(world, shared, count);
        drop(a);
        let mut b = b.borrow_mut();
        let right_sides = speech_sides(b.world(generation), shared, count);
        let decision = if generation < shared.generation {
            None
        } else {
            // An action at the split year can be the divergent decision. Compare
            // exact action positions rather than throwing that year away. The
            // decision lives on whichever telling branched, so look on the side
            // being read first, then the other.
            let after = common.point.action + usize::from(common.point.offset > 0);
            drop(b);
            [left, right].into_iter().find_map(|side| {
                let scope = self.scope(side, "").ok()?;
                let mut scope = scope.borrow_mut();
                let first = scope
                    .world(generation)
                    .decisions
                    .iter()
                    .find(|d| d.action >= after)
                    .map(|d| d.action)?;
                // Tip reads do not replace an older cached historical world.
                let world = if generation == scope.latest() {
                    scope.chronicle.latest()
                } else {
                    scope
                        .cached
                        .as_ref()
                        .expect("the past reading was cached above")
                };
                scope
                    .decision_views(world)
                    .into_iter()
                    .find(|d| d.index == first)
            })
        };
        to_json(&SpeechDelta {
            generation,
            diverged: shared.generation,
            decision,
            languages: left_sides
                .into_iter()
                .zip(right_sides)
                .enumerate()
                .map(|(id, (left, right))| language_delta(id, left, right))
                .collect(),
        })
    }
}

#[wasm_bindgen]
impl Workbench {
    pub fn correspondences(
        &mut self,
        generation: u32,
        a: f64,
        b: f64,
        meanings: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .correspondences(
                generation,
                entity_id(a).map_err(fail)?,
                entity_id(b).map_err(fail)?,
                meanings,
            )
            .map_err(fail)
    }

    pub fn truth(
        &mut self,
        generation: u32,
        a: f64,
        b: f64,
        meanings: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .truth(
                generation,
                entity_id(a).map_err(fail)?,
                entity_id(b).map_err(fail)?,
                meanings,
            )
            .map_err(fail)
    }

    #[wasm_bindgen(js_name = meaningSets)]
    pub fn meaning_sets(&self) -> Result<String, JsValue> {
        self.bench.meaning_sets().map_err(fail)
    }

    #[wasm_bindgen(js_name = speechDelta)]
    pub fn speech_delta(&self, left: u32, right: u32, generation: u32) -> Result<String, JsValue> {
        self.bench
            .speech_delta(left, right, generation)
            .map_err(fail)
    }
}

#[wasm_bindgen]
impl ReadView {
    pub fn correspondences(
        &mut self,
        generation: u32,
        a: f64,
        b: f64,
        meanings: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .correspondences(
                generation,
                entity_id(a).map_err(fail)?,
                entity_id(b).map_err(fail)?,
                meanings,
            )
            .map_err(fail)
    }

    pub fn truth(
        &mut self,
        generation: u32,
        a: f64,
        b: f64,
        meanings: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .truth(
                generation,
                entity_id(a).map_err(fail)?,
                entity_id(b).map_err(fail)?,
                meanings,
            )
            .map_err(fail)
    }

    #[wasm_bindgen(js_name = meaningSets)]
    pub fn meaning_sets(&self) -> Result<String, JsValue> {
        self.bench.borrow().meaning_sets().map_err(fail)
    }

    #[wasm_bindgen(js_name = speechDelta)]
    pub fn speech_delta(&self, left: u32, right: u32, generation: u32) -> Result<String, JsValue> {
        self.bench
            .borrow()
            .speech_delta(left, right, generation)
            .map_err(fail)
    }
}

#[cfg(test)]
mod tests;

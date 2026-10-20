//! Structured language evidence, read from the selected world's records only.
//!
//! No historical competition weights are stored. Word lives therefore name only
//! witnessed holders (founding, explicit replacements, and the current holder),
//! leaving unknown dominance dates null. Marker shares are the current observed
//! share, not an invented curve; majority_generations is a counter, not dates.
//! Spellings use the reading's orthography, while IPA uses historical stress.
use crate::notebook::Subject;
use crate::provenance::{LoanCauseView, cause_view, entry_variety, event_position};
use crate::{
    AgreementView, Bench, CoinedPartView, CoinedView, ReadView, SPECIMEN, Workbench,
    agreement_view, coined_view, entity_id, fail, form_after, grammar_origin_view, substrate_label,
    to_json,
};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use umran_sim::coinage::CoinageKind;
use umran_sim::concepts::{Concept, Relation, by_id};
use umran_sim::grammar::{Category, GrammarEntry, GrammarEvent, Marker, MarkerOrigin};
use umran_sim::names::Name;
use umran_sim::{
    Event, Form, Law, Lexeme, LexemeId, Origin, StressRule, Variety, World, WorldEvent,
};
use wasm_bindgen::prelude::*;

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct FormView {
    spelled: String,
    ipa: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
struct LawOccurrenceRef {
    variety: usize,
    law: std::borrow::Cow<'static, str>,
    generation: u32,
    index: usize,
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum ChangeCause {
    Law {
        law: std::borrow::Cow<'static, str>,
        label: String,
        occurrence: Option<LawOccurrenceRef>,
    },
    Loan {
        cause: LoanCauseView,
    },
    Coined {
        coined: CoinedView,
    },
    Inherited {
        from: usize,
    },
    Founding,
    Kept {
        from: usize,
    },
    Extended {
        from: &'static str,
    },
    Expressive,
    Marker {
        origin: crate::GrammarOriginView,
    },
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum Arrival {
    Own,
    Wave { from: usize },
    Inherited { ancestor: usize },
    Authored { decision: usize },
    Substrate,
    Unrecorded,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LawOccurrence {
    #[serde(rename = "ref")]
    reference: LawOccurrenceRef,
    /// Position in the requested language's ledger, including inherited entries.
    #[serde(skip)]
    order: usize,
    label: String,
    rules: Vec<String>,
    arrival: Arrival,
    annal: Option<String>,
    words: Vec<LawWord>,
    markers: Vec<LawMarker>,
    names: Vec<LawName>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LawWord {
    concept: &'static str,
    gloss: &'static str,
    before: FormView,
    after: FormView,
    dominant: bool,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct LawMarker {
    category: &'static str,
    marker: String,
    before: FormView,
    after: FormView,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct LawName {
    subject: Subject,
    before: String,
    after: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FormPoint {
    generation: u32,
    form: FormView,
    cause: ChangeCause,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WordLife {
    concept: &'static str,
    gloss: &'static str,
    holders: Vec<Holder>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Holder {
    forms: Vec<FormPoint>,
    entered: u32,
    dominant_from: Option<u32>,
    until: Option<u32>,
    share: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MeaningForm {
    concept: &'static str,
    gloss: &'static str,
    form: FormView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Donor {
    variety: usize,
    form: FormView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoanTrail {
    concept: &'static str,
    donor: Donor,
    cause: LoanCauseView,
    heard: FormView,
    adapted: FormView,
    since: u32,
    after: Vec<FormPoint>,
    same_meeting: Vec<MeaningForm>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GrammarStory {
    category: &'static str,
    label: &'static str,
    markers: Vec<MarkerStory>,
    agreement: Option<AgreementView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MarkerStory {
    marker: String,
    kind: &'static str,
    scope: umran_sim::grammar::MarkerScope,
    source: Option<MeaningForm>,
    born: u32,
    retired: Option<u32>,
    shares: Vec<SharePoint>,
    forms: Vec<GrammarPoint>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SharePoint {
    generation: u32,
    share: f32,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GrammarCause {
    Change(ChangeCause),
    Analogy { kind: &'static str },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GrammarPoint {
    generation: u32,
    form: FormView,
    cause: GrammarCause,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuilderExample {
    builder: &'static str,
    base: MeaningForm,
    built: MeaningForm,
}

fn speech(world: &World, variety: usize) -> Result<&Variety, String> {
    world
        .varieties
        .get(variety)
        .ok_or_else(|| "No such language variety.".into())
}

fn form_view(variety: &Variety, form: &Form, stress: StressRule) -> FormView {
    FormView {
        spelled: variety.spell(form),
        ipa: form.ipa_stressed(stress),
    }
}

fn law_label(world: &World, law: &str) -> String {
    world
        .law_by_id(law)
        .map_or_else(|| substrate_label(law), |l| l.label.to_string())
}

/// Match a stored lexical change to its owner and its ordinal in that owner's
/// log. Unchanged applications have no per-word entry; the engine has no event
/// id with which to distinguish such gaps within a generation.
fn word_occurrence(
    world: &World,
    variety: usize,
    word: &Lexeme,
    at: usize,
) -> Option<LawOccurrenceRef> {
    let entry = &word.log[at];
    let Event::SoundLaw { ref law, .. } = entry.event else {
        unreachable!()
    };
    let owner = entry_variety(world, variety, word, at);
    let observed = word
        .log
        .iter()
        .enumerate()
        .filter(|(i, prior)| {
            prior.generation == entry.generation
                && matches!(&prior.event, Event::SoundLaw { law: id, .. } if id == law)
                && entry_variety(world, variety, word, *i) == owner
        })
        .count();
    if observed != application_count(world, owner, entry.generation, law) {
        return None;
    }
    let index = word.log[..at]
        .iter()
        .enumerate()
        .filter(|(i, prior)| {
            prior.generation == entry.generation
                && matches!(&prior.event, Event::SoundLaw { law: id, .. } if id == law)
                && entry_variety(world, variety, word, *i) == owner
        })
        .count();
    Some(LawOccurrenceRef {
        variety: owner,
        law: law.clone(),
        generation: entry.generation,
        index,
    })
}

/// Lexical evidence resolves daughter-local changes at the fork generation.
/// Where none survives, the inherited prefix of the law ledger is all that is
/// recorded (identical same-year applications cannot always be distinguished).
fn ledger_owner(world: &World, variety: usize, at: usize) -> usize {
    let v = &world.varieties[variety];
    let Some(fork) = v.parent else { return variety };
    let (generation, ref law) = v.laws[at];
    if generation > fork.generation {
        return variety;
    }
    let ordinal = v.laws[..at]
        .iter()
        .filter(|(g, l)| *g == generation && l == law)
        .count();
    if generation == fork.generation {
        let total = v
            .laws
            .iter()
            .filter(|(g, id)| *g == generation && id == law)
            .count();
        for word in &v.lexicon.lexemes {
            if word.id.0 >= fork.inherited {
                continue;
            }
            let observed = word
                .log
                .iter()
                .filter(|entry| {
                    entry.generation == generation
                        && matches!(&entry.event, Event::SoundLaw { law: id, .. } if id == law)
                })
                .count();
            if observed != total {
                continue;
            }
            if let Some((i, _)) = word
                .log
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.generation == generation
                        && matches!(&entry.event, Event::SoundLaw { law: id, .. } if id == law)
                })
                .nth(ordinal)
            {
                return entry_variety(world, variety, word, i);
            }
        }
    }
    let parent = &world.varieties[fork.variety];
    if parent.laws.get(..=at) == Some(&v.laws[..=at]) {
        ledger_owner(world, fork.variety, at)
    } else {
        variety
    }
}

fn application_count(world: &World, variety: usize, generation: u32, law: &str) -> usize {
    world.varieties[variety]
        .laws
        .iter()
        .enumerate()
        .filter(|(at, entry)| {
            entry.0 == generation && entry.1 == law && ledger_owner(world, variety, *at) == variety
        })
        .count()
}

fn stress_transition(world: &World, reference: &LawOccurrenceRef) -> (StressRule, StressRule) {
    let v = &world.varieties[reference.variety];
    let mut stress = v
        .stress_history
        .iter()
        .find(|(g, _)| *g >= reference.generation)
        .map_or(v.stress(), |(_, prior)| *prior);
    let mut index = 0;
    for (at, (generation, id)) in v.laws.iter().cloned().enumerate() {
        if generation != reference.generation {
            continue;
        }
        let prior = stress;
        if let Some(next) = world.law_by_id(&id).and_then(|l| l.stress) {
            stress = next;
        }
        if id == reference.law && ledger_owner(world, reference.variety, at) == reference.variety {
            if index == reference.index {
                return (prior, stress);
            }
            index += 1;
        }
    }
    (stress, stress)
}

fn law_cause(
    world: &World,
    law: std::borrow::Cow<'static, str>,
    occurrence: Option<LawOccurrenceRef>,
) -> ChangeCause {
    ChangeCause::Law {
        label: law_label(world, &law),
        law,
        occurrence,
    }
}

fn after_stress(
    world: &World,
    variety: usize,
    generation: u32,
    law: &str,
    occurrence: Option<&LawOccurrenceRef>,
) -> StressRule {
    occurrence.map_or_else(
        || {
            world
                .law_by_id(law)
                .and_then(|l| l.stress)
                .unwrap_or_else(|| world.varieties[variety].stress_at(generation))
        },
        |reference| stress_transition(world, reference).1,
    )
}

fn initial_stress(world: &World, variety: usize, word: &Lexeme) -> StressRule {
    word.log
        .iter()
        .enumerate()
        .find(|(_, e)| matches!(e.event, Event::SoundLaw { .. }))
        .filter(|(_, e)| e.generation == word.born)
        .and_then(|(i, _)| word_occurrence(world, variety, word, i))
        .map_or_else(
            || world.varieties[variety].stress_at(word.born),
            |reference| stress_transition(world, &reference).0,
        )
}

fn grammar_after<'a>(entries: &'a [GrammarEntry], current: &'a Form, at: usize) -> &'a Form {
    entries[at + 1..]
        .iter()
        .find_map(|entry| match &entry.event {
            GrammarEvent::SoundLaw { before, .. } | GrammarEvent::Analogy { before } => {
                Some(before)
            }
            _ => None,
        })
        .unwrap_or(current)
}

fn marker_owner(world: &World, mut variety: usize, marker: &Marker, at: usize) -> usize {
    while let Some(fork) = world.varieties[variety].parent {
        if marker.history[at].generation > fork.generation {
            break;
        }
        let Some(parent) = world.varieties[fork.variety]
            .grammar
            .markers
            .get(marker.id as usize)
        else {
            break;
        };
        if parent.born != marker.born || parent.history.get(at) != Some(&marker.history[at]) {
            break;
        }
        variety = fork.variety;
    }
    variety
}

fn marker_occurrence(
    world: &World,
    variety: usize,
    marker: &Marker,
    at: usize,
) -> Option<LawOccurrenceRef> {
    let entry = &marker.history[at];
    let GrammarEvent::SoundLaw { ref law, .. } = entry.event else {
        unreachable!()
    };
    let owner = marker_owner(world, variety, marker, at);
    let observed = marker
        .history
        .iter()
        .enumerate()
        .filter(|(i, prior)| {
            prior.generation == entry.generation
                && matches!(&prior.event, GrammarEvent::SoundLaw { law: id, .. } if id == law)
                && marker_owner(world, variety, marker, *i) == owner
        })
        .count();
    if observed != application_count(world, owner, entry.generation, law) {
        return None;
    }
    let index = marker.history[..at]
        .iter()
        .enumerate()
        .filter(|(i, prior)| {
            prior.generation == entry.generation
                && matches!(&prior.event, GrammarEvent::SoundLaw { law: id, .. } if id == law)
                && marker_owner(world, variety, marker, *i) == owner
        })
        .count();
    Some(LawOccurrenceRef {
        variety: owner,
        law: law.clone(),
        generation: entry.generation,
        index,
    })
}

fn law_occurrences(world: &World, variety: usize, law: &str) -> Result<Vec<LawOccurrence>, String> {
    let v = speech(world, variety)?;
    let mut records = BTreeMap::new();
    let mut counts = BTreeMap::new();
    for (at, (generation, id)) in v.laws.iter().cloned().enumerate() {
        if id != law {
            continue;
        }
        let owner = ledger_owner(world, variety, at);
        let index = counts.entry((owner, generation)).or_insert(0);
        let reference = LawOccurrenceRef {
            variety: owner,
            law: id.clone(),
            generation,
            index: *index,
        };
        *index += 1;
        let recorded = &world.varieties[owner];
        let total = application_count(world, owner, generation, &id);
        let arrival = if owner != variety {
            Arrival::Inherited { ancestor: owner }
        } else if let Some(&decision) = world.authored_laws.get(&(owner, generation, id.clone())) {
            if reference.index + 1 == total {
                Arrival::Authored { decision }
            } else {
                Arrival::Unrecorded
            }
        } else if let Some(&(_, _, from)) = recorded
            .waves
            .iter()
            .find(|(g, l, _)| *g == generation && *l == id)
        {
            if total == 1 {
                Arrival::Wave { from }
            } else {
                Arrival::Unrecorded
            }
        } else if id == "substrate" {
            Arrival::Substrate
        } else {
            Arrival::Own
        };
        // This is the same existence rule as annals::sound_changes, without
        // rendering annals (their prose chooses wording using its own RNG).
        let has_annal = recorded
            .parent
            .is_none_or(|fork| generation > fork.generation)
            || world
                .authored_laws
                .contains_key(&(owner, generation, id.clone()));
        records.insert(
            reference.clone(),
            LawOccurrence {
                reference,
                order: at,
                label: law_label(world, &id),
                rules: world
                    .law_by_id(&id)
                    .map_or_else(Vec::new, Law::rule_sources),
                arrival,
                annal: has_annal.then(|| format!("sounds:{owner}:{generation}")),
                words: Vec::new(),
                markers: Vec::new(),
                names: Vec::new(),
            },
        );
    }
    for word in &v.lexicon.lexemes {
        for (i, entry) in word.log.iter().enumerate() {
            let Event::SoundLaw {
                law: ref id,
                ref before,
            } = entry.event
            else {
                continue;
            };
            if id != law {
                continue;
            }
            let Some(reference) = word_occurrence(world, variety, word, i) else {
                continue;
            };
            let Some(record) = records.get_mut(&reference) else {
                continue;
            };
            let (prior, after) = stress_transition(world, &reference);
            // Include historical meanings too, not just surviving slots.
            for concept in word_meanings_at(word, i) {
                record.words.push(LawWord {
                    concept: concept.id,
                    gloss: concept.gloss,
                    before: form_view(&world.varieties[reference.variety], before, prior),
                    after: form_view(
                        &world.varieties[reference.variety],
                        form_after(word, i),
                        after,
                    ),
                    dominant: v.lexicon.slot(concept).dominant() == Some(word.id),
                });
            }
        }
    }
    for marker in &v.grammar.markers {
        for (i, entry) in marker.history.iter().enumerate() {
            let GrammarEvent::SoundLaw {
                law: ref id,
                ref before,
            } = entry.event
            else {
                continue;
            };
            if id != law {
                continue;
            }
            let Some(reference) = marker_occurrence(world, variety, marker, i) else {
                continue;
            };
            let Some(record) = records.get_mut(&reference) else {
                continue;
            };
            let (prior, after) = stress_transition(world, &reference);
            record.markers.push(LawMarker {
                category: marker.category.id(),
                marker: marker.id.to_string(),
                before: form_view(v, before, prior),
                after: form_view(v, grammar_after(&marker.history, &marker.form, i), after),
            });
        }
    }
    // Full inflected realizations also retain changes when the shared marker
    // did not change (for example an eroded stem). They are evidence, not a
    // substitute for the marker's own form series in grammar_story.
    for word in &v.lexicon.lexemes {
        for paradigm in &word.paradigms {
            for realization in &paradigm.realizations {
                let Some(current) = &realization.form else {
                    continue;
                };
                let mut counts = BTreeMap::new();
                for (i, entry) in realization.history.iter().enumerate() {
                    let GrammarEvent::SoundLaw {
                        law: ref id,
                        ref before,
                    } = entry.event
                    else {
                        continue;
                    };
                    if id != law {
                        continue;
                    }
                    let mut owner = variety;
                    while let Some(fork) = world.varieties[owner].parent {
                        if word.id.0 >= fork.inherited || entry.generation > fork.generation {
                            break;
                        }
                        let parent = world.varieties[fork.variety].lexicon.get(word.id);
                        let inherited = parent
                            .paradigms
                            .iter()
                            .find(|p| p.category == paradigm.category)
                            .and_then(|p| {
                                p.realizations
                                    .iter()
                                    .find(|r| r.marker == realization.marker)
                            });
                        if inherited.is_none_or(|r| r.history.get(i) != Some(entry)) {
                            break;
                        }
                        owner = fork.variety;
                    }
                    let index = counts.entry((owner, entry.generation)).or_insert(0);
                    let reference = LawOccurrenceRef {
                        variety: owner,
                        law: id.clone(),
                        generation: entry.generation,
                        index: *index,
                    };
                    *index += 1;
                    let observed = realization.history.iter().filter(|e| {
                        e.generation == entry.generation
                            && matches!(&e.event, GrammarEvent::SoundLaw { law, .. } if law == id)
                    }).count();
                    let ledger = v
                        .laws
                        .iter()
                        .filter(|(g, law)| *g == entry.generation && law == id)
                        .count();
                    if observed != ledger {
                        continue;
                    }
                    let Some(record) = records.get_mut(&reference) else {
                        continue;
                    };
                    let (prior, after) = stress_transition(world, &reference);
                    let evidence = LawMarker {
                        category: paradigm.category.id(),
                        marker: realization.marker.to_string(),
                        before: form_view(v, before, prior),
                        after: form_view(v, grammar_after(&realization.history, current, i), after),
                    };
                    if !record.markers.contains(&evidence) {
                        record.markers.push(evidence);
                    }
                }
            }
        }
    }
    for record in records.values_mut() {
        occurrence_names(world, record);
    }
    let mut out: Vec<_> = records.into_values().collect();
    out.sort_by_key(|r| r.order);
    Ok(out)
}

fn occurrence_names(world: &World, occurrence: &mut LawOccurrence) {
    let reference = &occurrence.reference;
    let v = &world.varieties[reference.variety];
    let total = application_count(
        world,
        reference.variety,
        reference.generation,
        &reference.law,
    );
    let mut add = |subject: Subject, name: &Name| {
        let observed = name
            .log
            .iter()
            .filter(|e| {
                e.generation == reference.generation
                    && matches!(&e.event, Event::SoundLaw { law, .. } if *law == reference.law)
            })
            .count();
        if observed != total {
            return;
        }
        let entry = name
            .log
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.generation == reference.generation
                    && matches!(&e.event, Event::SoundLaw { law, .. } if *law == reference.law)
            })
            .nth(reference.index);
        let Some((i, entry)) = entry else { return };
        let Event::SoundLaw { ref before, .. } = entry.event else {
            return;
        };
        let after = name.log[i + 1..]
            .iter()
            .find_map(|e| match &e.event {
                Event::SoundLaw { before, .. } => Some(before),
                _ => None,
            })
            .unwrap_or(&name.form);
        let evidence = LawName {
            subject,
            before: v.title(before),
            after: v.title(after),
        };
        if !occurrence.names.contains(&evidence) {
            occurrence.names.push(evidence);
        }
    };
    add(
        Subject::Language {
            variety: reference.variety,
        },
        &v.name,
    );
    for (id, community) in world.communities.iter().enumerate() {
        if community.variety == reference.variety {
            add(Subject::People { id }, &community.name);
        }
    }
    for (id, state) in world.states.iter().enumerate() {
        if world.communities[state.rulers].variety == reference.variety {
            add(Subject::State { id }, &state.name);
        }
    }
    for (region, names) in world.places.iter().enumerate() {
        for place in names.iter().filter(|p| p.variety == reference.variety) {
            add(Subject::Land { region }, &place.name);
        }
    }
    for (id, names) in world.river_names.iter().enumerate() {
        for place in names.iter().filter(|p| p.variety == reference.variety) {
            add(Subject::River { id }, &place.name);
        }
    }
    for (id, names) in world.lake_names.iter().enumerate() {
        for place in names.iter().filter(|p| p.variety == reference.variety) {
            add(Subject::Lake { id }, &place.name);
        }
    }
    for (region, name) in &v.exonyms {
        add(Subject::Land { region: *region }, name);
    }
    for (id, name) in &v.river_exonyms {
        add(Subject::River { id: *id }, name);
    }
    for (id, name) in &v.lake_exonyms {
        add(Subject::Lake { id: *id }, name);
    }
}

fn word_meanings_at(word: &Lexeme, at: usize) -> Vec<&'static Concept> {
    let mut meanings = vec![word.first_sense];
    for entry in &word.log[..at] {
        match entry.event {
            Event::Extended { to } if !meanings.iter().any(|c| c.id == to.id) => meanings.push(to),
            Event::Lost { sense } => meanings.retain(|c| c.id != sense.id),
            _ => {}
        }
    }
    meanings
}

fn origin_cause(world: &World, variety: usize, word: &Lexeme) -> Option<ChangeCause> {
    let v = &world.varieties[variety];
    match word.origin {
        Origin::Founding => Some(ChangeCause::Founding),
        Origin::Borrowed {
            from,
            cause: umran_sim::LoanCause::Shift { .. },
            ..
        } => Some(ChangeCause::Kept { from }),
        Origin::Borrowed { cause, .. } => Some(ChangeCause::Loan {
            cause: cause_view(world, cause, word.born),
        }),
        Origin::Derived { .. } | Origin::Compound { .. } | Origin::Renewed { .. } => {
            let coined = coined_view(v, word).or_else(|| {
                let (bases, kind) = match word.origin {
                    Origin::Derived { base, .. } | Origin::Renewed { base, with: None } => {
                        (vec![base], CoinageKind::Derived)
                    }
                    Origin::Compound { modifier, head } => {
                        (vec![modifier, head], CoinageKind::Compound)
                    }
                    Origin::Renewed {
                        base,
                        with: Some(other),
                    } => (vec![base, other], CoinageKind::Compound),
                    _ => return None,
                };
                Some(CoinedView {
                    parts: bases
                        .into_iter()
                        .map(|base| {
                            let base = v.lexicon.get(base);
                            CoinedPartView {
                                concept: base.first_sense.id,
                                gloss: base.first_sense.gloss,
                                spelled: v.spell(base.form_at(word.born)),
                            }
                        })
                        .collect(),
                    kind,
                    generation: word.born,
                    from: None,
                    opaque_since: None,
                })
            })?;
            Some(ChangeCause::Coined { coined })
        }
        Origin::Expressive => Some(ChangeCause::Expressive),
    }
}

fn word_forms(world: &World, variety: usize, word: &Lexeme) -> Vec<FormPoint> {
    let v = &world.varieties[variety];
    let mut out = Vec::new();
    if let Some(cause) = origin_cause(world, variety, word) {
        let initial = word
            .log
            .iter()
            .find_map(|e| match &e.event {
                Event::SoundLaw { before, .. } => Some(before),
                _ => None,
            })
            .unwrap_or(&word.form);
        out.push(FormPoint {
            generation: word.born,
            form: form_view(v, initial, initial_stress(world, variety, word)),
            cause,
        });
    }
    for (i, entry) in word.log.iter().enumerate() {
        match entry.event {
            Event::SoundLaw { ref law, .. } => {
                let occurrence = word_occurrence(world, variety, word, i);
                let owner = entry_variety(world, variety, word, i);
                let stress = after_stress(world, owner, entry.generation, law, occurrence.as_ref());
                out.push(FormPoint {
                    generation: entry.generation,
                    form: form_view(&world.varieties[owner], form_after(word, i), stress),
                    cause: law_cause(world, law.clone(), occurrence),
                });
            }
            Event::Extended { .. } => out.push(FormPoint {
                generation: entry.generation,
                form: form_view(v, form_after(word, i), v.stress_at(entry.generation)),
                // The log records the new sense, not which intermediate sense
                // motivated it. `from` is the word's recorded first meaning.
                cause: ChangeCause::Extended {
                    from: word.first_sense.id,
                },
            }),
            _ => {}
        }
    }
    let mut descendant = variety;
    while let Some(fork) = world.varieties[descendant].parent {
        if word.id.0 >= fork.inherited {
            break;
        }
        let inherited = world.varieties[fork.variety].lexicon.get(word.id);
        let first_local = word
            .log
            .iter()
            .enumerate()
            .find(|(i, e)| e.generation >= fork.generation && inherited.log.get(*i) != Some(*e))
            .map_or(word.log.len(), |(i, _)| i);
        let at_fork = word.log[first_local..]
            .iter()
            .find_map(|e| match &e.event {
                Event::SoundLaw { before, .. } => Some(before),
                _ => None,
            })
            .unwrap_or(&word.form);
        let point = FormPoint {
            generation: fork.generation,
            form: form_view(
                &world.varieties[descendant],
                at_fork,
                world.varieties[descendant].stress_at(fork.generation),
            ),
            cause: ChangeCause::Inherited { from: fork.variety },
        };
        // Inheritance is before daughter-local changes, even within one year.
        let insert = out.iter().position(|p| p.generation > fork.generation
            || (p.generation == fork.generation && matches!(&p.cause, ChangeCause::Law { occurrence: Some(occurrence), .. } if occurrence.variety == descendant)))
            .unwrap_or(out.len());
        out.insert(insert, point);
        descendant = fork.variety;
    }
    out
}

fn word_life(world: &World, variety: usize, concept: &'static Concept) -> Result<WordLife, String> {
    let v = speech(world, variety)?;
    let current = v.lexicon.slot(concept).dominant();
    // The id accompanies a witnessed holder only inside this reader. It is not
    // exposed as an unstable cross-language lexical identity.
    let mut witnessed: Vec<(LexemeId, Option<u32>, Option<u32>)> = Vec::new();
    if let Some(word) = v.lexicon.lexemes.iter().find(|word| {
        word.born == 0
            && word.first_sense.id == concept.id
            && matches!(word.origin, Origin::Founding | Origin::Derived { .. })
    }) {
        witnessed.push((word.id, Some(0), None));
    }
    let mut replacements = Vec::new();
    for &(generation, ref event) in &world.events {
        if let WorldEvent::TabooReplaced {
            variety: owner,
            concept: sense,
            old,
            word,
            ..
        } = *event
        {
            if sense.id != concept.id {
                continue;
            }
            let mut inherited = variety;
            loop {
                if inherited == owner {
                    replacements.push((generation, old, word));
                    break;
                }
                let Some(fork) = world.varieties[inherited].parent else {
                    break;
                };
                if generation > fork.generation
                    || old.0 >= fork.inherited
                    || word.0 >= fork.inherited
                {
                    break;
                }
                inherited = fork.variety;
            }
        }
    }
    for episode in &v.purism {
        for replacement in episode.replaced.iter().filter(|r| r.concept == concept.id) {
            replacements.push((episode.since, replacement.loan, replacement.native));
        }
    }
    replacements.sort_by_key(|r| r.0);
    for (generation, old, new) in replacements {
        if let Some(holder) = witnessed.last_mut().filter(|h| h.0 == old) {
            holder.2 = Some(generation);
        } else {
            witnessed.push((old, None, Some(generation)));
        }
        witnessed.push((new, Some(generation), None));
    }
    if let Some(current) = current
        && witnessed.last().is_none_or(|h| h.0 != current)
    {
        witnessed.push((current, None, None));
    }
    let holders = witnessed
        .into_iter()
        .map(|(id, dominant_from, until)| {
            let word = v.lexicon.get(id);
            let until = if Some(id) == current && until.is_none() {
                None
            } else {
                until.or_else(|| {
                    word.log.iter().find_map(|e| {
                        (dominant_from.is_none_or(|from| e.generation >= from)
                            && matches!(e.event, Event::Lost { sense } if sense.id == concept.id))
                        .then_some(e.generation)
                    })
                })
            };
            let share = if until.is_some() {
                0.0
            } else {
                v.lexicon
                    .slot(concept)
                    .variants
                    .iter()
                    .find(|variant| variant.lexeme == id)
                    .map_or(0.0, |variant| variant.weight)
            };
            let forms = word_forms(world, variety, word)
                .into_iter()
                .filter(|point| until.is_none_or(|end| point.generation <= end))
                .collect();
            Holder {
                forms,
                entered: word.born,
                dominant_from,
                until,
                share,
            }
        })
        .collect();
    Ok(WordLife {
        concept: concept.id,
        gloss: concept.gloss,
        holders,
    })
}

fn loan_trail(
    world: &World,
    variety: usize,
    concept: &'static Concept,
) -> Result<Option<LoanTrail>, String> {
    let v = speech(world, variety)?;
    let Some(word) = v
        .lexicon
        .slot(concept)
        .variants
        .iter()
        .filter(|variant| {
            matches!(
                v.lexicon.get(variant.lexeme).origin,
                Origin::Borrowed { .. }
            )
        })
        .max_by(|a, b| a.weight.total_cmp(&b.weight))
        .map(|variant| v.lexicon.get(variant.lexeme))
    else {
        return Ok(None);
    };
    let Origin::Borrowed { from, cause, .. } = word.origin else {
        unreachable!()
    };
    let Some((borrowed, entry)) = word
        .log
        .iter()
        .enumerate()
        .find(|(_, e)| matches!(e.event, Event::Borrowed { .. }))
    else {
        // Old records lacking the heard form cannot support an adaptation strip.
        return Ok(None);
    };
    let Event::Borrowed { ref source, .. } = entry.event else {
        unreachable!()
    };
    let donor = &world.varieties[from];
    let meeting = event_position(world, cause, word.born);
    let mut same_meeting = Vec::new();
    if let Some(meeting) = meeting {
        let mut seen = HashSet::new();
        for other in &v.lexicon.lexemes {
            if other.id == word.id || other.first_sense.id == concept.id {
                continue;
            }
            if let Origin::Borrowed { cause, .. } = other.origin
                && event_position(world, cause, other.born) == Some(meeting)
                && seen.insert(other.first_sense.id)
            {
                same_meeting.push(MeaningForm {
                    concept: other.first_sense.id,
                    gloss: other.first_sense.gloss,
                    form: form_view(
                        v,
                        &other.form,
                        v.stress_at(other.obsolete.unwrap_or(world.generation)),
                    ),
                });
            }
        }
    }
    let after = word_forms(world, variety, word)
        .into_iter()
        .filter(|point| {
            !matches!(
                point.cause,
                ChangeCause::Loan { .. } | ChangeCause::Kept { .. }
            ) && point.generation >= entry.generation
        })
        .collect();
    Ok(Some(LoanTrail {
        concept: concept.id,
        donor: Donor {
            variety: from,
            form: form_view(donor, source, donor.stress_at(entry.generation)),
        },
        cause: cause_view(world, cause, word.born),
        heard: form_view(donor, source, donor.stress_at(entry.generation)),
        adapted: form_view(
            v,
            form_after(word, borrowed),
            initial_stress(world, variety, word),
        ),
        since: entry.generation,
        after,
        same_meeting,
    }))
}

fn grammar_story(
    world: &World,
    variety: usize,
    category: Category,
) -> Result<GrammarStory, String> {
    let v = speech(world, variety)?;
    let mut markers = Vec::new();
    for marker in v.grammar.markers.iter().filter(|m| m.category == category) {
        let source = match &marker.origin {
            MarkerOrigin::Grammaticalized {
                concept,
                source_form,
                ..
            } => Some(MeaningForm {
                concept: concept.id,
                gloss: concept.gloss,
                form: form_view(v, source_form, v.stress_at(marker.born)),
            }),
            _ => None,
        };
        let mut forms = Vec::new();
        for (i, entry) in marker.history.iter().enumerate() {
            let (cause, stress) = match entry.event {
                GrammarEvent::SoundLaw { ref law, .. } => {
                    let occurrence = marker_occurrence(world, variety, marker, i);
                    let owner = marker_owner(world, variety, marker, i);
                    let stress =
                        after_stress(world, owner, entry.generation, law, occurrence.as_ref());
                    (
                        GrammarCause::Change(law_cause(world, law.clone(), occurrence)),
                        stress,
                    )
                }
                GrammarEvent::Analogy { .. } => (
                    GrammarCause::Analogy { kind: "analogy" },
                    v.stress_at(entry.generation),
                ),
                GrammarEvent::Created | GrammarEvent::Imported { .. } => (
                    GrammarCause::Change(ChangeCause::Marker {
                        origin: grammar_origin_view(world, variety, marker),
                    }),
                    v.stress_at(entry.generation),
                ),
                GrammarEvent::Retired => continue,
            };
            forms.push(GrammarPoint {
                generation: entry.generation,
                form: form_view(v, grammar_after(&marker.history, &marker.form, i), stress),
                cause,
            });
        }
        let mut shares = Vec::new();
        if let Some(retired) = marker.retired {
            shares.push(SharePoint {
                generation: retired,
                share: 0.0,
            });
        } else {
            shares.push(SharePoint {
                generation: world.generation,
                share: v.grammar.marker_share(marker.id),
            });
        }
        markers.push(MarkerStory {
            marker: marker.id.to_string(),
            kind: marker.kind.id(),
            scope: marker.scope,
            source,
            born: marker.born,
            retired: marker.retired,
            shares,
            forms,
        });
    }
    markers.sort_by_key(|m| m.born);
    Ok(GrammarStory {
        category: category.id(),
        label: category.label(),
        markers,
        agreement: (category == Category::Genitive)
            .then(|| agreement_view(v, v.grammar.possessor_agreement(), world.generation)),
    })
}

fn builder_examples(world: &World, variety: usize) -> Result<Vec<BuilderExample>, String> {
    let v = speech(world, variety)?;
    let mut examples = Vec::new();
    let mut seen = HashSet::new();
    for word in &v.lexicon.lexemes {
        let (builder, base) = match word.origin {
            Origin::Derived { base, relation } => (relation.label(), base),
            Origin::Renewed { base, with: None } => ("renewing", base),
            _ => continue,
        };
        if !seen.insert(builder) {
            continue;
        }
        let base = v.lexicon.get(base);
        let base_form = word
            .coined
            .as_ref()
            .and_then(|c| c.parts.first())
            .map_or_else(|| base.form_at(word.born), |p| &p.form);
        let built_form = word
            .log
            .iter()
            .find_map(|e| match &e.event {
                Event::SoundLaw { before, .. } => Some(before),
                _ => None,
            })
            .unwrap_or(&word.form);
        examples.push(BuilderExample {
            builder,
            base: MeaningForm {
                concept: base.first_sense.id,
                gloss: base.first_sense.gloss,
                form: form_view(v, base_form, v.stress_at(word.born)),
            },
            built: MeaningForm {
                concept: word.first_sense.id,
                gloss: word.first_sense.gloss,
                form: form_view(v, built_form, initial_stress(world, variety, word)),
            },
        });
    }
    examples.sort_by_key(|e| {
        Relation::ALL
            .iter()
            .position(|r| r.label() == e.builder)
            .unwrap_or(Relation::ALL.len())
    });
    Ok(examples)
}

impl Bench {
    pub fn law_occurrences(
        &mut self,
        generation: u32,
        variety: usize,
        law: &str,
    ) -> Result<String, String> {
        to_json(&law_occurrences(self.world(generation), variety, law)?)
    }
    pub fn specimen_lives(&mut self, generation: u32, variety: usize) -> Result<String, String> {
        let world = self.world(generation);
        let lives: Result<Vec<_>, _> = SPECIMEN
            .iter()
            .map(|id| word_life(world, variety, by_id(id).expect("specimen concept")))
            .collect();
        to_json(&lives?)
    }
    pub fn word_life(
        &mut self,
        generation: u32,
        variety: usize,
        concept: &str,
    ) -> Result<String, String> {
        let concept = by_id(concept).ok_or("Unknown concept.")?;
        to_json(&word_life(self.world(generation), variety, concept)?)
    }
    pub fn loan_trail(
        &mut self,
        generation: u32,
        variety: usize,
        concept: &str,
    ) -> Result<String, String> {
        let concept = by_id(concept).ok_or("Unknown concept.")?;
        to_json(&loan_trail(self.world(generation), variety, concept)?)
    }
    pub fn grammar_story(
        &mut self,
        generation: u32,
        variety: usize,
        category: &str,
    ) -> Result<String, String> {
        let category = Category::ALL
            .into_iter()
            .find(|c| c.id() == category)
            .ok_or("Unknown grammatical category.")?;
        to_json(&grammar_story(self.world(generation), variety, category)?)
    }
    pub fn builder_examples(&mut self, generation: u32, variety: usize) -> Result<String, String> {
        to_json(&builder_examples(self.world(generation), variety)?)
    }
}

#[wasm_bindgen]
impl Workbench {
    #[wasm_bindgen(js_name = lawOccurrences)]
    pub fn law_occurrences(
        &mut self,
        generation: u32,
        variety: f64,
        law: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .law_occurrences(generation, entity_id(variety).map_err(fail)?, law)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = specimenLives)]
    pub fn specimen_lives(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .specimen_lives(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = wordLife)]
    pub fn word_life(
        &mut self,
        generation: u32,
        variety: f64,
        concept: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .word_life(generation, entity_id(variety).map_err(fail)?, concept)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = loanTrail)]
    pub fn loan_trail(
        &mut self,
        generation: u32,
        variety: f64,
        concept: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .loan_trail(generation, entity_id(variety).map_err(fail)?, concept)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = grammarStory)]
    pub fn grammar_story(
        &mut self,
        generation: u32,
        variety: f64,
        category: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .grammar_story(generation, entity_id(variety).map_err(fail)?, category)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = builderExamples)]
    pub fn builder_examples(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .builder_examples(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
}

#[wasm_bindgen]
impl ReadView {
    #[wasm_bindgen(js_name = lawOccurrences)]
    pub fn law_occurrences(
        &mut self,
        generation: u32,
        variety: f64,
        law: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .law_occurrences(generation, entity_id(variety).map_err(fail)?, law)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = specimenLives)]
    pub fn specimen_lives(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .specimen_lives(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = wordLife)]
    pub fn word_life(
        &mut self,
        generation: u32,
        variety: f64,
        concept: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .word_life(generation, entity_id(variety).map_err(fail)?, concept)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = loanTrail)]
    pub fn loan_trail(
        &mut self,
        generation: u32,
        variety: f64,
        concept: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .loan_trail(generation, entity_id(variety).map_err(fail)?, concept)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = grammarStory)]
    pub fn grammar_story(
        &mut self,
        generation: u32,
        variety: f64,
        category: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .grammar_story(generation, entity_id(variety).map_err(fail)?, category)
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = builderExamples)]
    pub fn builder_examples(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .builder_examples(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use umran_sim::{Entry, HistoryPoint, LoanCause, Params, SoundProfile};

    fn world() -> World {
        World::solo(5, &SoundProfile::base(), Params::static_society())
    }

    fn form(ipa: &str) -> Form {
        Form::from_ipa(ipa).unwrap()
    }

    // Supplied records test the facade join, independently of a catalog law's
    // eligibility, random selection, and linguistic mechanics.
    fn repeated_world() -> World {
        let mut world = world();
        world.generation = 2;
        let v = &mut world.varieties[0];
        let id = v.lexicon.word_for(by_id("water").unwrap()).unwrap().id;
        let word = v.lexicon.get_mut(id);
        word.form = form("kta");
        word.log = vec![
            Entry {
                generation: 2,
                event: Event::SoundLaw {
                    law: std::borrow::Cow::Borrowed("fixture-law"),
                    before: form("katata"),
                },
            },
            Entry {
                generation: 2,
                event: Event::SoundLaw {
                    law: std::borrow::Cow::Borrowed("fixture-law"),
                    before: form("katta"),
                },
            },
        ];
        v.laws = vec![(2, "fixture-law".into()), (2, "fixture-law".into())];
        world
    }

    #[test]
    fn repeated_law_in_one_generation_has_two_occurrences() {
        let world = repeated_world();
        let rows = law_occurrences(&world, 0, "fixture-law").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].reference.index, 0);
        assert_eq!(rows[1].reference.index, 1);
        let first = rows[0].words.iter().find(|w| w.concept == "water").unwrap();
        let second = rows[1].words.iter().find(|w| w.concept == "water").unwrap();
        assert_eq!(first.after.ipa, second.before.ipa);
        assert_ne!(first.before.ipa, first.after.ipa);
        assert_ne!(second.before.ipa, second.after.ipa);
    }

    #[test]
    fn inherited_occurrence_belongs_to_ancestor() {
        let mut world = repeated_world();
        let daughter = world.varieties[0].fork(0, 3);
        world.varieties.push(daughter);
        world.generation = 3;
        let rows = law_occurrences(&world, 1, "fixture-law").unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.reference.variety == 0));
        assert!(
            rows.iter()
                .all(|row| matches!(row.arrival, Arrival::Inherited { ancestor: 0 }))
        );
        assert!(
            rows.iter()
                .all(|row| row.annal.as_deref() == Some("sounds:0:2"))
        );
    }

    #[test]
    fn fork_generation_keeps_daughter_application_local() {
        let mut world = repeated_world();
        let mut daughter = world.varieties[0].fork(0, 2);
        let id = daughter
            .lexicon
            .word_for(by_id("water").unwrap())
            .unwrap()
            .id;
        let word = daughter.lexicon.get_mut(id);
        word.log.push(Entry {
            generation: 2,
            event: Event::SoundLaw {
                law: std::borrow::Cow::Borrowed("fixture-law"),
                before: word.form.clone(),
            },
        });
        word.form = form("ka");
        daughter.laws.push((2, "fixture-law".into()));
        world.varieties.push(daughter);
        let rows = law_occurrences(&world, 1, "fixture-law").unwrap();
        assert_eq!(
            rows.iter()
                .map(|r| (r.reference.variety, r.reference.index))
                .collect::<Vec<_>>(),
            vec![(0, 0), (0, 1), (1, 0)]
        );
        let own = rows.iter().find(|row| row.reference.variety == 1).unwrap();
        assert_eq!(own.reference.index, 0);
        assert_eq!(own.words.iter().filter(|w| w.concept == "water").count(), 1);
        assert_eq!(
            rows.iter().filter(|row| row.reference.variety == 0).count(),
            2
        );
    }

    #[test]
    fn repeated_authorship_does_not_copy_latest_decision_to_earlier_change() {
        let mut world = repeated_world();
        world.authored_laws.insert((0, 2, "fixture-law".into()), 7);
        let rows = law_occurrences(&world, 0, "fixture-law").unwrap();
        assert!(matches!(rows[0].arrival, Arrival::Unrecorded));
        assert!(matches!(rows[1].arrival, Arrival::Authored { decision: 7 }));
    }

    #[test]
    fn loan_trail_keeps_later_laws_and_not_unrecorded_meetings() {
        let mut world = world();
        world.varieties.push(world.varieties[0].clone());
        world.generation = 4;
        let concept = by_id("water").unwrap();
        let source = world.varieties[1].lexicon.word_for(concept).unwrap().id;
        let cause = LoanCause::Unrecorded;
        let v = &mut world.varieties[0];
        let id = v.lexicon.coin(
            form("ta"),
            Origin::Borrowed {
                from: 1,
                source,
                cause,
            },
            concept,
            2,
        );
        v.lexicon.get_mut(id).log = vec![
            Entry {
                generation: 2,
                event: Event::Borrowed {
                    from: 1,
                    source: form("tata"),
                    cause,
                },
            },
            Entry {
                generation: 3,
                event: Event::SoundLaw {
                    law: std::borrow::Cow::Borrowed("fixture-law"),
                    before: form("tata"),
                },
            },
        ];
        v.laws.push((3, "fixture-law".into()));
        v.lexicon.slot_mut(concept).variants.clear();
        v.lexicon.slot_mut(concept).introduce(id, 1.0);
        let trail = loan_trail(&world, 0, concept).unwrap().unwrap();
        assert_eq!(trail.since, 2);
        assert_eq!(trail.after.len(), 1);
        assert_eq!(trail.after[0].generation, 3);
        assert!(matches!(
            trail.after[0].cause,
            ChangeCause::Law {
                law: std::borrow::Cow::Borrowed("fixture-law"),
                ..
            }
        ));
        assert_ne!(trail.adapted.ipa, trail.after[0].form.ipa);
        assert!(trail.same_meeting.is_empty());
    }

    #[test]
    fn recorded_replacement_shows_two_holders() {
        let mut world = world();
        world.generation = 3;
        let concept = by_id("water").unwrap();
        let v = &mut world.varieties[0];
        let old = v.lexicon.word_for(concept).unwrap().id;
        let new = v.lexicon.coin(
            form("tataka"),
            Origin::Renewed {
                base: old,
                with: None,
            },
            concept,
            3,
        );
        v.lexicon.get_mut(old).log.push(Entry {
            generation: 3,
            event: Event::Lost { sense: concept },
        });
        v.lexicon.slot_mut(concept).variants.clear();
        v.lexicon.slot_mut(concept).introduce(new, 1.0);
        world.events.push((
            3,
            WorldEvent::TabooReplaced {
                religion: 0,
                community: 0,
                variety: 0,
                concept,
                old,
                word: new,
            },
        ));
        let life = word_life(&world, 0, concept).unwrap();
        assert_eq!(life.holders.len(), 2);
        assert_eq!(life.holders[0].dominant_from, Some(0));
        assert_eq!(life.holders[0].until, Some(3));
        assert_eq!(life.holders[0].share, 0.0);
        assert_eq!(life.holders[1].entered, 3);
        assert_eq!(life.holders[1].dominant_from, Some(3));
        assert_eq!(life.holders[1].until, None);
        assert_eq!(life.holders[1].share, 1.0);
    }

    #[test]
    fn ordinary_competition_does_not_invent_a_dominance_date() {
        let mut world = world();
        world.generation = 8;
        let concept = by_id("water").unwrap();
        let lexicon = &mut world.varieties[0].lexicon;
        let new = lexicon.coin(form("tataka"), Origin::Expressive, concept, 2);
        lexicon.slot_mut(concept).introduce(new, 0.9);
        let life = word_life(&world, 0, concept).unwrap();
        assert_eq!(life.holders.len(), 2);
        assert_eq!(life.holders[1].entered, 2);
        assert_eq!(life.holders[1].dominant_from, None);
        assert!(matches!(
            life.holders[1].forms[0].cause,
            ChangeCause::Expressive
        ));
    }

    #[test]
    fn no_op_gaps_keep_forms_without_guessing_an_occurrence() {
        let mut world = repeated_world();
        let id = world.varieties[0]
            .lexicon
            .word_for(by_id("water").unwrap())
            .unwrap()
            .id;
        world.varieties[0].lexicon.get_mut(id).log.remove(0);
        let rows = law_occurrences(&world, 0, "fixture-law").unwrap();
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .all(|r| r.words.iter().all(|w| w.concept != "water"))
        );
        let life = word_life(&world, 0, by_id("water").unwrap()).unwrap();
        assert!(life.holders[0].forms.iter().any(|p| matches!(
            p.cause,
            ChangeCause::Law {
                law: std::borrow::Cow::Borrowed("fixture-law"),
                occurrence: None,
                ..
            }
        )));
    }

    #[test]
    fn grammar_story_uses_marker_origins_and_only_recorded_shares() {
        let mut world = world();
        world.generation = 6;
        let v = &mut world.varieties[0];
        let concept = by_id("water").unwrap();
        let source = v.lexicon.word_for(concept).unwrap().id;
        let marker = &mut v.grammar.markers[0];
        marker.born = 2;
        marker.retired = Some(4);
        marker.origin = MarkerOrigin::Grammaticalized {
            source,
            concept,
            source_form: form("tata"),
        };
        marker.form = form("a");
        marker.history = vec![
            GrammarEntry {
                generation: 2,
                event: GrammarEvent::Created,
            },
            GrammarEntry {
                generation: 3,
                event: GrammarEvent::SoundLaw {
                    law: std::borrow::Cow::Borrowed("fixture-law"),
                    before: form("ta"),
                },
            },
            GrammarEntry {
                generation: 4,
                event: GrammarEvent::Retired,
            },
        ];
        let id = marker.id.to_string();
        let category = marker.category;
        v.laws.push((3, "fixture-law".into()));
        let story = grammar_story(&world, 0, category).unwrap();
        let marker = story.markers.iter().find(|m| m.marker == id).unwrap();
        assert_eq!(marker.source.as_ref().unwrap().concept, "water");
        assert_eq!(marker.shares.len(), 1);
        assert_eq!(marker.shares[0].generation, 4);
        assert_eq!(marker.shares[0].share, 0.0);
        assert_eq!(marker.forms.len(), 2);
        assert!(matches!(
            marker.forms[0].cause,
            GrammarCause::Change(ChangeCause::Marker { .. })
        ));
        assert!(matches!(
            marker.forms[1].cause,
            GrammarCause::Change(ChangeCause::Law {
                occurrence: Some(_),
                ..
            })
        ));
    }

    #[test]
    fn builder_example_uses_recorded_parts_before_later_erosion() {
        let mut world = world();
        world.generation = 5;
        let v = &mut world.varieties[0];
        let concept = by_id("water").unwrap();
        let base = v.lexicon.word_for(concept).unwrap().id;
        let id = v
            .lexicon
            .coin(form("ta"), Origin::Renewed { base, with: None }, concept, 3);
        let word = v.lexicon.get_mut(id);
        word.coined = Some(umran_sim::coinage::Coinage {
            parts: vec![umran_sim::coinage::Part {
                concept,
                form: form("kaka"),
            }],
            kind: CoinageKind::Derived,
            generation: 3,
            from: None,
            opaque_since: None,
        });
        word.log.push(Entry {
            generation: 4,
            event: Event::SoundLaw {
                law: std::borrow::Cow::Borrowed("fixture-law"),
                before: form("tataka"),
            },
        });
        v.laws.push((4, "fixture-law".into()));
        let examples = builder_examples(&world, 0).unwrap();
        let example = examples.iter().find(|e| e.builder == "renewing").unwrap();
        assert_eq!(
            example.base.form.spelled,
            world.varieties[0].spell(&form("kaka"))
        );
        assert_eq!(
            example.built.form.spelled,
            world.varieties[0].spell(&form("tataka"))
        );
        let unique: HashSet<_> = examples.iter().map(|e| e.builder).collect();
        assert_eq!(unique.len(), examples.len());
    }

    #[test]
    fn all_six_reads_leave_world_and_save_digests_unchanged() {
        let mut bench = Bench::new(5, "small").unwrap();
        bench.fixed = Some((HistoryPoint::default(), repeated_world()));
        let before_world = umran_sim::rng::key(&format!("{:?}", bench.fixed.as_ref().unwrap().1));
        let before_save = umran_sim::rng::key(&bench.save().unwrap());
        let mutation = bench.mutation;
        let occurrences: serde_json::Value =
            serde_json::from_str(&bench.law_occurrences(2, 0, "fixture-law").unwrap()).unwrap();
        assert_eq!(occurrences[1]["ref"]["index"], 1);
        let lives: serde_json::Value =
            serde_json::from_str(&bench.specimen_lives(2, 0).unwrap()).unwrap();
        assert_eq!(lives.as_array().unwrap().len(), SPECIMEN.len());
        assert!(lives[0]["holders"][0].get("dominantFrom").is_some());
        bench.word_life(2, 0, "water").unwrap();
        bench.loan_trail(2, 0, "water").unwrap();
        bench.grammar_story(2, 0, "plural").unwrap();
        bench.builder_examples(2, 0).unwrap();
        assert_eq!(
            before_world,
            umran_sim::rng::key(&format!("{:?}", bench.fixed.as_ref().unwrap().1))
        );
        assert_eq!(before_save, umran_sim::rng::key(&bench.save().unwrap()));
        assert_eq!(bench.mutation, mutation);
    }
}

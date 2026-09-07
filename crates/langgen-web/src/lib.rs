use langgen_core::linguistics::{LexicalClass, SemanticFrame, StressPattern};
use langgen_core::{
    Aesthetic, CATALOG, Community, FoundLanguage, History, HistoryCheckpoint, HistoryEffect,
    HistoryEvent, Lexeme, LexemeRef, Origin, PhonemeId, SemanticOperation, Syllable, Variety, Word,
    WordTrace, rule_detail, rule_label,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Workbench {
    history: History,
}

#[wasm_bindgen]
impl Workbench {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, aesthetic: &str, empty: bool) -> Result<Workbench, JsValue> {
        let aesthetic = Aesthetic::by_id(aesthetic)
            .ok_or_else(|| fail(format!("Unknown aesthetic '{aesthetic}'.")))?;
        let history = if empty {
            History::new(u64::from(seed))
        } else {
            History::coastal_scenario(u64::from(seed), aesthetic).map_err(fail)?
        };
        Ok(Workbench { history })
    }

    pub fn latest(&self) -> u32 {
        self.history
            .checkpoints()
            .last()
            .map_or(0, |cp| cp.id as u32)
    }

    pub fn snapshot(&self, checkpoint: u32) -> Result<String, JsValue> {
        to_json(&present(&self.history, Some(checkpoint), None).map_err(fail)?)
    }

    pub fn options(
        &self,
        checkpoint: u32,
        variety: u32,
        base: &str,
        sense: u32,
    ) -> Result<String, JsValue> {
        let options = self
            .history
            .formations(checkpoint as usize, u64::from(variety), base, sense)
            .map_err(fail)?;
        let view = checkpoint_at(&self.history, checkpoint).map_err(fail)?;
        let variety = view
            .varieties
            .iter()
            .find(|item| item.id == u64::from(variety))
            .ok_or_else(|| fail("The selected language variety is absent."))?;
        let options: Vec<_> = options
            .iter()
            .map(|option| BrowserFormationOption {
                construction: &option.construction,
                label: &option.label,
                gloss: &option.gloss,
                form: variety.form(&option.word),
                ipa: word_ipa(&option.word, option.stress),
                base: &option.base,
                sense: option.sense,
                exponent: variety.form(&option.exponent),
                existing: option.existing.as_deref(),
            })
            .collect();
        to_json(&options)
    }

    pub fn preview(&self, event_json: &str) -> Result<String, JsValue> {
        let projected = self
            .history
            .preview(read_event(event_json)?)
            .map_err(fail)?;
        to_json(&present(&self.history, None, Some(&projected)).map_err(fail)?)
    }

    pub fn commit(&mut self, event_json: &str) -> Result<String, JsValue> {
        self.history.commit(read_event(event_json)?).map_err(fail)?;
        self.snapshot(self.latest())
    }

    pub fn save(&self) -> Result<String, JsValue> {
        self.history.to_json().map_err(fail)
    }

    pub fn load(&mut self, json: &str) -> Result<(), JsValue> {
        let history = History::from_json(json).map_err(fail)?;
        constrain_history(&history).map_err(fail)?;
        self.history = history;
        Ok(())
    }
}

#[derive(Serialize)]
struct BrowserSnapshot<'a> {
    seed: u32,
    checkpoint: u32,
    latest: u32,
    summary: &'a str,
    event: &'a Option<HistoryEvent>,
    checkpoints: Vec<BrowserTimelineEntry<'a>>,
    communities: &'a [Community],
    varieties: Vec<BrowserVariety<'a>>,
    effects: &'a [HistoryEffect],
    rules: Vec<BrowserRule<'a>>,
}

#[derive(Serialize)]
struct BrowserTimelineEntry<'a> {
    id: u32,
    summary: &'a str,
    event: &'a Option<HistoryEvent>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserVariety<'a> {
    id: u32,
    name: &'a str,
    parent: Option<u32>,
    founded_at: u32,
    aesthetic: BrowserAesthetic<'a>,
    inventory: BrowserInventory,
    classes: &'a [LexicalClass],
    stress: &'a StressPattern,
    constructions: Vec<BrowserConstruction<'a>>,
    lexicon: Vec<BrowserLexeme<'a>>,
}

#[derive(Serialize)]
struct BrowserAesthetic<'a> {
    id: &'a str,
    name: &'a str,
    description: &'a str,
}

#[derive(Serialize)]
struct BrowserInventory {
    consonants: Vec<&'static str>,
    vowels: Vec<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserConstruction<'a> {
    id: &'a str,
    label: &'a str,
    input_class: &'a str,
    output_class: &'a str,
    operation: String,
    exponents: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserLexeme<'a> {
    id: &'a str,
    class_id: &'a str,
    class_label: &'a str,
    senses: Vec<BrowserSense<'a>>,
    form: String,
    ipa: String,
    stress: usize,
    boundaries: &'a [usize],
    analysis: Option<BrowserAnalysis<'a>>,
    origin: BrowserOrigin<'a>,
    retired: bool,
    traces: &'a [WordTrace],
}

#[derive(Serialize)]
struct BrowserSense<'a> {
    id: u32,
    gloss: String,
    frame: &'a SemanticFrame,
}

#[derive(Serialize)]
struct BrowserAnalysis<'a> {
    base: &'a str,
    sense: u32,
    construction: &'a str,
    label: &'a str,
}

#[derive(Serialize)]
struct BrowserOrigin<'a> {
    kind: &'static str,
    label: String,
    source: Option<&'a LexemeRef>,
    construction: Option<&'a str>,
    sense: Option<u32>,
    checkpoint: u32,
}

#[derive(Serialize)]
struct BrowserRule<'a> {
    id: &'a str,
    label: String,
    detail: String,
}

#[derive(Serialize)]
struct BrowserFormationOption<'a> {
    construction: &'a str,
    label: &'a str,
    gloss: &'a str,
    form: String,
    ipa: String,
    base: &'a str,
    sense: u32,
    exponent: String,
    existing: Option<&'a str>,
}

fn present<'a>(
    history: &'a History,
    selected: Option<u32>,
    projected: Option<&'a HistoryCheckpoint>,
) -> Result<BrowserSnapshot<'a>, String> {
    let accepted = history.checkpoints();
    let accepted_latest = accepted.last().ok_or("This history has no checkpoints.")?;
    let view = match projected {
        Some(view) => view,
        None => checkpoint_at(history, selected.ok_or("No checkpoint was selected.")?)?,
    };
    constrain_checkpoint(view)?;
    let mut timeline = accepted
        .iter()
        .map(timeline_entry)
        .collect::<Result<Vec<_>, _>>()?;
    if projected.is_some() {
        timeline.push(timeline_entry(view)?);
    }
    Ok(BrowserSnapshot {
        seed: fit_u64(history.seed())?,
        checkpoint: fit_usize(view.id)?,
        latest: fit_usize(if projected.is_some() {
            view.id
        } else {
            accepted_latest.id
        })?,
        summary: &view.summary,
        event: &view.event,
        checkpoints: timeline,
        communities: &view.communities,
        varieties: view
            .varieties
            .iter()
            .map(|variety| browser_variety(history, view, variety))
            .collect::<Result<_, _>>()?,
        effects: &view.effects,
        rules: view
            .rules
            .iter()
            .map(|rule| BrowserRule {
                id: &rule.id,
                label: rule_label(&rule.id),
                detail: rule_detail(&rule.id),
            })
            .collect(),
    })
}

fn timeline_entry(checkpoint: &HistoryCheckpoint) -> Result<BrowserTimelineEntry<'_>, String> {
    Ok(BrowserTimelineEntry {
        id: fit_usize(checkpoint.id)?,
        summary: &checkpoint.summary,
        event: &checkpoint.event,
    })
}

fn browser_variety<'a>(
    history: &'a History,
    view: &'a HistoryCheckpoint,
    variety: &'a Variety,
) -> Result<BrowserVariety<'a>, String> {
    Ok(BrowserVariety {
        id: fit_u64(variety.id)?,
        name: &variety.name,
        parent: variety.parent.map(fit_u64).transpose()?,
        founded_at: fit_usize(variety.founded_at)?,
        aesthetic: BrowserAesthetic {
            id: &variety.aesthetic.id,
            name: &variety.aesthetic.name,
            description: &variety.aesthetic.description,
        },
        inventory: BrowserInventory {
            consonants: ipa_list(&variety.consonants),
            vowels: ipa_list(&variety.vowels),
        },
        classes: &variety.grammar.classes,
        stress: &variety.grammar.stress,
        constructions: variety
            .grammar
            .constructions
            .iter()
            .map(|construction| BrowserConstruction {
                id: &construction.id,
                label: &construction.label,
                input_class: &construction.input_class,
                output_class: &construction.output_class,
                operation: match &construction.operation {
                    SemanticOperation::Participant(role) => {
                        format!("Names an event's {role:?} participant")
                    }
                    SemanticOperation::PlaceOf => "Names a place associated with an event".into(),
                    SemanticOperation::Collective => "Names a group of countable entities".into(),
                },
                exponents: construction
                    .exponents()
                    .iter()
                    .map(|word| variety.form(word))
                    .collect(),
            })
            .collect(),
        lexicon: variety
            .lexicon
            .iter()
            .map(|lexeme| browser_lexeme(history, view, variety, lexeme))
            .collect::<Result<_, _>>()?,
    })
}

fn browser_lexeme<'a>(
    history: &'a History,
    view: &'a HistoryCheckpoint,
    variety: &'a Variety,
    lexeme: &'a Lexeme,
) -> Result<BrowserLexeme<'a>, String> {
    let class = variety
        .grammar
        .classes
        .iter()
        .find(|class| class.id == lexeme.class)
        .ok_or("A word refers to an absent lexical class.")?;
    let analysis = lexeme
        .analysis
        .as_ref()
        .map(|analysis| {
            let construction = variety
                .grammar
                .constructions
                .iter()
                .find(|item| item.id == analysis.construction)
                .ok_or("A word refers to an absent construction.")?;
            Ok::<_, String>(BrowserAnalysis {
                base: &analysis.base,
                sense: analysis.sense,
                construction: &analysis.construction,
                label: &construction.label,
            })
        })
        .transpose()?;
    Ok(BrowserLexeme {
        id: &lexeme.id,
        class_id: &lexeme.class,
        class_label: &class.label,
        senses: lexeme
            .senses
            .iter()
            .map(|sense| BrowserSense {
                id: sense.id,
                gloss: sense.meaning.label(),
                frame: &sense.frame,
            })
            .collect(),
        form: variety.form(&lexeme.word),
        ipa: word_ipa(&lexeme.word, lexeme.stress),
        stress: lexeme.stress,
        boundaries: &lexeme.boundaries,
        analysis,
        origin: browser_origin(history, view, &lexeme.origin)?,
        retired: lexeme.retired,
        traces: &lexeme.traces,
    })
}

fn browser_origin<'a>(
    history: &'a History,
    view: &'a HistoryCheckpoint,
    origin: &'a Origin,
) -> Result<BrowserOrigin<'a>, String> {
    let (kind, source, construction, sense, checkpoint) = match origin {
        Origin::Unrecorded { checkpoint, .. } => ("Unrecorded", None, None, None, *checkpoint),
        Origin::Formed {
            source,
            sense,
            construction,
            checkpoint,
        } => (
            "Formed",
            Some(source),
            Some(construction.as_str()),
            Some(*sense),
            *checkpoint,
        ),
        Origin::Borrowed { source, checkpoint } => {
            ("Borrowed", Some(source), None, None, *checkpoint)
        }
        Origin::Inherited { source, checkpoint } => {
            ("Inherited", Some(source), None, None, *checkpoint)
        }
    };
    let label = if let Some(source) = source {
        let source_view = history
            .checkpoints()
            .get(source.checkpoint)
            .or_else(|| (view.id == source.checkpoint).then_some(view))
            .ok_or("A historical source checkpoint is absent.")?;
        let source_variety = source_view
            .varieties
            .iter()
            .find(|item| item.id == source.variety)
            .ok_or("A historical source language is absent.")?;
        let source_word = source_variety
            .lexicon
            .iter()
            .find(|item| item.id == source.lexeme)
            .ok_or("A historical source word is absent.")?;
        let source_meaning = source_word
            .senses
            .iter()
            .find(|item| sense.is_none_or(|id| item.id == id))
            .ok_or("A historical source sense is absent.")?
            .meaning
            .label();
        let source_form = source_variety.form(&source_word.word);
        match construction {
            Some(id) => {
                let construction = source_variety
                    .grammar
                    .constructions
                    .iter()
                    .find(|item| item.id == id)
                    .ok_or("A historical source construction is absent.")?;
                format!(
                    "Formed with {} from {source_form} ‘{source_meaning}’ in {}.",
                    construction.label, source_variety.name
                )
            }
            None => format!(
                "{kind} from {source_form} ‘{source_meaning}’ in {}.",
                source_variety.name
            ),
        }
    } else {
        "Earlier history unrecorded; not a claim that the word was coined here.".into()
    };
    Ok(BrowserOrigin {
        kind,
        label,
        source,
        construction,
        sense,
        checkpoint: fit_usize(checkpoint)?,
    })
}

fn checkpoint_at(history: &History, id: u32) -> Result<&HistoryCheckpoint, String> {
    history
        .checkpoints()
        .get(id as usize)
        .ok_or_else(|| format!("There is no checkpoint {id} in this history."))
}

fn read_event(event_json: &str) -> Result<HistoryEvent, JsValue> {
    let event = serde_json::from_str(event_json)
        .map_err(|err| fail(format!("Could not read that event: {err}")))?;
    constrain_event(&event).map_err(fail)?;
    Ok(event)
}

fn constrain_history(history: &History) -> Result<(), String> {
    fit_u64(history.seed())?;
    for checkpoint in history.checkpoints() {
        constrain_checkpoint(checkpoint)?;
    }
    Ok(())
}

fn constrain_checkpoint(checkpoint: &HistoryCheckpoint) -> Result<(), String> {
    fit_usize(checkpoint.id)?;
    if let Some(event) = &checkpoint.event {
        constrain_event(event)?;
    }
    for community in &checkpoint.communities {
        fit_u64(community.id)?;
        for usage in &community.uses {
            fit_u64(usage.variety)?;
        }
    }
    for variety in &checkpoint.varieties {
        fit_u64(variety.id)?;
        if let Some(parent) = variety.parent {
            fit_u64(parent)?;
        }
        fit_usize(variety.founded_at)?;
        for lexeme in &variety.lexicon {
            fit_usize(lexeme.stress)?;
            for boundary in &lexeme.boundaries {
                fit_usize(*boundary)?;
            }
            match &lexeme.origin {
                Origin::Unrecorded {
                    variety,
                    checkpoint,
                } => {
                    fit_u64(*variety)?;
                    fit_usize(*checkpoint)?;
                }
                Origin::Formed {
                    source, checkpoint, ..
                }
                | Origin::Borrowed { source, checkpoint }
                | Origin::Inherited { source, checkpoint } => {
                    constrain_source(source)?;
                    fit_usize(*checkpoint)?;
                }
            }
            for trace in &lexeme.traces {
                fit_usize(trace.checkpoint)?;
                fit_u64(trace.variety)?;
            }
        }
    }
    for effect in &checkpoint.effects {
        fit_u64(effect.variety)?;
    }
    Ok(())
}

fn constrain_source(source: &LexemeRef) -> Result<(), String> {
    fit_u64(source.variety)?;
    fit_usize(source.checkpoint)?;
    Ok(())
}

fn constrain_event(event: &HistoryEvent) -> Result<(), String> {
    match event {
        HistoryEvent::Found {
            language: FoundLanguage::New { .. },
            ..
        } => {}
        HistoryEvent::Found {
            language: FoundLanguage::Existing { variety },
            ..
        }
        | HistoryEvent::Derive { variety, .. }
        | HistoryEvent::ExtendSense { variety, .. }
        | HistoryEvent::ShiftSense { variety, .. }
        | HistoryEvent::Replace { variety, .. }
        | HistoryEvent::Lexicalize { variety, .. }
        | HistoryEvent::Remodel { variety, .. } => {
            fit_u64(*variety)?;
        }
        HistoryEvent::UseLanguage {
            community, variety, ..
        } => {
            fit_u64(*community)?;
            fit_u64(*variety)?;
        }
        HistoryEvent::Separate {
            parent, community, ..
        } => {
            fit_u64(*parent)?;
            fit_u64(*community)?;
        }
        HistoryEvent::Develop { variety, steps } => {
            fit_u64(*variety)?;
            fit_usize(*steps)?;
        }
        HistoryEvent::Contact {
            donor,
            recipient,
            count,
            ..
        } => {
            fit_u64(*donor)?;
            fit_u64(*recipient)?;
            fit_usize(*count)?;
        }
    }
    Ok(())
}

fn word_ipa(word: &Word, stress: usize) -> String {
    let mut ipa = String::new();
    for (index, syllable) in word.syllables.iter().enumerate() {
        if index > 0 {
            ipa.push('.');
        }
        if index == stress {
            ipa.push('ˈ');
        }
        append_syllable_ipa(&mut ipa, syllable);
    }
    ipa
}

fn append_syllable_ipa(ipa: &mut String, syllable: &Syllable) {
    for id in syllable.onset.iter().chain(&syllable.nucleus) {
        ipa.push_str(CATALOG.get(*id).ipa());
    }
    if syllable.long {
        ipa.push('ː');
    }
    for id in &syllable.coda {
        ipa.push_str(CATALOG.get(*id).ipa());
    }
}

fn ipa_list(ids: &[PhonemeId]) -> Vec<&'static str> {
    ids.iter().map(|id| CATALOG.get(*id).ipa()).collect()
}

fn to_json<T: Serialize>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(|err| fail(format!("Could not prepare this view: {err}")))
}

fn fit_u64(value: u64) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| too_big())
}

fn fit_usize(value: usize) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| too_big())
}

fn too_big() -> String {
    "This history has an identifier the browser cannot represent.".into()
}

fn fail(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipa_keeps_stress_and_vowel_length_on_their_syllables() {
        let phone = |ipa| CATALOG.id_by_ipa(ipa).unwrap();
        let word = Word {
            syllables: vec![
                Syllable {
                    onset: vec![phone("t")],
                    nucleus: vec![phone("i")],
                    coda: vec![phone("n")],
                    long: true,
                },
                Syllable {
                    onset: vec![phone("k")],
                    nucleus: vec![phone("a")],
                    coda: vec![],
                    long: false,
                },
            ],
            join_at: None,
        };
        assert_eq!(word_ipa(&word, 1), "tiːn.ˈka");
    }
}

use langgen_core::{
    Aesthetic, CATALOG, Community, FormationKind, FormationOption, History, HistoryCheckpoint,
    HistoryEvent, HistoryLexeme, PhonemeId, Syllable, Word, rule_detail, rule_label,
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
        if Aesthetic::by_id(aesthetic).is_none() {
            return Err(fail(format!("Unknown aesthetic '{aesthetic}'.")));
        }
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
            .and_then(|cp| u32::try_from(cp.id).ok())
            .unwrap_or(0)
    }

    pub fn snapshot(&self, checkpoint: u32) -> Result<String, JsValue> {
        let view = present(&self.history, Some(checkpoint), None).map_err(fail)?;
        to_json(&view)
    }

    pub fn options(&self, checkpoint: u32, community: u32, base: &str) -> Result<String, JsValue> {
        let checkpoint_id = usize::try_from(checkpoint).map_err(|_| fail(too_big()))?;
        let options = self
            .history
            .formations(checkpoint_id, u64::from(community), base)
            .map_err(fail)?;
        let comm = community_at(&self.history, checkpoint_id, u64::from(community))?;
        let view: Vec<BrowserFormationOption> = options
            .iter()
            .map(|option| browser_option(comm, option))
            .collect();
        to_json(&view)
    }

    pub fn preview(&self, event_json: &str) -> Result<String, JsValue> {
        let event = read_event(event_json)?;
        let projected = self.history.preview(event).map_err(fail)?;
        let view = present(&self.history, None, Some(&projected)).map_err(fail)?;
        to_json(&view)
    }

    pub fn commit(&mut self, event_json: &str) -> Result<String, JsValue> {
        let event = read_event(event_json)?;
        self.history.commit(event).map_err(fail)?;
        let latest = self.latest();
        let view = present(&self.history, Some(latest), None).map_err(fail)?;
        to_json(&view)
    }

    pub fn save(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.history)
            .map_err(|err| fail(format!("Could not save this history: {err}")))
    }

    pub fn load(&mut self, json: &str) -> Result<(), JsValue> {
        let history: History = serde_json::from_str(json)
            .map_err(|err| fail(format!("Could not load this history: {err}")))?;
        constrain_history(&history).map_err(fail)?;
        history.validate().map_err(fail)?;
        self.history = history;
        Ok(())
    }
}

#[derive(Serialize)]
struct BrowserSnapshot {
    seed: u32,
    checkpoint: u32,
    latest: u32,
    summary: String,
    event: Option<HistoryEvent>,
    checkpoints: Vec<BrowserTimelineEntry>,
    communities: Vec<BrowserCommunity>,
    effects: Vec<BrowserEffect>,
    rules: Vec<BrowserRule>,
}

#[derive(Serialize)]
struct BrowserTimelineEntry {
    id: u32,
    summary: String,
    event: Option<HistoryEvent>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserCommunity {
    id: u32,
    name: String,
    parent: Option<u32>,
    founded_at: u32,
    aesthetic: BrowserAesthetic,
    inventory: BrowserInventory,
    lexicon: Vec<BrowserLexeme>,
    formations: Vec<BrowserFormation>,
}

#[derive(Serialize)]
struct BrowserAesthetic {
    id: String,
    name: String,
    description: String,
}

#[derive(Serialize)]
struct BrowserInventory {
    consonants: Vec<String>,
    vowels: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BrowserLexeme {
    id: String,
    gloss: String,
    form: String,
    ipa: String,
    origin_community: u32,
    origin_lexeme: String,
    base_lexeme: Option<String>,
    formation: Option<FormationKind>,
    traces: Vec<BrowserTrace>,
}

#[derive(Serialize)]
struct BrowserTrace {
    checkpoint: u32,
    community: u32,
    before: Option<String>,
    after: String,
    explanation: String,
}

#[derive(Serialize)]
struct BrowserEffect {
    community: u32,
    lexeme: String,
    gloss: String,
    before: Option<String>,
    after: String,
    explanation: String,
}

#[derive(Serialize)]
struct BrowserRule {
    id: String,
    label: String,
    detail: String,
}

#[derive(Serialize)]
struct BrowserFormation {
    kind: FormationKind,
    label: String,
    form: String,
    ipa: String,
}

#[derive(Serialize)]
struct BrowserFormationOption {
    kind: FormationKind,
    label: String,
    gloss: String,
    form: String,
    ipa: String,
    base: String,
    exponent: String,
    existing: Option<String>,
}

fn present(
    history: &History,
    selected: Option<u32>,
    projected: Option<&HistoryCheckpoint>,
) -> Result<BrowserSnapshot, String> {
    if let Some(checkpoint) = projected {
        constrain_checkpoint(checkpoint)?;
    }

    let accepted = history.checkpoints();
    let accepted_latest = accepted
        .last()
        .ok_or_else(|| "This history has no checkpoints.".to_string())?;

    let (view, latest, timeline) = if let Some(projected) = projected {
        let mut timeline: Vec<BrowserTimelineEntry> = accepted
            .iter()
            .map(timeline_entry)
            .collect::<Result<_, _>>()?;
        timeline.push(timeline_entry(projected)?);
        let latest = fit_usize(projected.id)?;
        (projected, latest, timeline)
    } else {
        let selected = selected.ok_or_else(|| "No checkpoint was selected.".to_string())?;
        let view = checkpoint_at(history, selected)?;
        let latest = fit_usize(accepted_latest.id)?;
        let timeline = accepted
            .iter()
            .map(timeline_entry)
            .collect::<Result<_, _>>()?;
        (view, latest, timeline)
    };

    Ok(BrowserSnapshot {
        seed: fit_u64(history.seed())?,
        checkpoint: fit_usize(view.id)?,
        latest,
        summary: view.summary.clone(),
        event: view.event.clone(),
        checkpoints: timeline,
        communities: view
            .communities
            .iter()
            .map(browser_community)
            .collect::<Result<_, _>>()?,
        effects: view
            .effects
            .iter()
            .map(|effect| {
                Ok(BrowserEffect {
                    community: fit_u64(effect.community)?,
                    lexeme: effect.lexeme.clone(),
                    gloss: effect.gloss.clone(),
                    before: effect.before.clone(),
                    after: effect.after.clone(),
                    explanation: effect.explanation.clone(),
                })
            })
            .collect::<Result<_, String>>()?,
        rules: view
            .rules
            .iter()
            .map(|rule| BrowserRule {
                id: rule.id.clone(),
                label: rule_label(&rule.id),
                detail: rule_detail(&rule.id),
            })
            .collect(),
    })
}

fn timeline_entry(checkpoint: &HistoryCheckpoint) -> Result<BrowserTimelineEntry, String> {
    Ok(BrowserTimelineEntry {
        id: fit_usize(checkpoint.id)?,
        summary: checkpoint.summary.clone(),
        event: checkpoint.event.clone(),
    })
}

fn browser_community(community: &Community) -> Result<BrowserCommunity, String> {
    Ok(BrowserCommunity {
        id: fit_u64(community.id)?,
        name: community.name.clone(),
        parent: community.parent.map(fit_u64).transpose()?,
        founded_at: fit_usize(community.founded_at)?,
        aesthetic: BrowserAesthetic {
            id: community.aesthetic.id.clone(),
            name: community.aesthetic.name.clone(),
            description: community.aesthetic.description.clone(),
        },
        inventory: BrowserInventory {
            consonants: ipa_list(&community.consonants),
            vowels: ipa_list(&community.vowels),
        },
        lexicon: community
            .lexicon
            .iter()
            .map(|lexeme| browser_lexeme(community, lexeme))
            .collect::<Result<_, _>>()?,
        formations: community
            .formations
            .iter()
            .map(|rule| BrowserFormation {
                kind: rule.kind,
                label: rule.kind.label().to_string(),
                form: community.form(&rule.exponent),
                ipa: word_ipa(&rule.exponent),
            })
            .collect(),
    })
}

fn browser_lexeme(community: &Community, lexeme: &HistoryLexeme) -> Result<BrowserLexeme, String> {
    Ok(BrowserLexeme {
        id: lexeme.id.clone(),
        gloss: lexeme.gloss.clone(),
        form: community.form(&lexeme.word),
        ipa: word_ipa(&lexeme.word),
        origin_community: fit_u64(lexeme.origin_community)?,
        origin_lexeme: lexeme.origin_lexeme.clone(),
        base_lexeme: lexeme.base_lexeme.clone(),
        formation: lexeme.formation,
        traces: lexeme
            .traces
            .iter()
            .map(|trace| {
                Ok(BrowserTrace {
                    checkpoint: fit_usize(trace.checkpoint)?,
                    community: fit_u64(trace.community)?,
                    before: trace.before.clone(),
                    after: trace.after.clone(),
                    explanation: trace.explanation.clone(),
                })
            })
            .collect::<Result<_, String>>()?,
    })
}

fn browser_option(community: &Community, option: &FormationOption) -> BrowserFormationOption {
    BrowserFormationOption {
        kind: option.kind,
        label: option.label.clone(),
        gloss: option.gloss.clone(),
        form: community.form(&option.word),
        ipa: word_ipa(&option.word),
        base: option.base.clone(),
        exponent: community.form(&option.exponent),
        existing: option.existing.clone(),
    }
}

fn checkpoint_at(history: &History, id: u32) -> Result<&HistoryCheckpoint, String> {
    history
        .checkpoints()
        .get(id as usize)
        .ok_or_else(|| format!("There is no checkpoint {id} in this history."))
}

fn community_at(
    history: &History,
    checkpoint: usize,
    community: u64,
) -> Result<&Community, JsValue> {
    let view = history
        .checkpoints()
        .iter()
        .find(|cp| cp.id == checkpoint)
        .ok_or_else(|| {
            fail(format!(
                "There is no checkpoint {checkpoint} in this history."
            ))
        })?;
    view.communities
        .iter()
        .find(|comm| comm.id == community)
        .ok_or_else(|| {
            fail(format!(
                "There is no community {community} at checkpoint {checkpoint}."
            ))
        })
}

fn read_event(event_json: &str) -> Result<HistoryEvent, JsValue> {
    let event: HistoryEvent = serde_json::from_str(event_json)
        .map_err(|err| fail(format!("Could not read that event: {err}")))?;
    constrain_event(&event).map_err(fail)?;
    Ok(event)
}

fn constrain_history(history: &History) -> Result<(), String> {
    if history.seed() > u64::from(u32::MAX) {
        return Err("This history seed is too large to open in the browser.".into());
    }
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
        if let Some(parent) = community.parent {
            fit_u64(parent)?;
        }
        fit_usize(community.founded_at)?;
        for lexeme in &community.lexicon {
            fit_u64(lexeme.origin_community)?;
            for trace in &lexeme.traces {
                fit_usize(trace.checkpoint)?;
                fit_u64(trace.community)?;
            }
        }
    }
    for effect in &checkpoint.effects {
        fit_u64(effect.community)?;
    }
    Ok(())
}

fn constrain_event(event: &HistoryEvent) -> Result<(), String> {
    match event {
        HistoryEvent::Found { .. } => Ok(()),
        HistoryEvent::Separate { parent, .. } => fit_u64(*parent).map(|_| ()),
        HistoryEvent::Develop {
            community, steps, ..
        } => {
            fit_u64(*community)?;
            fit_usize(*steps)?;
            Ok(())
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
            Ok(())
        }
        HistoryEvent::Derive { community, .. } => fit_u64(*community).map(|_| ()),
    }
}

fn word_ipa(word: &Word) -> String {
    word.syllables
        .iter()
        .map(syllable_ipa)
        .collect::<Vec<_>>()
        .join(".")
}

fn syllable_ipa(syllable: &Syllable) -> String {
    let mut ipa = String::new();
    for id in syllable.onset.iter().chain(&syllable.nucleus) {
        ipa.push_str(CATALOG.get(*id).ipa());
    }
    if syllable.long {
        ipa.push('ː');
    }
    for id in &syllable.coda {
        ipa.push_str(CATALOG.get(*id).ipa());
    }
    ipa
}

fn ipa_list(ids: &[PhonemeId]) -> Vec<String> {
    ids.iter()
        .map(|id| CATALOG.get(*id).ipa().to_string())
        .collect()
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
    fn ipa_marks_vowel_length_before_the_coda() {
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
        assert_eq!(word_ipa(&word), "tiːn.ka");
    }
}

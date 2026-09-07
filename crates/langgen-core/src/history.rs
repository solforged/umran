use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::Language;
use crate::Word;
use crate::aesthetic::Aesthetic;
use crate::change::SoundChange;
use crate::change_packs::{conservative, radical};
use crate::contact::nativize_word;
use crate::inventory::Inventory;
use crate::linguistics::{
    Analysis, ClassKind, FormationOption, Grammar, Lexeme, LexemeRef, LexicalClass, Meaning,
    Origin, SemanticFrame, Sense, WordTrace, derive_lexeme, evolve_language, formation_options,
    language_rule_applies, remodel_lexeme, seed_language, validate_language,
};
use crate::ortho;
use crate::phoneme::{CATALOG, PhonemeId};

const HISTORY_VERSION: u32 = 3;
const MAX_LABEL: usize = 80;

const MARITIME_GLOSSES: &[&str] = &[
    "sea", "water", "river", "wind", "sky", "path", "food", "fish", "fishing",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactDomain {
    General,
    Maritime,
}

impl ContactDomain {
    pub fn label(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Maritime => "maritime",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseDomain {
    Home,
    Trade,
    Ritual,
}

impl UseDomain {
    fn label(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Trade => "trade",
            Self::Ritual => "ritual",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageUse {
    pub variety: u64,
    pub domain: UseDomain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundLanguage {
    New { name: String, aesthetic: String },
    Existing { variety: u64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HistoryEvent {
    Found {
        name: String,
        group: String,
        location: String,
        ancestry: String,
        language: FoundLanguage,
    },
    UseLanguage {
        community: u64,
        variety: u64,
        domain: UseDomain,
    },
    Separate {
        parent: u64,
        name: String,
        community: u64,
    },
    Develop {
        variety: u64,
        steps: usize,
    },
    Contact {
        donor: u64,
        recipient: u64,
        domain: ContactDomain,
        count: usize,
    },
    Derive {
        variety: u64,
        base: String,
        sense: u32,
        construction: String,
    },
    ExtendSense {
        variety: u64,
        lexeme: String,
        gloss: String,
        frame: SemanticFrame,
    },
    ShiftSense {
        variety: u64,
        lexeme: String,
        sense: u32,
        gloss: String,
        frame: SemanticFrame,
    },
    Replace {
        variety: u64,
        lexeme: String,
        replacement: String,
    },
    Lexicalize {
        variety: u64,
        lexeme: String,
    },
    Remodel {
        variety: u64,
        lexeme: String,
        base: String,
        sense: u32,
        construction: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryEffect {
    pub variety: u64,
    pub lexeme: String,
    pub gloss: String,
    pub before: Option<String>,
    pub after: String,
    pub explanation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Community {
    pub id: u64,
    pub name: String,
    pub group: String,
    pub location: String,
    pub ancestry: String,
    pub uses: Vec<LanguageUse>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variety {
    pub id: u64,
    pub name: String,
    pub parent: Option<u64>,
    pub founded_at: usize,
    pub aesthetic: Aesthetic,
    pub consonants: Vec<PhonemeId>,
    pub vowels: Vec<PhonemeId>,
    pub grammar: Grammar,
    pub lexicon: Vec<Lexeme>,
}

impl Variety {
    pub fn form(&self, word: &Word) -> String {
        ortho::romanize(word, &self.aesthetic)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryCheckpoint {
    pub id: usize,
    pub event: Option<HistoryEvent>,
    pub summary: String,
    pub communities: Vec<Community>,
    pub varieties: Vec<Variety>,
    pub effects: Vec<HistoryEffect>,
    pub rules: Vec<SoundChange>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct History {
    seed: u64,
    version: u32,
    checkpoints: Vec<HistoryCheckpoint>,
}

impl History {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            version: HISTORY_VERSION,
            checkpoints: vec![beginning()],
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn checkpoints(&self) -> &[HistoryCheckpoint] {
        &self.checkpoints
    }

    pub fn formations(
        &self,
        checkpoint: usize,
        variety: u64,
        base: &str,
        sense: u32,
    ) -> Result<Vec<FormationOption>, String> {
        let cp = self
            .checkpoints
            .get(checkpoint)
            .ok_or_else(|| format!("unknown checkpoint {checkpoint}"))?;
        let variety = cp
            .varieties
            .iter()
            .find(|item| item.id == variety)
            .ok_or_else(|| format!("unknown variety {variety}"))?;
        formation_options(&variety.grammar, &variety.lexicon, base, sense)
    }

    pub fn preview(&self, event: HistoryEvent) -> Result<HistoryCheckpoint, String> {
        let latest = self.latest()?;
        let mut communities = latest.communities.clone();
        let mut varieties = latest.varieties.clone();
        let mut effects = Vec::new();
        let mut rules = Vec::new();
        let checkpoint = latest
            .id
            .checked_add(1)
            .ok_or_else(|| "checkpoint overflow".to_string())?;
        let summary = match &event {
            HistoryEvent::Found {
                name,
                group,
                location,
                ancestry,
                language,
            } => apply_found(
                &mut communities,
                &mut varieties,
                &mut effects,
                self.seed,
                checkpoint,
                name,
                group,
                location,
                ancestry,
                language,
            )?,
            HistoryEvent::UseLanguage {
                community,
                variety,
                domain,
            } => apply_use_language(&mut communities, &varieties, *community, *variety, *domain)?,
            HistoryEvent::Separate {
                parent,
                name,
                community,
            } => apply_separate(
                &mut communities,
                &mut varieties,
                checkpoint,
                *parent,
                name,
                *community,
            )?,
            HistoryEvent::Develop { variety, steps } => apply_develop(
                &mut varieties,
                &mut effects,
                &mut rules,
                self.seed,
                checkpoint,
                *variety,
                *steps,
            )?,
            HistoryEvent::Contact {
                donor,
                recipient,
                domain,
                count,
            } => apply_contact(
                &mut varieties,
                &mut effects,
                checkpoint,
                *donor,
                *recipient,
                *domain,
                *count,
            )?,
            HistoryEvent::Derive {
                variety,
                base,
                sense,
                construction,
            } => apply_derive(
                &mut varieties,
                &mut effects,
                checkpoint,
                *variety,
                base,
                *sense,
                construction,
            )?,
            HistoryEvent::ExtendSense {
                variety,
                lexeme,
                gloss,
                frame,
            } => apply_extend_sense(
                &mut varieties,
                &mut effects,
                checkpoint,
                *variety,
                lexeme,
                gloss,
                frame.clone(),
            )?,
            HistoryEvent::ShiftSense {
                variety,
                lexeme,
                sense,
                gloss,
                frame,
            } => apply_shift_sense(
                &mut varieties,
                &mut effects,
                checkpoint,
                *variety,
                lexeme,
                *sense,
                gloss,
                frame.clone(),
            )?,
            HistoryEvent::Replace {
                variety,
                lexeme,
                replacement,
            } => apply_replace(
                &mut varieties,
                &mut effects,
                checkpoint,
                *variety,
                lexeme,
                replacement,
            )?,
            HistoryEvent::Lexicalize { variety, lexeme } => {
                apply_lexicalize(&mut varieties, &mut effects, checkpoint, *variety, lexeme)?
            }
            HistoryEvent::Remodel {
                variety,
                lexeme,
                base,
                sense,
                construction,
            } => apply_remodel(
                &mut varieties,
                &mut effects,
                checkpoint,
                *variety,
                lexeme,
                base,
                *sense,
                construction,
            )?,
        };
        let next = HistoryCheckpoint {
            id: checkpoint,
            event: Some(event),
            summary,
            communities,
            varieties,
            effects,
            rules,
        };
        validate_checkpoint(&next)?;
        Ok(next)
    }

    pub fn commit(&mut self, event: HistoryEvent) -> Result<(), String> {
        let next = self.preview(event)?;
        self.checkpoints.push(next);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != HISTORY_VERSION {
            return Err(format!("unsupported history version {}", self.version));
        }
        if self.checkpoints.is_empty() {
            return Err("history has no checkpoints".into());
        }
        for (i, cp) in self.checkpoints.iter().enumerate() {
            if cp.id != i {
                return Err(format!("checkpoint id {0} is not sequential", cp.id));
            }
            validate_checkpoint(cp)?;
        }
        validate_chronology(&self.checkpoints)?;
        validate_source_refs(&self.checkpoints)?;
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|err| format!("Could not save this history: {err}"))
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let history: History = serde_json::from_str(json)
            .map_err(|err| format!("Could not load this history: {err}"))?;
        history.validate()?;
        Ok(history)
    }

    pub fn coastal_scenario(seed: u64, aesthetic: Aesthetic) -> Result<Self, String> {
        let other = if aesthetic.id == "kuo-toa" {
            Aesthetic::by_id("elvish").ok_or_else(|| "missing elvish aesthetic".to_string())?
        } else {
            Aesthetic::by_id("kuo-toa").ok_or_else(|| "missing kuo-toa aesthetic".to_string())?
        };
        let mut history = Self::new(seed);
        history.commit(HistoryEvent::Found {
            name: "Inland".into(),
            group: "Inland".into(),
            location: "the interior".into(),
            ancestry: "Unspecified".into(),
            language: FoundLanguage::New {
                name: "Inland speech".into(),
                aesthetic: aesthetic.id.clone(),
            },
        })?;
        history.commit(HistoryEvent::Found {
            name: "Coast".into(),
            group: "Coast".into(),
            location: "the shore".into(),
            ancestry: "Unspecified".into(),
            language: FoundLanguage::New {
                name: "Coastal speech".into(),
                aesthetic: other.id,
            },
        })?;
        let coast = named_community_id(&history, "Coast")?;
        let inland_speech = named_variety_id(&history, "Inland speech")?;
        let coastal_speech = named_variety_id(&history, "Coastal speech")?;
        history.commit(HistoryEvent::UseLanguage {
            community: coast,
            variety: inland_speech,
            domain: UseDomain::Trade,
        })?;
        history.commit(HistoryEvent::Separate {
            parent: inland_speech,
            name: "Colony speech".into(),
            community: coast,
        })?;
        history.commit(HistoryEvent::Develop {
            variety: inland_speech,
            steps: 2,
        })?;
        let colony_speech = named_variety_id(&history, "Colony speech")?;
        history.commit(HistoryEvent::Contact {
            donor: coastal_speech,
            recipient: colony_speech,
            domain: ContactDomain::Maritime,
            count: 12,
        })?;
        history.commit(HistoryEvent::Develop {
            variety: colony_speech,
            steps: 5,
        })?;
        history.validate()?;
        Ok(history)
    }

    fn latest(&self) -> Result<&HistoryCheckpoint, String> {
        self.checkpoints
            .last()
            .ok_or_else(|| "history has no checkpoints".into())
    }
}

fn beginning() -> HistoryCheckpoint {
    HistoryCheckpoint {
        id: 0,
        event: None,
        summary: "Beginning".into(),
        communities: Vec::new(),
        varieties: Vec::new(),
        effects: Vec::new(),
        rules: Vec::new(),
    }
}

fn named_community_id(history: &History, name: &str) -> Result<u64, String> {
    history
        .latest()?
        .communities
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.id)
        .ok_or_else(|| format!("missing community '{name}'"))
}

fn named_variety_id(history: &History, name: &str) -> Result<u64, String> {
    history
        .latest()?
        .varieties
        .iter()
        .find(|v| v.name == name)
        .map(|v| v.id)
        .ok_or_else(|| format!("missing variety '{name}'"))
}

fn next_id(ids: impl Iterator<Item = u64>) -> Result<u64, String> {
    ids.max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "identifier overflow".into())
}

fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn founder_seed(history_seed: u64, variety: u64) -> u64 {
    mix64(history_seed ^ mix64(variety.wrapping_add(1)))
}

fn event_seed(history_seed: u64, variety: u64, checkpoint: usize, steps: usize) -> u64 {
    mix64(history_seed ^ mix64(variety) ^ mix64(checkpoint as u64) ^ mix64(steps as u64))
}

fn validate_text(field: &str, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(format!(
            "{field} must be nonempty and contain no control characters"
        ));
    }
    Ok(value.to_string())
}

fn validate_gloss(gloss: &str) -> Result<String, String> {
    let gloss = validate_text("gloss", gloss)?;
    if gloss.len() > MAX_LABEL {
        return Err("gloss must be at most 80 characters".into());
    }
    Ok(gloss)
}

fn primary_gloss(lex: &Lexeme) -> String {
    lex.senses
        .first()
        .map(|sense| sense.meaning.label())
        .unwrap_or_else(|| lex.id.clone())
}

fn apply_found(
    communities: &mut Vec<Community>,
    varieties: &mut Vec<Variety>,
    effects: &mut Vec<HistoryEffect>,
    history_seed: u64,
    checkpoint: usize,
    name: &str,
    group: &str,
    location: &str,
    ancestry: &str,
    language: &FoundLanguage,
) -> Result<String, String> {
    let name = validate_text("community name", name)?;
    let group = validate_text("group", group)?;
    let location = validate_text("location", location)?;
    let ancestry = validate_text("ancestry", ancestry)?;
    if communities.iter().any(|c| c.name == name) {
        return Err(format!("community name '{name}' already exists"));
    }
    let community_id = next_id(communities.iter().map(|c| c.id))?;
    match language {
        FoundLanguage::New {
            name: language_name,
            aesthetic: aesthetic_id,
        } => {
            let language_name = validate_text("variety name", language_name)?;
            if varieties.iter().any(|v| v.name == language_name) {
                return Err(format!("variety name '{language_name}' already exists"));
            }
            let aesthetic = Aesthetic::by_id(aesthetic_id)
                .ok_or_else(|| format!("unknown aesthetic '{aesthetic_id}'"))?;
            let variety_id = next_id(varieties.iter().map(|v| v.id))?;
            let lang = Language::new(founder_seed(history_seed, variety_id), aesthetic.clone());
            let (grammar, mut lexicon) = seed_language(&lang, variety_id, checkpoint);
            if lexicon.is_empty() {
                return Err(format!("founded variety '{language_name}' has no words"));
            }
            for lex in &mut lexicon {
                for trace in &mut lex.traces {
                    trace.after = ortho::romanize(&lex.word, &aesthetic);
                }
            }
            for lex in &lexicon {
                effects.push(HistoryEffect {
                    variety: variety_id,
                    lexeme: lex.id.clone(),
                    gloss: primary_gloss(lex),
                    before: None,
                    after: ortho::romanize(&lex.word, &aesthetic),
                    explanation: format!("founded {name}"),
                });
            }
            varieties.push(Variety {
                id: variety_id,
                name: language_name.clone(),
                parent: None,
                founded_at: checkpoint,
                aesthetic,
                consonants: lang.inventory.consonants.clone(),
                vowels: lang.inventory.vowels.clone(),
                grammar,
                lexicon,
            });
            communities.push(Community {
                id: community_id,
                name: name.clone(),
                group,
                location,
                ancestry,
                uses: vec![LanguageUse {
                    variety: variety_id,
                    domain: UseDomain::Home,
                }],
            });
            Ok(format!("Founded {name} speaking {language_name}"))
        }
        FoundLanguage::Existing { variety } => {
            let variety_name = varieties
                .iter()
                .find(|item| item.id == *variety)
                .map(|item| item.name.clone())
                .ok_or_else(|| format!("unknown variety {variety}"))?;
            communities.push(Community {
                id: community_id,
                name: name.clone(),
                group,
                location,
                ancestry,
                uses: vec![LanguageUse {
                    variety: *variety,
                    domain: UseDomain::Home,
                }],
            });
            effects.push(HistoryEffect {
                variety: *variety,
                lexeme: String::new(),
                gloss: String::new(),
                before: None,
                after: variety_name.clone(),
                explanation: format!("{name} takes up {variety_name}"),
            });
            Ok(format!("Founded {name} using {variety_name}"))
        }
    }
}

fn apply_use_language(
    communities: &mut [Community],
    varieties: &[Variety],
    community_id: u64,
    variety_id: u64,
    domain: UseDomain,
) -> Result<String, String> {
    if !varieties.iter().any(|v| v.id == variety_id) {
        return Err(format!("unknown variety {variety_id}"));
    }
    let community = community_mut(communities, community_id)?;
    if community
        .uses
        .iter()
        .any(|usage| usage.variety == variety_id && usage.domain == domain)
    {
        return Err(format!(
            "{} already uses this variety for {}",
            community.name,
            domain.label()
        ));
    }
    community.uses.push(LanguageUse {
        variety: variety_id,
        domain,
    });
    let variety_name = varieties
        .iter()
        .find(|v| v.id == variety_id)
        .map(|v| v.name.as_str())
        .unwrap_or("variety");
    Ok(format!(
        "{} uses {variety_name} for {}",
        community.name,
        domain.label()
    ))
}

fn apply_separate(
    communities: &mut [Community],
    varieties: &mut Vec<Variety>,
    checkpoint: usize,
    parent_id: u64,
    name: &str,
    community_id: u64,
) -> Result<String, String> {
    let name = validate_text("variety name", name)?;
    if varieties.iter().any(|v| v.name == name) {
        return Err(format!("variety name '{name}' already exists"));
    }
    let parent = varieties
        .iter()
        .find(|v| v.id == parent_id)
        .cloned()
        .ok_or_else(|| format!("unknown parent variety {parent_id}"))?;
    let community = community_mut(communities, community_id)?;
    let id = next_id(varieties.iter().map(|v| v.id))?;
    let source_checkpoint = checkpoint.saturating_sub(1);
    let new_ids: Vec<String> = parent
        .lexicon
        .iter()
        .enumerate()
        .map(|(i, _)| format!("v{id}:r{i}"))
        .collect();
    let mapped = |old: &str| {
        parent
            .lexicon
            .iter()
            .position(|lex| lex.id == old)
            .map(|i| new_ids[i].clone())
            .unwrap_or_else(|| old.to_string())
    };
    let lexicon = parent
        .lexicon
        .iter()
        .enumerate()
        .map(|(i, lex)| {
            let mut traces = lex.traces.clone();
            let form = parent.form(&lex.word);
            traces.push(WordTrace {
                checkpoint,
                variety: id,
                before: Some(form.clone()),
                after: form,
                explanation: format!("inherited from {} at separation", parent.name),
            });
            Lexeme {
                id: new_ids[i].clone(),
                class: lex.class.clone(),
                senses: lex.senses.clone(),
                word: lex.word.clone(),
                stress: lex.stress,
                boundaries: lex.boundaries.clone(),
                analysis: lex.analysis.as_ref().map(|analysis| Analysis {
                    base: mapped(&analysis.base),
                    sense: analysis.sense,
                    construction: analysis.construction.clone(),
                }),
                origin: Origin::Inherited {
                    source: LexemeRef {
                        variety: parent_id,
                        checkpoint: source_checkpoint,
                        lexeme: lex.id.clone(),
                    },
                    checkpoint,
                },
                traces,
                retired: lex.retired,
            }
        })
        .collect();
    let parent_name = parent.name.clone();
    varieties.push(Variety {
        id,
        name: name.clone(),
        parent: Some(parent_id),
        founded_at: checkpoint,
        aesthetic: parent.aesthetic,
        consonants: parent.consonants,
        vowels: parent.vowels,
        grammar: parent.grammar,
        lexicon,
    });
    community.uses.push(LanguageUse {
        variety: id,
        domain: UseDomain::Home,
    });
    Ok(format!(
        "{name} separates from {parent_name} in {}",
        community.name
    ))
}

fn apply_develop(
    varieties: &mut [Variety],
    effects: &mut Vec<HistoryEffect>,
    rules: &mut Vec<SoundChange>,
    history_seed: u64,
    checkpoint: usize,
    variety_id: u64,
    steps: usize,
) -> Result<String, String> {
    if !(1..=5).contains(&steps) {
        return Err(format!("develop steps must be 1..=5, got {steps}"));
    }
    let variety = variety_mut(varieties, variety_id)?;
    let before: Vec<(String, String, Vec<PhonemeId>)> = variety
        .lexicon
        .iter()
        .map(|lex| (lex.id.clone(), variety.form(&lex.word), lex.word.phones()))
        .collect();
    let aesthetic = variety.aesthetic.clone();
    let mut rng = StdRng::seed_from_u64(event_seed(history_seed, variety_id, checkpoint, steps));
    let mut pool = pack_rules();
    for _ in 0..steps {
        let applicable: Vec<usize> = pool
            .iter()
            .enumerate()
            .filter_map(|(i, rule)| {
                language_rule_applies(&variety.grammar, &variety.lexicon, rule).then_some(i)
            })
            .collect();
        if applicable.is_empty() {
            break;
        }
        let weights: Vec<f32> = applicable
            .iter()
            .map(|&i| rule_weight(&aesthetic.id, &pool[i].id))
            .collect();
        let idx = applicable[pick_weighted(&mut rng, &weights)];
        let rule = pool.remove(idx);
        evolve_language(
            &mut variety.grammar,
            &mut variety.lexicon,
            &rule,
            &aesthetic,
            checkpoint,
            variety_id,
        );
        rules.push(rule);
    }
    absorb_novel_phones(variety);
    for (id, form_before, phones_before) in before {
        let Some(lex) = variety
            .lexicon
            .iter()
            .find(|lex| lex.id == id && lex.word.phones() != phones_before)
        else {
            continue;
        };
        effects.push(HistoryEffect {
            variety: variety_id,
            lexeme: lex.id.clone(),
            gloss: primary_gloss(lex),
            before: Some(form_before),
            after: variety.form(&lex.word),
            explanation: lex
                .traces
                .iter()
                .filter(|trace| trace.checkpoint == checkpoint)
                .map(|trace| trace.explanation.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        });
    }
    Ok(format!(
        "{} develops ({} changes)",
        variety.name,
        rules.len()
    ))
}

fn apply_contact(
    varieties: &mut [Variety],
    effects: &mut Vec<HistoryEffect>,
    checkpoint: usize,
    donor_id: u64,
    recipient_id: u64,
    domain: ContactDomain,
    count: usize,
) -> Result<String, String> {
    if donor_id == recipient_id {
        return Err("contact donor and recipient must be distinct".into());
    }
    if !(1..=36).contains(&count) {
        return Err(format!("contact count must be 1..=36, got {count}"));
    }
    if !varieties.iter().any(|v| v.id == donor_id) {
        return Err(format!("unknown donor variety {donor_id}"));
    }
    if !varieties.iter().any(|v| v.id == recipient_id) {
        return Err(format!("unknown recipient variety {recipient_id}"));
    }
    let donor = varieties
        .iter()
        .find(|v| v.id == donor_id)
        .cloned()
        .ok_or_else(|| format!("unknown donor variety {donor_id}"))?;
    let recipient = variety_mut(varieties, recipient_id)?;
    let inventory = variety_inventory(recipient);
    let source_checkpoint = checkpoint.saturating_sub(1);
    let mut borrowed = 0usize;
    for source in contact_candidates(&donor, domain) {
        if borrowed >= count {
            break;
        }
        let loan_id = format!("v{recipient_id}:loan:{donor_id}:{}", source.id);
        if recipient.lexicon.iter().any(|lex| lex.id == loan_id) {
            continue;
        }
        let class = map_class(&source.class, &donor.grammar, &recipient.grammar)?;
        let nativized = nativize_word(&source.word, &inventory);
        let before = donor.form(&source.word);
        let after = recipient.form(&nativized);
        let mut traces = source.traces.clone();
        traces.push(WordTrace {
            checkpoint,
            variety: recipient_id,
            before: Some(before.clone()),
            after: after.clone(),
            explanation: format!("loaned from {} ({})", donor.name, domain.label()),
        });
        effects.push(HistoryEffect {
            variety: recipient_id,
            lexeme: loan_id.clone(),
            gloss: primary_gloss(source),
            before: Some(before),
            after,
            explanation: format!("loaned from {} ({})", donor.name, domain.label()),
        });
        recipient.lexicon.push(Lexeme {
            id: loan_id,
            class,
            senses: source.senses.clone(),
            word: nativized,
            stress: source.stress,
            boundaries: source.boundaries.clone(),
            analysis: None,
            origin: Origin::Borrowed {
                source: LexemeRef {
                    variety: donor_id,
                    checkpoint: source_checkpoint,
                    lexeme: source.id.clone(),
                },
                checkpoint,
            },
            traces,
            retired: false,
        });
        borrowed += 1;
    }
    Ok(format!(
        "{borrowed} {} loans from {} to {}",
        domain.label(),
        donor.name,
        recipient.name
    ))
}

fn apply_derive(
    varieties: &mut [Variety],
    effects: &mut Vec<HistoryEffect>,
    checkpoint: usize,
    variety_id: u64,
    base: &str,
    sense: u32,
    construction: &str,
) -> Result<String, String> {
    if base.trim().is_empty() {
        return Err("derive base must be nonempty".into());
    }
    if construction.trim().is_empty() {
        return Err("derive construction must be nonempty".into());
    }
    let variety = variety_mut(varieties, variety_id)?;
    let source = LexemeRef {
        variety: variety_id,
        checkpoint,
        lexeme: base.to_string(),
    };
    let stem = format!("v{variety_id}:drv:{base}:{sense}:{construction}");
    let id = unique_lexeme_id(&variety.lexicon, &stem)?;
    let mut derived = derive_lexeme(
        &variety.grammar,
        &variety.lexicon,
        source,
        sense,
        construction,
        id,
        checkpoint,
    )?;
    let gloss = primary_gloss(&derived);
    let after = variety.form(&derived.word);
    if let Some(trace) = derived.traces.last_mut() {
        trace.after = after.clone();
    }
    effects.push(HistoryEffect {
        variety: variety_id,
        lexeme: derived.id.clone(),
        gloss: gloss.clone(),
        before: None,
        after,
        explanation: format!("formed {gloss}"),
    });
    let name = variety.name.clone();
    variety.lexicon.push(derived);
    Ok(format!("{name} forms {gloss}"))
}

fn apply_extend_sense(
    varieties: &mut [Variety],
    effects: &mut Vec<HistoryEffect>,
    checkpoint: usize,
    variety_id: u64,
    lexeme: &str,
    gloss: &str,
    frame: SemanticFrame,
) -> Result<String, String> {
    let gloss = validate_gloss(gloss)?;
    let meaning = Meaning::Atom(gloss.clone());
    let variety = variety_mut(varieties, variety_id)?;
    let class = class_of(variety, lexeme)?;
    frame_fits(&class, &frame)?;
    let name = variety.name.clone();
    let form = variety
        .lexicon
        .iter()
        .find(|item| item.id == lexeme)
        .map(|item| variety.form(&item.word))
        .ok_or_else(|| format!("no lexeme '{lexeme}' in variety {variety_id}"))?;
    let lex = active_lexeme_mut(variety, lexeme)?;
    if lex
        .senses
        .iter()
        .any(|sense| sense.meaning == meaning && sense.frame == frame)
    {
        return Err("lexeme already has this sense".into());
    }
    let id = lex
        .senses
        .iter()
        .map(|sense| sense.id)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "sense id overflow".to_string())?;
    lex.senses.push(Sense { id, meaning, frame });
    lex.traces.push(WordTrace {
        checkpoint,
        variety: variety_id,
        before: Some(form.clone()),
        after: form.clone(),
        explanation: format!("extended sense '{gloss}'"),
    });
    let summary = format!("{name} adds the sense ‘{gloss}’ to {form}");
    effects.push(HistoryEffect {
        variety: variety_id,
        lexeme: lexeme.to_string(),
        gloss: gloss.clone(),
        before: None,
        after: form,
        explanation: format!("extended sense '{gloss}'"),
    });
    Ok(summary)
}

fn apply_shift_sense(
    varieties: &mut [Variety],
    effects: &mut Vec<HistoryEffect>,
    checkpoint: usize,
    variety_id: u64,
    lexeme: &str,
    sense_id: u32,
    gloss: &str,
    frame: SemanticFrame,
) -> Result<String, String> {
    let gloss = validate_gloss(gloss)?;
    let meaning = Meaning::Atom(gloss.clone());
    let variety = variety_mut(varieties, variety_id)?;
    let class = class_of(variety, lexeme)?;
    frame_fits(&class, &frame)?;
    let name = variety.name.clone();
    let form = variety
        .lexicon
        .iter()
        .find(|item| item.id == lexeme)
        .map(|item| variety.form(&item.word))
        .ok_or_else(|| format!("no lexeme '{lexeme}' in variety {variety_id}"))?;
    let lex = active_lexeme_mut(variety, lexeme)?;
    let before_label = {
        let sense = lex
            .senses
            .iter_mut()
            .find(|sense| sense.id == sense_id)
            .ok_or_else(|| format!("lexeme '{lexeme}' has no sense {sense_id}"))?;
        if sense.meaning == meaning && sense.frame == frame {
            return Err("shift would not change this sense".into());
        }
        let before_label = sense.meaning.label();
        sense.meaning = meaning;
        sense.frame = frame;
        before_label
    };
    let summary = format!("{name}: {form} shifts from ‘{before_label}’ to ‘{gloss}’");
    lex.traces.push(WordTrace {
        checkpoint,
        variety: variety_id,
        before: Some(form.clone()),
        after: form,
        explanation: format!("shifted sense to '{gloss}'"),
    });
    effects.push(HistoryEffect {
        variety: variety_id,
        lexeme: lexeme.to_string(),
        gloss: gloss.clone(),
        before: Some(before_label),
        after: gloss.clone(),
        explanation: format!("shifted sense to '{gloss}'"),
    });
    Ok(summary)
}

fn apply_replace(
    varieties: &mut [Variety],
    effects: &mut Vec<HistoryEffect>,
    checkpoint: usize,
    variety_id: u64,
    lexeme: &str,
    replacement: &str,
) -> Result<String, String> {
    if lexeme == replacement {
        return Err("replace cannot target the same lexeme".into());
    }
    let variety = variety_mut(varieties, variety_id)?;
    let source_idx = variety
        .lexicon
        .iter()
        .position(|item| item.id == lexeme)
        .ok_or_else(|| format!("no lexeme '{lexeme}' in variety {variety_id}"))?;
    let target_idx = variety
        .lexicon
        .iter()
        .position(|item| item.id == replacement)
        .ok_or_else(|| format!("no lexeme '{replacement}' in variety {variety_id}"))?;
    if variety.lexicon[source_idx].retired {
        return Err(format!("lexeme '{lexeme}' is already retired"));
    }
    if variety.lexicon[target_idx].retired {
        return Err(format!("lexeme '{replacement}' is retired"));
    }
    if variety.lexicon[source_idx].class != variety.lexicon[target_idx].class {
        return Err("replace requires the same lexical class".into());
    }
    let transferred = variety.lexicon[source_idx].senses.clone();
    if transferred.is_empty() {
        return Err("replace would transfer no meanings".into());
    }
    if transferred.iter().any(|sense| {
        variety.lexicon[target_idx]
            .senses
            .iter()
            .any(|existing| existing.meaning == sense.meaning && existing.frame == sense.frame)
    }) {
        return Err("replace would duplicate an existing meaning".into());
    }
    let mut next_sense = variety.lexicon[target_idx]
        .senses
        .iter()
        .map(|sense| sense.id)
        .max()
        .unwrap_or(0);
    for sense in transferred {
        next_sense = next_sense
            .checked_add(1)
            .ok_or_else(|| "sense id overflow".to_string())?;
        variety.lexicon[target_idx].senses.push(Sense {
            id: next_sense,
            meaning: sense.meaning,
            frame: sense.frame,
        });
    }
    variety.lexicon[source_idx].retired = true;
    let source_form = variety.form(&variety.lexicon[source_idx].word);
    let target_form = variety.form(&variety.lexicon[target_idx].word);
    variety.lexicon[source_idx].traces.push(WordTrace {
        checkpoint,
        variety: variety_id,
        before: Some(source_form.clone()),
        after: source_form.clone(),
        explanation: format!("replaced by {target_form}"),
    });
    variety.lexicon[target_idx].traces.push(WordTrace {
        checkpoint,
        variety: variety_id,
        before: Some(target_form.clone()),
        after: target_form.clone(),
        explanation: format!("received meanings from {source_form}"),
    });
    let gloss = primary_gloss(&variety.lexicon[source_idx]);
    let name = variety.name.clone();
    let summary = format!("{name} replaces {source_form} with {target_form}");
    let explanation = format!("replaced by {target_form}");
    effects.push(HistoryEffect {
        variety: variety_id,
        lexeme: lexeme.to_string(),
        gloss,
        before: Some(source_form),
        after: target_form,
        explanation,
    });
    Ok(summary)
}

fn apply_lexicalize(
    varieties: &mut [Variety],
    effects: &mut Vec<HistoryEffect>,
    checkpoint: usize,
    variety_id: u64,
    lexeme: &str,
) -> Result<String, String> {
    let variety = variety_mut(varieties, variety_id)?;
    let name = variety.name.clone();
    let form = variety
        .lexicon
        .iter()
        .find(|item| item.id == lexeme)
        .map(|item| variety.form(&item.word))
        .ok_or_else(|| format!("no lexeme '{lexeme}' in variety {variety_id}"))?;
    let lex = active_lexeme_mut(variety, lexeme)?;
    if lex.analysis.is_none() {
        return Err(format!("lexeme '{lexeme}' is already unanalyzed"));
    }
    lex.analysis = None;
    lex.traces.push(WordTrace {
        checkpoint,
        variety: variety_id,
        before: Some(form.clone()),
        after: form.clone(),
        explanation: "lost current analysis".into(),
    });
    let summary = format!("{name}: {form} loses its productive analysis");
    effects.push(HistoryEffect {
        variety: variety_id,
        lexeme: lexeme.to_string(),
        gloss: primary_gloss(lex),
        before: None,
        after: form,
        explanation: "lost current analysis".into(),
    });
    Ok(summary)
}

fn apply_remodel(
    varieties: &mut [Variety],
    effects: &mut Vec<HistoryEffect>,
    checkpoint: usize,
    variety_id: u64,
    lexeme: &str,
    base: &str,
    sense: u32,
    construction: &str,
) -> Result<String, String> {
    let variety = variety_mut(varieties, variety_id)?;
    let mut updated = remodel_lexeme(
        &variety.grammar,
        &variety.lexicon,
        lexeme,
        base,
        sense,
        construction,
        checkpoint,
        variety_id,
    )?;
    let gloss = primary_gloss(&updated);
    let after = variety.form(&updated.word);
    let (before, idx) = {
        let prev = variety
            .lexicon
            .iter()
            .find(|item| item.id == lexeme)
            .ok_or_else(|| format!("no lexeme '{lexeme}' in variety {variety_id}"))?;
        if updated.word == prev.word
            && updated.stress == prev.stress
            && updated.boundaries == prev.boundaries
            && updated.analysis == prev.analysis
        {
            return Err("remodel would not change this word".into());
        }
        (
            Some(variety.form(&prev.word)),
            variety
                .lexicon
                .iter()
                .position(|item| item.id == lexeme)
                .ok_or_else(|| format!("no lexeme '{lexeme}' in variety {variety_id}"))?,
        )
    };
    if let Some(trace) = updated.traces.last_mut() {
        trace.before = before.clone();
        trace.after = after.clone();
    }
    variety.lexicon[idx] = updated;
    let summary = format!("{} remodels {after} by analogy", variety.name);
    effects.push(HistoryEffect {
        variety: variety_id,
        lexeme: lexeme.to_string(),
        gloss,
        before,
        after,
        explanation: "remodeled the form or current analysis by analogy".into(),
    });
    Ok(summary)
}

fn community_mut(communities: &mut [Community], id: u64) -> Result<&mut Community, String> {
    communities
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("unknown community {id}"))
}

fn variety_mut(varieties: &mut [Variety], id: u64) -> Result<&mut Variety, String> {
    varieties
        .iter_mut()
        .find(|v| v.id == id)
        .ok_or_else(|| format!("unknown variety {id}"))
}

fn variety_inventory(variety: &Variety) -> Inventory {
    Inventory::from_ids(
        variety
            .consonants
            .iter()
            .copied()
            .chain(variety.vowels.iter().copied()),
    )
}

fn class_of(variety: &Variety, lexeme: &str) -> Result<LexicalClass, String> {
    let lex = variety
        .lexicon
        .iter()
        .find(|item| item.id == lexeme)
        .ok_or_else(|| format!("no lexeme '{lexeme}' in variety {}", variety.id))?;
    if lex.retired {
        return Err(format!("lexeme '{lexeme}' is retired"));
    }
    variety
        .grammar
        .classes
        .iter()
        .find(|class| class.id == lex.class)
        .cloned()
        .ok_or_else(|| format!("lexeme '{lexeme}' has unknown class '{}'", lex.class))
}

fn active_lexeme_mut<'a>(variety: &'a mut Variety, lexeme: &str) -> Result<&'a mut Lexeme, String> {
    let lex = variety
        .lexicon
        .iter_mut()
        .find(|item| item.id == lexeme)
        .ok_or_else(|| format!("no lexeme '{lexeme}' in variety {}", variety.id))?;
    if lex.retired {
        return Err(format!("lexeme '{lexeme}' is retired"));
    }
    Ok(lex)
}

fn frame_fits(class: &LexicalClass, frame: &SemanticFrame) -> Result<(), String> {
    match (class.kind, frame) {
        (ClassKind::Entity, SemanticFrame::Entity { .. }) => Ok(()),
        (ClassKind::Event, SemanticFrame::Event { roles }) => {
            if roles.is_empty() {
                return Err("event frame must name at least one role".into());
            }
            Ok(())
        }
        _ => Err("sense frame does not match the lexeme class".into()),
    }
}

fn map_class(source_class: &str, donor: &Grammar, recipient: &Grammar) -> Result<String, String> {
    let donor_class = donor
        .classes
        .iter()
        .find(|class| class.id == source_class)
        .ok_or_else(|| format!("donor class '{source_class}' is missing"))?;
    if let Some(class) = recipient
        .classes
        .iter()
        .find(|class| class.id == donor_class.id && class.kind == donor_class.kind)
    {
        return Ok(class.id.clone());
    }
    if let Some(class) = recipient
        .classes
        .iter()
        .find(|class| class.label == donor_class.label && class.kind == donor_class.kind)
    {
        return Ok(class.id.clone());
    }
    recipient
        .classes
        .iter()
        .find(|class| class.kind == donor_class.kind)
        .map(|class| class.id.clone())
        .ok_or_else(|| "recipient has no class of the same kind".into())
}

fn unique_lexeme_id(lexicon: &[Lexeme], stem: &str) -> Result<String, String> {
    if lexicon.iter().all(|lex| lex.id != stem) {
        return Ok(stem.to_string());
    }
    let mut n = 2u32;
    loop {
        let id = format!("{stem}:{n}");
        if lexicon.iter().all(|lex| lex.id != id) {
            return Ok(id);
        }
        n = n
            .checked_add(1)
            .ok_or_else(|| "lexeme id overflow".to_string())?;
    }
}

fn contact_candidates(donor: &Variety, domain: ContactDomain) -> Vec<&Lexeme> {
    let mut items: Vec<&Lexeme> = donor.lexicon.iter().filter(|lex| !lex.retired).collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    match domain {
        ContactDomain::General => items,
        ContactDomain::Maritime => {
            let mut preferred = Vec::new();
            let mut rest = Vec::new();
            for lex in items {
                if lex
                    .senses
                    .iter()
                    .any(|sense| MARITIME_GLOSSES.contains(&sense.meaning.label().as_str()))
                {
                    preferred.push(lex);
                } else {
                    rest.push(lex);
                }
            }
            preferred.sort_by_key(|lex| {
                (
                    lex.senses
                        .iter()
                        .filter_map(|sense| {
                            MARITIME_GLOSSES
                                .iter()
                                .position(|g| *g == sense.meaning.label())
                        })
                        .min()
                        .unwrap_or(usize::MAX),
                    lex.id.clone(),
                )
            });
            preferred.extend(rest);
            preferred
        }
    }
}

fn pack_rules() -> Vec<SoundChange> {
    let mut rules = conservative();
    for rule in radical() {
        if !rules.iter().any(|existing| existing.id == rule.id) {
            rules.push(rule);
        }
    }
    rules
}

fn rule_weight(aesthetic_id: &str, rule_id: &str) -> f32 {
    let conservative = rule_id == "t_to_s_before_i" || rule_id == "p_lenite_v_v";
    match aesthetic_id {
        "elvish" => {
            if conservative {
                4.0
            } else {
                1.0
            }
        }
        "kuo-toa" | "illithid" => {
            if conservative {
                1.0
            } else {
                4.0
            }
        }
        _ => {
            if conservative {
                3.0
            } else {
                1.0
            }
        }
    }
}

fn pick_weighted(rng: &mut impl Rng, weights: &[f32]) -> usize {
    let total: f32 = weights.iter().copied().map(|w| w.max(0.0)).sum();
    if total <= 0.0 {
        return rng.gen_range(0..weights.len());
    }
    let mut x = rng.r#gen::<f32>() * total;
    for (i, w) in weights.iter().enumerate() {
        x -= w.max(0.0);
        if x <= 0.0 {
            return i;
        }
    }
    weights.len() - 1
}

fn absorb_novel_phones(variety: &mut Variety) {
    let mut seen: Vec<PhonemeId> = variety
        .consonants
        .iter()
        .copied()
        .chain(variety.vowels.iter().copied())
        .collect();
    let mut extras = Vec::new();
    let mut consider = |phone: PhonemeId| {
        if !seen.contains(&phone) && !extras.contains(&phone) {
            extras.push(phone);
        }
    };
    for lex in &variety.lexicon {
        for phone in lex.word.phonemes() {
            consider(phone);
        }
    }
    for construction in &variety.grammar.constructions {
        for word in construction.exponents() {
            for phone in word.phonemes() {
                consider(phone);
            }
        }
    }
    for phone in extras {
        seen.push(phone);
        let Some(seg) = CATALOG.segments.get(phone.0 as usize) else {
            continue;
        };
        if seg.is_vowel() {
            variety.vowels.push(phone);
        } else {
            variety.consonants.push(phone);
        }
    }
}

fn validate_checkpoint(cp: &HistoryCheckpoint) -> Result<(), String> {
    let mut community_ids = Vec::new();
    let mut community_names = Vec::new();
    let mut variety_ids = Vec::new();
    let mut variety_names = Vec::new();
    for variety in &cp.varieties {
        if variety.id == 0 || variety.founded_at > cp.id {
            return Err("variety id or founding checkpoint is out of range".into());
        }
        if variety_ids.contains(&variety.id) {
            return Err(format!("duplicate variety id {}", variety.id));
        }
        variety_ids.push(variety.id);
        if variety.name.trim().is_empty() {
            return Err(format!("variety {} has an empty name", variety.id));
        }
        if variety_names.iter().any(|n| n == &variety.name) {
            return Err(format!("duplicate variety name '{}'", variety.name));
        }
        variety_names.push(variety.name.clone());
        if variety.aesthetic.id.trim().is_empty() {
            return Err(format!("variety {} has an empty aesthetic", variety.id));
        }
        if let Some(parent) = variety.parent {
            if !cp.varieties.iter().any(|item| item.id == parent) {
                return Err(format!("variety {} parent {parent} is missing", variety.id));
            }
        }
        validate_inventory(variety)?;
        if variety.lexicon.is_empty() {
            return Err(format!("variety {} has an empty lexicon", variety.id));
        }
        validate_language(&variety.grammar, &variety.lexicon)?;
        for lex in &variety.lexicon {
            if lex
                .word
                .phonemes()
                .any(|id| !variety.consonants.contains(&id) && !variety.vowels.contains(&id))
            {
                return Err(format!(
                    "lexeme '{}' uses a phoneme outside its variety inventory",
                    lex.id
                ));
            }
            if lex.traces.is_empty() {
                return Err(format!("lexeme '{}' has no traces", lex.id));
            }
            for trace in &lex.traces {
                if trace.after.trim().is_empty() {
                    return Err(format!("lexeme '{}' has an empty trace", lex.id));
                }
            }
            if !current_form_recorded(variety, lex) {
                return Err(format!(
                    "lexeme '{}' does not match its recorded form",
                    lex.id
                ));
            }
        }
    }
    for community in &cp.communities {
        if community.id == 0 {
            return Err("community id is out of range".into());
        }
        if community_ids.contains(&community.id) {
            return Err(format!("duplicate community id {}", community.id));
        }
        community_ids.push(community.id);
        if community.name.trim().is_empty() {
            return Err(format!("community {} has an empty name", community.id));
        }
        if community_names.iter().any(|n| n == &community.name) {
            return Err(format!("duplicate community name '{}'", community.name));
        }
        community_names.push(community.name.clone());
        if community.group.trim().is_empty()
            || community.location.trim().is_empty()
            || community.ancestry.trim().is_empty()
        {
            return Err(format!(
                "community {} is missing authored metadata",
                community.id
            ));
        }
        if community.uses.is_empty() {
            return Err(format!("community {} has no language uses", community.id));
        }
        let mut seen_uses = Vec::new();
        for usage in &community.uses {
            if !cp.varieties.iter().any(|v| v.id == usage.variety) {
                return Err(format!(
                    "community {} uses unknown variety {}",
                    community.id, usage.variety
                ));
            }
            if seen_uses
                .iter()
                .any(|(variety, domain)| *variety == usage.variety && *domain == usage.domain)
            {
                return Err(format!("community {} repeats a language use", community.id));
            }
            seen_uses.push((usage.variety, usage.domain));
        }
    }
    Ok(())
}

fn current_form_recorded(variety: &Variety, lex: &Lexeme) -> bool {
    lex.traces
        .last()
        .is_some_and(|last| last.after == variety.form(&lex.word))
}

fn validate_inventory(variety: &Variety) -> Result<(), String> {
    if variety.consonants.is_empty() || variety.vowels.is_empty() {
        return Err(format!(
            "variety {} has an empty phonological inventory",
            variety.id
        ));
    }
    let mut seen = Vec::new();
    for (id, listed_as_vowel) in variety
        .consonants
        .iter()
        .map(|id| (id, false))
        .chain(variety.vowels.iter().map(|id| (id, true)))
    {
        let Some(seg) = CATALOG.segments.get(id.0 as usize) else {
            return Err(format!(
                "variety {} inventory contains an unknown phoneme",
                variety.id
            ));
        };
        if seen.contains(id) {
            return Err(format!(
                "variety {} inventory repeats a phoneme",
                variety.id
            ));
        }
        seen.push(*id);
        if seg.is_vowel() != listed_as_vowel {
            return Err(format!(
                "variety {} inventory classifies {} incorrectly",
                variety.id,
                seg.ipa()
            ));
        }
    }
    Ok(())
}

fn validate_source_refs(checkpoints: &[HistoryCheckpoint]) -> Result<(), String> {
    for cp in checkpoints {
        for variety in &cp.varieties {
            for lex in &variety.lexicon {
                validate_origin(checkpoints, cp.id, &lex.origin)?;
                let mut previous = 0;
                for trace in &lex.traces {
                    if trace.checkpoint < previous || trace.checkpoint > cp.id {
                        return Err(format!("lexeme '{}' has invalid trace ancestry", lex.id));
                    }
                    let Some(at) = checkpoints.get(trace.checkpoint) else {
                        return Err(format!("lexeme '{}' trace checkpoint is missing", lex.id));
                    };
                    let known = at.varieties.iter().any(|item| {
                        item.id == trace.variety && item.founded_at <= trace.checkpoint
                    });
                    if !known {
                        return Err(format!(
                            "lexeme '{}' trace refers to an unknown variety",
                            lex.id
                        ));
                    }
                    previous = trace.checkpoint;
                }
            }
        }
    }
    Ok(())
}

fn validate_origin(
    checkpoints: &[HistoryCheckpoint],
    current: usize,
    origin: &Origin,
) -> Result<(), String> {
    match origin {
        Origin::Unrecorded {
            variety,
            checkpoint,
        } => {
            if *checkpoint > current {
                return Err("unrecorded origin is in the future".into());
            }
            variety_at(checkpoints, *checkpoint, *variety).map(|_| ())
        }
        Origin::Formed {
            source,
            sense,
            construction,
            checkpoint,
        } => {
            if *checkpoint > current {
                return Err("formed origin is in the future".into());
            }
            let src = resolve_ref(checkpoints, source)?;
            if !src.senses.iter().any(|item| item.id == *sense) {
                return Err(format!(
                    "formed origin sense {} is missing on '{}'",
                    sense, source.lexeme
                ));
            }
            let at = variety_at(checkpoints, *checkpoint, source.variety)?;
            if !at
                .grammar
                .constructions
                .iter()
                .any(|item| item.id == *construction)
            {
                return Err(format!(
                    "formed origin construction '{construction}' is missing"
                ));
            }
            Ok(())
        }
        Origin::Borrowed { source, checkpoint } | Origin::Inherited { source, checkpoint } => {
            if *checkpoint > current {
                return Err("origin checkpoint is in the future".into());
            }
            resolve_ref(checkpoints, source).map(|_| ())
        }
    }
}

fn resolve_ref<'a>(
    checkpoints: &'a [HistoryCheckpoint],
    source: &LexemeRef,
) -> Result<&'a Lexeme, String> {
    let variety = variety_at(checkpoints, source.checkpoint, source.variety)?;
    variety
        .lexicon
        .iter()
        .find(|lex| lex.id == source.lexeme)
        .ok_or_else(|| format!("historical source '{}' is missing", source.lexeme))
}

fn variety_at(
    checkpoints: &[HistoryCheckpoint],
    checkpoint: usize,
    id: u64,
) -> Result<&Variety, String> {
    let cp = checkpoints
        .get(checkpoint)
        .ok_or_else(|| format!("historical checkpoint {checkpoint} is missing"))?;
    cp.varieties
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| format!("historical variety {id} is missing at {checkpoint}"))
}

fn validate_chronology(checkpoints: &[HistoryCheckpoint]) -> Result<(), String> {
    let first = &checkpoints[0];
    if first.event.is_some() || !first.communities.is_empty() || !first.varieties.is_empty() {
        return Err("checkpoint 0 must be the empty beginning".into());
    }
    let mut prev_communities: &[Community] = &first.communities;
    let mut prev_varieties: &[Variety] = &first.varieties;
    for cp in checkpoints.iter().skip(1) {
        let event = cp
            .event
            .as_ref()
            .ok_or_else(|| format!("checkpoint {} missing event", cp.id))?;
        match event {
            HistoryEvent::Found {
                name,
                group,
                location,
                ancestry,
                language,
            } => {
                let added = expect_added_community(
                    prev_communities,
                    &cp.communities,
                    name,
                    group,
                    location,
                    ancestry,
                )?;
                match language {
                    FoundLanguage::New {
                        name: language_name,
                        aesthetic,
                    } => {
                        if Aesthetic::by_id(aesthetic).is_none() {
                            return Err(format!("unknown aesthetic '{aesthetic}'"));
                        }
                        expect_frozen_varieties(prev_varieties, &cp.varieties, &[])?;
                        let variety = expect_added_variety(
                            prev_varieties,
                            &cp.varieties,
                            language_name,
                            None,
                            cp.id,
                        )?;
                        if variety.aesthetic.id != *aesthetic {
                            return Err(format!(
                                "checkpoint {} variety aesthetic does not match the event",
                                cp.id
                            ));
                        }
                        expect_home_use(added, variety.id)?;
                    }
                    FoundLanguage::Existing { variety } => {
                        if !prev_varieties.iter().any(|item| item.id == *variety) {
                            return Err(format!(
                                "checkpoint {} founds with unknown variety {variety}",
                                cp.id
                            ));
                        }
                        expect_frozen_varieties(prev_varieties, &cp.varieties, &[])?;
                        expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                        expect_home_use(added, *variety)?;
                    }
                }
            }
            HistoryEvent::UseLanguage {
                community,
                variety,
                domain,
            } => {
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[])?;
                let Some(prev_c) = prev_communities.iter().find(|item| item.id == *community)
                else {
                    return Err(format!(
                        "checkpoint {} uses a language in unknown community {community}",
                        cp.id
                    ));
                };
                if !prev_varieties.iter().any(|item| item.id == *variety) {
                    return Err(format!(
                        "checkpoint {} uses unknown variety {variety}",
                        cp.id
                    ));
                }
                expect_frozen_communities(prev_communities, &cp.communities, &[*community])?;
                let Some(next_c) = cp.communities.iter().find(|item| item.id == *community) else {
                    return Err(format!(
                        "checkpoint {} is missing community {community}",
                        cp.id
                    ));
                };
                if !community_meta_same(prev_c, next_c) {
                    return Err(format!("checkpoint {} changed community metadata", cp.id));
                }
                expect_use_appended(&prev_c.uses, &next_c.uses, *variety, *domain, cp.id)?;
            }
            HistoryEvent::Separate {
                parent,
                name,
                community,
            } => {
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                if !prev_varieties.iter().any(|item| item.id == *parent) {
                    return Err(format!(
                        "checkpoint {} separates from unknown parent {parent}",
                        cp.id
                    ));
                }
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[])?;
                let added = expect_added_variety(
                    prev_varieties,
                    &cp.varieties,
                    name,
                    Some(*parent),
                    cp.id,
                )?;
                expect_frozen_communities(prev_communities, &cp.communities, &[*community])?;
                let Some(prev_c) = prev_communities.iter().find(|item| item.id == *community)
                else {
                    return Err(format!(
                        "checkpoint {} separates into unknown community {community}",
                        cp.id
                    ));
                };
                let Some(next_c) = cp.communities.iter().find(|item| item.id == *community) else {
                    return Err(format!(
                        "checkpoint {} is missing community {community}",
                        cp.id
                    ));
                };
                if !community_meta_same(prev_c, next_c) {
                    return Err(format!(
                        "checkpoint {} changed community metadata during separate",
                        cp.id
                    ));
                }
                expect_use_appended(&prev_c.uses, &next_c.uses, added.id, UseDomain::Home, cp.id)?;
            }
            HistoryEvent::Develop { variety, steps } => {
                if !(1..=5).contains(steps) {
                    return Err(format!("checkpoint {} has invalid develop steps", cp.id));
                }
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                if !prev_varieties.iter().any(|item| item.id == *variety) {
                    return Err(format!(
                        "checkpoint {} develops unknown variety {variety}",
                        cp.id
                    ));
                }
                expect_frozen_communities(prev_communities, &cp.communities, &[])?;
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[*variety])?;
            }
            HistoryEvent::Contact {
                donor,
                recipient,
                count,
                ..
            } => {
                if donor == recipient {
                    return Err(format!(
                        "checkpoint {} contacts a variety with itself",
                        cp.id
                    ));
                }
                if !(1..=36).contains(count) {
                    return Err(format!("checkpoint {} has invalid contact count", cp.id));
                }
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                if !prev_varieties.iter().any(|item| item.id == *donor) {
                    return Err(format!("checkpoint {} has unknown donor {donor}", cp.id));
                }
                if !prev_varieties.iter().any(|item| item.id == *recipient) {
                    return Err(format!(
                        "checkpoint {} has unknown recipient {recipient}",
                        cp.id
                    ));
                }
                expect_frozen_communities(prev_communities, &cp.communities, &[])?;
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[*recipient])?;
            }
            HistoryEvent::Derive {
                variety,
                base,
                sense,
                construction,
            } => {
                if base.trim().is_empty() || construction.trim().is_empty() {
                    return Err(format!(
                        "checkpoint {} derives from an empty reference",
                        cp.id
                    ));
                }
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                let Some(prev_v) = prev_varieties.iter().find(|item| item.id == *variety) else {
                    return Err(format!(
                        "checkpoint {} derives in unknown variety {variety}",
                        cp.id
                    ));
                };
                if !prev_v.lexicon.iter().any(|lex| lex.id == *base) {
                    return Err(format!(
                        "checkpoint {} derives from unknown base '{base}'",
                        cp.id
                    ));
                }
                expect_frozen_communities(prev_communities, &cp.communities, &[])?;
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[*variety])?;
                let Some(next_v) = cp.varieties.iter().find(|item| item.id == *variety) else {
                    return Err(format!("checkpoint {} is missing variety {variety}", cp.id));
                };
                if next_v.lexicon.len() != prev_v.lexicon.len() + 1 {
                    return Err(format!(
                        "checkpoint {} did not add exactly one derived lexeme",
                        cp.id
                    ));
                }
                if next_v.grammar != prev_v.grammar {
                    return Err(format!(
                        "checkpoint {} changed grammar during derive",
                        cp.id
                    ));
                }
                if !next_v.lexicon.iter().any(|lex| {
                    lex.analysis.as_ref().is_some_and(|analysis| {
                        analysis.base == *base
                            && analysis.sense == *sense
                            && analysis.construction == *construction
                    })
                }) {
                    return Err(format!(
                        "checkpoint {} is missing the derived analysis",
                        cp.id
                    ));
                }
            }
            HistoryEvent::ExtendSense {
                variety, lexeme, ..
            } => {
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                let Some(prev_v) = prev_varieties.iter().find(|item| item.id == *variety) else {
                    return Err(format!(
                        "checkpoint {} extends a sense in unknown variety {variety}",
                        cp.id
                    ));
                };
                let Some(prev_lex) = prev_v.lexicon.iter().find(|item| item.id == *lexeme) else {
                    return Err(format!(
                        "checkpoint {} extends unknown lexeme '{lexeme}'",
                        cp.id
                    ));
                };
                expect_frozen_communities(prev_communities, &cp.communities, &[])?;
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[*variety])?;
                let next_lex = lexeme_at(&cp.varieties, *variety, lexeme, cp.id)?;
                if next_lex.senses.len() != prev_lex.senses.len() + 1 {
                    return Err(format!(
                        "checkpoint {} did not add exactly one sense",
                        cp.id
                    ));
                }
                if next_lex.word != prev_lex.word
                    || next_lex.origin != prev_lex.origin
                    || next_lex.analysis != prev_lex.analysis
                    || next_lex.retired
                {
                    return Err(format!(
                        "checkpoint {} changed more than the extended sense",
                        cp.id
                    ));
                }
            }
            HistoryEvent::ShiftSense {
                variety,
                lexeme,
                sense,
                ..
            } => {
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                let Some(prev_v) = prev_varieties.iter().find(|item| item.id == *variety) else {
                    return Err(format!(
                        "checkpoint {} shifts a sense in unknown variety {variety}",
                        cp.id
                    ));
                };
                let Some(prev_lex) = prev_v.lexicon.iter().find(|item| item.id == *lexeme) else {
                    return Err(format!(
                        "checkpoint {} shifts unknown lexeme '{lexeme}'",
                        cp.id
                    ));
                };
                let Some(prev_sense) = prev_lex.senses.iter().find(|item| item.id == *sense) else {
                    return Err(format!("checkpoint {} shifts unknown sense {sense}", cp.id));
                };
                expect_frozen_communities(prev_communities, &cp.communities, &[])?;
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[*variety])?;
                let next_lex = lexeme_at(&cp.varieties, *variety, lexeme, cp.id)?;
                let Some(next_sense) = next_lex.senses.iter().find(|item| item.id == *sense) else {
                    return Err(format!("checkpoint {} dropped sense {sense}", cp.id));
                };
                if next_lex.senses.len() != prev_lex.senses.len()
                    || next_lex.word != prev_lex.word
                    || next_lex.origin != prev_lex.origin
                    || next_lex.analysis != prev_lex.analysis
                    || next_sense.id != prev_sense.id
                    || (next_sense.meaning == prev_sense.meaning
                        && next_sense.frame == prev_sense.frame)
                {
                    return Err(format!(
                        "checkpoint {} did not shift the selected sense in place",
                        cp.id
                    ));
                }
            }
            HistoryEvent::Replace {
                variety,
                lexeme,
                replacement,
            } => {
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                expect_frozen_communities(prev_communities, &cp.communities, &[])?;
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[*variety])?;
                let prev_src = lexeme_at(prev_varieties, *variety, lexeme, cp.id)?;
                let next_src = lexeme_at(&cp.varieties, *variety, lexeme, cp.id)?;
                let prev_dst = lexeme_at(prev_varieties, *variety, replacement, cp.id)?;
                let next_dst = lexeme_at(&cp.varieties, *variety, replacement, cp.id)?;
                if !next_src.retired
                    || next_src.word != prev_src.word
                    || next_src.origin != prev_src.origin
                    || next_dst.retired
                    || next_dst.senses.len() <= prev_dst.senses.len()
                {
                    return Err(format!(
                        "checkpoint {} did not retire and transfer meanings",
                        cp.id
                    ));
                }
            }
            HistoryEvent::Lexicalize { variety, lexeme } => {
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                expect_frozen_communities(prev_communities, &cp.communities, &[])?;
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[*variety])?;
                let prev_lex = lexeme_at(prev_varieties, *variety, lexeme, cp.id)?;
                let next_lex = lexeme_at(&cp.varieties, *variety, lexeme, cp.id)?;
                if prev_lex.analysis.is_none()
                    || next_lex.analysis.is_some()
                    || next_lex.word != prev_lex.word
                    || next_lex.origin != prev_lex.origin
                    || next_lex.senses != prev_lex.senses
                {
                    return Err(format!(
                        "checkpoint {} did not lexicalize only the current analysis",
                        cp.id
                    ));
                }
            }
            HistoryEvent::Remodel {
                variety, lexeme, ..
            } => {
                expect_same_community_ids(prev_communities, &cp.communities, cp.id)?;
                expect_same_variety_ids(prev_varieties, &cp.varieties, cp.id)?;
                expect_frozen_communities(prev_communities, &cp.communities, &[])?;
                expect_frozen_varieties(prev_varieties, &cp.varieties, &[*variety])?;
                let prev_lex = lexeme_at(prev_varieties, *variety, lexeme, cp.id)?;
                let next_lex = lexeme_at(&cp.varieties, *variety, lexeme, cp.id)?;
                if next_lex.origin != prev_lex.origin
                    || next_lex.senses != prev_lex.senses
                    || next_lex.analysis.is_none()
                {
                    return Err(format!(
                        "checkpoint {} did not remodel while keeping meaning and origin",
                        cp.id
                    ));
                }
            }
        }
        prev_communities = &cp.communities;
        prev_varieties = &cp.varieties;
    }
    Ok(())
}

fn lexeme_at<'a>(
    varieties: &'a [Variety],
    variety: u64,
    lexeme: &str,
    checkpoint: usize,
) -> Result<&'a Lexeme, String> {
    varieties
        .iter()
        .find(|item| item.id == variety)
        .and_then(|item| item.lexicon.iter().find(|lex| lex.id == lexeme))
        .ok_or_else(|| format!("checkpoint {checkpoint} is missing lexeme '{lexeme}'"))
}

fn expect_home_use(community: &Community, variety: u64) -> Result<(), String> {
    if community
        .uses
        .iter()
        .any(|usage| usage.variety == variety && usage.domain == UseDomain::Home)
    {
        Ok(())
    } else {
        Err(format!(
            "community {} is missing a home use of variety {variety}",
            community.id
        ))
    }
}

fn expect_use_appended(
    prev: &[LanguageUse],
    next: &[LanguageUse],
    variety: u64,
    domain: UseDomain,
    checkpoint: usize,
) -> Result<(), String> {
    if next.len() != prev.len() + 1 {
        return Err(format!(
            "checkpoint {checkpoint} did not add exactly one language use"
        ));
    }
    if next[..prev.len()] != *prev {
        return Err(format!(
            "checkpoint {checkpoint} deleted or reordered language uses"
        ));
    }
    if next.last() != Some(&LanguageUse { variety, domain }) {
        return Err(format!(
            "checkpoint {checkpoint} did not append the expected language use"
        ));
    }
    Ok(())
}

fn expect_same_community_ids(
    prev: &[Community],
    next: &[Community],
    checkpoint: usize,
) -> Result<(), String> {
    if prev.len() != next.len() {
        return Err(format!("checkpoint {checkpoint} changed the community set"));
    }
    for community in prev {
        if !next.iter().any(|item| item.id == community.id) {
            return Err(format!(
                "checkpoint {checkpoint} dropped community {}",
                community.id
            ));
        }
    }
    Ok(())
}

fn expect_same_variety_ids(
    prev: &[Variety],
    next: &[Variety],
    checkpoint: usize,
) -> Result<(), String> {
    if prev.len() != next.len() {
        return Err(format!("checkpoint {checkpoint} changed the variety set"));
    }
    for variety in prev {
        if !next.iter().any(|item| item.id == variety.id) {
            return Err(format!(
                "checkpoint {checkpoint} dropped variety {}",
                variety.id
            ));
        }
    }
    Ok(())
}

fn expect_added_community<'a>(
    prev: &[Community],
    next: &'a [Community],
    name: &str,
    group: &str,
    location: &str,
    ancestry: &str,
) -> Result<&'a Community, String> {
    if next.len() != prev.len() + 1 {
        return Err("event did not add exactly one community".into());
    }
    expect_frozen_communities(prev, next, &[])?;
    let added = next
        .iter()
        .find(|c| !prev.iter().any(|p| p.id == c.id))
        .ok_or_else(|| "missing new community".to_string())?;
    if added.name != name.trim()
        || added.group != group.trim()
        || added.location != location.trim()
        || added.ancestry != ancestry.trim()
    {
        return Err("community metadata does not match the event".into());
    }
    Ok(added)
}

fn expect_added_variety<'a>(
    prev: &[Variety],
    next: &'a [Variety],
    name: &str,
    parent: Option<u64>,
    checkpoint: usize,
) -> Result<&'a Variety, String> {
    if next.len() != prev.len() + 1 {
        return Err(format!(
            "checkpoint {checkpoint} did not add exactly one variety"
        ));
    }
    let added = next
        .iter()
        .find(|v| !prev.iter().any(|p| p.id == v.id))
        .ok_or_else(|| format!("checkpoint {checkpoint} missing new variety"))?;
    if added.name != name.trim() {
        return Err(format!(
            "checkpoint {checkpoint} variety name does not match the event"
        ));
    }
    if added.parent != parent {
        return Err(format!(
            "checkpoint {checkpoint} variety parent does not match the event"
        ));
    }
    if added.founded_at != checkpoint {
        return Err(format!(
            "checkpoint {checkpoint} variety founded_at is wrong"
        ));
    }
    Ok(added)
}

fn expect_frozen_communities(
    prev: &[Community],
    next: &[Community],
    allowed: &[u64],
) -> Result<(), String> {
    for earlier in prev {
        if allowed.contains(&earlier.id) {
            continue;
        }
        let Some(later) = next.iter().find(|c| c.id == earlier.id) else {
            return Err(format!("community {} disappeared", earlier.id));
        };
        if later != earlier {
            return Err(format!(
                "community {} changed outside the event",
                earlier.id
            ));
        }
    }
    Ok(())
}

fn expect_frozen_varieties(
    prev: &[Variety],
    next: &[Variety],
    allowed: &[u64],
) -> Result<(), String> {
    for earlier in prev {
        if allowed.contains(&earlier.id) {
            continue;
        }
        let Some(later) = next.iter().find(|v| v.id == earlier.id) else {
            return Err(format!("variety {} disappeared", earlier.id));
        };
        if !frozen_variety(earlier, later) {
            return Err(format!("variety {} changed outside the event", earlier.id));
        }
    }
    Ok(())
}

fn frozen_variety(a: &Variety, b: &Variety) -> bool {
    a.id == b.id
        && a.name == b.name
        && a.parent == b.parent
        && a.founded_at == b.founded_at
        && a.aesthetic.id == b.aesthetic.id
        && a.consonants == b.consonants
        && a.vowels == b.vowels
        && a.grammar == b.grammar
        && a.lexicon == b.lexicon
}

fn community_meta_same(a: &Community, b: &Community) -> bool {
    a.id == b.id
        && a.name == b.name
        && a.group == b.group
        && a.location == b.location
        && a.ancestry == b.ancestry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Syllable;

    fn found(name: &str, language: &str, aesthetic: &str) -> HistoryEvent {
        HistoryEvent::Found {
            name: name.into(),
            group: name.into(),
            location: "here".into(),
            ancestry: "Unspecified".into(),
            language: FoundLanguage::New {
                name: language.into(),
                aesthetic: aesthetic.into(),
            },
        }
    }

    fn named_community<'a>(history: &'a History, name: &str) -> &'a Community {
        history
            .checkpoints()
            .last()
            .unwrap()
            .communities
            .iter()
            .find(|c| c.name == name)
            .unwrap()
    }

    fn named_variety<'a>(history: &'a History, name: &str) -> &'a Variety {
        history
            .checkpoints()
            .last()
            .unwrap()
            .varieties
            .iter()
            .find(|v| v.name == name)
            .unwrap()
    }

    fn named_variety_at<'a>(history: &'a History, checkpoint: usize, name: &str) -> &'a Variety {
        history.checkpoints()[checkpoint]
            .varieties
            .iter()
            .find(|v| v.name == name)
            .unwrap()
    }

    fn phones(variety: &Variety) -> Vec<(String, Vec<PhonemeId>)> {
        variety
            .lexicon
            .iter()
            .map(|lex| (lex.id.clone(), lex.word.phones()))
            .collect()
    }

    fn checkpoint_fingerprint(cp: &HistoryCheckpoint) -> Vec<(u64, String, Vec<PhonemeId>)> {
        cp.varieties
            .iter()
            .flat_map(|v| {
                v.lexicon
                    .iter()
                    .map(|lex| (v.id, lex.id.clone(), lex.word.phones()))
            })
            .collect()
    }

    fn first_open_option(history: &History) -> (u64, String, u32, String) {
        let cp = history.checkpoints().len() - 1;
        let variety = &history.checkpoints()[cp].varieties[0];
        for lex in &variety.lexicon {
            if lex.retired {
                continue;
            }
            for sense in &lex.senses {
                if let Ok(opts) = history.formations(cp, variety.id, &lex.id, sense.id) {
                    if let Some(opt) = opts.into_iter().find(|opt| opt.existing.is_none()) {
                        return (variety.id, lex.id.clone(), sense.id, opt.construction);
                    }
                }
            }
        }
        panic!("no open formation");
    }

    fn entity_pair(variety: &Variety) -> (String, String) {
        let ids: Vec<String> = variety
            .lexicon
            .iter()
            .filter(|lex| {
                !lex.retired
                    && variety
                        .grammar
                        .classes
                        .iter()
                        .any(|class| class.id == lex.class && class.kind == ClassKind::Entity)
            })
            .map(|lex| lex.id.clone())
            .collect();
        (ids[0].clone(), ids[1].clone())
    }

    #[test]
    fn development_can_feed_a_previously_inapplicable_rule() {
        let mut history = History::new(42);
        history.commit(found("A", "A", "elvish")).unwrap();
        let t = CATALOG.id_by_ipa("t").unwrap();
        let i = CATALOG.id_by_ipa("i").unwrap();
        let variety = &mut history.checkpoints.last_mut().unwrap().varieties[0];
        variety.consonants = ["t", "s", "h"]
            .iter()
            .map(|p| CATALOG.id_by_ipa(p).unwrap())
            .collect();
        variety.vowels = vec![i];
        variety.grammar.constructions.clear();
        variety.lexicon.truncate(1);
        variety.lexicon[0].analysis = None;
        variety.lexicon[0].word = Word {
            syllables: vec![Syllable {
                onset: vec![t],
                nucleus: vec![i],
                coda: vec![],
                long: false,
            }],
            join_at: None,
        };
        variety.lexicon[0].stress = 0;
        variety.lexicon[0].boundaries.clear();
        let id = variety.id;
        history
            .commit(HistoryEvent::Develop {
                variety: id,
                steps: 2,
            })
            .unwrap();
        let word = &history.checkpoints.last().unwrap().varieties[0].lexicon[0].word;
        assert_eq!(CATALOG.ipa_string(&word.phones()), "hi");
    }

    #[test]
    fn preview_is_nonmutating_and_matches_commit() {
        let mut history = History::new(42);
        let found = found("A", "A", "elvish");
        let preview_found = history.preview(found.clone()).unwrap();
        assert_eq!(history.checkpoints().len(), 1);
        history.commit(found).unwrap();
        assert_eq!(history.checkpoints().len(), 2);
        assert_eq!(
            checkpoint_fingerprint(&preview_found),
            checkpoint_fingerprint(history.checkpoints().last().unwrap())
        );

        let develop = HistoryEvent::Develop {
            variety: named_variety(&history, "A").id,
            steps: 2,
        };
        let first = history.preview(develop.clone()).unwrap();
        let second = history.preview(develop.clone()).unwrap();
        assert_eq!(history.checkpoints().len(), 2);
        assert_eq!(first.rules, second.rules);
        assert_eq!(
            checkpoint_fingerprint(&first),
            checkpoint_fingerprint(&second)
        );
        history.commit(develop).unwrap();
        let committed = history.checkpoints().last().unwrap();
        assert_eq!(first.rules, committed.rules);
        assert_eq!(
            checkpoint_fingerprint(&first),
            checkpoint_fingerprint(committed)
        );
    }

    #[test]
    fn separate_branches_develop_independently() {
        let mut history = History::new(7);
        history.commit(found("Parent", "Parent", "elvish")).unwrap();
        let parent_id = named_variety(&history, "Parent").id;
        let community_id = named_community(&history, "Parent").id;
        history
            .commit(HistoryEvent::Separate {
                parent: parent_id,
                name: "Child".into(),
                community: community_id,
            })
            .unwrap();
        let split_parent = phones(named_variety(&history, "Parent"));
        let split_child = phones(named_variety(&history, "Child"));
        assert_eq!(
            split_parent.iter().map(|(_, p)| p).collect::<Vec<_>>(),
            split_child.iter().map(|(_, p)| p).collect::<Vec<_>>()
        );

        let child_id = named_variety(&history, "Child").id;
        history
            .commit(HistoryEvent::Develop {
                variety: parent_id,
                steps: 2,
            })
            .unwrap();
        assert_eq!(phones(named_variety(&history, "Child")), split_child);
        let after_parent = phones(named_variety(&history, "Parent"));

        history
            .commit(HistoryEvent::Develop {
                variety: child_id,
                steps: 2,
            })
            .unwrap();
        assert_eq!(phones(named_variety(&history, "Parent")), after_parent);
        assert_ne!(phones(named_variety(&history, "Child")), split_child);
        assert_eq!(
            named_community(&history, "Parent").group,
            named_community(&history, "Parent").name
        );
    }

    #[test]
    fn loan_provenance_evolves_and_freezes_prior_snapshot() {
        let mut history = History::new(42);
        history.commit(found("Donor", "Donor", "elvish")).unwrap();
        history
            .commit(found("Recipient", "Recipient", "kuo-toa"))
            .unwrap();
        let donor_id = named_variety(&history, "Donor").id;
        let recipient_id = named_variety(&history, "Recipient").id;
        history
            .commit(HistoryEvent::Contact {
                donor: donor_id,
                recipient: recipient_id,
                domain: ContactDomain::General,
                count: 8,
            })
            .unwrap();
        let contact_idx = history.checkpoints().len() - 1;
        let recipient = named_variety(&history, "Recipient");
        let loans: Vec<&Lexeme> = recipient
            .lexicon
            .iter()
            .filter(|lex| matches!(lex.origin, Origin::Borrowed { .. }))
            .collect();
        assert_eq!(loans.len(), 8);
        let donor = named_variety(&history, "Donor");
        for loan in &loans {
            let Origin::Borrowed { source, checkpoint } = &loan.origin else {
                panic!("expected borrowed origin");
            };
            assert_eq!(source.variety, donor_id);
            assert_eq!(*checkpoint, contact_idx);
            assert!(donor.lexicon.iter().any(|d| d.id == source.lexeme));
            assert!(loan.analysis.is_none());
            assert!(
                loan.traces
                    .iter()
                    .any(|trace| trace.checkpoint == contact_idx && trace.variety == recipient_id)
            );
        }
        let frozen: Vec<(String, Vec<PhonemeId>)> = loans
            .iter()
            .map(|lex| (lex.id.clone(), lex.word.phones()))
            .collect();

        history
            .commit(HistoryEvent::Develop {
                variety: recipient_id,
                steps: 5,
            })
            .unwrap();

        let prior = named_variety_at(&history, contact_idx, "Recipient");
        for (id, old_phones) in &frozen {
            let lex = prior.lexicon.iter().find(|l| l.id == *id).unwrap();
            assert_eq!(lex.word.phones(), *old_phones);
        }
        let now = named_variety(&history, "Recipient");
        assert!(
            frozen.iter().any(|(id, old_phones)| {
                now.lexicon
                    .iter()
                    .find(|l| l.id == *id)
                    .unwrap()
                    .word
                    .phones()
                    != *old_phones
            }),
            "borrowed forms should evolve under later development"
        );
        assert_eq!(
            phones(named_variety(&history, "Donor")),
            phones(named_variety_at(&history, contact_idx, "Donor"))
        );
    }

    #[test]
    fn invalid_events_leave_history_unchanged() {
        let mut history = History::new(3);
        history.commit(found("A", "A", "elvish")).unwrap();
        let before_len = history.checkpoints().len();
        let before = checkpoint_fingerprint(history.checkpoints().last().unwrap());
        let variety = named_variety(&history, "A").id;

        assert!(history.commit(found("  ", "B", "elvish")).is_err());
        assert!(history.commit(found("B", "B", "no-such-pack")).is_err());
        assert!(
            history
                .commit(HistoryEvent::Separate {
                    parent: 99,
                    name: "X".into(),
                    community: named_community(&history, "A").id,
                })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Develop { variety, steps: 0 })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Develop { variety, steps: 6 })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Contact {
                    donor: variety,
                    recipient: variety,
                    domain: ContactDomain::General,
                    count: 3,
                })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Contact {
                    donor: variety,
                    recipient: 99,
                    domain: ContactDomain::Maritime,
                    count: 2,
                })
                .is_err()
        );
        let (id, base, sense, construction) = first_open_option(&history);
        assert!(
            history
                .commit(HistoryEvent::Remodel {
                    variety: id,
                    lexeme: base.clone(),
                    base,
                    sense,
                    construction,
                })
                .is_err()
        );

        assert_eq!(history.checkpoints().len(), before_len);
        assert_eq!(
            checkpoint_fingerprint(history.checkpoints().last().unwrap()),
            before
        );
    }

    #[test]
    fn derive_preview_duplicate_and_absent_from_prior() {
        let mut history = History::new(42);
        history.commit(found("A", "A", "elvish")).unwrap();
        let found_idx = history.checkpoints().len() - 1;
        let (id, base, sense, construction) = first_open_option(&history);
        let event = HistoryEvent::Derive {
            variety: id,
            base: base.clone(),
            sense,
            construction: construction.clone(),
        };
        let preview = history.preview(event.clone()).unwrap();
        assert_eq!(history.checkpoints().len(), found_idx + 1);
        history.commit(event.clone()).unwrap();
        assert_eq!(
            checkpoint_fingerprint(&preview),
            checkpoint_fingerprint(history.checkpoints().last().unwrap())
        );
        let derived = named_variety(&history, "A")
            .lexicon
            .iter()
            .find(|lex| {
                lex.analysis.as_ref().is_some_and(|analysis| {
                    analysis.base == base
                        && analysis.sense == sense
                        && analysis.construction == construction
                })
            })
            .unwrap();
        let derived_id = derived.id.clone();
        assert!(
            named_variety_at(&history, found_idx, "A")
                .lexicon
                .iter()
                .all(|lex| lex.id != derived_id)
        );
        assert!(history.preview(event.clone()).is_err());
        assert!(history.commit(event).is_err());
        let attested = history
            .formations(history.checkpoints().len() - 1, id, &base, sense)
            .unwrap()
            .into_iter()
            .find(|opt| opt.construction == construction)
            .unwrap();
        assert_eq!(attested.existing.as_deref(), Some(derived_id.as_str()));
        assert_eq!(
            attested.word,
            named_variety(&history, "A")
                .lexicon
                .iter()
                .find(|lex| lex.id == derived_id)
                .unwrap()
                .word
        );
    }

    #[test]
    fn separate_remaps_analysis_not_origin_refs() {
        let mut history = History::new(7);
        history.commit(found("Parent", "Parent", "elvish")).unwrap();
        let parent_id = named_variety(&history, "Parent").id;
        let community_id = named_community(&history, "Parent").id;
        let parent = named_variety(&history, "Parent");
        let parent_derived = parent
            .lexicon
            .iter()
            .find(|lex| lex.analysis.is_some())
            .unwrap();
        let parent_derived_id = parent_derived.id.clone();
        let parent_analysis = parent_derived.analysis.clone().unwrap();
        let parent_origin = parent_derived.origin.clone();
        history
            .commit(HistoryEvent::Separate {
                parent: parent_id,
                name: "Child".into(),
                community: community_id,
            })
            .unwrap();
        let split_at = history.checkpoints().len() - 1;
        let child = named_variety(&history, "Child");
        let child_derived = child
            .lexicon
            .iter()
            .find(|lex| {
                matches!(
                    &lex.origin,
                    Origin::Inherited { source, .. } if source.lexeme == parent_derived_id
                )
            })
            .unwrap();
        let Origin::Inherited { source, checkpoint } = &child_derived.origin else {
            panic!("expected inherited origin");
        };
        assert_eq!(source.variety, parent_id);
        assert_eq!(source.checkpoint, split_at - 1);
        assert_eq!(*checkpoint, split_at);
        let child_analysis = child_derived.analysis.as_ref().unwrap();
        assert_ne!(child_analysis.base, parent_analysis.base);
        assert_eq!(child_analysis.sense, parent_analysis.sense);
        assert_eq!(child_analysis.construction, parent_analysis.construction);
        assert!(
            child
                .lexicon
                .iter()
                .any(|lex| lex.id == child_analysis.base)
        );
        let parent_now = named_variety(&history, "Parent");
        let parent_same = parent_now
            .lexicon
            .iter()
            .find(|lex| lex.id == parent_derived_id)
            .unwrap();
        assert_eq!(parent_same.analysis.as_ref(), Some(&parent_analysis));
        assert_eq!(parent_same.origin, parent_origin);
        assert!(
            named_community(&history, "Parent")
                .uses
                .iter()
                .any(|usage| usage.variety == child.id && usage.domain == UseDomain::Home)
        );
    }

    #[test]
    fn develop_keeps_a_words_only_vowel() {
        let mut history = History::new(42);
        history.commit(found("A", "A", "elvish")).unwrap();
        let id = named_variety(&history, "A").id;
        history
            .commit(HistoryEvent::Develop {
                variety: id,
                steps: 5,
            })
            .unwrap();
        for lex in &named_variety(&history, "A").lexicon {
            assert!(
                lex.word.phonemes().any(|id| CATALOG.get(id).is_vowel()),
                "lexeme '{}' lost its last vowel",
                lex.id
            );
        }
    }

    #[test]
    fn from_json_rejects_old_schema() {
        let err = History::from_json(r#"{"seed":1,"version":2,"checkpoints":[]}"#).unwrap_err();
        assert!(err.contains("unsupported history version 2"));
    }

    #[test]
    fn coastal_scenario_keeps_social_and_language_identities() {
        let history = History::coastal_scenario(42, Aesthetic::by_id("elvish").unwrap()).unwrap();
        let last = history.checkpoints().last().unwrap();
        let inland = last
            .communities
            .iter()
            .find(|c| c.name == "Inland")
            .unwrap();
        let coast = last.communities.iter().find(|c| c.name == "Coast").unwrap();
        let inland_speech = last
            .varieties
            .iter()
            .find(|v| v.name == "Inland speech")
            .unwrap();
        let coastal_speech = last
            .varieties
            .iter()
            .find(|v| v.name == "Coastal speech")
            .unwrap();
        let colony = last
            .varieties
            .iter()
            .find(|v| v.name == "Colony speech")
            .unwrap();
        assert_eq!(inland.group, "Inland");
        assert_eq!(coast.group, "Coast");
        assert_eq!(inland.ancestry, "Unspecified");
        assert_eq!(coast.ancestry, "Unspecified");
        assert_eq!(inland_speech.aesthetic.id, "elvish");
        assert_eq!(coastal_speech.aesthetic.id, "kuo-toa");
        assert_eq!(colony.parent, Some(inland_speech.id));
        assert!(
            inland.uses.iter().any(|usage| {
                usage.variety == inland_speech.id && usage.domain == UseDomain::Home
            })
        );
        assert!(coast.uses.iter().any(|usage| {
            usage.variety == coastal_speech.id && usage.domain == UseDomain::Home
        }));
        assert!(coast.uses.iter().any(|usage| {
            usage.variety == inland_speech.id && usage.domain == UseDomain::Trade
        }));
        assert!(
            coast
                .uses
                .iter()
                .any(|usage| { usage.variety == colony.id && usage.domain == UseDomain::Home })
        );
        assert_eq!(coast.uses.len(), 3);
        let json = history.to_json().unwrap();
        let loaded = History::from_json(&json).unwrap();
        assert_eq!(loaded.seed(), history.seed());
        assert_eq!(loaded.checkpoints().len(), history.checkpoints().len());
    }

    #[test]
    fn found_existing_and_use_language_share_a_variety() {
        let mut history = History::new(9);
        history.commit(found("A", "Speech", "elvish")).unwrap();
        let variety = named_variety(&history, "Speech").id;
        history
            .commit(HistoryEvent::Found {
                name: "B".into(),
                group: "B".into(),
                location: "elsewhere".into(),
                ancestry: "Unspecified".into(),
                language: FoundLanguage::Existing { variety },
            })
            .unwrap();
        assert_eq!(history.checkpoints().last().unwrap().varieties.len(), 1);
        assert_eq!(named_community(&history, "B").uses[0].variety, variety);
        history
            .commit(HistoryEvent::UseLanguage {
                community: named_community(&history, "B").id,
                variety,
                domain: UseDomain::Ritual,
            })
            .unwrap();
        let uses = &named_community(&history, "B").uses;
        assert_eq!(uses.len(), 2);
        assert_eq!(uses[1].domain, UseDomain::Ritual);
        assert!(
            history
                .commit(HistoryEvent::UseLanguage {
                    community: named_community(&history, "B").id,
                    variety,
                    domain: UseDomain::Ritual,
                })
                .is_err()
        );
    }

    #[test]
    fn sense_replace_lexicalize_and_remodel_keep_provenance() {
        let mut history = History::new(42);
        history.commit(found("A", "A", "elvish")).unwrap();
        let variety_id = named_variety(&history, "A").id;
        let water = named_variety(&history, "A")
            .lexicon
            .iter()
            .find(|lex| lex.id.ends_with(":water"))
            .unwrap();
        let water_id = water.id.clone();
        let water_class = named_variety(&history, "A")
            .grammar
            .classes
            .iter()
            .find(|class| class.id == water.class)
            .unwrap()
            .clone();
        let frame = SemanticFrame::Entity {
            countable: matches!(water_class.kind, ClassKind::Entity) && false,
        };
        let extend = HistoryEvent::ExtendSense {
            variety: variety_id,
            lexeme: water_id.clone(),
            gloss: "ice".into(),
            frame: frame.clone(),
        };
        let preview = history.preview(extend.clone()).unwrap();
        assert_eq!(history.checkpoints().len(), 2);
        history.commit(extend.clone()).unwrap();
        assert_eq!(
            checkpoint_fingerprint(&preview),
            checkpoint_fingerprint(history.checkpoints().last().unwrap())
        );
        {
            let variety = named_variety(&history, "A");
            let after_extend = variety
                .lexicon
                .iter()
                .find(|lex| lex.id == water_id)
                .unwrap();
            assert!(
                after_extend
                    .senses
                    .iter()
                    .any(|sense| sense.meaning.label() == "ice")
            );
            let extend_form = variety.form(&after_extend.word);
            let extend_at = history.checkpoints().len() - 1;
            assert!(after_extend.traces.iter().any(|trace| {
                trace.checkpoint == extend_at
                    && trace.after == extend_form
                    && trace.before.as_deref() == Some(extend_form.as_str())
                    && trace.explanation.contains("extended")
            }));
        }
        assert!(history.commit(extend).is_err());

        let shift = HistoryEvent::ShiftSense {
            variety: variety_id,
            lexeme: water_id.clone(),
            sense: 0,
            gloss: "river".into(),
            frame: frame.clone(),
        };
        history.commit(shift.clone()).unwrap();
        {
            let variety = named_variety(&history, "A");
            let shifted = variety
                .lexicon
                .iter()
                .find(|lex| lex.id == water_id)
                .unwrap();
            assert_eq!(shifted.senses[0].meaning.label(), "river");
            let shift_form = variety.form(&shifted.word);
            let shift_at = history.checkpoints().len() - 1;
            assert!(
                shifted
                    .traces
                    .iter()
                    .any(|trace| { trace.checkpoint == shift_at && trace.after == shift_form })
            );
        }
        assert!(history.commit(shift).is_err());

        let fishing = named_variety(&history, "A")
            .lexicon
            .iter()
            .find(|lex| lex.id.ends_with(":fishing"))
            .unwrap();
        let derived = named_variety(&history, "A")
            .lexicon
            .iter()
            .find(|lex| {
                lex.analysis
                    .as_ref()
                    .is_some_and(|analysis| analysis.base == fishing.id)
            })
            .unwrap();
        let derived_id = derived.id.clone();
        let derived_meaning = derived.senses[0].meaning.clone();
        let fishing_id = fishing.id.clone();
        let fishing_frame = fishing.senses[0].frame.clone();
        history
            .commit(HistoryEvent::ShiftSense {
                variety: variety_id,
                lexeme: fishing_id.clone(),
                sense: 0,
                gloss: "trawling".into(),
                frame: fishing_frame,
            })
            .unwrap();
        let derived_now = named_variety(&history, "A")
            .lexicon
            .iter()
            .find(|lex| lex.id == derived_id)
            .unwrap();
        assert_eq!(derived_now.senses[0].meaning, derived_meaning);
        assert_eq!(
            named_variety(&history, "A")
                .lexicon
                .iter()
                .find(|lex| lex.id == fishing_id)
                .unwrap()
                .senses[0]
                .meaning
                .label(),
            "trawling"
        );

        let (left, right) = entity_pair(named_variety(&history, "A"));
        let prior_target_senses = named_variety(&history, "A")
            .lexicon
            .iter()
            .find(|lex| lex.id == right)
            .unwrap()
            .senses
            .clone();
        history
            .commit(HistoryEvent::Replace {
                variety: variety_id,
                lexeme: left.clone(),
                replacement: right.clone(),
            })
            .unwrap();
        {
            let variety = named_variety(&history, "A");
            let source = variety.lexicon.iter().find(|lex| lex.id == left).unwrap();
            let target = variety.lexicon.iter().find(|lex| lex.id == right).unwrap();
            assert!(source.retired);
            assert!(!target.retired);
            let replace_at = history.checkpoints().len() - 1;
            assert!(
                source
                    .traces
                    .iter()
                    .any(|trace| { trace.checkpoint == replace_at })
            );
            assert!(
                target
                    .traces
                    .iter()
                    .any(|trace| { trace.checkpoint == replace_at })
            );
            assert!(
                source
                    .senses
                    .iter()
                    .chain(&prior_target_senses)
                    .all(|sense| {
                        target.senses.iter().any(|existing| {
                            existing.meaning == sense.meaning && existing.frame == sense.frame
                        })
                    })
            );
        }
        assert!(
            history
                .commit(HistoryEvent::Replace {
                    variety: variety_id,
                    lexeme: left.clone(),
                    replacement: right.clone(),
                })
                .is_err()
        );

        let analysis = named_variety(&history, "A")
            .lexicon
            .iter()
            .find(|lex| lex.id == derived_id)
            .unwrap()
            .analysis
            .clone()
            .unwrap();
        let origin = named_variety(&history, "A")
            .lexicon
            .iter()
            .find(|lex| lex.id == derived_id)
            .unwrap()
            .origin
            .clone();
        history
            .commit(HistoryEvent::Lexicalize {
                variety: variety_id,
                lexeme: derived_id.clone(),
            })
            .unwrap();
        {
            let variety = named_variety(&history, "A");
            let lexicalized = variety
                .lexicon
                .iter()
                .find(|lex| lex.id == derived_id)
                .unwrap();
            assert!(lexicalized.analysis.is_none());
            assert_eq!(lexicalized.origin, origin);
            let lex_form = variety.form(&lexicalized.word);
            assert_eq!(lexicalized.traces.last().unwrap().after, lex_form);
            assert!(
                lexicalized
                    .traces
                    .last()
                    .unwrap()
                    .explanation
                    .contains("analysis")
            );
        }
        history
            .commit(HistoryEvent::Remodel {
                variety: variety_id,
                lexeme: derived_id.clone(),
                base: analysis.base.clone(),
                sense: analysis.sense,
                construction: analysis.construction.clone(),
            })
            .unwrap();
        let remodeled = named_variety(&history, "A")
            .lexicon
            .iter()
            .find(|lex| lex.id == derived_id)
            .unwrap();
        assert_eq!(remodeled.origin, origin);
        assert_eq!(
            remodeled.analysis.as_ref().unwrap().construction,
            analysis.construction
        );
        history.validate().unwrap();
    }
}

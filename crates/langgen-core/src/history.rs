use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::Language;
use crate::aesthetic::Aesthetic;
use crate::change::{SoundChange, apply_changes};
use crate::change_packs::{conservative, radical, rule_detail};
use crate::contact::nativize_word;
use crate::generate::{Syllable, Word};
use crate::inventory::Inventory;
use crate::lexicon::{founding_stem, mint_roots};
use crate::ortho;
use crate::phoneme::{CATALOG, PhonemeId};

const HISTORY_VERSION: u32 = 2;

const MARITIME_GLOSSES: &[&str] = &[
    "sea", "water", "river", "wind", "sky", "path", "food", "fish",
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
pub enum FormationKind {
    Agent,
    Place,
    Collective,
}

impl FormationKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Place => "place",
            Self::Collective => "collective",
        }
    }

    fn all() -> [Self; 3] {
        [Self::Agent, Self::Place, Self::Collective]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormationRule {
    pub kind: FormationKind,
    pub exponent: Word,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormationOption {
    pub kind: FormationKind,
    pub label: String,
    pub gloss: String,
    pub word: Word,
    pub base: String,
    pub exponent: Word,
    pub existing: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HistoryEvent {
    Found {
        name: String,
        aesthetic: String,
    },
    Separate {
        parent: u64,
        name: String,
    },
    Develop {
        community: u64,
        steps: usize,
    },
    Contact {
        donor: u64,
        recipient: u64,
        domain: ContactDomain,
        count: usize,
    },
    Derive {
        community: u64,
        base: String,
        kind: FormationKind,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WordTrace {
    pub checkpoint: usize,
    pub community: u64,
    pub before: Option<String>,
    pub after: String,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryLexeme {
    pub id: String,
    pub gloss: String,
    pub word: Word,
    pub origin_community: u64,
    pub origin_lexeme: String,
    pub base_lexeme: Option<String>,
    pub formation: Option<FormationKind>,
    pub traces: Vec<WordTrace>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEffect {
    pub community: u64,
    pub lexeme: String,
    pub gloss: String,
    pub before: Option<String>,
    pub after: String,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Community {
    pub id: u64,
    pub name: String,
    pub parent: Option<u64>,
    pub founded_at: usize,
    pub aesthetic: Aesthetic,
    pub consonants: Vec<PhonemeId>,
    pub vowels: Vec<PhonemeId>,
    pub lexicon: Vec<HistoryLexeme>,
    pub formations: Vec<FormationRule>,
}

impl Community {
    pub fn form(&self, word: &Word) -> String {
        ortho::romanize(word, &self.aesthetic)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryCheckpoint {
    pub id: usize,
    pub event: Option<HistoryEvent>,
    pub summary: String,
    pub communities: Vec<Community>,
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
        community: u64,
        base: &str,
    ) -> Result<Vec<FormationOption>, String> {
        let cp = self
            .checkpoints
            .get(checkpoint)
            .ok_or_else(|| format!("unknown checkpoint {checkpoint}"))?;
        let community = cp
            .communities
            .iter()
            .find(|c| c.id == community)
            .ok_or_else(|| format!("unknown community {community}"))?;
        let base_lex = community
            .lexicon
            .iter()
            .find(|lex| lex.id == base)
            .ok_or_else(|| format!("no lexeme '{base}' in community {}", community.id))?;
        let mut options = Vec::new();
        for kind in FormationKind::all() {
            let Some(rule) = community.formations.iter().find(|rule| rule.kind == kind) else {
                continue;
            };
            let existing = community.lexicon.iter().find(|lex| {
                lex.base_lexeme.as_deref() == Some(base) && lex.formation == Some(kind)
            });
            options.push(FormationOption {
                kind,
                label: kind.label().to_string(),
                gloss: formation_gloss(kind, &base_lex.gloss),
                word: existing
                    .map(|lex| lex.word.clone())
                    .unwrap_or_else(|| attach_exponent(&base_lex.word, &rule.exponent)),
                base: base.to_string(),
                exponent: rule.exponent.clone(),
                existing: existing.map(|lex| lex.id.clone()),
            });
        }
        Ok(options)
    }

    pub fn preview(&self, event: HistoryEvent) -> Result<HistoryCheckpoint, String> {
        let latest = self.latest()?;
        let mut communities = latest.communities.clone();
        let mut effects = Vec::new();
        let mut rules = Vec::new();
        let checkpoint = latest.id + 1;
        let summary = match &event {
            HistoryEvent::Found { name, aesthetic } => apply_found(
                &mut communities,
                &mut effects,
                self.seed,
                checkpoint,
                name,
                aesthetic,
            )?,
            HistoryEvent::Separate { parent, name } => {
                apply_separate(&mut communities, checkpoint, *parent, name)?
            }
            HistoryEvent::Develop { community, steps } => apply_develop(
                &mut communities,
                &mut effects,
                &mut rules,
                self.seed,
                checkpoint,
                *community,
                *steps,
            )?,
            HistoryEvent::Contact {
                donor,
                recipient,
                domain,
                count,
            } => apply_contact(
                &mut communities,
                &mut effects,
                checkpoint,
                *donor,
                *recipient,
                *domain,
                *count,
            )?,
            HistoryEvent::Derive {
                community,
                base,
                kind,
            } => apply_derive(
                &mut communities,
                &mut effects,
                checkpoint,
                *community,
                base,
                *kind,
            )?,
        };
        Ok(HistoryCheckpoint {
            id: checkpoint,
            event: Some(event),
            summary,
            communities,
            effects,
            rules,
        })
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
        Ok(())
    }

    pub fn coastal_scenario(seed: u64, aesthetic: &str) -> Result<Self, String> {
        let other = if aesthetic == "kuo-toa" {
            "elvish"
        } else {
            "kuo-toa"
        };
        let mut history = Self::new(seed);
        history.commit(HistoryEvent::Found {
            name: "Inland".into(),
            aesthetic: aesthetic.into(),
        })?;
        history.commit(HistoryEvent::Found {
            name: "Coast".into(),
            aesthetic: other.into(),
        })?;
        let inland = named_id(&history, "Inland")?;
        let coast = named_id(&history, "Coast")?;
        history.commit(HistoryEvent::Separate {
            parent: inland,
            name: "Colony".into(),
        })?;
        history.commit(HistoryEvent::Develop {
            community: inland,
            steps: 2,
        })?;
        let colony = named_id(&history, "Colony")?;
        history.commit(HistoryEvent::Contact {
            donor: coast,
            recipient: colony,
            domain: ContactDomain::Maritime,
            count: 12,
        })?;
        history.commit(HistoryEvent::Develop {
            community: colony,
            steps: 5,
        })?;
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
        effects: Vec::new(),
        rules: Vec::new(),
    }
}

fn named_id(history: &History, name: &str) -> Result<u64, String> {
    history
        .latest()?
        .communities
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.id)
        .ok_or_else(|| format!("missing community '{name}'"))
}

fn next_community_id(communities: &[Community]) -> u64 {
    communities.iter().map(|c| c.id).max().unwrap_or(0) + 1
}

fn mix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn founder_seed(history_seed: u64, community: u64) -> u64 {
    mix64(history_seed ^ mix64(community.wrapping_add(1)))
}

fn event_seed(history_seed: u64, community: u64, checkpoint: usize, steps: usize) -> u64 {
    mix64(history_seed ^ mix64(community) ^ mix64(checkpoint as u64) ^ mix64(steps as u64))
}

fn validate_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        return Err("community name must be nonempty and contain no control characters".into());
    }
    Ok(name.to_string())
}

fn apply_found(
    communities: &mut Vec<Community>,
    effects: &mut Vec<HistoryEffect>,
    history_seed: u64,
    checkpoint: usize,
    name: &str,
    aesthetic_id: &str,
) -> Result<String, String> {
    let name = validate_name(name)?;
    if communities.iter().any(|c| c.name == name) {
        return Err(format!("community name '{name}' already exists"));
    }
    let aesthetic = Aesthetic::by_id(aesthetic_id)
        .ok_or_else(|| format!("unknown aesthetic '{aesthetic_id}'"))?;
    let id = next_community_id(communities);
    let lang = Language::new(founder_seed(history_seed, id), aesthetic.clone());
    let roots = mint_roots(&lang);
    let lex_ids: Vec<String> = (0..roots.len()).map(|i| format!("c{id}:r{i}")).collect();
    let mut lexicon = Vec::with_capacity(roots.len());
    for (i, root) in roots.iter().enumerate() {
        let lex_id = lex_ids[i].clone();
        let after = ortho::romanize(&root.proto, &aesthetic);
        let gloss = root.id.to_string();
        let base = founding_stem(root.id);
        let base_lexeme = base.and_then(|stem| {
            roots
                .iter()
                .position(|r| r.id == stem)
                .map(|j| lex_ids[j].clone())
        });
        effects.push(HistoryEffect {
            community: id,
            lexeme: lex_id.clone(),
            gloss: gloss.clone(),
            before: None,
            after: after.clone(),
            explanation: format!("founded {name}"),
        });
        lexicon.push(HistoryLexeme {
            id: lex_id.clone(),
            gloss,
            word: root.proto.clone(),
            origin_community: id,
            origin_lexeme: lex_id,
            base_lexeme,
            formation: None,
            traces: vec![WordTrace {
                checkpoint,
                community: id,
                before: None,
                after,
                explanation: base.map_or_else(
                    || "coined as a proto root".into(),
                    |stem| format!("formed from {stem} in the founding lexicon"),
                ),
            }],
        });
    }
    let mut suffix_rng = StdRng::seed_from_u64(founder_seed(history_seed, id).wrapping_add(29));
    let formations = mint_formation_rules(&lang.inventory, &mut suffix_rng);
    communities.push(Community {
        id,
        name: name.clone(),
        parent: None,
        founded_at: checkpoint,
        aesthetic,
        consonants: lang.inventory.consonants.clone(),
        vowels: lang.inventory.vowels.clone(),
        lexicon,
        formations,
    });
    Ok(format!("Founded {name}"))
}

fn apply_separate(
    communities: &mut Vec<Community>,
    checkpoint: usize,
    parent_id: u64,
    name: &str,
) -> Result<String, String> {
    let name = validate_name(name)?;
    if communities.iter().any(|c| c.name == name) {
        return Err(format!("community name '{name}' already exists"));
    }
    let parent = communities
        .iter()
        .find(|c| c.id == parent_id)
        .cloned()
        .ok_or_else(|| format!("unknown parent community {parent_id}"))?;
    let id = next_community_id(communities);
    let new_ids: Vec<String> = parent
        .lexicon
        .iter()
        .enumerate()
        .map(|(i, _)| format!("c{id}:r{i}"))
        .collect();
    let lexicon = parent
        .lexicon
        .iter()
        .enumerate()
        .map(|(i, lex)| {
            let mut traces = lex.traces.clone();
            let form = parent.form(&lex.word);
            traces.push(WordTrace {
                checkpoint,
                community: id,
                before: Some(form.clone()),
                after: form,
                explanation: format!("inherited from {} at separation", parent.name),
            });
            let base_lexeme = lex.base_lexeme.as_ref().and_then(|base| {
                parent
                    .lexicon
                    .iter()
                    .position(|l| &l.id == base)
                    .map(|j| new_ids[j].clone())
            });
            HistoryLexeme {
                id: new_ids[i].clone(),
                gloss: lex.gloss.clone(),
                word: lex.word.clone(),
                origin_community: lex.origin_community,
                origin_lexeme: lex.origin_lexeme.clone(),
                base_lexeme,
                formation: lex.formation,
                traces,
            }
        })
        .collect();
    let parent_name = parent.name.clone();
    communities.push(Community {
        id,
        name: name.clone(),
        parent: Some(parent_id),
        founded_at: checkpoint,
        aesthetic: parent.aesthetic,
        consonants: parent.consonants,
        vowels: parent.vowels,
        lexicon,
        formations: parent.formations,
    });
    Ok(format!("{name} separates from {parent_name}"))
}

fn apply_develop(
    communities: &mut [Community],
    effects: &mut Vec<HistoryEffect>,
    rules: &mut Vec<SoundChange>,
    history_seed: u64,
    checkpoint: usize,
    community_id: u64,
    steps: usize,
) -> Result<String, String> {
    if !(1..=5).contains(&steps) {
        return Err(format!("develop steps must be 1..=5, got {steps}"));
    }
    let community = community_mut(communities, community_id)?;
    let before: Vec<(String, String, Vec<PhonemeId>)> = community
        .lexicon
        .iter()
        .map(|lex| (lex.id.clone(), community.form(&lex.word), lex.word.phones()))
        .collect();
    let affix_before: Vec<(FormationKind, String, Vec<PhonemeId>)> = community
        .formations
        .iter()
        .map(|rule| {
            (
                rule.kind,
                community.form(&rule.exponent),
                rule.exponent.phones(),
            )
        })
        .collect();
    let mut rng = StdRng::seed_from_u64(event_seed(history_seed, community_id, checkpoint, steps));
    let mut pool = pack_rules();
    let mut affix_explanations = Vec::new();
    for _ in 0..steps {
        let applicable: Vec<usize> = pool
            .iter()
            .enumerate()
            .filter_map(|(i, rule)| rule_applies(rule, community).then_some(i))
            .collect();
        if applicable.is_empty() {
            break;
        }
        let weights: Vec<f32> = applicable
            .iter()
            .map(|&i| rule_weight(&community.aesthetic.id, &pool[i].id))
            .collect();
        let idx = applicable[pick_weighted(&mut rng, &weights)];
        let rule = pool.remove(idx);
        apply_rule_to_community(community, &rule, checkpoint, &mut affix_explanations);
        rules.push(rule);
    }
    absorb_novel_phones(community);
    for (id, form_before, phones_before) in before {
        let Some(lex) = community
            .lexicon
            .iter()
            .find(|lex| lex.id == id && lex.word.phones() != phones_before)
        else {
            continue;
        };
        effects.push(HistoryEffect {
            community: community_id,
            lexeme: lex.id.clone(),
            gloss: lex.gloss.clone(),
            before: Some(form_before),
            after: community.form(&lex.word),
            explanation: lex
                .traces
                .iter()
                .filter(|trace| trace.checkpoint == checkpoint)
                .map(|trace| trace.explanation.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        });
    }
    for (kind, form_before, phones_before) in affix_before {
        let Some(rule) = community
            .formations
            .iter()
            .find(|rule| rule.kind == kind && rule.exponent.phones() != phones_before)
        else {
            continue;
        };
        effects.push(HistoryEffect {
            community: community_id,
            lexeme: String::new(),
            gloss: format!("{} suffix", kind.label()),
            before: Some(form_before),
            after: community.form(&rule.exponent),
            explanation: affix_explanations
                .iter()
                .filter(|(changed_kind, _)| *changed_kind == kind)
                .map(|(_, explanation)| explanation.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        });
    }
    Ok(format!(
        "{} develops ({} changes)",
        community.name,
        rules.len()
    ))
}

fn apply_contact(
    communities: &mut [Community],
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
    if !communities.iter().any(|c| c.id == donor_id) {
        return Err(format!("unknown donor community {donor_id}"));
    }
    if !communities.iter().any(|c| c.id == recipient_id) {
        return Err(format!("unknown recipient community {recipient_id}"));
    }
    let donor = communities
        .iter()
        .find(|c| c.id == donor_id)
        .cloned()
        .ok_or_else(|| format!("unknown donor community {donor_id}"))?;
    let recipient = community_mut(communities, recipient_id)?;
    let inventory = community_inventory(recipient);
    let existing: Vec<String> = recipient.lexicon.iter().map(|lex| lex.id.clone()).collect();
    let mut borrowed = 0usize;
    for source in contact_candidates(&donor, domain) {
        if borrowed >= count {
            break;
        }
        let loan_id = loan_lexeme_id(recipient_id, donor_id, &source.id);
        if existing.iter().any(|id| id == &loan_id) {
            continue;
        }
        let nativized = nativize_word(&source.word, &inventory);
        let before = donor.form(&source.word);
        let after = recipient.form(&nativized);
        let mut traces = source.traces.clone();
        traces.push(WordTrace {
            checkpoint,
            community: recipient_id,
            before: Some(before.clone()),
            after: after.clone(),
            explanation: format!("loaned from {} ({})", donor.name, domain.label()),
        });
        effects.push(HistoryEffect {
            community: recipient_id,
            lexeme: loan_id.clone(),
            gloss: source.gloss.clone(),
            before: Some(before),
            after,
            explanation: format!("loaned from {} ({})", donor.name, domain.label()),
        });
        recipient.lexicon.push(HistoryLexeme {
            id: loan_id,
            gloss: source.gloss.clone(),
            word: nativized,
            origin_community: source.origin_community,
            origin_lexeme: source.origin_lexeme.clone(),
            base_lexeme: None,
            formation: None,
            traces,
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
    communities: &mut [Community],
    effects: &mut Vec<HistoryEffect>,
    checkpoint: usize,
    community_id: u64,
    base: &str,
    kind: FormationKind,
) -> Result<String, String> {
    if base.trim().is_empty() {
        return Err("derive base must be nonempty".into());
    }
    let community = community_mut(communities, community_id)?;
    if let Some(existing) = existing_derivation(community, base, kind) {
        return Err(format!(
            "{} already has a {} formation from '{base}' ({existing})",
            community.name,
            kind.label()
        ));
    }
    let base_lex = community
        .lexicon
        .iter()
        .find(|lex| lex.id == base)
        .ok_or_else(|| format!("no lexeme '{base}' in community {community_id}"))?
        .clone();
    let exponent = community
        .formations
        .iter()
        .find(|rule| rule.kind == kind)
        .map(|rule| rule.exponent.clone())
        .ok_or_else(|| format!("{} has no {} formation", community.name, kind.label()))?;
    let word = attach_exponent(&base_lex.word, &exponent);
    let after = community.form(&word);
    let gloss = formation_gloss(kind, &base_lex.gloss);
    let lex_id = format!("c{community_id}:drv:{base}:{}", kind.label());
    let mut traces = base_lex.traces.clone();
    traces.push(WordTrace {
        checkpoint,
        community: community_id,
        before: Some(community.form(&base_lex.word)),
        after: after.clone(),
        explanation: format!("{} formation from {}", kind.label(), base_lex.gloss),
    });
    effects.push(HistoryEffect {
        community: community_id,
        lexeme: lex_id.clone(),
        gloss: gloss.clone(),
        before: None,
        after: after.clone(),
        explanation: format!("{} formation from {}", kind.label(), base_lex.gloss),
    });
    let name = community.name.clone();
    community.lexicon.push(HistoryLexeme {
        id: lex_id.clone(),
        gloss,
        word,
        origin_community: community_id,
        origin_lexeme: lex_id,
        base_lexeme: Some(base.to_string()),
        formation: Some(kind),
        traces,
    });
    Ok(format!(
        "{name} forms {} from {}",
        kind.label(),
        base_lex.gloss
    ))
}

fn community_mut(communities: &mut [Community], id: u64) -> Result<&mut Community, String> {
    communities
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("unknown community {id}"))
}

fn community_inventory(community: &Community) -> Inventory {
    Inventory::from_ids(
        community
            .consonants
            .iter()
            .copied()
            .chain(community.vowels.iter().copied()),
    )
}

fn loan_lexeme_id(recipient: u64, donor: u64, source_id: &str) -> String {
    format!("c{recipient}:loan:{donor}:{source_id}")
}

fn contact_candidates(donor: &Community, domain: ContactDomain) -> Vec<&HistoryLexeme> {
    let mut items: Vec<&HistoryLexeme> = donor.lexicon.iter().collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));
    match domain {
        ContactDomain::General => items,
        ContactDomain::Maritime => {
            let mut preferred = Vec::new();
            let mut rest = Vec::new();
            for lex in items {
                if MARITIME_GLOSSES.contains(&lex.gloss.as_str()) {
                    preferred.push(lex);
                } else {
                    rest.push(lex);
                }
            }
            preferred.sort_by_key(|lex| {
                (
                    MARITIME_GLOSSES
                        .iter()
                        .position(|g| *g == lex.gloss.as_str())
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

fn rule_applies(rule: &SoundChange, community: &Community) -> bool {
    community.lexicon.iter().any(|lex| {
        !apply_changes(&lex.word, std::slice::from_ref(rule))
            .phonemes()
            .eq(lex.word.phonemes())
    }) || community.formations.iter().any(|formation| {
        !apply_changes(&formation.exponent, std::slice::from_ref(rule))
            .phonemes()
            .eq(formation.exponent.phonemes())
    })
}

fn apply_rule_to_community(
    community: &mut Community,
    rule: &SoundChange,
    checkpoint: usize,
    affix_explanations: &mut Vec<(FormationKind, String)>,
) {
    let aesthetic = &community.aesthetic;
    let community_id = community.id;
    for lex in &mut community.lexicon {
        let next = apply_changes(&lex.word, std::slice::from_ref(rule));
        if next.phonemes().eq(lex.word.phonemes()) {
            continue;
        }
        let before = ortho::romanize(&lex.word, aesthetic);
        let after = ortho::romanize(&next, aesthetic);
        lex.traces.push(WordTrace {
            checkpoint,
            community: community_id,
            before: Some(before),
            after,
            explanation: rule_detail(&rule.id),
        });
        lex.word = next;
    }
    for formation in &mut community.formations {
        let next = apply_changes(&formation.exponent, std::slice::from_ref(rule));
        if next.phonemes().eq(formation.exponent.phonemes()) {
            continue;
        }
        affix_explanations.push((formation.kind, rule_detail(&rule.id)));
        formation.exponent = next;
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

/// Inventory stays independent of the sampled lexicon: unused proto phones
/// are kept, and only novel sound-change outputs are appended.
/// Limitation: phones are never dropped or resampled from current words,
/// and contact nativization maps into this set instead of importing donor phones.
fn absorb_novel_phones(community: &mut Community) {
    let mut seen: Vec<PhonemeId> = community
        .consonants
        .iter()
        .copied()
        .chain(community.vowels.iter().copied())
        .collect();
    for word in community
        .lexicon
        .iter()
        .map(|lex| &lex.word)
        .chain(community.formations.iter().map(|rule| &rule.exponent))
    {
        for phone in word.phonemes() {
            if seen.contains(&phone) {
                continue;
            }
            let Some(seg) = CATALOG.segments.get(phone.0 as usize) else {
                continue;
            };
            seen.push(phone);
            if seg.is_vowel() {
                community.vowels.push(phone);
            } else {
                community.consonants.push(phone);
            }
        }
    }
}

fn validate_checkpoint(cp: &HistoryCheckpoint) -> Result<(), String> {
    let mut seen_ids = Vec::new();
    let mut seen_names = Vec::new();
    for community in &cp.communities {
        if community.id == 0 || community.id > cp.id as u64 || community.founded_at > cp.id {
            return Err("community id or founding checkpoint is out of range".into());
        }
        if seen_ids.contains(&community.id) {
            return Err(format!("duplicate community id {}", community.id));
        }
        seen_ids.push(community.id);
        if community.name.trim().is_empty() {
            return Err(format!("community {} has an empty name", community.id));
        }
        if seen_names.iter().any(|n| n == &community.name) {
            return Err(format!("duplicate community name '{}'", community.name));
        }
        seen_names.push(community.name.clone());
        if community.aesthetic.id.trim().is_empty() {
            return Err(format!("community {} has an empty aesthetic", community.id));
        }
        validate_inventory(community)?;
        validate_lexicon(community)?;
        validate_formations(community)?;
        for lex in &community.lexicon {
            let origin = cp
                .communities
                .iter()
                .find(|c| c.id == lex.origin_community)
                .and_then(|c| c.lexicon.iter().find(|l| l.id == lex.origin_lexeme));
            if origin.is_none() {
                return Err(format!("lexeme '{}' has an unknown origin", lex.id));
            }
            let mut previous_checkpoint = 0;
            for trace in &lex.traces {
                if trace.checkpoint < previous_checkpoint
                    || trace.checkpoint > cp.id
                    || !cp
                        .communities
                        .iter()
                        .any(|c| c.id == trace.community && c.founded_at <= trace.checkpoint)
                {
                    return Err(format!("lexeme '{}' has invalid trace ancestry", lex.id));
                }
                previous_checkpoint = trace.checkpoint;
            }
            if lex.traces.last().map(|trace| &trace.after) != Some(&community.form(&lex.word)) {
                return Err(format!(
                    "lexeme '{}' does not match its recorded form",
                    lex.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_inventory(community: &Community) -> Result<(), String> {
    if community.consonants.is_empty() || community.vowels.is_empty() {
        return Err(format!(
            "community {} has an empty phonological inventory",
            community.id
        ));
    }
    let mut seen = Vec::new();
    for (id, listed_as_vowel) in community
        .consonants
        .iter()
        .map(|id| (id, false))
        .chain(community.vowels.iter().map(|id| (id, true)))
    {
        let Some(seg) = CATALOG.segments.get(id.0 as usize) else {
            return Err(format!(
                "community {} inventory contains an unknown phoneme",
                community.id
            ));
        };
        if seen.contains(id) {
            return Err(format!(
                "community {} inventory repeats a phoneme",
                community.id
            ));
        }
        seen.push(*id);
        if seg.is_vowel() != listed_as_vowel {
            return Err(format!(
                "community {} inventory classifies {} incorrectly",
                community.id,
                seg.ipa()
            ));
        }
    }
    Ok(())
}

fn validate_lexicon(community: &Community) -> Result<(), String> {
    if community.lexicon.is_empty() {
        return Err(format!("community {} has an empty lexicon", community.id));
    }
    let mut ids = Vec::new();
    for lex in &community.lexicon {
        if lex.id.trim().is_empty() || lex.origin_lexeme.trim().is_empty() {
            return Err(format!(
                "community {} has a malformed lexeme id",
                community.id
            ));
        }
        if ids.iter().any(|id| id == &lex.id) {
            return Err(format!(
                "community {} has a duplicate lexeme '{}'",
                community.id, lex.id
            ));
        }
        ids.push(lex.id.clone());
        if lex.gloss.trim().is_empty() {
            return Err(format!(
                "community {} lexeme '{}' has an empty gloss",
                community.id, lex.id
            ));
        }
        validate_word(&lex.word)?;
        if lex
            .word
            .phonemes()
            .any(|id| !community.consonants.contains(&id) && !community.vowels.contains(&id))
        {
            return Err(format!(
                "lexeme '{}' uses a phoneme outside its community inventory",
                lex.id
            ));
        }
        for trace in &lex.traces {
            if trace.after.trim().is_empty() {
                return Err(format!(
                    "community {} lexeme '{}' has an empty trace",
                    community.id, lex.id
                ));
            }
        }
    }
    validate_base_links(community)
}

fn validate_word(word: &Word) -> Result<(), String> {
    if word.syllables.is_empty() {
        return Err("word has no syllables".into());
    }
    if let Some(join_at) = word.join_at {
        if join_at >= word.syllables.len() {
            return Err("word join_at is out of range".into());
        }
    }
    for syl in &word.syllables {
        if syl.nucleus.is_empty() {
            return Err("word has a syllable without a vowel".into());
        }
        for (id, vowel) in syl
            .onset
            .iter()
            .chain(&syl.coda)
            .map(|id| (id, false))
            .chain(syl.nucleus.iter().map(|id| (id, true)))
        {
            match CATALOG.segments.get(id.0 as usize) {
                Some(segment) if segment.is_vowel() == vowel => {}
                _ => return Err("word contains an unknown or misplaced phoneme".into()),
            }
        }
    }
    Ok(())
}

fn validate_chronology(checkpoints: &[HistoryCheckpoint]) -> Result<(), String> {
    let first = &checkpoints[0];
    if first.event.is_some() || !first.communities.is_empty() {
        return Err("checkpoint 0 must be the empty beginning".into());
    }
    let mut prev: &[Community] = &first.communities;
    for cp in checkpoints.iter().skip(1) {
        let event = cp
            .event
            .as_ref()
            .ok_or_else(|| format!("checkpoint {} missing event", cp.id))?;
        match event {
            HistoryEvent::Found { name, aesthetic } => {
                if name.trim().is_empty() {
                    return Err("found event has an empty name".into());
                }
                if Aesthetic::by_id(aesthetic).is_none() {
                    return Err(format!("unknown aesthetic '{aesthetic}'"));
                }
                expect_added(prev, &cp.communities, cp.id, None, name)?;
            }
            HistoryEvent::Separate { parent, name } => {
                if name.trim().is_empty() {
                    return Err("separate event has an empty name".into());
                }
                if !prev.iter().any(|c| c.id == *parent) {
                    return Err(format!(
                        "checkpoint {} separates from unknown parent {parent}",
                        cp.id
                    ));
                }
                expect_added(prev, &cp.communities, cp.id, Some(*parent), name)?;
            }
            HistoryEvent::Develop { community, steps } => {
                if !(1..=5).contains(steps) {
                    return Err(format!("checkpoint {} has invalid develop steps", cp.id));
                }
                expect_same_ids(prev, &cp.communities, cp.id)?;
                if !prev.iter().any(|c| c.id == *community) {
                    return Err(format!(
                        "checkpoint {} develops unknown community {community}",
                        cp.id
                    ));
                }
                expect_frozen_except(prev, &cp.communities, &[*community])?;
            }
            HistoryEvent::Contact {
                donor,
                recipient,
                count,
                ..
            } => {
                if donor == recipient {
                    return Err(format!(
                        "checkpoint {} contacts a community with itself",
                        cp.id
                    ));
                }
                if !(1..=36).contains(count) {
                    return Err(format!("checkpoint {} has invalid contact count", cp.id));
                }
                expect_same_ids(prev, &cp.communities, cp.id)?;
                if !prev.iter().any(|c| c.id == *donor) {
                    return Err(format!("checkpoint {} has unknown donor {donor}", cp.id));
                }
                if !prev.iter().any(|c| c.id == *recipient) {
                    return Err(format!(
                        "checkpoint {} has unknown recipient {recipient}",
                        cp.id
                    ));
                }
                expect_frozen_except(prev, &cp.communities, &[*recipient])?;
            }
            HistoryEvent::Derive {
                community,
                base,
                kind,
            } => {
                if base.trim().is_empty() {
                    return Err(format!("checkpoint {} derives from an empty base", cp.id));
                }
                expect_same_ids(prev, &cp.communities, cp.id)?;
                let Some(prev_c) = prev.iter().find(|c| c.id == *community) else {
                    return Err(format!(
                        "checkpoint {} derives in unknown community {community}",
                        cp.id
                    ));
                };
                if !prev_c.lexicon.iter().any(|lex| lex.id == *base) {
                    return Err(format!(
                        "checkpoint {} derives from unknown base '{base}'",
                        cp.id
                    ));
                }
                expect_frozen_except(prev, &cp.communities, &[*community])?;
                let Some(next_c) = cp.communities.iter().find(|c| c.id == *community) else {
                    return Err(format!(
                        "checkpoint {} is missing community {community}",
                        cp.id
                    ));
                };
                if next_c.lexicon.len() != prev_c.lexicon.len() + 1 {
                    return Err(format!(
                        "checkpoint {} did not add exactly one derived lexeme",
                        cp.id
                    ));
                }
                if next_c.formations != prev_c.formations {
                    return Err(format!(
                        "checkpoint {} changed formation exponents during derive",
                        cp.id
                    ));
                }
                if !next_c.lexicon.iter().any(|lex| {
                    lex.base_lexeme.as_deref() == Some(base.as_str())
                        && lex.formation == Some(*kind)
                }) {
                    return Err(format!(
                        "checkpoint {} is missing the derived {} form",
                        cp.id,
                        kind.label()
                    ));
                }
            }
        }
        for community in &cp.communities {
            if let Some(parent) = community.parent {
                if !cp.communities.iter().any(|c| c.id == parent) {
                    return Err(format!(
                        "community {} parent {parent} is missing",
                        community.id
                    ));
                }
            }
            for lex in &community.lexicon {
                if !origin_known(checkpoints, cp.id, lex.origin_community) {
                    return Err(format!(
                        "lexeme '{}' origin community {} is missing",
                        lex.id, lex.origin_community
                    ));
                }
            }
        }
        prev = &cp.communities;
    }
    Ok(())
}

fn origin_known(checkpoints: &[HistoryCheckpoint], through: usize, origin: u64) -> bool {
    checkpoints
        .iter()
        .take(through + 1)
        .any(|cp| cp.communities.iter().any(|c| c.id == origin))
}

fn expect_same_ids(
    prev: &[Community],
    next: &[Community],
    checkpoint: usize,
) -> Result<(), String> {
    if prev.len() != next.len() {
        return Err(format!("checkpoint {checkpoint} changed the community set"));
    }
    for community in prev {
        if !next.iter().any(|c| c.id == community.id) {
            return Err(format!(
                "checkpoint {checkpoint} dropped community {}",
                community.id
            ));
        }
    }
    Ok(())
}

fn expect_added(
    prev: &[Community],
    next: &[Community],
    checkpoint: usize,
    parent: Option<u64>,
    name: &str,
) -> Result<(), String> {
    if next.len() != prev.len() + 1 {
        return Err(format!(
            "checkpoint {checkpoint} did not add exactly one community"
        ));
    }
    expect_frozen_except(prev, next, &[])?;
    let added = next
        .iter()
        .find(|c| !prev.iter().any(|p| p.id == c.id))
        .ok_or_else(|| format!("checkpoint {checkpoint} missing new community"))?;
    if added.name != name.trim() {
        return Err(format!(
            "checkpoint {checkpoint} community name does not match the event"
        ));
    }
    if added.parent != parent {
        return Err(format!(
            "checkpoint {checkpoint} community parent does not match the event"
        ));
    }
    if added.founded_at != checkpoint {
        return Err(format!(
            "checkpoint {checkpoint} community founded_at is wrong"
        ));
    }
    Ok(())
}

fn expect_frozen_except(
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
        if !frozen_same(earlier, later) {
            return Err(format!(
                "community {} changed outside the event",
                earlier.id
            ));
        }
    }
    Ok(())
}

fn frozen_same(a: &Community, b: &Community) -> bool {
    a.id == b.id
        && a.name == b.name
        && a.parent == b.parent
        && a.founded_at == b.founded_at
        && a.aesthetic.id == b.aesthetic.id
        && a.consonants == b.consonants
        && a.vowels == b.vowels
        && a.lexicon.len() == b.lexicon.len()
        && a.formations == b.formations
        && a.lexicon.iter().zip(&b.lexicon).all(|(x, y)| {
            x.id == y.id
                && x.gloss == y.gloss
                && x.origin_community == y.origin_community
                && x.origin_lexeme == y.origin_lexeme
                && x.base_lexeme == y.base_lexeme
                && x.formation == y.formation
                && x.word == y.word
                && x.traces == y.traces
        })
}

fn formation_gloss(kind: FormationKind, base_gloss: &str) -> String {
    match kind {
        FormationKind::Agent => format!("person associated with {base_gloss}"),
        FormationKind::Place => format!("place of {base_gloss}"),
        FormationKind::Collective => format!("collection of {base_gloss}"),
    }
}

fn attach_exponent(base: &Word, exponent: &Word) -> Word {
    let mut syllables = base.syllables.clone();
    syllables.extend(exponent.syllables.iter().cloned());
    Word {
        syllables,
        join_at: base.join_at,
    }
}

fn existing_derivation(community: &Community, base: &str, kind: FormationKind) -> Option<String> {
    community
        .lexicon
        .iter()
        .find(|lex| lex.base_lexeme.as_deref() == Some(base) && lex.formation == Some(kind))
        .map(|lex| lex.id.clone())
}

fn mint_formation_rules(inventory: &Inventory, rng: &mut impl Rng) -> Vec<FormationRule> {
    let exponents = mint_exponents(&inventory.consonants, &inventory.vowels, rng);
    FormationKind::all()
        .into_iter()
        .zip(exponents)
        .map(|(kind, exponent)| FormationRule { kind, exponent })
        .collect()
}

fn mint_exponents(consonants: &[PhonemeId], vowels: &[PhonemeId], rng: &mut impl Rng) -> Vec<Word> {
    // Sample shape indices first; allocate only the three chosen words.
    let cv = consonants.len() * vowels.len();
    let total = if cv >= 3 {
        cv
    } else if cv > 0 {
        cv + cv * consonants.len()
    } else {
        vowels.len()
    };
    let mut pool: Vec<usize> = (0..total).collect();
    let mut chosen = Vec::with_capacity(3);
    for _ in 0..3 {
        if pool.is_empty() {
            if let Some(last) = chosen.last().cloned() {
                chosen.push(last);
            }
            continue;
        }
        let rank = rng.gen_range(0..pool.len() as u32) as usize;
        let index = pool.swap_remove(rank);
        let word = if consonants.is_empty() {
            affix_word(&[], vowels[index], &[])
        } else if index < cv {
            affix_word(
                &[consonants[index / vowels.len()]],
                vowels[index % vowels.len()],
                &[],
            )
        } else {
            let index = index - cv;
            let coda = consonants[index % consonants.len()];
            let stem = index / consonants.len();
            affix_word(
                &[consonants[stem / vowels.len()]],
                vowels[stem % vowels.len()],
                &[coda],
            )
        };
        chosen.push(word);
    }
    chosen
}

fn affix_word(onset: &[PhonemeId], nucleus: PhonemeId, coda: &[PhonemeId]) -> Word {
    Word {
        syllables: vec![Syllable {
            onset: onset.to_vec(),
            nucleus: vec![nucleus],
            coda: coda.to_vec(),
            long: false,
        }],
        join_at: None,
    }
}

fn validate_formations(community: &Community) -> Result<(), String> {
    let mut kinds = Vec::new();
    for rule in &community.formations {
        if kinds.contains(&rule.kind) {
            return Err(format!(
                "community {} repeats a {} formation",
                community.id,
                rule.kind.label()
            ));
        }
        kinds.push(rule.kind);
        validate_word(&rule.exponent)?;
        if rule
            .exponent
            .phonemes()
            .any(|id| !community.consonants.contains(&id) && !community.vowels.contains(&id))
        {
            return Err(format!(
                "community {} {} suffix uses a phoneme outside its inventory",
                community.id,
                rule.kind.label()
            ));
        }
    }
    Ok(())
}

fn validate_base_links(community: &Community) -> Result<(), String> {
    let mut seen_pairs: Vec<(String, FormationKind)> = Vec::new();
    for lex in &community.lexicon {
        if lex.formation.is_some() && lex.base_lexeme.is_none() {
            return Err(format!(
                "lexeme '{}' has a formation without a base",
                lex.id
            ));
        }
        if let Some(base) = &lex.base_lexeme {
            if !community.lexicon.iter().any(|other| other.id == *base) {
                return Err(format!(
                    "lexeme '{}' points at unknown base '{base}'",
                    lex.id
                ));
            }
            if let Some(kind) = lex.formation {
                if seen_pairs.iter().any(|(b, k)| b == base && *k == kind) {
                    return Err(format!(
                        "community {} has two {} formations from '{base}'",
                        community.id,
                        kind.label()
                    ));
                }
                seen_pairs.push((base.clone(), kind));
            }
        }
        let mut walk = Vec::new();
        let mut cur = Some(lex.id.as_str());
        while let Some(id) = cur {
            if walk.iter().any(|seen| *seen == id) {
                return Err(format!("lexeme '{}' has a cyclic base link", lex.id));
            }
            walk.push(id);
            cur = community
                .lexicon
                .iter()
                .find(|other| other.id == id)
                .and_then(|other| other.base_lexeme.as_deref());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn development_can_feed_a_previously_inapplicable_rule() {
        let mut history = History::new(42);
        history
            .commit(HistoryEvent::Found {
                name: "A".into(),
                aesthetic: "elvish".into(),
            })
            .unwrap();
        let community = &mut history.checkpoints.last_mut().unwrap().communities[0];
        let t = CATALOG.id_by_ipa("t").unwrap();
        let i = CATALOG.id_by_ipa("i").unwrap();
        community.consonants = ["t", "s", "h"]
            .iter()
            .map(|p| CATALOG.id_by_ipa(p).unwrap())
            .collect();
        community.vowels = vec![i];
        community.lexicon.truncate(1);
        community.lexicon[0].base_lexeme = None;
        community.lexicon[0].formation = None;
        community.lexicon[0].word = Word {
            syllables: vec![crate::Syllable {
                onset: vec![t],
                nucleus: vec![i],
                coda: vec![],
                long: false,
            }],
            join_at: None,
        };
        community.formations.clear();
        let id = community.id;
        history
            .commit(HistoryEvent::Develop {
                community: id,
                steps: 2,
            })
            .unwrap();
        let word = &history.checkpoints.last().unwrap().communities[0].lexicon[0].word;
        assert_eq!(CATALOG.ipa_string(&word.phones()), "hi");
    }

    fn phones(community: &Community) -> Vec<(String, Vec<PhonemeId>)> {
        community
            .lexicon
            .iter()
            .map(|lex| (lex.id.clone(), lex.word.phones()))
            .collect()
    }

    fn named<'a>(history: &'a History, name: &str) -> &'a Community {
        history
            .checkpoints()
            .last()
            .unwrap()
            .communities
            .iter()
            .find(|c| c.name == name)
            .unwrap()
    }

    fn named_at<'a>(history: &'a History, checkpoint: usize, name: &str) -> &'a Community {
        history.checkpoints()[checkpoint]
            .communities
            .iter()
            .find(|c| c.name == name)
            .unwrap()
    }

    fn checkpoint_fingerprint(cp: &HistoryCheckpoint) -> Vec<(u64, String, Vec<PhonemeId>)> {
        cp.communities
            .iter()
            .flat_map(|c| {
                c.lexicon
                    .iter()
                    .map(|lex| (c.id, lex.id.clone(), lex.word.phones()))
            })
            .collect()
    }

    #[test]
    fn preview_is_nonmutating_and_matches_commit() {
        let mut history = History::new(42);
        let found = HistoryEvent::Found {
            name: "A".into(),
            aesthetic: "elvish".into(),
        };
        let preview_found = history.preview(found.clone()).unwrap();
        assert_eq!(history.checkpoints().len(), 1);
        history.commit(found).unwrap();
        assert_eq!(history.checkpoints().len(), 2);
        assert_eq!(
            checkpoint_fingerprint(&preview_found),
            checkpoint_fingerprint(history.checkpoints().last().unwrap())
        );

        let develop = HistoryEvent::Develop {
            community: named(&history, "A").id,
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
        history
            .commit(HistoryEvent::Found {
                name: "Parent".into(),
                aesthetic: "elvish".into(),
            })
            .unwrap();
        let parent_id = named(&history, "Parent").id;
        history
            .commit(HistoryEvent::Separate {
                parent: parent_id,
                name: "Child".into(),
            })
            .unwrap();
        let split_parent = phones(named(&history, "Parent"));
        let split_child = phones(named(&history, "Child"));
        assert_eq!(
            split_parent.iter().map(|(_, p)| p).collect::<Vec<_>>(),
            split_child.iter().map(|(_, p)| p).collect::<Vec<_>>()
        );

        let child_id = named(&history, "Child").id;
        history
            .commit(HistoryEvent::Develop {
                community: parent_id,
                steps: 2,
            })
            .unwrap();
        assert_eq!(phones(named(&history, "Child")), split_child);
        let after_parent = phones(named(&history, "Parent"));

        history
            .commit(HistoryEvent::Develop {
                community: child_id,
                steps: 2,
            })
            .unwrap();
        assert_eq!(phones(named(&history, "Parent")), after_parent);
        assert_ne!(phones(named(&history, "Child")), split_child);
    }

    #[test]
    fn loan_provenance_evolves_and_freezes_prior_snapshot() {
        let mut history = History::new(42);
        history
            .commit(HistoryEvent::Found {
                name: "Donor".into(),
                aesthetic: "elvish".into(),
            })
            .unwrap();
        history
            .commit(HistoryEvent::Found {
                name: "Recipient".into(),
                aesthetic: "kuo-toa".into(),
            })
            .unwrap();
        let donor_id = named(&history, "Donor").id;
        let recipient_id = named(&history, "Recipient").id;
        history
            .commit(HistoryEvent::Contact {
                donor: donor_id,
                recipient: recipient_id,
                domain: ContactDomain::General,
                count: 8,
            })
            .unwrap();
        let contact_idx = history.checkpoints().len() - 1;
        let recipient = named(&history, "Recipient");
        let loans: Vec<&HistoryLexeme> = recipient
            .lexicon
            .iter()
            .filter(|lex| lex.origin_community == donor_id)
            .collect();
        assert_eq!(loans.len(), 8);
        let donor = named(&history, "Donor");
        for loan in &loans {
            assert!(donor.lexicon.iter().any(|d| d.id == loan.origin_lexeme));
            assert!(loan.base_lexeme.is_none());
            assert!(loan.formation.is_none());
            assert!(loan
                .traces
                .iter()
                .any(|trace| trace.checkpoint == contact_idx && trace.community == recipient_id));
        }
        let frozen: Vec<(String, Vec<PhonemeId>)> = loans
            .iter()
            .map(|lex| (lex.id.clone(), lex.word.phones()))
            .collect();

        history
            .commit(HistoryEvent::Develop {
                community: recipient_id,
                steps: 5,
            })
            .unwrap();

        let prior = named_at(&history, contact_idx, "Recipient");
        for (id, old_phones) in &frozen {
            let lex = prior.lexicon.iter().find(|l| l.id == *id).unwrap();
            assert_eq!(lex.word.phones(), *old_phones);
        }
        let now = named(&history, "Recipient");
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
    }

    #[test]
    fn invalid_events_leave_history_unchanged() {
        let mut history = History::new(3);
        history
            .commit(HistoryEvent::Found {
                name: "A".into(),
                aesthetic: "elvish".into(),
            })
            .unwrap();
        let before_len = history.checkpoints().len();
        let before = checkpoint_fingerprint(history.checkpoints().last().unwrap());

        assert!(
            history
                .commit(HistoryEvent::Found {
                    name: "  ".into(),
                    aesthetic: "elvish".into(),
                })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Found {
                    name: "B".into(),
                    aesthetic: "no-such-pack".into(),
                })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Separate {
                    parent: 99,
                    name: "X".into(),
                })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Develop {
                    community: named(&history, "A").id,
                    steps: 0,
                })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Develop {
                    community: named(&history, "A").id,
                    steps: 6,
                })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Contact {
                    donor: named(&history, "A").id,
                    recipient: named(&history, "A").id,
                    domain: ContactDomain::General,
                    count: 3,
                })
                .is_err()
        );
        assert!(
            history
                .commit(HistoryEvent::Contact {
                    donor: named(&history, "A").id,
                    recipient: 99,
                    domain: ContactDomain::Maritime,
                    count: 2,
                })
                .is_err()
        );

        assert_eq!(history.checkpoints().len(), before_len);
        assert_eq!(
            checkpoint_fingerprint(history.checkpoints().last().unwrap()),
            before
        );
    }

    fn gloss_id(community: &Community, gloss: &str) -> String {
        community
            .lexicon
            .iter()
            .find(|lex| lex.gloss == gloss)
            .unwrap()
            .id
            .clone()
    }

    #[test]
    fn same_suffix_across_roots_uses_founding_links() {
        let mut history = History::new(42);
        history
            .commit(HistoryEvent::Found {
                name: "A".into(),
                aesthetic: "elvish".into(),
            })
            .unwrap();
        let checkpoint = history.checkpoints().len() - 1;
        let community = named(&history, "A");
        let id = community.id;
        let person = gloss_id(community, "person");
        let people = community
            .lexicon
            .iter()
            .find(|lex| lex.gloss == "people")
            .unwrap();
        assert_eq!(people.base_lexeme.as_deref(), Some(person.as_str()));
        assert!(people.formation.is_none());
        let water = community
            .lexicon
            .iter()
            .find(|lex| lex.gloss == "water")
            .unwrap();
        let fire = community
            .lexicon
            .iter()
            .find(|lex| lex.gloss == "fire")
            .unwrap();
        let water_opts = history.formations(checkpoint, id, &water.id).unwrap();
        let fire_opts = history.formations(checkpoint, id, &fire.id).unwrap();
        assert_eq!(water_opts.len(), 3);
        assert_eq!(fire_opts.len(), 3);
        for (a, b) in water_opts.iter().zip(&fire_opts) {
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.exponent, b.exponent);
        }
    }

    #[test]
    fn derive_preview_duplicate_and_absent_from_prior() {
        let mut history = History::new(42);
        history
            .commit(HistoryEvent::Found {
                name: "A".into(),
                aesthetic: "elvish".into(),
            })
            .unwrap();
        let found_idx = history.checkpoints().len() - 1;
        let community = named(&history, "A");
        let id = community.id;
        let water_id = gloss_id(community, "water");
        let event = HistoryEvent::Derive {
            community: id,
            base: water_id.clone(),
            kind: FormationKind::Agent,
        };
        let preview = history.preview(event.clone()).unwrap();
        assert_eq!(history.checkpoints().len(), found_idx + 1);
        history.commit(event.clone()).unwrap();
        assert_eq!(
            checkpoint_fingerprint(&preview),
            checkpoint_fingerprint(history.checkpoints().last().unwrap())
        );
        let derived = named(&history, "A")
            .lexicon
            .iter()
            .find(|lex| {
                lex.base_lexeme.as_deref() == Some(water_id.as_str())
                    && lex.formation == Some(FormationKind::Agent)
            })
            .unwrap();
        let derived_id = derived.id.clone();
        assert!(
            named_at(&history, found_idx, "A")
                .lexicon
                .iter()
                .all(|lex| lex.id != derived_id)
        );
        assert!(history.preview(event.clone()).is_err());
        assert!(history.commit(event).is_err());
        let agent = history
            .formations(history.checkpoints().len() - 1, id, &water_id)
            .unwrap()
            .into_iter()
            .find(|opt| opt.kind == FormationKind::Agent)
            .unwrap();
        assert_eq!(agent.existing.as_deref(), Some(derived_id.as_str()));
    }

    #[test]
    fn derivation_and_suffix_evolve_without_altering_past() {
        let mut history = History::new(42);
        history
            .commit(HistoryEvent::Found {
                name: "A".into(),
                aesthetic: "elvish".into(),
            })
            .unwrap();
        let phone = |ipa| CATALOG.id_by_ipa(ipa).unwrap();
        let community = &mut history.checkpoints[1].communities[0];
        community.consonants = vec![phone("t"), phone("s"), phone("h")];
        community.vowels = vec![phone("a"), phone("i")];
        community.lexicon.retain(|lex| lex.gloss == "water");
        community.lexicon[0].word = affix_word(&[phone("t")], phone("a"), &[]);
        community.lexicon[0].traces[0].after = "ta".into();
        community.formations = vec![FormationRule {
            kind: FormationKind::Agent,
            exponent: affix_word(&[phone("s")], phone("i"), &[]),
        }];
        let id = community.id;
        let base = community.lexicon[0].id.clone();
        history
            .commit(HistoryEvent::Derive {
                community: id,
                base: base.clone(),
                kind: FormationKind::Agent,
            })
            .unwrap();
        history
            .commit(HistoryEvent::Develop {
                community: id,
                steps: 5,
            })
            .unwrap();

        let prior = named_at(&history, 2, "A");
        assert_eq!(CATALOG.ipa_string(&prior.lexicon[1].word.phones()), "tasi");
        assert_eq!(
            CATALOG.ipa_string(&prior.formations[0].exponent.phones()),
            "si"
        );
        let now = named(&history, "A");
        assert_eq!(CATALOG.ipa_string(&now.lexicon[1].word.phones()), "tas");
        assert_eq!(
            CATALOG.ipa_string(&now.formations[0].exponent.phones()),
            "hi"
        );
        let attested = history.formations(3, id, &base).unwrap().remove(0);
        assert_eq!(
            attested.existing.as_deref(),
            Some(now.lexicon[1].id.as_str())
        );
        assert_eq!(
            CATALOG.ipa_string(&attested.word.phones()),
            "tas",
            "an existing formation is its historical form, not a newly recomposed tahi"
        );
    }

    #[test]
    fn separate_remaps_formation_links() {
        let mut history = History::new(7);
        history
            .commit(HistoryEvent::Found {
                name: "Parent".into(),
                aesthetic: "elvish".into(),
            })
            .unwrap();
        let parent_id = named(&history, "Parent").id;
        let water_id = gloss_id(named(&history, "Parent"), "water");
        history
            .commit(HistoryEvent::Derive {
                community: parent_id,
                base: water_id.clone(),
                kind: FormationKind::Place,
            })
            .unwrap();
        let parent = named(&history, "Parent");
        let parent_derived = parent
            .lexicon
            .iter()
            .find(|lex| lex.formation == Some(FormationKind::Place))
            .unwrap();
        let parent_derived_id = parent_derived.id.clone();
        let parent_base = parent_derived.base_lexeme.clone().unwrap();
        let parent_suffixes = parent.formations.clone();
        history
            .commit(HistoryEvent::Separate {
                parent: parent_id,
                name: "Child".into(),
            })
            .unwrap();
        let child = named(&history, "Child");
        let child_derived = child
            .lexicon
            .iter()
            .find(|lex| lex.formation == Some(FormationKind::Place))
            .unwrap();
        assert_eq!(child_derived.origin_community, parent_id);
        assert_eq!(child_derived.origin_lexeme, parent_derived_id);
        let child_base = child_derived.base_lexeme.clone().unwrap();
        assert_ne!(child_base, parent_base);
        assert_eq!(
            child
                .lexicon
                .iter()
                .find(|lex| lex.id == child_base)
                .unwrap()
                .gloss,
            "water"
        );
        assert_eq!(child.formations, parent_suffixes);
        let parent_now = named(&history, "Parent");
        assert_eq!(
            parent_now
                .lexicon
                .iter()
                .find(|lex| lex.id == parent_derived_id)
                .unwrap()
                .base_lexeme
                .as_deref(),
            Some(parent_base.as_str())
        );
    }
}

use std::collections::{HashMap, HashSet};

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

use crate::Aesthetic;
use crate::Language;
use crate::change::{
    SoundChange, apply_changes_tracked, map_offset, realize_phones, realize_phones_long,
    word_length_flags,
};
use crate::change_packs::rule_detail;
use crate::generate::Word;
use crate::lexicon::{mint_roots, mint_stem};
use crate::ortho;
use crate::phoneme::{CATALOG, PhonemeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Agent,
    Experiencer,
    Patient,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticFrame {
    Entity { countable: bool },
    Event { roles: Vec<Role> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Meaning {
    Atom(String),
    Participant { role: Role, event: Box<Meaning> },
    PlaceOf { event: Box<Meaning> },
    Collective { member: Box<Meaning> },
}

impl Meaning {
    pub fn label(&self) -> String {
        match self {
            Meaning::Atom(name) => name.clone(),
            Meaning::Participant { role, event } => {
                format!("{} of {}", role_label(*role), event.label())
            }
            Meaning::PlaceOf { event } => format!("place of {}", event.label()),
            Meaning::Collective { member } => format!("collection of {}", member.label()),
        }
    }
}

fn role_label(role: Role) -> &'static str {
    match role {
        Role::Agent => "agent",
        Role::Experiencer => "experiencer",
        Role::Patient => "patient",
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sense {
    pub id: u32,
    pub meaning: Meaning,
    pub frame: SemanticFrame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassKind {
    Entity,
    Event,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexicalClass {
    pub id: String,
    pub label: String,
    pub kind: ClassKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticOperation {
    Participant(Role),
    PlaceOf,
    Collective,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum AffixPosition {
    Prefix,
    Suffix,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct BoundAllomorph {
    phones: Vec<PhonemeId>,
    adjacent: Option<PhonemeId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Construction {
    pub id: String,
    pub label: String,
    pub input_class: String,
    pub output_class: String,
    pub operation: SemanticOperation,
    pub output_countable: bool,
    position: AffixPosition,
    allomorphs: Vec<BoundAllomorph>,
}

impl Construction {
    pub fn exponents(&self) -> Vec<Word> {
        self.allomorphs
            .iter()
            .map(|allo| realize_phones(&allo.phones))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StressPattern {
    Initial,
    Penultimate,
    Final,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grammar {
    pub classes: Vec<LexicalClass>,
    pub constructions: Vec<Construction>,
    pub stress: StressPattern,
    host_vowel: PhonemeId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexemeRef {
    pub variety: u64,
    pub checkpoint: usize,
    pub lexeme: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    Unrecorded {
        variety: u64,
        checkpoint: usize,
    },
    Formed {
        source: LexemeRef,
        sense: u32,
        construction: String,
        checkpoint: usize,
    },
    Borrowed {
        source: LexemeRef,
        checkpoint: usize,
    },
    Inherited {
        source: LexemeRef,
        checkpoint: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Analysis {
    pub base: String,
    pub sense: u32,
    pub construction: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordTrace {
    pub checkpoint: usize,
    pub variety: u64,
    pub before: Option<String>,
    pub after: String,
    pub explanation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lexeme {
    pub id: String,
    pub class: String,
    pub senses: Vec<Sense>,
    pub word: Word,
    pub stress: usize,
    pub boundaries: Vec<usize>,
    pub analysis: Option<Analysis>,
    pub origin: Origin,
    pub traces: Vec<WordTrace>,
    pub retired: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FormationOption {
    pub construction: String,
    pub label: String,
    pub gloss: String,
    pub word: Word,
    pub stress: usize,
    pub base: String,
    pub sense: u32,
    pub exponent: Word,
    pub existing: Option<String>,
}

#[derive(Clone, Copy)]
enum ConceptFrame {
    Count,
    Mass,
    Event(&'static [Role]),
}

const AGENT_PATIENT: &[Role] = &[Role::Agent, Role::Patient];
const AGENT: &[Role] = &[Role::Agent];
const PATIENT: &[Role] = &[Role::Patient];
const EXPERIENCER_PATIENT: &[Role] = &[Role::Experiencer, Role::Patient];

const CONCEPTS: &[(&'static str, ConceptFrame)] = &[
    ("person", ConceptFrame::Count),
    ("people", ConceptFrame::Count),
    ("water", ConceptFrame::Mass),
    ("stone", ConceptFrame::Count),
    ("tree", ConceptFrame::Count),
    ("hill", ConceptFrame::Count),
    ("star", ConceptFrame::Count),
    ("light", ConceptFrame::Mass),
    ("dark", ConceptFrame::Mass),
    ("fire", ConceptFrame::Mass),
    ("sky", ConceptFrame::Count),
    ("earth", ConceptFrame::Mass),
    ("river", ConceptFrame::Count),
    ("sea", ConceptFrame::Mass),
    ("house", ConceptFrame::Count),
    ("path", ConceptFrame::Count),
    ("name", ConceptFrame::Count),
    ("child", ConceptFrame::Count),
    ("sun", ConceptFrame::Count),
    ("moon", ConceptFrame::Count),
    ("wind", ConceptFrame::Mass),
    ("hand", ConceptFrame::Count),
    ("eye", ConceptFrame::Count),
    ("speak", ConceptFrame::Event(AGENT_PATIENT)),
    ("go", ConceptFrame::Event(AGENT)),
    ("see", ConceptFrame::Event(EXPERIENCER_PATIENT)),
    ("life", ConceptFrame::Mass),
    ("death", ConceptFrame::Mass),
    ("food", ConceptFrame::Mass),
    ("war", ConceptFrame::Mass),
    ("king", ConceptFrame::Count),
    ("night", ConceptFrame::Count),
    ("day", ConceptFrame::Count),
    ("blood", ConceptFrame::Mass),
    ("heart", ConceptFrame::Count),
    ("voice", ConceptFrame::Count),
    ("fishing", ConceptFrame::Event(AGENT_PATIENT)),
    ("dying", ConceptFrame::Event(PATIENT)),
    ("wood", ConceptFrame::Mass),
];

fn colex_pairs(seed: u64) -> Vec<(&'static str, &'static str)> {
    let mut pairs = Vec::new();
    match seed % 5 {
        1 | 4 => pairs.push(("tree", "wood")),
        2 => pairs.push(("water", "sea")),
        3 => pairs.push(("fire", "light")),
        _ => {}
    }
    if seed % 7 == 3 {
        pairs.push(("water", "river"));
    }
    if seed % 11 == 2 {
        pairs.push(("day", "sun"));
    }
    if seed % 13 == 1 {
        pairs.push(("night", "dark"));
    }
    pairs
}

pub fn seed_language(
    language: &Language,
    variety: u64,
    checkpoint: usize,
) -> (Grammar, Vec<Lexeme>) {
    let mut rng = StdRng::seed_from_u64(language.seed.wrapping_add(101));
    let (entity, event) = class_pair(variety, language.seed);
    let stress = match language.seed % 3 {
        0 => StressPattern::Initial,
        1 => StressPattern::Penultimate,
        _ => StressPattern::Final,
    };
    let host_vowel = language
        .inventory
        .vowels
        .first()
        .copied()
        .or_else(|| CATALOG.id_by_ipa("a"))
        .unwrap_or(PhonemeId(0));
    let constructions = seed_constructions(language, variety, &entity.id, &event.id, &mut rng);
    let grammar = Grammar {
        classes: vec![entity.clone(), event.clone()],
        constructions,
        stress,
        host_vowel,
    };

    let mut stems: HashMap<&str, Word> = mint_roots(language)
        .into_iter()
        .map(|root| (root.id, root.proto))
        .collect();
    let pairs = colex_pairs(language.seed);
    let extras: HashSet<&str> = pairs.iter().map(|(_, extra)| *extra).collect();
    let mut lexicon = Vec::new();
    for &(gloss, frame) in CONCEPTS {
        if extras.contains(gloss) {
            continue;
        }
        let mut senses = vec![concept_sense(0, gloss, frame)];
        let mut sid = 1u32;
        for &(host, extra) in &pairs {
            if host != gloss {
                continue;
            }
            let extra_frame = CONCEPTS
                .iter()
                .find(|(g, _)| *g == extra)
                .map(|(_, f)| *f)
                .unwrap_or(ConceptFrame::Mass);
            senses.push(concept_sense(sid, extra, extra_frame));
            sid += 1;
        }
        let class = match frame {
            ConceptFrame::Event(_) => event.id.as_str(),
            _ => entity.id.as_str(),
        };
        let word = stems
            .remove(gloss)
            .unwrap_or_else(|| mint_stem(language, &mut rng));
        push_root(
            &mut lexicon,
            variety,
            checkpoint,
            class,
            stress,
            gloss,
            word,
            senses,
        );
    }

    let licensed: Vec<(String, u32, String, String)> = grammar
        .constructions
        .iter()
        .filter_map(|c| match c.operation {
            SemanticOperation::Participant(Role::Agent) => Some((
                format!("v{variety}:fishing"),
                0,
                c.id.clone(),
                format!("{}-fishing", c.id),
            )),
            SemanticOperation::PlaceOf => Some((
                format!("v{variety}:fishing"),
                0,
                c.id.clone(),
                format!("{}-place", c.id),
            )),
            SemanticOperation::Collective => Some((
                format!("v{variety}:person"),
                0,
                c.id.clone(),
                format!("{}-people", c.id),
            )),
            _ => None,
        })
        .collect();
    for (base, sense, construction, id) in licensed {
        let source = LexemeRef {
            variety,
            checkpoint,
            lexeme: base,
        };
        if let Ok(formed) = derive_lexeme(
            &grammar,
            &lexicon,
            source,
            sense,
            &construction,
            id,
            checkpoint,
        ) {
            lexicon.push(formed);
        }
    }

    (grammar, lexicon)
}

pub fn formation_options(
    grammar: &Grammar,
    lexicon: &[Lexeme],
    base: &str,
    sense: u32,
) -> Result<Vec<FormationOption>, String> {
    let lex = require_active(lexicon, base)?;
    let sense_rec = require_sense(lex, sense)?;
    let mut options = Vec::new();
    for construction in &grammar.constructions {
        if let Some(existing) = existing_formation(lexicon, base, sense, &construction.id) {
            let exponent = exponent_from_word(existing, construction);
            options.push(FormationOption {
                construction: construction.id.clone(),
                label: construction.label.clone(),
                gloss: existing
                    .senses
                    .first()
                    .map(|s| s.meaning.label())
                    .unwrap_or_default(),
                word: existing.word.clone(),
                stress: existing.stress,
                base: base.to_string(),
                sense,
                exponent,
                existing: Some(existing.id.clone()),
            });
            continue;
        }
        if !eligible(construction, lex, sense_rec) {
            continue;
        }
        let (word, stress, _bounds, exponent) = realize(lex, construction, grammar.stress);
        let gloss = derived_meaning(&sense_rec.meaning, construction.operation).label();
        options.push(FormationOption {
            construction: construction.id.clone(),
            label: construction.label.clone(),
            gloss,
            word,
            stress,
            base: base.to_string(),
            sense,
            exponent,
            existing: None,
        });
    }
    Ok(options)
}

pub fn derive_lexeme(
    grammar: &Grammar,
    lexicon: &[Lexeme],
    source: LexemeRef,
    sense: u32,
    construction: &str,
    id: String,
    checkpoint: usize,
) -> Result<Lexeme, String> {
    if id.trim().is_empty() {
        return Err("lexeme id must be nonempty".into());
    }
    if lexicon.iter().any(|l| l.id == id) {
        return Err(format!("lexeme id '{id}' already exists"));
    }
    let base = require_active(lexicon, &source.lexeme)?;
    let sense_rec = require_sense(base, sense)?;
    let construction = require_construction(grammar, construction)?;
    if !eligible(construction, base, sense_rec) {
        return Err(format!(
            "construction '{}' is not licensed for this sense",
            construction.id
        ));
    }
    if existing_formation(lexicon, &source.lexeme, sense, &construction.id).is_some() {
        return Err(format!(
            "an active formation already uses construction '{}'",
            construction.id
        ));
    }
    let meaning = derived_meaning(&sense_rec.meaning, construction.operation);
    let (word, stress, boundaries, _) = realize(base, construction, grammar.stress);
    let variety = source.variety;
    Ok(Lexeme {
        id,
        class: construction.output_class.clone(),
        senses: vec![Sense {
            id: 0,
            meaning,
            frame: SemanticFrame::Entity {
                countable: construction.output_countable,
            },
        }],
        word: word.clone(),
        stress,
        boundaries,
        analysis: Some(Analysis {
            base: source.lexeme.clone(),
            sense,
            construction: construction.id.clone(),
        }),
        origin: Origin::Formed {
            source,
            sense,
            construction: construction.id.clone(),
            checkpoint,
        },
        traces: vec![WordTrace {
            checkpoint,
            variety,
            before: None,
            after: form_ipa(&word),
            explanation: format!("formed with {}", construction.label),
        }],
        retired: false,
    })
}

pub fn remodel_lexeme(
    grammar: &Grammar,
    lexicon: &[Lexeme],
    target: &str,
    base: &str,
    sense: u32,
    construction: &str,
    checkpoint: usize,
    variety: u64,
) -> Result<Lexeme, String> {
    let target_lex = require_active(lexicon, target)?;
    let base_lex = require_active(lexicon, base)?;
    let sense_rec = require_sense(base_lex, sense)?;
    let construction = require_construction(grammar, construction)?;
    if !eligible(construction, base_lex, sense_rec) {
        return Err(format!(
            "construction '{}' is not licensed for this sense",
            construction.id
        ));
    }
    if construction.output_class != target_lex.class {
        return Err(format!(
            "class mismatch: construction outputs '{}', target is '{}'",
            construction.output_class, target_lex.class
        ));
    }
    if let Some(existing) = existing_formation(lexicon, base, sense, &construction.id) {
        if existing.id != target {
            return Err(format!(
                "an active formation already uses construction '{}'",
                construction.id
            ));
        }
    }
    let (word, stress, boundaries, _) = realize(base_lex, construction, grammar.stress);
    let mut next = target_lex.clone();
    let before = form_ipa(&next.word);
    let after = form_ipa(&word);
    next.word = word;
    next.stress = stress;
    next.boundaries = boundaries;
    next.analysis = Some(Analysis {
        base: base.to_string(),
        sense,
        construction: construction.id.clone(),
    });
    next.traces.push(WordTrace {
        checkpoint,
        variety,
        before: Some(before),
        after,
        explanation: format!("remodeled with {}", construction.label),
    });
    Ok(next)
}

pub fn language_rule_applies(grammar: &Grammar, lexicon: &[Lexeme], rule: &SoundChange) -> bool {
    let rules = std::slice::from_ref(rule);
    lexicon.iter().any(|lex| {
        if lex.retired {
            return false;
        }
        let phones = lex.word.phones();
        let applied = apply_changes_tracked(&phones, &word_length_flags(&lex.word), rules);
        applied.phones != phones
    }) || grammar.constructions.iter().any(|c| {
        c.allomorphs.iter().any(|allo| {
            let next = apply_bound(
                &allo.phones,
                c.position,
                allo.adjacent,
                grammar.host_vowel,
                rule,
            );
            next != allo.phones
        })
    })
}

pub fn evolve_language(
    grammar: &mut Grammar,
    lexicon: &mut [Lexeme],
    rule: &SoundChange,
    aesthetic: &Aesthetic,
    checkpoint: usize,
    variety: u64,
) {
    let rules = std::slice::from_ref(rule);
    for lex in lexicon.iter_mut() {
        if lex.retired {
            continue;
        }
        let phones = lex.word.phones();
        let long = word_length_flags(&lex.word);
        let applied = apply_changes_tracked(&phones, &long, rules);
        if applied.phones == phones {
            continue;
        }
        let before = ortho::romanize(&lex.word, aesthetic);
        let next = realize_phones_long(&applied.phones, &applied.long);
        let stress_phone = nucleus_index(&lex.word, lex.stress);
        let mapped_stress = map_offset(&applied.ancestry, stress_phone);
        lex.boundaries = lex
            .boundaries
            .iter()
            .map(|b| map_offset(&applied.ancestry, *b))
            .collect();
        lex.stress = syllable_index(&next, mapped_stress);
        lex.word = next;
        let after = ortho::romanize(&lex.word, aesthetic);
        lex.traces.push(WordTrace {
            checkpoint,
            variety,
            before: Some(before),
            after,
            explanation: rule_detail(&rule.id),
        });
    }

    for construction in &mut grammar.constructions {
        let position = construction.position;
        let host = grammar.host_vowel;
        let current = construction.allomorphs.clone();
        let mut unique: Vec<BoundAllomorph> = Vec::new();
        for allo in current {
            if !unique.iter().any(|a| a.adjacent == allo.adjacent) {
                unique.push(allo);
            }
        }
        construction.allomorphs.clear();
        for allo in unique {
            let next = apply_bound(&allo.phones, position, allo.adjacent, host, rule);
            set_allomorph(
                &mut construction.allomorphs,
                if next.is_empty() { allo.phones } else { next },
                allo.adjacent,
            );
        }
    }

    let mut witnesses = Vec::new();
    for lex in lexicon.iter() {
        if lex.retired {
            continue;
        }
        let Some(analysis) = &lex.analysis else {
            continue;
        };
        let Some(construction) = grammar
            .constructions
            .iter()
            .find(|c| c.id == analysis.construction)
        else {
            continue;
        };
        let phones = lex.word.phones();
        let b = outer_boundary(&lex.boundaries, construction.position, phones.len());
        let affix = match construction.position {
            AffixPosition::Suffix => phones.get(b..).unwrap_or(&[]).to_vec(),
            AffixPosition::Prefix => phones.get(..b).unwrap_or(&[]).to_vec(),
        };
        if affix.is_empty() {
            continue;
        }
        let adjacent = match construction.position {
            AffixPosition::Suffix => b.checked_sub(1).and_then(|i| phones.get(i)).copied(),
            AffixPosition::Prefix => phones.get(b).copied(),
        };
        witnesses.push((construction.id.clone(), affix, adjacent));
    }
    for (id, phones, adjacent) in witnesses {
        if let Some(construction) = grammar.constructions.iter_mut().find(|c| c.id == id) {
            set_allomorph(&mut construction.allomorphs, phones, adjacent);
        }
    }
}

pub fn validate_language(grammar: &Grammar, lexicon: &[Lexeme]) -> Result<(), String> {
    let mut class_ids = Vec::new();
    for class in &grammar.classes {
        if class.id.trim().is_empty() {
            return Err("lexical class id must be nonempty".into());
        }
        if class_ids.contains(&class.id) {
            return Err(format!("duplicate class '{}'", class.id));
        }
        class_ids.push(class.id.clone());
    }
    let mut construction_ids = Vec::new();
    for construction in &grammar.constructions {
        if construction.id.trim().is_empty() {
            return Err("construction id must be nonempty".into());
        }
        if construction_ids.contains(&construction.id) {
            return Err(format!("duplicate construction '{}'", construction.id));
        }
        construction_ids.push(construction.id.clone());
        if !grammar
            .classes
            .iter()
            .any(|c| c.id == construction.input_class)
        {
            return Err(format!(
                "construction '{}' has unknown input class '{}'",
                construction.id, construction.input_class
            ));
        }
        let Some(out) = grammar
            .classes
            .iter()
            .find(|c| c.id == construction.output_class)
        else {
            return Err(format!(
                "construction '{}' has unknown output class '{}'",
                construction.id, construction.output_class
            ));
        };
        if out.kind != ClassKind::Entity {
            return Err(format!(
                "construction '{}' must output an entity class",
                construction.id
            ));
        }
        if construction.allomorphs.is_empty()
            || construction.allomorphs.iter().all(|a| a.phones.is_empty())
        {
            return Err(format!(
                "construction '{}' has no bound exponent",
                construction.id
            ));
        }
    }

    let mut lex_ids = Vec::new();
    for lex in lexicon {
        if lex.id.trim().is_empty() {
            return Err("lexeme id must be nonempty".into());
        }
        if lex_ids.contains(&lex.id) {
            return Err(format!("duplicate lexeme '{}'", lex.id));
        }
        lex_ids.push(lex.id.clone());
        let Some(class) = grammar.classes.iter().find(|c| c.id == lex.class) else {
            return Err(format!(
                "lexeme '{}' has unknown class '{}'",
                lex.id, lex.class
            ));
        };
        if lex.senses.is_empty() {
            return Err(format!("lexeme '{}' has no senses", lex.id));
        }
        let mut sense_ids = Vec::new();
        for sense in &lex.senses {
            if sense_ids.contains(&sense.id) {
                return Err(format!("lexeme '{}' repeats sense {}", lex.id, sense.id));
            }
            sense_ids.push(sense.id);
            match (&class.kind, &sense.frame) {
                (ClassKind::Entity, SemanticFrame::Entity { .. }) => {}
                (ClassKind::Event, SemanticFrame::Event { roles }) => {
                    if roles.is_empty() {
                        return Err(format!(
                            "lexeme '{}' sense {} has an event frame with no roles",
                            lex.id, sense.id
                        ));
                    }
                    let mut seen = Vec::new();
                    for role in roles {
                        if seen.contains(role) {
                            return Err(format!(
                                "lexeme '{}' sense {} repeats role {:?}",
                                lex.id, sense.id, role
                            ));
                        }
                        seen.push(*role);
                    }
                }
                _ => {
                    return Err(format!(
                        "lexeme '{}' sense {} frame does not match class",
                        lex.id, sense.id
                    ));
                }
            }
            match &sense.frame {
                SemanticFrame::Entity { .. } => check_meaning(&sense.meaning)?,
                SemanticFrame::Event { .. } => match &sense.meaning {
                    Meaning::Atom(name) => check_label(name)?,
                    _ => {
                        return Err(format!(
                            "lexeme '{}' sense {} event meaning must be an atom",
                            lex.id, sense.id
                        ));
                    }
                },
            }
        }
        if !lex.word.phonemes().any(|id| CATALOG.get(id).is_vowel()) {
            return Err(format!("lexeme '{}' has no vowel", lex.id));
        }
        if lex.word.syllables.is_empty() || lex.stress >= lex.word.syllables.len() {
            return Err(format!("lexeme '{}' has invalid stress", lex.id));
        }
        let nphones = lex.word.phones().len();
        if lex.boundaries.iter().any(|b| *b > nphones) {
            return Err(format!("lexeme '{}' has a boundary past the word", lex.id));
        }
        if let Some(analysis) = &lex.analysis {
            if analysis.base == lex.id {
                return Err(format!("lexeme '{}' analyzes as itself", lex.id));
            }
            let Some(base) = find_lex(lexicon, &analysis.base) else {
                return Err(format!(
                    "lexeme '{}' analyzes against unknown base '{}'",
                    lex.id, analysis.base
                ));
            };
            if require_sense(base, analysis.sense).is_err() {
                return Err(format!(
                    "lexeme '{}' analyzes against unknown sense {} on '{}'",
                    lex.id, analysis.sense, analysis.base
                ));
            }
            let Some(construction) = grammar
                .constructions
                .iter()
                .find(|c| c.id == analysis.construction)
            else {
                return Err(format!(
                    "lexeme '{}' analyzes with unknown construction '{}'",
                    lex.id, analysis.construction
                ));
            };
            if construction.input_class != base.class {
                return Err(format!(
                    "lexeme '{}' analysis input class does not match base",
                    lex.id
                ));
            }
            if construction.output_class != lex.class {
                return Err(format!(
                    "lexeme '{}' analysis output class does not match lexeme",
                    lex.id
                ));
            }
        }
    }
    for lex in lexicon {
        if has_analysis_cycle(lexicon, &lex.id) {
            return Err(format!("lexeme '{}' is in an analysis cycle", lex.id));
        }
    }
    Ok(())
}

fn class_pair(variety: u64, seed: u64) -> (LexicalClass, LexicalClass) {
    let (nid, nlab, vid, vlab) = match seed % 3 {
        0 => ("n", "noun", "v", "verb"),
        1 => ("thing", "thing", "act", "action"),
        _ => ("entity", "entity", "event", "event"),
    };
    (
        LexicalClass {
            id: format!("v{variety}:{nid}"),
            label: nlab.into(),
            kind: ClassKind::Entity,
        },
        LexicalClass {
            id: format!("v{variety}:{vid}"),
            label: vlab.into(),
            kind: ClassKind::Event,
        },
    )
}

fn seed_constructions(
    language: &Language,
    variety: u64,
    entity: &str,
    event: &str,
    rng: &mut StdRng,
) -> Vec<Construction> {
    let bits = language.seed.wrapping_add(17);
    let mut want_agent = bits & 1 != 0;
    let want_patient = bits & 2 != 0;
    let mut want_place = bits & 4 != 0;
    let mut want_coll = bits & 8 != 0;
    let want_exp = bits & 16 != 0;
    if !want_agent && !want_patient && !want_exp {
        want_agent = true;
    }
    if !want_place && !want_coll {
        if bits & 32 != 0 {
            want_place = true;
        } else {
            want_coll = true;
        }
    }
    let mut ops = Vec::new();
    if want_agent {
        ops.push(SemanticOperation::Participant(Role::Agent));
    }
    if want_patient {
        ops.push(SemanticOperation::Participant(Role::Patient));
    }
    if want_exp {
        ops.push(SemanticOperation::Participant(Role::Experiencer));
    }
    if want_place {
        ops.push(SemanticOperation::PlaceOf);
    }
    if want_coll {
        ops.push(SemanticOperation::Collective);
    }
    let mut constructions = Vec::with_capacity(ops.len());
    for (i, operation) in ops.into_iter().enumerate() {
        let input_class = if matches!(operation, SemanticOperation::Collective) {
            entity
        } else {
            event
        };
        let position = if matches!(operation, SemanticOperation::PlaceOf) && language.seed % 7 == 0
        {
            AffixPosition::Prefix
        } else {
            AffixPosition::Suffix
        };
        constructions.push(Construction {
            id: format!("v{variety}:c{i}"),
            label: construction_label(operation, language.seed, i),
            input_class: input_class.to_string(),
            output_class: entity.to_string(),
            operation,
            output_countable: true,
            position,
            allomorphs: vec![BoundAllomorph {
                phones: mint_exponent(
                    &language.inventory.consonants,
                    &language.inventory.vowels,
                    rng,
                ),
                adjacent: None,
            }],
        });
    }
    constructions
}

fn construction_label(operation: SemanticOperation, seed: u64, index: usize) -> String {
    match operation {
        SemanticOperation::Participant(Role::Agent) => {
            if seed % 2 == 0 {
                "doer"
            } else {
                "actor"
            }
        }
        SemanticOperation::Participant(Role::Patient) => "undergoer",
        SemanticOperation::Participant(Role::Experiencer) => "perceiver",
        SemanticOperation::PlaceOf => {
            if index % 2 == 0 {
                "place"
            } else {
                "ground"
            }
        }
        SemanticOperation::Collective => {
            if seed % 2 == 0 {
                "group"
            } else {
                "set"
            }
        }
    }
    .into()
}

fn mint_exponent(
    consonants: &[PhonemeId],
    vowels: &[PhonemeId],
    rng: &mut StdRng,
) -> Vec<PhonemeId> {
    if vowels.is_empty() {
        return CATALOG.id_by_ipa("a").into_iter().collect();
    }
    let vowel = vowels[rng.gen_range(0..vowels.len())];
    if consonants.is_empty() {
        return vec![vowel];
    }
    let cons = consonants[rng.gen_range(0..consonants.len())];
    match rng.gen_range(0..3u8) {
        0 => vec![cons, vowel],
        1 => vec![vowel, cons],
        _ => vec![vowel],
    }
}

fn count_sense(id: u32, gloss: &str) -> Sense {
    Sense {
        id,
        meaning: Meaning::Atom(gloss.into()),
        frame: SemanticFrame::Entity { countable: true },
    }
}

fn mass_sense(id: u32, gloss: &str) -> Sense {
    Sense {
        id,
        meaning: Meaning::Atom(gloss.into()),
        frame: SemanticFrame::Entity { countable: false },
    }
}

fn find_lex<'a>(lexicon: &'a [Lexeme], id: &str) -> Option<&'a Lexeme> {
    lexicon.iter().find(|l| l.id == id)
}

fn require_active<'a>(lexicon: &'a [Lexeme], id: &str) -> Result<&'a Lexeme, String> {
    let lex = find_lex(lexicon, id).ok_or_else(|| format!("unknown lexeme '{id}'"))?;
    if lex.retired {
        return Err(format!("lexeme '{id}' is retired"));
    }
    Ok(lex)
}

fn require_sense(lex: &Lexeme, sense: u32) -> Result<&Sense, String> {
    lex.senses
        .iter()
        .find(|s| s.id == sense)
        .ok_or_else(|| format!("unknown sense {sense} on '{}'", lex.id))
}

fn require_construction<'a>(grammar: &'a Grammar, id: &str) -> Result<&'a Construction, String> {
    grammar
        .constructions
        .iter()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("unknown construction '{id}'"))
}

fn eligible(construction: &Construction, base: &Lexeme, sense: &Sense) -> bool {
    if construction.input_class != base.class {
        return false;
    }
    match (&construction.operation, &sense.frame) {
        (SemanticOperation::Participant(role), SemanticFrame::Event { roles }) => {
            roles.contains(role)
        }
        (SemanticOperation::PlaceOf, SemanticFrame::Event { .. }) => true,
        (SemanticOperation::Collective, SemanticFrame::Entity { countable: true }) => true,
        _ => false,
    }
}

fn existing_formation<'a>(
    lexicon: &'a [Lexeme],
    base: &str,
    sense: u32,
    construction: &str,
) -> Option<&'a Lexeme> {
    lexicon.iter().find(|lex| {
        !lex.retired
            && lex.analysis.as_ref().is_some_and(|a| {
                a.base == base && a.sense == sense && a.construction == construction
            })
    })
}

fn derived_meaning(base: &Meaning, operation: SemanticOperation) -> Meaning {
    match operation {
        SemanticOperation::Participant(role) => Meaning::Participant {
            role,
            event: Box::new(base.clone()),
        },
        SemanticOperation::PlaceOf => Meaning::PlaceOf {
            event: Box::new(base.clone()),
        },
        SemanticOperation::Collective => Meaning::Collective {
            member: Box::new(base.clone()),
        },
    }
}

fn pick_allomorph<'a>(construction: &'a Construction, stem: &[PhonemeId]) -> &'a BoundAllomorph {
    let adjacent = match construction.position {
        AffixPosition::Suffix => stem.last().copied(),
        AffixPosition::Prefix => stem.first().copied(),
    };
    construction
        .allomorphs
        .iter()
        .find(|a| a.adjacent == adjacent)
        .or_else(|| {
            construction
                .allomorphs
                .iter()
                .find(|a| a.adjacent.is_none())
        })
        .unwrap_or(&construction.allomorphs[0])
}

fn realize(
    stem: &Lexeme,
    construction: &Construction,
    stress: StressPattern,
) -> (Word, usize, Vec<usize>, Word) {
    let stem_phones = stem.word.phones();
    let stem_long = word_length_flags(&stem.word);
    let allo = pick_allomorph(construction, &stem_phones);
    let (phones, long, boundary) = match construction.position {
        AffixPosition::Suffix => {
            let mut phones = stem_phones;
            let mut long = stem_long;
            let boundary = phones.len();
            phones.extend_from_slice(&allo.phones);
            long.extend(std::iter::repeat(false).take(allo.phones.len()));
            (phones, long, boundary)
        }
        AffixPosition::Prefix => {
            let mut phones = allo.phones.clone();
            let mut long = vec![false; allo.phones.len()];
            let boundary = phones.len();
            phones.extend(stem.word.phones());
            long.extend(stem_long);
            (phones, long, boundary)
        }
    };
    let word = realize_phones_long(&phones, &long);
    let stress = stress_index(word.syllables.len(), stress);
    let exponent = match construction.position {
        AffixPosition::Suffix => realize_phones(phones.get(boundary..).unwrap_or(&[])),
        AffixPosition::Prefix => realize_phones(phones.get(..boundary).unwrap_or(&[])),
    };
    let boundaries = match construction.position {
        AffixPosition::Suffix => {
            let mut bounds = stem.boundaries.clone();
            bounds.push(boundary);
            bounds
        }
        AffixPosition::Prefix => {
            let mut bounds = vec![boundary];
            bounds.extend(stem.boundaries.iter().map(|b| b + boundary));
            bounds
        }
    };
    (word, stress, boundaries, exponent)
}

fn concept_sense(id: u32, gloss: &str, frame: ConceptFrame) -> Sense {
    match frame {
        ConceptFrame::Count => count_sense(id, gloss),
        ConceptFrame::Mass => mass_sense(id, gloss),
        ConceptFrame::Event(roles) => Sense {
            id,
            meaning: Meaning::Atom(gloss.into()),
            frame: SemanticFrame::Event {
                roles: roles.to_vec(),
            },
        },
    }
}

fn push_root(
    lexicon: &mut Vec<Lexeme>,
    variety: u64,
    checkpoint: usize,
    class: &str,
    stress: StressPattern,
    slug: &str,
    word: Word,
    senses: Vec<Sense>,
) {
    let stress = stress_index(word.syllables.len(), stress);
    lexicon.push(Lexeme {
        id: format!("v{variety}:{slug}"),
        class: class.into(),
        senses,
        word: word.clone(),
        stress,
        boundaries: Vec::new(),
        analysis: None,
        origin: Origin::Unrecorded {
            variety,
            checkpoint,
        },
        traces: vec![WordTrace {
            checkpoint,
            variety,
            before: None,
            after: form_ipa(&word),
            explanation: "first recorded at this checkpoint; earlier history unrecorded".into(),
        }],
        retired: false,
    });
}
fn exponent_from_word(lex: &Lexeme, construction: &Construction) -> Word {
    let phones = lex.word.phones();
    let b = outer_boundary(&lex.boundaries, construction.position, phones.len());
    match construction.position {
        AffixPosition::Suffix => realize_phones(phones.get(b..).unwrap_or(&[])),
        AffixPosition::Prefix => realize_phones(phones.get(..b).unwrap_or(&[])),
    }
}

fn host_context(
    position: AffixPosition,
    adjacent: Option<PhonemeId>,
    host_vowel: PhonemeId,
) -> Vec<PhonemeId> {
    match adjacent {
        Some(phone) if CATALOG.get(phone).is_vowel() => vec![phone],
        Some(phone) => match position {
            AffixPosition::Suffix => vec![host_vowel, phone],
            AffixPosition::Prefix => vec![phone, host_vowel],
        },
        None => vec![host_vowel],
    }
}

fn set_allomorph(
    allomorphs: &mut Vec<BoundAllomorph>,
    phones: Vec<PhonemeId>,
    adjacent: Option<PhonemeId>,
) {
    if phones.is_empty() {
        return;
    }
    if let Some(existing) = allomorphs.iter_mut().find(|a| a.adjacent == adjacent) {
        existing.phones = phones;
    } else {
        allomorphs.push(BoundAllomorph { phones, adjacent });
    }
}

fn outer_boundary(boundaries: &[usize], position: AffixPosition, nphones: usize) -> usize {
    match position {
        AffixPosition::Suffix => boundaries.last().copied().unwrap_or(nphones).min(nphones),
        AffixPosition::Prefix => boundaries.first().copied().unwrap_or(0).min(nphones),
    }
}

fn apply_bound(
    phones: &[PhonemeId],
    position: AffixPosition,
    adjacent: Option<PhonemeId>,
    host_vowel: PhonemeId,
    rule: &SoundChange,
) -> Vec<PhonemeId> {
    let host = host_context(position, adjacent, host_vowel);
    let (segs, split) = match position {
        AffixPosition::Suffix => {
            let mut segs = host;
            let split = segs.len();
            segs.extend_from_slice(phones);
            (segs, split)
        }
        AffixPosition::Prefix => {
            let mut segs = phones.to_vec();
            let split = segs.len();
            segs.extend(host);
            (segs, split)
        }
    };
    let applied =
        apply_changes_tracked(&segs, &vec![false; segs.len()], std::slice::from_ref(rule));
    let mapped = map_offset(&applied.ancestry, split);
    match position {
        AffixPosition::Suffix => applied.phones.get(mapped..).unwrap_or(&[]).to_vec(),
        AffixPosition::Prefix => applied.phones.get(..mapped).unwrap_or(&[]).to_vec(),
    }
}

fn stress_index(syllables: usize, pattern: StressPattern) -> usize {
    if syllables == 0 {
        return 0;
    }
    match pattern {
        StressPattern::Initial => 0,
        StressPattern::Final => syllables - 1,
        StressPattern::Penultimate => syllables.saturating_sub(2),
    }
}

fn nucleus_index(word: &Word, stress: usize) -> usize {
    let mut i = 0;
    for (si, syl) in word.syllables.iter().enumerate() {
        if si == stress {
            return i + syl.onset.len();
        }
        i += syl.onset.len() + syl.nucleus.len() + syl.coda.len();
    }
    0
}

fn syllable_index(word: &Word, phone: usize) -> usize {
    let mut i = 0;
    for (si, syl) in word.syllables.iter().enumerate() {
        let n = syl.onset.len() + syl.nucleus.len() + syl.coda.len();
        if phone < i + n {
            return si;
        }
        i += n;
    }
    word.syllables.len().saturating_sub(1)
}

fn form_ipa(word: &Word) -> String {
    CATALOG.ipa_string(&word.phones())
}

fn check_label(label: &str) -> Result<(), String> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.len() > 80 || trimmed.chars().any(char::is_control) {
        return Err("meaning label must be nonempty, bounded, and free of controls".into());
    }
    Ok(())
}

fn check_meaning(meaning: &Meaning) -> Result<(), String> {
    match meaning {
        Meaning::Atom(name) => check_label(name),
        Meaning::Participant { event, .. } => check_meaning(event),
        Meaning::PlaceOf { event } => check_meaning(event),
        Meaning::Collective { member } => check_meaning(member),
    }
}

fn has_analysis_cycle(lexicon: &[Lexeme], start: &str) -> bool {
    let mut seen: Vec<String> = Vec::new();
    let mut cur = start;
    loop {
        if seen.iter().any(|id| id == cur) {
            return true;
        }
        seen.push(cur.to_string());
        let Some(lex) = find_lex(lexicon, cur) else {
            return false;
        };
        match &lex.analysis {
            Some(analysis) => cur = &analysis.base,
            None => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change::{Env, Matcher, Rewrite};
    use crate::phoneme::{Backness, Height, Manner, Place};

    fn id(ipa: &str) -> PhonemeId {
        CATALOG.id_by_ipa(ipa).unwrap()
    }

    fn phones(ipas: &[&str]) -> Word {
        realize_phones(&ipas.iter().map(|s| id(s)).collect::<Vec<_>>())
    }

    fn entity_class() -> LexicalClass {
        LexicalClass {
            id: "n".into(),
            label: "noun".into(),
            kind: ClassKind::Entity,
        }
    }

    fn event_class() -> LexicalClass {
        LexicalClass {
            id: "v".into(),
            label: "verb".into(),
            kind: ClassKind::Event,
        }
    }

    fn suffix(cid: &str, input: &str, op: SemanticOperation, phones: &[&str]) -> Construction {
        Construction {
            id: cid.into(),
            label: cid.into(),
            input_class: input.into(),
            output_class: "n".into(),
            operation: op,
            output_countable: true,
            position: AffixPosition::Suffix,
            allomorphs: vec![BoundAllomorph {
                phones: phones.iter().map(|s| id(s)).collect(),
                adjacent: None,
            }],
        }
    }

    fn grammar(constructions: Vec<Construction>) -> Grammar {
        Grammar {
            classes: vec![entity_class(), event_class()],
            constructions,
            stress: StressPattern::Initial,
            host_vowel: id("a"),
        }
    }

    fn root(id: &str, class: &str, word: Word, sense: Sense) -> Lexeme {
        Lexeme {
            id: id.into(),
            class: class.into(),
            senses: vec![sense],
            word,
            stress: 0,
            boundaries: Vec::new(),
            analysis: None,
            origin: Origin::Unrecorded {
                variety: 1,
                checkpoint: 0,
            },
            traces: Vec::new(),
            retired: false,
        }
    }

    fn fishing() -> Lexeme {
        root(
            "fishing",
            "v",
            phones(&["p", "a"]),
            Sense {
                id: 0,
                meaning: Meaning::Atom("fishing".into()),
                frame: SemanticFrame::Event {
                    roles: vec![Role::Agent, Role::Patient],
                },
            },
        )
    }

    fn dying() -> Lexeme {
        root(
            "dying",
            "v",
            phones(&["m", "a"]),
            Sense {
                id: 0,
                meaning: Meaning::Atom("dying".into()),
                frame: SemanticFrame::Event {
                    roles: vec![Role::Patient],
                },
            },
        )
    }

    fn tree() -> Lexeme {
        root(
            "tree",
            "n",
            phones(&["t", "a"]),
            Sense {
                id: 0,
                meaning: Meaning::Atom("tree".into()),
                frame: SemanticFrame::Entity { countable: true },
            },
        )
    }

    fn water() -> Lexeme {
        root(
            "water",
            "n",
            phones(&["w", "a"]),
            Sense {
                id: 0,
                meaning: Meaning::Atom("water".into()),
                frame: SemanticFrame::Entity { countable: false },
            },
        )
    }

    fn t_to_s_before_i() -> SoundChange {
        SoundChange {
            id: "t_to_s_before_i".into(),
            target: Matcher::Consonant {
                place: Some(Place::Alveolar),
                manner: Some(Manner::Stop),
                voiced: Some(false),
            },
            result: Rewrite::Consonant {
                place: None,
                manner: Some(Manner::Fricative),
                voiced: None,
            },
            left: Env::Any,
            right: Env::Matcher(Matcher::Vowel {
                height: Some(Height::Close),
                backness: Some(Backness::Front),
                rounded: Some(false),
            }),
        }
    }

    fn k_to_x_initial() -> SoundChange {
        SoundChange {
            id: "k_to_x_initial".into(),
            target: Matcher::Consonant {
                place: Some(Place::Velar),
                manner: Some(Manner::Stop),
                voiced: Some(false),
            },
            result: Rewrite::Consonant {
                place: Some(Place::Velar),
                manner: Some(Manner::Fricative),
                voiced: Some(false),
            },
            left: Env::WordEdge,
            right: Env::Any,
        }
    }

    fn ipa(word: &Word) -> String {
        CATALOG.ipa_string(&word.phones())
    }

    fn has_gloss(lexicon: &[Lexeme], gloss: &str) -> bool {
        lexicon
            .iter()
            .any(|l| l.senses.iter().any(|s| s.meaning.label() == gloss))
    }

    fn colex_tree_wood(lexicon: &[Lexeme]) -> bool {
        lexicon.iter().any(|l| {
            let tree = l.senses.iter().any(|s| s.meaning.label() == "tree");
            let wood = l.senses.iter().any(|s| s.meaning.label() == "wood");
            tree && wood
        })
    }

    #[test]
    fn agentive_sense_licenses_agent_construction() {
        let grammar = grammar(vec![suffix(
            "v1:c0",
            "v",
            SemanticOperation::Participant(Role::Agent),
            &["i"],
        )]);
        let lexicon = vec![fishing(), dying()];
        let fish = formation_options(&grammar, &lexicon, "fishing", 0).unwrap();
        let die = formation_options(&grammar, &lexicon, "dying", 0).unwrap();
        assert_eq!(fish.len(), 1);
        assert_eq!(fish[0].gloss, "agent of fishing");
        assert!(die.is_empty());
        assert!(
            derive_lexeme(
                &grammar,
                &lexicon,
                LexemeRef {
                    variety: 1,
                    checkpoint: 0,
                    lexeme: "dying".into(),
                },
                0,
                "v1:c0",
                "bad".into(),
                1,
            )
            .is_err()
        );
    }

    #[test]
    fn collective_licenses_count_not_mass() {
        let grammar = grammar(vec![suffix(
            "v1:c0",
            "n",
            SemanticOperation::Collective,
            &["i"],
        )]);
        let lexicon = vec![tree(), water()];
        let trees = formation_options(&grammar, &lexicon, "tree", 0).unwrap();
        let waters = formation_options(&grammar, &lexicon, "water", 0).unwrap();
        assert_eq!(trees.len(), 1);
        assert_eq!(trees[0].gloss, "collection of tree");
        assert!(waters.is_empty());
    }

    #[test]
    fn bound_exponent_skips_word_initial_rule() {
        let aesthetic = Aesthetic::by_id("elvish").unwrap();
        let mut grammar = grammar(vec![suffix(
            "v1:c0",
            "v",
            SemanticOperation::Participant(Role::Agent),
            &["k", "a"],
        )]);
        let stem = root(
            "go",
            "v",
            phones(&["t", "i"]),
            Sense {
                id: 0,
                meaning: Meaning::Atom("go".into()),
                frame: SemanticFrame::Event {
                    roles: vec![Role::Agent],
                },
            },
        );
        let free = root("ka", "n", phones(&["k", "a"]), count_sense(0, "stone"));
        let derived = derive_lexeme(
            &grammar,
            &[stem.clone()],
            LexemeRef {
                variety: 1,
                checkpoint: 0,
                lexeme: "go".into(),
            },
            0,
            "v1:c0",
            "goer".into(),
            1,
        )
        .unwrap();
        assert_eq!(ipa(&derived.word), "tika");
        let mut lexicon = vec![stem, free, derived];
        evolve_language(
            &mut grammar,
            &mut lexicon,
            &k_to_x_initial(),
            &aesthetic,
            2,
            1,
        );
        let ka = lexicon.iter().find(|l| l.id == "ka").unwrap();
        let goer = lexicon.iter().find(|l| l.id == "goer").unwrap();
        assert_eq!(ipa(&ka.word), "xa");
        assert_eq!(ipa(&goer.word), "tika");
        assert_eq!(
            CATALOG.ipa_string(&grammar.constructions[0].allomorphs[0].phones),
            "ka"
        );
        let option = formation_options(&grammar, &lexicon, "go", 0)
            .unwrap()
            .remove(0);
        assert_eq!(option.existing.as_deref(), Some("goer"));
        assert_eq!(ipa(&option.word), "tika");
    }

    #[test]
    fn boundary_conditioned_change_and_copied_meaning() {
        let aesthetic = Aesthetic::by_id("elvish").unwrap();
        let mut grammar = grammar(vec![suffix(
            "v1:c0",
            "v",
            SemanticOperation::Participant(Role::Agent),
            &["i"],
        )]);
        let mut stem = fishing();
        stem.word = phones(&["k", "a", "t"]);
        let derived = derive_lexeme(
            &grammar,
            &[stem.clone()],
            LexemeRef {
                variety: 1,
                checkpoint: 0,
                lexeme: "fishing".into(),
            },
            0,
            "v1:c0",
            "fisher".into(),
            1,
        )
        .unwrap();
        assert_eq!(ipa(&derived.word), "kati");
        assert_eq!(
            derived.senses[0].meaning,
            Meaning::Participant {
                role: Role::Agent,
                event: Box::new(Meaning::Atom("fishing".into())),
            }
        );
        let origin = derived.origin.clone();
        let mut lexicon = vec![stem, derived];
        lexicon[0].senses[0].meaning = Meaning::Atom("hunting".into());
        lexicon[0].senses[0].frame = SemanticFrame::Event {
            roles: vec![Role::Patient],
        };
        assert_eq!(lexicon[1].senses[0].meaning.label(), "agent of fishing");
        assert!(matches!(
            lexicon[1].analysis.as_ref(),
            Some(Analysis {
                base,
                sense: 0,
                construction
            }) if base == "fishing" && construction == "v1:c0"
        ));
        evolve_language(
            &mut grammar,
            &mut lexicon,
            &t_to_s_before_i(),
            &aesthetic,
            2,
            1,
        );
        let fisher = lexicon.iter().find(|l| l.id == "fisher").unwrap();
        let fishing = lexicon.iter().find(|l| l.id == "fishing").unwrap();
        assert_eq!(ipa(&fishing.word), "kat");
        assert_eq!(ipa(&fisher.word), "kasi");
        assert_eq!(fisher.origin, origin);
        let option = formation_options(&grammar, &lexicon, "fishing", 0)
            .unwrap()
            .remove(0);
        assert_eq!(ipa(&option.word), "kasi");
        assert_eq!(option.existing.as_deref(), Some("fisher"));
        assert_eq!(option.gloss, "agent of fishing");
    }

    #[test]
    fn remodel_keeps_origin_and_senses() {
        let grammar = grammar(vec![
            suffix(
                "v1:c0",
                "v",
                SemanticOperation::Participant(Role::Agent),
                &["i"],
            ),
            suffix(
                "v1:c1",
                "v",
                SemanticOperation::Participant(Role::Patient),
                &["u"],
            ),
        ]);
        let stem = fishing();
        let derived = derive_lexeme(
            &grammar,
            &[stem.clone()],
            LexemeRef {
                variety: 1,
                checkpoint: 0,
                lexeme: "fishing".into(),
            },
            0,
            "v1:c0",
            "fisher".into(),
            1,
        )
        .unwrap();
        let origin = derived.origin.clone();
        let senses = derived.senses.clone();
        let lexicon = vec![stem, derived];
        let remodeled =
            remodel_lexeme(&grammar, &lexicon, "fisher", "fishing", 0, "v1:c1", 2, 1).unwrap();
        assert_eq!(remodeled.origin, origin);
        assert_eq!(remodeled.senses, senses);
        assert_eq!(
            remodeled.analysis.as_ref().map(|a| a.construction.as_str()),
            Some("v1:c1")
        );
        assert_eq!(ipa(&remodeled.word), "pau");
    }

    #[test]
    fn existing_attested_form_is_not_recomposed() {
        let grammar = grammar(vec![suffix(
            "v1:c0",
            "v",
            SemanticOperation::Participant(Role::Agent),
            &["i"],
        )]);
        let stem = fishing();
        let mut derived = derive_lexeme(
            &grammar,
            &[stem.clone()],
            LexemeRef {
                variety: 1,
                checkpoint: 0,
                lexeme: "fishing".into(),
            },
            0,
            "v1:c0",
            "fisher".into(),
            1,
        )
        .unwrap();
        derived.word = phones(&["s", "u"]);
        let lexicon = vec![stem, derived];
        let option = formation_options(&grammar, &lexicon, "fishing", 0)
            .unwrap()
            .remove(0);
        assert_eq!(option.existing.as_deref(), Some("fisher"));
        assert_eq!(ipa(&option.word), "su");
    }

    #[test]
    fn seed_has_required_senses_and_namespaced_constructions() {
        let aesthetic = Aesthetic::by_id("elvish").unwrap();
        let lang = Language::new(7, aesthetic.clone());
        let (grammar, lexicon) = seed_language(&lang, 3, 0);
        assert!(validate_language(&grammar, &lexicon).is_ok());
        for (gloss, _) in crate::lexicon::glosses() {
            assert!(has_gloss(&lexicon, gloss), "missing {gloss}");
        }
        for gloss in ["fishing", "dying", "wood", "river", "day"] {
            assert!(has_gloss(&lexicon, gloss), "missing {gloss}");
        }
        let fishing = lexicon
            .iter()
            .find(|l| l.senses.iter().any(|s| s.meaning.label() == "fishing"))
            .unwrap();
        let dying = lexicon
            .iter()
            .find(|l| l.senses.iter().any(|s| s.meaning.label() == "dying"))
            .unwrap();
        match &fishing.senses[0].frame {
            SemanticFrame::Event { roles } => assert!(roles.contains(&Role::Agent)),
            _ => panic!("fishing should be agentive"),
        }
        match &dying.senses[0].frame {
            SemanticFrame::Event { roles } => assert!(!roles.contains(&Role::Agent)),
            _ => panic!("dying should be an event"),
        }
        for c in &grammar.constructions {
            assert!(
                c.id.starts_with("v3:c"),
                "construction {} should be variety-scoped",
                c.id
            );
        }
        let formed = lexicon.iter().filter(|l| l.analysis.is_some()).count();
        let roots = lexicon.iter().filter(|l| l.analysis.is_none()).count();
        assert!(formed > 0 && roots > formed);

        let mut saw_diff = false;
        for seed in 1..12u64 {
            let a = seed_language(&Language::new(seed, aesthetic.clone()), 1, 0).1;
            let b = seed_language(&Language::new(seed + 1, aesthetic.clone()), 1, 0).1;
            if colex_tree_wood(&a) != colex_tree_wood(&b) {
                saw_diff = true;
                break;
            }
        }
        assert!(saw_diff, "tree/wood colexification should vary by language");
    }

    fn p_lenite_v_v() -> SoundChange {
        SoundChange {
            id: "p_lenite_v_v".into(),
            target: Matcher::Consonant {
                place: Some(Place::Bilabial),
                manner: Some(Manner::Stop),
                voiced: Some(false),
            },
            result: Rewrite::Consonant {
                place: Some(Place::Labiodental),
                manner: Some(Manner::Fricative),
                voiced: Some(false),
            },
            left: Env::Matcher(Matcher::AnyVowel),
            right: Env::Matcher(Matcher::AnyVowel),
        }
    }

    fn event_stem(id: &str, word: Word) -> Lexeme {
        let mut lex = fishing();
        lex.id = id.into();
        lex.word = word;
        lex
    }

    #[test]
    fn repeated_conditioned_evolution_keeps_consonant_context() {
        let aesthetic = Aesthetic::by_id("elvish").unwrap();
        let mut grammar = grammar(vec![suffix(
            "v1:c0",
            "v",
            SemanticOperation::Participant(Role::Agent),
            &["p", "a"],
        )]);
        let vowel_stem = event_stem("ta", phones(&["t", "a"]));
        let cons_stem = event_stem("at", phones(&["a", "t"]));
        let from_vowel = derive_lexeme(
            &grammar,
            &[vowel_stem.clone()],
            LexemeRef {
                variety: 1,
                checkpoint: 0,
                lexeme: "ta".into(),
            },
            0,
            "v1:c0",
            "ta-er".into(),
            1,
        )
        .unwrap();
        let from_cons = derive_lexeme(
            &grammar,
            &[cons_stem.clone()],
            LexemeRef {
                variety: 1,
                checkpoint: 0,
                lexeme: "at".into(),
            },
            0,
            "v1:c0",
            "at-er".into(),
            1,
        )
        .unwrap();
        assert_eq!(ipa(&from_vowel.word), "tapa");
        assert_eq!(ipa(&from_cons.word), "atpa");
        let mut lexicon = vec![vowel_stem, cons_stem, from_vowel, from_cons];
        let rule = p_lenite_v_v();
        evolve_language(&mut grammar, &mut lexicon, &rule, &aesthetic, 2, 1);
        evolve_language(&mut grammar, &mut lexicon, &rule, &aesthetic, 3, 1);
        let ta_er = lexicon.iter().find(|l| l.id == "ta-er").unwrap();
        let at_er = lexicon.iter().find(|l| l.id == "at-er").unwrap();
        assert_eq!(ipa(&ta_er.word), "tafa");
        assert_eq!(ipa(&at_er.word), "atpa");
        let pa_after_t = grammar.constructions[0]
            .allomorphs
            .iter()
            .find(|a| a.adjacent == Some(id("t")))
            .expect("consonant-conditioned allomorph");
        assert_eq!(CATALOG.ipa_string(&pa_after_t.phones), "pa");
        let after_a = grammar.constructions[0]
            .allomorphs
            .iter()
            .find(|a| a.adjacent == Some(id("a")))
            .expect("vowel-conditioned allomorph");
        assert_eq!(CATALOG.ipa_string(&after_a.phones), "fa");
    }

    #[test]
    fn realize_preserves_stem_length_and_nested_boundaries() {
        let grammar = grammar(vec![
            suffix(
                "v1:c0",
                "v",
                SemanticOperation::Participant(Role::Agent),
                &["i"],
            ),
            suffix("v1:c1", "n", SemanticOperation::Collective, &["u"]),
        ]);
        let mut stem = fishing();
        stem.word.syllables[0].long = true;
        let inner = derive_lexeme(
            &grammar,
            &[stem.clone()],
            LexemeRef {
                variety: 1,
                checkpoint: 0,
                lexeme: "fishing".into(),
            },
            0,
            "v1:c0",
            "fisher".into(),
            1,
        )
        .unwrap();
        assert!(inner.word.syllables[0].long);
        assert_eq!(inner.boundaries, vec![2]);
        let lexicon = vec![stem, inner];
        let outer = derive_lexeme(
            &grammar,
            &lexicon,
            LexemeRef {
                variety: 1,
                checkpoint: 1,
                lexeme: "fisher".into(),
            },
            0,
            "v1:c1",
            "fishers".into(),
            2,
        )
        .unwrap();
        assert!(outer.word.syllables[0].long);
        assert_eq!(outer.boundaries, vec![2, 3]);
        let phones = outer.word.phones();
        let outer_b = *outer.boundaries.last().unwrap();
        assert_eq!(CATALOG.ipa_string(&phones[outer_b..]), "u");
    }

    #[test]
    fn retired_words_do_not_evolve_or_witness() {
        let aesthetic = Aesthetic::by_id("elvish").unwrap();
        let mut grammar = grammar(vec![suffix(
            "v1:c0",
            "v",
            SemanticOperation::Participant(Role::Agent),
            &["p", "a"],
        )]);
        let mut frozen = event_stem("old", phones(&["t", "a", "p", "a"]));
        frozen.retired = true;
        frozen.analysis = Some(Analysis {
            base: "go".into(),
            sense: 0,
            construction: "v1:c0".into(),
        });
        frozen.boundaries = vec![2];
        let mut lexicon = vec![frozen];
        evolve_language(
            &mut grammar,
            &mut lexicon,
            &p_lenite_v_v(),
            &aesthetic,
            2,
            1,
        );
        assert_eq!(ipa(&lexicon[0].word), "tapa");
        assert!(
            !grammar.constructions[0]
                .allomorphs
                .iter()
                .any(|a| CATALOG.ipa_string(&a.phones) == "fa" && a.adjacent == Some(id("a")))
        );
    }
}

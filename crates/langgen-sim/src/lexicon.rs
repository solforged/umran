use crate::concepts::{CONCEPTS, Concept};
use crate::form::Form;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LexemeId(pub u32);

/// How a word entered the language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// One of the roots minted when the variety was founded.
    Founding,
    /// A new, expressive root coined from the language's current sounds.
    Expressive,
}

/// One word: a form plus its history. Meanings live in `Slot`s, so a
/// word can serve several concepts and gain or lose them over time.
#[derive(Clone, Debug, PartialEq)]
pub struct Lexeme {
    pub id: LexemeId,
    pub form: Form,
    pub origin: Origin,
    /// The concept the word first expressed.
    pub first_sense: &'static Concept,
    pub born: u32,
    /// Generation the word stopped being used for any concept.
    pub obsolete: Option<u32>,
    pub log: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub generation: u32,
    pub event: Event,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    SoundLaw {
        law: &'static str,
        before: Form,
    },
    /// The word began to be used for another concept as well.
    Extended {
        to: &'static Concept,
    },
    /// The word fell out of use for a concept.
    Lost {
        sense: &'static Concept,
    },
    Obsolete,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variant {
    pub lexeme: LexemeId,
    /// Share of usage for the slot's concept; a slot's weights sum to 1.
    pub weight: f32,
}

/// The words competing to express one concept.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub concept: &'static Concept,
    pub variants: Vec<Variant>,
}

impl Slot {
    pub fn dominant(&self) -> Option<LexemeId> {
        self.variants
            .iter()
            .max_by(|a, b| a.weight.total_cmp(&b.weight))
            .map(|v| v.lexeme)
    }

    pub fn has(&self, lexeme: LexemeId) -> bool {
        self.variants.iter().any(|v| v.lexeme == lexeme)
    }

    /// Adds a newcomer at `weight`, scaling incumbents to make room.
    pub fn introduce(&mut self, lexeme: LexemeId, weight: f32) {
        for v in &mut self.variants {
            v.weight *= 1.0 - weight;
        }
        self.variants.push(Variant { lexeme, weight });
    }
}

/// Every word a variety has ever had, and one slot per concept.
#[derive(Clone, Debug, PartialEq)]
pub struct Lexicon {
    pub lexemes: Vec<Lexeme>,
    /// Parallel to `CONCEPTS`.
    pub slots: Vec<Slot>,
}

impl Lexicon {
    /// One founding word per concept, in `CONCEPTS` order.
    pub fn found(roots: impl IntoIterator<Item = (&'static Concept, Form)>) -> Self {
        let mut lexicon = Self {
            lexemes: Vec::new(),
            slots: CONCEPTS
                .iter()
                .map(|concept| Slot {
                    concept,
                    variants: Vec::new(),
                })
                .collect(),
        };
        for (concept, form) in roots {
            let id = lexicon.coin(form, Origin::Founding, concept, 0);
            lexicon.slot_mut(concept).introduce(id, 1.0);
        }
        lexicon
    }

    pub fn coin(
        &mut self,
        form: Form,
        origin: Origin,
        sense: &'static Concept,
        born: u32,
    ) -> LexemeId {
        let id = LexemeId(self.lexemes.len() as u32);
        self.lexemes.push(Lexeme {
            id,
            form,
            origin,
            first_sense: sense,
            born,
            obsolete: None,
            log: Vec::new(),
        });
        id
    }

    pub fn get(&self, id: LexemeId) -> &Lexeme {
        &self.lexemes[id.0 as usize]
    }

    pub fn get_mut(&mut self, id: LexemeId) -> &mut Lexeme {
        &mut self.lexemes[id.0 as usize]
    }

    pub fn slot(&self, concept: &Concept) -> &Slot {
        let i = CONCEPTS
            .iter()
            .position(|c| c.id == concept.id)
            .expect("known concept");
        &self.slots[i]
    }

    pub fn slot_mut(&mut self, concept: &Concept) -> &mut Slot {
        let i = CONCEPTS
            .iter()
            .position(|c| c.id == concept.id)
            .expect("known concept");
        &mut self.slots[i]
    }

    pub fn living(&self) -> impl Iterator<Item = &Lexeme> {
        self.lexemes.iter().filter(|l| l.obsolete.is_none())
    }

    /// The dominant word for a concept, if any.
    pub fn word_for(&self, concept: &Concept) -> Option<&Lexeme> {
        self.slot(concept).dominant().map(|id| self.get(id))
    }

    /// Concepts a word currently expresses.
    pub fn senses(&self, id: LexemeId) -> impl Iterator<Item = &'static Concept> + '_ {
        self.slots
            .iter()
            .filter(move |s| s.has(id))
            .map(|s| s.concept)
    }

    /// Words in use that sound exactly like a different word in use.
    pub fn clashing(&self) -> HashSet<LexemeId> {
        let mut by_form: HashMap<&Form, Vec<LexemeId>> = HashMap::new();
        let in_use: HashSet<LexemeId> = self
            .slots
            .iter()
            .flat_map(|s| s.variants.iter().map(|v| v.lexeme))
            .collect();
        for &id in &in_use {
            by_form.entry(&self.get(id).form).or_default().push(id);
        }
        by_form
            .into_values()
            .filter(|ids| ids.len() > 1)
            .flatten()
            .collect()
    }

    /// Share of Leipzig–Jakarta concepts still dominated by their founding
    /// word: the classic glottochronological retention measure. Sound
    /// change does not count as replacement; a word stays itself.
    pub fn core_retention(&self) -> f32 {
        let core: Vec<&Slot> = self
            .slots
            .iter()
            .filter(|s| s.concept.stability.is_some())
            .collect();
        let kept = core
            .iter()
            .filter(|s| self.keeps_founding_word(s.concept))
            .count();
        kept as f32 / core.len() as f32
    }

    /// Whether `concept` is still dominated by the root it was founded with.
    pub fn keeps_founding_word(&self, concept: &Concept) -> bool {
        self.word_for(concept)
            .is_some_and(|l| l.origin == Origin::Founding && l.first_sense.id == concept.id)
    }
}

use crate::concepts::{CONCEPTS, Concept, Relation};
use crate::form::Form;
use crate::grammar::Paradigm;
use crate::root::Minted;
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
    /// Taken from another variety's word, adapted to this one's sounds.
    Borrowed { from: usize, source: LexemeId },
    /// Built from another word of the same language: fish > fishing.
    Derived { base: LexemeId, relation: Relation },
    /// Rebuilt from a word that had worn too short or come to sound like
    /// another: with the renewing affix (Latin auris > auricula, "ear"),
    /// or compounded `with` a related word (Mandarin ěr > ěrduo).
    Renewed {
        base: LexemeId,
        with: Option<LexemeId>,
    },
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
    /// Stored grammatical forms, separate from words competing for meanings.
    pub paradigms: Vec<Paradigm>,
}

impl Lexeme {
    /// The form the word had at `generation`, before any later sound law.
    pub fn form_at(&self, generation: u32) -> &Form {
        self.log
            .iter()
            .filter(|e| e.generation > generation)
            .find_map(|e| match &e.event {
                Event::SoundLaw { before, .. } => Some(before),
                _ => None,
            })
            .unwrap_or(&self.form)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub generation: u32,
    pub event: Event,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Entered from variety `from`, where it sounded like `source`.
    Borrowed {
        from: usize,
        source: Form,
    },
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

    /// Adds a newcomer at `weight`, scaling incumbents to make room. The
    /// first word for a meaning takes all its uses, whatever `weight`.
    pub fn introduce(&mut self, lexeme: LexemeId, weight: f32) {
        let weight = if self.variants.is_empty() {
            1.0
        } else {
            weight
        };
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
    pub fn found(roots: impl IntoIterator<Item = Minted>) -> Self {
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
        let roots: Vec<Minted> = roots.into_iter().collect();
        for minted in &roots {
            let origin = match minted.derived {
                // Founding words are coined in concept order, so a base's
                // id is its position in the list.
                Some((base, relation)) => {
                    let base = roots
                        .iter()
                        .position(|m| m.concept.id == base.id)
                        .expect("bases are minted");
                    Origin::Derived {
                        base: LexemeId(base as u32),
                        relation,
                    }
                }
                None => Origin::Founding,
            };
            let id = lexicon.coin(minted.form.clone(), origin, minted.concept, 0);
            lexicon.slot_mut(minted.concept).introduce(id, 1.0);
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
            paradigms: Vec::new(),
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

    /// Whether `concept` is still dominated by the word it was founded with,
    /// a root or a word derived at founding. A renewed word keeps its old
    /// root, as French oreille keeps Latin auris, so it counts as kept.
    pub fn keeps_founding_word(&self, concept: &Concept) -> bool {
        let mut word = self.word_for(concept);
        while let Some(Origin::Renewed { base, .. }) = word.map(|l| l.origin) {
            word = Some(self.get(base));
        }
        word.is_some_and(|l| {
            l.born == 0
                && matches!(l.origin, Origin::Founding | Origin::Derived { .. })
                && l.first_sense.id == concept.id
        })
    }
}

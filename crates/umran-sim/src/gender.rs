//! Semantic noun classes, formal assignment, and agreement on determiners.
//! Following Corbett (1991), agreement is the observable class distinction.
use crate::concepts::by_id;
use crate::form::Form;
use crate::grammar::{GrammarEntry, GrammarEvent, Side, same_sound};
use crate::laws::Law;
use crate::lexicon::{LexemeId, Lexicon, Origin};
use crate::morphology::Morphology;
use crate::phonotactics::Phonotactics;
use crate::profile::SoundProfile;
use crate::prosody::{MinimalWord, StressRule};
use crate::rng::{index, key, stream, weighted_index};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClassChoice {
    None,
    Sex,
    Animacy,
    Many,
}
impl ClassChoice {
    pub fn draw(seed: u64) -> Self {
        let mut rng = stream(seed, &[key("noun classes founding")]);
        [Self::None, Self::Sex, Self::Animacy, Self::Many]
            [weighted_index(&mut rng, [0.60, 0.20, 0.15, 0.05].into_iter())]
    }
    /// Size is independent of the semantic basis. These are founding priors,
    /// not observed modern frequencies: sound mergers and recruitment follow.
    pub fn draw_count(self, seed: u64) -> usize {
        if self == Self::None {
            return 0;
        }
        let mut rng = stream(seed, &[key("noun class count founding")]);
        draw_count(&mut rng)
    }
    fn cores(self, count: usize) -> &'static [Core] {
        use Core::*;
        let cores: &[Core] = match self {
            Self::None => return &[],
            Self::Sex => &[Male, Female, Remainder, Animate, Long, Flat, Round, Mass],
            Self::Animacy if count == 2 => &[Animate, Remainder],
            Self::Animacy => &[Human, Animate, Remainder, Long, Flat, Round, Mass, Formal],
            Self::Many => &[Human, Remainder, Animate, Long, Flat, Round, Mass, Formal],
        };
        &cores[..count]
    }
}

fn draw_count(rng: &mut impl Rng) -> usize {
    match weighted_index(rng, [0.34, 0.23, 0.11, 0.32].into_iter()) {
        0 => 2,
        1 => 3,
        2 => 4,
        _ => 5 + index(rng, 4),
    }
}

/// Metadata about the meaning, independent of a language's class system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NounSemantics {
    Male,
    Female,
    Human,
    Animate,
    Long,
    Flat,
    Round,
    Mass,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Basis {
    Animacy,
    Sex,
    Shape,
    Formal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Core {
    Male,
    Female,
    Human,
    Animate,
    Long,
    Flat,
    Round,
    Mass,
    Remainder,
    Formal,
}
impl Core {
    fn basis(self, choice: ClassChoice) -> Basis {
        match self {
            Self::Male | Self::Female => Basis::Sex,
            Self::Human | Self::Animate => Basis::Animacy,
            Self::Long | Self::Flat | Self::Round | Self::Mass => Basis::Shape,
            Self::Remainder if choice == ClassChoice::Animacy => Basis::Animacy,
            _ => Basis::Formal,
        }
    }
    fn matches(self, noun: NounSemantics) -> bool {
        use NounSemantics as N;
        matches!(
            (self, noun),
            (Self::Male, N::Male)
                | (Self::Female, N::Female)
                | (Self::Human, N::Male | N::Female | N::Human)
                | (Self::Animate, N::Male | N::Female | N::Human | N::Animate)
                | (Self::Long, N::Long)
                | (Self::Flat, N::Flat)
                | (Self::Round, N::Round)
                | (Self::Mass, N::Mass)
        )
    }
    fn source(self) -> &'static str {
        match self {
            Self::Male => "father",
            Self::Female => "mother",
            Self::Human => "person",
            Self::Animate => "dog",
            Self::Long => "tree",
            Self::Flat => "leaf",
            Self::Round => "stone",
            Self::Mass => "water",
            Self::Remainder => "this",
            Self::Formal => "all",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NounClass {
    pub id: u32,
    pub basis: Basis,
    pub marker: Form,
    pub members: usize,
    pub born: u32,
    pub merged_into: Option<u32>,
    pub retired: Option<u32>,
    /// A lexical classifier or demonstrative copied at emergence; none at founding.
    pub source: Option<LexemeId>,
    /// Complete determiner, including its class affix. Never rebuilt on a read.
    pub agreement: Form,
    pub history: Vec<GrammarEntry>,
    core: Core,
    edge: usize,
}
impl NounClass {
    pub fn agreement_at(&self, generation: u32) -> &Form {
        self.history
            .iter()
            .filter(|e| e.generation > generation)
            .find_map(|e| {
                if let GrammarEvent::SoundLaw { before, .. } = &e.event {
                    Some(before)
                } else {
                    None
                }
            })
            .unwrap_or(&self.agreement)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassChange {
    Emerged { classes: Vec<u32> },
    Merged { class: u32, into: u32 },
    Lost,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassNotice {
    pub generation: u32,
    pub change: ClassChange,
    /// Exact phonological trigger, rather than a cause inferred from adjacent prose.
    pub cause: Option<SoundCause>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundCause {
    pub law: &'static str,
    pub generation: u32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gender {
    pub classes: Vec<NounClass>,
    pub events: Vec<ClassNotice>,
    pub lost: Option<u32>,
    side: Option<Side>,
    assignments: Vec<Option<u32>>,
    /// Reused scratch, filled from current senses (including semantic extensions).
    nouns: Vec<Option<NounSemantics>>,
}
impl Gender {
    pub fn found(
        seed: u64,
        profile: &SoundProfile,
        tactics: &Phonotactics,
        morphology: &Morphology,
        stress: StressRule,
        lexicon: &Lexicon,
    ) -> Self {
        let choice = profile
            .grammar
            .classes
            .unwrap_or_else(|| ClassChoice::draw(seed));
        let mut gender = Self::default();
        if choice == ClassChoice::None {
            return gender;
        }
        let count = choice.draw_count(seed);
        let mut rng = stream(seed, &[key("noun class markers")]);
        let mut candidates = Vec::new();
        for &(vowel, _) in &tactics.nuclei {
            candidates.push(Form::from_phones(vec![vowel]));
            for (onset, _) in tactics.onsets.iter().filter(|(o, _)| o.len() == 1) {
                candidates.push(Form::from_phones(vec![onset[0], vowel]));
            }
        }
        let markers = choice
            .cores(count)
            .iter()
            .map(|_| {
                let at = index(&mut rng, candidates.len());
                (candidates.swap_remove(at), None)
            })
            .collect();
        let side = if rng.r#gen::<f32>() < profile.morphology.suffixing {
            Side::Suffix
        } else {
            Side::Prefix
        };
        gender.establish(choice, markers, side, morphology, stress, lexicon, 0);
        gender
    }
    #[allow(clippy::too_many_arguments)]
    fn establish(
        &mut self,
        choice: ClassChoice,
        markers: Vec<(Form, Option<LexemeId>)>,
        side: Side,
        morphology: &Morphology,
        stress: StressRule,
        lexicon: &Lexicon,
        generation: u32,
    ) {
        let Some(target) = lexicon.word_for(by_id("this").unwrap()) else {
            return;
        };
        let start = self.classes.len();
        for (&core, (marker, source)) in choice.cores(markers.len()).iter().zip(markers) {
            let mut agreement = match side {
                Side::Prefix => morphology.join(&marker, &target.form),
                Side::Suffix => morphology.join(&target.form, &marker),
            };
            let edge = agreement.boundaries[0];
            if stress == StressRule::Free && agreement.stress.is_none() {
                agreement.stress = Some(match side {
                    Side::Suffix => target.form.stress.unwrap_or(0),
                    Side::Prefix => {
                        agreement
                            .vowel_count()
                            .saturating_sub(target.form.vowel_count())
                            + target.form.stress.unwrap_or(0)
                    }
                });
            }
            self.classes.push(NounClass {
                id: self.classes.len() as u32,
                basis: core.basis(choice),
                marker,
                members: 0,
                born: generation,
                merged_into: None,
                retired: None,
                source,
                agreement,
                history: Vec::new(),
                core,
                edge,
            });
        }
        // A junction can erase distinctions even before the first sound law.
        // Reject an indistinguishable founding system instead of claiming agreement.
        if self.classes[start..].iter().enumerate().any(|(i, a)| {
            self.classes[start..start + i]
                .iter()
                .any(|b| same_sound(&a.agreement, &b.agreement, stress))
        }) {
            self.classes.truncate(start);
            return;
        }
        self.side = Some(side);
        self.lost = None;
        if generation > 0 {
            self.events.push(ClassNotice {
                generation,
                change: ClassChange::Emerged {
                    classes: (start..self.classes.len()).map(|i| i as u32).collect(),
                },
                cause: None,
            });
        }
        self.sync(lexicon);
    }
    pub fn active_count(&self) -> usize {
        self.classes.iter().filter(|c| c.retired.is_none()).count()
    }
    pub fn class_of(&self, word: LexemeId) -> Option<u32> {
        self.assignments.get(word.0 as usize).copied().flatten()
    }
    pub fn agreement(&self, word: LexemeId) -> Option<&NounClass> {
        let class = &self.classes[self.class_of(word)? as usize];
        class.retired.is_none().then_some(class)
    }
    pub fn forms(&self) -> impl Iterator<Item = (&Form, f32)> {
        self.classes
            .iter()
            .filter(|c| c.retired.is_none())
            .map(|c| (&c.agreement, 1.0))
    }
    fn resolve(&self, mut id: u32) -> u32 {
        while let Some(next) = self.classes[id as usize].merged_into {
            id = next;
        }
        id
    }
    fn assign(&self, noun: NounSemantics, word: &crate::Lexeme) -> u32 {
        // Loans are integrated by their adapted ending, not the donor's class.
        if !matches!(word.origin, Origin::Borrowed { .. })
            && let Some(class) = self
                .classes
                .iter()
                .find(|c| c.core.matches(noun) && c.born == self.classes.last().unwrap().born)
        {
            return self.resolve(class.id);
        }
        if !matches!(word.origin, Origin::Borrowed { .. })
            && let Some(class) = self.classes.iter().find(|c| {
                c.core == Core::Remainder
                    && c.basis == Basis::Animacy
                    && c.born == self.classes.last().unwrap().born
            })
        {
            return self.resolve(class.id);
        }
        let active = || self.classes.iter().filter(|c| c.retired.is_none());
        if let Some(ending) = word.form.segs.last()
            && let Some(class) = active().find(|c| c.marker.segs.last() == Some(ending))
        {
            return class.id;
        }
        // Unmatched endings distribute stably among formal classes. A system
        // without formal alternatives uses its inanimate/default class.
        let formal = active().filter(|c| c.basis == Basis::Formal).count();
        if formal > 0 {
            let at = word.form.segs.last().map_or(0, |s| s.phone.0 as usize) % formal;
            return active()
                .filter(|c| c.basis == Basis::Formal)
                .nth(at)
                .unwrap()
                .id;
        }
        active()
            .find(|c| c.core == Core::Remainder)
            .unwrap_or_else(|| {
                let at =
                    word.form.segs.last().map_or(0, |s| s.phone.0 as usize) % self.active_count();
                active().nth(at).unwrap()
            })
            .id
    }
    pub fn sync(&mut self, lexicon: &Lexicon) {
        self.assignments.resize(lexicon.lexemes.len(), None);
        if self.active_count() < 2 {
            return;
        }
        self.nouns.resize(lexicon.lexemes.len(), None);
        self.nouns.fill(None);
        for slot in &lexicon.slots {
            if let Some(noun) = slot.concept.noun_semantics() {
                for variant in &slot.variants {
                    // The earliest current noun sense wins when a word is polysemous.
                    self.nouns[variant.lexeme.0 as usize].get_or_insert(noun);
                }
            }
        }
        for class in &mut self.classes {
            class.members = 0;
        }
        for word in lexicon.living() {
            let i = word.id.0 as usize;
            let Some(noun) = self.nouns[i] else {
                self.assignments[i] = None;
                continue;
            };
            let id =
                self.assignments[i].map_or_else(|| self.assign(noun, word), |id| self.resolve(id));
            self.assignments[i] = Some(id);
            self.classes[id as usize].members += 1;
        }
    }
    /// A rare Greenberg-style recruitment of ordinary classifiers as bound
    /// agreement. The lexical sources retain their own uses and sound histories.
    #[allow(clippy::too_many_arguments)]
    pub fn evolve(
        &mut self,
        seed: u64,
        variety: usize,
        generation: u32,
        rate: f32,
        profile: &SoundProfile,
        morphology: &Morphology,
        stress: StressRule,
        lexicon: &Lexicon,
    ) {
        if rate == 0.0 || self.active_count() > 0 || self.lost.is_some_and(|g| generation < g + 32)
        {
            return;
        }
        let mut rng = stream(
            seed,
            &[
                key("noun class emergence"),
                variety as u64,
                generation as u64,
            ],
        );
        if rng.r#gen::<f32>() >= rate {
            return;
        }
        let choice =
            [ClassChoice::Sex, ClassChoice::Animacy, ClassChoice::Many][index(&mut rng, 3)];
        let mut count_rng = stream(
            seed,
            &[
                key("noun class count emergence"),
                variety as u64,
                generation as u64,
            ],
        );
        let count = draw_count(&mut count_rng);
        let mut markers = Vec::new();
        for core in choice.cores(count) {
            let Some(source) = lexicon.word_for(by_id(core.source()).unwrap()) else {
                return;
            };
            let mut form = source.form.clone();
            // Classifiers shorten at recruitment to their first syllable; later
            // loss is exclusively regular sound change on the complete determiner.
            let Some(end) = (0..form.segs.len()).find(|&i| form.is_vowel(i)) else {
                return;
            };
            form.segs.truncate(end + 1);
            form.boundaries.clear();
            form.stress = None;
            if markers
                .iter()
                .any(|(old, _): &(Form, Option<LexemeId>)| old.segs == form.segs)
            {
                return;
            }
            markers.push((form, Some(source.id)));
        }
        let side = if rng.r#gen::<f32>() < profile.morphology.suffixing {
            Side::Suffix
        } else {
            Side::Prefix
        };
        self.establish(
            choice, markers, side, morphology, stress, lexicon, generation,
        );
    }
    pub fn apply_law(
        &mut self,
        law: &Law,
        minimal: MinimalWord,
        stress: StressRule,
        generation: u32,
        lexicon: &Lexicon,
    ) {
        let Some(side) = self.side else {
            return;
        };
        self.assignments.resize(lexicon.lexemes.len(), None);
        for class in self.classes.iter_mut().filter(|c| c.retired.is_none()) {
            let before = class.agreement.clone();
            for rule in &law.rules {
                let (next, edge) = rule.apply_with_edge(&class.agreement, stress, class.edge);
                if !minimal.blocks(&class.agreement, &next) {
                    class.agreement = next;
                    class.edge = edge;
                }
            }
            if law.changes(&before, &class.agreement, stress) {
                class.history.push(GrammarEntry {
                    generation,
                    event: GrammarEvent::SoundLaw {
                        law: law.id,
                        before,
                    },
                });
            }
            let edge = class.edge.min(class.agreement.segs.len());
            class.marker.segs.clear();
            class.marker.segs.extend_from_slice(match side {
                Side::Prefix => &class.agreement.segs[..edge],
                Side::Suffix => &class.agreement.segs[edge..],
            });
        }
        let cause = Some(SoundCause {
            law: law.id,
            generation,
        });
        // Complete surface identity, not merely empty endings: umlaut may keep
        // two classes distinct after their affixes disappear.
        for i in 0..self.classes.len() {
            if self.classes[i].retired.is_some() {
                continue;
            }
            if let Some(into) = (0..i).find(|&j| {
                self.classes[j].retired.is_none()
                    && same_sound(
                        &self.classes[i].agreement,
                        &self.classes[j].agreement,
                        law.stress.unwrap_or(stress),
                    )
            }) {
                self.classes[i].merged_into = Some(into as u32);
                self.classes[i].retired = Some(generation);
                self.classes[i].members = 0;
                self.events.push(ClassNotice {
                    generation,
                    change: ClassChange::Merged {
                        class: i as u32,
                        into: into as u32,
                    },
                    cause,
                });
            }
        }
        if self.active_count() == 1 {
            for class in self.classes.iter_mut().filter(|c| c.retired.is_none()) {
                class.retired = Some(generation);
                class.members = 0;
            }
            for word in lexicon.living() {
                self.assignments[word.id.0 as usize] = None;
            }
            self.lost = Some(generation);
            self.events.push(ClassNotice {
                generation,
                change: ClassChange::Lost,
                cause,
            });
        } else {
            self.sync(lexicon);
        }
    }
}

#[cfg(test)]
mod tests;

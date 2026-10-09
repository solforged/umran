use crate::change::{Env, Matcher, Rewrite, SoundChange};
use crate::form::Form;
use crate::inventory::preference;
use crate::phoneme::{Backness, Height, Manner, PhonemeId, Place, Secondary};
use crate::profile::InventoryPrior;
use crate::prosody::{MinimalWord, StressRule};
use std::borrow::Cow;
use std::collections::HashSet;

/// Without syllabic consonants or a cluster-repair stage, syncope must not
/// create the six/seven-consonant runs produced by wholesale vowel deletion.
/// This is a word-local modelling bound, not a universal phonotactic claim.
const MAX_SYNCOPATED_CLUSTER: usize = 3;

/// A named sound law: one or more rules applied in order, each regularly
/// across every living word.
#[derive(Clone, Debug, PartialEq)]
pub struct Law {
    pub id: &'static str,
    pub label: &'static str,
    pub rules: Vec<SoundChange>,
    /// Rough typological commonness, a hand estimate rather than a measured
    /// frequency. Lenition and palatalization are common; wholesale
    /// spirantization is rare.
    pub commonness: f32,
    /// A suprasegmental law, assessed by movement of word stress.
    pub stress: Option<StressRule>,
}

/// What a law would do to a lexicon if chosen now.
#[derive(Clone, Debug, PartialEq)]
pub struct Assessment {
    /// Changed forms, not independent lexical evidence or usage support.
    pub words: usize,
    /// Usage support of changed forms; splitting an alternative's share
    /// among several forms does not create additional support.
    pub word_weight: f32,
    /// Usage-weighted mean change in preference score per altered segment;
    /// positive when the law moves sounds toward what the culture prefers.
    /// Deletions count as neutral; forbidden segments score very low.
    pub pull: f32,
    /// Every altered segment, its outcome, and the form's usage weight.
    pub shifts: Vec<(PhonemeId, Option<PhonemeId>, f32)>,
}

impl Assessment {
    /// Usage-weighted mean movement toward the sound set `target`, from -1
    /// to 1: +1 when a segment outside it becomes one inside, -1 for the
    /// reverse. Deletions count as neutral.
    pub fn toward(&self, target: &HashSet<PhonemeId>) -> f32 {
        let (total, weight) =
            self.shifts
                .iter()
                .fold((0.0, 0.0), |(total, weight), &(old, new, contribution)| {
                    let movement = new.map_or(0.0, |new| {
                        f32::from(target.contains(&new)) - f32::from(target.contains(&old))
                    });
                    (total + contribution * movement, weight + contribution)
                });
        if weight > 0.0 { total / weight } else { 0.0 }
    }
}

/// A borrowed snapshot for one candidate selection. Iteration order and every
/// usage contribution are retained; only repeated traversal and segment taste
/// calculations are shared between laws. Dropping it ends the cache lifetime,
/// so lexical, grammatical, or profile mutations cannot leave stale scores.
pub(crate) struct Assessor<'a> {
    forms: Vec<(&'a Form, f32)>,
    prior: &'a InventoryPrior,
    preferences: Vec<Option<f32>>,
    minimal: MinimalWord,
    stress: StressRule,
}

impl<'a> Assessor<'a> {
    pub(crate) fn new(
        forms: impl Iterator<Item = (&'a Form, f32)>,
        prior: &'a InventoryPrior,
        minimal: MinimalWord,
        stress: StressRule,
    ) -> Self {
        Self {
            forms: forms.collect(),
            prior,
            preferences: vec![None; crate::CATALOG.segments.len()],
            minimal,
            stress,
        }
    }

    pub(crate) fn assess(&mut self, law: &Law) -> Option<Assessment> {
        law.assess_scored(
            self.forms.iter().copied(),
            |id| *self.preferences[id.0 as usize].get_or_insert_with(|| preference(self.prior, id)),
            self.minimal,
            self.stress,
        )
    }
}

impl Law {
    /// The law's rules in order, each passing by a word it would wear
    /// below the language's `minimal` size.
    pub fn apply(&self, form: &Form, minimal: MinimalWord, stress: StressRule) -> Form {
        let mut current = Cow::Borrowed(form);
        for rule in &self.rules {
            if let Cow::Owned(next) = rule.apply_borrowed(&current, stress)
                && !minimal.blocks(&current, &next)
            {
                current = Cow::Owned(next);
            }
        }
        current.into_owned()
    }
    pub fn changes(&self, form: &Form, after: &Form, stress: StressRule) -> bool {
        form != after
            || self
                .stress
                .is_some_and(|next| form.stressed_syllable(stress) != after.stressed_syllable(next))
    }

    /// `None` when the law would change nothing.
    pub fn assess<'a>(
        &self,
        forms: impl Iterator<Item = &'a Form>,
        prior: &InventoryPrior,
        minimal: MinimalWord,
        stress: StressRule,
    ) -> Option<Assessment> {
        self.assess_weighted(forms.map(|form| (form, 1.0)), prior, minimal, stress)
    }

    /// Assesses separate words with their usage contributions. Alternatives
    /// for one use should have weights summing to that use's weight; particles
    /// stay separate words and are supplied once, not concatenated with hosts.
    pub fn assess_weighted<'a>(
        &self,
        forms: impl Iterator<Item = (&'a Form, f32)>,
        prior: &InventoryPrior,
        minimal: MinimalWord,
        stress: StressRule,
    ) -> Option<Assessment> {
        self.assess_scored(forms, |id| preference(prior, id), minimal, stress)
    }

    fn assess_scored<'a>(
        &self,
        forms: impl Iterator<Item = (&'a Form, f32)>,
        mut preference: impl FnMut(PhonemeId) -> f32,
        minimal: MinimalWord,
        stress: StressRule,
    ) -> Option<Assessment> {
        if self.stress == Some(stress) {
            return None;
        }
        if let Some(next) = self.stress {
            let (words, word_weight) = forms
                .filter(|(form, weight)| {
                    weight.is_finite()
                        && *weight > 0.0
                        && form.stressed_syllable(stress) != form.stressed_syllable(next)
                })
                .fold((0, 0.0), |(words, weight), (_, contribution)| {
                    (words + 1, weight + contribution)
                });
            return (words > 0).then_some(Assessment {
                words,
                word_weight,
                pull: 0.0,
                shifts: Vec::new(),
            });
        }
        let (mut words, mut word_weight) = (0, 0.0);
        let (mut total, mut total_weight) = (0.0, 0.0);
        let mut shifts = Vec::new();
        for (form, weight) in forms {
            if !weight.is_finite() || weight <= 0.0 {
                continue;
            }
            // Keep the accumulation order and apply each rule only once.
            // Roll back this word's assessment if the full chain cancels out.
            let before_total = total;
            let before_weight = total_weight;
            let before_shifts = shifts.len();
            let mut current = Cow::Borrowed(form);
            for rule in &self.rules {
                // No-hit forms stay borrowed. Accepted rewrites share the
                // actual protected survivor mapping with normal application.
                let Some(outcome) = rule.outcome(&current, stress) else {
                    continue;
                };
                let next = SoundChange::from_outcome(&current, &outcome, 0).0;
                if minimal.blocks(&current, &next) {
                    continue;
                }
                for (i, &out) in outcome.iter().enumerate() {
                    let old = current.segs[i];
                    if out == Some(old) {
                        continue;
                    }
                    let phone = match rule.result {
                        // Assimilation adopts the following surviving
                        // consonant's sound as well as its length.
                        Rewrite::GeminateNext if out.is_none() => {
                            outcome.get(i + 1).and_then(|s| s.map(|s| s.phone))
                        }
                        _ => out.map(|s| s.phone),
                    };
                    if let Some(new) = phone {
                        total += weight * (preference(new) - preference(old.phone));
                    }
                    total_weight += weight;
                    shifts.push((old.phone, phone, weight));
                }
                current = Cow::Owned(next);
            }
            if self.changes(form, &current, stress) {
                words += 1;
                word_weight += weight;
            } else {
                total = before_total;
                total_weight = before_weight;
                shifts.truncate(before_shifts);
            }
        }
        // Register must replace an actual voiced onset. Voiceless-only
        // words take high pitch as part of that change, not as its cause.
        if self
            .rules
            .iter()
            .any(|rule| matches!(rule.result, Rewrite::Tone(crate::tone::Change::Register)))
            && !shifts.iter().any(|&(old, new, _)| {
                old != new.unwrap_or(old)
                    && crate::CATALOG
                        .get(old)
                        .consonant()
                        .is_some_and(|c| c.voiced)
                    && new.is_some_and(|p| {
                        crate::CATALOG.get(p).consonant().is_some_and(|c| !c.voiced)
                    })
            })
        {
            return None;
        }
        (words > 0).then(|| Assessment {
            words,
            word_weight,
            pull: if total_weight > 0.0 {
                total / total_weight
            } else {
                0.0
            },
            shifts,
        })
    }
}

fn cons(place: Option<Place>, manner: Option<Manner>, voiced: Option<bool>) -> Matcher {
    Matcher::Consonant {
        place,
        manner,
        voiced,
        secondary: None,
    }
}

fn vowel(height: Option<Height>, backness: Option<Backness>, rounded: Option<bool>) -> Matcher {
    Matcher::Vowel {
        height,
        backness,
        rounded,
    }
}

fn into_cons(place: Option<Place>, manner: Option<Manner>, voiced: Option<bool>) -> Rewrite {
    Rewrite::Consonant {
        place,
        manner,
        voiced,
        secondary: None,
    }
}

fn with_secondary(m: Matcher, s: Secondary) -> Matcher {
    match m {
        Matcher::Consonant {
            place,
            manner,
            voiced,
            ..
        } => Matcher::Consonant {
            place,
            manner,
            voiced,
            secondary: Some(s),
        },
        other => other,
    }
}

fn into_secondary(r: Rewrite, s: Secondary) -> Rewrite {
    match r {
        Rewrite::Consonant {
            place,
            manner,
            voiced,
            ..
        } => Rewrite::Consonant {
            place,
            manner,
            voiced,
            secondary: Some(s),
        },
        other => other,
    }
}

fn rule(target: Matcher, result: Rewrite, left: Env, right: Env) -> SoundChange {
    SoundChange {
        id: String::new(),
        target,
        result,
        left,
        right,
    }
}

fn law(id: &'static str, label: &'static str, commonness: f32, rules: Vec<SoundChange>) -> Law {
    let rules = rules
        .into_iter()
        .enumerate()
        .map(|(i, r)| SoundChange {
            id: format!("{id}.{i}"),
            ..r
        })
        .collect();
    Law {
        id,
        label,
        rules,
        commonness,
        stress: None,
    }
}

fn v() -> Env {
    Env::Matcher(Matcher::AnyVowel)
}

fn c() -> Env {
    Env::Matcher(Matcher::AnyConsonant)
}

fn at(m: Matcher) -> Env {
    Env::Matcher(m)
}
fn length(target: Matcher, long: bool) -> Matcher {
    Matcher::Length {
        target: Box::new(target),
        long,
    }
}

fn stressed(target: Matcher, stressed: bool) -> Matcher {
    Matcher::Stressed {
        target: Box::new(target),
        stressed,
    }
}

fn phone(ipa: &str) -> Matcher {
    Matcher::Phone(crate::CATALOG.id_by_ipa(ipa).unwrap())
}

fn stress_law(id: &'static str, label: &'static str, stress: StressRule) -> Law {
    Law {
        stress: Some(stress),
        ..law(id, label, 0.35, Vec::new())
    }
}

const ANY: Env = Env::Any;
const EDGE: Env = Env::WordEdge;

/// The sound laws a variety can undergo. Each is applied at most once.
pub fn catalog() -> Vec<Law> {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    let close_front = || vowel(Some(Close), Some(Front), Some(false));
    let mid_front = || vowel(Some(CloseMid), Some(Front), Some(false));
    let mut laws = vec![
        law(
            "intervocalic-voicing",
            "Voiceless stops voice between vowels",
            1.0,
            vec![rule(
                length(cons(None, Some(Stop), Some(false)), false),
                into_cons(None, None, Some(true)),
                v(),
                v(),
            )],
        ),
        law(
            "intervocalic-spirantization",
            "Voiced stops become fricatives between vowels: b > v, d > ð, g > ɣ",
            0.8,
            vec![
                rule(
                    cons(Some(Bilabial), Some(Stop), Some(true)),
                    into_cons(Some(Labiodental), Some(Fricative), None),
                    v(),
                    v(),
                ),
                rule(
                    cons(Some(Alveolar), Some(Stop), Some(true)),
                    into_cons(Some(Dental), Some(Fricative), None),
                    v(),
                    v(),
                ),
                rule(
                    cons(Some(Velar), Some(Stop), Some(true)),
                    into_cons(None, Some(Fricative), None),
                    v(),
                    v(),
                ),
            ],
        ),
        law(
            "final-devoicing",
            "Voiced obstruents devoice at the end of a word",
            0.9,
            vec![
                rule(
                    cons(None, Some(Stop), Some(true)),
                    into_cons(None, None, Some(false)),
                    ANY,
                    EDGE,
                ),
                rule(
                    cons(None, Some(Fricative), Some(true)),
                    into_cons(None, None, Some(false)),
                    ANY,
                    EDGE,
                ),
                rule(
                    cons(None, Some(Affricate), Some(true)),
                    into_cons(None, None, Some(false)),
                    ANY,
                    EDGE,
                ),
            ],
        ),
        law(
            "velar-palatalization",
            "Velar stops become affricates before front vowels: k > tʃ, g > dʒ",
            1.0,
            vec![
                rule(
                    cons(Some(Velar), Some(Stop), None),
                    into_cons(Some(Postalveolar), Some(Affricate), None),
                    ANY,
                    at(close_front()),
                ),
                rule(
                    cons(Some(Velar), Some(Stop), None),
                    into_cons(Some(Postalveolar), Some(Affricate), None),
                    ANY,
                    at(mid_front()),
                ),
            ],
        ),
        law(
            "assibilation",
            "t becomes s before i",
            0.6,
            vec![rule(
                cons(Some(Alveolar), Some(Stop), Some(false)),
                into_cons(None, Some(Fricative), None),
                ANY,
                at(close_front()),
            )],
        ),
        law(
            "s-debuccalization",
            "s becomes h at the start of a word and between vowels",
            0.5,
            vec![
                rule(
                    cons(Some(Alveolar), Some(Fricative), Some(false)),
                    into_cons(Some(Glottal), None, None),
                    EDGE,
                    ANY,
                ),
                rule(
                    cons(Some(Alveolar), Some(Fricative), Some(false)),
                    into_cons(Some(Glottal), None, None),
                    v(),
                    v(),
                ),
            ],
        ),
        law(
            "h-loss",
            "h disappears",
            0.9,
            vec![rule(
                cons(Some(Glottal), Some(Fricative), None),
                Rewrite::Delete,
                ANY,
                ANY,
            )],
        ),
        law(
            "nasal-assimilation",
            "n takes the place of a following consonant: np > mp, nk > ŋk",
            0.9,
            vec![
                rule(
                    cons(Some(Alveolar), Some(Nasal), None),
                    into_cons(Some(Bilabial), None, None),
                    ANY,
                    at(cons(Some(Bilabial), None, None)),
                ),
                rule(
                    cons(Some(Alveolar), Some(Nasal), None),
                    into_cons(Some(Velar), None, None),
                    ANY,
                    at(cons(Some(Velar), None, None)),
                ),
            ],
        ),
        law(
            "apocope",
            "Final vowels after a consonant are lost",
            0.7,
            vec![rule(Matcher::AnyVowel, Rewrite::Delete, c(), EDGE)],
        ),
        law(
            "final-stop-loss",
            "Word-final stops are lost",
            0.5,
            vec![rule(
                cons(None, Some(Stop), None),
                Rewrite::Delete,
                ANY,
                EDGE,
            )],
        ),
        law(
            "final-raising",
            "Final mid vowels rise: e > i, o > u",
            0.6,
            vec![rule(
                vowel(Some(CloseMid), None, None),
                Rewrite::Vowel {
                    height: Some(Close),
                    backness: None,
                    rounded: None,
                },
                ANY,
                EDGE,
            )],
        ),
        law(
            "rhotacism",
            "s becomes r between vowels",
            0.4,
            vec![rule(
                cons(Some(Alveolar), Some(Fricative), None),
                into_cons(None, Some(Trill), Some(true)),
                v(),
                v(),
            )],
        ),
        law(
            "w-fortition",
            "w becomes v",
            0.6,
            vec![rule(
                cons(Some(Bilabial), Some(Approximant), None),
                into_cons(Some(Labiodental), Some(Fricative), Some(true)),
                ANY,
                ANY,
            )],
        ),
        law(
            "th-fronting",
            "θ and ð become f and v",
            0.4,
            vec![rule(
                cons(Some(Dental), Some(Fricative), None),
                into_cons(Some(Labiodental), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "th-stopping",
            "θ and ð become t and d",
            0.5,
            vec![rule(
                cons(Some(Dental), Some(Fricative), None),
                into_cons(Some(Alveolar), Some(Stop), None),
                ANY,
                ANY,
            )],
        ),
        law(
            "x-debuccalization",
            "x becomes h",
            0.7,
            vec![rule(
                cons(Some(Velar), Some(Fricative), Some(false)),
                into_cons(Some(Glottal), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "spirantization",
            "Voiceless stops become fricatives: p > f, t > θ, k > x",
            0.3,
            vec![
                rule(
                    cons(Some(Bilabial), Some(Stop), Some(false)),
                    into_cons(Some(Labiodental), Some(Fricative), None),
                    ANY,
                    ANY,
                ),
                rule(
                    cons(Some(Alveolar), Some(Stop), Some(false)),
                    into_cons(Some(Dental), Some(Fricative), None),
                    ANY,
                    ANY,
                ),
                rule(
                    cons(Some(Velar), Some(Stop), Some(false)),
                    into_cons(None, Some(Fricative), None),
                    ANY,
                    ANY,
                ),
            ],
        ),
        law(
            "stop-devoicing",
            "Voiced stops devoice everywhere",
            0.3,
            vec![rule(
                cons(None, Some(Stop), Some(true)),
                into_cons(None, None, Some(false)),
                ANY,
                ANY,
            )],
        ),
        law(
            "velar-nasal-fronting",
            "ŋ becomes n",
            0.4,
            vec![rule(
                cons(Some(Velar), Some(Nasal), None),
                into_cons(Some(Alveolar), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "rhotic-merger",
            "The tap merges with the trill",
            0.5,
            vec![rule(
                cons(Some(Alveolar), Some(Tap), None),
                into_cons(None, Some(Trill), None),
                ANY,
                ANY,
            )],
        ),
        law(
            "l-vocalization",
            "l becomes w before a consonant or at the end of a word",
            0.4,
            vec![
                rule(
                    cons(Some(Alveolar), Some(Lateral), None),
                    into_cons(Some(Bilabial), Some(Approximant), None),
                    ANY,
                    c(),
                ),
                rule(
                    cons(Some(Alveolar), Some(Lateral), None),
                    into_cons(Some(Bilabial), Some(Approximant), None),
                    ANY,
                    EDGE,
                ),
            ],
        ),
        law(
            "initial-glide-loss",
            "j and w are lost at the start of a word",
            0.4,
            vec![
                rule(
                    cons(Some(Palatal), Some(Approximant), None),
                    Rewrite::Delete,
                    EDGE,
                    ANY,
                ),
                rule(
                    cons(Some(Bilabial), Some(Approximant), None),
                    Rewrite::Delete,
                    EDGE,
                    ANY,
                ),
            ],
        ),
        law(
            "final-nasal-merger",
            "Final m becomes n",
            0.5,
            vec![rule(
                cons(Some(Bilabial), Some(Nasal), None),
                into_cons(Some(Alveolar), None, None),
                ANY,
                EDGE,
            )],
        ),
        law(
            "u-fronting",
            "u becomes y",
            0.2,
            vec![rule(
                vowel(Some(Close), Some(Back), Some(true)),
                Rewrite::Vowel {
                    height: None,
                    backness: Some(Front),
                    rounded: None,
                },
                ANY,
                ANY,
            )],
        ),
        law(
            "pre-nasal-raising",
            "e becomes i before a nasal",
            0.4,
            vec![rule(
                mid_front(),
                Rewrite::Vowel {
                    height: Some(Close),
                    backness: None,
                    rounded: None,
                },
                ANY,
                at(cons(None, Some(Nasal), None)),
            )],
        ),
        law(
            "flapping",
            "t and d become a tap between vowels",
            0.6,
            vec![rule(
                cons(Some(Alveolar), Some(Stop), None),
                into_cons(None, Some(Tap), Some(true)),
                v(),
                v(),
            )],
        ),
        law(
            "s-voicing",
            "s becomes z between vowels",
            0.5,
            vec![rule(
                cons(Some(Alveolar), Some(Fricative), Some(false)),
                into_cons(None, None, Some(true)),
                v(),
                v(),
            )],
        ),
        law(
            "deaspiration",
            "Aspirated stops lose their aspiration: pʰ > p",
            0.6,
            vec![rule(
                with_secondary(cons(None, None, None), Secondary::Aspirated),
                into_secondary(into_cons(None, None, None), Secondary::Plain),
                ANY,
                ANY,
            )],
        ),
        law(
            "aspirate-spirantization",
            "Aspirated stops become fricatives: pʰ > f, tʰ > θ, kʰ > x",
            0.3,
            vec![
                rule(
                    with_secondary(
                        cons(Some(Bilabial), Some(Stop), Some(false)),
                        Secondary::Aspirated,
                    ),
                    into_secondary(
                        into_cons(Some(Labiodental), Some(Fricative), None),
                        Secondary::Plain,
                    ),
                    ANY,
                    ANY,
                ),
                rule(
                    with_secondary(
                        cons(Some(Alveolar), Some(Stop), Some(false)),
                        Secondary::Aspirated,
                    ),
                    into_secondary(
                        into_cons(Some(Dental), Some(Fricative), None),
                        Secondary::Plain,
                    ),
                    ANY,
                    ANY,
                ),
                rule(
                    with_secondary(
                        cons(Some(Velar), Some(Stop), Some(false)),
                        Secondary::Aspirated,
                    ),
                    into_secondary(into_cons(None, Some(Fricative), None), Secondary::Plain),
                    ANY,
                    ANY,
                ),
            ],
        ),
        law(
            "breathy-loss",
            "Breathy-voiced stops become plain voiced: bʱ > b",
            0.6,
            vec![rule(
                with_secondary(cons(None, None, None), Secondary::Breathy),
                into_secondary(into_cons(None, None, None), Secondary::Plain),
                ANY,
                ANY,
            )],
        ),
        law(
            "labiovelar-to-labial",
            "Labiovelars become labials: kʷ > p, gʷ > b",
            0.3,
            vec![rule(
                with_secondary(cons(Some(Velar), Some(Stop), None), Secondary::Labialized),
                into_secondary(into_cons(Some(Bilabial), None, None), Secondary::Plain),
                ANY,
                ANY,
            )],
        ),
        law(
            "delabialization",
            "Labialized consonants lose their rounding: kʷ > k",
            0.5,
            vec![rule(
                with_secondary(cons(None, None, None), Secondary::Labialized),
                into_secondary(into_cons(None, None, None), Secondary::Plain),
                ANY,
                ANY,
            )],
        ),
        law(
            "retroflex-merger",
            "Retroflex consonants merge with alveolars: ʈ > t",
            0.4,
            vec![rule(
                cons(Some(Retroflex), None, None),
                into_cons(Some(Alveolar), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "pharyngeal-weakening",
            "Pharyngeals weaken to glottals: ħ > h, ʕ > ʔ",
            0.5,
            vec![
                rule(
                    cons(Some(Pharyngeal), None, Some(false)),
                    into_cons(Some(Glottal), None, None),
                    ANY,
                    ANY,
                ),
                rule(
                    cons(Some(Pharyngeal), None, Some(true)),
                    into_cons(Some(Glottal), Some(Stop), Some(false)),
                    ANY,
                    ANY,
                ),
            ],
        ),
        law(
            "uvular-fronting",
            "Uvulars move forward to velars: q > k, χ > x",
            0.5,
            vec![rule(
                cons(Some(Uvular), None, None),
                into_cons(Some(Velar), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "lateral-affricate-loss",
            "The lateral affricate simplifies: tɬ > t",
            0.3,
            vec![rule(
                cons(None, Some(LateralAffricate), None),
                into_cons(None, Some(Stop), None),
                ANY,
                ANY,
            )],
        ),
        law(
            "unstressed-reduction",
            "Unstressed vowels reduce to schwa",
            0.7,
            vec![rule(
                stressed(Matcher::AnyVowel, false),
                Rewrite::Phone(crate::CATALOG.id_by_ipa("ə").unwrap()),
                ANY,
                ANY,
            )],
        ),
        law(
            "unstressed-syncope",
            "Every other unstressed vowel between consonants drops out",
            0.45,
            vec![rule(
                Matcher::RhythmicVowel {
                    max_cluster: MAX_SYNCOPATED_CLUSTER,
                },
                Rewrite::Delete,
                at(length(Matcher::AnyConsonant, false)),
                at(length(Matcher::AnyConsonant, false)),
            )],
        ),
        law(
            "unstressed-apocope",
            "Unstressed final vowels are lost",
            0.45,
            vec![rule(
                stressed(Matcher::AnyVowel, false),
                Rewrite::Delete,
                c(),
                EDGE,
            )],
        ),
        law(
            "stressed-open-lengthening",
            "Stressed vowels lengthen in open syllables",
            0.6,
            vec![rule(
                stressed(Matcher::OpenSyllable(Box::new(Matcher::AnyVowel)), true),
                Rewrite::Length(true),
                ANY,
                ANY,
            )],
        ),
        law(
            "verner-voicing",
            "Fricatives voice after an unstressed vowel",
            0.45,
            vec![rule(
                length(cons(None, Some(Fricative), Some(false)), false),
                into_cons(None, None, Some(true)),
                at(stressed(Matcher::AnyVowel, false)),
                ANY,
            )],
        ),
        law(
            "cluster-gemination",
            "Clusters assimilate to geminates: kt, pt > tt; ks > ss",
            0.7,
            vec![
                rule(phone("k"), Rewrite::GeminateNext, ANY, at(phone("t"))),
                rule(phone("p"), Rewrite::GeminateNext, ANY, at(phone("t"))),
                rule(phone("k"), Rewrite::GeminateNext, ANY, at(phone("s"))),
            ],
        ),
        law(
            "j-gemination",
            "Consonants lengthen before j",
            0.6,
            vec![rule(
                length(Matcher::AnyConsonant, false),
                Rewrite::Length(true),
                v(),
                at(phone("j")),
            )],
        ),
        law(
            "degemination",
            "Long consonants shorten",
            0.6,
            vec![rule(
                length(Matcher::AnyConsonant, true),
                Rewrite::Length(false),
                ANY,
                ANY,
            )],
        ),
        law(
            "romance-lenition",
            "Single stops voice between vowels, then long consonants shorten",
            0.65,
            vec![
                rule(
                    length(cons(None, Some(Stop), Some(false)), false),
                    into_cons(None, None, Some(true)),
                    v(),
                    v(),
                ),
                rule(
                    length(Matcher::AnyConsonant, true),
                    Rewrite::Length(false),
                    ANY,
                    ANY,
                ),
            ],
        ),
        law(
            "compensatory-lengthening",
            "Preconsonantal nasals disappear and lengthen the preceding vowel",
            0.45,
            vec![rule(
                cons(None, Some(Nasal), None),
                Rewrite::Compensate,
                v(),
                c(),
            )],
        ),
        stress_law(
            "initial-stress",
            "Stress moves to the first syllable",
            StressRule::Initial,
        ),
        stress_law(
            "penult-stress",
            "Stress moves to the penult",
            StressRule::Penult,
        ),
        law(
            "koine-levelling",
            "Minority sounds merge into majority sounds",
            0.0,
            // Its exact rules depend on the city's makeup, not a portable
            // catalog change. The resulting variety's later laws can spread.
            Vec::new(),
        ),
        law(
            "umlaut",
            "Vowels front before a front vowel in the next syllable: a > e, o > ø, u > y",
            0.4,
            vec![
                rule(
                    Matcher::Phone(crate::CATALOG.id_by_ipa("a").unwrap()),
                    Rewrite::Phone(crate::CATALOG.id_by_ipa("e").unwrap()),
                    ANY,
                    Env::FollowingSyllableVowel(vowel(None, Some(Front), None)),
                ),
                rule(
                    vowel(None, Some(Back), Some(true)),
                    Rewrite::Vowel {
                        height: None,
                        backness: Some(Front),
                        rounded: None,
                    },
                    ANY,
                    Env::FollowingSyllableVowel(vowel(None, Some(Front), None)),
                ),
            ],
        ),
        law(
            "coda-tonogenesis",
            "Final laryngeals leave lexical tone",
            1.0,
            vec![
                rule(
                    phone("ʔ"),
                    Rewrite::Tone(crate::tone::Change::Coda(crate::tone::Tone::Rising)),
                    v(),
                    EDGE,
                ),
                rule(
                    phone("h"),
                    Rewrite::Tone(crate::tone::Change::Coda(crate::tone::Tone::Falling)),
                    v(),
                    EDGE,
                ),
                rule(
                    phone("s"),
                    Rewrite::Tone(crate::tone::Change::Coda(crate::tone::Tone::Falling)),
                    v(),
                    EDGE,
                ),
            ],
        ),
        law(
            "register-tonogenesis",
            "Onset voicing becomes a pitch register",
            0.65,
            vec![rule(
                Matcher::AnyConsonant,
                Rewrite::Tone(crate::tone::Change::Register),
                ANY,
                v(),
            )],
        ),
        law(
            "tone-merger",
            "Contour tones merge into level tones",
            0.25,
            vec![rule(
                Matcher::AnyVowel,
                Rewrite::Tone(crate::tone::Change::Merge),
                ANY,
                ANY,
            )],
        ),
        law(
            "tone-loss",
            "Lexical tone is lost",
            0.4,
            vec![rule(
                Matcher::AnyVowel,
                Rewrite::Tone(crate::tone::Change::Loss),
                ANY,
                ANY,
            )],
        ),
    ];
    laws.extend(crate::harmony::catalog_laws());
    laws
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::SoundProfile;
    use std::collections::HashSet;

    fn run(id: &str, ipa: &str) -> String {
        let law = catalog().into_iter().find(|l| l.id == id).unwrap();
        law.apply(
            &Form::from_ipa(ipa).unwrap(),
            MinimalWord::Syllable,
            StressRule::Initial,
        )
        .ipa()
    }

    #[test]
    fn ids_are_unique() {
        let ids: HashSet<_> = catalog().iter().map(|l| l.id).collect();
        assert_eq!(ids.len(), catalog().len());
    }

    #[test]
    fn shared_assessment_is_bit_identical_to_individual_candidates() {
        let laws = catalog();
        for preset in [
            "familiar",
            "polynesian",
            "iranian",
            "semitic",
            "bantu",
            "finnic",
        ] {
            let mut world = crate::World::solo(
                7,
                &SoundProfile::by_id(preset).unwrap(),
                crate::Params::static_society(),
            );
            for _ in 0..2 {
                for variety in &world.varieties {
                    let mut shared = Assessor::new(
                        variety.spoken_forms(),
                        &variety.profile.inventory,
                        variety.minimal,
                        variety.stress(),
                    );
                    for law in &laws {
                        assert_eq!(
                            shared.assess(law),
                            law.assess_weighted(
                                variety.spoken_forms(),
                                &variety.profile.inventory,
                                variety.minimal,
                                variety.stress(),
                            ),
                            "{preset}: {}",
                            law.id
                        );
                    }
                }
                world.run(12);
            }
        }
    }

    #[test]
    fn laws_do_what_their_labels_say() {
        assert_eq!(run("intervocalic-voicing", "pata"), "pada");
        assert_eq!(run("intervocalic-spirantization", "abadaga"), "avaðaɣa");
        assert_eq!(run("final-devoicing", "bad"), "bat");
        assert_eq!(run("velar-palatalization", "kika"), "tʃika");
        assert_eq!(run("s-debuccalization", "sasa"), "haha");
        assert_eq!(run("nasal-assimilation", "anpanka"), "ampaŋka");
        assert_eq!(run("apocope", "kata"), "kat");
        assert_eq!(run("final-raising", "kote"), "koti");
        assert_eq!(run("spirantization", "pitak"), "fiθax");
        assert_eq!(run("l-vocalization", "kaltal"), "kawtaw");
        assert_eq!(run("flapping", "pata"), "paɾa");
        assert_eq!(run("deaspiration", "pʰatʰa"), "pata");
        assert_eq!(run("aspirate-spirantization", "kʰapʰa"), "xafa");
        assert_eq!(run("breathy-loss", "bʱadʱa"), "bada");
        assert_eq!(run("labiovelar-to-labial", "kʷigʷa"), "piba");
        assert_eq!(run("delabialization", "kʷaxʷa"), "kaxa");
        assert_eq!(run("retroflex-merger", "ʈaɳa"), "tana");
        assert_eq!(run("pharyngeal-weakening", "ħaʕa"), "haʔa");
        assert_eq!(run("uvular-fronting", "qaχa"), "kaxa");
        assert_eq!(run("lateral-affricate-loss", "tɬatɬ"), "tat");
    }
    #[test]
    fn umlaut_crosses_a_bound_ending_but_not_a_separate_particle() {
        let laws = catalog();
        let umlaut = laws.iter().find(|law| law.id == "umlaut").unwrap();
        let apocope = laws.iter().find(|law| law.id == "apocope").unwrap();
        let base = Form::from_ipa("fot").unwrap();
        let particle = Form::from_ipa("mi").unwrap();
        assert_eq!(
            umlaut.apply(&base, MinimalWord::Syllable, StressRule::Initial),
            base,
        );
        assert_eq!(
            umlaut.apply(&particle, MinimalWord::Syllable, StressRule::Initial),
            particle,
        );
        let prior = SoundProfile::by_id("polynesian").unwrap().inventory;
        assert_eq!(
            umlaut.assess_weighted(
                [(&base, 1.0), (&particle, 1.0)].into_iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Initial,
            ),
            None,
        );

        let mut bound = Form::from_ipa("foti").unwrap();
        bound.boundaries = vec![3];
        let marked = umlaut.apply(&bound, MinimalWord::Syllable, StressRule::Initial);
        assert_eq!(marked.ipa(), "føti");
        assert_eq!(marked.boundaries, vec![3]);
        assert_eq!(
            apocope
                .apply(&marked, MinimalWord::Syllable, StressRule::Initial)
                .ipa(),
            "føt",
        );
        assert_eq!(run("umlaut", "anti"), "enti");
        assert_eq!(run("umlaut", "unti"), "ynti");
        // Fronting in the second syllable must not feed the first one.
        assert_eq!(run("umlaut", "uomi"), "uømi");
        assert_eq!(run("umlaut", "aomi"), "aømi");
    }

    #[test]
    fn weighted_alternatives_contribute_their_usage_not_their_count() {
        let phone = |ipa| crate::CATALOG.id_by_ipa(ipa).unwrap();
        let law = law(
            "weighted-correspondences",
            "Two opposing sound-set movements",
            1.0,
            vec![
                rule(
                    Matcher::Phone(phone("p")),
                    Rewrite::Phone(phone("s")),
                    ANY,
                    ANY,
                ),
                rule(
                    Matcher::Phone(phone("k")),
                    Rewrite::Phone(phone("t")),
                    ANY,
                    ANY,
                ),
            ],
        );
        let prior = SoundProfile::by_id("polynesian").unwrap().inventory;
        let original = Form::from_ipa("pa").unwrap();
        let competitor_a = Form::from_ipa("pam").unwrap();
        let competitor_b = Form::from_ipa("pan").unwrap();
        let other = Form::from_ipa("ka").unwrap();
        let assess = |forms| {
            law.assess_weighted(forms, &prior, MinimalWord::Syllable, StressRule::Initial)
                .unwrap()
        };
        let unsplit = assess([(&original, 0.25), (&other, 0.75)].into_iter());
        let split = law
            .assess_weighted(
                [
                    (&competitor_a, 0.125),
                    (&competitor_b, 0.125),
                    (&other, 0.75),
                ]
                .into_iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Initial,
            )
            .unwrap();
        let target = HashSet::from([phone("s"), phone("k")]);
        assert!((unsplit.toward(&target) + 0.5).abs() < 1e-6);
        assert!((split.toward(&target) + 0.5).abs() < 1e-6);
        let expected_pull = 0.25
            * (preference(&prior, phone("s")) - preference(&prior, phone("p")))
            + 0.75 * (preference(&prior, phone("t")) - preference(&prior, phone("k")));
        assert!((unsplit.pull - expected_pull).abs() < 1e-6);
        assert!((split.pull - expected_pull).abs() < 1e-6);
        assert_eq!(unsplit.word_weight, 1.0);
        assert_eq!(split.word_weight, 1.0);

        let minority = assess([(&competitor_a, 0.125), (&competitor_b, 0.125)].into_iter());
        assert_eq!(minority.toward(&target), 1.0);
        assert_eq!(minority.word_weight, 0.25);
    }

    #[test]
    fn weighted_assessment_uses_protected_last_vowel_and_size_outcomes() {
        let law = law(
            "vowel-loss",
            "Vowels disappear",
            1.0,
            vec![rule(Matcher::AnyVowel, Rewrite::Delete, ANY, ANY)],
        );
        let prior = SoundProfile::by_id("polynesian").unwrap().inventory;
        let form = Form::from_ipa("kata").unwrap();
        let survivor = Form::from_ipa("ta").unwrap();
        let assessment = law
            .assess_weighted(
                [(&form, 0.25), (&survivor, 0.75)].into_iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Initial,
            )
            .unwrap();
        assert_eq!(
            assessment.shifts,
            vec![(crate::CATALOG.id_by_ipa("a").unwrap(), None, 0.25)],
        );
        assert_eq!(assessment.pull, 0.0);
        assert_eq!(
            law.apply(&form, MinimalWord::Syllable, StressRule::Initial)
                .ipa(),
            "kta",
        );
        assert_eq!(
            law.assess_weighted(
                [(&form, 0.25)].into_iter(),
                &prior,
                MinimalWord::TwoSyllables,
                StressRule::Initial,
            ),
            None,
        );
    }

    #[test]
    fn cancelling_rule_chains_do_not_add_usage_or_movement() {
        let p = crate::CATALOG.id_by_ipa("p").unwrap();
        let s = crate::CATALOG.id_by_ipa("s").unwrap();
        let a = crate::CATALOG.id_by_ipa("a").unwrap();
        let law = law(
            "conditional-cancellation",
            "Fronting is reversed before a",
            1.0,
            vec![
                rule(Matcher::Phone(p), Rewrite::Phone(s), ANY, ANY),
                rule(
                    Matcher::Phone(s),
                    Rewrite::Phone(p),
                    ANY,
                    at(Matcher::Phone(a)),
                ),
            ],
        );
        let prior = SoundProfile::by_id("polynesian").unwrap().inventory;
        let changed = Form::from_ipa("pi").unwrap();
        let cancelled = Form::from_ipa("pa").unwrap();
        let assessment = law
            .assess_weighted(
                [(&changed, 0.25), (&cancelled, 0.75)].into_iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Initial,
            )
            .unwrap();
        assert_eq!(assessment.words, 1);
        assert_eq!(assessment.word_weight, 0.25);
        assert_eq!(assessment.shifts, vec![(p, Some(s), 0.25)]);
        assert!((assessment.pull - (preference(&prior, s) - preference(&prior, p))).abs() < 1e-6);
        assert_eq!(assessment.toward(&HashSet::from([s])), 1.0);
    }

    #[test]
    fn weighted_stress_assessment_counts_only_changed_usage() {
        let law = catalog()
            .into_iter()
            .find(|law| law.id == "initial-stress")
            .unwrap();
        let prior = SoundProfile::by_id("polynesian").unwrap().inventory;
        let short = Form::from_ipa("ka").unwrap();
        let mut first = Form::from_ipa("kata").unwrap();
        first.stress = Some(0);
        let mut last = Form::from_ipa("katama").unwrap();
        last.stress = Some(2);
        let forms = [(&short, 0.5), (&first, 0.25), (&last, 0.125)];
        let free = law
            .assess_weighted(
                forms.into_iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Free,
            )
            .unwrap();
        assert_eq!(free.words, 1);
        assert_eq!(free.word_weight, 0.125);
        assert_eq!(free.pull, 0.0);
        assert!(free.shifts.is_empty());

        // Predictable stress ignores the lexical accent of the first form.
        let predictable = law
            .assess_weighted(
                forms.into_iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Final,
            )
            .unwrap();
        assert_eq!(predictable.words, 2);
        assert_eq!(predictable.word_weight, 0.375);
    }

    #[test]
    fn assessment_ignores_words_unchanged_by_the_complete_chain() {
        let id = |ipa| crate::phoneme::CATALOG.id_by_ipa(ipa).unwrap();
        let change = |from, to| {
            rule(
                Matcher::Phone(id(from)),
                Rewrite::Phone(id(to)),
                Env::Any,
                Env::Any,
            )
        };
        let law = law(
            "round-trip",
            "",
            1.0,
            vec![change("t", "s"), change("s", "t"), change("p", "b")],
        );
        let prior = SoundProfile::by_id("polynesian").unwrap().inventory;
        let forms = ["ta", "pa", "ta"].map(|ipa| Form::from_ipa(ipa).unwrap());
        let assessment = law
            .assess(
                forms.iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Initial,
            )
            .unwrap();
        assert_eq!(assessment.words, 1);
        assert_eq!(assessment.shifts, vec![(id("p"), Some(id("b")), 1.0)]);
        assert_eq!(
            assessment.pull,
            preference(&prior, id("b")) - preference(&prior, id("p"))
        );
    }

    #[test]
    fn preferences_pull_toward_preferred_sounds() {
        // Kuo-toa dislikes fricatives, so spirantization pulls against it.
        let prior = SoundProfile::by_id("polynesian").unwrap().inventory;
        let law = catalog()
            .into_iter()
            .find(|l| l.id == "spirantization")
            .unwrap();
        let forms = [Form::from_ipa("pata").unwrap()];
        let a = law
            .assess(
                forms.iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Initial,
            )
            .unwrap();
        assert_eq!(a.words, 1);
        assert!(a.pull < 0.0);
        let untouched = [Form::from_ipa("ama").unwrap()];
        assert_eq!(
            law.assess(
                untouched.iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Initial
            ),
            None
        );
    }
}

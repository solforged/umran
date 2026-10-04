//! Lexical pitch carried by vowel nuclei, independently of vowel quality.
//! Coda loss follows Haudricourt's Vietnamese model; onset devoicing follows
//! the register splits surveyed by Matisoff for Southeast Asia.

use crate::change::Rewrite;
use crate::form::{Form, Seg};
use crate::phoneme::{CATALOG, Manner, Secondary};
use crate::{Law, Variety, World, WorldEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    High,
    Low,
    Rising,
    Falling,
    LowRising,
    LowFalling,
}

impl Tone {
    pub const ALL: [Self; 6] = [
        Self::High,
        Self::Low,
        Self::Rising,
        Self::Falling,
        Self::LowRising,
        Self::LowFalling,
    ];

    /// Chao tone letters follow the vowel and any length mark in IPA only.
    pub fn ipa(self) -> &'static str {
        match self {
            Self::High => "˥",
            Self::Low => "˩",
            Self::Rising => "˧˥",
            Self::Falling => "˥˩",
            Self::LowRising => "˩˧",
            Self::LowFalling => "˧˩",
        }
    }

    pub(crate) fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tone| tone.ipa() == raw)
    }

    fn lower(self) -> Self {
        match self {
            Self::Rising | Self::LowRising => Self::LowRising,
            Self::Falling | Self::LowFalling => Self::LowFalling,
            Self::High | Self::Low => Self::Low,
        }
    }

    pub(crate) fn merged(self) -> Self {
        match self {
            Self::High | Self::Rising => Self::High,
            Self::Low | Self::Falling | Self::LowRising | Self::LowFalling => Self::Low,
        }
    }
}

/// Tone rewrites share the segmental law's simultaneous survivor mapping,
/// grammatical-edge tracking, last-vowel guard, and minimal-word guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Change {
    Coda(Tone),
    Register,
    Merge,
    Loss,
}

impl Change {
    pub(crate) fn segment(self, form: &Form, i: usize) -> Option<Seg> {
        let mut seg = form.segs[i];
        match self {
            Self::Coda(_) => return None,
            Self::Register => {
                if let Some(c) = CATALOG.get(seg.phone).consonant()
                    && c.voiced
                    && matches!(
                        c.manner,
                        Manner::Stop | Manner::Fricative | Manner::Affricate
                    )
                    && let Some((phone, _)) = CATALOG.consonants().find(|(_, other)| {
                        other.place == c.place
                            && other.manner == c.manner
                            && !other.voiced
                            && other.secondary
                                == if c.secondary == Secondary::Breathy {
                                    Secondary::Plain
                                } else {
                                    c.secondary
                                }
                    })
                {
                    seg.phone = phone;
                }
            }
            Self::Merge => seg.tone = seg.tone.map(Tone::merged),
            Self::Loss => seg.tone = None,
        }
        Some(seg)
    }

    pub(crate) fn neighbor(self, form: &Form, i: usize) -> Option<(usize, Tone)> {
        match self {
            Self::Coda(tone) => i
                .checked_sub(1)
                .filter(|&j| form.is_vowel(j))
                .map(|j| (j, tone)),
            Self::Register => {
                let c = CATALOG.get(form.segs[i].phone).consonant()?;
                if !matches!(
                    c.manner,
                    Manner::Stop | Manner::Fricative | Manner::Affricate
                ) || i + 1 >= form.segs.len()
                    || !form.is_vowel(i + 1)
                {
                    return None;
                }
                // Do not invent loss of a voicing contrast without its actual merger.
                if c.voiced && self.segment(form, i)?.phone == form.segs[i].phone {
                    return None;
                }
                let prior = form.segs[i + 1].tone;
                Some((
                    i + 1,
                    if c.voiced {
                        prior.unwrap_or(Tone::High).lower()
                    } else {
                        prior.unwrap_or(Tone::High)
                    },
                ))
            }
            Self::Merge | Self::Loss => None,
        }
    }
}

pub fn is_tone_law(law: &Law) -> bool {
    law.rules
        .iter()
        .any(|rule| matches!(rule.result, Rewrite::Tone(_)))
}

/// A register split needs voiced obstruents to merge: without them the
/// law would mark every voiceless onset high with no contrast lost.
pub(crate) fn applies(law: &Law, variety: &Variety) -> bool {
    let register = law
        .rules
        .iter()
        .any(|rule| matches!(rule.result, Rewrite::Tone(Change::Register)));
    !register
        || variety.spoken_forms().any(|(form, _)| {
            form.segs.iter().any(|seg| {
                CATALOG.get(seg.phone).consonant().is_some_and(|c| {
                    c.voiced
                        && matches!(
                            c.manner,
                            Manner::Stop | Manner::Fricative | Manner::Affricate
                        )
                })
            })
        })
}

/// The latest period of tonality, in generations. A new gain after loss
/// starts a new period; the world's events retain all earlier periods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Tonal {
    pub since: u32,
    pub lost: Option<u32>,
}

impl Variety {
    /// Distinct marked tones in living words and grammatical forms.
    pub fn tones(&self) -> usize {
        inventory(self.grammar.forms(&self.lexicon).map(|(form, _)| form)).count_ones() as usize
    }
}

pub(crate) fn inventory<'a>(forms: impl Iterator<Item = &'a Form>) -> u8 {
    forms.flat_map(|form| &form.segs).fold(0, |mask, seg| {
        mask | if CATALOG.get(seg.phone).is_vowel() {
            seg.tone.map_or(0, |tone| 1 << tone as u8)
        } else {
            0
        }
    })
}

impl World {
    pub(crate) fn tone_change(&mut self, v: usize) {
        if self.params.tone_rate <= 0.0 {
            return;
        }
        let mut rng = self.at(v).rng(&[crate::rng::key("tone")]);
        if rng.r#gen::<f32>() >= self.params.tone_rate * self.pace(v) {
            return;
        }
        let variety = &self.varieties[v];
        let candidates: Vec<_> = self
            .law_catalog()
            .iter()
            .filter(|law| is_tone_law(law) && applies(law, variety))
            .filter(|law| {
                !variety.laws.iter().any(|&(g, id)| {
                    id == law.id && self.generation.saturating_sub(g) < crate::world::LAW_RECURRENCE
                })
            })
            .filter(|law| {
                law.assess_weighted(
                    variety.grammar.forms(&variety.lexicon),
                    &variety.profile.inventory,
                    variety.minimal,
                    variety.stress(),
                )
                .is_some()
            })
            .collect();
        let weights = candidates.iter().map(|law| law.commonness).chain([1.0]);
        if let Some(law) = candidates.get(crate::rng::weighted_index(&mut rng, weights)) {
            let law = (*law).clone();
            self.apply_law(v, &law);
        }
    }

    pub(crate) fn observe_tone(&mut self, v: usize, law: Option<&'static str>) {
        let variety = &mut self.varieties[v];
        let tonal = variety.tones() > 0;
        let was = variety.tonal.is_some_and(|period| period.lost.is_none());
        if tonal == was {
            return;
        }
        if tonal {
            variety.tonal = Some(Tonal {
                since: self.generation,
                lost: None,
            });
        } else if let Some(period) = &mut variety.tonal {
            period.lost = Some(self.generation);
        }
        self.record_event(WorldEvent::Tone {
            variety: v,
            gained: tonal,
            law,
        });
    }

    pub(crate) fn tone_wave_cause(&mut self, v: usize, from: usize, start: usize) {
        let cause = self.contacts.iter().find_map(|contact| {
            let pair = (
                self.communities[contact.a].variety,
                self.communities[contact.b].variety,
            );
            ((pair == (v, from)) || (pair == (from, v)))
                .then_some(contact.cause)
                .flatten()
        });
        if let Some(cause) = cause {
            for event in start..self.events.len() {
                if matches!(self.events[event].1, WorldEvent::Tone { variety, .. } if variety == v)
                {
                    self.causes.insert(
                        event,
                        crate::Cause {
                            mechanism: crate::Mechanism::Contact,
                            ..cause
                        },
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

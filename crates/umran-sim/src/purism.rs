//! Cause-led lexical purism in a written high form, never in its vernacular.
//! Native substitutes follow lexical-purist reforms; fixing a written norm
//! follows the grammarians' separation of classical and everyday speech.

use crate::concepts::{CONCEPTS, FAMILIES, by_id};
use crate::lexicon::{Entry, Event, LexemeId, Lexicon, Origin, Variant};
use crate::phonotactics::Phonotactics;
use crate::rng::{key, stream};
use crate::{Cause, Fixing, Form, Mechanism, World, WorldEvent};
use rand::Rng;
use std::collections::HashSet;

/// A challenge can sustain a reform movement for six centuries, but cannot
/// cause reforms indefinitely after its keepers have changed language.
const PRESSURE_SPAN: u32 = 24;
const REFORM_REST: u32 = 12;
const REPLACED_SHARE: f32 = 0.4;

#[derive(Clone, Debug, PartialEq)]
pub struct Purism {
    pub since: u32,
    pub cause: Cause,
    pub replaced: Vec<Replacement>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Replacement {
    pub concept: &'static str,
    pub loan: LexemeId,
    pub native: LexemeId,
    /// Frozen spellings at the reform, not reconstructed from later speech.
    pub loan_form: String,
    pub native_form: String,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Pressure {
    community: usize,
    speech: usize,
    high: Option<usize>,
    cause: Cause,
}

impl World {
    /// Called at the actual neighbouring-state challenge, while the selected
    /// rival, prestige relation, and languages still have their recorded values.
    pub(crate) fn purist_rival(&mut self, state: usize, rival: usize) {
        let rulers = self.states[state].rulers;
        let foreign = self.states[rival].rulers;
        if self.communities[foreign].prestige <= self.communities[rulers].prestige
            || self.family(self.communities[rulers].variety)
                == self.family(self.communities[foreign].variety)
        {
            return;
        }
        self.purist_pressure(
            rulers,
            Cause {
                event: self.triggers.states[&state],
                mechanism: Mechanism::ForeignPrestige,
            },
        );
    }

    pub(crate) fn purist_pressure(&mut self, community: usize, cause: Cause) {
        if self.params.purism_rate <= 0.0 {
            return;
        }
        let speech = self.communities[community].variety;
        self.purist_pressures.push(Pressure {
            community,
            speech,
            high: self
                .diglossic(speech)
                .then_some(self.varieties[speech].high)
                .flatten(),
            cause,
        });
    }

    /// A standard must be written. A high form can outlive the state that
    /// fixed it, provided this movement still has living keepers.
    fn purist_target(&self, pressure: Pressure) -> Option<(usize, Option<usize>)> {
        let c = &self.communities[pressure.community];
        if !c.living() || c.variety != pressure.speech {
            return None;
        }
        if let Some(high) = pressure.high.or_else(|| {
            self.diglossic(c.variety)
                .then_some(self.varieties[c.variety].high)
                .flatten()
        }) {
            return Some((high, None));
        }
        let state = self.state_of(pressure.community)?;
        self.states[state].standard?;
        let v = self.standard_variety(state);
        // A city standard may differ from its rulers' dialect, but must still
        // belong to their language family for this native-language movement.
        if self.family(v) != self.family(pressure.speech) || self.varieties[v].written.is_none() {
            return None;
        }
        Some(
            self.states[state]
                .classical
                .map_or((v, Some(state)), |high| (high.variety, None)),
        )
    }

    pub(crate) fn purify_high_forms(&mut self) {
        if self.params.purism_rate <= 0.0 {
            return;
        }
        // Keep the allocation across generations. Removing a completed pressure
        // preserves the order of the remaining recorded causes.
        let mut i = 0;
        while i < self.purist_pressures.len() {
            let pressure = self.purist_pressures[i];
            let expired = self
                .generation
                .saturating_sub(self.events[pressure.cause.event].0)
                > PRESSURE_SPAN;
            let keeper = &self.communities[pressure.community];
            if expired || !keeper.living() || keeper.variety != pressure.speech {
                self.purist_pressures.remove(i);
                continue;
            }
            if self.try_purist_reform(pressure) {
                self.purist_pressures.remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn try_purist_reform(&mut self, pressure: Pressure) -> bool {
        let Some((target, fix)) = self.purist_target(pressure) else {
            return false;
        };
        if self.varieties[target]
            .purism
            .last()
            .is_some_and(|p| self.generation < p.since + REFORM_REST)
        {
            return false;
        }
        let mut rng = stream(
            self.seed,
            &[
                key("purism episode"),
                target as u64,
                pressure.cause.event as u64,
                u64::from(self.generation),
            ],
        );
        if rng.r#gen::<f32>() >= self.params.purism_rate {
            return false;
        }
        let selected: Vec<_> = self.varieties[target]
            .lexicon
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                let loan = slot.dominant()?;
                (matches!(
                    self.varieties[target].lexicon.get(loan).origin,
                    Origin::Borrowed { .. }
                ) && rng.r#gen::<f32>() < REPLACED_SHARE)
                    .then_some((i, loan))
            })
            .collect();
        if selected.is_empty() {
            return false;
        }
        let high = if let Some(state) = fix {
            self.fix(state, Fixing::Purism);
            // The fixing, as well as the lexical reform, answers this cause.
            self.causes.insert(self.events.len() - 1, pressure.cause);
            self.states[state].classical.expect("just fixed").variety
        } else {
            target
        };
        let since = self.generation;
        let mut replaced = Vec::with_capacity(selected.len());
        for (slot, loan) in selected {
            let mut words = stream(
                self.seed,
                &[
                    key("purism native"),
                    high as u64,
                    u64::from(since),
                    key(CONCEPTS[slot].id),
                ],
            );
            let variety = &mut self.varieties[high];
            let loan_form = variety.written_word(variety.lexicon.get(loan));
            let native = native_substitute(variety, slot, since, &mut words);
            let lexicon = &mut variety.lexicon;
            let already_used = lexicon.slots[slot].has(native);
            for variant in &lexicon.slots[slot].variants {
                if variant.lexeme != native {
                    lexicon.lexemes[variant.lexeme.0 as usize].log.push(Entry {
                        generation: since,
                        event: Event::Lost {
                            sense: CONCEPTS.get(slot).unwrap(),
                        },
                    });
                }
            }
            let word = lexicon.get_mut(native);
            if !already_used && word.born < since {
                // An obsolete native word returns with its old form and identity.
                // Its earlier Obsolete entry remains part of the history.
                word.log.push(Entry {
                    generation: since,
                    event: Event::Extended {
                        to: CONCEPTS.get(slot).unwrap(),
                    },
                });
            }
            word.obsolete = None;
            lexicon.slots[slot].variants.clear();
            lexicon.slots[slot].variants.push(Variant {
                lexeme: native,
                weight: 1.0,
            });
            let native_form = variety.written_word(variety.lexicon.get(native));
            replaced.push(Replacement {
                concept: CONCEPTS[slot].id,
                loan,
                native,
                loan_form,
                native_form,
            });
        }
        self.retire(high);
        self.varieties[high].sync_grammar(since);
        let episode = self.varieties[high].purism.len();
        self.varieties[high].purism.push(Purism {
            since,
            cause: pressure.cause,
            replaced,
        });
        let event = self.record_response(
            WorldEvent::PuristReform {
                variety: high,
                episode,
            },
            Some(pressure.cause),
        );
        for replacement in 0..self.varieties[high].purism[episode].replaced.len() {
            self.record_response(
                WorldEvent::PuristReplacement {
                    variety: high,
                    episode,
                    replacement,
                },
                Some(Cause {
                    event,
                    mechanism: Mechanism::PuristNorm,
                }),
            );
        }
        true
    }
}

/// Deriving from a loan does not make its root native. Origin links always
/// point backwards, including the two ingredients of a renewed compound.
fn native(lexicon: &Lexicon, id: LexemeId) -> bool {
    match lexicon.get(id).origin {
        Origin::Founding | Origin::Expressive => true,
        Origin::Borrowed { .. } => false,
        Origin::Derived { base, .. } => native(lexicon, base),
        Origin::Renewed { base, with } => {
            native(lexicon, base) && with.is_none_or(|id| native(lexicon, id))
        }
        Origin::Compound { modifier, head } => native(lexicon, modifier) && native(lexicon, head),
    }
}

fn native_substitute(
    variety: &mut crate::Variety,
    slot: usize,
    generation: u32,
    rng: &mut impl Rng,
) -> LexemeId {
    let lexicon = &variety.lexicon;
    let concept = &CONCEPTS[slot];
    // Prefer a surviving native competitor, then an attested archaism. Do not
    // revive a homophone of an unrelated living word or a borrowed derivative.
    let current = lexicon.slots[slot]
        .variants
        .iter()
        .filter(|v| native(lexicon, v.lexeme))
        .max_by(|a, b| a.weight.total_cmp(&b.weight))
        .map(|v| v.lexeme);
    if let Some(id) = current.or_else(|| {
        lexicon
            .lexemes
            .iter()
            .find(|word| {
                word.obsolete.is_some()
                    && word.first_sense.id == concept.id
                    && native(lexicon, word.id)
                    && !lexicon.living().any(|other| other.form == word.form)
            })
            .map(|word| word.id)
    }) {
        return id;
    }
    let built = FAMILIES
        .iter()
        .filter(|(_, to, _)| *to == concept.id)
        .find_map(|&(from, _, relation)| {
            let base = lexicon.word_for(by_id(from)?)?;
            if !native(lexicon, base.id) {
                return None;
            }
            let form = variety.morphology.derive(&base.form, None, relation)?;
            (!lexicon.living().any(|w| w.form == form)).then_some((
                form,
                Origin::Derived {
                    base: base.id,
                    relation,
                },
            ))
        });
    let (form, origin) = built.unwrap_or_else(|| {
        let observed = if lexicon.living().any(|w| native(lexicon, w.id)) {
            Phonotactics::observe(
                lexicon
                    .living()
                    .filter(|w| native(lexicon, w.id))
                    .map(|w| &w.form),
            )
        } else {
            Phonotactics::compile(&variety.profile.phonotactics, &variety.founding_inventory)
        };
        let used: HashSet<Form> = lexicon.living().map(|w| w.form.clone()).collect();
        let field: HashSet<Form> = lexicon
            .slots
            .iter()
            .filter(|s| s.concept.field == concept.field)
            .flat_map(|s| s.variants.iter())
            .map(|v| lexicon.get(v.lexeme).form.clone())
            .collect();
        (
            crate::root::mint_one(
                rng,
                &observed,
                &variety.profile.spelling,
                Some(&variety.morphology),
                concept,
                &used,
                &field,
            ),
            Origin::Expressive,
        )
    });
    variety.lexicon.coin(form, origin, concept, generation)
}

#[cfg(test)]
mod tests;

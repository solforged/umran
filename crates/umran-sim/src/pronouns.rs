//! Personal pronouns share the lexicon's words, histories, and regular sound laws.
//! Renewal copies a noun into a new grammatical use; intense contact may copy
//! a pronoun, and a court may extend plural address to a polite singular.
use crate::adapt::Adapter;
use crate::concepts::{Concept, by_id};
use crate::grammar::same_sound;
use crate::lexicon::{Entry, Event, LexemeId, Lexicon, Origin, Variant};
use crate::phonotactics::Phonotactics;
use crate::rng::{index, key, stream, weighted_index};
use crate::{Cause, ContactKind, Form, Mechanism, Variety, World};
use rand::Rng;
use std::collections::BTreeMap;

pub const CELLS: [(u8, &str, &str); 6] = [
    (1, "sg", "1sg"),
    (1, "pl", "1pl"),
    (2, "sg", "2sg"),
    (2, "pl", "2pl"),
    (3, "sg", "3sg"),
    (3, "pl", "3pl"),
];

pub fn is_pronoun(concept: &Concept) -> bool {
    CELLS.iter().any(|(_, _, id)| *id == concept.id)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pronouns {
    /// The second-person singular cell now serves as polite address.
    pub polite: bool,
    pub events: Vec<Notice>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub generation: u32,
    pub cell: &'static str,
    pub before: Form,
    pub after: Form,
    pub event: NoticeKind,
    pub cause: Option<Cause>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NoticeKind {
    Renewed {
        source: LexemeId,
        concept: &'static str,
        merger: bool,
    },
    Polite {
        state: usize,
    },
    Borrowed {
        from: usize,
        source: LexemeId,
        source_form: Form,
    },
}

/// Simple, short syllables favour the same common segments as the founding
/// inventory. Collisions draw again; a small inventory may need longer forms.
pub(crate) fn found(seed: u64, tactics: &Phonotactics, lexicon: &mut Lexicon) {
    let onsets: Vec<_> = tactics
        .onsets
        .iter()
        .filter(|(p, _)| p.len() == 1)
        .collect();
    let mut used = Vec::with_capacity(CELLS.len());
    for (_, _, cell) in CELLS {
        let id = lexicon
            .slot(by_id(cell).expect("pronoun concept"))
            .dominant()
            .expect("founding pronoun");
        let stress = lexicon.get(id).form.stress.map(|_| 0);
        let mut rng = stream(seed, &[key("pronoun founding"), key(cell)]);
        let mut attempt = 0;
        let form = loop {
            let syllables = 1 + attempt / 32;
            let mut phones = Vec::with_capacity(syllables * 2);
            for _ in 0..syllables {
                if !onsets.is_empty() {
                    let onset = onsets[weighted_index(&mut rng, onsets.iter().map(|(_, w)| *w))];
                    phones.push(onset.0[0]);
                }
                phones.push(
                    tactics.nuclei
                        [weighted_index(&mut rng, tactics.nuclei.iter().map(|(_, w)| *w))]
                    .0,
                );
            }
            let mut form = Form::from_phones(phones);
            form.stress = stress;
            if !used.contains(&form) {
                break form;
            }
            attempt += 1;
        };
        lexicon.get_mut(id).form = form.clone();
        used.push(form);
    }
}

fn replace(
    variety: &mut Variety,
    cell: &'static str,
    form: Form,
    origin: Origin,
    generation: u32,
) -> (LexemeId, Form) {
    let concept = by_id(cell).expect("pronoun concept");
    let old = variety
        .lexicon
        .slot(concept)
        .dominant()
        .expect("living pronoun");
    let before = variety.lexicon.get(old).form.clone();
    let new = variety.lexicon.coin(form, origin, concept, generation);
    let slot = variety.lexicon.slot_mut(concept);
    slot.variants.clear();
    slot.variants.push(Variant {
        lexeme: new,
        weight: 1.0,
    });
    variety.lexicon.get_mut(old).log.push(Entry {
        generation,
        event: Event::Lost { sense: concept },
    });
    if variety.lexicon.senses(old).next().is_none() {
        let old = variety.lexicon.get_mut(old);
        old.obsolete = Some(generation);
        old.log.push(Entry {
            generation,
            event: Event::Obsolete,
        });
    }
    (new, before)
}

fn clashes(variety: &Variety, cell: &str, form: &Form) -> bool {
    CELLS.iter().any(|(_, _, other)| {
        // Plural-for-polite is an intentional shared form, not a merger.
        let polite_pair =
            variety.pronouns.polite && matches!((cell, *other), ("2sg", "2pl") | ("2pl", "2sg"));
        *other != cell
            && !polite_pair
            && variety
                .lexicon
                .word_for(by_id(other).unwrap())
                .is_some_and(|word| same_sound(form, &word.form, variety.stress()))
    })
}

impl World {
    pub(crate) fn evolve_pronouns(&mut self, spoken: &[bool]) {
        let rate = self.params.pronoun_rate;
        if rate <= 0.0 {
            return;
        }
        let generation = self.generation;
        for (v, &is_spoken) in spoken.iter().enumerate() {
            if !is_spoken {
                continue;
            }
            for (_, _, cell) in CELLS {
                let variety = &self.varieties[v];
                let word = variety
                    .lexicon
                    .word_for(by_id(cell).unwrap())
                    .expect("living pronoun");
                let merger = clashes(variety, cell, &word.form);
                let worn = variety.minimal.worn(&word.form) || word.form.segs.len() < 2;
                if !merger && !worn {
                    continue;
                }
                let mut rng = stream(
                    self.seed,
                    &[
                        key("pronoun renewal"),
                        v as u64,
                        generation as u64,
                        key(cell),
                    ],
                );
                if rng.r#gen::<f32>() >= rate {
                    continue;
                }
                let nouns: &[&str] = if cell.ends_with("pl") {
                    &["people", "person", "child"]
                } else {
                    &["person", "head", "heart", "child"]
                };
                let candidates: Vec<_> = nouns
                    .iter()
                    .filter_map(|id| {
                        let noun = variety.lexicon.word_for(by_id(id)?)?;
                        (!variety.minimal.worn(&noun.form)
                            && noun.form.segs.len() >= 2
                            && !same_sound(&word.form, &noun.form, variety.stress())
                            && !clashes(variety, cell, &noun.form))
                        .then_some((noun.id, *id))
                    })
                    .collect();
                if candidates.is_empty() {
                    continue;
                }
                let (source, concept) = candidates[index(&mut rng, candidates.len())];
                let after = variety.lexicon.get(source).form.clone();
                let variety = &mut self.varieties[v];
                let (_, before) = replace(
                    variety,
                    cell,
                    after.clone(),
                    Origin::Renewed {
                        base: source,
                        with: None,
                    },
                    generation,
                );
                variety.pronouns.events.push(Notice {
                    generation,
                    cell,
                    before,
                    after,
                    event: NoticeKind::Renewed {
                        source,
                        concept,
                        merger,
                    },
                    cause: None,
                });
            }
        }
        self.polite_pronouns();
        self.borrow_pronouns();
    }

    fn polite_pronouns(&mut self) {
        let mut courts = BTreeMap::new();
        for community in self.living() {
            if let Some(state) = self.state_of(community)
                && self.generation.saturating_sub(self.states[state].rose) >= 8
            {
                courts
                    .entry(self.communities[community].variety)
                    .or_insert(state);
            }
        }
        for (v, state) in courts {
            if self.varieties[v].pronouns.polite {
                continue;
            }
            let mut rng = stream(
                self.seed,
                &[key("pronoun politeness"), v as u64, self.generation as u64],
            );
            if rng.r#gen::<f32>() >= self.params.pronoun_rate * 6.0 {
                continue;
            }
            let source = self.varieties[v]
                .lexicon
                .word_for(by_id("2pl").unwrap())
                .unwrap();
            let (source, after) = (source.id, source.form.clone());
            let cause = self.triggers.states.get(&state).map(|&event| Cause {
                event,
                mechanism: Mechanism::Court,
            });
            let variety = &mut self.varieties[v];
            let (_, before) = replace(
                variety,
                "2sg",
                after.clone(),
                Origin::Renewed {
                    base: source,
                    with: None,
                },
                self.generation,
            );
            variety.pronouns.polite = true;
            variety.pronouns.events.push(Notice {
                generation: self.generation,
                cell: "2sg",
                before,
                after,
                event: NoticeKind::Polite { state },
                cause,
            });
        }
    }

    fn borrow_pronouns(&mut self) {
        // Decide against a snapshot: no loan becomes another donor's form this tick.
        let mut loans = BTreeMap::new();
        for contact in &self.contacts {
            if !matches!(contact.kind, ContactKind::Rule | ContactKind::Intermarriage)
                || contact.intensity < 0.9
            {
                continue;
            }
            for (recipient, donor) in [(contact.a, contact.b), (contact.b, contact.a)] {
                let (r, d) = (&self.communities[recipient], &self.communities[donor]);
                if !r.living() || !d.living() || r.variety == d.variety || r.openness < 0.75 {
                    continue;
                }
                let own = &self.varieties[r.variety];
                if own
                    .grammar
                    .contact_generations
                    .get(&d.variety)
                    .copied()
                    .unwrap_or(0)
                    < 8
                {
                    continue;
                }
                let mut rng = stream(
                    self.seed,
                    &[
                        key("pronoun borrowing"),
                        r.variety as u64,
                        d.variety as u64,
                        self.generation as u64,
                    ],
                );
                if rng.r#gen::<f32>()
                    >= self.params.pronoun_rate * 0.05 * contact.intensity * r.openness
                {
                    continue;
                }
                // Third-person plural is less resistant than speaker and addressee.
                let cell =
                    CELLS[weighted_index(&mut rng, [1.0, 1.0, 1.0, 1.0, 2.0, 3.0].into_iter())].2;
                let word = self.varieties[d.variety]
                    .lexicon
                    .word_for(by_id(cell).unwrap())
                    .unwrap();
                let adapter = Adapter::new(
                    own.lexicon.living().map(|l| &l.form),
                    &own.profile.inventory,
                );
                let after = adapter.adapt(
                    &word.form,
                    self.params.bilingual_keep * contact.intensity * r.openness,
                    &mut rng,
                );
                let before = &own.lexicon.word_for(by_id(cell).unwrap()).unwrap().form;
                if same_sound(before, &after, own.stress())
                    || after.vowel_count() == 0
                    || clashes(own, cell, &after)
                {
                    continue;
                }
                loans.entry(r.variety).or_insert((
                    cell,
                    d.variety,
                    word.id,
                    word.form.clone(),
                    after,
                    self.contact_loan_cause(contact, donor, recipient),
                    self.contact_cause(recipient, donor),
                ));
            }
        }
        for (v, (cell, from, source, source_form, after, provenance, cause)) in loans {
            let variety = &mut self.varieties[v];
            let (new, before) = replace(
                variety,
                cell,
                after.clone(),
                Origin::Borrowed {
                    from,
                    source,
                    cause: provenance,
                },
                self.generation,
            );
            variety.lexicon.get_mut(new).log.push(Entry {
                generation: self.generation,
                event: Event::Borrowed {
                    from,
                    source: source_form.clone(),
                    cause: provenance,
                },
            });
            variety.pronouns.events.push(Notice {
                generation: self.generation,
                cell,
                before,
                after,
                event: NoticeKind::Borrowed {
                    from,
                    source,
                    source_form,
                },
                cause,
            });
        }
    }
}

#[cfg(test)]
mod tests;

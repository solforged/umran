//! Productive native word building and contact-driven loan translations.
use crate::concepts::{CONCEPTS, Concept, FAMILIES, by_id};
use crate::ideas::{BUILDS, need};
use crate::lexicon::{LexemeId, Lexicon, Origin};
use crate::morphology::Morphology;
use crate::rng::{index, key};
use crate::{Form, World, WorldEvent};
use rand::Rng;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoinageKind {
    Compound,
    Derived,
    Calque,
}

/// Parts retain the local meanings and forms used at creation, not donor sounds.
/// Compound parts are modifier then head, independently of surface order.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub concept: &'static Concept,
    pub form: Form,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Coinage {
    pub parts: Vec<Part>,
    pub kind: CoinageKind,
    pub generation: u32,
    pub from: Option<usize>,
    pub opaque_since: Option<u32>,
}

/// Curated semantic recipes, not freely paired words. Includes noun+noun and
/// verb+noun ("pray-house", "write-word"); order comes from Morphology::compound.
const COMPOUNDS: &[(&str, &str, &str)] = &[
    ("fire", "stone", "iron"),
    ("sun", "stone", "bronze"),
    ("iron", "person", "smith"),
    ("horse", "cloth", "saddle"),
    ("wind", "cloth", "sail"),
    ("sea", "boat", "ship"),
    ("word", "tree", "book"),
    ("write", "word", "letter"),
    ("pray", "house", "temple"),
    ("god", "person", "prophet"),
    ("god", "sky", "heaven"),
    ("die", "spirit", "demon"),
    ("spirit", "person", "sorcerer"),
    ("fish", "cloth", "net"),
    ("fight", "cloth", "shield"),
    ("hunt", "stone", "spear"),
    ("grain", "water", "beer"),
    ("buy", "house", "market"),
    ("people", "head", "chief"),
];

pub(crate) struct Build {
    form: Form,
    origin: Origin,
    parts: Vec<Part>,
    kind: CoinageKind,
    from: Option<usize>,
}

fn part(lexicon: &Lexicon, concept: &'static Concept) -> Option<(LexemeId, Part)> {
    let word = lexicon.word_for(concept)?;
    Some((
        word.id,
        Part {
            concept,
            form: word.form.clone(),
        },
    ))
}

fn compound(
    world: &World,
    v: usize,
    modifier: &'static Concept,
    head: &'static Concept,
    from: Option<usize>,
) -> Option<Build> {
    let variety = &world.varieties[v];
    let (modifier, m) = part(&variety.lexicon, modifier)?;
    let (head, h) = part(&variety.lexicon, head)?;
    let form = variety.morphology.compound(&m.form, &h.form);
    Some(Build {
        form,
        origin: Origin::Compound { modifier, head },
        parts: vec![m, h],
        kind: if from.is_some() {
            CoinageKind::Calque
        } else {
            CoinageKind::Compound
        },
        from,
    })
}

/// Follow derivational bases rather than treating a pattern prefix as a radical.
fn skeleton(lexicon: &Lexicon, mut id: LexemeId) -> Option<[crate::PhonemeId; 3]> {
    loop {
        let word = lexicon.get(id);
        match word.origin {
            Origin::Derived { base, .. } | Origin::Renewed { base, .. } => id = base,
            _ => return Morphology::skeleton(&word.form),
        }
    }
}

impl World {
    fn derived_coinage(
        &self,
        v: usize,
        concept: &'static Concept,
        productive: bool,
    ) -> Option<Build> {
        let variety = &self.varieties[v];
        BUILDS
            .iter()
            .chain(FAMILIES.iter().filter(|_| productive))
            .filter(|(_, target, _)| *target == concept.id)
            .find_map(|&(base, _, relation)| {
                let (base, part) = part(&variety.lexicon, by_id(base)?)?;
                let root = if productive {
                    skeleton(&variety.lexicon, base)
                } else {
                    None
                };
                let form = variety.morphology.derive(&part.form, root, relation)?;
                (!variety.lexicon.living().any(|l| l.form == form)).then_some(Build {
                    form,
                    origin: Origin::Derived { base, relation },
                    parts: vec![part],
                    kind: CoinageKind::Derived,
                    from: None,
                })
            })
    }

    /// A native build, or a translation of a transparent donor compound. An
    /// opaque donor cannot teach its segmentation; a missing local part cannot
    /// be replaced with donor material and passed off as a calque.
    pub(crate) fn plan_coinage(
        &self,
        v: usize,
        concept: &'static Concept,
        donor: Option<usize>,
    ) -> Option<Build> {
        if self.params.coinage_rate <= 0.0 {
            return self.derived_coinage(v, concept, false);
        }
        let mut rng = self.at(v).rng(&[key("coinage-build"), key(concept.id)]);
        let start = index(&mut rng, 3);
        for offset in 0..3 {
            let build = match (start + offset) % 3 {
                0 => self.derived_coinage(v, concept, true),
                1 => COMPOUNDS
                    .iter()
                    .filter(|(_, _, target)| *target == concept.id)
                    .find_map(|&(modifier, head, _)| {
                        compound(self, v, by_id(modifier)?, by_id(head)?, None)
                    }),
                _ => donor.and_then(|from| {
                    let source = self.varieties[from]
                        .lexicon
                        .word_for(concept)?
                        .coined
                        .as_ref()?;
                    if source.opaque_since.is_some()
                        || source.parts.len() != 2
                        || source.kind == CoinageKind::Derived
                    {
                        return None;
                    }
                    compound(
                        self,
                        v,
                        source.parts[0].concept,
                        source.parts[1].concept,
                        Some(from),
                    )
                }),
            };
            if let Some(build) = build
                && !self.varieties[v]
                    .lexicon
                    .living()
                    .any(|l| l.form == build.form)
            {
                return Some(build);
            }
        }
        None
    }

    /// Store one lexical item. It is never rebuilt when its components change.
    pub(crate) fn install_coinage(
        &mut self,
        community: usize,
        concept: &'static Concept,
        build: Build,
    ) -> LexemeId {
        let v = self.communities[community].variety;
        let id = self.varieties[v]
            .lexicon
            .coin(build.form, build.origin, concept, self.generation);
        if self.params.coinage_rate > 0.0 {
            self.varieties[v].lexicon.get_mut(id).coined = Some(Coinage {
                parts: build.parts,
                kind: build.kind,
                generation: self.generation,
                from: build.from,
                opaque_since: None,
            });
            let cause = need(concept)
                .and_then(|need| self.word_need_cause(community, need))
                .or_else(|| {
                    build
                        .from
                        .and_then(|from| self.speakers(from))
                        .and_then(|from| self.contact_cause(community, from))
                });
            self.record_response(
                WorldEvent::Coined {
                    community,
                    variety: v,
                    word: id,
                    from: build.from,
                },
                cause,
            );
        }
        id
    }

    /// Existing meanings can acquire new native competitors too. find_word
    /// handles gaps, including the choice between building and borrowing.
    pub(crate) fn coin_words(&mut self, v: usize) {
        if self.params.coinage_rate <= 0.0 {
            return;
        }
        let Some(community) = self.speakers(v) else {
            return;
        };
        let rate = self.params.coinage_rate * (1.0 + self.purism(v));
        for concept in CONCEPTS {
            let slot = self.varieties[v].lexicon.slot(concept);
            if slot.variants.is_empty()
                || slot.variants.len() >= crate::world::MAX_VARIANTS
                || need(concept).is_some_and(|n| !self.holds(community, n))
            {
                continue;
            }
            let mut rng = self.at(v).rng(&[key("coinage-turnover"), key(concept.id)]);
            if rng.r#gen::<f32>() >= rate {
                continue;
            }
            let donor = self
                .partners(community)
                .filter(|&(o, _, _)| self.communities[o].variety != v)
                .filter(|&(o, _, _)| {
                    self.variety_of(o)
                        .lexicon
                        .word_for(concept)
                        .is_some_and(|w| w.coined.is_some())
                })
                .max_by(|a, b| {
                    let weight = |&(o, i, _): &(usize, f32, crate::ContactKind)| {
                        i * (0.1 + self.communities[o].prestige)
                    };
                    weight(a).total_cmp(&weight(b)).then(b.0.cmp(&a.0))
                })
                .map(|(o, _, _)| self.communities[o].variety);
            if let Some(build) = self.plan_coinage(v, concept, donor) {
                let id = self.install_coinage(community, concept, build);
                self.varieties[v]
                    .lexicon
                    .slot_mut(concept)
                    .introduce(id, self.params.newcomer_share);
            }
        }
    }
}

/// Literal segment recognizability, independent of spelling and stress. Root
/// patterns and boundary coalescence can already be opaque at creation. Once
/// recorded, opacity is historical even if later sound mergers restore a match.
pub(crate) fn observe_opacity(lexicon: &mut Lexicon, generation: u32) {
    for i in 0..lexicon.lexemes.len() {
        let word = &lexicon.lexemes[i];
        if word.obsolete.is_some() {
            continue;
        }
        let Some(coinage) = &word.coined else {
            continue;
        };
        if coinage.opaque_since.is_some() {
            continue;
        }
        let opaque = coinage.parts.iter().any(|part| {
            lexicon.word_for(part.concept).is_none_or(|current| {
                let part = &current.form.segs;
                part.is_empty()
                    || !word
                        .form
                        .segs
                        .windows(part.len())
                        .any(|window| window == part)
            })
        });
        if opaque {
            lexicon.lexemes[i].coined.as_mut().unwrap().opaque_since = Some(generation);
        }
    }
}

#[cfg(test)]
mod tests;

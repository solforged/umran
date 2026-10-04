//! Doctrinal commitments, dissent, sacred registers, and lexical avoidance.
//! These are social biases, not claims that a worldview entails a theology.
use crate::concepts::by_id;
use crate::ethos::Ethos;
use crate::ideas::{Craft, Need, need};
use crate::lexicon::{LexemeId, Origin};
use crate::provenance::LoanCause;
use crate::rng::{index, key, stream};
use crate::schisms::SchismCause;
use crate::{Cause, Mechanism, World, WorldEvent};
use rand::Rng;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tenet {
    SacredLanguage,
    Images,
    Hierarchy,
    Purity,
    Pilgrimage,
    Monasticism,
}

impl Tenet {
    pub const ALL: [Self; 6] = [
        Self::SacredLanguage,
        Self::Images,
        Self::Hierarchy,
        Self::Purity,
        Self::Pilgrimage,
        Self::Monasticism,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::SacredLanguage => "sacred-language",
            Self::Images => "images",
            Self::Hierarchy => "hierarchy",
            Self::Purity => "purity",
            Self::Pilgrimage => "pilgrimage",
            Self::Monasticism => "monasticism",
        }
    }

    /// The positive pole requires sacred speech, venerates images, keeps
    /// clergy, purity observances, pilgrimage, or monastic withdrawal.
    fn inclination(self, e: Ethos) -> f32 {
        match self {
            Self::SacredLanguage => 0.2 + 0.4 * e.pious - 0.5 * e.open,
            Self::Images => 0.35 * e.open - 0.25 * e.pious,
            Self::Hierarchy => 0.8 * e.hierarchical,
            Self::Purity => 0.2 + 0.5 * e.pious - 0.3 * e.open,
            Self::Pilgrimage => 0.4 * e.pious + 0.4 * e.roving,
            Self::Monasticism => 0.5 * e.pious - 0.3 * e.martial,
        }
        .clamp(-1.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Position {
    pub tenet: Tenet,
    pub stance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Dispute {
    pub tenet: Tenet,
    pub previous: f32,
    pub stance: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Doctrine {
    stances: [f32; 6],
    /// Last publicly recorded stance, for hysteresis and causal attribution.
    announced: [f32; 6],
    events: [Option<usize>; 6],
    /// A concept, never a sound sequence: other senses of a word survive.
    pub taboo: &'static str,
}

impl Doctrine {
    pub fn positions(&self) -> impl Iterator<Item = Position> + '_ {
        Tenet::ALL.into_iter().map(|tenet| Position {
            tenet,
            stance: self.stance(tenet),
        })
    }

    pub fn stance(&self, tenet: Tenet) -> f32 {
        self.stances[tenet as usize]
    }

    pub(crate) fn found(
        seed: u64,
        religion: usize,
        ethos: Ethos,
        enabled: bool,
        translates: bool,
    ) -> Self {
        let mut stances = [0.0; 6];
        if enabled {
            for tenet in Tenet::ALL {
                let mut rng = stream(
                    seed,
                    &[key("doctrine found"), religion as u64, key(tenet.id())],
                );
                stances[tenet as usize] =
                    (tenet.inclination(ethos) + rng.gen_range(-0.45..0.45)).clamp(-1.0, 1.0);
            }
        } else {
            stances[Tenet::SacredLanguage as usize] = if translates { -0.5 } else { 0.5 };
        }
        let mut rng = stream(seed, &[key("doctrine taboo"), religion as u64]);
        let taboo = ["blood", "god", "cattle"][index(&mut rng, 3)];
        Self {
            stances,
            announced: stances,
            events: [None; 6],
            taboo,
        }
    }
}

impl World {
    pub(crate) fn adopt_doctrine(&mut self, religion: usize, cause: Option<Cause>) {
        if !self.params.doctrine_enabled {
            return;
        }
        for tenet in Tenet::ALL {
            let stance = self.religions[religion].doctrine.stance(tenet);
            let event = self.record_response(
                WorldEvent::TenetAdopted {
                    religion,
                    tenet,
                    previous: None,
                    stance,
                },
                cause,
            );
            self.religions[religion].doctrine.events[tenet as usize] = Some(event);
        }
    }

    /// Dissent is measured against the reforming community's worldview.
    /// A language reform specifically demands vernacular teaching; rule and
    /// succession make authority more contentious without predetermining it.
    pub(crate) fn disputed_doctrine(
        &self,
        parent: usize,
        community: usize,
        cause: SchismCause,
    ) -> (Doctrine, Option<Tenet>) {
        let mut doctrine = self.religions[parent].doctrine.clone();
        if !self.params.doctrine_enabled {
            return (doctrine, None);
        }
        let ethos = self.communities[community].ethos;
        let target = |tenet: Tenet| {
            if cause == SchismCause::Reform && tenet == Tenet::SacredLanguage {
                -0.85
            } else {
                tenet.inclination(ethos)
            }
        };
        let tension = |tenet: Tenet| {
            (target(tenet) - doctrine.stance(tenet)).abs()
                + match (cause, tenet) {
                    (SchismCause::Reform, Tenet::SacredLanguage) => 2.0,
                    (SchismCause::Rule | SchismCause::Succession, Tenet::Hierarchy) => 0.6,
                    _ => 0.0,
                }
        };
        let mut disputed = Tenet::ALL[0];
        for tenet in Tenet::ALL.into_iter().skip(1) {
            if tension(tenet) > tension(disputed) {
                disputed = tenet;
            }
        }
        let old = doctrine.stance(disputed);
        let desired = target(disputed);
        let direction = if desired < old || (desired == old && old >= 0.0) {
            -1.0
        } else {
            1.0
        };
        let stance = if cause == SchismCause::Reform && disputed == Tenet::SacredLanguage {
            desired
        } else {
            (old + direction * (desired - old).abs().max(0.6)).clamp(-1.0, 1.0)
        };
        doctrine.stances[disputed as usize] = stance;
        doctrine.announced[disputed as usize] = stance;
        // An inherited assertion is not a newly observed cause in the branch.
        doctrine.events = [None; 6];
        (doctrine, Some(disputed))
    }

    pub(crate) fn record_doctrine_dispute(
        &mut self,
        religion: usize,
        parent: usize,
        tenet: Option<Tenet>,
        schism: usize,
    ) {
        let Some(tenet) = tenet else {
            return;
        };
        let event = self.record_response(
            WorldEvent::TenetDisputed {
                religion,
                parent,
                tenet,
                previous: self.religions[parent].doctrine.stance(tenet),
                stance: self.religions[religion].doctrine.stance(tenet),
            },
            Some(Cause {
                event: schism,
                mechanism: Mechanism::Doctrine,
            }),
        );
        self.religions[religion].doctrine.events[tenet as usize] = Some(event);
    }

    pub(crate) fn change_doctrine(&mut self) {
        if !self.params.doctrine_enabled {
            return;
        }
        // One population-weighted update per faith, never dependent on the
        // iteration order in which its followers learn the resulting tenets.
        let mut means = vec![([0.0; 6], 0.0); self.religions.len()];
        for c in self.living() {
            let people = &self.communities[c];
            if let Some(faith) = people.faith {
                for tenet in Tenet::ALL {
                    means[faith].0[tenet as usize] += tenet.inclination(people.ethos) * people.size;
                }
                means[faith].1 += people.size;
            }
        }
        for (religion, (means, size)) in means.into_iter().enumerate() {
            if size <= 0.0 {
                continue;
            }
            for tenet in Tenet::ALL {
                let i = tenet as usize;
                let mut rng = stream(
                    self.seed,
                    &[
                        key("doctrine drift"),
                        religion as u64,
                        u64::from(self.generation),
                        key(tenet.id()),
                    ],
                );
                let doctrine = &mut self.religions[religion].doctrine;
                let previous = doctrine.announced[i];
                let old = doctrine.stances[i];
                let stance = (old + 0.003 * (means[i] / size - old) + rng.gen_range(-0.008..0.008))
                    .clamp(-1.0, 1.0);
                doctrine.stances[i] = stance;
                if (stance - previous).abs() >= 0.25 {
                    doctrine.announced[i] = stance;
                    let event = self.record_event(WorldEvent::TenetAdopted {
                        religion,
                        tenet,
                        previous: Some(previous),
                        stance,
                    });
                    self.religions[religion].doctrine.events[i] = Some(event);
                }
            }
            // Hysteresis avoids repeated register switches near neutrality.
            let sacred = self.religions[religion]
                .doctrine
                .stance(Tenet::SacredLanguage);
            if sacred > 0.15 {
                self.religions[religion].translates = false;
            }
            if sacred < -0.15 {
                self.religions[religion].translates = true;
            }
        }
        for c in 0..self.communities.len() {
            if !self.communities[c].living() {
                continue;
            }
            let Some(religion) = self.communities[c].faith else {
                continue;
            };
            self.doctrine_register(c, religion);
            let v = self.communities[c].variety;
            // Shared speech receives one set of changes from its largest
            // community, not repeated renewals from every small congregation.
            if self.speakers(v) != Some(c) {
                continue;
            }
            let purity = self.religions[religion].doctrine.stance(Tenet::Purity);
            let mut rng = self
                .at(v)
                .rng(&[key("doctrine avoidance"), religion as u64]);
            if purity > 0.35 && rng.r#gen::<f32>() < 0.025 * purity {
                self.replace_taboo(c, religion);
            }
            self.doctrine_words(c, religion);
        }
    }

    fn replace_taboo(&mut self, community: usize, religion: usize) -> Option<LexemeId> {
        let doctrine = &self.religions[religion].doctrine;
        let concept = by_id(doctrine.taboo)?;
        let cause = doctrine.events[Tenet::Purity as usize].map(|event| Cause {
            event,
            mechanism: Mechanism::Doctrine,
        });
        let v = self.communities[community].variety;
        let variety = &self.varieties[v];
        let old = variety.lexicon.slot(concept).dominant()?;
        let form = variety.morphology.renew(&variety.lexicon.get(old).form);
        if variety.lexicon.living().any(|word| word.form == form) {
            return None;
        }
        let lexicon = &mut self.varieties[v].lexicon;
        let word = lexicon.coin(
            form,
            Origin::Renewed {
                base: old,
                with: None,
            },
            concept,
            self.generation,
        );
        while let Some(variant) = lexicon.slot_mut(concept).variants.pop() {
            lexicon.get_mut(variant.lexeme).log.push(crate::Entry {
                generation: self.generation,
                event: crate::Event::Lost { sense: concept },
            });
            if !lexicon.slots.iter().any(|slot| slot.has(variant.lexeme)) {
                lexicon.get_mut(variant.lexeme).obsolete = Some(self.generation);
                lexicon.get_mut(variant.lexeme).log.push(crate::Entry {
                    generation: self.generation,
                    event: crate::Event::Obsolete,
                });
            }
        }
        lexicon.slot_mut(concept).introduce(word, 1.0);
        self.varieties[v].sync_grammar(self.generation);
        self.record_response(
            WorldEvent::TabooReplaced {
                religion,
                community,
                variety: v,
                concept,
                old,
                word,
            },
            cause,
        );
        Some(word)
    }

    /// A separate stream adds doctrinal loans without shifting contact draws.
    /// Every borrowed word retains the existing faith provenance.
    fn doctrine_words(&mut self, community: usize, religion: usize) {
        let faith = &self.religions[religion];
        if faith.translates {
            return;
        }
        let stance = faith.doctrine.stance(Tenet::SacredLanguage).max(0.0);
        let sacred = faith.sacred;
        let taboo = (faith.doctrine.stance(Tenet::Purity) > 0.35).then_some(faith.doctrine.taboo);
        let v = self.communities[community].variety;
        if v == sacred {
            return;
        }
        for concept in crate::CONCEPTS {
            if need(concept) != Some(Need::Faith) || taboo == Some(concept.id) {
                continue;
            }
            let mut rng =
                self.at(v)
                    .rng(&[key("doctrine vocabulary"), religion as u64, key(concept.id)]);
            if rng.r#gen::<f32>() >= 0.04 * stance {
                continue;
            }
            let Some(source) = self.varieties[sacred].lexicon.slot(concept).dominant() else {
                continue;
            };
            if self.varieties[v]
                .lexicon
                .slot(concept)
                .variants
                .iter()
                .any(|variant| {
                    self.varieties[v]
                        .lexicon
                        .get(variant.lexeme)
                        .origin
                        .is_loan_from(sacred, source)
                })
            {
                continue;
            }
            let cause = LoanCause::Faith {
                religion,
                teacher: None,
                recipient: community,
            };
            if let Some(word) = self.loan(v, sacred, concept, cause, &mut rng) {
                let slot = self.varieties[v].lexicon.slot_mut(concept);
                if !slot.has(word) {
                    slot.introduce(word, self.params.loan_share);
                }
            }
        }
    }

    /// Called at conversion as well as drift, so authored conversions take
    /// effect without requiring an otherwise unrelated generation to pass.
    pub(crate) fn doctrine_register(&mut self, community: usize, religion: usize) {
        if !self.params.doctrine_enabled
            || !self.religions[religion].scripture
            || !self.communities[community].crafts.contains(&Craft::Writing)
        {
            return;
        }
        let v = self.communities[community].variety;
        if self.religions[religion].translates {
            self.write_vernacular(v, crate::Vernacular::Scripture { religion });
        } else {
            self.write_sacred(v, self.religions[religion].sacred);
        }
    }
}

#[cfg(test)]
mod tests;

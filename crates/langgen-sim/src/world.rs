use crate::adapt::Adapter;
use crate::concepts::{CONCEPTS, Concept, Field, related};
use crate::form::Form;
use crate::laws::{Law, catalog};
use crate::lexicon::{Entry, Event, LexemeId, Origin};
use crate::phonotactics::Phonotactics;
use crate::profile::SoundProfile;
use crate::rng::{key, stream, weighted_index};
use crate::root::mint_one;
use crate::variety::Variety;
use rand::{Rng, RngCore};
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

/// Effective Leipzig–Jakarta rank for cultural vocabulary when it comes to
/// internal replacement.
const CULTURAL_RANK: f32 = 100.0;
/// Weight of no law happening when a sound change is due: about as likely
/// as one common law that neither pleases nor offends the culture.
const NO_CHANGE_WEIGHT: f32 = 1.0;
/// Most words competing for one concept at once.
const MAX_VARIANTS: usize = 3;
/// Floor on a loan's usage fitness, however low its donor's prestige.
const MIN_LOAN_FITNESS: f32 = 0.2;

/// Rates per generation. Defaults are calibrated so an isolated variety
/// keeps about 84% of its core list per 40 generations.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    /// Chance that a new sound law takes hold.
    pub sound_change_rate: f32,
    /// Chance that a mid-ranked core concept gains a new competing word.
    pub innovation_rate: f32,
    /// How many times more often the least stable core concept (rank 100)
    /// gains competitors than the most stable (rank 1).
    pub stability_spread: f32,
    /// Usage share a new native competitor starts with.
    pub newcomer_share: f32,
    /// Uses of each concept sampled per generation. Fewer speakers means
    /// faster random drift, as in small communities.
    pub speakers: u32,
    /// How strongly sound preferences steer which law happens.
    pub preference_pull: f32,
    /// Share of innovations that are new roots even when a related
    /// concept's word could extend instead.
    pub expressive_share: f32,
    /// Extra innovation hazard for a concept whose dominant word sounds
    /// like another word in use: 1.0 doubles it (homonymic clash).
    pub clash_pressure: f32,
    /// Usage disadvantage of a word that sounds like another word in use.
    pub clash_cost: f32,
    /// Chance per generation that a fully borrowable concept is borrowed
    /// across a contact of intensity 1 by a fully open community of equal
    /// prestige.
    pub loan_rate: f32,
    /// Usage share a loan starts with.
    pub loan_share: f32,
    /// How much a prestige gap multiplies borrowing toward the less
    /// prestigious side: the factor is exp(pull × gap).
    pub prestige_pull: f32,
    /// Usage advantage of a loan per unit of prestige its donor holds over
    /// the recipient.
    pub prestige_selection: f32,
    /// Chance a foreign sound survives in a loan, scaled by contact
    /// intensity and openness (a stand-in for bilingualism).
    pub bilingual_keep: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            sound_change_rate: 0.3,
            innovation_rate: 0.003,
            stability_spread: 10.0,
            newcomer_share: 0.25,
            speakers: 12,
            preference_pull: 1.0,
            expressive_share: 0.2,
            clash_pressure: 6.0,
            clash_cost: 0.25,
            loan_rate: 0.05,
            loan_share: 0.3,
            prestige_pull: 2.0,
            prestige_selection: 0.3,
            bilingual_keep: 0.5,
        }
    }
}

/// A group of people with a home variety and a few traits.
#[derive(Clone, Debug, PartialEq)]
pub struct Community {
    pub name: String,
    /// Index into `World::varieties`.
    pub variety: usize,
    /// Social standing relative to others, 0–1. Loans flow mostly downhill.
    pub prestige: f32,
    /// Readiness to accept foreign words, 0–1.
    pub openness: f32,
}

/// What brings two communities together, which shapes what they borrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactKind {
    /// Plain proximity: every semantic field equally exposed.
    Neighbours,
    /// Goods, tools, food, and seafaring.
    Trade,
    /// Government, law, and war.
    Rule,
    /// Belief, ritual, and the vocabulary of thought.
    Religion,
    /// Households, kin, and food.
    Intermarriage,
}

impl ContactKind {
    /// Multiplier on borrowing in `field` for this kind of contact. Hand
    /// estimates of where each kind of contact concentrates.
    pub fn affinity(self, field: Field) -> f32 {
        use ContactKind::*;
        use Field::*;
        match (self, field) {
            (Neighbours, _) => 1.0,
            (Trade, Possession) => 3.0,
            (Trade, FoodDrink | ClothingGrooming | BasicActions) => 2.0,
            (Trade, Motion | Animals) => 1.5,
            (Trade, PhysicalWorld) => 1.2,
            (Trade, _) => 0.6,
            (Rule, Social | Law) => 3.0,
            (Rule, Warfare) => 2.5,
            (Rule, Possession) => 1.5,
            (Rule, _) => 0.7,
            (ContactKind::Religion, Field::Religion) => 4.0,
            (ContactKind::Religion, Speech | Emotions | Cognition) => 1.5,
            (ContactKind::Religion, _) => 0.5,
            (Intermarriage, Kinship) => 3.0,
            (Intermarriage, House | FoodDrink) => 2.0,
            (Intermarriage, ClothingGrooming) => 1.5,
            (Intermarriage, _) => 1.0,
        }
    }
}

/// Ongoing contact between two communities, in both directions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub a: usize,
    pub b: usize,
    /// 0–1: how much the communities deal with each other.
    pub intensity: f32,
    pub kind: ContactKind,
}

/// Communities, their varieties, and the contacts between them, stepping
/// through generations of about 25 years.
#[derive(Clone, Debug)]
pub struct World {
    pub seed: u64,
    pub generation: u32,
    pub communities: Vec<Community>,
    pub varieties: Vec<Variety>,
    pub contacts: Vec<Contact>,
    pub params: Params,
    laws: Vec<Law>,
}

/// Per-variety random streams for one generation.
struct Step {
    seed: u64,
    generation: u32,
    variety: usize,
}

impl Step {
    fn rng(&self, labels: &[u64]) -> ChaCha8Rng {
        let mut keys = vec![
            key("step"),
            u64::from(self.generation),
            key("variety"),
            self.variety as u64,
        ];
        keys.extend_from_slice(labels);
        stream(self.seed, &keys)
    }
}

impl World {
    pub fn new(seed: u64, params: Params) -> Self {
        Self {
            seed,
            generation: 0,
            communities: Vec::new(),
            varieties: Vec::new(),
            contacts: Vec::new(),
            params,
            laws: catalog(),
        }
    }

    /// A world with one community and its variety, for studying drift alone.
    pub fn solo(seed: u64, profile: &SoundProfile, params: Params) -> Self {
        let mut world = Self::new(seed, params);
        world.found("Solo", profile, 0.5, 0.5);
        world
    }

    /// Founds a community with a new variety of its own; returns its index.
    pub fn found(
        &mut self,
        name: &str,
        profile: &SoundProfile,
        prestige: f32,
        openness: f32,
    ) -> usize {
        let index = self.communities.len();
        let variety_seed = stream(self.seed, &[key("found"), index as u64]).next_u64();
        self.varieties.push(Variety::found(variety_seed, profile));
        self.communities.push(Community {
            name: name.into(),
            variety: self.varieties.len() - 1,
            prestige: prestige.clamp(0.0, 1.0),
            openness: openness.clamp(0.0, 1.0),
        });
        index
    }

    pub fn connect(&mut self, a: usize, b: usize, intensity: f32, kind: ContactKind) {
        self.contacts.push(Contact {
            a,
            b,
            intensity: intensity.clamp(0.0, 1.0),
            kind,
        });
    }

    /// The home variety of `community`.
    pub fn variety_of(&self, community: usize) -> &Variety {
        &self.varieties[self.communities[community].variety]
    }

    pub fn run(&mut self, generations: u32) {
        for _ in 0..generations {
            self.step();
        }
    }

    pub fn step(&mut self) {
        self.generation += 1;
        for v in 0..self.varieties.len() {
            self.sound_change(v);
        }
        self.borrow();
        let prestige = self.variety_prestige();
        for v in 0..self.varieties.len() {
            let clashes = self.varieties[v].lexicon.clashing();
            self.innovate(v, &clashes);
            self.drift(v, &clashes, &prestige);
            self.retire(v);
        }
    }

    fn at(&self, variety: usize) -> Step {
        Step {
            seed: self.seed,
            generation: self.generation,
            variety,
        }
    }

    /// Prestige of each variety: the highest among communities at home in it.
    fn variety_prestige(&self) -> Vec<f32> {
        let mut out = vec![0.0_f32; self.varieties.len()];
        for c in &self.communities {
            out[c.variety] = out[c.variety].max(c.prestige);
        }
        out
    }

    /// Per-generation chance that `concept` gains a native competitor.
    pub fn innovation_hazard(&self, concept: &Concept) -> f32 {
        let rank = concept.stability.map_or(CULTURAL_RANK, f32::from);
        self.params.innovation_rate * self.params.stability_spread.powf((rank - 50.5) / 99.0)
    }

    /// Maybe applies one new sound law to every living word. Laws that
    /// would change nothing are skipped; laws that move sounds toward the
    /// culture's preferences are likelier.
    fn sound_change(&mut self, v: usize) {
        let mut rng = self.at(v).rng(&[key("sound")]);
        if rng.r#gen::<f32>() >= self.params.sound_change_rate {
            return;
        }
        let variety = &self.varieties[v];
        let applied: HashSet<&str> = variety.laws.iter().map(|(_, id)| *id).collect();
        let prior = &variety.profile.inventory;
        let candidates: Vec<(&Law, f32)> = self
            .laws
            .iter()
            .filter(|law| !applied.contains(law.id))
            .filter_map(|law| {
                let a = law.assess(variety.lexicon.living().map(|l| &l.form), prior)?;
                let bias = (self.params.preference_pull * a.pull.clamp(-5.0, 3.0)).exp();
                Some((law, law.commonness * bias))
            })
            .collect();
        // "Nothing happens" competes like a common, neutral law, so a
        // culture is not forced into a change it strongly disprefers.
        let weights = candidates.iter().map(|(_, w)| *w).chain([NO_CHANGE_WEIGHT]);
        let Some((law, _)) = candidates.get(weighted_index(&mut rng, weights)) else {
            return;
        };
        let law = (*law).clone();

        let generation = self.generation;
        let variety = &mut self.varieties[v];
        for lexeme in &mut variety.lexicon.lexemes {
            if lexeme.obsolete.is_some() {
                continue;
            }
            let after = law.apply(&lexeme.form);
            if after != lexeme.form {
                let before = std::mem::replace(&mut lexeme.form, after);
                lexeme.log.push(Entry {
                    generation,
                    event: Event::SoundLaw {
                        law: law.id,
                        before,
                    },
                });
            }
        }
        variety.laws.push((generation, law.id));
    }

    /// Each contact carries words both ways, mostly from the more
    /// prestigious side. A concept's chance of being borrowed scales with
    /// intensity, the recipient's openness, the prestige gap, the contact's
    /// affinity for its field, and its own borrowability. The donor's
    /// current word is adapted to the recipient's sounds and enters as a
    /// competitor; all loans are decided against this generation's state.
    fn borrow(&mut self) {
        struct Loan {
            recipient: usize,
            concept: usize,
            from: usize,
            source: LexemeId,
            source_form: Form,
            form: Form,
        }
        let mut loans = Vec::new();
        let mut adapters: Vec<Option<Adapter>> = vec![None; self.varieties.len()];
        for contact in &self.contacts {
            for (donor, recipient) in [(contact.a, contact.b), (contact.b, contact.a)] {
                let (d, r) = (&self.communities[donor], &self.communities[recipient]);
                if d.variety == r.variety {
                    continue;
                }
                let base = self.params.loan_rate
                    * contact.intensity
                    * r.openness
                    * (self.params.prestige_pull * (d.prestige - r.prestige)).exp();
                let keep_foreign = self.params.bilingual_keep * contact.intensity * r.openness;
                let donor_lexicon = &self.varieties[d.variety].lexicon;
                let recipient_lexicon = &self.varieties[r.variety].lexicon;
                for (i, concept) in CONCEPTS.iter().enumerate() {
                    let hazard =
                        base * contact.kind.affinity(concept.field) * concept.borrowability();
                    let mut rng =
                        self.at(r.variety)
                            .rng(&[key("borrow"), d.variety as u64, key(concept.id)]);
                    if rng.r#gen::<f32>() >= hazard {
                        continue;
                    }
                    let Some(source) = donor_lexicon.slots[i].dominant() else {
                        continue;
                    };
                    let already = recipient_lexicon.slots[i].variants.iter().any(|v| {
                        recipient_lexicon.get(v.lexeme).origin
                            == Origin::Borrowed {
                                from: d.variety,
                                source,
                            }
                    });
                    if already {
                        continue;
                    }
                    let adapter = adapters[r.variety].get_or_insert_with(|| {
                        Adapter::new(
                            recipient_lexicon.living().map(|l| &l.form),
                            &self.varieties[r.variety].profile.inventory,
                        )
                    });
                    let source_form = donor_lexicon.get(source).form.clone();
                    let form = adapter.adapt(&source_form, keep_foreign, &mut rng);
                    loans.push(Loan {
                        recipient: r.variety,
                        concept: i,
                        from: d.variety,
                        source,
                        source_form,
                        form,
                    });
                }
            }
        }
        let generation = self.generation;
        for loan in loans {
            let lexicon = &mut self.varieties[loan.recipient].lexicon;
            if lexicon.slots[loan.concept].variants.len() >= MAX_VARIANTS {
                continue;
            }
            let concept = &CONCEPTS[loan.concept];
            let origin = Origin::Borrowed {
                from: loan.from,
                source: loan.source,
            };
            // A donor word already borrowed for another sense is the same
            // loanword gaining a meaning, not a second borrowing.
            let existing = lexicon.living().find(|l| l.origin == origin).map(|l| l.id);
            let id = match existing {
                Some(id) => {
                    lexicon.get_mut(id).log.push(Entry {
                        generation,
                        event: Event::Extended { to: concept },
                    });
                    id
                }
                None => {
                    let id = lexicon.coin(loan.form, origin, concept, generation);
                    lexicon.get_mut(id).log.push(Entry {
                        generation,
                        event: Event::Borrowed {
                            from: loan.from,
                            source: loan.source_form,
                        },
                    });
                    id
                }
            };
            lexicon.slots[loan.concept].introduce(id, self.params.loan_share);
        }
    }

    /// Concepts gain native competitors: usually a word for a related
    /// concept extends to cover it (sun > day, see > know), otherwise a new
    /// root is coined from the language's current sounds, avoiding forms
    /// its semantic field already uses.
    fn innovate(&mut self, v: usize, clashes: &HashSet<LexemeId>) {
        let generation = self.generation;
        let step = self.at(v);
        let hazards: Vec<f32> = CONCEPTS.iter().map(|c| self.innovation_hazard(c)).collect();
        let params = &self.params;
        let lexicon = &mut self.varieties[v].lexicon;
        let observed = Phonotactics::observe(lexicon.living().map(|l| &l.form));
        for (i, concept) in CONCEPTS.iter().enumerate() {
            let mut rng = step.rng(&[key("innovate"), key(concept.id)]);
            let clash = lexicon.slots[i]
                .dominant()
                .is_some_and(|id| clashes.contains(&id));
            let pressure = if clash {
                1.0 + params.clash_pressure
            } else {
                1.0
            };
            if rng.r#gen::<f32>() >= hazards[i] * pressure {
                continue;
            }
            if lexicon.slots[i].variants.len() >= MAX_VARIANTS {
                continue;
            }
            let mut donors: Vec<LexemeId> = related(concept)
                .filter_map(|other| lexicon.slot(other).dominant())
                .filter(|id| !lexicon.slots[i].has(*id))
                .collect();
            donors.sort();
            donors.dedup();

            let newcomer = if !donors.is_empty() && rng.r#gen::<f32>() >= params.expressive_share {
                let id = donors[rng.gen_range(0..donors.len())];
                lexicon.get_mut(id).log.push(Entry {
                    generation,
                    event: Event::Extended { to: concept },
                });
                id
            } else {
                let used: HashSet<Form> = lexicon.living().map(|l| l.form.clone()).collect();
                let field: HashSet<Form> = lexicon
                    .slots
                    .iter()
                    .filter(|s| s.concept.field == concept.field)
                    .flat_map(|s| s.variants.iter())
                    .map(|v| lexicon.get(v.lexeme).form.clone())
                    .collect();
                let form = mint_one(&mut rng, &observed, concept, &used, &field);
                lexicon.coin(form, Origin::Expressive, concept, generation)
            };
            lexicon.slots[i].introduce(newcomer, params.newcomer_share);
        }
    }

    /// Usage shares drift by resampling each contested concept's uses.
    /// Words that sound like other words in use pay a small cost; loans
    /// gain or lose by their donor's prestige relative to the recipient's.
    /// A word that draws no uses loses that concept.
    fn drift(&mut self, v: usize, clashes: &HashSet<LexemeId>, prestige: &[f32]) {
        let generation = self.generation;
        let step = self.at(v);
        let params = &self.params;
        let n = params.speakers.max(1);
        let own = prestige[v];
        let lexicon = &mut self.varieties[v].lexicon;
        let fitness: Vec<f32> = lexicon
            .lexemes
            .iter()
            .map(|l| {
                let clash = if clashes.contains(&l.id) {
                    1.0 - params.clash_cost
                } else {
                    1.0
                };
                let loan = match l.origin {
                    Origin::Borrowed { from, .. } => (1.0
                        + params.prestige_selection * (prestige[from] - own))
                        .max(MIN_LOAN_FITNESS),
                    _ => 1.0,
                };
                clash * loan
            })
            .collect();
        for (i, concept) in CONCEPTS.iter().enumerate() {
            if lexicon.slots[i].variants.len() < 2 {
                continue;
            }
            let mut rng = step.rng(&[key("drift"), key(concept.id)]);
            let slot = &mut lexicon.slots[i];
            let mut counts = vec![0u32; slot.variants.len()];
            for _ in 0..n {
                let weights = slot
                    .variants
                    .iter()
                    .map(|var| var.weight * fitness[var.lexeme.0 as usize]);
                counts[weighted_index(&mut rng, weights)] += 1;
            }
            for (variant, count) in slot.variants.iter_mut().zip(counts) {
                variant.weight = count as f32 / n as f32;
            }
            let lost: Vec<LexemeId> = slot
                .variants
                .iter()
                .filter(|var| var.weight == 0.0)
                .map(|var| var.lexeme)
                .collect();
            slot.variants.retain(|var| var.weight > 0.0);
            for id in lost {
                lexicon.get_mut(id).log.push(Entry {
                    generation,
                    event: Event::Lost { sense: concept },
                });
            }
        }
    }

    /// Words no longer used for any concept become obsolete. They keep
    /// their history but stop undergoing sound change.
    fn retire(&mut self, v: usize) {
        let generation = self.generation;
        let lexicon = &mut self.varieties[v].lexicon;
        let used: HashSet<LexemeId> = lexicon
            .slots
            .iter()
            .flat_map(|s| s.variants.iter().map(|var| var.lexeme))
            .collect();
        for lexeme in &mut lexicon.lexemes {
            if lexeme.obsolete.is_none() && !used.contains(&lexeme.id) {
                lexeme.obsolete = Some(generation);
                lexeme.log.push(Entry {
                    generation,
                    event: Event::Obsolete,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_are_reproducible() {
        let profile = SoundProfile::by_id("elvish").unwrap();
        let mut a = World::solo(7, &profile, Params::default());
        let mut b = World::solo(7, &profile, Params::default());
        a.run(40);
        b.run(40);
        assert_eq!(a.varieties[0].lexicon, b.varieties[0].lexicon);
        assert_eq!(a.varieties[0].laws, b.varieties[0].laws);
    }

    #[test]
    fn slots_stay_normalized_and_words_keep_a_vowel() {
        let profile = SoundProfile::by_id("kuo-toa").unwrap();
        let mut sim = World::solo(3, &profile, Params::default());
        sim.run(80);
        for slot in &sim.varieties[0].lexicon.slots {
            let total: f32 = slot.variants.iter().map(|v| v.weight).sum();
            assert!(
                (total - 1.0).abs() < 1e-4,
                "{} sums to {total}",
                slot.concept.id
            );
        }
        for lexeme in sim.varieties[0].lexicon.living() {
            assert!(
                lexeme.form.vowel_count() > 0,
                "{} lost its vowels",
                lexeme.form.ipa()
            );
        }
    }

    /// Glottochronology's classic estimate is roughly 80–90% retention of
    /// the 100-item core list per millennium (about 40 generations), with
    /// the most stable meanings kept far more often than the least.
    #[test]
    fn core_retention_per_millennium_matches_glottochronology() {
        let seeds = 60;
        let (mut total, mut top, mut bottom) = (0.0, 0.0, 0.0);
        for (n, profile) in SoundProfile::examples()
            .iter()
            .cycle()
            .take(seeds)
            .enumerate()
        {
            let mut sim = World::solo(n as u64, profile, Params::default());
            sim.run(40);
            let lexicon = &sim.varieties[0].lexicon;
            total += lexicon.core_retention();
            let kept = |lo: u8, hi: u8| {
                lexicon
                    .slots
                    .iter()
                    .filter(|s| s.concept.stability.is_some_and(|r| (lo..=hi).contains(&r)))
                    .filter(|s| lexicon.keeps_founding_word(s.concept))
                    .count() as f32
                    / 20.0
            };
            top += kept(1, 20);
            bottom += kept(81, 100);
        }
        let (mean, top, bottom) = (
            total / seeds as f32,
            top / seeds as f32,
            bottom / seeds as f32,
        );
        assert!((0.80..=0.90).contains(&mean), "mean retention {mean:.3}");
        assert!(
            top > bottom + 0.1,
            "ranks 1–20 kept {top:.2}, ranks 81–100 {bottom:.2}"
        );
    }

    /// Sound preferences steer which laws a language undergoes: a culture
    /// that likes fricatives spirantizes far more often, and one that
    /// dislikes labiodental fricatives rarely turns w into v.
    #[test]
    fn preferences_steer_sound_change() {
        let rate = |profile: &SoundProfile, law: &str| {
            let seeds = 120;
            let hits = (0..seeds)
                .filter(|&seed| {
                    let mut sim = World::solo(seed, profile, Params::default());
                    sim.run(40);
                    sim.varieties[0].laws.iter().any(|(_, id)| *id == law)
                })
                .count();
            hits as f32 / seeds as f32
        };
        let neutral = SoundProfile::by_id("neutral").unwrap();
        let illithid = SoundProfile::by_id("illithid").unwrap();
        let fishy = neutral.flavored(&crate::flavor::Flavor::by_id("fish-mouthed").unwrap());
        let (spir_neutral, spir_illithid) = (
            rate(&neutral, "spirantization"),
            rate(&illithid, "spirantization"),
        );
        assert!(
            spir_illithid > 3.0 * spir_neutral + 0.2,
            "{spir_illithid} vs {spir_neutral}"
        );
        let (w_neutral, w_fishy) = (rate(&neutral, "w-fortition"), rate(&fishy, "w-fortition"));
        assert!(w_fishy < 0.5 * w_neutral, "{w_fishy} vs {w_neutral}");
    }

    /// A prestigious donor and an open recipient in contact for 40
    /// generations.
    fn pair(seed: u64, kind: ContactKind) -> World {
        let mut world = World::new(seed, Params::default());
        let d = world.found("Donor", &SoundProfile::by_id("illithid").unwrap(), 0.8, 0.4);
        let r = world.found(
            "Recipient",
            &SoundProfile::by_id("kuo-toa").unwrap(),
            0.3,
            0.7,
        );
        world.connect(d, r, 0.8, kind);
        world.run(40);
        world
    }

    fn is_loan(world: &World, community: usize, concept: &str) -> bool {
        let lexicon = &world.variety_of(community).lexicon;
        let concept = crate::concepts::by_id(concept).unwrap();
        lexicon
            .word_for(concept)
            .is_some_and(|l| matches!(l.origin, Origin::Borrowed { .. }))
    }

    /// Mean share of each field's concepts that the recipient now
    /// expresses with a loan.
    fn field_shares(worlds: &[World]) -> Vec<(Field, f32)> {
        let mut out: Vec<(Field, f32)> = Vec::new();
        for &(field, _) in crate::wold::BORROWED_SCORE {
            let ids: Vec<&str> = CONCEPTS
                .iter()
                .filter(|c| c.field == field)
                .map(|c| c.id)
                .collect();
            let loans: usize = worlds
                .iter()
                .map(|w| ids.iter().filter(|id| is_loan(w, 1, id)).count())
                .sum();
            out.push((field, loans as f32 / (ids.len() * worlds.len()) as f32));
        }
        out
    }

    /// The milestone check: with no field favoured by the kind of contact,
    /// loan shares by field rank roughly as WOLD's do. Borrowability is set
    /// per concept, so this ranking has to emerge from what each field
    /// contains.
    #[test]
    fn loan_shares_by_field_rank_like_wold() {
        let worlds: Vec<World> = (0..40).map(|s| pair(s, ContactKind::Neighbours)).collect();
        let shares = field_shares(&worlds);
        let pairs: Vec<(f32, f32)> = shares
            .iter()
            .zip(crate::wold::BORROWED_SCORE)
            .map(|((_, sim), (_, wold))| (*sim, *wold))
            .collect();
        let rho = crate::wold::spearman(&pairs);
        assert!(rho >= 0.5, "Spearman {rho:.2} against WOLD: {shares:?}");
        let overall: f32 = shares.iter().map(|(_, s)| s).sum::<f32>() / shares.len() as f32;
        assert!(
            (0.1..=0.45).contains(&overall),
            "overall field share {overall:.2}"
        );
    }

    #[test]
    fn core_vocabulary_stays_mostly_native() {
        let worlds: Vec<World> = (0..40).map(|s| pair(s, ContactKind::Neighbours)).collect();
        let pronouns = worlds
            .iter()
            .flat_map(|w| ["1sg", "2sg", "3sg"].map(|p| is_loan(w, 1, p)))
            .filter(|&loan| loan)
            .count() as f32
            / (3 * worlds.len()) as f32;
        assert!(pronouns < 0.05, "pronouns borrowed {pronouns:.3}");
        let shares = field_shares(&worlds);
        let share = |f: Field| shares.iter().find(|(x, _)| *x == f).unwrap().1;
        assert!(
            share(Field::Body) < 0.5 * share(Field::Religion),
            "{shares:?}"
        );
    }

    #[test]
    fn loans_flow_toward_the_less_prestigious_side() {
        let (mut down, mut up) = (0, 0);
        for seed in 0..20 {
            let world = pair(seed, ContactKind::Neighbours);
            let count = |c: usize| CONCEPTS.iter().filter(|k| is_loan(&world, c, k.id)).count();
            down += count(1);
            up += count(0);
        }
        assert!(
            down > 5 * up.max(1),
            "recipient took {down}, donor took {up}"
        );
    }

    #[test]
    fn contact_kinds_concentrate_borrowing() {
        let share = |kind: ContactKind, field: Field| {
            let worlds: Vec<World> = (0..20).map(|s| pair(s, kind)).collect();
            field_shares(&worlds)
                .into_iter()
                .find(|(f, _)| *f == field)
                .unwrap()
                .1
        };
        assert!(
            share(ContactKind::Religion, Field::Religion)
                > share(ContactKind::Neighbours, Field::Religion) + 0.1
        );
        assert!(
            share(ContactKind::Trade, Field::Possession)
                > share(ContactKind::Trade, Field::Body) + 0.2
        );
    }

    #[test]
    fn loans_record_their_source_and_later_history() {
        let world = pair(5, ContactKind::Neighbours);
        let recipient = world.variety_of(1);
        let donor = world.communities[0].variety;
        let loans: Vec<_> = recipient
            .lexicon
            .lexemes
            .iter()
            .filter(|l| matches!(l.origin, Origin::Borrowed { .. }))
            .collect();
        assert!(!loans.is_empty());
        for loan in loans {
            let Origin::Borrowed { from, source } = loan.origin else {
                unreachable!()
            };
            assert_eq!(from, donor);
            assert!(world.varieties[from].lexicon.lexemes.len() > source.0 as usize);
            assert!(
                matches!(loan.log.first().map(|e| &e.event), Some(Event::Borrowed { from: f, .. }) if *f == donor),
                "a loan's history starts with its borrowing"
            );
            // Sound laws can only follow the borrowing, never precede it.
            for entry in &loan.log {
                assert!(entry.generation >= loan.born);
            }
        }
        // One donor word is borrowed once, however many senses it brings.
        let mut sources: Vec<_> = recipient
            .lexicon
            .living()
            .filter_map(|l| match l.origin {
                Origin::Borrowed { from, source } => Some((from, source)),
                _ => None,
            })
            .collect();
        let n = sources.len();
        sources.sort();
        sources.dedup();
        assert_eq!(sources.len(), n);
    }
}

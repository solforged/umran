use crate::concepts::{CONCEPTS, Concept, related};
use crate::form::Form;
use crate::laws::{Law, catalog};
use crate::lexicon::{Entry, Event, LexemeId, Origin};
use crate::phonotactics::Phonotactics;
use crate::profile::SoundProfile;
use crate::rng::{key, stream, weighted_index};
use crate::root::mint_one;
use crate::variety::Variety;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

/// Effective Leipzig–Jakarta rank for cultural vocabulary, which is more
/// exposed to replacement than even the least stable core meaning.
const CULTURAL_RANK: f32 = 100.0;
/// Weight of no law happening when a sound change is due: about as likely
/// as one common law that neither pleases nor offends the culture.
const NO_CHANGE_WEIGHT: f32 = 1.0;
/// Most words competing for one concept at once.
const MAX_VARIANTS: usize = 3;

/// Rates for one variety changing on its own, per generation.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    /// Chance that a new sound law takes hold.
    pub sound_change_rate: f32,
    /// Chance that a mid-ranked core concept gains a new competing word.
    pub innovation_rate: f32,
    /// How many times more often the least stable core concept (rank 100)
    /// gains competitors than the most stable (rank 1).
    pub stability_spread: f32,
    /// Usage share a new competitor starts with.
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
    /// Usage disadvantage of a word that sounds like another word in use,
    /// as a fraction of its share each generation.
    pub clash_cost: f32,
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
        }
    }
}

/// One variety stepping through generations of about 25 years.
#[derive(Clone, Debug)]
pub struct Sim {
    pub seed: u64,
    pub generation: u32,
    pub variety: Variety,
    pub params: Params,
    laws: Vec<Law>,
}

impl Sim {
    pub fn new(seed: u64, profile: &SoundProfile, params: Params) -> Self {
        Self {
            seed,
            generation: 0,
            variety: Variety::found(seed, profile),
            params,
            laws: catalog(),
        }
    }

    pub fn run(&mut self, generations: u32) {
        for _ in 0..generations {
            self.step();
        }
    }

    pub fn step(&mut self) {
        self.generation += 1;
        self.sound_change();
        let clashes = self.variety.lexicon.clashing();
        self.innovate(&clashes);
        self.drift(&clashes);
        self.retire();
    }

    fn rng(&self, labels: &[u64]) -> ChaCha8Rng {
        let mut keys = vec![key("step"), u64::from(self.generation)];
        keys.extend_from_slice(labels);
        stream(self.seed, &keys)
    }

    /// Per-generation chance that `concept` gains a competitor.
    pub fn innovation_hazard(&self, concept: &Concept) -> f32 {
        let rank = concept.stability.map_or(CULTURAL_RANK, f32::from);
        self.params.innovation_rate * self.params.stability_spread.powf((rank - 50.5) / 99.0)
    }

    /// Maybe applies one new sound law to every living word. Laws that
    /// would change nothing are skipped; laws that move sounds toward the
    /// culture's preferences are likelier.
    fn sound_change(&mut self) {
        let mut rng = self.rng(&[key("sound")]);
        if rng.r#gen::<f32>() >= self.params.sound_change_rate {
            return;
        }
        let applied: HashSet<&str> = self.variety.laws.iter().map(|(_, id)| *id).collect();
        let prior = &self.variety.profile.inventory;
        let lexicon = &self.variety.lexicon;
        let candidates: Vec<(&Law, f32)> = self
            .laws
            .iter()
            .filter(|law| !applied.contains(law.id))
            .filter_map(|law| {
                let a = law.assess(lexicon.living().map(|l| &l.form), prior)?;
                let bias = (self.params.preference_pull * a.pull.clamp(-3.0, 3.0)).exp();
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
        for lexeme in &mut self.variety.lexicon.lexemes {
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
        self.variety.laws.push((generation, law.id));
    }

    /// Concepts gain competing words: usually a word for a related concept
    /// extends to cover it (sun > day, see > know), otherwise a new root is
    /// coined from the language's current sounds, avoiding forms its
    /// semantic field already uses.
    fn innovate(&mut self, clashes: &HashSet<LexemeId>) {
        let generation = self.generation;
        let observed = Phonotactics::observe(self.variety.lexicon.living().map(|l| &l.form));
        for (i, concept) in CONCEPTS.iter().enumerate() {
            let mut rng = self.rng(&[key("innovate"), key(concept.id)]);
            let clash = self.variety.lexicon.slots[i]
                .dominant()
                .is_some_and(|id| clashes.contains(&id));
            let pressure = if clash {
                1.0 + self.params.clash_pressure
            } else {
                1.0
            };
            if rng.r#gen::<f32>() >= self.innovation_hazard(concept) * pressure {
                continue;
            }
            let lexicon = &mut self.variety.lexicon;
            if lexicon.slots[i].variants.len() >= MAX_VARIANTS {
                continue;
            }
            let mut donors: Vec<LexemeId> = related(concept)
                .filter_map(|other| lexicon.slot(other).dominant())
                .filter(|id| !lexicon.slots[i].has(*id))
                .collect();
            donors.sort();
            donors.dedup();

            let newcomer =
                if !donors.is_empty() && rng.r#gen::<f32>() >= self.params.expressive_share {
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
            lexicon.slots[i].introduce(newcomer, self.params.newcomer_share);
        }
    }

    /// Usage shares drift by resampling each contested concept's uses, with
    /// a small cost for words that sound like other words in use. A word
    /// that draws no uses loses that concept.
    fn drift(&mut self, clashes: &HashSet<LexemeId>) {
        let cost = 1.0 - self.params.clash_cost;
        let generation = self.generation;
        let n = self.params.speakers.max(1);
        for (i, concept) in CONCEPTS.iter().enumerate() {
            if self.variety.lexicon.slots[i].variants.len() < 2 {
                continue;
            }
            let mut rng = self.rng(&[key("drift"), key(concept.id)]);
            let slot = &mut self.variety.lexicon.slots[i];
            let mut counts = vec![0u32; slot.variants.len()];
            for _ in 0..n {
                let fitness = slot.variants.iter().map(|v| {
                    if clashes.contains(&v.lexeme) {
                        v.weight * cost
                    } else {
                        v.weight
                    }
                });
                counts[weighted_index(&mut rng, fitness)] += 1;
            }
            for (variant, count) in slot.variants.iter_mut().zip(counts) {
                variant.weight = count as f32 / n as f32;
            }
            let lost: Vec<LexemeId> = slot
                .variants
                .iter()
                .filter(|v| v.weight == 0.0)
                .map(|v| v.lexeme)
                .collect();
            slot.variants.retain(|v| v.weight > 0.0);
            for id in lost {
                self.variety.lexicon.get_mut(id).log.push(Entry {
                    generation,
                    event: Event::Lost { sense: concept },
                });
            }
        }
    }

    /// Words no longer used for any concept become obsolete. They keep
    /// their history but stop undergoing sound change.
    fn retire(&mut self) {
        let generation = self.generation;
        let lexicon = &mut self.variety.lexicon;
        let used: HashSet<LexemeId> = lexicon
            .slots
            .iter()
            .flat_map(|s| s.variants.iter().map(|v| v.lexeme))
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
        let mut a = Sim::new(7, &profile, Params::default());
        let mut b = Sim::new(7, &profile, Params::default());
        a.run(40);
        b.run(40);
        assert_eq!(a.variety.lexicon, b.variety.lexicon);
        assert_eq!(a.variety.laws, b.variety.laws);
    }

    #[test]
    fn slots_stay_normalized_and_words_keep_a_vowel() {
        let profile = SoundProfile::by_id("kuo-toa").unwrap();
        let mut sim = Sim::new(3, &profile, Params::default());
        sim.run(80);
        for slot in &sim.variety.lexicon.slots {
            let total: f32 = slot.variants.iter().map(|v| v.weight).sum();
            assert!(
                (total - 1.0).abs() < 1e-4,
                "{} sums to {total}",
                slot.concept.id
            );
        }
        for lexeme in sim.variety.lexicon.living() {
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
            let mut sim = Sim::new(n as u64, profile, Params::default());
            sim.run(40);
            let lexicon = &sim.variety.lexicon;
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
                    let mut sim = Sim::new(seed, profile, Params::default());
                    sim.run(40);
                    sim.variety.laws.iter().any(|(_, id)| *id == law)
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
}

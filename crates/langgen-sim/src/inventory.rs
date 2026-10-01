use crate::phoneme::{CATALOG, PhonemeId, Segment};
use crate::profile::InventoryPrior;
use crate::rng::weighted_index;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Segments below this preference score are never sampled.
const MIN_SCORE: f32 = 0.45;
/// Weight floor so a required but dispreferred segment still gets used.
const MIN_WEIGHT: f32 = 0.08;
/// Score for a manner or height a prior does not list.
pub(crate) const UNLISTED_PRIMARY: f32 = -0.9;
/// Score for a place or backness a prior does not list.
pub(crate) const UNLISTED_SECONDARY: f32 = -0.5;

/// The segments a variety uses, each with a usage weight from its profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub consonants: Vec<PhonemeId>,
    pub vowels: Vec<PhonemeId>,
    weights: Vec<(PhonemeId, f32)>,
}

impl Inventory {
    pub fn sample(prior: &InventoryPrior, rng: &mut impl Rng) -> Self {
        let mut consonants = pick_class(prior, false, prior.consonant_count, rng);
        let mut vowels = pick_class(prior, true, prior.vowel_count, rng);
        repair_universals(&mut consonants, &mut vowels);

        consonants.sort_by_key(|id| CATALOG.get(*id).ipa());
        vowels.sort_by_key(|id| CATALOG.get(*id).ipa());
        let weights = consonants
            .iter()
            .chain(&vowels)
            .map(|&id| (id, score(prior, CATALOG.get(id)).max(MIN_WEIGHT)))
            .collect();
        Self {
            consonants,
            vowels,
            weights,
        }
    }

    pub fn contains(&self, id: PhonemeId) -> bool {
        self.consonants.contains(&id) || self.vowels.contains(&id)
    }

    pub fn weight(&self, id: PhonemeId) -> f32 {
        self.weights
            .iter()
            .find(|(x, _)| *x == id)
            .map_or(MIN_WEIGHT, |(_, w)| *w)
    }
}

/// Required segments of one class, topped up by weighted sampling to a
/// size drawn from `lo..=hi`.
fn pick_class(
    prior: &InventoryPrior,
    vowels: bool,
    (lo, hi): (u8, u8),
    rng: &mut impl Rng,
) -> Vec<PhonemeId> {
    let allowed = |id: PhonemeId| {
        CATALOG.get(id).is_vowel() == vowels
            && !prior.forbidden.iter().any(|f| f == CATALOG.get(id).ipa())
    };
    let mut chosen: Vec<PhonemeId> = prior
        .required
        .iter()
        .filter_map(|ipa| CATALOG.id_by_ipa(ipa))
        .filter(|&id| allowed(id))
        .collect();
    let mut pool: Vec<(PhonemeId, f32)> = (0..CATALOG.segments.len())
        .map(|i| PhonemeId(i as u16))
        .filter(|&id| allowed(id) && !chosen.contains(&id))
        .map(|id| (id, score(prior, CATALOG.get(id))))
        .filter(|&(_, s)| s > MIN_SCORE)
        .collect();
    let target = (rng.gen_range(lo..=hi) as usize).max(chosen.len());
    while chosen.len() < target && !pool.is_empty() {
        let i = weighted_index(rng, pool.iter().map(|(_, w)| *w));
        chosen.push(pool.swap_remove(i).0);
    }
    chosen
}

/// Preference score: base plus feature weights, minus penalties for
/// features the prior does not mention.
pub fn score(prior: &InventoryPrior, seg: Segment) -> f32 {
    fn lookup<T: PartialEq>(list: &[(T, f32)], key: T, missing: f32) -> f32 {
        list.iter()
            .find(|(k, _)| *k == key)
            .map_or(missing, |(_, w)| *w)
    }
    let mut s = 0.12;
    match seg {
        Segment::Consonant(c) => {
            s += lookup(&prior.manner, c.manner, UNLISTED_PRIMARY);
            s += lookup(&prior.place, c.place, UNLISTED_SECONDARY);
            s += if c.voiced {
                prior.voiced
            } else {
                -prior.voiced
            };
        }
        Segment::Vowel(v) => {
            s += lookup(&prior.height, v.height, UNLISTED_PRIMARY);
            s += lookup(&prior.backness, v.backness, UNLISTED_SECONDARY);
            s += if v.rounded {
                prior.rounded
            } else {
                -prior.rounded
            };
        }
    }
    s + prior
        .extra
        .iter()
        .find(|(ipa, _)| ipa == seg.ipa())
        .map_or(0.0, |(_, w)| *w)
}

/// Voiced stops imply their voiceless partners and marked nasals imply /n/,
/// as they nearly always do in attested inventories.
fn repair_universals(consonants: &mut Vec<PhonemeId>, vowels: &mut Vec<PhonemeId>) {
    let has = |set: &[PhonemeId], ipa: &str| set.iter().any(|id| CATALOG.get(*id).ipa() == ipa);
    let add = |set: &mut Vec<PhonemeId>, ipa: &str| {
        let id = CATALOG.id_by_ipa(ipa).expect("catalog segment");
        if !set.contains(&id) {
            set.push(id);
        }
    };
    for (marked, base) in [("g", "k"), ("d", "t"), ("b", "p"), ("ŋ", "n"), ("ɲ", "n")] {
        if has(consonants, marked) {
            add(consonants, base);
        }
    }
    if vowels.is_empty() {
        for v in ["a", "i", "u"] {
            add(vowels, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::SoundProfile;
    use crate::rng::stream;

    #[test]
    fn respects_required_forbidden_and_counts() {
        for profile in SoundProfile::examples() {
            let prior = &profile.inventory;
            for seed in 0..200 {
                let inv = Inventory::sample(prior, &mut stream(seed, &[]));
                let ipa = |id: &PhonemeId| CATALOG.get(*id).ipa().to_string();
                let all: Vec<String> = inv.consonants.iter().chain(&inv.vowels).map(ipa).collect();
                for f in &prior.forbidden {
                    assert!(
                        !all.contains(f),
                        "{} seed {seed} has forbidden {f}",
                        profile.id
                    );
                }
                for r in &prior.required {
                    assert!(
                        all.contains(r),
                        "{} seed {seed} lacks required {r}",
                        profile.id
                    );
                }
                assert!(inv.vowels.len() >= prior.vowel_count.0 as usize);
                assert!(inv.consonants.len() >= prior.consonant_count.0 as usize);
            }
        }
    }
}

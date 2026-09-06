mod aesthetic;
mod generate;
mod inventory;
mod ortho;
mod phoneme;

pub use aesthetic::{Aesthetic, ClusterPolicy, InventoryPrior, LongVowel, NameStyle, OrthoStyle, Signature, WordShape};
pub use generate::{Generator, NameKind, Syllable, Word};
pub use inventory::Inventory;
pub use phoneme::{Catalog, PhonemeId, Segment, CATALOG};

use generate::NameKind as NK;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct Language {
    pub seed: u64,
    pub aesthetic: Aesthetic,
    pub inventory: Inventory,
    pub generator: Generator,
    pub autonym: String,
}

#[derive(Serialize)]
pub struct Snapshot {
    pub seed: u64,
    pub aesthetic: String,
    pub autonym: String,
    pub consonants: Vec<String>,
    pub vowels: Vec<String>,
    pub onsets: Vec<String>,
    pub people: Vec<String>,
    pub places: Vec<String>,
    pub words: Vec<String>,
}

impl Language {
    pub fn new(seed: u64, aesthetic: Aesthetic) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let inventory = Inventory::sample(&aesthetic, &mut rng);
        let generator = Generator::compile(&aesthetic, &inventory);
        let mut lang = Self {
            seed,
            aesthetic,
            inventory,
            generator,
            autonym: String::new(),
        };
        let mut arng = StdRng::seed_from_u64(seed.wrapping_add(4));
        lang.autonym = lang.person_name(&mut arng);
        lang
    }

    pub fn word(&self, rng: &mut impl rand::Rng) -> Word {
        Word::generate(&self.aesthetic, &self.generator, &self.inventory, rng, NK::Word)
    }

    pub fn person_name(&self, rng: &mut impl rand::Rng) -> String {
        self.named(rng, NK::Person)
    }

    pub fn place_name(&self, rng: &mut impl rand::Rng) -> String {
        self.named(rng, NK::Place)
    }

    fn named(&self, rng: &mut impl rand::Rng, kind: NK) -> String {
        let mut best = String::new();
        for _ in 0..8 {
            let s = capitalize(self.romanize(&Word::generate(
                &self.aesthetic,
                &self.generator,
                &self.inventory,
                rng,
                kind,
            )));
            let letters = s.chars().filter(|c| c.is_alphabetic()).count();
            if letters >= 4 {
                return s;
            }
            if letters > best.chars().filter(|c| c.is_alphabetic()).count() {
                best = s;
            }
        }
        best
    }

    pub fn romanize(&self, word: &Word) -> String {
        ortho::romanize(word, &self.aesthetic)
    }

    pub fn sample_people(&self, n: usize) -> Vec<String> {
        let mut rng = StdRng::seed_from_u64(self.seed.wrapping_add(1));
        (0..n).map(|_| self.person_name(&mut rng)).collect()
    }

    pub fn sample_places(&self, n: usize) -> Vec<String> {
        let mut rng = StdRng::seed_from_u64(self.seed.wrapping_add(2));
        (0..n).map(|_| self.place_name(&mut rng)).collect()
    }

    pub fn sample_words(&self, n: usize) -> Vec<String> {
        let mut rng = StdRng::seed_from_u64(self.seed.wrapping_add(3));
        (0..n).map(|_| self.romanize(&self.word(&mut rng))).collect()
    }

    pub fn snapshot(&self, n: usize) -> Snapshot {
        Snapshot {
            seed: self.seed,
            aesthetic: self.aesthetic.id.clone(),
            autonym: self.autonym.clone(),
            consonants: self
                .inventory
                .ipas(&self.inventory.consonants)
                .into_iter()
                .map(str::to_string)
                .collect(),
            vowels: self
                .inventory
                .ipas(&self.inventory.vowels)
                .into_iter()
                .map(str::to_string)
                .collect(),
            onsets: self.generator.onset_ipas().into_iter().take(24).collect(),
            people: self.sample_people(n),
            places: self.sample_places(n),
            words: self.sample_words(n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn same_seed_same_language() {
        let a = Aesthetic::by_id("elvish").unwrap();
        let l1 = Language::new(42, a.clone());
        let l2 = Language::new(42, a);
        assert_eq!(l1.autonym, l2.autonym);
        assert_eq!(l1.sample_words(8), l2.sample_words(8));
        assert_eq!(l1.inventory.consonants, l2.inventory.consonants);
    }

    #[test]
    fn words_use_inventory() {
        for id in ["elvish", "kuo-toa", "illithid", "neutral"] {
            let lang = Language::new(7, Aesthetic::by_id(id).unwrap());
            let mut rng = StdRng::seed_from_u64(99);
            for _ in 0..40 {
                let w = lang.word(&mut rng);
                for p in w.phonemes() {
                    assert!(
                        lang.inventory.contains(p),
                        "{id} generated {} outside inventory",
                        CATALOG.get(p).ipa()
                    );
                }
                let roman = lang.romanize(&w);
                assert!(!roman.is_empty(), "{id} produced an empty romanization");
            }
        }
    }

    #[test]
    fn all_packs_load() {
        assert_eq!(Aesthetic::all().len(), 4);
        for a in Aesthetic::all() {
            let lang = Language::new(1, a);
            assert!(!lang.inventory.vowels.is_empty());
            assert!(!lang.inventory.consonants.is_empty());
            assert!(!lang.sample_people(3).is_empty());
        }
    }
}

fn capitalize(s: String) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => s,
    }
}

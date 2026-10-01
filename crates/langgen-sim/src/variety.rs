use crate::adapt::ESTABLISHED_SHARE;
use crate::form::Form;
use crate::inventory::Inventory;
use crate::lexicon::Lexicon;
use crate::phoneme::PhonemeId;
use crate::phonotactics::Phonotactics;
use crate::profile::SoundProfile;
use crate::rng::{key, stream};
use crate::root::mint_roots;
use std::collections::{BTreeSet, HashMap, HashSet};

/// One language variety: the profile its speakers' preferences come from,
/// its words, and the sound laws it has undergone.
#[derive(Clone, Debug)]
pub struct Variety {
    pub profile: SoundProfile,
    /// Segments sampled at founding. The current inventory is whatever the
    /// living words use; see `inventory`.
    pub founding_inventory: Inventory,
    pub lexicon: Lexicon,
    /// Sound laws in the order applied, with their generation.
    pub laws: Vec<(u32, &'static str)>,
    /// Where this variety split from, if it did.
    pub parent: Option<Fork>,
}

/// A variety's descent from another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fork {
    /// Index of the parent variety in its `World`.
    pub variety: usize,
    pub generation: u32,
    /// Words with ids below this were inherited from the parent and share
    /// those ids there; later ids are this variety's own.
    pub inherited: u32,
}

impl Variety {
    /// A new variety at founding: an inventory sampled from the profile and
    /// one root per concept. The same seed and profile always agree.
    pub fn found(seed: u64, profile: &SoundProfile) -> Self {
        let inventory =
            Inventory::sample(&profile.inventory, &mut stream(seed, &[key("inventory")]));
        let phonotactics = Phonotactics::compile(&profile.phonotactics, &inventory);
        Self {
            profile: profile.clone(),
            lexicon: Lexicon::found(mint_roots(seed, &phonotactics)),
            founding_inventory: inventory,
            laws: Vec::new(),
            parent: None,
        }
    }

    /// A daughter of this variety, identical at the moment of the split.
    /// Laws already applied stay applied; obsolete words stay in the record.
    pub fn fork(&self, parent: usize, generation: u32) -> Self {
        Self {
            parent: Some(Fork {
                variety: parent,
                generation,
                inherited: self.lexicon.lexemes.len() as u32,
            }),
            ..self.clone()
        }
    }

    /// Segments in at least 2% of living words (and at least two): the
    /// sounds speakers treat as their own rather than marginal.
    pub fn established(&self) -> HashSet<PhonemeId> {
        let mut in_words: HashMap<PhonemeId, u32> = HashMap::new();
        let mut words = 0;
        for lexeme in self.lexicon.living() {
            words += 1;
            let unique: HashSet<PhonemeId> = lexeme.form.phones().collect();
            for p in unique {
                *in_words.entry(p).or_default() += 1;
            }
        }
        let threshold = (words as f32 * ESTABLISHED_SHARE).max(2.0);
        in_words
            .into_iter()
            .filter(|&(_, n)| n as f32 >= threshold)
            .map(|(p, _)| p)
            .collect()
    }

    /// Segments used by living words, consonants then vowels, by IPA.
    pub fn inventory(&self) -> (Vec<PhonemeId>, Vec<PhonemeId>) {
        let used: BTreeSet<u16> = self
            .lexicon
            .living()
            .flat_map(|l| l.form.phones().map(|p| p.0))
            .collect();
        let mut ids: Vec<PhonemeId> = used.into_iter().map(PhonemeId).collect();
        ids.sort_by_key(|id| crate::CATALOG.get(*id).ipa());
        ids.into_iter()
            .partition(|id| !crate::CATALOG.get(*id).is_vowel())
    }

    pub fn spell(&self, form: &Form) -> String {
        self.profile.spelling.write(form)
    }
}

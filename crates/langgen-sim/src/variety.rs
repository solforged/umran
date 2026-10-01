use crate::form::Form;
use crate::inventory::Inventory;
use crate::lexicon::Lexicon;
use crate::phoneme::PhonemeId;
use crate::phonotactics::Phonotactics;
use crate::profile::SoundProfile;
use crate::rng::{key, stream};
use crate::root::mint_roots;
use std::collections::BTreeSet;

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
        }
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

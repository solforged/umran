use crate::concepts::Concept;
use crate::form::Form;
use crate::inventory::Inventory;
use crate::phonotactics::Phonotactics;
use crate::preset::{Preset, Spelling};
use crate::rng::{key, stream};
use crate::root::mint_roots;

/// One language variety: its sound system, spelling, and words.
#[derive(Clone, Debug)]
pub struct Variety {
    pub preset: String,
    pub inventory: Inventory,
    pub phonotactics: Phonotactics,
    pub spelling: Spelling,
    pub words: Vec<Word>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub concept: &'static Concept,
    pub form: Form,
}

impl Variety {
    /// A new variety at founding: an inventory sampled from the preset and
    /// one root per concept. The same seed and preset always agree.
    pub fn found(seed: u64, preset: &Preset) -> Self {
        let inventory =
            Inventory::sample(&preset.inventory, &mut stream(seed, &[key("inventory")]));
        let phonotactics = Phonotactics::compile(&preset.phonotactics, &inventory);
        let words = mint_roots(seed, &phonotactics)
            .into_iter()
            .map(|(concept, form)| Word { concept, form })
            .collect();
        Self {
            preset: preset.id.clone(),
            inventory,
            phonotactics,
            spelling: preset.spelling.clone(),
            words,
        }
    }

    pub fn word(&self, concept: &str) -> Option<&Word> {
        self.words.iter().find(|w| w.concept.id == concept)
    }

    pub fn spell(&self, form: &Form) -> String {
        self.spelling.write(form)
    }
}

use crate::adapt::ESTABLISHED_SHARE;
use crate::form::Form;
use crate::ideas::{Need, need};
use crate::inventory::Inventory;
use crate::lexicon::{Lexeme, Lexicon};
use crate::livelihood::Livelihood;
use crate::morphology::Morphology;
use crate::names::{GivenName, Name, NameStyle, given_stock};
use crate::phoneme::PhonemeId;
use crate::phonotactics::Phonotactics;
use crate::profile::SoundProfile;
use crate::prosody::MinimalWord;
use crate::rng::{key, stream};
use crate::root::mint_roots;
use rand::Rng;
use std::collections::{BTreeSet, HashMap, HashSet};

/// One language variety: the profile its speakers' preferences come from,
/// its words, and the sound laws it has undergone.
#[derive(Clone, Debug)]
pub struct Variety {
    /// What its speakers call it; empty until a world names it.
    pub name: Name,
    pub profile: SoundProfile,
    /// Segments sampled at founding. The current inventory is whatever the
    /// living words use; see `inventory`.
    pub founding_inventory: Inventory,
    pub lexicon: Lexicon,
    /// How this language builds words from words.
    pub morphology: Morphology,
    /// The smallest word sound change may leave.
    pub minimal: MinimalWord,
    /// Sound laws in the order applied, with their generation. A law may
    /// appear more than once: kinds of change recur over long spans.
    pub laws: Vec<(u32, &'static str)>,
    /// Laws among `laws` that reached it from a neighbour rather than
    /// arising in it, by generation, with the variety each came from.
    pub waves: Vec<(u32, &'static str, usize)>,
    /// Where this variety split from, if it did.
    pub parent: Option<Fork>,
    /// How its speakers build given names, and the names in fashion now.
    pub style: NameStyle,
    pub given: Vec<GivenName>,
    /// The generation it was first written, or last respelled: words are
    /// spelled as they sounded then, however they have changed since.
    pub written: Option<u32>,
    /// What its speakers call lands others hold, by region: each heard
    /// once, when they first lived on or beside it, and changed since by
    /// this language's own sound laws, as German Mailand came from
    /// Mediolanum.
    pub exonyms: Vec<(usize, Name)>,
    /// The classical form its speakers write, or wrote before writing
    /// their own speech (`diglossia.rs`).
    pub high: Option<usize>,
    /// When its speakers began to write their own speech in place of
    /// `high`, if they have.
    pub vernacular: Option<u32>,
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
    /// A new variety at founding: an inventory sampled from the profile,
    /// one root per concept its speakers living by `livelihood` know of
    /// (meanings that wait for a craft or a faith have none yet), and a
    /// stock of given names. The same seed, profile, and livelihood always
    /// agree.
    pub fn found(seed: u64, profile: &SoundProfile, livelihood: Livelihood) -> Self {
        let inventory =
            Inventory::sample(&profile.inventory, &mut stream(seed, &[key("inventory")]));
        let phonotactics = Phonotactics::compile(&profile.phonotactics, &inventory);
        let morphology = Morphology::found(
            &profile.morphology,
            &phonotactics,
            &mut stream(seed, &[key("morphology")]),
        );
        let known = |minted: &crate::root::Minted| match need(minted.concept) {
            None => true,
            Some(Need::Livelihood(l)) => l == livelihood,
            Some(_) => false,
        };
        let roots = mint_roots(
            seed,
            &phonotactics,
            &profile.spelling,
            &morphology,
            profile.morphology.derivation,
        );
        let style = if stream(seed, &[key("given style")]).r#gen::<f32>() < 0.6 {
            NameStyle::Double
        } else {
            NameStyle::Single
        };
        let mut variety = Self {
            name: Name::default(),
            profile: profile.clone(),
            lexicon: Lexicon::found(roots.into_iter().filter(known)),
            morphology,
            minimal: MinimalWord::draw(
                profile.phonotactics.disyllabic_roots,
                &mut stream(seed, &[key("minimal word")]),
            ),
            founding_inventory: inventory,
            laws: Vec::new(),
            waves: Vec::new(),
            parent: None,
            style,
            given: Vec::new(),
            written: None,
            exonyms: Vec::new(),
            high: None,
            vernacular: None,
        };
        variety.given = given_stock(&variety, livelihood, &mut stream(seed, &[key("given")]));
        variety
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

    /// How a word is written: as it sounded when the language was first
    /// written (or last respelled), or when the word came in if later,
    /// however it sounds now, as English still writes the k of knight.
    /// Unwritten languages spell words as they sound.
    pub fn written_word(&self, lexeme: &Lexeme) -> String {
        match self.written {
            Some(g) => self.spell(lexeme.form_at(g.max(lexeme.born))),
            None => self.spell(&lexeme.form),
        }
    }

    /// How a name is written, as `written_word` writes words.
    pub fn written_name(&self, name: &Name) -> String {
        match self.written {
            Some(g) => self.spell(name.form_at(g.max(name.coined))),
            None => self.spell(&name.form),
        }
    }

    /// A name's form spelled and capitalized.
    pub fn title(&self, form: &Form) -> String {
        crate::names::title(&self.spell(form))
    }
}

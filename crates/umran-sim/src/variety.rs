use crate::adapt::ESTABLISHED_SHARE;
use crate::form::Form;
use crate::grammar::Grammar;
use crate::ideas::{Need, need};
use crate::inventory::Inventory;
use crate::lexicon::{Lexeme, Lexicon};
use crate::livelihood::Livelihood;
use crate::morphology::Morphology;
use crate::names::{GivenName, Name, NameStyle, given_stock};
use crate::phoneme::PhonemeId;
use crate::phonotactics::Phonotactics;
use crate::profile::SoundProfile;
use crate::prosody::{MinimalWord, StressRule};
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
    /// Grammatical markers, with inherited word and possessor order.
    pub grammar: Grammar,
    /// Personal pronouns use the lexicon's six person/number slots.
    pub pronouns: crate::pronouns::Pronouns,
    /// Noun classes and the determiner forms that agree with them.
    pub gender: crate::gender::Gender,
    /// Productive word-level vowel agreement, including its last loss.
    pub harmony: Option<crate::harmony::Harmony>,
    pub harmony_events: Vec<crate::harmony::Notice>,
    /// The smallest word sound change may leave.
    pub minimal: MinimalWord,
    /// Sound laws in the order applied, with their generation. A law may
    /// appear more than once: kinds of change recur over long spans.
    pub laws: Vec<(u32, &'static str)>,
    /// The rule before each stress shift, including inherited shifts.
    pub stress_history: Vec<(u32, StressRule)>,
    pub tonal: Option<crate::tone::Tonal>,
    /// Laws among `laws` that reached it from a neighbour rather than
    /// arising in it, by generation, with the variety each came from.
    pub waves: Vec<(u32, &'static str, usize)>,
    /// Where this variety split from, if it did.
    pub parent: Option<Fork>,
    /// Contributors to this koiné at formation, largest first.
    pub koine_of: Vec<(usize, f32)>,
    /// Regular minority-to-majority sound mergers at its formation.
    pub koine_mergers: Vec<(PhonemeId, PhonemeId)>,
    /// How its speakers build given names, and the names in fashion now.
    pub style: NameStyle,
    pub given: Vec<GivenName>,
    /// The generation it was first written, or last respelled: words are
    /// spelled as they sounded then, however they have changed since.
    pub written: Option<u32>,
    /// Remembered regional names: inherited local names, surveys, and
    /// first-heard foreign names. They evolve through this language's own
    /// sound laws, as German Mailand came from Mediolanum.
    pub exonyms: Vec<(usize, Name)>,
    /// Remembered hydronyms, indexed by river rather than land. Local
    /// alternatives survive handovers and evolve with this language.
    pub river_exonyms: Vec<(usize, Name)>,
    /// Remembered lake names, independent of river identities.
    pub lake_exonyms: Vec<(usize, Name)>,
    /// The classical form its speakers write, or wrote before writing
    /// their own speech (`diglossia.rs`).
    pub high: Option<usize>,
    /// When its speakers began to write their own speech in place of
    /// `high`, if they have.
    pub vernacular: Option<u32>,
    /// Recorded reforms of this written high form, not its everyday speech.
    pub purism: Vec<crate::purism::Purism>,
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
    /// stock of given names weighted by ethos. Identical inputs always agree.
    pub fn found(
        seed: u64,
        profile: &SoundProfile,
        livelihood: Livelihood,
        ethos: crate::Ethos,
    ) -> Self {
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
            grammar: Grammar::default(),
            pronouns: crate::pronouns::Pronouns::default(),
            gender: crate::gender::Gender::default(),
            harmony: None,
            harmony_events: Vec::new(),
            minimal: MinimalWord::draw(
                profile.phonotactics.disyllabic_roots,
                &mut stream(seed, &[key("minimal word")]),
            ),
            founding_inventory: inventory,
            laws: Vec::new(),
            stress_history: Vec::new(),
            tonal: None,
            waves: Vec::new(),
            parent: None,
            koine_of: Vec::new(),
            koine_mergers: Vec::new(),
            style,
            given: Vec::new(),
            written: None,
            exonyms: Vec::new(),
            river_exonyms: Vec::new(),
            lake_exonyms: Vec::new(),
            high: None,
            vernacular: None,
            purism: Vec::new(),
        };
        let mut prosody_rng = stream(seed, &[key("founding prosody")]);
        variety.profile.stress = Some(
            profile
                .stress
                .unwrap_or_else(|| StressRule::draw(&mut prosody_rng)),
        );
        for word in &mut variety.lexicon.lexemes {
            if variety.profile.stress == Some(StressRule::Free) {
                word.form.stress =
                    Some(crate::rng::index(&mut prosody_rng, word.form.vowel_count()));
            }
            if profile.phonotactics.geminates > 0.0 {
                let end = word.form.segs.len().saturating_sub(1);
                for i in 1..end {
                    if !word.form.is_vowel(i)
                        && word.form.is_vowel(i - 1)
                        && word.form.is_vowel(i + 1)
                    {
                        word.form.segs[i].long =
                            prosody_rng.r#gen::<f32>() < profile.phonotactics.geminates;
                    }
                }
            }
        }
        crate::pronouns::found(seed, &phonotactics, &mut variety.lexicon);
        variety.grammar = Grammar::found(
            seed,
            profile,
            &phonotactics,
            &variety.morphology,
            variety.stress(),
            &mut variety.lexicon,
        );
        variety.gender = crate::gender::Gender::found(
            seed,
            profile,
            &phonotactics,
            &variety.morphology,
            variety.stress(),
            &variety.lexicon,
        );
        variety.given = given_stock(
            &variety,
            livelihood,
            ethos,
            &mut stream(seed, &[key("given")]),
        );
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
            koine_of: Vec::new(),
            koine_mergers: Vec::new(),
            ..self.clone()
        }
    }
    pub fn stress(&self) -> StressRule {
        self.profile
            .stress
            .expect("founded varieties have a stress rule")
    }

    /// The first later shift remembers the rule used at this generation.
    pub fn stress_at(&self, generation: u32) -> StressRule {
        self.stress_history
            .iter()
            .find(|(g, _)| *g > generation)
            .map_or(self.stress(), |(_, before)| *before)
    }

    pub(crate) fn sync_grammar(&mut self, generation: u32) {
        self.harmonize_words(generation);
        let stress = self.stress();
        self.grammar
            .sync(&mut self.lexicon, &self.morphology, stress, generation);
        self.gender.sync(&self.lexicon);
    }

    /// Every living lexical or grammatical word used to assess a sound law.
    pub fn spoken_forms(&self) -> impl Iterator<Item = (&Form, f32)> {
        self.grammar.forms(&self.lexicon).chain(self.gender.forms())
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
            .grammar
            .forms(&self.lexicon)
            .flat_map(|(form, _)| form.phones().map(|p| p.0))
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

use crate::flavor::Flavor;
use crate::phoneme::{Backness, Height, Manner, Place};
use serde::{Deserialize, Serialize};

/// The full numbers behind a sound system: segment preferences, syllable
/// shapes, word building, and spelling. Languages are founded from a
/// `LanguageDesign`, which resolves to one of these; presets come from the
/// typical base with a flavor applied.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub inventory: InventoryPrior,
    pub phonotactics: PhonotacticPrior,
    pub morphology: MorphologyPrior,
    pub spelling: Spelling,
}

/// How words are built from other words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MorphologyKind {
    /// Affixes attach before or after a base: fish > fish-er.
    Concatenative,
    /// Roots are consonant skeletons and words are vowel patterns over them,
    /// as in Arabic k-t-b: kataba "he wrote", kitāb "book", maktab "office".
    RootPattern,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MorphologyPrior {
    pub kind: MorphologyKind,
    /// Chance an affix is a suffix rather than a prefix. Suffixing is the
    /// more common choice worldwide.
    pub suffixing: f32,
    /// Chance each word-family link is realized by derivation rather than
    /// by an unrelated root.
    pub derivation: f32,
}

/// Log-weight adjustments by feature; a listed feature adds its weight, an
/// unlisted one is penalized. `extra` adjusts single segments by IPA.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InventoryPrior {
    /// Use exactly the required segments: no sampling, no repairs. Set by
    /// a `LanguageDesign`, where a person chose every sound.
    #[serde(default)]
    pub exact: bool,
    pub consonant_count: (u8, u8),
    pub vowel_count: (u8, u8),
    pub manner: Vec<(Manner, f32)>,
    pub place: Vec<(Place, f32)>,
    pub voiced: f32,
    pub height: Vec<(Height, f32)>,
    pub backness: Vec<(Backness, f32)>,
    pub rounded: f32,
    pub extra: Vec<(String, f32)>,
    pub required: Vec<String>,
    pub forbidden: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhonotacticPrior {
    pub max_onset: u8,
    pub max_coda: u8,
    /// Chance that a word-final syllable is closed.
    pub final_coda: f32,
    /// Word-internal syllables never take a coda.
    pub open_medial: bool,
    pub preferred_onsets: Vec<String>,
    pub preferred_codas: Vec<String>,
    /// Chance that a founding root has two syllables (CVCV or CVCVC)
    /// rather than one.
    pub disyllabic_roots: f32,
    /// Chance a root may repeat a consonant (as in "kika"). Most languages
    /// avoid it in roots; echoing, reduplicating styles tolerate it.
    pub identical_consonants: f32,
    /// Chance each vowel of a new root is long, for languages where vowel
    /// length distinguishes words (Sanskrit, Old English, Nahuatl).
    #[serde(default)]
    pub long_vowels: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spelling {
    /// IPA segment to letters, overriding the catalog romanization.
    pub overrides: Vec<(String, String)>,
    pub kw_as_qu: bool,
    pub long_vowels: LongVowel,
    /// Write an apostrophe between adjacent vowels of different syllables.
    pub mark_hiatus: bool,
    /// Written between morphemes, e.g. `'` in compounds.
    pub boundary_mark: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LongVowel {
    Acute,
    Double,
    /// ā ī ū, as scholarly romanizations of Sanskrit, Old Persian, Old
    /// English, and Nahuatl write them.
    Macron,
    Unmarked,
}

impl SoundProfile {
    /// A typical human language: flat tastes, so world frequencies decide.
    /// The base every preset and "fill typical" starts from.
    pub fn typical() -> SoundProfile {
        typical()
    }

    /// Ready-made starting points: typical, and typical with each example
    /// flavor applied.
    pub fn presets() -> Vec<SoundProfile> {
        let base = typical();
        std::iter::once(base.clone())
            .chain(Flavor::examples().iter().map(|f| base.flavored(f)))
            .collect()
    }

    /// A preset by id: "typical", or a flavor's id ("indic", "semitic"...).
    pub fn by_id(id: &str) -> Option<SoundProfile> {
        if id == "typical" {
            return Some(typical());
        }
        Flavor::by_id(id).map(|f| typical().flavored(&f))
    }
}

fn ipas(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn typical() -> SoundProfile {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    // Flat tastes: which sounds appear, and how often, comes mostly from how
    // common they are across the world's languages (see `typology`).
    SoundProfile {
        id: "typical".into(),
        name: "Typical".into(),
        description: "A typical human language: sounds weighted by how common they are worldwide."
            .into(),
        inventory: InventoryPrior {
            exact: false,
            consonant_count: (14, 20),
            vowel_count: (5, 7),
            manner: vec![
                (Stop, 1.0),
                (Nasal, 1.0),
                (Fricative, 1.0),
                (Affricate, 1.0),
                (Lateral, 1.0),
                (Approximant, 1.0),
                (Trill, 1.0),
                (Tap, 1.0),
                (LateralFricative, 0.6),
                (LateralAffricate, 0.6),
                (Ejective, 0.6),
                (Implosive, 0.6),
            ],
            place: vec![
                (Bilabial, 1.0),
                (Labiodental, 1.0),
                (Dental, 1.0),
                (Alveolar, 1.0),
                (Postalveolar, 1.0),
                (Retroflex, 1.0),
                (Palatal, 1.0),
                (Velar, 1.0),
                (Uvular, 1.0),
                (Pharyngeal, 1.0),
                (Glottal, 1.0),
            ],
            voiced: 0.0,
            height: vec![
                (Close, 1.0),
                (NearClose, 0.6),
                (CloseMid, 1.0),
                (Mid, 0.8),
                (OpenMid, 1.0),
                (NearOpen, 0.6),
                (Open, 1.0),
            ],
            backness: vec![(Front, 1.0), (Central, 0.8), (Back, 1.0)],
            rounded: 0.0,
            extra: vec![],
            required: ipas(&["p", "t", "k", "m", "n", "i", "a", "u"]),
            forbidden: vec![],
        },
        phonotactics: PhonotacticPrior {
            max_onset: 2,
            max_coda: 1,
            final_coda: 0.45,
            open_medial: false,
            preferred_onsets: vec![],
            preferred_codas: ipas(&["n", "m", "ŋ", "l", "r", "s", "k", "t"]),
            disyllabic_roots: 0.25,
            identical_consonants: 0.1,
            long_vowels: 0.0,
        },
        morphology: MorphologyPrior {
            kind: MorphologyKind::Concatenative,
            suffixing: 0.7,
            derivation: 0.5,
        },
        spelling: Spelling {
            overrides: vec![],
            kw_as_qu: false,
            long_vowels: LongVowel::Unmarked,
            mark_hiatus: false,
            boundary_mark: None,
        },
    }
}

use crate::flavor::Flavor;
use crate::grammar::GrammarPrior;
use crate::phoneme::{Backness, Height, Manner, Place};
use crate::prosody::StressRule;
use serde::{Deserialize, Serialize};

/// The full numbers behind a sound system: segment preferences, syllable
/// shapes, primary stress, word building, grammar, and spelling. Languages
/// are founded from a `LanguageDesign`, which resolves to one of these;
/// presets come from the base with a flavor applied.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub inventory: InventoryPrior,
    pub phonotactics: PhonotacticPrior,
    pub morphology: MorphologyPrior,
    /// Independent plural/past founding choices; unpinned categories use
    /// morphology's weak bound/particle/none and suffix/prefix biases.
    #[serde(default)]
    pub grammar: GrammarPrior,
    pub spelling: Spelling,
    /// Absent in a design/profile means draw on a separate founding stream.
    #[serde(default)]
    pub stress: Option<StressRule>,
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
    /// Chance an internal consonant is long; disabled unless requested.
    #[serde(default)]
    pub geminates: f32,
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
    /// The common ground every flavor starts from: flat tastes, so how
    /// common each sound is worldwide decides. It is not offered as a
    /// language of its own, since there is no typical human language.
    pub fn base() -> SoundProfile {
        base()
    }

    /// Ready-made starting points: the base with each example flavor.
    pub fn presets() -> Vec<SoundProfile> {
        let base = base();
        Flavor::examples()
            .iter()
            .map(|f| base.flavored(f))
            .collect()
    }

    /// A preset by its flavor's id ("indic", "semitic"...).
    pub fn by_id(id: &str) -> Option<SoundProfile> {
        Flavor::by_id(id).map(|f| base().flavored(&f))
    }
}

fn ipas(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn base() -> SoundProfile {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    // Flat tastes: which sounds appear, and how often, comes mostly from how
    // common they are across the world's languages (see `typology`).
    SoundProfile {
        id: "base".into(),
        name: "Base".into(),
        description: "Sounds weighted by how common they are worldwide.".into(),
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
            geminates: 0.0,
        },
        morphology: MorphologyPrior {
            kind: MorphologyKind::Concatenative,
            suffixing: 0.7,
            derivation: 0.5,
        },
        grammar: GrammarPrior::default(),
        spelling: Spelling {
            overrides: vec![],
            kw_as_qu: false,
            long_vowels: LongVowel::Unmarked,
            mark_hiatus: false,
            boundary_mark: None,
        },
        stress: None,
    }
}

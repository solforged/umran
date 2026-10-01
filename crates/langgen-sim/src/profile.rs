use crate::phoneme::{Backness, Height, Manner, Place};
use serde::{Deserialize, Serialize};

/// The full numbers behind a sound system: segment preferences, syllable
/// shapes, and spelling. The four built-in profiles are examples; a culture
/// usually starts from one and layers `Flavor` adjustments on top.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub inventory: InventoryPrior,
    pub phonotactics: PhonotacticPrior,
    pub spelling: Spelling,
}

/// Log-weight adjustments by feature; a listed feature adds its weight, an
/// unlisted one is penalized. `extra` adjusts single segments by IPA.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InventoryPrior {
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
    /// Chance that a founding root is CVCV rather than CV or CVC.
    pub disyllabic_roots: f32,
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
    Unmarked,
}

impl SoundProfile {
    pub fn examples() -> Vec<SoundProfile> {
        vec![elvish(), kuo_toa(), illithid(), neutral()]
    }

    pub fn by_id(id: &str) -> Option<SoundProfile> {
        Self::examples().into_iter().find(|p| p.id == id)
    }
}

fn ipas(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

fn weights(list: &[(&str, f32)]) -> Vec<(String, f32)> {
    list.iter().map(|(s, w)| (s.to_string(), *w)).collect()
}

/// Rare or marked segments that no example profile wants.
fn marked(more: &[&str]) -> Vec<String> {
    let mut out = ipas(&[
        "q", "ɓ", "ɗ", "pʼ", "tʼ", "kʼ", "ɨ", "ø", "æ", "ʊ", "y", "ə", "ɪ", "ɬ", "ʎ",
    ]);
    out.extend(ipas(more));
    out
}

fn elvish() -> SoundProfile {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    SoundProfile {
        id: "elvish".into(),
        name: "Elvish".into(),
        description: "Open, liquid, and palatal; Tolkienic rather than a reconstruction.".into(),
        inventory: InventoryPrior {
            consonant_count: (13, 16),
            vowel_count: (5, 5),
            manner: vec![
                (Lateral, 2.6),
                (Trill, 1.6),
                (Approximant, 1.2),
                (Nasal, 1.0),
                (Fricative, 0.7),
                (Stop, 0.35),
                (Tap, 0.2),
                (Affricate, -6.0),
                (Ejective, -8.0),
                (Implosive, -8.0),
                (LateralFricative, -6.0),
            ],
            place: vec![
                (Alveolar, 1.3),
                (Dental, 1.8),
                (Palatal, 0.9),
                (Labiodental, 0.25),
                (Velar, 0.2),
                (Bilabial, 0.15),
                (Uvular, -8.0),
                (Glottal, -2.0),
                (Pharyngeal, -8.0),
            ],
            voiced: 0.05,
            height: vec![(Close, 0.8), (CloseMid, 1.0), (Open, 0.7)],
            backness: vec![(Front, 0.7), (Back, 0.35), (Central, -3.0)],
            rounded: -0.15,
            extra: weights(&[
                ("l", 2.2),
                ("r", 1.8),
                ("θ", 1.8),
                ("ð", 1.2),
                ("n", 1.0),
                ("k", 0.9),
                ("s", 0.6),
                ("j", 0.8),
                ("v", 0.5),
                ("w", 0.5),
                ("h", 0.3),
                ("m", 0.5),
            ]),
            required: ipas(&[
                "l", "r", "n", "t", "k", "s", "m", "θ", "i", "e", "a", "o", "u",
            ]),
            forbidden: marked(&[
                "ʔ", "ŋ", "dz", "ts", "dʒ", "ʒ", "ɣ", "ɛ", "ɔ", "ɑ", "ɲ", "z",
            ]),
        },
        phonotactics: PhonotacticPrior {
            max_onset: 2,
            max_coda: 1,
            final_coda: 0.28,
            open_medial: true,
            preferred_onsets: ipas(&[
                "l", "r", "n", "m", "s", "t", "k", "θ", "ð", "j", "v", "w", "h", "st", "tr", "kr",
                "pr", "gl", "kw", "θr",
            ]),
            preferred_codas: ipas(&["n", "r", "l", "s", "θ"]),
            disyllabic_roots: 0.3,
        },
        spelling: Spelling {
            overrides: pairs(&[("k", "c"), ("x", "ch")]),
            kw_as_qu: true,
            long_vowels: LongVowel::Acute,
            mark_hiatus: false,
            boundary_mark: None,
        },
    }
}

fn kuo_toa() -> SoundProfile {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    SoundProfile {
        id: "kuo-toa".into(),
        name: "Kuo-toa".into(),
        description: "Wet, labial, and echoing; Blibdoolpoolp as a north star, not a template."
            .into(),
        inventory: InventoryPrior {
            consonant_count: (8, 11),
            vowel_count: (3, 4),
            manner: vec![
                (Stop, 1.5),
                (Nasal, 1.5),
                (Lateral, 2.2),
                (Approximant, 0.1),
                (Fricative, -3.0),
                (Trill, -1.0),
                (Affricate, -6.0),
                (Ejective, -8.0),
                (Implosive, -4.0),
            ],
            place: vec![
                (Bilabial, 2.6),
                (Velar, 0.9),
                (Alveolar, 0.25),
                (Glottal, -3.0),
                (Dental, -4.0),
                (Palatal, -4.0),
                (Uvular, -6.0),
                (Labiodental, -3.0),
            ],
            voiced: 0.35,
            height: vec![(Close, 0.5), (CloseMid, 1.3), (Open, 0.9)],
            backness: vec![(Back, 1.8), (Front, -0.8), (Central, -3.0)],
            rounded: 1.2,
            extra: weights(&[
                ("b", 2.4),
                ("p", 2.2),
                ("l", 2.0),
                ("m", 1.6),
                ("g", 1.2),
                ("o", 2.0),
                ("u", 1.6),
                ("a", 0.8),
                ("ŋ", 0.7),
                ("d", 0.3),
            ]),
            required: ipas(&["b", "p", "l", "m", "g", "n", "o", "u", "a"]),
            forbidden: marked(&[
                "θ", "ð", "ʃ", "ʒ", "tʃ", "dʒ", "ts", "dz", "ʔ", "j", "ɣ", "f", "v", "x", "ɛ", "ɔ",
                "ɑ", "ɲ", "h",
            ]),
        },
        phonotactics: PhonotacticPrior {
            max_onset: 2,
            max_coda: 2,
            final_coda: 0.78,
            open_medial: false,
            preferred_onsets: ipas(&["bl", "pl", "gl", "b", "p", "m", "g", "l", "n"]),
            preferred_codas: ipas(&["p", "b", "l", "m", "ŋ", "lp"]),
            disyllabic_roots: 0.25,
        },
        spelling: Spelling {
            overrides: vec![],
            kw_as_qu: false,
            long_vowels: LongVowel::Double,
            mark_hiatus: false,
            boundary_mark: None,
        },
    }
}

fn illithid() -> SoundProfile {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    SoundProfile {
        id: "illithid".into(),
        name: "Illithid".into(),
        description: "Sibilant, dental, and slightly illegal clusters; cerebral rather than wet."
            .into(),
        inventory: InventoryPrior {
            consonant_count: (11, 14),
            vowel_count: (4, 5),
            manner: vec![
                (Fricative, 2.0),
                (Stop, 0.7),
                (Lateral, 1.3),
                (Nasal, 0.3),
                (Affricate, 0.2),
                (Approximant, -0.4),
                (Trill, -0.5),
                (Ejective, -5.0),
                (Implosive, -8.0),
            ],
            place: vec![
                (Dental, 2.2),
                (Velar, 1.2),
                (Alveolar, 0.5),
                (Postalveolar, 0.7),
                (Glottal, -1.5),
                (Bilabial, -3.0),
                (Labiodental, 0.2),
                (Uvular, -1.0),
                (Palatal, -1.5),
            ],
            voiced: -0.15,
            height: vec![(Close, 1.3), (Open, 0.8), (CloseMid, 0.3)],
            backness: vec![(Front, 0.7), (Back, 0.3), (Central, -3.0)],
            rounded: -0.25,
            extra: weights(&[
                ("θ", 2.6),
                ("k", 1.5),
                ("l", 1.4),
                ("s", 1.3),
                ("g", 0.9),
                ("ʃ", 0.7),
                ("i", 1.5),
                ("a", 0.9),
                ("d", 0.4),
                ("v", 0.4),
                ("ð", 0.8),
            ]),
            required: ipas(&["θ", "k", "l", "s", "n", "i", "a", "u"]),
            forbidden: marked(&[
                "ɲ", "ŋ", "ɾ", "ts", "ɛ", "ɔ", "b", "p", "m", "w", "ɣ", "dz", "dʒ", "ɑ",
            ]),
        },
        phonotactics: PhonotacticPrior {
            max_onset: 2,
            max_coda: 1,
            final_coda: 0.58,
            open_medial: false,
            preferred_onsets: ipas(&[
                "θ", "k", "g", "s", "l", "n", "sk", "kθ", "gθ", "st", "ʃ", "v",
            ]),
            preferred_codas: ipas(&["θ", "n", "k", "l", "s", "d"]),
            disyllabic_roots: 0.1,
        },
        spelling: Spelling {
            overrides: vec![],
            kw_as_qu: false,
            long_vowels: LongVowel::Double,
            mark_hiatus: true,
            boundary_mark: Some("'".into()),
        },
    }
}

fn neutral() -> SoundProfile {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    SoundProfile {
        id: "neutral".into(),
        name: "Neutral".into(),
        description: "A bland human baseline; fork this when inventing a new vibe.".into(),
        inventory: InventoryPrior {
            consonant_count: (11, 14),
            vowel_count: (5, 5),
            manner: vec![
                (Stop, 1.1),
                (Nasal, 0.9),
                (Fricative, 0.4),
                (Lateral, 0.6),
                (Approximant, 0.35),
                (Trill, 0.3),
                (Affricate, -5.0),
                (Ejective, -8.0),
                (Implosive, -8.0),
            ],
            place: vec![
                (Alveolar, 1.1),
                (Bilabial, 0.7),
                (Velar, 0.7),
                (Labiodental, 0.15),
                (Glottal, -1.0),
                (Uvular, -6.0),
                (Dental, -3.0),
                (Palatal, -2.0),
            ],
            voiced: 0.0,
            height: vec![(Close, 0.5), (CloseMid, 0.5), (Open, 0.5)],
            backness: vec![(Front, 0.4), (Back, 0.4), (Central, -3.0)],
            rounded: 0.0,
            extra: vec![],
            required: ipas(&[
                "p", "t", "k", "m", "n", "s", "l", "r", "i", "e", "a", "o", "u",
            ]),
            forbidden: marked(&[
                "ð", "ʃ", "ʒ", "tʃ", "dʒ", "θ", "ŋ", "ɲ", "ʔ", "ɛ", "ɔ", "ɑ", "ɣ", "x",
            ]),
        },
        phonotactics: PhonotacticPrior {
            max_onset: 2,
            max_coda: 1,
            final_coda: 0.5,
            open_medial: false,
            preferred_onsets: ipas(&["t", "k", "s", "n", "m", "p", "l", "r", "st", "tr", "pl"]),
            preferred_codas: ipas(&["n", "k", "t", "s", "m"]),
            disyllabic_roots: 0.15,
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

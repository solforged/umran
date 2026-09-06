use crate::phoneme::{Backness, Height, Manner, Place};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Aesthetic {
    pub id: String,
    pub name: String,
    pub description: String,
    pub inventory: InventoryPrior,
    pub clusters: ClusterPolicy,
    pub shape: WordShape,
    pub names: NameStyle,
    pub ortho: OrthoStyle,
    pub signatures: Vec<Signature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClusterPolicy {
    pub max_onset: u8,
    pub max_coda: u8,
    pub empty_onset: f32,
    pub empty_coda: f32,
    pub empty_coda_nonfinal: f32,
    pub onset_clusters: Vec<(Manner, Manner)>,
    pub coda_clusters: Vec<(Manner, Manner)>,
    pub preferred_onsets: Vec<String>,
    pub preferred_codas: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WordShape {
    pub syllables: (u8, u8),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NameStyle {
    pub person_syllables: (u8, u8),
    pub place_syllables: (u8, u8),
    pub person_endings: Vec<String>,
    pub place_endings: Vec<String>,
    pub compound_place: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrthoStyle {
    pub overrides: Vec<(String, String)>,
    pub long_vowels: LongVowel,
    pub k_as_c: bool,
    pub kw_as_qu: bool,
    pub compound_joiner: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LongVowel {
    Acute,
    Double,
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Signature {
    Reduplicate { probability: f32, partial: bool },
    PenultimateLength { probability: f32 },
    OpenNonfinal,
    GlottalHiatus,
}

impl Aesthetic {
    pub fn all() -> Vec<Aesthetic> {
        vec![elvish(), kuo_toa(), illithid(), neutral()]
    }

    pub fn by_id(id: &str) -> Option<Aesthetic> {
        Self::all().into_iter().find(|a| a.id == id)
    }
}

fn ugly() -> Vec<String> {
    [
        "q", "ɓ", "ɗ", "pʼ", "tʼ", "kʼ", "ɨ", "ø", "æ", "ʊ", "y", "ə", "ɪ", "ɬ", "ʎ",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn elvish() -> Aesthetic {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    let mut forbidden = ugly();
    forbidden.extend(
        [
            "ʔ", "ŋ", "dz", "ts", "dʒ", "ʒ", "ɣ", "ɛ", "ɔ", "ɑ", "ɲ", "z",
        ]
        .into_iter()
        .map(str::to_string),
    );
    Aesthetic {
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
            extra: vec![
                ("l".into(), 2.2),
                ("r".into(), 1.8),
                ("θ".into(), 1.8),
                ("ð".into(), 1.2),
                ("n".into(), 1.0),
                ("k".into(), 0.9),
                ("s".into(), 0.6),
                ("j".into(), 0.8),
                ("v".into(), 0.5),
                ("w".into(), 0.5),
                ("h".into(), 0.3),
                ("m".into(), 0.5),
            ],
            required: vec![
                "l".into(),
                "r".into(),
                "n".into(),
                "t".into(),
                "k".into(),
                "s".into(),
                "m".into(),
                "θ".into(),
                "i".into(),
                "e".into(),
                "a".into(),
                "o".into(),
                "u".into(),
            ],
            forbidden,
        },
        clusters: ClusterPolicy {
            max_onset: 2,
            max_coda: 1,
            empty_onset: 0.05,
            empty_coda: 0.72,
            empty_coda_nonfinal: 0.96,
            onset_clusters: vec![],
            coda_clusters: vec![],
            preferred_onsets: vec![
                "l".into(),
                "r".into(),
                "n".into(),
                "m".into(),
                "s".into(),
                "t".into(),
                "k".into(),
                "θ".into(),
                "ð".into(),
                "j".into(),
                "v".into(),
                "w".into(),
                "h".into(),
                "st".into(),
                "tr".into(),
                "kr".into(),
                "pr".into(),
                "gl".into(),
                "kw".into(),
                "θr".into(),
            ],
            preferred_codas: vec!["n".into(), "r".into(), "l".into(), "s".into(), "θ".into()],
        },
        shape: WordShape { syllables: (2, 3) },
        names: NameStyle {
            person_syllables: (2, 3),
            place_syllables: (2, 3),
            person_endings: vec![
                "iel".into(),
                "ion".into(),
                "wen".into(),
                "el".into(),
                "ar".into(),
            ],
            place_endings: vec![
                "dor".into(),
                "ond".into(),
                "aθ".into(),
                "del".into(),
                "ian".into(),
            ],
            compound_place: 0.4,
        },
        ortho: OrthoStyle {
            overrides: vec![("k".into(), "c".into()), ("x".into(), "ch".into())],
            long_vowels: LongVowel::Acute,
            k_as_c: true,
            kw_as_qu: true,
            compound_joiner: None,
        },
        signatures: vec![
            Signature::OpenNonfinal,
            Signature::PenultimateLength { probability: 0.55 },
        ],
    }
}

fn kuo_toa() -> Aesthetic {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    let mut forbidden = ugly();
    forbidden.extend(
        [
            "θ", "ð", "ʃ", "ʒ", "tʃ", "dʒ", "ts", "dz", "ʔ", "j", "ɣ", "f", "v", "x", "ɛ", "ɔ",
            "ɑ", "ɲ", "h",
        ]
        .into_iter()
        .map(str::to_string),
    );
    Aesthetic {
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
            extra: vec![
                ("b".into(), 2.4),
                ("p".into(), 2.2),
                ("l".into(), 2.0),
                ("m".into(), 1.6),
                ("g".into(), 1.2),
                ("o".into(), 2.0),
                ("u".into(), 1.6),
                ("a".into(), 0.8),
                ("ŋ".into(), 0.7),
                ("d".into(), 0.3),
            ],
            required: vec![
                "b".into(),
                "p".into(),
                "l".into(),
                "m".into(),
                "g".into(),
                "n".into(),
                "o".into(),
                "u".into(),
                "a".into(),
            ],
            forbidden,
        },
        clusters: ClusterPolicy {
            max_onset: 2,
            max_coda: 2,
            empty_onset: 0.03,
            empty_coda: 0.22,
            empty_coda_nonfinal: 0.72,
            onset_clusters: vec![],
            coda_clusters: vec![],
            preferred_onsets: vec![
                "bl".into(),
                "pl".into(),
                "gl".into(),
                "b".into(),
                "p".into(),
                "m".into(),
                "g".into(),
                "l".into(),
                "n".into(),
            ],
            preferred_codas: vec![
                "p".into(),
                "b".into(),
                "l".into(),
                "m".into(),
                "ŋ".into(),
                "lp".into(),
            ],
        },
        shape: WordShape { syllables: (2, 3) },
        names: NameStyle {
            person_syllables: (2, 3),
            place_syllables: (2, 3),
            person_endings: vec!["olp".into(), "up".into(), "ub".into()],
            place_endings: vec!["pol".into(), "blub".into(), "glop".into()],
            compound_place: 0.15,
        },
        ortho: OrthoStyle {
            overrides: vec![],
            long_vowels: LongVowel::Double,
            k_as_c: false,
            kw_as_qu: false,
            compound_joiner: None,
        },
        signatures: vec![
            Signature::Reduplicate {
                probability: 0.32,
                partial: true,
            },
            Signature::PenultimateLength { probability: 0.7 },
        ],
    }
}

fn illithid() -> Aesthetic {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    let mut forbidden = ugly();
    forbidden.extend(
        [
            "ɲ", "ŋ", "ɾ", "ts", "ɛ", "ɔ", "b", "p", "m", "w", "ɣ", "dz", "dʒ", "ɑ",
        ]
        .into_iter()
        .map(str::to_string),
    );
    Aesthetic {
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
            extra: vec![
                ("θ".into(), 2.6),
                ("k".into(), 1.5),
                ("l".into(), 1.4),
                ("s".into(), 1.3),
                ("g".into(), 0.9),
                ("ʃ".into(), 0.7),
                ("i".into(), 1.5),
                ("a".into(), 0.9),
                ("d".into(), 0.4),
                ("v".into(), 0.4),
                ("ð".into(), 0.8),
            ],
            required: vec![
                "θ".into(),
                "k".into(),
                "l".into(),
                "s".into(),
                "n".into(),
                "i".into(),
                "a".into(),
                "u".into(),
            ],
            forbidden,
        },
        clusters: ClusterPolicy {
            max_onset: 2,
            max_coda: 1,
            empty_onset: 0.1,
            empty_coda: 0.42,
            empty_coda_nonfinal: 0.7,
            onset_clusters: vec![],
            coda_clusters: vec![],
            preferred_onsets: vec![
                "θ".into(),
                "k".into(),
                "g".into(),
                "s".into(),
                "l".into(),
                "n".into(),
                "sk".into(),
                "kθ".into(),
                "gθ".into(),
                "st".into(),
                "ʃ".into(),
                "v".into(),
            ],
            preferred_codas: vec![
                "θ".into(),
                "n".into(),
                "k".into(),
                "l".into(),
                "s".into(),
                "d".into(),
            ],
        },
        shape: WordShape { syllables: (2, 3) },
        names: NameStyle {
            person_syllables: (2, 3),
            place_syllables: (2, 3),
            person_endings: vec!["iθ".into(), "id".into(), "aθ".into()],
            place_endings: vec!["θon".into(), "iθ".into(), "ak".into()],
            compound_place: 0.18,
        },
        ortho: OrthoStyle {
            overrides: vec![],
            long_vowels: LongVowel::Double,
            k_as_c: false,
            kw_as_qu: false,
            compound_joiner: Some("'".into()),
        },
        signatures: vec![
            Signature::GlottalHiatus,
            Signature::PenultimateLength { probability: 0.3 },
        ],
    }
}

fn neutral() -> Aesthetic {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    let mut forbidden = ugly();
    forbidden.extend(
        [
            "ð", "ʃ", "ʒ", "tʃ", "dʒ", "θ", "ŋ", "ɲ", "ʔ", "ɛ", "ɔ", "ɑ", "ɣ", "x",
        ]
        .into_iter()
        .map(str::to_string),
    );
    Aesthetic {
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
            required: vec![
                "p".into(),
                "t".into(),
                "k".into(),
                "m".into(),
                "n".into(),
                "s".into(),
                "l".into(),
                "r".into(),
                "i".into(),
                "e".into(),
                "a".into(),
                "o".into(),
                "u".into(),
            ],
            forbidden,
        },
        clusters: ClusterPolicy {
            max_onset: 2,
            max_coda: 1,
            empty_onset: 0.1,
            empty_coda: 0.5,
            empty_coda_nonfinal: 0.75,
            onset_clusters: vec![],
            coda_clusters: vec![],
            preferred_onsets: vec![
                "t".into(),
                "k".into(),
                "s".into(),
                "n".into(),
                "m".into(),
                "p".into(),
                "l".into(),
                "r".into(),
                "st".into(),
                "tr".into(),
                "pl".into(),
            ],
            preferred_codas: vec!["n".into(), "k".into(), "t".into(), "s".into(), "m".into()],
        },
        shape: WordShape { syllables: (1, 3) },
        names: NameStyle {
            person_syllables: (2, 3),
            place_syllables: (2, 3),
            person_endings: vec![],
            place_endings: vec![],
            compound_place: 0.12,
        },
        ortho: OrthoStyle {
            overrides: vec![],
            long_vowels: LongVowel::None,
            k_as_c: false,
            kw_as_qu: false,
            compound_joiner: None,
        },
        signatures: vec![],
    }
}

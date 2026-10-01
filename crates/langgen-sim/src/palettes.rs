//! Flavors modelled on the sound palettes of real language families,
//! written by Claude from one-line briefs. Each is a nudge, not a
//! reconstruction: it pushes sounds, word shape, vowel length, word
//! building, and spelling toward the family's feel, and stacks with other
//! flavors. Signature sounds are required so the nudge is always audible;
//! the rest are favoured or forbidden.
//!
//! Not yet expressible: tone, vowel harmony, consonant length, and
//! prenasalized stops; comments note where a family relies on them.

use crate::flavor::Flavor;
use crate::phoneme::{Backness, Height, Manner, Place};
use crate::profile::{LongVowel, MorphologyKind};

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn weighted(list: &[(&str, f32)]) -> Vec<(String, f32)> {
    list.iter().map(|(s, w)| (s.to_string(), *w)).collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

/// Sounds no family palette here wants unless it asks for them.
const EXOTIC: &[&str] = &[
    "pʼ", "tʼ", "kʼ", "ɓ", "ɗ", "ɬ", "tɬ", "ʎ", "qʷ", "xʷ", "ħ", "ʕ", "χ", "ʁ", "q",
];

fn forbid(extra: &[&str], except: &[&str]) -> Vec<String> {
    EXOTIC
        .iter()
        .chain(extra)
        .filter(|s| !except.contains(s))
        .map(|s| s.to_string())
        .collect()
}

pub fn all() -> Vec<Flavor> {
    vec![
        indic(),
        iranian(),
        germanic(),
        semitic(),
        nahuatl(),
        bantu(),
        polynesian(),
        finnic(),
        caucasian(),
    ]
}

/// Aspirated and breathy stops, retroflexes, three sibilants, and long
/// vowels, as in Sanskrit.
fn indic() -> Flavor {
    Flavor {
        id: "indic".into(),
        name: "Indic".into(),
        brief: "Make it more like Sanskrit.".into(),
        segments: weighted(&[
            ("pʰ", 2.0),
            ("tʰ", 2.0),
            ("kʰ", 2.0),
            ("bʱ", 4.0),
            ("dʱ", 4.0),
            ("gʱ", 4.0),
            ("ɖ", 2.5),
            ("ʂ", 2.5),
            ("ɭ", 1.0),
            ("ʃ", 1.5),
            ("tʃ", 1.0),
            ("dʒ", 1.0),
            ("ɲ", 1.0),
            ("ŋ", 1.0),
            ("v", 1.0),
            ("a", 1.0),
            ("i", 1.0),
            ("u", 1.0),
        ]),
        require: strings(&["kʰ", "bʱ", "ʈ", "ɳ", "ʂ", "a", "i", "u", "e", "o"]),
        forbid: forbid(
            &[
                "f", "z", "θ", "ð", "x", "ɣ", "ʔ", "β", "ɸ", "kʷ", "gʷ", "ø", "y", "ɨ",
            ],
            &[],
        ),
        consonant_count: Some((22, 28)),
        vowel_count: Some((5, 5)),
        final_coda: Some(0.35),
        disyllabic_roots: Some(0.55),
        long_vowels: Some(0.35),
        suffixing: Some(0.95),
        derivation: Some(0.7),
        long_spelling: Some(LongVowel::Macron),
        spelling: pairs(&[
            ("ʃ", "ś"),
            ("tʃ", "c"),
            ("dʒ", "j"),
            ("ɲ", "ñ"),
            ("ŋ", "ṅ"),
            ("j", "y"),
        ]),
        ..Flavor::default()
    }
}

/// Fricatives f θ x where other families have stops, three short and three
/// long vowels, as in Old Persian.
fn iranian() -> Flavor {
    Flavor {
        id: "iranian".into(),
        name: "Iranian".into(),
        brief: "Make it sound more like Old Persian.".into(),
        segments: weighted(&[
            ("f", 1.5),
            ("θ", 3.0),
            ("x", 2.0),
            ("ʃ", 1.0),
            ("s", 0.5),
            ("z", 0.5),
            ("h", 0.5),
            ("r", 0.8),
            ("w", 0.5),
        ]),
        require: strings(&["θ", "x", "f", "a", "i", "u"]),
        forbid: forbid(
            &[
                "pʰ", "tʰ", "kʰ", "tʃʰ", "bʱ", "dʱ", "gʱ", "ʈ", "ɖ", "ɳ", "ʂ", "ʐ", "ɭ", "kʷ",
                "gʷ", "ʔ", "e", "o", "ɛ", "ɔ", "ø", "y",
            ],
            &[],
        ),
        vowel_count: Some((3, 3)),
        final_coda: Some(0.45),
        disyllabic_roots: Some(0.55),
        long_vowels: Some(0.3),
        suffixing: Some(0.9),
        long_spelling: Some(LongVowel::Macron),
        spelling: pairs(&[("x", "x"), ("θ", "θ"), ("ʃ", "š"), ("tʃ", "č"), ("dʒ", "j")]),
        ..Flavor::default()
    }
}

/// Short closed syllables, θ ð x, æ and y, long vowels, and Old English
/// spelling (þ, ð, æ, c for k, sc for ʃ).
fn germanic() -> Flavor {
    Flavor {
        id: "germanic".into(),
        name: "Germanic".into(),
        brief: "Make it sound more like Old English.".into(),
        segments: weighted(&[
            ("f", 1.2),
            ("θ", 3.0),
            ("ð", 2.0),
            ("x", 1.5),
            ("ʃ", 0.8),
            ("w", 1.0),
            ("r", 0.5),
            ("l", 0.5),
            ("æ", 2.5),
            ("y", 2.0),
            ("e", 0.5),
            ("o", 0.5),
        ]),
        require: strings(&["θ", "f", "x", "w", "æ", "a", "e", "i", "o", "u"]),
        forbid: forbid(
            &[
                "pʰ", "tʰ", "kʰ", "tʃʰ", "bʱ", "dʱ", "gʱ", "ʈ", "ɖ", "ɳ", "ʂ", "ʐ", "ɭ", "kʷ",
                "gʷ", "ʔ", "ɲ", "ts", "dz", "β", "ɸ",
            ],
            &[],
        ),
        vowel_count: Some((7, 8)),
        final_coda: Some(0.75),
        disyllabic_roots: Some(0.4),
        long_vowels: Some(0.3),
        suffixing: Some(0.95),
        derivation: Some(0.7),
        long_spelling: Some(LongVowel::Macron),
        spelling: pairs(&[
            ("θ", "þ"),
            ("ð", "ð"),
            ("æ", "æ"),
            ("k", "c"),
            ("ʃ", "sc"),
            ("x", "h"),
            ("tʃ", "ċ"),
            ("j", "ġ"),
        ]),
        ..Flavor::default()
    }
}

/// Consonant-skeleton roots with vowel patterns, pharyngeals and uvulars,
/// three vowels with length, and no p.
fn semitic() -> Flavor {
    use Manner::*;
    use Place::*;
    Flavor {
        id: "semitic".into(),
        name: "Semitic".into(),
        brief: "Give it a Semitic, triconsonantal feel, like Arabic or Hebrew.".into(),
        manner: vec![(Fricative, 0.8)],
        place: vec![(Uvular, 1.5), (Pharyngeal, 2.0), (Glottal, 1.5)],
        segments: weighted(&[
            ("ħ", 4.0),
            ("ʕ", 4.0),
            ("q", 2.0),
            ("χ", 2.0),
            ("ʔ", 1.5),
            ("h", 1.0),
            ("ʃ", 1.0),
            ("a", 1.0),
            ("i", 1.0),
            ("u", 1.0),
        ]),
        require: strings(&["ħ", "ʕ", "q", "ʔ", "a", "i", "u"]),
        forbid: forbid(
            &[
                "p", "v", "pʰ", "tʰ", "kʰ", "bʱ", "dʱ", "gʱ", "ʈ", "ɖ", "ɳ", "ʂ", "kʷ", "gʷ",
            ],
            &["ħ", "ʕ", "q", "χ"],
        ),
        vowel_count: Some((3, 3)),
        disyllabic_roots: Some(0.7),
        long_vowels: Some(0.3),
        morphology: Some(MorphologyKind::RootPattern),
        derivation: Some(0.8),
        long_spelling: Some(LongVowel::Macron),
        spelling: pairs(&[("ʃ", "sh"), ("x", "kh"), ("ɣ", "gh")]),
        ..Flavor::default()
    }
}

/// tl, tz, kw, no voiced stops or r, four vowels with length, prefixes and
/// suffixes, and Classical Nahuatl spelling.
fn nahuatl() -> Flavor {
    Flavor {
        id: "nahuatl".into(),
        name: "Nahuatl-like".into(),
        brief: "Give it Nahuatl-like sounds.".into(),
        segments: weighted(&[
            ("tɬ", 8.0),
            ("ts", 2.0),
            ("tʃ", 1.5),
            ("kʷ", 3.0),
            ("ʃ", 1.5),
            ("ʔ", 1.0),
            ("l", 1.0),
            ("w", 1.0),
            ("a", 1.0),
            ("e", 1.0),
            ("i", 1.0),
            ("o", 1.0),
        ]),
        require: strings(&["tɬ", "ts", "kʷ", "a", "e", "i", "o"]),
        forbid: forbid(
            &[
                "b", "d", "g", "f", "v", "θ", "ð", "z", "ʒ", "r", "ɾ", "x", "u", "ɛ", "ɔ", "pʰ",
                "tʰ", "kʰ", "bʱ", "dʱ", "gʱ", "ʈ", "ɖ", "ɳ", "ʂ", "ʐ", "ɭ",
            ],
            &["tɬ"],
        ),
        vowel_count: Some((4, 4)),
        final_coda: Some(0.3),
        disyllabic_roots: Some(0.7),
        long_vowels: Some(0.25),
        suffixing: Some(0.5),
        derivation: Some(0.8),
        long_spelling: Some(LongVowel::Macron),
        spelling: pairs(&[
            ("k", "c"),
            ("kʷ", "cu"),
            ("ʃ", "x"),
            ("tʃ", "ch"),
            ("ts", "tz"),
            ("tɬ", "tl"),
            ("w", "hu"),
            ("j", "y"),
            ("ʔ", "h"),
        ]),
        ..Flavor::default()
    }
}

/// Open syllables, two-syllable roots, five vowels, and class prefixes.
/// Real Bantu languages also have tone and prenasalized stops (mb, nd),
/// which this cannot yet express.
fn bantu() -> Flavor {
    Flavor {
        id: "bantu".into(),
        name: "Bantu-like".into(),
        brief: "Make it more Bantu-like.".into(),
        segments: weighted(&[
            ("m", 0.5),
            ("n", 0.5),
            ("ŋ", 0.5),
            ("ɲ", 0.5),
            ("b", 0.5),
            ("w", 0.5),
            ("a", 1.0),
            ("e", 0.5),
            ("i", 1.0),
            ("o", 0.5),
            ("u", 1.0),
        ]),
        require: strings(&["a", "e", "i", "o", "u"]),
        forbid: forbid(&["θ", "ð", "x", "ɣ", "ʔ", "ø", "y", "ɨ"], &["ɓ", "ɗ"]),
        vowel_count: Some((5, 5)),
        final_coda: Some(0.0),
        open_medial: Some(true),
        disyllabic_roots: Some(0.85),
        long_vowels: Some(0.1),
        suffixing: Some(0.2),
        derivation: Some(0.7),
        long_spelling: Some(LongVowel::Double),
        ..Flavor::default()
    }
}

/// A small inventory, strictly open syllables, long vowels, and easy
/// reduplication, as in Hawaiian or Māori.
fn polynesian() -> Flavor {
    use Manner::*;
    Flavor {
        id: "polynesian".into(),
        name: "Polynesian-like".into(),
        brief: "Make it more Polynesian.".into(),
        manner: vec![(Fricative, -1.0), (Affricate, -3.0)],
        segments: weighted(&[
            ("p", 1.0),
            ("t", 1.0),
            ("k", 1.0),
            ("m", 1.0),
            ("n", 1.0),
            ("ŋ", 1.5),
            ("ʔ", 1.5),
            ("h", 1.0),
            ("w", 1.0),
            ("l", 0.5),
            ("r", 0.5),
            ("f", 0.5),
            ("v", 0.5),
        ]),
        require: strings(&["p", "t", "k", "m", "n", "a", "e", "i", "o", "u"]),
        forbid: forbid(
            &[
                "b", "d", "g", "s", "z", "ʃ", "ʒ", "tʃ", "dʒ", "ts", "dz", "x", "ɣ", "θ", "ð", "ɲ",
                "j", "pʰ", "tʰ", "kʰ", "bʱ", "dʱ", "gʱ", "ʈ", "ɖ", "ɳ", "ʂ", "ɭ", "kʷ", "gʷ", "β",
                "ɸ",
            ],
            &[],
        ),
        consonant_count: Some((8, 10)),
        vowel_count: Some((5, 5)),
        final_coda: Some(0.0),
        open_medial: Some(true),
        disyllabic_roots: Some(0.8),
        identical_consonants: Some(0.4),
        long_vowels: Some(0.25),
        long_spelling: Some(LongVowel::Macron),
        spelling: pairs(&[("ʔ", "ʻ"), ("ŋ", "ng")]),
        ..Flavor::default()
    }
}

/// Front rounded vowels, long vowels written double, two-syllable roots,
/// few voiced stops, and only suffixes, as in Finnish. Real Finnic also has
/// vowel harmony and long consonants, not yet expressible.
fn finnic() -> Flavor {
    use Backness::*;
    use Height::*;
    Flavor {
        id: "finnic".into(),
        name: "Finnic".into(),
        brief: "Make it more like Finnish.".into(),
        height: vec![(Close, 0.5), (CloseMid, 0.5), (NearOpen, 1.0)],
        backness: vec![(Front, 0.5)],
        segments: weighted(&[
            ("y", 3.0),
            ("ø", 3.5),
            ("æ", 3.0),
            ("t", 0.5),
            ("k", 0.5),
            ("s", 0.5),
            ("h", 0.8),
            ("l", 0.5),
            ("r", 0.5),
            ("j", 0.5),
            ("v", 0.8),
        ]),
        require: strings(&["y", "ø", "æ", "a", "e", "i", "o", "u", "h", "j"]),
        forbid: forbid(
            &[
                "b", "g", "f", "z", "ʃ", "ʒ", "tʃ", "dʒ", "θ", "ð", "x", "ɣ", "pʰ", "tʰ", "kʰ",
                "bʱ", "dʱ", "gʱ", "ʈ", "ɖ", "ɳ", "ʂ", "kʷ", "gʷ", "ʔ",
            ],
            &[],
        ),
        vowel_count: Some((8, 8)),
        final_coda: Some(0.4),
        preferred_codas: strings(&["n", "t", "s", "l", "r"]),
        disyllabic_roots: Some(0.85),
        long_vowels: Some(0.3),
        suffixing: Some(1.0),
        derivation: Some(0.8),
        long_spelling: Some(LongVowel::Double),
        spelling: pairs(&[("æ", "ä"), ("ø", "ö"), ("j", "j")]),
        ..Flavor::default()
    }
}

/// Ejectives, uvulars, labialized consonants, heavy consonant inventories,
/// and few vowels, as across the Caucasus.
fn caucasian() -> Flavor {
    Flavor {
        id: "caucasian".into(),
        name: "Caucasian".into(),
        brief: "Give it a Caucasian edge: ejectives, uvulars, few vowels.".into(),
        segments: weighted(&[
            ("pʼ", 4.0),
            ("tʼ", 4.0),
            ("kʼ", 4.0),
            ("q", 2.5),
            ("qʷ", 3.0),
            ("χ", 2.5),
            ("ʁ", 2.0),
            ("kʷ", 1.5),
            ("xʷ", 2.0),
            ("ts", 1.0),
            ("tʃ", 1.0),
            ("ɬ", 1.5),
        ]),
        require: strings(&["pʼ", "tʼ", "kʼ", "q", "χ", "a", "e", "i", "o", "u"]),
        forbid: strings(&[
            "ɓ", "ɗ", "bʱ", "dʱ", "gʱ", "ʈ", "ɖ", "ɳ", "ʂ", "ɭ", "ø", "y",
        ]),
        consonant_count: Some((24, 30)),
        vowel_count: Some((5, 5)),
        final_coda: Some(0.6),
        disyllabic_roots: Some(0.45),
        suffixing: Some(0.7),
        spelling: pairs(&[
            ("pʼ", "p'"),
            ("tʼ", "t'"),
            ("kʼ", "k'"),
            ("χ", "x"),
            ("qʷ", "qw"),
        ]),
        ..Flavor::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::Inventory;
    use crate::phoneme::CATALOG;
    use crate::profile::SoundProfile;
    use crate::rng::stream;

    /// Every palette's required and forbidden sounds are real catalog
    /// segments, and its signature sounds appear in every inventory.
    #[test]
    fn palettes_reach_their_signature_sounds() {
        let neutral = SoundProfile::base();
        for flavor in all() {
            for s in flavor.require.iter().chain(&flavor.forbid) {
                assert!(
                    CATALOG.id_by_ipa(s).is_some(),
                    "{}: unknown segment {s}",
                    flavor.id
                );
            }
            for (s, _) in &flavor.segments {
                assert!(
                    CATALOG.id_by_ipa(s).is_some(),
                    "{}: unknown segment {s}",
                    flavor.id
                );
            }
            let profile = neutral.flavored(&flavor);
            for seed in 0..30 {
                let inv = Inventory::sample(&profile.inventory, &mut stream(seed, &[]));
                for s in &flavor.require {
                    let id = CATALOG.id_by_ipa(s).unwrap();
                    assert!(inv.contains(id), "{} seed {seed} lacks {s}", flavor.id);
                }
                for s in &flavor.forbid {
                    let id = CATALOG.id_by_ipa(s).unwrap();
                    if !flavor.require.contains(s) {
                        assert!(
                            !inv.contains(id),
                            "{} seed {seed} has forbidden {s}",
                            flavor.id
                        );
                    }
                }
            }
        }
    }
}

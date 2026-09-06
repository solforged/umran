use std::collections::{HashMap, HashSet};

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::Language;
use crate::Word;
use crate::generate::NameKind;

#[derive(Clone, Debug)]
pub struct Root {
    pub id: &'static str,
    pub kind: NameKind,
    pub proto: Word,
}

const GLOSSES: &[(&'static str, NameKind)] = &[
    ("person", NameKind::Person),
    ("people", NameKind::Person),
    ("water", NameKind::Word),
    ("stone", NameKind::Word),
    ("tree", NameKind::Word),
    ("hill", NameKind::Place),
    ("star", NameKind::Word),
    ("light", NameKind::Word),
    ("dark", NameKind::Word),
    ("fire", NameKind::Word),
    ("sky", NameKind::Place),
    ("earth", NameKind::Place),
    ("river", NameKind::Place),
    ("sea", NameKind::Place),
    ("house", NameKind::Place),
    ("path", NameKind::Place),
    ("name", NameKind::Word),
    ("child", NameKind::Person),
    ("sun", NameKind::Word),
    ("moon", NameKind::Word),
    ("wind", NameKind::Word),
    ("hand", NameKind::Word),
    ("eye", NameKind::Word),
    ("speak", NameKind::Word),
    ("go", NameKind::Word),
    ("see", NameKind::Word),
    ("life", NameKind::Word),
    ("death", NameKind::Word),
    ("food", NameKind::Word),
    ("war", NameKind::Word),
    ("king", NameKind::Person),
    ("night", NameKind::Word),
    ("day", NameKind::Word),
    ("blood", NameKind::Word),
    ("heart", NameKind::Word),
    ("voice", NameKind::Word),
];

#[derive(Clone, Copy)]
enum Derive {
    Same,
    Redup,
    Clip,
    Person(usize),
    Place(usize),
}

/// Gloss, stem-gloss, how to build the surface form from that stem.
const DERIVE: &[(&'static str, &'static str, Derive)] = &[
    ("person", "person", Derive::Same),
    ("people", "person", Derive::Person(0)),
    ("child", "person", Derive::Redup),
    ("king", "person", Derive::Person(1)),
    ("water", "water", Derive::Same),
    ("river", "water", Derive::Place(0)),
    ("sea", "water", Derive::Place(1)),
    ("stone", "stone", Derive::Same),
    ("tree", "tree", Derive::Same),
    ("hill", "hill", Derive::Same),
    ("earth", "hill", Derive::Place(0)),
    ("house", "hill", Derive::Place(1)),
    ("star", "star", Derive::Same),
    ("light", "light", Derive::Same),
    ("sun", "light", Derive::Place(0)),
    ("day", "light", Derive::Redup),
    ("fire", "light", Derive::Clip),
    ("dark", "dark", Derive::Same),
    ("night", "dark", Derive::Place(0)),
    ("death", "dark", Derive::Redup),
    ("sky", "wind", Derive::Place(1)),
    ("wind", "wind", Derive::Same),
    ("path", "go", Derive::Place(0)),
    ("go", "go", Derive::Same),
    ("name", "speak", Derive::Person(2)),
    ("speak", "speak", Derive::Same),
    ("voice", "speak", Derive::Person(0)),
    ("see", "see", Derive::Same),
    ("eye", "see", Derive::Clip),
    ("life", "life", Derive::Same),
    ("heart", "life", Derive::Clip),
    ("blood", "life", Derive::Redup),
    ("hand", "hand", Derive::Same),
    ("food", "food", Derive::Same),
    ("war", "war", Derive::Same),
    ("moon", "moon", Derive::Same),
];

pub fn glosses() -> &'static [(&'static str, NameKind)] {
    GLOSSES
}

/// Stem gloss for a founding lexeme that DERIVE builds from another stem.
pub(crate) fn founding_stem(gloss: &str) -> Option<&'static str> {
    DERIVE.iter().find_map(|&(item, stem, derive)| {
        (item == gloss && !matches!(derive, Derive::Same)).then_some(stem)
    })
}

pub fn mint_roots(lang: &Language) -> Vec<Root> {
    let mut rng = StdRng::seed_from_u64(lang.seed.wrapping_add(11));
    let mut seen = HashSet::new();
    let mut stems: HashMap<&str, Word> = HashMap::new();
    for &(_, stem, _) in DERIVE {
        if stems.contains_key(stem) {
            continue;
        }
        stems.insert(stem, mint_stem(lang, &mut rng, &mut seen));
    }

    GLOSSES
        .iter()
        .map(|&(id, kind)| {
            let proto = DERIVE
                .iter()
                .find(|(gloss, _, _)| *gloss == id)
                .and_then(|(_, stem, derive)| {
                    stems
                        .get(stem)
                        .map(|word| apply_derive(word, *derive, lang))
                })
                .unwrap_or_else(|| {
                    stems
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| mint_stem(lang, &mut rng, &mut seen))
                });
            Root { id, kind, proto }
        })
        .collect()
}

fn mint_stem(lang: &Language, rng: &mut StdRng, seen: &mut HashSet<String>) -> Word {
    for _ in 0..48 {
        let word = Word {
            syllables: vec![lang.generator.root_syllable(rng)],
            join_at: None,
        };
        let roman = lang.romanize(&word);
        let letters = roman.chars().filter(|c| c.is_alphabetic()).count();
        if letters >= 1 && seen.insert(roman) {
            return word;
        }
    }
    Word {
        syllables: vec![lang.generator.root_syllable(rng)],
        join_at: None,
    }
}

fn apply_derive(stem: &Word, derive: Derive, lang: &Language) -> Word {
    match derive {
        Derive::Same => stem.clone(),
        Derive::Redup => stem.redup_first(),
        Derive::Clip => {
            let clip = stem.first_syllable();
            if clip.syllables.is_empty() {
                stem.clone()
            } else {
                clip
            }
        }
        Derive::Person(index) => attach_or_redup(stem, lang, NameKind::Person, index),
        Derive::Place(index) => attach_or_redup(stem, lang, NameKind::Place, index),
    }
}

fn attach_or_redup(stem: &Word, lang: &Language, kind: NameKind, index: usize) -> Word {
    let endings = match kind {
        NameKind::Person => &lang.aesthetic.names.person_endings,
        NameKind::Place => &lang.aesthetic.names.place_endings,
        NameKind::Word => return stem.clone(),
    };
    if endings.is_empty() {
        return stem.redup_first();
    }
    let raw = &endings[index % endings.len()];
    let next = stem.attach_ipa(raw, &lang.inventory);
    if next.phones() != stem.phones() {
        return next;
    }
    stem.redup_first()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Aesthetic;
    use crate::CATALOG;
    use std::collections::HashSet;

    fn flattened_ipa(word: &Word) -> String {
        let ids: Vec<_> = word.phonemes().collect();
        CATALOG.ipa_string(&ids)
    }

    #[test]
    fn thirty_six_unique_glosses() {
        assert_eq!(glosses().len(), 36);
        let ids: HashSet<_> = glosses().iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), 36);
        let derived: HashSet<_> = DERIVE.iter().map(|(id, _, _)| *id).collect();
        for (id, _) in glosses() {
            assert!(derived.contains(id), "missing derive for {id}");
        }
    }

    #[test]
    fn mint_roots_matches_glosses_and_is_deterministic() {
        let aesthetic = Aesthetic::by_id("elvish").unwrap();
        let lang = Language::new(42, aesthetic.clone());
        let a = mint_roots(&lang);
        let b = mint_roots(&lang);
        assert_eq!(a.len(), glosses().len());
        assert_eq!(b.len(), glosses().len());
        for ((ra, rb), (id, kind)) in a.iter().zip(b.iter()).zip(glosses()) {
            assert_eq!(ra.id, *id);
            assert_eq!(rb.id, *id);
            assert_eq!(ra.kind, *kind);
            assert_eq!(flattened_ipa(&ra.proto), flattened_ipa(&rb.proto));
        }

        let other = mint_roots(&Language::new(43, aesthetic));
        assert!(
            a.iter()
                .zip(other.iter())
                .any(|(x, y)| flattened_ipa(&x.proto) != flattened_ipa(&y.proto))
        );
    }

    #[test]
    fn person_family_shares_a_stem() {
        let lang = Language::new(42, Aesthetic::by_id("elvish").unwrap());
        let roots = mint_roots(&lang);
        let get = |id: &str| {
            roots
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.proto.phones())
                .unwrap()
        };
        let person = get("person");
        let people = get("people");
        let child = get("child");
        assert!(
            people.starts_with(&person) || person.starts_with(&people),
            "people should be person plus morphology"
        );
        let first = roots
            .iter()
            .find(|r| r.id == "person")
            .unwrap()
            .proto
            .first_syllable()
            .phones();
        assert!(
            child.starts_with(&first),
            "child should reduplicate the person stem"
        );
    }

    #[test]
    fn proto_stems_are_one_syllable() {
        let lang = Language::new(42, Aesthetic::by_id("elvish").unwrap());
        let roots = mint_roots(&lang);
        for &(gloss, _, derive) in DERIVE {
            if matches!(derive, Derive::Same) {
                let root = roots.iter().find(|r| r.id == gloss).unwrap();
                assert_eq!(
                    root.proto.syllables.len(),
                    1,
                    "{gloss} proto should be one syllable"
                );
            }
        }
    }
}

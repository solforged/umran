use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::Language;
use crate::NameKind;
use crate::Word;

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

pub fn glosses() -> &'static [(&'static str, NameKind)] {
    GLOSSES
}

pub fn mint_roots(lang: &Language) -> Vec<Root> {
    let mut rng = StdRng::seed_from_u64(lang.seed.wrapping_add(11));
    GLOSSES
        .iter()
        .map(|&(id, kind)| {
            let mut proto = lang.word(&mut rng);
            for _ in 1..8 {
                if !lang.romanize(&proto).is_empty() {
                    break;
                }
                proto = lang.word(&mut rng);
            }
            Root { id, kind, proto }
        })
        .collect()
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
        assert!(a
            .iter()
            .zip(other.iter())
            .any(|(x, y)| flattened_ipa(&x.proto) != flattened_ipa(&y.proto)));
    }
}

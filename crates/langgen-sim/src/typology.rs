//! How common each catalog segment is across the world's languages, for
//! weighting sound choices toward what human languages actually use.
//!
//! Counts of inventories containing the segment, out of 3,020, from PHOIBLE
//! 2.0 (Moran and McCloy, eds., 2019), <https://phoible.org/parameters>,
//! retrieved 2026-09-30. Where PHOIBLE splits a sound by detail (dental
//! versus alveolar t, t̠ʃ for tʃ), the more common variant is used.

use crate::phoneme::{CATALOG, PhonemeId};

/// Inventories in PHOIBLE 2.0.
const INVENTORIES: f32 = 3020.0;

const COUNTS: &[(&str, u32)] = &[
    ("p", 2594),
    ("b", 1906),
    ("t", 2064),
    ("d", 1376),
    ("k", 2730),
    ("g", 1712),
    ("q", 256),
    ("ʔ", 1131),
    ("m", 2914),
    ("n", 2349),
    ("ɲ", 1255),
    ("ŋ", 1897),
    ("r", 1332),
    ("ɾ", 774),
    ("f", 1329),
    ("v", 816),
    ("θ", 123),
    ("ð", 160),
    ("s", 2020),
    ("z", 893),
    ("ʃ", 1104),
    ("ʒ", 478),
    ("x", 576),
    ("ɣ", 436),
    ("h", 1703),
    ("ts", 666),
    ("dz", 312),
    ("tʃ", 1218),
    ("dʒ", 820),
    ("w", 2483),
    ("j", 2716),
    ("l", 2044),
    ("ʎ", 147),
    ("ɬ", 149),
    ("ɓ", 300),
    ("ɗ", 248),
    ("pʼ", 178),
    ("tʼ", 156),
    ("kʼ", 242),
    ("i", 2779),
    ("y", 175),
    ("ɨ", 491),
    ("u", 2646),
    ("ɪ", 444),
    ("ʊ", 409),
    ("e", 1841),
    ("ø", 94),
    ("ə", 675),
    ("o", 1826),
    ("ɛ", 1129),
    ("ɔ", 1070),
    ("æ", 223),
    ("a", 2600),
    ("ɑ", 225),
];

/// Share of the world's inventories that have this segment, 0–1.
pub fn share(id: PhonemeId) -> f32 {
    let ipa = CATALOG.get(id).ipa();
    COUNTS
        .iter()
        .find(|(s, _)| *s == ipa)
        .map_or(0.01, |(_, n)| *n as f32 / INVENTORIES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_segment_has_a_count() {
        for (i, seg) in CATALOG.segments.iter().enumerate() {
            assert!(
                COUNTS.iter().any(|(s, _)| *s == seg.ipa()),
                "no PHOIBLE count for {}",
                seg.ipa()
            );
            assert!(share(PhonemeId(i as u16)) > 0.0);
        }
    }

    #[test]
    fn common_sounds_outrank_rare_ones() {
        let id = |s: &str| CATALOG.id_by_ipa(s).unwrap();
        assert!(share(id("m")) > 0.9 && share(id("i")) > 0.9);
        assert!(share(id("θ")) < 0.1 && share(id("ø")) < 0.1);
    }
}

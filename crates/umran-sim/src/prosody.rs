//! The smallest word a language tolerates, its prosodic minimum. Many
//! languages will not let sound change wear a word below a certain size:
//! final vowels drop from long words but stay on short ones (Finnish keeps
//! the -i of käsi "hand"), and a lone light syllable is not a word (English
//! has no content words like *bɪ, only bit or bee). Below the minimum, a
//! word is felt to be worn, and speakers renew it from fuller material.

use crate::form::Form;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Predictable word stress, or a lexical syllable carried by each form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StressRule {
    Initial,
    Penult,
    Final,
    /// Heavy penult, otherwise antepenult (or the first of two syllables).
    Weight,
    Free,
}

impl StressRule {
    pub fn draw(rng: &mut impl Rng) -> Self {
        match crate::rng::index(rng, 10) {
            0..=2 => Self::Initial,
            3..=5 => Self::Penult,
            6 => Self::Final,
            7..=8 => Self::Weight,
            _ => Self::Free,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Penult => "penult",
            Self::Final => "final",
            Self::Weight => "weight",
            Self::Free => "free",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinimalWord {
    /// Any syllable will do, even a bare vowel (French eau, Mandarin è).
    Syllable,
    /// Two moras: a long vowel or a closing consonant, or two syllables
    /// (English, Latin, Arabic, Japanese).
    Heavy,
    /// Two syllables (Bantu, Polynesian, Finnic, Australian languages).
    TwoSyllables,
}

/// Chance that a language with no disyllabic minimum tolerates any lone
/// light syllable; most have a bimoraic minimum instead.
const LIGHT_SHARE: f32 = 0.35;

impl MinimalWord {
    /// Languages whose roots are mostly disyllabic tend to require two
    /// syllables; the rest usually require a heavy one.
    pub fn draw(disyllabic_roots: f32, rng: &mut impl Rng) -> Self {
        let roots = disyllabic_roots.clamp(0.0, 1.0);
        let two = roots * roots;
        if rng.r#gen::<f32>() < two {
            Self::TwoSyllables
        } else if rng.r#gen::<f32>() < LIGHT_SHARE {
            Self::Syllable
        } else {
            Self::Heavy
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Syllable => "a single syllable",
            Self::Heavy => "a heavy syllable",
            Self::TwoSyllables => "two syllables",
        }
    }

    /// A word's size in the units this minimum counts.
    fn size(self, form: &Form) -> usize {
        match self {
            Self::Syllable | Self::TwoSyllables => form.vowel_count(),
            Self::Heavy => moras(form),
        }
    }

    fn least(self) -> usize {
        match self {
            Self::Syllable => 1,
            Self::Heavy | Self::TwoSyllables => 2,
        }
    }

    /// Whether a change from `before` to `after` would wear a word below
    /// the minimum, or further below it. Such a change passes the word by.
    pub fn blocks(self, before: &Form, after: &Form) -> bool {
        let is = self.size(after);
        is < self.least() && is < self.size(before)
    }

    /// Whether a word is below the minimum, as words founded short or
    /// borrowed can be.
    pub fn worn(self, form: &Form) -> bool {
        self.size(form) < self.least()
    }
}

/// Moras: one per syllable, one more for a long vowel, and one more for a
/// closing consonant.
pub fn moras(form: &Form) -> usize {
    form.syllables()
        .iter()
        .map(|s| 1 + usize::from(form.segs[s.nucleus].long) + usize::from(!s.coda.is_empty()))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(ipa: &str) -> Form {
        Form::from_ipa(ipa).unwrap()
    }

    #[test]
    fn counts_moras() {
        assert_eq!(moras(&form("ta")), 1);
        assert_eq!(moras(&form("tat")), 2);
        assert_eq!(moras(&form("taː")), 2);
        assert_eq!(moras(&form("kata")), 2);
        assert_eq!(moras(&form("kasta")), 3);
    }

    #[test]
    fn minimum_blocks_only_wear_below_it() {
        let heavy = MinimalWord::Heavy;
        assert!(heavy.blocks(&form("tat"), &form("ta")));
        assert!(!heavy.blocks(&form("tata"), &form("tat")));
        assert!(!heavy.blocks(&form("ta"), &form("da")), "not wear");
        let two = MinimalWord::TwoSyllables;
        assert!(two.blocks(&form("kata"), &form("kat")));
        assert!(!two.blocks(&form("katata"), &form("katat")));
        assert!(two.worn(&form("kat")) && !two.worn(&form("kata")));
        assert!(!MinimalWord::Syllable.blocks(&form("tat"), &form("ta")));
    }
}

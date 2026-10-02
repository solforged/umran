use crate::phoneme::{CATALOG, PhonemeId};
use crate::prosody::StressRule;
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Seg {
    pub phone: PhonemeId,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub long: bool,
}

/// A word's sounds as a flat segment string.
///
/// Syllables are derived on demand rather than stored, so a sound change
/// can never leave stale syllable structure behind.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Form {
    pub segs: Vec<Seg>,
    /// Offsets where a later morpheme begins; never 0 or `segs.len()`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundaries: Vec<usize>,
    /// Lexical stress for free-stress languages; predictable stress ignores
    /// this. An unaccented new form receives initial lexical stress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stress: Option<usize>,
}

/// One syllable as index ranges into `Form::segs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Syllable {
    pub onset: Range<usize>,
    pub nucleus: usize,
    pub coda: Range<usize>,
}

impl Form {
    pub fn from_phones(phones: impl IntoIterator<Item = PhonemeId>) -> Self {
        Self {
            segs: phones
                .into_iter()
                .map(|phone| Seg { phone, long: false })
                .collect(),
            boundaries: Vec::new(),
            stress: None,
        }
    }

    /// Parses IPA such as `kaːs`; `ː` lengthens the preceding segment.
    pub fn from_ipa(raw: &str) -> Option<Self> {
        let mut segs: Vec<Seg> = Vec::new();
        for part in raw.split('ː') {
            if !segs.is_empty() {
                segs.last_mut()?.long = true;
            }
            segs.extend(
                CATALOG
                    .parse_ipa(part)?
                    .into_iter()
                    .map(|phone| Seg { phone, long: false }),
            );
        }
        Some(Self {
            segs,
            boundaries: Vec::new(),
            stress: None,
        })
    }

    pub fn ipa(&self) -> String {
        let mut out = String::new();
        for seg in &self.segs {
            out.push_str(CATALOG.get(seg.phone).ipa());
            if seg.long {
                out.push('ː');
            }
        }
        out
    }

    pub fn phones(&self) -> impl Iterator<Item = PhonemeId> + '_ {
        self.segs.iter().map(|s| s.phone)
    }

    pub fn is_vowel(&self, i: usize) -> bool {
        CATALOG.get(self.segs[i].phone).is_vowel()
    }

    pub fn vowel_count(&self) -> usize {
        (0..self.segs.len()).filter(|&i| self.is_vowel(i)).count()
    }
    pub fn stressed_syllable(&self, rule: StressRule) -> Option<usize> {
        let count = self.vowel_count();
        if count == 0 {
            return None;
        }
        Some(match rule {
            StressRule::Initial => 0,
            StressRule::Penult => count.saturating_sub(2),
            StressRule::Final => count - 1,
            StressRule::Free => self.stress.unwrap_or(0).min(count - 1),
            StressRule::Weight if count < 3 => 0,
            StressRule::Weight => {
                let penult = count - 2;
                let mut nuclei = (0..self.segs.len()).filter(|&i| self.is_vowel(i));
                let nucleus = nuclei.nth(penult).unwrap();
                let last = nuclei.next().unwrap();
                let onset = self.onset_start(nucleus + 1, last);
                let heavy = self.segs[nucleus].long
                    || onset > nucleus + 1
                    || (onset < last && self.segs[onset].long);
                if heavy { penult } else { count - 3 }
            }
        })
    }

    /// Display IPA, without changing the stress-free serialization/parser.
    /// A shared geminate is written once, with the mark before its onset.
    pub fn ipa_stressed(&self, rule: StressRule) -> String {
        if self.vowel_count() < 2 {
            return self.ipa();
        }
        let syllables = self.syllables();
        let start = syllables[self.stressed_syllable(rule).unwrap()].onset.start;
        let mut out = String::new();
        for (i, seg) in self.segs.iter().enumerate() {
            if i == start {
                out.push('ˈ');
            }
            out.push_str(CATALOG.get(seg.phone).ipa());
            if seg.long {
                out.push('ː');
            }
        }
        out
    }

    /// Keep lexical stress on its surviving nucleus. If that nucleus is
    /// deleted, use the next surviving vowel, or the last if none follows.
    pub(crate) fn stress_after(&self, outcome: &[Option<Seg>]) -> Option<usize> {
        let stressed = self.stress?;
        let nucleus = (0..self.segs.len())
            .filter(|&i| self.is_vowel(i))
            .nth(stressed)?;
        let before = outcome[..nucleus]
            .iter()
            .filter(|s| s.is_some_and(|s| CATALOG.get(s.phone).is_vowel()))
            .count();
        let count = outcome
            .iter()
            .filter(|s| s.is_some_and(|s| CATALOG.get(s.phone).is_vowel()))
            .count();
        (count > 0).then(|| before.min(count - 1))
    }

    /// Syllables by maximal onset: each intervocalic cluster gives the
    /// following vowel the longest tail of rising sonority. A form with no
    /// vowel has no syllables.
    pub fn syllables(&self) -> Vec<Syllable> {
        let nuclei: Vec<usize> = (0..self.segs.len()).filter(|&i| self.is_vowel(i)).collect();
        let mut out: Vec<Syllable> = Vec::with_capacity(nuclei.len());
        for (n, &v) in nuclei.iter().enumerate() {
            let onset_start = match n {
                0 => 0,
                _ => {
                    let from = nuclei[n - 1] + 1;
                    let start = self.onset_start(from, v);
                    // A geminate is one segment shared by coda and onset.
                    let shared = usize::from(start < v && self.segs[start].long);
                    out.last_mut().unwrap().coda = from..start + shared;
                    start
                }
            };
            out.push(Syllable {
                onset: onset_start..v,
                nucleus: v,
                coda: v + 1..v + 1,
            });
        }
        if let Some(last) = out.last_mut() {
            last.coda = last.nucleus + 1..self.segs.len();
        }
        out
    }

    fn onset_start(&self, from: usize, to: usize) -> usize {
        let sonority = |i: usize| CATALOG.get(self.segs[i].phone).sonority();
        let from = (from..to).rfind(|&i| self.segs[i].long).unwrap_or(from);
        (from..=to)
            .find(|&start| (start + 1..to).all(|i| sonority(i - 1) < sonority(i)))
            .unwrap_or(to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(form: &Form) -> String {
        let ipa = |r: Range<usize>| {
            form.segs[r]
                .iter()
                .map(|s| CATALOG.get(s.phone).ipa())
                .collect::<String>()
        };
        form.syllables()
            .into_iter()
            .map(|s| {
                format!(
                    "{}{}{}",
                    ipa(s.onset),
                    ipa(s.nucleus..s.nucleus + 1),
                    ipa(s.coda)
                )
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    #[test]
    fn ipa_round_trips_with_length() {
        let form = Form::from_ipa("kaːsta").unwrap();
        assert!(form.segs[1].long);
        assert_eq!(form.ipa(), "kaːsta");
    }

    #[test]
    fn syllabifies_by_maximal_rising_onset() {
        assert_eq!(shape(&Form::from_ipa("kasta").unwrap()), "kas.ta");
        assert_eq!(shape(&Form::from_ipa("katra").unwrap()), "ka.tra");
        assert_eq!(shape(&Form::from_ipa("aia").unwrap()), "a.i.a");
        assert_eq!(shape(&Form::from_ipa("strak").unwrap()), "strak");
    }

    #[test]
    fn vowelless_form_has_no_syllables() {
        assert!(Form::from_ipa("st").unwrap().syllables().is_empty());
    }
}

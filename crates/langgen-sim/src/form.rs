use crate::phoneme::{CATALOG, PhonemeId};
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
                    out.last_mut().unwrap().coda = from..start;
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

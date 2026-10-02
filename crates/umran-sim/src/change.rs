use crate::form::{Form, Seg};
use crate::phoneme::{Backness, CATALOG, Height, Manner, PhonemeId, Place, Secondary, Segment};
use crate::prosody::StressRule;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A natural class of segments; unset features match anything.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Matcher {
    Phone(PhonemeId),
    Consonant {
        place: Option<Place>,
        manner: Option<Manner>,
        voiced: Option<bool>,
        #[serde(default)]
        secondary: Option<Secondary>,
    },
    Vowel {
        height: Option<Height>,
        backness: Option<Backness>,
        rounded: Option<bool>,
    },
    AnyConsonant,
    AnyVowel,
    Length {
        target: Box<Matcher>,
        long: bool,
    },
    Stressed {
        target: Box<Matcher>,
        stressed: bool,
    },
    OpenSyllable(Box<Matcher>),
    /// A noninitial, nonfinal vowel nucleus.
    MedialVowel,
}

/// What a matched segment becomes. A feature rewrite with no catalog
/// segment for the resulting bundle leaves the segment unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Rewrite {
    Phone(PhonemeId),
    /// Unset features, including the secondary articulation, are kept.
    Consonant {
        place: Option<Place>,
        manner: Option<Manner>,
        voiced: Option<bool>,
        #[serde(default)]
        secondary: Option<Secondary>,
    },
    Vowel {
        height: Option<Height>,
        backness: Option<Backness>,
        rounded: Option<bool>,
    },
    Delete,
    Length(bool),
    /// Delete this consonant and lengthen the following consonant.
    GeminateNext,
    /// Delete this consonant and lengthen the preceding vowel.
    Compensate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Env {
    Any,
    WordEdge,
    Matcher(Matcher),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundChange {
    pub id: String,
    pub target: Matcher,
    pub result: Rewrite,
    pub left: Env,
    pub right: Env,
}

impl Matcher {
    pub fn matches(&self, id: PhonemeId) -> bool {
        let seg = CATALOG.get(id);
        match (self, seg) {
            (Matcher::Phone(want), _) => *want == id,
            (Matcher::AnyConsonant, _) => !seg.is_vowel(),
            (Matcher::AnyVowel, _) => seg.is_vowel(),
            (
                Matcher::Consonant {
                    place,
                    manner,
                    voiced,
                    secondary,
                },
                Segment::Consonant(c),
            ) => {
                place.is_none_or(|p| p == c.place)
                    && manner.is_none_or(|m| m == c.manner)
                    && voiced.is_none_or(|v| v == c.voiced)
                    && secondary.is_none_or(|s| s == c.secondary)
            }
            (
                Matcher::Vowel {
                    height,
                    backness,
                    rounded,
                },
                Segment::Vowel(v),
            ) => {
                height.is_none_or(|h| h == v.height)
                    && backness.is_none_or(|b| b == v.backness)
                    && rounded.is_none_or(|r| r == v.rounded)
            }
            (
                Matcher::Length { target, .. }
                | Matcher::Stressed { target, .. }
                | Matcher::OpenSyllable(target),
                _,
            ) => target.matches(id),
            (Matcher::MedialVowel, _) => seg.is_vowel(),
            _ => false,
        }
    }
    fn needs_syllables(&self) -> bool {
        match self {
            Self::Stressed { .. } | Self::OpenSyllable(_) => true,
            Self::Length { target, .. } => target.needs_syllables(),
            _ => false,
        }
    }

    fn at(
        &self,
        form: &Form,
        i: usize,
        syllables: &[crate::form::Syllable],
        stress: Option<usize>,
    ) -> bool {
        match self {
            Self::Length { target, long } => {
                form.segs[i].long == *long && target.at(form, i, syllables, stress)
            }
            Self::Stressed { target, stressed } => {
                let syllable = syllables
                    .iter()
                    .position(|s| s.onset.contains(&i) || s.nucleus == i || s.coda.contains(&i));
                syllable.is_some()
                    && (syllable == stress) == *stressed
                    && target.at(form, i, syllables, stress)
            }
            Self::OpenSyllable(target) => {
                syllables
                    .iter()
                    .any(|s| s.nucleus == i && s.coda.is_empty())
                    && target.at(form, i, syllables, stress)
            }
            Self::MedialVowel => {
                form.is_vowel(i)
                    && (0..i).any(|j| form.is_vowel(j))
                    && (i + 1..form.segs.len()).any(|j| form.is_vowel(j))
            }
            _ => self.matches(form.segs[i].phone),
        }
    }
}

impl Env {
    fn needs_syllables(&self) -> bool {
        matches!(self, Self::Matcher(m) if m.needs_syllables())
    }

    fn at(
        &self,
        form: &Form,
        i: Option<usize>,
        syllables: &[crate::form::Syllable],
        stress: Option<usize>,
    ) -> bool {
        match self {
            Self::Any => true,
            Self::WordEdge => i.is_none(),
            Self::Matcher(m) => i.is_some_and(|i| m.at(form, i, syllables, stress)),
        }
    }
}

impl Rewrite {
    /// `None` deletes the segment.
    fn apply(&self, id: PhonemeId) -> Option<PhonemeId> {
        match (self, CATALOG.get(id)) {
            (Rewrite::Delete | Rewrite::GeminateNext | Rewrite::Compensate, _) => None,
            (Rewrite::Phone(p), _) => Some(*p),
            (
                Rewrite::Consonant {
                    place,
                    manner,
                    voiced,
                    secondary,
                },
                Segment::Consonant(c),
            ) => {
                let (place, manner, voiced, secondary) = (
                    place.unwrap_or(c.place),
                    manner.unwrap_or(c.manner),
                    voiced.unwrap_or(c.voiced),
                    secondary.unwrap_or(c.secondary),
                );
                Some(
                    CATALOG
                        .consonants()
                        .find(|(_, k)| {
                            k.place == place
                                && k.manner == manner
                                && k.voiced == voiced
                                && k.secondary == secondary
                        })
                        .map_or(id, |(cid, _)| cid),
                )
            }
            (
                Rewrite::Vowel {
                    height,
                    backness,
                    rounded,
                },
                Segment::Vowel(v),
            ) => {
                let (height, backness, rounded) = (
                    height.unwrap_or(v.height),
                    backness.unwrap_or(v.backness),
                    rounded.unwrap_or(v.rounded),
                );
                Some(
                    CATALOG
                        .vowels()
                        .find(|(_, k)| {
                            k.height == height && k.backness == backness && k.rounded == rounded
                        })
                        .map_or(id, |(vid, _)| vid),
                )
            }
            _ => Some(id),
        }
    }
}

impl SoundChange {
    /// Every match reads the input, including stress and length.
    pub fn hits<'a>(
        &'a self,
        form: &'a Form,
        rule: StressRule,
    ) -> impl Iterator<Item = (usize, Option<Seg>)> + 'a {
        let needs = self.target.needs_syllables()
            || self.left.needs_syllables()
            || self.right.needs_syllables();
        let syllables = if needs { form.syllables() } else { Vec::new() };
        let stress = if needs {
            form.stressed_syllable(rule)
        } else {
            None
        };
        (0..form.segs.len()).filter_map(move |i| {
            let seg = form.segs[i];
            if !self.target.at(form, i, &syllables, stress)
                || !self.left.at(form, i.checked_sub(1), &syllables, stress)
                || !self.right.at(
                    form,
                    (i + 1 < form.segs.len()).then_some(i + 1),
                    &syllables,
                    stress,
                )
            {
                return None;
            }
            let out = match self.result {
                Rewrite::Length(long) => Some(Seg { long, ..seg }),
                _ => self
                    .result
                    .apply(seg.phone)
                    .map(|phone| Seg { phone, ..seg }),
            };
            (out != Some(seg)).then_some((i, out))
        })
    }

    /// Simultaneous application, last-vowel protection, and remapping of
    /// morpheme boundaries and lexical stress to surviving nuclei.
    pub fn apply(&self, form: &Form, rule: StressRule) -> Form {
        self.apply_borrowed(form, rule).into_owned()
    }

    /// Most candidate laws do not touch a word. Borrow those inputs instead
    /// of allocating an outcome and remapping unchanged segments.
    pub(crate) fn apply_borrowed<'a>(&self, form: &'a Form, rule: StressRule) -> Cow<'a, Form> {
        let mut hits = self.hits(form, rule);
        let first = hits.next();
        if first.is_none()
            && form
                .boundaries
                .iter()
                .all(|&b| b > 0 && b < form.segs.len())
            && form.boundaries.windows(2).all(|b| b[0] != b[1])
            && form.stress.is_none_or(|s| s < form.vowel_count())
        {
            return Cow::Borrowed(form);
        }
        let mut outcome: Vec<Option<Seg>> = form.segs.iter().copied().map(Some).collect();
        for (i, out) in first.into_iter().chain(hits) {
            outcome[i] = out;
        }
        // Side effects also read the input and never resurrect a deletion.
        if matches!(self.result, Rewrite::GeminateNext | Rewrite::Compensate) {
            for (i, _) in self.hits(form, rule) {
                let neighbor = match self.result {
                    Rewrite::GeminateNext => {
                        (i + 1 < form.segs.len() && !form.is_vowel(i + 1)).then_some(i + 1)
                    }
                    Rewrite::Compensate => i.checked_sub(1).filter(|&j| form.is_vowel(j)),
                    _ => None,
                };
                if let Some(j) = neighbor
                    && let Some(seg) = &mut outcome[j]
                {
                    seg.long = true;
                }
            }
        }
        let vowel_survives = outcome
            .iter()
            .any(|o| o.is_some_and(|s| CATALOG.get(s.phone).is_vowel()));
        if !vowel_survives && let Some(i) = (0..form.segs.len()).rev().find(|&i| form.is_vowel(i)) {
            outcome[i] = Some(form.segs[i]);
        }
        let stress = form.stress_after(&outcome);
        let mut segs = Vec::with_capacity(form.segs.len());
        let mut new_index = Vec::with_capacity(form.segs.len() + 1);
        for out in &outcome {
            new_index.push(segs.len());
            if let Some(seg) = out {
                segs.push(*seg);
            }
        }
        new_index.push(segs.len());
        let mut boundaries: Vec<usize> = form
            .boundaries
            .iter()
            .map(|&b| new_index[b])
            .filter(|&b| b > 0 && b < segs.len())
            .collect();
        boundaries.dedup();
        Cow::Owned(Form {
            segs,
            boundaries,
            stress,
        })
    }
}

pub fn apply_all(form: &Form, rules: &[SoundChange], stress: StressRule) -> Form {
    rules
        .iter()
        .fold(form.clone(), |f, rule| rule.apply(&f, stress))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(ipa: &str) -> PhonemeId {
        CATALOG.id_by_ipa(ipa).unwrap()
    }

    fn form(ipa: &str) -> Form {
        Form::from_ipa(ipa).unwrap()
    }

    fn t_to_s_before_i() -> SoundChange {
        SoundChange {
            id: "t>s/_i".into(),
            target: Matcher::Consonant {
                place: Some(Place::Alveolar),
                manner: Some(Manner::Stop),
                voiced: Some(false),
                secondary: None,
            },
            result: Rewrite::Consonant {
                place: None,
                manner: Some(Manner::Fricative),
                voiced: None,
                secondary: None,
            },
            left: Env::Any,
            right: Env::Matcher(Matcher::Vowel {
                height: Some(Height::Close),
                backness: Some(Backness::Front),
                rounded: Some(false),
            }),
        }
    }

    fn apocope() -> SoundChange {
        SoundChange {
            id: "apocope".into(),
            target: Matcher::AnyVowel,
            result: Rewrite::Delete,
            left: Env::Any,
            right: Env::WordEdge,
        }
    }

    #[test]
    fn rewrites_by_features_in_context() {
        assert_eq!(
            t_to_s_before_i()
                .apply(&form("tita"), StressRule::Initial)
                .ipa(),
            "sita"
        );
    }

    #[test]
    fn never_deletes_the_last_vowel() {
        assert_eq!(
            apocope().apply(&form("kata"), StressRule::Initial).ipa(),
            "kat"
        );
        assert_eq!(
            apocope().apply(&form("ta"), StressRule::Initial).ipa(),
            "ta"
        );
    }

    #[test]
    fn missing_feature_bundle_keeps_the_segment() {
        let rule = SoundChange {
            id: "no-labiodental-stop".into(),
            target: Matcher::Phone(id("t")),
            result: Rewrite::Consonant {
                place: Some(Place::Labiodental),
                manner: Some(Manner::Stop),
                voiced: Some(false),
                secondary: None,
            },
            left: Env::Any,
            right: Env::Any,
        };
        assert_eq!(rule.apply(&form("ta"), StressRule::Initial).ipa(), "ta");
    }

    #[test]
    fn applies_simultaneously() {
        // a > e after e: only the first a follows an e in the input. Applied
        // left to right in place, the new e would also raise the second a.
        let rule = SoundChange {
            id: "a>e/e_".into(),
            target: Matcher::Phone(id("a")),
            result: Rewrite::Phone(id("e")),
            left: Env::Matcher(Matcher::Phone(id("e"))),
            right: Env::Any,
        };
        assert_eq!(rule.apply(&form("keaa"), StressRule::Initial).ipa(), "keea");
    }

    #[test]
    fn length_and_boundaries_follow_surviving_segments() {
        let mut word = form("kaːtiti");
        word.boundaries = vec![2, 4];
        let out = apply_all(&word, &[t_to_s_before_i(), apocope()], StressRule::Initial);
        assert_eq!(out.ipa(), "kaːsis");
        assert!(out.segs[1].long);
        assert_eq!(out.boundaries, vec![2, 4]);

        let mut word = form("katia");
        word.boundaries = vec![4];
        let out = apocope().apply(&word, StressRule::Initial);
        assert_eq!(out.ipa(), "kati");
        assert!(
            out.boundaries.is_empty(),
            "a boundary at the new word end is dropped"
        );
    }
}

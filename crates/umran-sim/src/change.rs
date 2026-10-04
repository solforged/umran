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
    /// Pitch changes and the consonant loss that conditions them.
    Tone(crate::tone::Change),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Env {
    Any,
    WordEdge,
    Matcher(Matcher),
    /// Matches the nucleus of the immediately following syllable in this
    /// word, across consonants and morpheme boundaries, never another word.
    FollowingSyllableVowel(Matcher),
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
        match self {
            Self::FollowingSyllableVowel(_) => true,
            Self::Matcher(m) => m.needs_syllables(),
            _ => false,
        }
    }

    fn at(
        &self,
        form: &Form,
        i: Option<usize>,
        target: usize,
        syllables: &[crate::form::Syllable],
        stress: Option<usize>,
    ) -> bool {
        match self {
            Self::Any => true,
            Self::WordEdge => i.is_none(),
            Self::Matcher(m) => i.is_some_and(|i| m.at(form, i, syllables, stress)),
            Self::FollowingSyllableVowel(m) => syllables
                .iter()
                .position(|s| {
                    s.onset.contains(&target) || s.nucleus == target || s.coda.contains(&target)
                })
                .and_then(|at| syllables.get(at + 1))
                .is_some_and(|next| m.at(form, next.nucleus, syllables, stress)),
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
                || !self.left.at(form, i.checked_sub(1), i, &syllables, stress)
                || !self.right.at(
                    form,
                    (i + 1 < form.segs.len()).then_some(i + 1),
                    i,
                    &syllables,
                    stress,
                )
            {
                return None;
            }
            let out = match self.result {
                Rewrite::Length(long) => Some(Seg { long, ..seg }),
                Rewrite::Tone(change) => change.segment(form, i),
                _ => self.result.apply(seg.phone).map(|phone| Seg {
                    phone,
                    tone: seg.tone.filter(|_| CATALOG.get(phone).is_vowel()),
                    ..seg
                }),
            };
            let pitch = match self.result {
                Rewrite::Tone(change) => change
                    .neighbor(form, i)
                    .is_some_and(|(j, tone)| form.segs[j].tone != Some(tone)),
                _ => false,
            };
            (out != Some(seg) || pitch).then_some((i, out))
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
        match self.outcome(form, rule) {
            Some(outcome) => Cow::Owned(Self::from_outcome(form, &outcome, 0).0),
            None => Cow::Borrowed(form),
        }
    }

    /// Applies the same protected rewrite while tracking a grammatical edge.
    /// The edge is the number of input segments before the marker boundary.
    pub fn apply_with_edge(&self, form: &Form, rule: StressRule, edge: usize) -> (Form, usize) {
        match self.outcome(form, rule) {
            Some(outcome) => Self::from_outcome(form, &outcome, edge),
            None => (form.clone(), edge.min(form.segs.len())),
        }
    }

    /// One actual outcome per input segment, including neighbor lengthening
    /// and last-vowel protection. A validated no-hit input needs no allocation.
    pub(crate) fn outcome(&self, form: &Form, rule: StressRule) -> Option<Vec<Option<Seg>>> {
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
            return None;
        }
        let mut outcome: Vec<Option<Seg>> = form.segs.iter().copied().map(Some).collect();
        for (i, out) in first.into_iter().chain(hits) {
            outcome[i] = out;
            let neighbor = match self.result {
                Rewrite::GeminateNext => {
                    (i + 1 < form.segs.len() && !form.is_vowel(i + 1)).then_some(i + 1)
                }
                Rewrite::Compensate => i.checked_sub(1).filter(|&j| form.is_vowel(j)),
                _ => None,
            };
            // Side effects never resurrect a deletion. A later matching
            // neighbor is still deleted by its own input-based rewrite.
            if let Some(j) = neighbor
                && let Some(seg) = &mut outcome[j]
            {
                seg.long = true;
            }
            if let Rewrite::Tone(change) = self.result
                && let Some((j, tone)) = change.neighbor(form, i)
                && let Some(seg) = &mut outcome[j]
            {
                seg.tone = Some(tone);
            }
        }
        let vowel_survives = outcome
            .iter()
            .any(|o| o.is_some_and(|s| CATALOG.get(s.phone).is_vowel()));
        if !vowel_survives && let Some(i) = (0..form.segs.len()).rev().find(|&i| form.is_vowel(i)) {
            outcome[i] = Some(form.segs[i]);
        }
        Some(outcome)
    }

    pub(crate) fn from_outcome(form: &Form, outcome: &[Option<Seg>], edge: usize) -> (Form, usize) {
        let stress = form.stress_after(outcome);
        let mut segs = Vec::with_capacity(form.segs.len());
        let mut new_index = if form.boundaries.is_empty() {
            Vec::new()
        } else {
            Vec::with_capacity(form.segs.len() + 1)
        };
        let mut mapped_edge = 0;
        for (i, out) in outcome.iter().enumerate() {
            if !form.boundaries.is_empty() {
                new_index.push(segs.len());
            }
            if let Some(seg) = out {
                segs.push(*seg);
                mapped_edge += usize::from(i < edge);
            }
        }
        if !form.boundaries.is_empty() {
            new_index.push(segs.len());
        }
        let mut boundaries: Vec<usize> = form
            .boundaries
            .iter()
            .map(|&b| new_index[b])
            .filter(|&b| b > 0 && b < segs.len())
            .collect();
        boundaries.dedup();
        (
            Form {
                segs,
                boundaries,
                stress,
            },
            mapped_edge,
        )
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
    fn following_syllable_vowel_skips_consonants_but_not_syllables() {
        let rule = SoundChange {
            id: "o>ø/next-front-vowel".into(),
            target: Matcher::Phone(id("o")),
            result: Rewrite::Phone(id("ø")),
            left: Env::Any,
            right: Env::FollowingSyllableVowel(Matcher::Vowel {
                height: None,
                backness: Some(Backness::Front),
                rounded: None,
            }),
        };
        assert_eq!(
            rule.apply(&form("folti"), StressRule::Initial).ipa(),
            "følti"
        );
        assert_eq!(
            rule.apply(&form("foltu"), StressRule::Initial).ipa(),
            "foltu"
        );
        assert_eq!(
            rule.apply(&form("foltumi"), StressRule::Initial).ipa(),
            "foltumi"
        );
        assert_eq!(rule.apply(&form("folt"), StressRule::Initial).ipa(), "folt");
        assert_eq!(
            rule.apply(&form("folty"), StressRule::Initial).ipa(),
            "følty"
        );
    }

    #[test]
    fn following_nucleus_matches_derived_stress_and_length() {
        let rule = SoundChange {
            id: "o>ø/next-stressed-long-i".into(),
            target: Matcher::Phone(id("o")),
            result: Rewrite::Phone(id("ø")),
            left: Env::Any,
            right: Env::FollowingSyllableVowel(Matcher::Stressed {
                target: Box::new(Matcher::Length {
                    target: Box::new(Matcher::Phone(id("i"))),
                    long: true,
                }),
                stressed: true,
            }),
        };
        let mut word = form("foltiː");
        word.stress = Some(1);
        assert_eq!(rule.apply(&word, StressRule::Initial).ipa(), "foltiː");
        assert_eq!(rule.apply(&word, StressRule::Final).ipa(), "føltiː");
        assert_eq!(rule.apply(&word, StressRule::Free).ipa(), "føltiː");
        assert_eq!(rule.apply(&form("folti"), StressRule::Final).ipa(), "folti");
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

    #[test]
    fn tracked_edges_use_protected_survivors_and_remap_lexical_stress() {
        let loss = SoundChange {
            id: "vowel-loss".into(),
            target: Matcher::AnyVowel,
            result: Rewrite::Delete,
            left: Env::Any,
            right: Env::Any,
        };
        let mut word = form("katia");
        word.boundaries = vec![2, 4];
        word.stress = Some(1);
        let (out, edge) = loss.apply_with_edge(&word, StressRule::Free, 4);
        assert_eq!(out.ipa(), "kta");
        assert_eq!(edge, 2);
        assert_eq!(out.boundaries, vec![1, 2]);
        assert_eq!(out.stress, Some(0));

        // The restored last vowel still lies before an edge at the word end.
        let (out, edge) = loss.apply_with_edge(&form("ta"), StressRule::Initial, 2);
        assert_eq!(out.ipa(), "ta");
        assert_eq!(edge, 2);

        let loss = SoundChange {
            target: Matcher::Phone(id("a")),
            ..loss
        };
        let mut word = form("katima");
        word.stress = Some(1);
        let (out, edge) = loss.apply_with_edge(&word, StressRule::Free, 4);
        assert_eq!(out.ipa(), "ktim");
        assert_eq!(edge, 3);
        assert_eq!(out.stress, Some(0));
    }

    #[test]
    fn tracked_edges_include_gemination_and_compensation() {
        let geminate = SoundChange {
            id: "cluster-gemination".into(),
            target: Matcher::Phone(id("k")),
            result: Rewrite::GeminateNext,
            left: Env::Any,
            right: Env::Matcher(Matcher::Phone(id("t"))),
        };
        let mut word = form("aktia");
        word.boundaries = vec![2];
        word.stress = Some(1);
        let (out, edge) = geminate.apply_with_edge(&word, StressRule::Free, 2);
        assert_eq!(out.ipa(), "atːia");
        assert_eq!(edge, 1);
        assert_eq!(out.boundaries, vec![1]);
        assert_eq!(out.stress, Some(1));

        let compensation = SoundChange {
            id: "compensatory-lengthening".into(),
            target: Matcher::Phone(id("n")),
            result: Rewrite::Compensate,
            left: Env::Matcher(Matcher::AnyVowel),
            right: Env::Matcher(Matcher::AnyConsonant),
        };
        let mut word = form("anta");
        word.boundaries = vec![2];
        let (out, edge) = compensation.apply_with_edge(&word, StressRule::Initial, 2);
        assert_eq!(out.ipa(), "aːta");
        assert_eq!(edge, 1);
        assert_eq!(out.boundaries, vec![1]);

        // A lengthened neighbor that also matches is deleted, not restored.
        let all = SoundChange {
            target: Matcher::AnyConsonant,
            right: Env::Any,
            ..geminate
        };
        let (out, edge) = all.apply_with_edge(&form("aktta"), StressRule::Initial, 3);
        assert_eq!(out.ipa(), "aa");
        assert_eq!(edge, 1);
    }
}

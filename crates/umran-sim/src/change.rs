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
    /// A rule target: alternate eligible unstressed medial nuclei, starting
    /// nearest the input stress on each side. Eligibility includes both
    /// environments and the consonant run that removing this nucleus joins.
    RhythmicVowel {
        max_cluster: usize,
    },
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
    /// The target stays and a sound appears beside it, after it or before
    /// (epenthesis).
    Insert {
        phone: PhonemeId,
        after: bool,
    },
    /// The target and the segment after it trade places (metathesis).
    SwapNext,
    /// The target and the segment after it become this one sound. It
    /// keeps the target's length.
    Fuse(PhonemeId),
    /// The target becomes these two sounds, both short.
    Break(PhonemeId, PhonemeId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Env {
    Any,
    WordEdge,
    Matcher(Matcher),
    /// Matches the nucleus of the immediately following syllable in this
    /// word, across consonants and morpheme boundaries, never another word.
    FollowingSyllableVowel(Matcher),
    /// Holds wherever the inner environment does not, the word's edge
    /// included: "not after s" holds at the start of a word.
    Not(Box<Env>),
    /// Several positions read outward from the target: the first beside
    /// it, the next one further off. Every position but the last must hold
    /// a segment, so only the last can be the word's edge.
    Run(Vec<Env>),
}

/// What a rule makes of one word: an outcome for each input segment, the
/// sounds that appear beside them, and where lexical stress lands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Outcome {
    /// What each input segment becomes. `None` is a loss.
    pub segs: Vec<Option<Seg>>,
    /// Sounds that appear, in order: the input segment each stands beside,
    /// whether it follows that segment, and the sound.
    pub added: Vec<(usize, bool, Seg)>,
    stress: Option<usize>,
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
            (Matcher::MedialVowel | Matcher::RhythmicVowel { .. }, _) => seg.is_vowel(),
            _ => false,
        }
    }
    fn needs_syllables(&self) -> bool {
        match self {
            Self::Stressed { .. } | Self::OpenSyllable(_) | Self::RhythmicVowel { .. } => true,
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
            Self::RhythmicVowel { max_cluster } => {
                let Some(n) = syllables.iter().position(|s| s.nucleus == i) else {
                    return false;
                };
                n > 0
                    && n + 1 < syllables.len()
                    && stress.is_some_and(|s| s != n)
                    && syllables[n + 1].nucleus - syllables[n - 1].nucleus - 2 <= *max_cluster
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
            Self::Not(env) => env.needs_syllables(),
            Self::Run(envs) => envs.iter().any(Self::needs_syllables),
            _ => false,
        }
    }

    /// Whether this holds at `i`, the position beside the target on its
    /// `right` or its left, or past the word's edge where `i` is `None`.
    fn at(
        &self,
        form: &Form,
        i: Option<usize>,
        target: usize,
        right: bool,
        syllables: &[crate::form::Syllable],
        stress: Option<usize>,
    ) -> bool {
        match self {
            Self::Not(env) => !env.at(form, i, target, right, syllables, stress),
            Self::Run(envs) => {
                let mut next = i;
                envs.iter().enumerate().all(|(k, env)| {
                    let here = next;
                    if here.is_none() && k + 1 < envs.len() {
                        return false;
                    }
                    next = here.and_then(|j| {
                        if right {
                            (j + 1 < form.segs.len()).then_some(j + 1)
                        } else {
                            j.checked_sub(1)
                        }
                    });
                    env.at(form, here, target, right, syllables, stress)
                })
            }
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
            (Rewrite::Phone(p) | Rewrite::Fuse(p) | Rewrite::Break(p, _), _) => Some(*p),
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
    /// Matched feature rewrites with no catalog realization. Normal application
    /// remains a no-op for these segments; previews disclose that fact.
    pub fn unrealisable(&self, form: &Form, stress: StressRule) -> usize {
        if !matches!(
            self.result,
            Rewrite::Consonant { .. } | Rewrite::Vowel { .. }
        ) {
            return 0;
        }
        let syllables = form.syllables();
        let accent = form.stressed_syllable(stress);
        (0..form.segs.len())
            .filter(|&i| {
                if !self.target.at(form, i, &syllables, accent)
                    || !self
                        .left
                        .at(form, i.checked_sub(1), i, false, &syllables, accent)
                    || !self.right.at(
                        form,
                        (i + 1 < form.segs.len()).then_some(i + 1),
                        i,
                        true,
                        &syllables,
                        accent,
                    )
                {
                    return false;
                }
                let id = form.segs[i].phone;
                if self.result.apply(id) != Some(id) {
                    return false;
                }
                match (&self.result, CATALOG.get(id)) {
                    (
                        Rewrite::Consonant {
                            place,
                            manner,
                            voiced,
                            secondary,
                        },
                        Segment::Consonant(c),
                    ) => {
                        place.is_some_and(|p| p != c.place)
                            || manner.is_some_and(|m| m != c.manner)
                            || voiced.is_some_and(|v| v != c.voiced)
                            || secondary.is_some_and(|s| s != c.secondary)
                    }
                    (
                        Rewrite::Vowel {
                            height,
                            backness,
                            rounded,
                        },
                        Segment::Vowel(v),
                    ) => {
                        height.is_some_and(|h| h != v.height)
                            || backness.is_some_and(|b| b != v.backness)
                            || rounded.is_some_and(|r| r != v.rounded)
                    }
                    _ => true,
                }
            })
            .count()
    }
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
        let rhythmic = matches!(self.target, Matcher::RhythmicVowel { .. });
        // A swap or a fusion takes the segment after its target too.
        let pair = matches!(self.result, Rewrite::SwapNext | Rewrite::Fuse(_));
        let accent = stress.map(|s| syllables[s].nucleus);
        let eligible = move |i| {
            (!pair || i + 1 < form.segs.len())
                && self.target.at(form, i, &syllables, stress)
                && self
                    .left
                    .at(form, i.checked_sub(1), i, false, &syllables, stress)
                && self.right.at(
                    form,
                    (i + 1 < form.segs.len()).then_some(i + 1),
                    i,
                    true,
                    &syllables,
                    stress,
                )
        };
        // The output still walks left-to-right. Counting the eligible
        // pretonic nuclei first lets its rhythm run outward from the accent
        // without allocating a second list or changing input-based matching.
        let mut before = if rhythmic {
            (0..accent.unwrap_or(0)).filter(|&i| eligible(i)).count()
        } else {
            0
        };
        let mut after = 0;
        // The first segment no earlier hit has taken: of two overlapping
        // pairs, the one further left wins.
        let mut free = 0;
        (0..form.segs.len()).filter_map(move |i| {
            if i < free || !eligible(i) {
                return None;
            }
            if rhythmic {
                let ordinal = if accent.is_some_and(|s| i < s) {
                    let remaining = before;
                    before -= 1;
                    remaining
                } else {
                    after += 1;
                    after
                };
                if ordinal % 2 == 0 {
                    return None;
                }
            }
            let seg = form.segs[i];
            let out = match self.result {
                Rewrite::Length(long) => Some(Seg { long, ..seg }),
                Rewrite::Tone(change) => change.segment(form, i),
                Rewrite::SwapNext => Some(form.segs[i + 1]),
                Rewrite::Break(first, _) => Some(Seg {
                    phone: first,
                    long: false,
                    tone: seg.tone.filter(|_| CATALOG.get(first).is_vowel()),
                }),
                // A vowel fused from a consonant and a vowel keeps the
                // vowel's pitch.
                Rewrite::Fuse(phone) => Some(Seg {
                    phone,
                    tone: seg
                        .tone
                        .or(form.segs[i + 1].tone)
                        .filter(|_| CATALOG.get(phone).is_vowel()),
                    ..seg
                }),
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
            // These change the word even where the target itself stands.
            let reshapes = matches!(
                self.result,
                Rewrite::Insert { .. } | Rewrite::Fuse(_) | Rewrite::Break(..)
            );
            let hit = out != Some(seg) || pitch || reshapes;
            if hit && pair {
                free = i + 2;
            }
            hit.then_some((i, out))
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
    pub(crate) fn outcome(&self, form: &Form, rule: StressRule) -> Option<Outcome> {
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
        let mut added = Vec::new();
        // An added vowel takes the pitch of the vowel it stands beside.
        let beside = |i: usize, phone: PhonemeId| Seg {
            phone,
            long: false,
            tone: form.segs[i].tone.filter(|_| CATALOG.get(phone).is_vowel()),
        };
        for (i, out) in first.into_iter().chain(hits) {
            outcome[i] = out;
            match self.result {
                Rewrite::SwapNext => outcome[i + 1] = Some(form.segs[i]),
                Rewrite::Fuse(_) => outcome[i + 1] = None,
                Rewrite::Insert { phone, after } => added.push((i, after, beside(i, phone))),
                Rewrite::Break(_, second) => added.push((i, true, beside(i, second))),
                _ => {}
            }
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
        let vowel = |s: &Seg| CATALOG.get(s.phone).is_vowel();
        let vowel_survives = outcome.iter().any(|o| o.as_ref().is_some_and(vowel))
            || added.iter().any(|(.., s)| vowel(s));
        if !vowel_survives && let Some(i) = (0..form.segs.len()).rev().find(|&i| form.is_vowel(i)) {
            outcome[i] = Some(form.segs[i]);
        }
        // Trading places moves no vowel past another, so the stressed
        // vowel is still the same one counted from the start.
        let stress = if matches!(self.result, Rewrite::SwapNext) {
            form.stress.filter(|&s| s < form.vowel_count())
        } else {
            form.stress_after(&outcome, &added)
        };
        Some(Outcome {
            segs: outcome,
            added,
            stress,
        })
    }

    pub(crate) fn from_outcome(form: &Form, outcome: &Outcome, edge: usize) -> (Form, usize) {
        let stress = outcome.stress;
        let mut added = outcome.added.iter().peekable();
        let mut segs = Vec::with_capacity(form.segs.len() + outcome.added.len());
        let mut new_index = if form.boundaries.is_empty() {
            Vec::new()
        } else {
            Vec::with_capacity(form.segs.len() + 1)
        };
        let mut mapped_edge = 0;
        for (i, out) in outcome.segs.iter().enumerate() {
            if !form.boundaries.is_empty() {
                new_index.push(segs.len());
            }
            // An added sound stays on its own segment's side of a morpheme
            // boundary and of the grammatical edge.
            let beside = added.next_if(|&&(at, ..)| at == i).copied();
            let before = beside.filter(|&(_, after, _)| !after).map(|(.., seg)| seg);
            let after = beside.filter(|&(_, after, _)| after).map(|(.., seg)| seg);
            for seg in before.into_iter().chain(*out).chain(after) {
                segs.push(seg);
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
        let mut form = Form {
            segs,
            boundaries,
            stress,
        };
        mapped_edge = form.fuse_consonants(mapped_edge);
        (form, mapped_edge)
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
    fn law_outputs_fuse_consonants_and_remap_boundaries_and_edges() {
        let rule = SoundChange {
            id: "p>f".into(),
            target: Matcher::Phone(id("p")),
            result: Rewrite::Phone(id("f")),
            left: Env::Any,
            right: Env::Any,
        };
        let mut input = form("ipffa");
        input.boundaries = vec![1, 2, 3, 4];
        input.stress = Some(1);
        let (out, edge) = rule.apply_with_edge(&input, StressRule::Free, 3);
        assert_eq!(out.ipa(), "ifːa");
        assert_eq!(out.boundaries, vec![1, 2]);
        assert_eq!(out.stress, Some(1));
        assert_eq!(edge, 2);
        assert_eq!(out.vowel_count(), input.vowel_count());
        assert_eq!(rule.apply(&input, StressRule::Free), out);
        let law = crate::Law {
            id: ("fusion-test").into(),
            label: ("fusion test").into(),
            source: None,
            commonness: 1.0,
            rules: vec![rule],
            stress: None,
        };
        let after = law.apply(&input, crate::MinimalWord::Syllable, StressRule::Free);
        assert_eq!(after, out);
        assert!(law.changes(&input, &after, StressRule::Free));
        assert!(!law.changes(
            &after,
            &law.apply(&after, crate::MinimalWord::Syllable, StressRule::Free),
            StressRule::Free
        ));
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
    fn added_and_traded_sounds_keep_stress_boundaries_and_edges() {
        let rule = |target: Matcher, result: Rewrite, left: Env, right: Env| SoundChange {
            id: "test".into(),
            target,
            result,
            left,
            right,
        };
        // A vowel added before the stressed one moves the count along.
        let prothesis = rule(
            Matcher::Phone(id("s")),
            Rewrite::Insert {
                phone: id("e"),
                after: false,
            },
            Env::WordEdge,
            Env::Matcher(Matcher::AnyConsonant),
        );
        let mut word = form("skola");
        word.stress = Some(0);
        word.boundaries = vec![3];
        let (out, edge) = prothesis.apply_with_edge(&word, StressRule::Free, 3);
        assert_eq!(out.ipa(), "eskola");
        assert_eq!((out.stress, edge), (Some(1), 4));
        assert_eq!(out.boundaries, vec![4]);
        // A sound added after its segment stays on that segment's side of
        // a boundary and of the edge; one added before stays on its own.
        let after_t = rule(
            Matcher::Phone(id("t")),
            Rewrite::Insert {
                phone: id("e"),
                after: true,
            },
            Env::Any,
            Env::Matcher(Matcher::AnyConsonant),
        );
        let mut word = form("patka");
        word.boundaries = vec![3];
        let (out, edge) = after_t.apply_with_edge(&word, StressRule::Initial, 3);
        assert_eq!(out.ipa(), "pateka");
        assert_eq!((out.boundaries.clone(), edge), (vec![4], 4));
        let before_k = rule(
            Matcher::Phone(id("k")),
            Rewrite::Insert {
                phone: id("e"),
                after: false,
            },
            Env::Matcher(Matcher::AnyConsonant),
            Env::Any,
        );
        let (out, edge) = before_k.apply_with_edge(&word, StressRule::Initial, 3);
        assert_eq!(out.ipa(), "pateka");
        assert_eq!((out.boundaries.clone(), edge), (vec![3], 3));
        // Trading places keeps the stress on the same vowel.
        let swap = rule(
            Matcher::Phone(id("r")),
            Rewrite::SwapNext,
            Env::Any,
            Env::Any,
        );
        let mut word = form("tra");
        word.stress = Some(0);
        assert_eq!(swap.apply(&word, StressRule::Free).ipa(), "tar");
        assert_eq!(swap.apply(&word, StressRule::Free).stress, Some(0));
        let mut word = form("katra");
        word.stress = Some(1);
        let out = swap.apply(&word, StressRule::Free);
        assert_eq!((out.ipa(), out.stress), ("katar".into(), Some(1)));
        // A fused pair is one sound: the stress stays with it, and a word
        // never loses its last vowel to a fusion.
        let fuse = rule(
            Matcher::Phone(id("a")),
            Rewrite::Fuse(id("e")),
            Env::Any,
            Env::Matcher(Matcher::Phone(id("i"))),
        );
        let mut word = form("kaita");
        word.stress = Some(0);
        let out = fuse.apply(&word, StressRule::Free);
        assert_eq!((out.ipa(), out.stress), ("keta".into(), Some(0)));
        let mut word = form("takai");
        word.stress = Some(2);
        let out = fuse.apply(&word, StressRule::Free);
        assert_eq!((out.ipa(), out.stress), ("take".into(), Some(1)));
        let to_glide = rule(
            Matcher::Phone(id("i")),
            Rewrite::Fuse(id("j")),
            Env::Any,
            Env::Matcher(Matcher::Phone(id("a"))),
        );
        assert_eq!(
            to_glide.apply(&form("tia"), StressRule::Initial).ipa(),
            "tja"
        );
        // A broken vowel hands its stress to the vowel that remains.
        let breaking = rule(
            Matcher::Phone(id("e")),
            Rewrite::Break(id("j"), id("e")),
            Env::Any,
            Env::Any,
        );
        let mut word = form("pate");
        word.stress = Some(1);
        let out = breaking.apply(&word, StressRule::Free);
        assert_eq!((out.ipa(), out.stress), ("patje".into(), Some(1)));
        // The pair needs its second sound, and a run stops at the edge.
        assert_eq!(swap.apply(&form("tar"), StressRule::Initial).ipa(), "tar");
        let run = rule(
            Matcher::Phone(id("t")),
            Rewrite::Phone(id("d")),
            Env::Run(vec![Env::Matcher(Matcher::AnyVowel), Env::WordEdge]),
            Env::Not(Box::new(Env::Matcher(Matcher::Phone(id("i"))))),
        );
        assert_eq!(
            run.apply(&form("atati"), StressRule::Initial).ipa(),
            "adati"
        );
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

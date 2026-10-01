use crate::form::{Form, Seg};
use crate::phoneme::{Backness, CATALOG, Height, Manner, PhonemeId, Place, Segment};
use serde::{Deserialize, Serialize};

/// A natural class of segments; unset features match anything.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Matcher {
    Phone(PhonemeId),
    Consonant {
        place: Option<Place>,
        manner: Option<Manner>,
        voiced: Option<bool>,
    },
    Vowel {
        height: Option<Height>,
        backness: Option<Backness>,
        rounded: Option<bool>,
    },
    AnyConsonant,
    AnyVowel,
}

/// What a matched segment becomes. A feature rewrite with no catalog
/// segment for the resulting bundle leaves the segment unchanged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Rewrite {
    Phone(PhonemeId),
    Consonant {
        place: Option<Place>,
        manner: Option<Manner>,
        voiced: Option<bool>,
    },
    Vowel {
        height: Option<Height>,
        backness: Option<Backness>,
        rounded: Option<bool>,
    },
    Delete,
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
                },
                Segment::Consonant(c),
            ) => {
                place.is_none_or(|p| p == c.place)
                    && manner.is_none_or(|m| m == c.manner)
                    && voiced.is_none_or(|v| v == c.voiced)
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
            _ => false,
        }
    }
}

impl Env {
    fn left_ok(&self, form: &Form, i: usize) -> bool {
        match self {
            Env::Any => true,
            Env::WordEdge => i == 0,
            Env::Matcher(m) => i > 0 && m.matches(form.segs[i - 1].phone),
        }
    }

    fn right_ok(&self, form: &Form, i: usize) -> bool {
        match self {
            Env::Any => true,
            Env::WordEdge => i + 1 == form.segs.len(),
            Env::Matcher(m) => i + 1 < form.segs.len() && m.matches(form.segs[i + 1].phone),
        }
    }
}

impl Rewrite {
    /// `None` deletes the segment.
    fn apply(&self, id: PhonemeId) -> Option<PhonemeId> {
        match (self, CATALOG.get(id)) {
            (Rewrite::Delete, _) => None,
            (Rewrite::Phone(p), _) => Some(*p),
            (
                Rewrite::Consonant {
                    place,
                    manner,
                    voiced,
                },
                Segment::Consonant(c),
            ) => {
                let (place, manner, voiced) = (
                    place.unwrap_or(c.place),
                    manner.unwrap_or(c.manner),
                    voiced.unwrap_or(c.voiced),
                );
                Some(
                    CATALOG
                        .consonants()
                        .find(|(_, k)| k.place == place && k.manner == manner && k.voiced == voiced)
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
    /// Applies the change simultaneously: every environment is matched
    /// against the input, so a rewrite never feeds or bleeds its own
    /// application elsewhere in the same word. A word never loses its last
    /// vowel; morpheme boundaries follow the segments that survive.
    pub fn apply(&self, form: &Form) -> Form {
        let mut outcome: Vec<Option<PhonemeId>> = (0..form.segs.len())
            .map(|i| {
                let id = form.segs[i].phone;
                let hit = self.target.matches(id)
                    && self.left.left_ok(form, i)
                    && self.right.right_ok(form, i);
                if hit { self.result.apply(id) } else { Some(id) }
            })
            .collect();

        let vowel_survives = outcome
            .iter()
            .any(|o| o.is_some_and(|id| CATALOG.get(id).is_vowel()));
        let last_vowel = (0..form.segs.len()).rev().find(|&i| form.is_vowel(i));
        if let (false, Some(i)) = (vowel_survives, last_vowel) {
            outcome[i] = Some(form.segs[i].phone);
        }

        let mut segs = Vec::with_capacity(form.segs.len());
        let mut new_index = Vec::with_capacity(form.segs.len() + 1);
        for (seg, out) in form.segs.iter().zip(&outcome) {
            new_index.push(segs.len());
            if let Some(phone) = *out {
                let long = seg.long && CATALOG.get(phone).is_vowel();
                segs.push(Seg { phone, long });
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
        Form { segs, boundaries }
    }
}

pub fn apply_all(form: &Form, rules: &[SoundChange]) -> Form {
    rules.iter().fold(form.clone(), |f, rule| rule.apply(&f))
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
            },
            result: Rewrite::Consonant {
                place: None,
                manner: Some(Manner::Fricative),
                voiced: None,
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
        assert_eq!(t_to_s_before_i().apply(&form("tita")).ipa(), "sita");
    }

    #[test]
    fn never_deletes_the_last_vowel() {
        assert_eq!(apocope().apply(&form("kata")).ipa(), "kat");
        assert_eq!(apocope().apply(&form("ta")).ipa(), "ta");
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
            },
            left: Env::Any,
            right: Env::Any,
        };
        assert_eq!(rule.apply(&form("ta")).ipa(), "ta");
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
        assert_eq!(rule.apply(&form("keaa")).ipa(), "keea");
    }

    #[test]
    fn length_and_boundaries_follow_surviving_segments() {
        let mut word = form("kaːtiti");
        word.boundaries = vec![2, 4];
        let out = apply_all(&word, &[t_to_s_before_i(), apocope()]);
        assert_eq!(out.ipa(), "kaːsis");
        assert!(out.segs[1].long);
        assert_eq!(out.boundaries, vec![2, 4]);

        let mut word = form("katia");
        word.boundaries = vec![4];
        let out = apocope().apply(&word);
        assert_eq!(out.ipa(), "kati");
        assert!(
            out.boundaries.is_empty(),
            "a boundary at the new word end is dropped"
        );
    }
}

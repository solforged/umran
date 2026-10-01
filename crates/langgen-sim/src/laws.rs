use crate::change::{Env, Matcher, Rewrite, SoundChange, apply_all};
use crate::form::Form;
use crate::inventory::score;
use crate::phoneme::{Backness, CATALOG, Height, Manner, PhonemeId, Place};
use crate::profile::InventoryPrior;

/// A named sound law: one or more rules applied in order, each regularly
/// across every living word.
#[derive(Clone, Debug, PartialEq)]
pub struct Law {
    pub id: &'static str,
    pub label: &'static str,
    pub rules: Vec<SoundChange>,
    /// Rough typological commonness, a hand estimate rather than a measured
    /// frequency. Lenition and palatalization are common; wholesale
    /// spirantization is rare.
    pub commonness: f32,
}

/// What a law would do to a lexicon if chosen now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Assessment {
    pub words: usize,
    /// Mean change in preference score per altered segment; positive when
    /// the law moves sounds toward what the culture prefers. Deletions count
    /// as neutral; forbidden segments score very low.
    pub pull: f32,
}

impl Law {
    pub fn apply(&self, form: &Form) -> Form {
        apply_all(form, &self.rules)
    }

    /// `None` when the law would change nothing.
    pub fn assess<'a>(
        &self,
        forms: impl Iterator<Item = &'a Form>,
        prior: &InventoryPrior,
    ) -> Option<Assessment> {
        let (mut words, mut segments, mut total) = (0, 0, 0.0);
        for form in forms {
            // Judge by the real result, so matches blocked by last-vowel
            // protection do not make a law look applicable.
            if self.apply(form) == *form {
                continue;
            }
            words += 1;
            let mut current = form.clone();
            for rule in &self.rules {
                for (i, out) in rule.hits(&current) {
                    segments += 1;
                    if let Some(new) = out {
                        let old = current.segs[i].phone;
                        total += preference(prior, new) - preference(prior, old);
                    }
                }
                current = rule.apply(&current);
            }
        }
        (words > 0).then(|| Assessment {
            words,
            pull: total / segments as f32,
        })
    }
}

/// Preference score for a segment, treating anything the culture forbids
/// as strongly dispreferred: a sound change can still create it, but rarely.
fn preference(prior: &InventoryPrior, id: PhonemeId) -> f32 {
    let seg = CATALOG.get(id);
    if prior.forbidden.iter().any(|f| f == seg.ipa()) {
        FORBIDDEN_SCORE
    } else {
        score(prior, seg)
    }
}

const FORBIDDEN_SCORE: f32 = -4.0;

fn cons(place: Option<Place>, manner: Option<Manner>, voiced: Option<bool>) -> Matcher {
    Matcher::Consonant {
        place,
        manner,
        voiced,
    }
}

fn vowel(height: Option<Height>, backness: Option<Backness>, rounded: Option<bool>) -> Matcher {
    Matcher::Vowel {
        height,
        backness,
        rounded,
    }
}

fn into_cons(place: Option<Place>, manner: Option<Manner>, voiced: Option<bool>) -> Rewrite {
    Rewrite::Consonant {
        place,
        manner,
        voiced,
    }
}

fn rule(target: Matcher, result: Rewrite, left: Env, right: Env) -> SoundChange {
    SoundChange {
        id: String::new(),
        target,
        result,
        left,
        right,
    }
}

fn law(id: &'static str, label: &'static str, commonness: f32, rules: Vec<SoundChange>) -> Law {
    let rules = rules
        .into_iter()
        .enumerate()
        .map(|(i, r)| SoundChange {
            id: format!("{id}.{i}"),
            ..r
        })
        .collect();
    Law {
        id,
        label,
        rules,
        commonness,
    }
}

fn v() -> Env {
    Env::Matcher(Matcher::AnyVowel)
}

fn c() -> Env {
    Env::Matcher(Matcher::AnyConsonant)
}

fn at(m: Matcher) -> Env {
    Env::Matcher(m)
}

const ANY: Env = Env::Any;
const EDGE: Env = Env::WordEdge;

/// The sound laws a variety can undergo. Each is applied at most once.
pub fn catalog() -> Vec<Law> {
    use Backness::*;
    use Height::*;
    use Manner::*;
    use Place::*;
    let close_front = || vowel(Some(Close), Some(Front), Some(false));
    let mid_front = || vowel(Some(CloseMid), Some(Front), Some(false));
    vec![
        law(
            "intervocalic-voicing",
            "Voiceless stops voice between vowels",
            1.0,
            vec![rule(
                cons(None, Some(Stop), Some(false)),
                into_cons(None, None, Some(true)),
                v(),
                v(),
            )],
        ),
        law(
            "intervocalic-spirantization",
            "Voiced stops become fricatives between vowels: b > v, d > ð, g > ɣ",
            0.8,
            vec![
                rule(
                    cons(Some(Bilabial), Some(Stop), Some(true)),
                    into_cons(Some(Labiodental), Some(Fricative), None),
                    v(),
                    v(),
                ),
                rule(
                    cons(Some(Alveolar), Some(Stop), Some(true)),
                    into_cons(Some(Dental), Some(Fricative), None),
                    v(),
                    v(),
                ),
                rule(
                    cons(Some(Velar), Some(Stop), Some(true)),
                    into_cons(None, Some(Fricative), None),
                    v(),
                    v(),
                ),
            ],
        ),
        law(
            "final-devoicing",
            "Voiced obstruents devoice at the end of a word",
            0.9,
            vec![
                rule(
                    cons(None, Some(Stop), Some(true)),
                    into_cons(None, None, Some(false)),
                    ANY,
                    EDGE,
                ),
                rule(
                    cons(None, Some(Fricative), Some(true)),
                    into_cons(None, None, Some(false)),
                    ANY,
                    EDGE,
                ),
                rule(
                    cons(None, Some(Affricate), Some(true)),
                    into_cons(None, None, Some(false)),
                    ANY,
                    EDGE,
                ),
            ],
        ),
        law(
            "velar-palatalization",
            "Velar stops become affricates before front vowels: k > tʃ, g > dʒ",
            1.0,
            vec![
                rule(
                    cons(Some(Velar), Some(Stop), None),
                    into_cons(Some(Postalveolar), Some(Affricate), None),
                    ANY,
                    at(close_front()),
                ),
                rule(
                    cons(Some(Velar), Some(Stop), None),
                    into_cons(Some(Postalveolar), Some(Affricate), None),
                    ANY,
                    at(mid_front()),
                ),
            ],
        ),
        law(
            "assibilation",
            "t becomes s before i",
            0.6,
            vec![rule(
                cons(Some(Alveolar), Some(Stop), Some(false)),
                into_cons(None, Some(Fricative), None),
                ANY,
                at(close_front()),
            )],
        ),
        law(
            "s-debuccalization",
            "s becomes h at the start of a word and between vowels",
            0.5,
            vec![
                rule(
                    cons(Some(Alveolar), Some(Fricative), Some(false)),
                    into_cons(Some(Glottal), None, None),
                    EDGE,
                    ANY,
                ),
                rule(
                    cons(Some(Alveolar), Some(Fricative), Some(false)),
                    into_cons(Some(Glottal), None, None),
                    v(),
                    v(),
                ),
            ],
        ),
        law(
            "h-loss",
            "h disappears",
            0.9,
            vec![rule(
                cons(Some(Glottal), Some(Fricative), None),
                Rewrite::Delete,
                ANY,
                ANY,
            )],
        ),
        law(
            "nasal-assimilation",
            "n takes the place of a following consonant: np > mp, nk > ŋk",
            0.9,
            vec![
                rule(
                    cons(Some(Alveolar), Some(Nasal), None),
                    into_cons(Some(Bilabial), None, None),
                    ANY,
                    at(cons(Some(Bilabial), None, None)),
                ),
                rule(
                    cons(Some(Alveolar), Some(Nasal), None),
                    into_cons(Some(Velar), None, None),
                    ANY,
                    at(cons(Some(Velar), None, None)),
                ),
            ],
        ),
        law(
            "apocope",
            "Final vowels after a consonant are lost",
            0.7,
            vec![rule(Matcher::AnyVowel, Rewrite::Delete, c(), EDGE)],
        ),
        law(
            "final-stop-loss",
            "Word-final stops are lost",
            0.5,
            vec![rule(
                cons(None, Some(Stop), None),
                Rewrite::Delete,
                ANY,
                EDGE,
            )],
        ),
        law(
            "final-raising",
            "Final mid vowels rise: e > i, o > u",
            0.6,
            vec![rule(
                vowel(Some(CloseMid), None, None),
                Rewrite::Vowel {
                    height: Some(Close),
                    backness: None,
                    rounded: None,
                },
                ANY,
                EDGE,
            )],
        ),
        law(
            "rhotacism",
            "s becomes r between vowels",
            0.4,
            vec![rule(
                cons(Some(Alveolar), Some(Fricative), None),
                into_cons(None, Some(Trill), Some(true)),
                v(),
                v(),
            )],
        ),
        law(
            "w-fortition",
            "w becomes v",
            0.6,
            vec![rule(
                cons(Some(Bilabial), Some(Approximant), None),
                into_cons(Some(Labiodental), Some(Fricative), Some(true)),
                ANY,
                ANY,
            )],
        ),
        law(
            "th-fronting",
            "θ and ð become f and v",
            0.4,
            vec![rule(
                cons(Some(Dental), Some(Fricative), None),
                into_cons(Some(Labiodental), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "th-stopping",
            "θ and ð become t and d",
            0.5,
            vec![rule(
                cons(Some(Dental), Some(Fricative), None),
                into_cons(Some(Alveolar), Some(Stop), None),
                ANY,
                ANY,
            )],
        ),
        law(
            "x-debuccalization",
            "x becomes h",
            0.7,
            vec![rule(
                cons(Some(Velar), Some(Fricative), Some(false)),
                into_cons(Some(Glottal), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "spirantization",
            "Voiceless stops become fricatives: p > f, t > θ, k > x",
            0.3,
            vec![
                rule(
                    cons(Some(Bilabial), Some(Stop), Some(false)),
                    into_cons(Some(Labiodental), Some(Fricative), None),
                    ANY,
                    ANY,
                ),
                rule(
                    cons(Some(Alveolar), Some(Stop), Some(false)),
                    into_cons(Some(Dental), Some(Fricative), None),
                    ANY,
                    ANY,
                ),
                rule(
                    cons(Some(Velar), Some(Stop), Some(false)),
                    into_cons(None, Some(Fricative), None),
                    ANY,
                    ANY,
                ),
            ],
        ),
        law(
            "stop-devoicing",
            "Voiced stops devoice everywhere",
            0.3,
            vec![rule(
                cons(None, Some(Stop), Some(true)),
                into_cons(None, None, Some(false)),
                ANY,
                ANY,
            )],
        ),
        law(
            "velar-nasal-fronting",
            "ŋ becomes n",
            0.4,
            vec![rule(
                cons(Some(Velar), Some(Nasal), None),
                into_cons(Some(Alveolar), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "rhotic-merger",
            "The tap merges with the trill",
            0.5,
            vec![rule(
                cons(Some(Alveolar), Some(Tap), None),
                into_cons(None, Some(Trill), None),
                ANY,
                ANY,
            )],
        ),
        law(
            "l-vocalization",
            "l becomes w before a consonant or at the end of a word",
            0.4,
            vec![
                rule(
                    cons(Some(Alveolar), Some(Lateral), None),
                    into_cons(Some(Bilabial), Some(Approximant), None),
                    ANY,
                    c(),
                ),
                rule(
                    cons(Some(Alveolar), Some(Lateral), None),
                    into_cons(Some(Bilabial), Some(Approximant), None),
                    ANY,
                    EDGE,
                ),
            ],
        ),
        law(
            "initial-glide-loss",
            "j and w are lost at the start of a word",
            0.4,
            vec![
                rule(
                    cons(Some(Palatal), Some(Approximant), None),
                    Rewrite::Delete,
                    EDGE,
                    ANY,
                ),
                rule(
                    cons(Some(Bilabial), Some(Approximant), None),
                    Rewrite::Delete,
                    EDGE,
                    ANY,
                ),
            ],
        ),
        law(
            "final-nasal-merger",
            "Final m becomes n",
            0.5,
            vec![rule(
                cons(Some(Bilabial), Some(Nasal), None),
                into_cons(Some(Alveolar), None, None),
                ANY,
                EDGE,
            )],
        ),
        law(
            "u-fronting",
            "u becomes y",
            0.2,
            vec![rule(
                vowel(Some(Close), Some(Back), Some(true)),
                Rewrite::Vowel {
                    height: None,
                    backness: Some(Front),
                    rounded: None,
                },
                ANY,
                ANY,
            )],
        ),
        law(
            "pre-nasal-raising",
            "e becomes i before a nasal",
            0.4,
            vec![rule(
                mid_front(),
                Rewrite::Vowel {
                    height: Some(Close),
                    backness: None,
                    rounded: None,
                },
                ANY,
                at(cons(None, Some(Nasal), None)),
            )],
        ),
        law(
            "flapping",
            "t and d become a tap between vowels",
            0.6,
            vec![rule(
                cons(Some(Alveolar), Some(Stop), None),
                into_cons(None, Some(Tap), Some(true)),
                v(),
                v(),
            )],
        ),
        law(
            "s-voicing",
            "s becomes z between vowels",
            0.5,
            vec![rule(
                cons(Some(Alveolar), Some(Fricative), Some(false)),
                into_cons(None, None, Some(true)),
                v(),
                v(),
            )],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::SoundProfile;
    use std::collections::HashSet;

    fn run(id: &str, ipa: &str) -> String {
        let law = catalog().into_iter().find(|l| l.id == id).unwrap();
        law.apply(&Form::from_ipa(ipa).unwrap()).ipa()
    }

    #[test]
    fn ids_are_unique() {
        let ids: HashSet<_> = catalog().iter().map(|l| l.id).collect();
        assert_eq!(ids.len(), catalog().len());
    }

    #[test]
    fn laws_do_what_their_labels_say() {
        assert_eq!(run("intervocalic-voicing", "pata"), "pada");
        assert_eq!(run("intervocalic-spirantization", "abadaga"), "avaðaɣa");
        assert_eq!(run("final-devoicing", "bad"), "bat");
        assert_eq!(run("velar-palatalization", "kika"), "tʃika");
        assert_eq!(run("s-debuccalization", "sasa"), "haha");
        assert_eq!(run("nasal-assimilation", "anpanka"), "ampaŋka");
        assert_eq!(run("apocope", "kata"), "kat");
        assert_eq!(run("final-raising", "kote"), "koti");
        assert_eq!(run("spirantization", "pitak"), "fiθax");
        assert_eq!(run("l-vocalization", "kaltal"), "kawtaw");
        assert_eq!(run("flapping", "pata"), "paɾa");
    }

    #[test]
    fn preferences_pull_toward_preferred_sounds() {
        // Kuo-toa dislikes fricatives, so spirantization pulls against it.
        let prior = SoundProfile::by_id("kuo-toa").unwrap().inventory;
        let law = catalog()
            .into_iter()
            .find(|l| l.id == "spirantization")
            .unwrap();
        let forms = [Form::from_ipa("pata").unwrap()];
        let a = law.assess(forms.iter(), &prior).unwrap();
        assert_eq!(a.words, 1);
        assert!(a.pull < 0.0);
        let untouched = [Form::from_ipa("ama").unwrap()];
        assert_eq!(law.assess(untouched.iter(), &prior), None);
    }
}

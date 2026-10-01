use crate::change::{Env, Matcher, Rewrite, SoundChange, apply_all};
use crate::form::Form;
use crate::inventory::preference;
use crate::phoneme::{Backness, Height, Manner, PhonemeId, Place, Secondary};
use crate::profile::InventoryPrior;
use std::collections::HashSet;

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
#[derive(Clone, Debug, PartialEq)]
pub struct Assessment {
    pub words: usize,
    /// Mean change in preference score per altered segment; positive when
    /// the law moves sounds toward what the culture prefers. Deletions count
    /// as neutral; forbidden segments score very low.
    pub pull: f32,
    /// Every segment the law would alter, and what it becomes.
    pub shifts: Vec<(PhonemeId, Option<PhonemeId>)>,
}

impl Assessment {
    /// Mean movement toward the sound set `target`, from -1 to 1: +1 when a
    /// segment outside it becomes one inside, -1 for the reverse.
    /// Deletions count as neutral.
    pub fn toward(&self, target: &HashSet<PhonemeId>) -> f32 {
        let total: f32 = self
            .shifts
            .iter()
            .map(|&(old, new)| match new {
                Some(new) => f32::from(target.contains(&new)) - f32::from(target.contains(&old)),
                None => 0.0,
            })
            .sum();
        total / self.shifts.len().max(1) as f32
    }
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
        let (mut words, mut total) = (0, 0.0);
        let mut shifts = Vec::new();
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
                    let old = current.segs[i].phone;
                    if let Some(new) = out {
                        total += preference(prior, new) - preference(prior, old);
                    }
                    shifts.push((old, out));
                }
                current = rule.apply(&current);
            }
        }
        (words > 0).then(|| Assessment {
            words,
            pull: total / shifts.len() as f32,
            shifts,
        })
    }
}

fn cons(place: Option<Place>, manner: Option<Manner>, voiced: Option<bool>) -> Matcher {
    Matcher::Consonant {
        place,
        manner,
        voiced,
        secondary: None,
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
        secondary: None,
    }
}

fn with_secondary(m: Matcher, s: Secondary) -> Matcher {
    match m {
        Matcher::Consonant {
            place,
            manner,
            voiced,
            ..
        } => Matcher::Consonant {
            place,
            manner,
            voiced,
            secondary: Some(s),
        },
        other => other,
    }
}

fn into_secondary(r: Rewrite, s: Secondary) -> Rewrite {
    match r {
        Rewrite::Consonant {
            place,
            manner,
            voiced,
            ..
        } => Rewrite::Consonant {
            place,
            manner,
            voiced,
            secondary: Some(s),
        },
        other => other,
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
        law(
            "deaspiration",
            "Aspirated stops lose their aspiration: pʰ > p",
            0.6,
            vec![rule(
                with_secondary(cons(None, None, None), Secondary::Aspirated),
                into_secondary(into_cons(None, None, None), Secondary::Plain),
                ANY,
                ANY,
            )],
        ),
        law(
            "aspirate-spirantization",
            "Aspirated stops become fricatives: pʰ > f, tʰ > θ, kʰ > x",
            0.3,
            vec![
                rule(
                    with_secondary(
                        cons(Some(Bilabial), Some(Stop), Some(false)),
                        Secondary::Aspirated,
                    ),
                    into_secondary(
                        into_cons(Some(Labiodental), Some(Fricative), None),
                        Secondary::Plain,
                    ),
                    ANY,
                    ANY,
                ),
                rule(
                    with_secondary(
                        cons(Some(Alveolar), Some(Stop), Some(false)),
                        Secondary::Aspirated,
                    ),
                    into_secondary(
                        into_cons(Some(Dental), Some(Fricative), None),
                        Secondary::Plain,
                    ),
                    ANY,
                    ANY,
                ),
                rule(
                    with_secondary(
                        cons(Some(Velar), Some(Stop), Some(false)),
                        Secondary::Aspirated,
                    ),
                    into_secondary(into_cons(None, Some(Fricative), None), Secondary::Plain),
                    ANY,
                    ANY,
                ),
            ],
        ),
        law(
            "breathy-loss",
            "Breathy-voiced stops become plain voiced: bʱ > b",
            0.6,
            vec![rule(
                with_secondary(cons(None, None, None), Secondary::Breathy),
                into_secondary(into_cons(None, None, None), Secondary::Plain),
                ANY,
                ANY,
            )],
        ),
        law(
            "labiovelar-to-labial",
            "Labiovelars become labials: kʷ > p, gʷ > b",
            0.3,
            vec![rule(
                with_secondary(cons(Some(Velar), Some(Stop), None), Secondary::Labialized),
                into_secondary(into_cons(Some(Bilabial), None, None), Secondary::Plain),
                ANY,
                ANY,
            )],
        ),
        law(
            "delabialization",
            "Labialized consonants lose their rounding: kʷ > k",
            0.5,
            vec![rule(
                with_secondary(cons(None, None, None), Secondary::Labialized),
                into_secondary(into_cons(None, None, None), Secondary::Plain),
                ANY,
                ANY,
            )],
        ),
        law(
            "retroflex-merger",
            "Retroflex consonants merge with alveolars: ʈ > t",
            0.4,
            vec![rule(
                cons(Some(Retroflex), None, None),
                into_cons(Some(Alveolar), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "pharyngeal-weakening",
            "Pharyngeals weaken to glottals: ħ > h, ʕ > ʔ",
            0.5,
            vec![
                rule(
                    cons(Some(Pharyngeal), None, Some(false)),
                    into_cons(Some(Glottal), None, None),
                    ANY,
                    ANY,
                ),
                rule(
                    cons(Some(Pharyngeal), None, Some(true)),
                    into_cons(Some(Glottal), Some(Stop), Some(false)),
                    ANY,
                    ANY,
                ),
            ],
        ),
        law(
            "uvular-fronting",
            "Uvulars move forward to velars: q > k, χ > x",
            0.5,
            vec![rule(
                cons(Some(Uvular), None, None),
                into_cons(Some(Velar), None, None),
                ANY,
                ANY,
            )],
        ),
        law(
            "lateral-affricate-loss",
            "The lateral affricate simplifies: tɬ > t",
            0.3,
            vec![rule(
                cons(None, Some(LateralAffricate), None),
                into_cons(None, Some(Stop), None),
                ANY,
                ANY,
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
        assert_eq!(run("deaspiration", "pʰatʰa"), "pata");
        assert_eq!(run("aspirate-spirantization", "kʰapʰa"), "xafa");
        assert_eq!(run("breathy-loss", "bʱadʱa"), "bada");
        assert_eq!(run("labiovelar-to-labial", "kʷigʷa"), "piba");
        assert_eq!(run("delabialization", "kʷaxʷa"), "kaxa");
        assert_eq!(run("retroflex-merger", "ʈaɳa"), "tana");
        assert_eq!(run("pharyngeal-weakening", "ħaʕa"), "haʔa");
        assert_eq!(run("uvular-fronting", "qaχa"), "kaxa");
        assert_eq!(run("lateral-affricate-loss", "tɬatɬ"), "tat");
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

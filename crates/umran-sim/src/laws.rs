use crate::change::{Env, Matcher, Rewrite, SoundChange};
use crate::form::Form;
use crate::inventory::preference;
use crate::phoneme::{Backness, Height, Manner, PhonemeId, Place, Secondary};
use crate::profile::InventoryPrior;
use crate::prosody::{MinimalWord, StressRule};
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
    /// A suprasegmental law, assessed by movement of word stress.
    pub stress: Option<StressRule>,
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
    /// The law's rules in order, each passing by a word it would wear
    /// below the language's `minimal` size.
    pub fn apply(&self, form: &Form, minimal: MinimalWord, stress: StressRule) -> Form {
        let mut current = form.clone();
        for rule in &self.rules {
            let next = rule.apply(&current, stress);
            if !minimal.blocks(&current, &next) {
                current = next;
            }
        }
        current
    }
    pub fn changes(&self, form: &Form, after: &Form, stress: StressRule) -> bool {
        form != after
            || self
                .stress
                .is_some_and(|next| form.stressed_syllable(stress) != after.stressed_syllable(next))
    }

    /// `None` when the law would change nothing.
    pub fn assess<'a>(
        &self,
        forms: impl Iterator<Item = &'a Form>,
        prior: &InventoryPrior,
        minimal: MinimalWord,
        stress: StressRule,
    ) -> Option<Assessment> {
        if self.stress == Some(stress) {
            return None;
        }
        if let Some(next) = self.stress {
            let words = forms
                .filter(|f| f.stressed_syllable(stress) != f.stressed_syllable(next))
                .count();
            return (words > 0).then_some(Assessment {
                words,
                pull: 0.0,
                shifts: Vec::new(),
            });
        }
        let (mut words, mut total) = (0, 0.0);
        let mut shifts = Vec::new();
        for form in forms {
            // Judge by the real result, so matches blocked by last-vowel
            // protection or the minimal word do not make a law look
            // applicable.
            if !self.changes(form, &self.apply(form, minimal, stress), stress) {
                continue;
            }
            words += 1;
            let mut current = form.clone();
            for rule in &self.rules {
                let next = rule.apply(&current, stress);
                if minimal.blocks(&current, &next) {
                    continue;
                }
                for (i, out) in rule.hits(&current, stress) {
                    let old = current.segs[i].phone;
                    let out = match rule.result {
                        Rewrite::GeminateNext => current.segs.get(i + 1).map(|s| s.phone),
                        _ => out.map(|s| s.phone),
                    };
                    if let Some(new) = out {
                        total += preference(prior, new) - preference(prior, old);
                    }
                    shifts.push((old, out));
                }
                current = next;
            }
        }
        (words > 0).then(|| Assessment {
            words,
            pull: total / shifts.len().max(1) as f32,
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
        stress: None,
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
fn length(target: Matcher, long: bool) -> Matcher {
    Matcher::Length {
        target: Box::new(target),
        long,
    }
}

fn stressed(target: Matcher, stressed: bool) -> Matcher {
    Matcher::Stressed {
        target: Box::new(target),
        stressed,
    }
}

fn phone(ipa: &str) -> Matcher {
    Matcher::Phone(crate::CATALOG.id_by_ipa(ipa).unwrap())
}

fn stress_law(id: &'static str, label: &'static str, stress: StressRule) -> Law {
    Law {
        stress: Some(stress),
        ..law(id, label, 0.35, Vec::new())
    }
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
                length(cons(None, Some(Stop), Some(false)), false),
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
        law(
            "unstressed-reduction",
            "Unstressed vowels reduce to schwa",
            0.7,
            vec![rule(
                stressed(Matcher::AnyVowel, false),
                Rewrite::Phone(crate::CATALOG.id_by_ipa("ə").unwrap()),
                ANY,
                ANY,
            )],
        ),
        law(
            "unstressed-syncope",
            "Unstressed medial vowels disappear between single consonants",
            0.45,
            vec![rule(
                stressed(Matcher::MedialVowel, false),
                Rewrite::Delete,
                at(length(Matcher::AnyConsonant, false)),
                at(length(Matcher::AnyConsonant, false)),
            )],
        ),
        law(
            "unstressed-apocope",
            "Unstressed final vowels are lost",
            0.45,
            vec![rule(
                stressed(Matcher::AnyVowel, false),
                Rewrite::Delete,
                c(),
                EDGE,
            )],
        ),
        law(
            "stressed-open-lengthening",
            "Stressed vowels lengthen in open syllables",
            0.6,
            vec![rule(
                stressed(Matcher::OpenSyllable(Box::new(Matcher::AnyVowel)), true),
                Rewrite::Length(true),
                ANY,
                ANY,
            )],
        ),
        law(
            "verner-voicing",
            "Fricatives voice after an unstressed vowel",
            0.45,
            vec![rule(
                length(cons(None, Some(Fricative), Some(false)), false),
                into_cons(None, None, Some(true)),
                at(stressed(Matcher::AnyVowel, false)),
                ANY,
            )],
        ),
        law(
            "cluster-gemination",
            "Clusters assimilate to geminates: kt, pt > tt; ks > ss",
            0.7,
            vec![
                rule(phone("k"), Rewrite::GeminateNext, ANY, at(phone("t"))),
                rule(phone("p"), Rewrite::GeminateNext, ANY, at(phone("t"))),
                rule(phone("k"), Rewrite::GeminateNext, ANY, at(phone("s"))),
            ],
        ),
        law(
            "j-gemination",
            "Consonants lengthen before j",
            0.6,
            vec![rule(
                length(Matcher::AnyConsonant, false),
                Rewrite::Length(true),
                v(),
                at(phone("j")),
            )],
        ),
        law(
            "degemination",
            "Long consonants shorten",
            0.6,
            vec![rule(
                length(Matcher::AnyConsonant, true),
                Rewrite::Length(false),
                ANY,
                ANY,
            )],
        ),
        law(
            "romance-lenition",
            "Single stops voice between vowels, then long consonants shorten",
            0.65,
            vec![
                rule(
                    length(cons(None, Some(Stop), Some(false)), false),
                    into_cons(None, None, Some(true)),
                    v(),
                    v(),
                ),
                rule(
                    length(Matcher::AnyConsonant, true),
                    Rewrite::Length(false),
                    ANY,
                    ANY,
                ),
            ],
        ),
        law(
            "compensatory-lengthening",
            "Preconsonantal nasals disappear and lengthen the preceding vowel",
            0.45,
            vec![rule(
                cons(None, Some(Nasal), None),
                Rewrite::Compensate,
                v(),
                c(),
            )],
        ),
        stress_law(
            "initial-stress",
            "Stress moves to the first syllable",
            StressRule::Initial,
        ),
        stress_law(
            "penult-stress",
            "Stress moves to the penult",
            StressRule::Penult,
        ),
        law(
            "koine-levelling",
            "Minority sounds merge into majority sounds",
            0.0,
            // Its exact rules depend on the city's makeup, not a portable
            // catalog change. The resulting variety's later laws can spread.
            Vec::new(),
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
        law.apply(
            &Form::from_ipa(ipa).unwrap(),
            MinimalWord::Syllable,
            StressRule::Initial,
        )
        .ipa()
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
        let prior = SoundProfile::by_id("polynesian").unwrap().inventory;
        let law = catalog()
            .into_iter()
            .find(|l| l.id == "spirantization")
            .unwrap();
        let forms = [Form::from_ipa("pata").unwrap()];
        let a = law
            .assess(
                forms.iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Initial,
            )
            .unwrap();
        assert_eq!(a.words, 1);
        assert!(a.pull < 0.0);
        let untouched = [Form::from_ipa("ama").unwrap()];
        assert_eq!(
            law.assess(
                untouched.iter(),
                &prior,
                MinimalWord::Syllable,
                StressRule::Initial
            ),
            None
        );
    }
}

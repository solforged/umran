use crate::generate::{Syllable, Word};
use crate::phoneme::{Backness, CATALOG, Height, Manner, PhonemeId, Place, Segment};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Matcher {
    /// Test-only; packs use feature matchers.
    #[serde(skip_serializing, skip_deserializing)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Rewrite {
    #[serde(skip_serializing, skip_deserializing)]
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
    fn matches(&self, id: PhonemeId) -> bool {
        let seg = CATALOG.get(id);
        match self {
            Matcher::Phone(want) => *want == id,
            Matcher::AnyConsonant => !seg.is_vowel(),
            Matcher::AnyVowel => seg.is_vowel(),
            Matcher::Consonant {
                place,
                manner,
                voiced,
            } => match seg {
                Segment::Consonant(c) => {
                    place.map_or(true, |p| p == c.place)
                        && manner.map_or(true, |m| m == c.manner)
                        && voiced.map_or(true, |v| v == c.voiced)
                }
                Segment::Vowel(_) => false,
            },
            Matcher::Vowel {
                height,
                backness,
                rounded,
            } => match seg {
                Segment::Vowel(v) => {
                    height.map_or(true, |h| h == v.height)
                        && backness.map_or(true, |b| b == v.backness)
                        && rounded.map_or(true, |r| r == v.rounded)
                }
                Segment::Consonant(_) => false,
            },
        }
    }
}

fn left_ok(env: &Env, segs: &[PhonemeId], i: usize) -> bool {
    match env {
        Env::Any => true,
        Env::WordEdge => i == 0,
        Env::Matcher(m) => i > 0 && m.matches(segs[i - 1]),
    }
}

fn right_ok(env: &Env, segs: &[PhonemeId], i: usize) -> bool {
    match env {
        Env::Any => true,
        Env::WordEdge => i + 1 == segs.len(),
        Env::Matcher(m) => i + 1 < segs.len() && m.matches(segs[i + 1]),
    }
}

/// `None` deletes; `Some` replaces (possibly with the original id).
fn rewrite(id: PhonemeId, result: &Rewrite) -> Option<PhonemeId> {
    match result {
        Rewrite::Delete => None,
        Rewrite::Phone(p) => Some(*p),
        Rewrite::Consonant {
            place,
            manner,
            voiced,
        } => {
            let Segment::Consonant(c) = CATALOG.get(id) else {
                return Some(id);
            };
            let place = place.unwrap_or(c.place);
            let manner = manner.unwrap_or(c.manner);
            let voiced = voiced.unwrap_or(c.voiced);
            Some(
                CATALOG
                    .consonants()
                    .find_map(|(cid, cand)| {
                        (cand.place == place && cand.manner == manner && cand.voiced == voiced)
                            .then_some(cid)
                    })
                    .unwrap_or(id),
            )
        }
        Rewrite::Vowel {
            height,
            backness,
            rounded,
        } => {
            let Segment::Vowel(v) = CATALOG.get(id) else {
                return Some(id);
            };
            let height = height.unwrap_or(v.height);
            let backness = backness.unwrap_or(v.backness);
            let rounded = rounded.unwrap_or(v.rounded);
            Some(
                CATALOG
                    .vowels()
                    .find_map(|(vid, cand)| {
                        (cand.height == height
                            && cand.backness == backness
                            && cand.rounded == rounded)
                            .then_some(vid)
                    })
                    .unwrap_or(id),
            )
        }
    }
}

fn apply_rule_tracked(
    segs: &mut Vec<PhonemeId>,
    long: &mut Vec<bool>,
    ancestry: &mut Vec<usize>,
    rule: &SoundChange,
) {
    let mut i = 0;
    while i < segs.len() {
        if rule.target.matches(segs[i])
            && left_ok(&rule.left, segs, i)
            && right_ok(&rule.right, segs, i)
        {
            match rewrite(segs[i], &rule.result) {
                None => {
                    let deleting_vowel = CATALOG.get(segs[i]).is_vowel();
                    let vowels = segs
                        .iter()
                        .filter(|id| CATALOG.get(**id).is_vowel())
                        .count();
                    if deleting_vowel && vowels <= 1 {
                        i += 1;
                        continue;
                    }
                    segs.remove(i);
                    long.remove(i);
                    ancestry.remove(i);
                }
                Some(new_id) => {
                    segs[i] = new_id;
                    if !CATALOG.get(new_id).is_vowel() {
                        long[i] = false;
                    }
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }
}

pub(crate) struct AppliedForm {
    pub phones: Vec<PhonemeId>,
    pub long: Vec<bool>,
    pub ancestry: Vec<usize>,
}

pub(crate) fn word_length_flags(word: &Word) -> Vec<bool> {
    let mut flags = Vec::new();
    for syl in &word.syllables {
        flags.extend(std::iter::repeat(false).take(syl.onset.len()));
        flags.extend(std::iter::repeat(syl.long).take(syl.nucleus.len()));
        flags.extend(std::iter::repeat(false).take(syl.coda.len()));
    }
    flags
}

pub(crate) fn map_offset(ancestry: &[usize], orig_offset: usize) -> usize {
    ancestry
        .iter()
        .position(|&i| i >= orig_offset)
        .unwrap_or(ancestry.len())
}

pub(crate) fn apply_changes_tracked(
    phones: &[PhonemeId],
    long: &[bool],
    rules: &[SoundChange],
) -> AppliedForm {
    let mut segs = phones.to_vec();
    let mut long = if long.len() == segs.len() {
        long.to_vec()
    } else {
        vec![false; segs.len()]
    };
    let mut ancestry: Vec<usize> = (0..segs.len()).collect();
    for rule in rules {
        apply_rule_tracked(&mut segs, &mut long, &mut ancestry, rule);
    }
    AppliedForm {
        phones: segs,
        long,
        ancestry,
    }
}

pub(crate) fn realize_phones(segs: &[PhonemeId]) -> Word {
    realize_phones_long(segs, &vec![false; segs.len()])
}

pub(crate) fn realize_phones_long(segs: &[PhonemeId], long: &[bool]) -> Word {
    let mut syllables = resyllabify(segs, segs.is_empty());
    let mut i = 0;
    for syl in &mut syllables {
        let nuc_from = i + syl.onset.len();
        let nuc_to = nuc_from + syl.nucleus.len();
        syl.long = long
            .get(nuc_from..nuc_to)
            .map(|slice| slice.iter().any(|flag| *flag))
            .unwrap_or(false);
        i += syl.onset.len() + syl.nucleus.len() + syl.coda.len();
    }
    Word {
        syllables,
        join_at: None,
    }
}

fn join_phone_offset(word: &Word, join_at: usize) -> usize {
    word.syllables
        .iter()
        .take(join_at)
        .map(|s| s.onset.len() + s.nucleus.len() + s.coda.len())
        .sum()
}

fn syllable_at_phone(syllables: &[Syllable], offset: usize) -> Option<usize> {
    let mut i = 0;
    for (idx, syl) in syllables.iter().enumerate() {
        let n = syl.onset.len() + syl.nucleus.len() + syl.coda.len();
        if offset < i + n {
            return Some(idx);
        }
        i += n;
    }
    None
}

fn rising_sonority(ids: &[PhonemeId]) -> bool {
    ids.windows(2)
        .all(|w| CATALOG.get(w[0]).sonority() < CATALOG.get(w[1]).sonority())
}

fn split_intervocalic(cluster: &[PhonemeId]) -> (Vec<PhonemeId>, Vec<PhonemeId>) {
    for onset_len in (0..=cluster.len()).rev() {
        let split = cluster.len() - onset_len;
        if rising_sonority(&cluster[split..]) {
            return (cluster[..split].to_vec(), cluster[split..].to_vec());
        }
    }
    (cluster.to_vec(), Vec::new())
}

fn empty_syllable() -> Syllable {
    Syllable {
        onset: Vec::new(),
        nucleus: Vec::new(),
        coda: Vec::new(),
        long: false,
    }
}

fn resyllabify(segs: &[PhonemeId], input_empty: bool) -> Vec<Syllable> {
    let nuclei: Vec<usize> = segs
        .iter()
        .enumerate()
        .filter(|&(_, &id)| CATALOG.get(id).is_vowel())
        .map(|(i, _)| i)
        .collect();

    if nuclei.is_empty() {
        if segs.is_empty() {
            return if input_empty {
                vec![empty_syllable()]
            } else {
                Vec::new()
            };
        }
        return vec![Syllable {
            onset: Vec::new(),
            nucleus: Vec::new(),
            coda: segs.to_vec(),
            long: false,
        }];
    }

    let mut syllables: Vec<Syllable> = Vec::with_capacity(nuclei.len());
    for (n, &vpos) in nuclei.iter().enumerate() {
        let cluster_from = if n == 0 { 0 } else { nuclei[n - 1] + 1 };
        let cluster = &segs[cluster_from..vpos];
        let onset = if n == 0 {
            cluster.to_vec()
        } else {
            let (coda, onset) = split_intervocalic(cluster);
            syllables.last_mut().unwrap().coda = coda;
            onset
        };
        syllables.push(Syllable {
            onset,
            nucleus: vec![segs[vpos]],
            coda: Vec::new(),
            long: false,
        });
    }
    let last_v = *nuclei.last().unwrap();
    if last_v + 1 < segs.len() {
        syllables.last_mut().unwrap().coda = segs[last_v + 1..].to_vec();
    }
    syllables
}

pub fn apply_changes(word: &Word, rules: &[SoundChange]) -> Word {
    let phones: Vec<PhonemeId> = word.phonemes().collect();
    let long = word_length_flags(word);
    let applied = apply_changes_tracked(&phones, &long, rules);
    if applied.phones == phones {
        return word.clone();
    }
    let mut next = realize_phones_long(&applied.phones, &applied.long);
    next.join_at = word.join_at.and_then(|j| {
        if j == 0 {
            return None;
        }
        let orig = join_phone_offset(word, j);
        let mapped = map_offset(&applied.ancestry, orig);
        if mapped == 0 || mapped >= applied.phones.len() {
            return None;
        }
        syllable_at_phone(&next.syllables, mapped).filter(|&s| s > 0 && s < next.syllables.len())
    });
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::Syllable;

    fn id(ipa: &str) -> PhonemeId {
        CATALOG.id_by_ipa(ipa).unwrap()
    }

    fn cv(c: &str, v: &str) -> Syllable {
        Syllable {
            onset: vec![id(c)],
            nucleus: vec![id(v)],
            coda: Vec::new(),
            long: false,
        }
    }

    fn word(syllables: Vec<Syllable>) -> Word {
        Word {
            syllables,
            join_at: None,
        }
    }

    fn ipa(w: &Word) -> String {
        CATALOG.ipa_string(&w.phonemes().collect::<Vec<_>>())
    }

    fn t_to_s_before_i() -> SoundChange {
        SoundChange {
            id: "t-s/_i".into(),
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
    fn t_becomes_s_before_i() {
        let out = apply_changes(&word(vec![cv("t", "i")]), &[t_to_s_before_i()]);
        assert_eq!(ipa(&out), "si");
    }

    #[test]
    fn apocope_deletes_final_vowel() {
        let out = apply_changes(&word(vec![cv("k", "a"), cv("t", "a")]), &[apocope()]);
        assert_eq!(ipa(&out), "kat");

        let out = apply_changes(&word(vec![cv("t", "a")]), &[apocope()]);
        assert_eq!(ipa(&out), "ta");
    }

    #[test]
    fn unmatched_feature_rewrite_keeps_phone() {
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
        let out = apply_changes(&word(vec![cv("t", "a")]), &[rule]);
        assert_eq!(ipa(&out), "ta");
        assert_eq!(out.phonemes().next(), Some(id("t")));
    }

    #[test]
    fn apply_changes_is_deterministic() {
        let rules = [t_to_s_before_i(), apocope()];
        let w = word(vec![cv("t", "i"), cv("k", "a")]);
        let a = apply_changes(&w, &rules);
        let b = apply_changes(&w, &rules);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        assert_eq!(ipa(&a), ipa(&b));
        assert_eq!(ipa(&a), "sik");
    }

    #[test]
    fn unmatched_rule_preserves_vowel_length() {
        let mut w = word(vec![cv("k", "a"), cv("t", "a")]);
        w.syllables[0].long = true;
        w.join_at = Some(1);
        let out = apply_changes(&w, &[t_to_s_before_i()]);
        assert_eq!(ipa(&out), "kata");
        assert!(out.syllables[0].long);
        assert_eq!(out.join_at, Some(1));
    }

    #[test]
    fn surviving_length_and_join_follow_phone_ancestry() {
        let mut w = word(vec![cv("k", "a"), cv("t", "i")]);
        w.syllables[0].long = true;
        w.join_at = Some(1);
        let out = apply_changes(&w, &[t_to_s_before_i()]);
        assert_eq!(ipa(&out), "kasi");
        assert!(out.syllables[0].long);
        assert_eq!(out.join_at, Some(1));
    }
}

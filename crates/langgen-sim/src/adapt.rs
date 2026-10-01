use crate::form::{Form, Seg};
use crate::inventory::preference;
use crate::phoneme::{CATALOG, Manner, PhonemeId, Segment};
use crate::profile::InventoryPrior;
use rand::Rng;
use std::collections::{BTreeMap, HashSet};

/// Below this share of closed final syllables, a language is treated as
/// allowing no codas at all, and loans get a vowel after final consonants.
const OPEN_SYLLABLE_THRESHOLD: f32 = 0.05;
/// Share of living words a sound must occur in to count as native.
pub(crate) const ESTABLISHED_SHARE: f32 = 0.02;
/// Preference score at which a foreign segment is kept half the time it
/// could be: the same threshold inventory sampling uses.
const ACCEPTANCE_MIDPOINT: f32 = 0.45;
/// Most repairs one loan can need; a safety bound, not a typological claim.
const MAX_REPAIRS: usize = 12;

/// Phonetic distance between two segments by features. Consonants and
/// vowels never substitute for each other.
pub fn distance(a: PhonemeId, b: PhonemeId) -> f32 {
    match (CATALOG.get(a), CATALOG.get(b)) {
        (Segment::Consonant(x), Segment::Consonant(y)) => {
            manner_distance(x.manner, y.manner)
                + 0.35 * (x.place as i32 - y.place as i32).abs() as f32
                + if x.voiced == y.voiced { 0.0 } else { 0.6 }
                + if x.secondary == y.secondary { 0.0 } else { 0.4 }
        }
        (Segment::Vowel(x), Segment::Vowel(y)) => {
            0.5 * (x.height as i32 - y.height as i32).abs() as f32
                + 1.0 * (x.backness as i32 - y.backness as i32).abs() as f32
                + if x.rounded == y.rounded { 0.0 } else { 0.8 }
        }
        _ => f32::INFINITY,
    }
}

fn manner_distance(a: Manner, b: Manner) -> f32 {
    use Manner::*;
    let obstruent = |m| {
        matches!(
            m,
            Stop | Affricate | Fricative | Ejective | Implosive | LateralFricative
        )
    };
    match (a, b) {
        _ if a == b => 0.0,
        (Stop, Ejective | Implosive) | (Ejective | Implosive, Stop) => 0.5,
        (Trill, Tap) | (Tap, Trill) => 0.3,
        (Lateral, LateralFricative) | (LateralFricative, Lateral) => 1.0,
        (Nasal, Stop) | (Stop, Nasal) => 1.5,
        _ if obstruent(a) == obstruent(b) => 1.0,
        _ => 2.0,
    }
}

/// How one recipient language hears and reshapes foreign words.
#[derive(Clone, Debug)]
pub struct Adapter {
    native: Vec<PhonemeId>,
    prior: InventoryPrior,
    onsets: HashSet<Vec<PhonemeId>>,
    codas: HashSet<Vec<PhonemeId>>,
    open: bool,
    has_length: bool,
    epenthetic: Option<PhonemeId>,
}

impl Adapter {
    /// Built from the recipient's living words: its established segments,
    /// attested onsets and codas, and its most frequent close vowel (or
    /// most frequent vowel) for breaking up illegal clusters. `prior` is the
    /// recipient culture's sound preferences.
    pub fn new<'a>(forms: impl Iterator<Item = &'a Form>, prior: &InventoryPrior) -> Self {
        let mut in_words: BTreeMap<u16, u32> = BTreeMap::new();
        let mut onsets = HashSet::new();
        let mut codas = HashSet::new();
        let mut vowels: BTreeMap<u16, u32> = BTreeMap::new();
        let (mut words, mut closed, mut has_length) = (0u32, 0u32, false);
        for form in forms {
            let syllables = form.syllables();
            let Some(last) = syllables.last() else {
                continue;
            };
            words += 1;
            closed += u32::from(!last.coda.is_empty());
            let mut seen: Vec<PhonemeId> = Vec::new();
            for seg in &form.segs {
                if !seen.contains(&seg.phone) {
                    seen.push(seg.phone);
                    *in_words.entry(seg.phone.0).or_default() += 1;
                }
                has_length |= seg.long;
            }
            for s in &syllables {
                *vowels.entry(form.segs[s.nucleus].phone.0).or_default() += 1;
                onsets.insert(form.segs[s.onset.clone()].iter().map(|x| x.phone).collect());
                codas.insert(form.segs[s.coda.clone()].iter().map(|x| x.phone).collect());
            }
        }
        // A sound in only a handful of words is marginal: most speakers
        // still adapt it in new loans.
        let established = (words as f32 * ESTABLISHED_SHARE).max(2.0);
        let native: Vec<PhonemeId> = in_words
            .into_iter()
            .filter(|&(_, n)| n as f32 >= established)
            .map(|(id, _)| PhonemeId(id))
            .collect();
        let close = |id: u16| {
            CATALOG
                .get(PhonemeId(id))
                .vowel()
                .is_some_and(|v| matches!(v.height, crate::phoneme::Height::Close))
        };
        let most = |filter: &dyn Fn(u16) -> bool| {
            vowels
                .iter()
                .filter(|(id, _)| filter(**id))
                .max_by_key(|(id, n)| (**n, std::cmp::Reverse(**id)))
                .map(|(id, _)| PhonemeId(*id))
        };
        let epenthetic = most(&close).or_else(|| most(&|_| true));
        Self {
            native,
            prior: prior.clone(),
            onsets,
            codas,
            open: words > 0 && (closed as f32 / words as f32) < OPEN_SYLLABLE_THRESHOLD,
            has_length,
            epenthetic,
        }
    }

    pub fn is_native(&self, id: PhonemeId) -> bool {
        self.native.contains(&id)
    }

    /// The loan as recipient speakers would say it. A foreign segment
    /// survives with probability `keep_foreign` (bilingualism) times how
    /// acceptable the recipient culture finds it; otherwise it becomes the
    /// nearest native segment. Then onset clusters and codas
    /// the recipient never uses are broken up with an epenthetic vowel.
    pub fn adapt(&self, source: &Form, keep_foreign: f32, rng: &mut impl Rng) -> Form {
        let segs = source
            .segs
            .iter()
            .map(|seg| {
                let keep = keep_foreign * self.acceptance(seg.phone);
                let phone = if self.is_native(seg.phone) || rng.r#gen::<f32>() < keep {
                    seg.phone
                } else {
                    self.nearest(seg.phone)
                };
                let long = seg.long && self.has_length && CATALOG.get(phone).is_vowel();
                Seg { phone, long }
            })
            .collect();
        let mut form = Form {
            segs,
            boundaries: Vec::new(),
        };
        for _ in 0..MAX_REPAIRS {
            let Some(at) = self.violation(&form) else {
                break;
            };
            let Some(vowel) = self.epenthetic else { break };
            form.segs.insert(
                at,
                Seg {
                    phone: vowel,
                    long: false,
                },
            );
        }
        form
    }

    /// 0–1: how readily speakers keep a foreign segment, a logistic curve
    /// on their preference score centred where inventories stop sampling.
    fn acceptance(&self, id: PhonemeId) -> f32 {
        1.0 / (1.0 + (-(preference(&self.prior, id) - ACCEPTANCE_MIDPOINT)).exp())
    }

    fn nearest(&self, id: PhonemeId) -> PhonemeId {
        self.native
            .iter()
            .copied()
            .filter(|n| distance(id, *n).is_finite())
            .min_by(|a, b| {
                distance(id, *a)
                    .total_cmp(&distance(id, *b))
                    .then(a.0.cmp(&b.0))
            })
            .unwrap_or(id)
    }

    /// Where to insert a vowel to fix the first illegal onset or coda.
    fn violation(&self, form: &Form) -> Option<usize> {
        let phones = |r: std::ops::Range<usize>| -> Vec<PhonemeId> {
            form.segs[r].iter().map(|s| s.phone).collect()
        };
        for s in form.syllables() {
            if s.onset.len() > 1 && !self.onsets.contains(&phones(s.onset.clone())) {
                return Some(s.onset.start + 1);
            }
            let coda_bad = if self.open {
                !s.coda.is_empty()
            } else {
                s.coda.len() > 1 && !self.codas.contains(&phones(s.coda.clone()))
            };
            if coda_bad {
                return Some(s.coda.start + 1);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::SoundProfile;
    use crate::rng::stream;

    /// A recipient that likes /b/ and forbids /θ/. Each word counts twice
    /// so every sound in it is established rather than marginal.
    fn adapter(words: &[&str]) -> Adapter {
        let forms: Vec<Form> = words
            .iter()
            .chain(words)
            .map(|w| Form::from_ipa(w).unwrap())
            .collect();
        let mut prior = SoundProfile::by_id("typical").unwrap().inventory;
        prior.extra.push(("b".into(), 5.0));
        prior.forbidden.push("θ".into());
        Adapter::new(forms.iter(), &prior)
    }

    fn adapt(adapter: &Adapter, ipa: &str, keep: f32) -> String {
        adapter
            .adapt(&Form::from_ipa(ipa).unwrap(), keep, &mut stream(1, &[]))
            .ipa()
    }

    #[test]
    fn foreign_sounds_map_to_nearest_native_ones() {
        let a = adapter(&["pata", "kisu", "mana", "lasu"]);
        assert_eq!(adapt(&a, "θasa", 0.0), "sasa");
        assert_eq!(adapt(&a, "bota", 0.0), "puta");
        assert_eq!(
            adapt(&a, "bata", 1.0),
            "bata",
            "bilingual speakers keep liked sounds"
        );
        assert_eq!(adapt(&a, "θasa", 1.0), "sasa", "but rarely forbidden ones");
    }

    #[test]
    fn marginal_sounds_are_still_adapted() {
        // /b/ occurs in one word of fifty: present, but not established.
        let mut words = vec!["pata"; 49];
        words.push("ba");
        let forms: Vec<Form> = words.iter().map(|w| Form::from_ipa(w).unwrap()).collect();
        let prior = SoundProfile::by_id("typical").unwrap().inventory;
        let a = Adapter::new(forms.iter(), &prior);
        assert!(!a.is_native(CATALOG.id_by_ipa("b").unwrap()));
        assert_eq!(adapt(&a, "bata", 0.0), "pata");
    }

    #[test]
    fn open_syllable_languages_break_clusters_and_codas() {
        let a = adapter(&["pata", "kisu", "mana", "lasu", "tiki"]);
        assert_eq!(adapt(&a, "stak", 0.0), "sitaki");
        let closed = adapter(&["pat", "kis", "man", "stal"]);
        assert_eq!(
            adapt(&closed, "stak", 0.0),
            "stak",
            "attested clusters and codas stay"
        );
    }
}

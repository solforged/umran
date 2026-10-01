use crate::concepts::{Concept, Iconic};
use crate::form::Form;
use crate::inventory::Inventory;
use crate::phoneme::{Backness, CATALOG, Height, Manner, PhonemeId, Place, Segment};
use crate::profile::PhonotacticPrior;
use crate::rng::weighted_index;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Chance a parent word follows the nursery pattern.
const NURSERY: f32 = 0.8;
/// Chance a nursery word is doubled (mama rather than ma).
const NURSERY_DOUBLED: f32 = 0.7;
/// Chance an expressive meaning gets a fully reduplicated root (kuku).
const REDUPLICATION: f32 = 0.3;
/// Repeat tolerance for expressive meanings, at least.
const EXPRESSIVE_REPEATS: f32 = 0.6;
/// Share of a language's repeat tolerance that plain meanings get.
const PLAIN_REPEATS: f32 = 0.25;
/// Draws a root gets to avoid repeating a consonant.
const REPEAT_TRIES: usize = 8;
/// Repeat tolerance for roots coined from a language's current sounds.
const OBSERVED_REPEATS: f32 = 0.1;
/// Weight added to a preferred onset or coda already in the inventory;
/// a preferred cluster enters with `PREFERRED_WEIGHT + 1`.
const PREFERRED_WEIGHT: f32 = 6.0;
/// How strongly a sound-symbolic concept favours its associated segments.
/// Deliberately weak: a tendency, not a rule.
const ICONIC_BOOST: f32 = 3.0;

/// A variety's syllable inventory: weighted onsets, nuclei, and codas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Phonotactics {
    pub onsets: Vec<(Vec<PhonemeId>, f32)>,
    pub nuclei: Vec<(PhonemeId, f32)>,
    pub codas: Vec<(Vec<PhonemeId>, f32)>,
    pub final_coda: f32,
    pub open_medial: bool,
    pub disyllabic_roots: f32,
    /// Chance a root may repeat a consonant.
    pub identical_consonants: f32,
}

impl Phonotactics {
    pub fn compile(prior: &PhonotacticPrior, inventory: &Inventory) -> Self {
        let singles = || -> Vec<(Vec<PhonemeId>, f32)> {
            inventory
                .consonants
                .iter()
                .map(|&id| (vec![id], inventory.weight(id)))
                .collect()
        };
        let mut onsets = singles();
        let mut codas = singles();
        prefer(&mut onsets, inventory, &prior.preferred_onsets);
        prefer(&mut codas, inventory, &prior.preferred_codas);
        // Velar and palatal nasals, glottal stops, and palatal laterals are
        // rare word-initially unless a profile asks for them.
        onsets.retain(|(ids, w)| {
            let rare = ids.len() == 1 && matches!(CATALOG.get(ids[0]).ipa(), "ŋ" | "ɲ" | "ʔ" | "ʎ");
            ids.len() <= prior.max_onset as usize && !(rare && *w < PREFERRED_WEIGHT)
        });
        codas.retain(|(ids, _)| ids.len() <= prior.max_coda as usize);
        Self {
            onsets,
            nuclei: inventory
                .vowels
                .iter()
                .map(|&id| (id, inventory.weight(id)))
                .collect(),
            codas,
            final_coda: prior.final_coda.clamp(0.0, 1.0),
            open_medial: prior.open_medial,
            disyllabic_roots: prior.disyllabic_roots.clamp(0.0, 1.0),
            identical_consonants: prior.identical_consonants.clamp(0.0, 1.0),
        }
    }

    /// How many syllables a new root for `concept` gets: the language's
    /// typical length, stretched for rarer meanings and shrunk for basic
    /// ones (Zipf's law of abbreviation). At most three.
    pub fn syllables_for(&self, rng: &mut impl Rng, concept: &Concept) -> usize {
        let bias = concept.length_bias();
        let two = (self.disyllabic_roots * bias).min(0.9);
        let three = (self.disyllabic_roots * bias - 0.6).clamp(0.0, 0.5);
        if rng.r#gen::<f32>() >= two {
            1
        } else if rng.r#gen::<f32>() >= three {
            2
        } else {
            3
        }
    }

    /// A root for `concept` with `syllables` syllables: open syllables, then
    /// an optional final consonant, onsets throughout while the language
    /// has any. Parent words usually follow the nursery pattern; expressive
    /// meanings may reduplicate or repeat consonants, which other meanings
    /// avoid.
    pub fn root(&self, rng: &mut impl Rng, concept: &Concept, syllables: usize) -> Form {
        if let Some(form) = self.nursery(rng, concept) {
            return form;
        }
        let onsets = singles(&self.onsets);
        let codas = singles(&self.codas);
        let iconic = concept.iconic;
        if concept.expressive && !onsets.is_empty() && rng.r#gen::<f32>() < REDUPLICATION {
            let c = pick(rng, &onsets, iconic);
            let v = pick(rng, &self.nuclei, iconic);
            return Form::from_phones([c, v, c, v]);
        }
        let tolerance = if concept.expressive {
            self.identical_consonants.max(EXPRESSIVE_REPEATS)
        } else {
            self.identical_consonants * PLAIN_REPEATS
        };
        let mut phones = Vec::with_capacity(2 * syllables + 1);
        for _ in 0..REPEAT_TRIES {
            phones.clear();
            for _ in 0..syllables.max(1) {
                // A language that has lost every initial consonant coins
                // vowel-initial words.
                if !onsets.is_empty() {
                    phones.push(pick(rng, &onsets, iconic));
                }
                phones.push(pick(rng, &self.nuclei, iconic));
            }
            if !codas.is_empty() && rng.r#gen::<f32>() < self.final_coda {
                phones.push(pick(rng, &codas, iconic));
            }
            // Most meanings avoid repeating a consonant within a root.
            if !repeats_consonant(&phones) || rng.r#gen::<f32>() < tolerance {
                break;
            }
        }
        Form::from_phones(phones)
    }

    /// Jakobson's "mama" and "papa": the earliest babbled syllables, a
    /// nasal or a lip or tongue-tip stop with an open vowel, become parent
    /// words in language after language. Usually, not always.
    fn nursery(&self, rng: &mut impl Rng, concept: &Concept) -> Option<Form> {
        let fits = |id: PhonemeId, mother: bool| match CATALOG.get(id) {
            Segment::Consonant(c) if mother => match (c.manner, c.place) {
                (Manner::Nasal, Place::Bilabial) => 3.0,
                (Manner::Nasal, Place::Alveolar | Place::Dental) => 1.0,
                _ => 0.0,
            },
            Segment::Consonant(c) => match (c.manner, c.place) {
                (Manner::Stop, Place::Bilabial) => 2.0,
                (Manner::Stop, Place::Alveolar | Place::Dental) => 1.0,
                _ => 0.0,
            },
            Segment::Vowel(_) => 0.0,
        };
        let mother = match concept.iconic {
            Some(Iconic::NurseryMother) => true,
            Some(Iconic::NurseryFather) => false,
            _ => return None,
        };
        if rng.r#gen::<f32>() >= NURSERY {
            return None;
        }
        let candidates: Vec<(PhonemeId, f32)> = singles(&self.onsets)
            .into_iter()
            .map(|(id, _)| (id, fits(id, mother)))
            .filter(|&(_, w)| w > 0.0)
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let c = candidates[weighted_index(rng, candidates.iter().map(|(_, w)| *w))].0;
        let open = self
            .nuclei
            .iter()
            .filter_map(|(id, _)| CATALOG.get(*id).vowel().map(|v| (*id, v.height)))
            .max_by_key(|(id, height)| (*height as u8, std::cmp::Reverse(id.0)))?
            .0;
        Some(if rng.r#gen::<f32>() < NURSERY_DOUBLED {
            Form::from_phones([c, open, c, open])
        } else {
            Form::from_phones([c, open])
        })
    }
    /// Syllable statistics of the words a language has now, for coining
    /// roots that sound like its present rather than its founding.
    pub fn observe<'a>(forms: impl Iterator<Item = &'a Form>) -> Self {
        let mut onsets: BTreeMap<u16, f32> = BTreeMap::new();
        let mut nuclei: BTreeMap<u16, f32> = BTreeMap::new();
        let mut codas: BTreeMap<u16, f32> = BTreeMap::new();
        let (mut words, mut closed, mut longer) = (0.0_f32, 0.0_f32, 0.0_f32);
        for form in forms {
            let syllables = form.syllables();
            let Some(last) = syllables.last() else {
                continue;
            };
            words += 1.0;
            closed += f32::from(!last.coda.is_empty());
            longer += f32::from(syllables.len() > 1);
            for s in &syllables {
                *nuclei.entry(form.segs[s.nucleus].phone.0).or_default() += 1.0;
                if s.onset.len() == 1 {
                    *onsets.entry(form.segs[s.onset.start].phone.0).or_default() += 1.0;
                }
                if s.coda.len() == 1 {
                    *codas.entry(form.segs[s.coda.start].phone.0).or_default() += 1.0;
                }
            }
        }
        let list = |m: BTreeMap<u16, f32>| m.into_iter().map(|(id, n)| (PhonemeId(id), n));
        Self {
            onsets: list(onsets).map(|(id, n)| (vec![id], n)).collect(),
            nuclei: list(nuclei).collect(),
            codas: list(codas).map(|(id, n)| (vec![id], n)).collect(),
            final_coda: if words > 0.0 { closed / words } else { 0.0 },
            open_medial: false,
            disyllabic_roots: if words > 0.0 { longer / words } else { 0.0 },
            identical_consonants: OBSERVED_REPEATS,
        }
    }

    /// Whether `form` is a shape `root` can produce: one to three open
    /// syllables with at most one final consonant, from this inventory.
    pub fn fits_root(&self, form: &Form) -> bool {
        let syllables = form.syllables();
        let single = |list: &[(Vec<PhonemeId>, f32)], id: PhonemeId| {
            list.iter().any(|(ids, _)| ids.as_slice() == [id])
        };
        let shape_ok = (1..=3).contains(&syllables.len())
            && syllables.iter().rev().skip(1).all(|s| s.coda.is_empty())
            && syllables.last().is_some_and(|s| s.coda.len() <= 1);
        shape_ok
            && form.boundaries.is_empty()
            && form.segs.iter().all(|s| !s.long)
            && syllables.iter().all(|s| {
                s.onset.len() == 1
                    && single(&self.onsets, form.segs[s.onset.start].phone)
                    && self
                        .nuclei
                        .iter()
                        .any(|(id, _)| *id == form.segs[s.nucleus].phone)
                    && s.coda
                        .clone()
                        .all(|i| single(&self.codas, form.segs[i].phone))
            })
    }
}

fn repeats_consonant(phones: &[PhonemeId]) -> bool {
    let consonants: Vec<PhonemeId> = phones
        .iter()
        .copied()
        .filter(|p| !CATALOG.get(*p).is_vowel())
        .collect();
    consonants
        .iter()
        .enumerate()
        .any(|(i, c)| consonants[i + 1..].contains(c))
}

fn singles(list: &[(Vec<PhonemeId>, f32)]) -> Vec<(PhonemeId, f32)> {
    list.iter()
        .filter(|(ids, _)| ids.len() == 1)
        .map(|(ids, w)| (ids[0], *w))
        .collect()
}

fn pick(rng: &mut impl Rng, items: &[(PhonemeId, f32)], iconic: Option<Iconic>) -> PhonemeId {
    let boosted = items.iter().map(|&(id, w)| match iconic {
        Some(i) if evokes(i, id) => w * ICONIC_BOOST,
        _ => w,
    });
    items[weighted_index(rng, boosted)].0
}

fn evokes(iconic: Iconic, id: PhonemeId) -> bool {
    match (iconic, CATALOG.get(id)) {
        (Iconic::Nasal, Segment::Consonant(c)) => c.manner == Manner::Nasal,
        (Iconic::Lateral, Segment::Consonant(c)) => c.manner == Manner::Lateral,
        (Iconic::Rhotic, Segment::Consonant(c)) => matches!(c.manner, Manner::Trill | Manner::Tap),
        (Iconic::Sibilant, Segment::Consonant(c)) => {
            c.manner == Manner::Fricative
                && matches!(c.place, Place::Alveolar | Place::Postalveolar)
        }
        (Iconic::LabialNasal, Segment::Consonant(c)) => {
            c.manner == Manner::Nasal && c.place == Place::Bilabial
        }
        (Iconic::CloseFront, Segment::Vowel(v)) => {
            v.height == Height::Close && v.backness == Backness::Front && !v.rounded
        }
        _ => false,
    }
}

fn prefer(dest: &mut Vec<(Vec<PhonemeId>, f32)>, inventory: &Inventory, preferred: &[String]) {
    for ipa in preferred {
        let Some(ids) = CATALOG.parse_ipa(ipa) else {
            continue;
        };
        if !ids.iter().all(|id| inventory.contains(*id)) {
            continue;
        }
        match dest.iter_mut().find(|(have, _)| *have == ids) {
            Some((_, w)) => *w += PREFERRED_WEIGHT,
            None => dest.push((ids, PREFERRED_WEIGHT + 1.0)),
        }
    }
}

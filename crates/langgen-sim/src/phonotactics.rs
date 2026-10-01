use crate::concepts::Iconic;
use crate::form::Form;
use crate::inventory::Inventory;
use crate::phoneme::{Backness, CATALOG, Height, Manner, PhonemeId, Place, Segment};
use crate::preset::PhonotacticPrior;
use crate::rng::weighted_index;
use rand::Rng;
use serde::{Deserialize, Serialize};

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
        // rare word-initially unless a preset asks for them.
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
        }
    }

    /// A founding root: CV or CVC, or CVCV when `disyllabic`. Roots always
    /// have an onset and use single consonants only.
    pub fn root(&self, rng: &mut impl Rng, iconic: Option<Iconic>, disyllabic: bool) -> Form {
        let onsets = singles(&self.onsets);
        let codas = singles(&self.codas);
        let mut phones = Vec::with_capacity(4);
        for _ in 0..if disyllabic { 2 } else { 1 } {
            phones.push(pick(rng, &onsets, iconic));
            phones.push(pick(rng, &self.nuclei, iconic));
        }
        if !disyllabic && !codas.is_empty() && rng.r#gen::<f32>() < self.final_coda {
            phones.push(pick(rng, &codas, iconic));
        }
        Form::from_phones(phones)
    }

    /// Whether `form` is a shape `root` can produce.
    pub fn fits_root(&self, form: &Form) -> bool {
        let syllables = form.syllables();
        let single = |list: &[(Vec<PhonemeId>, f32)], id: PhonemeId| {
            list.iter().any(|(ids, _)| ids.as_slice() == [id])
        };
        let shape_ok = match syllables.len() {
            1 => syllables[0].coda.len() <= 1,
            2 => syllables.iter().all(|s| s.coda.is_empty()),
            _ => false,
        };
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

use crate::aesthetic::{Aesthetic, Signature};
use crate::inventory::Inventory;
use crate::phoneme::{PhonemeId, CATALOG};
use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameKind {
    Word,
    Person,
    Place,
}

#[derive(Clone, Debug)]
pub struct Syllable {
    pub onset: Vec<PhonemeId>,
    pub nucleus: Vec<PhonemeId>,
    pub coda: Vec<PhonemeId>,
    pub long: bool,
}

#[derive(Clone, Debug)]
pub struct Word {
    pub syllables: Vec<Syllable>,
    pub join_at: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Generator {
    pub onsets: Vec<(Vec<PhonemeId>, f32)>,
    pub nuclei: Vec<(Vec<PhonemeId>, f32)>,
    pub codas: Vec<(Vec<PhonemeId>, f32)>,
    empty_onset: f32,
    empty_coda: f32,
    empty_coda_nonfinal: f32,
}

impl Generator {
    pub fn compile(aesthetic: &Aesthetic, inventory: &Inventory) -> Self {
        let mut onsets = singles(inventory, true);
        let mut codas = singles(inventory, true);
        let nuclei = inventory
            .vowels
            .iter()
            .map(|&id| (vec![id], inventory.score(id)))
            .collect();

        boost_preferred(&mut onsets, inventory, &aesthetic.clusters.preferred_onsets);
        boost_preferred(&mut codas, inventory, &aesthetic.clusters.preferred_codas);

        onsets.retain(|(ids, w)| {
            *w > 0.0 && ids.len() <= aesthetic.clusters.max_onset as usize && !banned_onset(ids, *w)
        });
        codas.retain(|(ids, w)| *w > 0.0 && ids.len() <= aesthetic.clusters.max_coda as usize);

        if onsets.is_empty() {
            onsets = singles(inventory, true);
        }

        Self {
            onsets,
            nuclei,
            codas,
            empty_onset: aesthetic.clusters.empty_onset.clamp(0.0, 0.95),
            empty_coda: aesthetic.clusters.empty_coda.clamp(0.0, 0.98),
            empty_coda_nonfinal: aesthetic.clusters.empty_coda_nonfinal.clamp(0.0, 1.0),
        }
    }

    pub fn syllable(&self, rng: &mut impl Rng, word_final: bool, open_nonfinal: bool) -> Syllable {
        let onset = if self.onsets.is_empty() || rng.gen::<f32>() < self.empty_onset {
            Vec::new()
        } else {
            pick(rng, &self.onsets).clone()
        };
        let nucleus = if self.nuclei.is_empty() {
            Vec::new()
        } else {
            pick(rng, &self.nuclei).clone()
        };
        let force_open = !word_final && open_nonfinal;
        let empty_p = if word_final {
            self.empty_coda
        } else {
            self.empty_coda_nonfinal
        };
        let coda = if force_open || self.codas.is_empty() || rng.gen::<f32>() < empty_p {
            Vec::new()
        } else {
            pick(rng, &self.codas).clone()
        };
        Syllable {
            onset,
            nucleus,
            coda,
            long: false,
        }
    }

    pub fn onset_ipas(&self) -> Vec<String> {
        let mut rows: Vec<_> = self
            .onsets
            .iter()
            .map(|(ids, w)| (*w, CATALOG.ipa_string(ids)))
            .collect();
        rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        rows.into_iter().map(|(_, s)| s).collect()
    }
}

impl Word {
    pub fn generate(
        aesthetic: &Aesthetic,
        generator: &Generator,
        inventory: &Inventory,
        rng: &mut impl Rng,
        kind: NameKind,
    ) -> Self {
        let (min, max) = match kind {
            NameKind::Word => aesthetic.shape.syllables,
            NameKind::Person => aesthetic.names.person_syllables,
            NameKind::Place => aesthetic.names.place_syllables,
        };
        let min = min.max(1);
        let max = max.max(min);
        let open_nonfinal = aesthetic
            .signatures
            .iter()
            .any(|s| matches!(s, Signature::OpenNonfinal));
        let compound = kind == NameKind::Place && rng.gen::<f32>() < aesthetic.names.compound_place;
        let (mut syllables, join_at) = if compound {
            let n1 = rng.gen_range(1..=2) as usize;
            let n2 = rng.gen_range(1..=2) as usize;
            let mut s = stem(generator, n1, open_nonfinal, rng);
            let join_at = Some(s.len());
            s.extend(stem(generator, n2, open_nonfinal, rng));
            (s, join_at)
        } else {
            let n = rng.gen_range(min..=max) as usize;
            (stem(generator, n, open_nonfinal, rng), None)
        };

        apply_signatures(&mut syllables, aesthetic, join_at, rng);
        apply_ending(&mut syllables, aesthetic, inventory, rng, kind, max);
        Word { syllables, join_at }
    }

    pub fn phonemes(&self) -> impl Iterator<Item = PhonemeId> + '_ {
        self.syllables.iter().flat_map(|s| {
            s.onset
                .iter()
                .copied()
                .chain(s.nucleus.iter().copied())
                .chain(s.coda.iter().copied())
        })
    }
}

fn apply_signatures(
    syllables: &mut Vec<Syllable>,
    aesthetic: &Aesthetic,
    join_at: Option<usize>,
    rng: &mut impl Rng,
) {
    for sig in &aesthetic.signatures {
        match sig {
            Signature::OpenNonfinal => {
                let last = syllables.len().saturating_sub(1);
                for (i, syl) in syllables.iter_mut().enumerate() {
                    if i != last {
                        syl.coda.clear();
                    }
                }
            }
            Signature::PenultimateLength { probability } => {
                if rng.gen::<f32>() < *probability && !syllables.is_empty() {
                    let i = syllables.len().saturating_sub(2);
                    syllables[i].long = true;
                }
            }
            Signature::Reduplicate {
                probability,
                partial,
            } => {
                if join_at.is_none()
                    && syllables.len() <= 2
                    && rng.gen::<f32>() < *probability
                    && !syllables.is_empty()
                {
                    let mut echo = syllables[0].clone();
                    if *partial {
                        echo.coda.clear();
                    }
                    syllables.push(echo);
                }
            }
            Signature::GlottalHiatus => {}
        }
    }
}

fn apply_ending(
    syllables: &mut Vec<Syllable>,
    aesthetic: &Aesthetic,
    inventory: &Inventory,
    rng: &mut impl Rng,
    kind: NameKind,
    max: u8,
) {
    let endings = match kind {
        NameKind::Person => &aesthetic.names.person_endings,
        NameKind::Place => &aesthetic.names.place_endings,
        NameKind::Word => return,
    };
    if endings.is_empty() || rng.gen::<f32>() > 0.55 {
        return;
    }
    let Some(raw) = endings.choose(rng) else {
        return;
    };
    let Some(ids) = CATALOG.parse_ipa(raw) else {
        return;
    };
    if ids.iter().any(|id| !inventory.contains(*id)) {
        return;
    }
    let Some(mut syl) = segs_to_syllable(&ids) else {
        return;
    };
    if syl.nucleus.is_empty() {
        if let Some(last) = syllables.last_mut() {
            last.coda.extend(syl.coda);
        }
        return;
    }
    if syllables.len() >= max as usize {
        if let Some(last) = syllables.last_mut() {
            if syl.onset.is_empty() {
                syl.onset.clone_from(&last.onset);
            }
            *last = syl;
        }
    } else {
        syllables.push(syl);
    }
}

fn segs_to_syllable(ids: &[PhonemeId]) -> Option<Syllable> {
    if ids.is_empty() {
        return None;
    }
    let mut onset = Vec::new();
    let mut nucleus = Vec::new();
    let mut coda = Vec::new();
    let mut seen_vowel = false;
    for &id in ids {
        if CATALOG.get(id).is_vowel() {
            seen_vowel = true;
            nucleus.push(id);
        } else if !seen_vowel {
            onset.push(id);
        } else {
            coda.push(id);
        }
    }
    if nucleus.is_empty() {
        if onset.is_empty() {
            return None;
        }
        // Consonant-only ending attaches as a coda syllable with no nucleus;
        // romanizer still emits the consonants.
        return Some(Syllable {
            onset: Vec::new(),
            nucleus: Vec::new(),
            coda: onset,
            long: false,
        });
    }
    Some(Syllable {
        onset,
        nucleus,
        coda,
        long: false,
    })
}

fn stem(generator: &Generator, n: usize, open_nonfinal: bool, rng: &mut impl Rng) -> Vec<Syllable> {
    let n = n.max(1);
    (0..n)
        .map(|i| generator.syllable(rng, i + 1 == n, open_nonfinal))
        .collect()
}

fn banned_onset(ids: &[PhonemeId], w: f32) -> bool {
    if ids.len() != 1 || w >= 6.5 {
        return false;
    }
    matches!(CATALOG.get(ids[0]).ipa(), "ŋ" | "ɲ" | "ʔ" | "ʎ")
}

fn singles(inventory: &Inventory, consonants: bool) -> Vec<(Vec<PhonemeId>, f32)> {
    let ids = if consonants {
        &inventory.consonants
    } else {
        &inventory.vowels
    };
    ids.iter()
        .map(|&id| (vec![id], inventory.score(id)))
        .collect()
}

fn boost_preferred(
    dest: &mut Vec<(Vec<PhonemeId>, f32)>,
    inventory: &Inventory,
    preferred: &[String],
) {
    for raw in preferred {
        let Some(ids) = CATALOG.parse_ipa(raw) else {
            continue;
        };
        if ids.iter().any(|id| !inventory.contains(*id)) {
            continue;
        }
        if let Some((_, w)) = dest.iter_mut().find(|(have, _)| have == &ids) {
            *w += 6.0;
        } else {
            dest.push((ids, 7.0));
        }
    }
}

fn pick<'a>(rng: &mut impl Rng, items: &'a [(Vec<PhonemeId>, f32)]) -> &'a Vec<PhonemeId> {
    let total: f32 = items.iter().map(|(_, w)| w.max(0.0)).sum();
    if total <= 0.0 {
        return &items[rng.gen_range(0..items.len())].0;
    }
    let mut x = rng.gen::<f32>() * total;
    for (item, w) in items {
        x -= w.max(0.0);
        if x <= 0.0 {
            return item;
        }
    }
    &items[items.len() - 1].0
}

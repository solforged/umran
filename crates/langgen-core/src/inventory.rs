use crate::aesthetic::Aesthetic;
use crate::phoneme::{CATALOG, Catalog, PhonemeId, Segment};
use rand::Rng;

#[derive(Clone, Debug)]
pub struct Inventory {
    pub consonants: Vec<PhonemeId>,
    pub vowels: Vec<PhonemeId>,
    scores: Vec<(PhonemeId, f32)>,
}

impl Inventory {
    pub fn sample(aesthetic: &Aesthetic, rng: &mut impl Rng) -> Self {
        let prior = &aesthetic.inventory;
        let forbidden: Vec<&str> = prior.forbidden.iter().map(String::as_str).collect();

        let mut c_pool: Vec<(PhonemeId, f32)> = CATALOG
            .consonants()
            .filter(|(_, c)| !forbidden.contains(&c.ipa))
            .map(|(id, _)| (id, score_segment(aesthetic, CATALOG.get(id))))
            .filter(|(_, s)| *s > 0.45)
            .collect();
        let mut v_pool: Vec<(PhonemeId, f32)> = CATALOG
            .vowels()
            .filter(|(_, v)| !forbidden.contains(&v.ipa))
            .map(|(id, _)| (id, score_segment(aesthetic, CATALOG.get(id))))
            .filter(|(_, s)| *s > 0.45)
            .collect();

        let mut consonants = take_required(&prior.required, true);
        let mut vowels = take_required(&prior.required, false);
        consonants.retain(|id| !forbidden.contains(&CATALOG.get(*id).ipa()));
        vowels.retain(|id| !forbidden.contains(&CATALOG.get(*id).ipa()));

        c_pool.retain(|(id, _)| !consonants.contains(id));
        v_pool.retain(|(id, _)| !vowels.contains(id));

        let n_c = (rng.gen_range(prior.consonant_count.0..=prior.consonant_count.1) as usize)
            .max(consonants.len());
        let n_v =
            (rng.gen_range(prior.vowel_count.0..=prior.vowel_count.1) as usize).max(vowels.len());
        while consonants.len() < n_c && !c_pool.is_empty() {
            let i = weighted_index(rng, &c_pool);
            consonants.push(c_pool.swap_remove(i).0);
        }
        while vowels.len() < n_v && !v_pool.is_empty() {
            let i = weighted_index(rng, &v_pool);
            vowels.push(v_pool.swap_remove(i).0);
        }

        repair_universals(&mut consonants, &mut vowels);

        let mut scores = Vec::new();
        for &id in consonants.iter().chain(vowels.iter()) {
            scores.push((id, score_segment(aesthetic, CATALOG.get(id)).max(0.08)));
        }

        consonants.sort_by_key(|id| CATALOG.get(*id).ipa());
        vowels.sort_by_key(|id| CATALOG.get(*id).ipa());

        Self {
            consonants,
            vowels,
            scores,
        }
    }

    pub fn from_ids(ids: impl IntoIterator<Item = PhonemeId>) -> Self {
        let mut consonants = Vec::new();
        let mut vowels = Vec::new();
        for id in ids {
            if (id.0 as usize) >= CATALOG.segments.len() {
                continue;
            }
            if consonants.contains(&id) || vowels.contains(&id) {
                continue;
            }
            if CATALOG.get(id).is_vowel() {
                vowels.push(id);
            } else {
                consonants.push(id);
            }
        }
        if vowels.is_empty() {
            for ipa in ["i", "a", "u"] {
                if let Some(id) = CATALOG.id_by_ipa(ipa) {
                    if !vowels.contains(&id) {
                        vowels.push(id);
                    }
                }
            }
        }
        consonants.sort_by_key(|id| CATALOG.get(*id).ipa());
        vowels.sort_by_key(|id| CATALOG.get(*id).ipa());
        let scores = consonants
            .iter()
            .chain(vowels.iter())
            .map(|&id| (id, 1.0))
            .collect();
        Self {
            consonants,
            vowels,
            scores,
        }
    }

    pub fn contains(&self, id: PhonemeId) -> bool {
        self.consonants.contains(&id) || self.vowels.contains(&id)
    }

    pub fn score(&self, id: PhonemeId) -> f32 {
        self.scores
            .iter()
            .find(|(x, _)| *x == id)
            .map(|(_, s)| *s)
            .unwrap_or(0.08)
    }

    pub fn ipas(&self, ids: &[PhonemeId]) -> Vec<&'static str> {
        ids.iter().map(|id| CATALOG.get(*id).ipa()).collect()
    }
}

pub fn score_segment(aesthetic: &Aesthetic, seg: Segment) -> f32 {
    let p = &aesthetic.inventory;
    let mut score = 0.12;
    match seg {
        Segment::Consonant(c) => {
            match p.manner.iter().find(|(m, _)| *m == c.manner) {
                Some((_, w)) => score += *w,
                None => score -= 0.9,
            }
            match p.place.iter().find(|(pl, _)| *pl == c.place) {
                Some((_, w)) => score += *w,
                None => score -= 0.5,
            }
            score += if c.voiced { p.voiced } else { -p.voiced };
        }
        Segment::Vowel(v) => {
            match p.height.iter().find(|(h, _)| *h == v.height) {
                Some((_, w)) => score += *w,
                None => score -= 0.9,
            }
            match p.backness.iter().find(|(b, _)| *b == v.backness) {
                Some((_, w)) => score += *w,
                None => score -= 0.5,
            }
            score += if v.rounded { p.rounded } else { -p.rounded };
        }
    }
    if let Some((_, w)) = p.extra.iter().find(|(ipa, _)| ipa == seg.ipa()) {
        score += *w;
    }
    score
}

fn take_required(required: &[String], consonants: bool) -> Vec<PhonemeId> {
    required
        .iter()
        .filter_map(|ipa| CATALOG.id_by_ipa(ipa))
        .filter(|id| CATALOG.get(*id).is_vowel() != consonants)
        .collect()
}

fn repair_universals(consonants: &mut Vec<PhonemeId>, vowels: &mut Vec<PhonemeId>) {
    let cat: &Catalog = &CATALOG;
    let has = |ipa: &str, set: &[PhonemeId]| set.iter().any(|id| cat.get(*id).ipa() == ipa);
    let add_if = |ipa: &str, set: &mut Vec<PhonemeId>| {
        if let Some(id) = cat.id_by_ipa(ipa) {
            if !set.contains(&id) {
                set.push(id);
            }
        }
    };
    if has("g", consonants) {
        add_if("k", consonants);
    }
    if has("d", consonants) {
        add_if("t", consonants);
    }
    if has("b", consonants) {
        add_if("p", consonants);
    }
    if has("ŋ", consonants) {
        add_if("n", consonants);
    }
    if has("ɲ", consonants) {
        add_if("n", consonants);
    }
    if vowels.is_empty() {
        add_if("a", vowels);
        add_if("i", vowels);
        add_if("u", vowels);
    }
}

pub fn weighted_index(rng: &mut impl Rng, items: &[(PhonemeId, f32)]) -> usize {
    let total: f32 = items.iter().map(|(_, w)| w.max(0.0)).sum();
    if total <= 0.0 {
        return rng.gen_range(0..items.len());
    }
    let mut x = rng.r#gen::<f32>() * total;
    for (i, (_, w)) in items.iter().enumerate() {
        x -= w.max(0.0);
        if x <= 0.0 {
            return i;
        }
    }
    items.len() - 1
}

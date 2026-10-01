//! A small automated comparative method. Given only two word lists, it
//! aligns same-meaning words, finds sound correspondences that recur, and
//! judges which pairs look cognate. It never sees lineage; the simulation's
//! record of descent is used only to grade it.

use crate::adapt::distance;
use crate::concepts::Concept;
use crate::form::Form;
use crate::lexicon::Lexicon;
use crate::phoneme::PhonemeId;
use std::collections::HashMap;

/// Aligned segments; `None` is a gap where one word has nothing.
pub type Pair = (Option<PhonemeId>, Option<PhonemeId>);

/// Thresholds for the comparative method.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// A correspondence must recur at least this often to count as regular...
    pub min_support: u32,
    /// ...and account for this share of either segment's occurrences.
    pub min_share: f32,
    /// Share of a pair's aligned positions that must be regular for the
    /// pair to be judged cognate.
    pub cognate_threshold: f32,
    /// Rounds of re-estimating correspondences from presumed cognates.
    pub rounds: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            min_support: 3,
            min_share: 0.25,
            cognate_threshold: 0.75,
            rounds: 4,
        }
    }
}

const GAP: f32 = 1.5;
/// Consonants and vowels never align with each other.
const MISMATCH: f32 = 99.0;

/// Minimum-cost alignment by feature distance (Needleman–Wunsch).
pub fn align(a: &Form, b: &Form) -> Vec<Pair> {
    align_with_cost(a, b).0
}

/// Phonetic similarity of two words, 0–1: one minus the alignment cost
/// over the cost of aligning nothing at all.
pub fn similarity(a: &Form, b: &Form) -> f32 {
    let worst = GAP * (a.segs.len() + b.segs.len()).max(1) as f32;
    (1.0 - align_with_cost(a, b).1 / worst).clamp(0.0, 1.0)
}

/// Rough mutual intelligibility, 0–1: mean phonetic similarity of the two
/// varieties' words for the same Leipzig–Jakarta concepts, corrected for
/// chance by the similarity of words for different concepts (as in ASJP's
/// LDND). Built only from what speakers hear, never from lineage.
pub fn intelligibility(a: &Lexicon, b: &Lexicon) -> f32 {
    let core: Vec<(&Form, &Form)> = crate::concepts::CONCEPTS
        .iter()
        .filter(|c| c.stability.is_some())
        .filter_map(|c| Some((&a.word_for(c)?.form, &b.word_for(c)?.form)))
        .collect();
    if core.len() < 2 {
        return 0.0;
    }
    let mean = |pairs: &mut dyn Iterator<Item = f32>| {
        let (sum, n) = pairs.fold((0.0, 0), |(s, n), x| (s + x, n + 1));
        sum / n as f32
    };
    let same = mean(&mut core.iter().map(|(x, y)| similarity(x, y)));
    // Mismatch meanings in both directions so the score is symmetric.
    let n = core.len();
    let chance = mean(&mut (0..n).flat_map(|i| {
        let j = (i + 1) % n;
        [
            similarity(core[i].0, core[j].1),
            similarity(core[j].0, core[i].1),
        ]
    }));
    if chance >= 1.0 {
        return 1.0;
    }
    ((same - chance) / (1.0 - chance)).clamp(0.0, 1.0)
}

fn align_with_cost(a: &Form, b: &Form) -> (Vec<Pair>, f32) {
    let (xa, xb): (Vec<PhonemeId>, Vec<PhonemeId>) = (a.phones().collect(), b.phones().collect());
    let (n, m) = (xa.len(), xb.len());
    let sub = |x: PhonemeId, y: PhonemeId| {
        let d = distance(x, y);
        if d.is_finite() {
            0.6 * d.min(5.0)
        } else {
            MISMATCH
        }
    };
    let mut cost = vec![vec![0.0_f32; m + 1]; n + 1];
    for (i, row) in cost.iter_mut().enumerate() {
        row[0] = i as f32 * GAP;
    }
    for (j, cell) in cost[0].iter_mut().enumerate() {
        *cell = j as f32 * GAP;
    }
    for i in 1..=n {
        for j in 1..=m {
            cost[i][j] = (cost[i - 1][j - 1] + sub(xa[i - 1], xb[j - 1]))
                .min(cost[i - 1][j] + GAP)
                .min(cost[i][j - 1] + GAP);
        }
    }
    let mut out = Vec::with_capacity(n.max(m));
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && cost[i][j] == cost[i - 1][j - 1] + sub(xa[i - 1], xb[j - 1]) {
            out.push((Some(xa[i - 1]), Some(xb[j - 1])));
            (i, j) = (i - 1, j - 1);
        } else if i > 0 && cost[i][j] == cost[i - 1][j] + GAP {
            out.push((Some(xa[i - 1]), None));
            i -= 1;
        } else {
            out.push((None, Some(xb[j - 1])));
            j -= 1;
        }
    }
    out.reverse();
    (out, cost[n][m])
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub concept: &'static Concept,
    pub a: Form,
    pub b: Form,
    pub alignment: Vec<Pair>,
    /// Share of aligned positions that are regular correspondences.
    pub regularity: f32,
    pub cognate: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    /// Regular correspondences with their support, most frequent first.
    pub regular: Vec<(Pair, u32)>,
    pub rows: Vec<Row>,
}

/// Correspondences that recur often enough among `alignments` to be regular.
pub fn regular_correspondences<'a>(
    alignments: impl Iterator<Item = &'a [Pair]>,
    settings: &Settings,
) -> Vec<(Pair, u32)> {
    let mut counts: HashMap<Pair, u32> = HashMap::new();
    let mut by_a: HashMap<Option<PhonemeId>, u32> = HashMap::new();
    let mut by_b: HashMap<Option<PhonemeId>, u32> = HashMap::new();
    for alignment in alignments {
        for &pair in alignment {
            *counts.entry(pair).or_default() += 1;
            *by_a.entry(pair.0).or_default() += 1;
            *by_b.entry(pair.1).or_default() += 1;
        }
    }
    let mut regular: Vec<(Pair, u32)> = counts
        .into_iter()
        .filter(|&(pair, n)| {
            let share_a = n as f32 / by_a[&pair.0] as f32;
            let share_b = n as f32 / by_b[&pair.1] as f32;
            n >= settings.min_support && share_a.max(share_b) >= settings.min_share
        })
        .collect();
    regular.sort_by(|x, y| {
        y.1.cmp(&x.1).then_with(|| {
            (x.0.0.map(|p| p.0), x.0.1.map(|p| p.0)).cmp(&(y.0.0.map(|p| p.0), y.0.1.map(|p| p.0)))
        })
    });
    regular
}

/// Compares the dominant words of two lexicons for `concepts`.
pub fn compare(a: &Lexicon, b: &Lexicon, concepts: &[&'static Concept]) -> Comparison {
    compare_with(a, b, concepts, &Settings::default())
}

pub fn compare_with(
    a: &Lexicon,
    b: &Lexicon,
    concepts: &[&'static Concept],
    settings: &Settings,
) -> Comparison {
    let mut rows: Vec<Row> = concepts
        .iter()
        .filter_map(|&concept| {
            let wa = a.word_for(concept)?.form.clone();
            let wb = b.word_for(concept)?.form.clone();
            let alignment = align(&wa, &wb);
            Some(Row {
                concept,
                a: wa,
                b: wb,
                alignment,
                regularity: 0.0,
                cognate: true,
            })
        })
        .collect();
    let mut regular = Vec::new();
    for _ in 0..settings.rounds {
        regular = regular_correspondences(
            rows.iter()
                .filter(|r| r.cognate)
                .map(|r| r.alignment.as_slice()),
            settings,
        );
        let set: HashMap<Pair, u32> = regular.iter().copied().collect();
        for row in &mut rows {
            let hits = row.alignment.iter().filter(|p| set.contains_key(p)).count();
            row.regularity = hits as f32 / row.alignment.len().max(1) as f32;
            row.cognate = row.regularity >= settings.cognate_threshold;
        }
    }
    Comparison { regular, rows }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phoneme::CATALOG;

    fn show(alignment: &[Pair]) -> String {
        alignment
            .iter()
            .map(|(a, b)| {
                let s = |x: &Option<PhonemeId>| x.map_or("-", |p| CATALOG.get(p).ipa());
                format!("{}{}", s(a), s(b))
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn similarity_runs_from_identical_to_unrelated() {
        let f = |s: &str| Form::from_ipa(s).unwrap();
        assert_eq!(similarity(&f("pata"), &f("pata")), 1.0);
        let near = similarity(&f("pata"), &f("bata"));
        let far = similarity(&f("pata"), &f("lomu"));
        assert!(near > 0.8 && far < near, "{near} {far}");
    }

    #[test]
    fn aligns_by_features_with_gaps() {
        let f = |s: &str| Form::from_ipa(s).unwrap();
        assert_eq!(show(&align(&f("pata"), &f("fata"))), "pf aa tt aa");
        assert_eq!(show(&align(&f("kata"), &f("kat"))), "kk aa tt a-");
        assert_eq!(show(&align(&f("sta"), &f("sita"))), "ss -i tt aa");
    }
}

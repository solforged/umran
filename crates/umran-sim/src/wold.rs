//! Reference figures from the World Loanword Database for validation only;
//! the simulation never reads them.
//!
//! Haspelmath, Martin and Uri Tadmor (eds.) 2009. World Loanword Database.
//! <https://wold.clld.org/semanticfield>, retrieved 2026-09-30.

use crate::concepts::Field;

/// Average borrowed score per semantic field across the 41 WOLD languages:
/// 1.0 means every word was clearly borrowed, 0.0 that none show evidence of
/// it. WOLD's "Modern world" field has no counterpart here. Its
/// "Miscellaneous function words" (0.44) is high because it includes many
/// conjunctions and discourse words, unlike this crate's pronouns and
/// deictics, so it is left out of comparisons.
pub const BORROWED_SCORE: &[(Field, f32)] = &[
    (Field::Social, 0.64),
    (Field::Religion, 0.49),
    (Field::Agriculture, 0.45),
    (Field::ClothingGrooming, 0.40),
    (Field::House, 0.40),
    (Field::FoodDrink, 0.37),
    (Field::Law, 0.36),
    (Field::Speech, 0.36),
    (Field::Possession, 0.34),
    (Field::Warfare, 0.34),
    (Field::BasicActions, 0.33),
    (Field::Quantity, 0.33),
    (Field::Animals, 0.32),
    (Field::Emotions, 0.30),
    (Field::Cognition, 0.29),
    (Field::Time, 0.28),
    (Field::Kinship, 0.23),
    (Field::Motion, 0.21),
    (Field::PhysicalWorld, 0.21),
    (Field::SensePerception, 0.19),
    (Field::Body, 0.17),
    (Field::Spatial, 0.15),
];

/// Spearman rank correlation between paired values, with average ranks for
/// ties.
pub fn spearman(pairs: &[(f32, f32)]) -> f32 {
    fn ranks(values: &[f32]) -> Vec<f32> {
        let mut order: Vec<usize> = (0..values.len()).collect();
        order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
        let mut out = vec![0.0; values.len()];
        let mut i = 0;
        while i < order.len() {
            let mut j = i;
            while j + 1 < order.len() && values[order[j + 1]] == values[order[i]] {
                j += 1;
            }
            let rank = (i + j) as f32 / 2.0 + 1.0;
            for k in i..=j {
                out[order[k]] = rank;
            }
            i = j + 1;
        }
        out
    }
    let (xs, ys): (Vec<f32>, Vec<f32>) = pairs.iter().copied().unzip();
    let (rx, ry) = (ranks(&xs), ranks(&ys));
    let n = pairs.len() as f32;
    let mean = (n + 1.0) / 2.0;
    let cov: f32 = rx
        .iter()
        .zip(&ry)
        .map(|(a, b)| (a - mean) * (b - mean))
        .sum();
    let var = |r: &[f32]| {
        r.iter()
            .map(|a| {
                let delta = a - mean;
                delta * delta
            })
            .sum::<f32>()
    };
    cov / (var(&rx) * var(&ry)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spearman_matches_known_cases() {
        assert!((spearman(&[(1.0, 2.0), (2.0, 4.0), (3.0, 9.0)]) - 1.0).abs() < 1e-6);
        assert!((spearman(&[(1.0, 3.0), (2.0, 2.0), (3.0, 1.0)]) + 1.0).abs() < 1e-6);
        let tied = spearman(&[(1.0, 1.0), (2.0, 1.0), (3.0, 2.0)]);
        assert!((tied - 0.866).abs() < 1e-3, "{tied}");
    }
}

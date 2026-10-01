use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// An independent random stream for one purpose, such as one concept's root.
///
/// Each stream is keyed by the seed plus labels, so adding a new consumer
/// never shifts the draws of an existing one. ChaCha8 is named explicitly
/// because `StdRng` may change algorithm between `rand` releases.
pub fn stream(seed: u64, keys: &[u64]) -> ChaCha8Rng {
    let mut state = splitmix(seed);
    for &k in keys {
        state = splitmix(state ^ k);
    }
    ChaCha8Rng::seed_from_u64(state)
}

/// Stable 64-bit FNV-1a hash of a label. `DefaultHasher` is not stable
/// across Rust releases, so it cannot key reproducible streams.
pub fn key(label: &str) -> u64 {
    label.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn splitmix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A uniform index below `len`. Drawn as a `u32` because sampling a
/// `usize` range draws 64 bits natively but 32 in WASM, which would make
/// the same seed tell a different history in the browser.
pub fn index(rng: &mut impl rand::Rng, len: usize) -> usize {
    let len = u32::try_from(len).expect("fewer than 2^32 choices");
    rng.gen_range(0..len) as usize
}

/// Index into `weights` chosen proportionally; non-positive weights never win
/// unless every weight is non-positive.
pub fn weighted_index(
    rng: &mut impl rand::Rng,
    weights: impl Iterator<Item = f32> + Clone,
) -> usize {
    let total: f32 = weights.clone().map(|w| w.max(0.0)).sum();
    let count = weights.clone().count();
    if total <= 0.0 {
        return index(rng, count);
    }
    let mut x = rng.r#gen::<f32>() * total;
    for (i, w) in weights.enumerate() {
        x -= w.max(0.0);
        if x <= 0.0 {
            return i;
        }
    }
    count - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn streams_are_reproducible_and_independent() {
        let a: u64 = stream(42, &[key("root"), key("fire")]).r#gen();
        let b: u64 = stream(42, &[key("root"), key("fire")]).r#gen();
        let c: u64 = stream(42, &[key("root"), key("nose")]).r#gen();
        let d: u64 = stream(43, &[key("root"), key("fire")]).r#gen();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn key_is_fnv1a() {
        assert_eq!(key(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(key("a"), 0xaf63_dc4c_8601_ec8c);
    }
}

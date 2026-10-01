//! How long words are over time: syllables per word in each language's
//! dominant vocabulary, how many are homophones, and how many were
//! renewed, for tuning erosion and renewal.
//!
//! cargo run --release -p langgen-sim --example length -- [seeds] [generations]

use langgen_sim::{Origin, Params, SoundProfile, World};

/// Generations at which to measure.
const MARKS: [u32; 6] = [0, 20, 40, 80, 160, 255];

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u64 = args.next().map_or(40, |s| s.parse().expect("seeds"));
    let last: u32 = args.next().map_or(255, |s| s.parse().expect("generations"));
    println!("preset              gen  syll  core  1-syll  4+syll  homophones  renewed");
    for profile in SoundProfile::presets() {
        let mut rows = vec![[0.0f32; 6]; MARKS.len()];
        for seed in 0..seeds {
            let mut world = World::solo(seed, &profile, Params::default());
            for (i, &mark) in MARKS.iter().enumerate().filter(|(_, m)| **m <= last) {
                world.run(mark - world.generation);
                let lexicon = &world.varieties[0].lexicon;
                let words: Vec<_> = lexicon
                    .slots
                    .iter()
                    .filter_map(|s| {
                        s.dominant()
                            .map(|id| (s.concept, &lexicon.get(id).form, id))
                    })
                    .collect();
                let syll = |f: &langgen_sim::Form| f.vowel_count() as f32;
                let renewed = words
                    .iter()
                    .filter(|(_, _, id)| matches!(lexicon.get(*id).origin, Origin::Renewed { .. }))
                    .count();
                let n = words.len() as f32;
                let core: Vec<_> = words
                    .iter()
                    .filter(|(c, ..)| c.stability.is_some())
                    .collect();
                // Homophones are distinct words that sound alike; one word
                // serving two concepts is polysemy and does not count.
                let mut ids: Vec<_> = words.iter().map(|(_, f, id)| (f.ipa(), *id)).collect();
                ids.sort();
                ids.dedup();
                let homophones = ids
                    .iter()
                    .filter(|(f, id)| ids.iter().any(|(g, other)| g == f && other != id))
                    .count();
                let row = &mut rows[i];
                row[0] += words.iter().map(|(_, f, _)| syll(f)).sum::<f32>() / n;
                row[1] += core.iter().map(|(_, f, _)| syll(f)).sum::<f32>() / core.len() as f32;
                row[2] += words
                    .iter()
                    .filter(|(_, f, _)| f.vowel_count() <= 1)
                    .count() as f32
                    / n;
                row[3] += words
                    .iter()
                    .filter(|(_, f, _)| f.vowel_count() >= 4)
                    .count() as f32
                    / n;
                row[4] += homophones as f32 / ids.len() as f32;
                row[5] += renewed as f32 / n;
            }
        }
        for (i, &mark) in MARKS.iter().enumerate().filter(|(_, m)| **m <= last) {
            let r = rows[i].map(|v| v / seeds as f32);
            println!(
                "{:<18} {mark:>4}  {:>4.2}  {:>4.2}  {:>5.0}%  {:>5.0}%  {:>9.0}%  {:>6.0}%",
                profile.id,
                r[0],
                r[1],
                r[2] * 100.0,
                r[3] * 100.0,
                r[4] * 100.0,
                r[5] * 100.0
            );
        }
    }
}

//! Grammatical contrast and synthesis by founding stress rule.
//! cargo run --release -p umran-sim --example endings -- [seeds]
use umran_sim::{Params, SoundProfile, StressRule, World};
fn main() {
    let seeds: u64 = std::env::args()
        .nth(1)
        .map_or(30, |s| s.parse().expect("seeds"));
    assert!(seeds > 0);
    println!("{seeds} seeds per founding rule; static societies, ordinary grammar and sound rates");
    println!("Stress rules may change through the ordinary sound laws.");
    println!("A category is audible when more than half its uses distinguish the marked form.");
    println!(
        "founding_stress,year,audible_plural,audible_past,how_synthetic,plural_retention,past_retention"
    );
    for stress in [
        StressRule::Initial,
        StressRule::Penult,
        StressRule::Final,
        StressRule::Weight,
        StressRule::Free,
    ] {
        let mut sums = [[0.0_f32; 5]; 4];
        for seed in 0..seeds {
            let mut profile = SoundProfile::base();
            profile.stress = Some(stress);
            let mut world = World::solo(seed, &profile, Params::static_society());
            for (checkpoint, row) in sums.iter_mut().enumerate() {
                if checkpoint > 0 {
                    world.run(80);
                }
                let summary = &world.varieties[0].grammar.summary;
                row[0] += f32::from(summary.categories[0].contrast_retention > 0.5);
                row[1] += f32::from(summary.categories[1].contrast_retention > 0.5);
                row[2] += summary.how_synthetic;
                row[3] += summary.categories[0].contrast_retention;
                row[4] += summary.categories[1].contrast_retention;
            }
        }
        for (checkpoint, row) in sums.iter().enumerate() {
            println!(
                "{},{},{:.3},{:.3},{:.3},{:.3},{:.3}",
                stress.id(),
                checkpoint * 2000,
                row[0] / seeds as f32,
                row[1] / seeds as f32,
                row[2] / seeds as f32,
                row[3] / seeds as f32,
                row[4] / seeds as f32
            );
        }
    }
}

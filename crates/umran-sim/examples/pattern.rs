//! cargo run --release -p umran-sim --example pattern -- [seeds] [generations]
use umran_sim::grammar::{Category, MarkerKind, NoticeKind};
use umran_sim::{Params, SoundProfile, World};

#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

fn share(world: &World) -> f32 {
    let grammar = &world.varieties[0].grammar;
    grammar
        .markers
        .iter()
        .filter(|m| m.kind == MarkerKind::Pattern)
        .map(|m| grammar.marker_share(m.id))
        .sum::<f32>()
        / 2.0
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let seeds: u64 = args.first().map_or(8, |n| n.parse().expect("seeds"));
    let generations: u32 = args.get(1).map_or(160, |n| n.parse().expect("generations"));
    assert!(seeds > 0);
    println!(
        "Root-and-pattern inflection: {seeds} seeds x {} years",
        generations * 25
    );
    println!(
        "A pattern interleaves three root consonants with vowels; an affix attaches an ending."
    );
    println!("Static societies, ordinary language change, pattern levelling enabled.");
    for profile in ["semitic", "germanic", "finnic"] {
        let mut initial = 0.0;
        let mut final_share = 0.0;
        let mut analogy = 0;
        let mut new_patterns = 0;
        for seed in 0..seeds {
            let params = Params {
                pattern_analogy_rate: Params::default().pattern_analogy_rate,
                ..Params::static_society()
            };
            let mut world = World::solo(seed, &SoundProfile::by_id(profile).unwrap(), params);
            initial += share(&world);
            if seed == 0 && profile == "semitic" {
                for marker in &world.varieties[0].grammar.markers {
                    if let Some(t) = &marker.template {
                        println!(
                            "  seed 0 {}: {} ({:.1}% of category uses)",
                            marker.category.id(),
                            t.notation(),
                            world.varieties[0].grammar.marker_share(marker.id) * 100.0
                        );
                    }
                }
            }
            world.run(generations);
            final_share += share(&world);
            let variety = &world.varieties[0];
            analogy += variety
                .grammar
                .events
                .iter()
                .filter(|e| matches!(e.event, NoticeKind::Analogy { .. }))
                .count();
            new_patterns += variety
                .lexicon
                .living()
                .filter(|w| w.born > 0)
                .flat_map(|w| &w.paradigms)
                .flat_map(|p| &p.realizations)
                .filter(|r| {
                    r.retired.is_none()
                        && variety.grammar.marker(r.marker).kind == MarkerKind::Pattern
                })
                .count();
            if seed == 0 && profile == "semitic" {
                for word in variety.lexicon.living().take(80) {
                    if let Some(r) = word
                        .paradigms
                        .iter()
                        .filter(|p| p.category == Category::Plural)
                        .flat_map(|p| &p.realizations)
                        .find(|r| {
                            r.retired.is_none()
                                && variety.grammar.marker(r.marker).kind == MarkerKind::Pattern
                        })
                    {
                        println!(
                            "  year {}: {} {} > plural {}",
                            generations * 25,
                            word.first_sense.id,
                            word.form.ipa(),
                            r.form.as_ref().unwrap().ipa()
                        );
                    }
                }
            }
        }
        println!(
            "{profile}: mean plural/past pattern share {:.3} -> {:.3}; {new_patterns} new-word pattern forms; {analogy} analogy events",
            initial / seeds as f32,
            final_share / seeds as f32
        );
    }
    let sample = sample::sample().world_at(160);
    let markers = sample
        .varieties
        .iter()
        .flat_map(|v| &v.grammar.markers)
        .filter(|m| m.kind == MarkerKind::Pattern)
        .count();
    println!("Workbench sample: 4000 years, {markers} pattern markers across inherited varieties");
}

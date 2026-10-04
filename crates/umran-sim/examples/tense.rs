//! Future and progressive marking across seeds, profiles, and the workbench sample.
//! cargo run --release -p umran-sim --example tense -- [seeds] [years]
use umran_sim::concepts::by_id;
use umran_sim::grammar::{MarkerKind, MarkerOrigin, NoticeKind};
use umran_sim::{Category, Params, SoundProfile, World};

#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u64 = args.next().map_or(40, |s| s.parse().expect("seeds"));
    let years: u32 = args.next().map_or(4000, |s| s.parse().expect("years"));
    assert!(seeds > 0 && years > 0 && years.is_multiple_of(25));
    println!(
        "{seeds} seeds × {years} years per profile; static societies with tense/aspect enabled"
    );
    println!(
        "Future: later event; progressive: event in progress. A particle is a separate auxiliary; fusion attaches it to the verb."
    );
    println!(
        "profile,category,founding_unmarked,new_particles,fusions,mean_retention,mean_synthesis"
    );
    for preset in ["germanic", "semitic", "polynesian"] {
        let mut totals = [[0.0_f32; 5]; 2];
        for seed in 0..seeds {
            let mut world = World::solo(
                seed,
                &SoundProfile::by_id(preset).unwrap(),
                Params {
                    tense_aspect: true,
                    ..Params::static_society()
                },
            );
            for (i, category) in [Category::Future, Category::Progressive]
                .into_iter()
                .enumerate()
            {
                totals[i][0] += f32::from(
                    world.varieties[0]
                        .grammar
                        .markers
                        .iter()
                        .any(|m| m.category == category && m.kind == MarkerKind::None),
                );
            }
            world.run(years / 25);
            let grammar = &world.varieties[0].grammar;
            for (i, category) in [Category::Future, Category::Progressive]
                .into_iter()
                .enumerate()
            {
                totals[i][1] += grammar
                    .events
                    .iter()
                    .filter(|e| {
                        e.category == category && matches!(e.event, NoticeKind::NewMarker { .. })
                    })
                    .count() as f32;
                totals[i][2] += grammar
                    .events
                    .iter()
                    .filter(|e| {
                        e.category == category && matches!(e.event, NoticeKind::Fusion { .. })
                    })
                    .count() as f32;
                let summary = grammar.summary.categories[category.position()];
                totals[i][3] += summary.contrast_retention;
                totals[i][4] += summary.how_synthetic;
            }
            if seed == 0 {
                show(&world, &format!("{preset} seed {seed}, year {years}"));
            }
        }
        for (category, row) in [Category::Future, Category::Progressive]
            .into_iter()
            .zip(totals)
        {
            println!(
                "{preset},{},{:.0},{:.0},{:.0},{:.3},{:.3}",
                category.id(),
                row[0],
                row[1],
                row[2],
                row[3] / seeds as f32,
                row[4] / seeds as f32
            );
        }
    }
    let sample = sample::sample();
    show(sample.latest(), "exact workbench sample, year 4000");
}

fn show(world: &World, label: &str) {
    println!("\n{label}");
    let spoken = world.spoken();
    for (index, variety) in world
        .varieties
        .iter()
        .enumerate()
        .filter(|(i, _)| spoken[*i])
    {
        let Some(word) = variety.lexicon.word_for(by_id("see").unwrap()) else {
            continue;
        };
        for category in [Category::Future, Category::Progressive] {
            let summary = variety.grammar.summary.categories[category.position()];
            println!(
                "  language {index}, {}: retention {:.3}, synthesis {:.3}",
                category.id(),
                summary.contrast_retention,
                summary.how_synthetic
            );
            for marker in variety
                .grammar
                .markers
                .iter()
                .filter(|m| m.category == category)
            {
                let origin = match marker.origin {
                    MarkerOrigin::Founding => "founding".into(),
                    MarkerOrigin::Grammaticalized { concept, .. } => format!("from {}", concept.id),
                    MarkerOrigin::Fused { particle } => format!("fused from #{particle}"),
                    MarkerOrigin::Imported { from, .. } => format!("imported from language {from}"),
                };
                println!(
                    "    #{} {} /{}/ {origin}; born {}, share {:.3}, retired {:?}",
                    marker.id,
                    marker.kind.id(),
                    marker.form.ipa(),
                    marker.born * 25,
                    variety.grammar.marker_share(marker.id),
                    marker.retired.map(|g| g * 25)
                );
            }
            if let Some(paradigm) = word.paradigms.iter().find(|p| p.category == category) {
                for realization in paradigm.realizations.iter().filter(|r| r.retired.is_none()) {
                    let surface = variety
                        .grammar
                        .surface(&word.form, realization)
                        .iter()
                        .map(|form| form.ipa_stressed(variety.stress()))
                        .collect::<Vec<_>>()
                        .join(" ");
                    println!(
                        "    see: /{surface}/, marker #{}, share {:.3}",
                        realization.marker, realization.share
                    );
                }
            }
        }
    }
}

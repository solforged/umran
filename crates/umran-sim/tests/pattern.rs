use umran_sim::grammar::{Grammar, GrammarDesign, GrammarPrior, MarkerKind, NoticeKind};
use umran_sim::{Params, SoundProfile, World};

#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

#[test]
#[ignore = "40 seeds x 4000 years and the workbench sample; run in release"]
fn productive_patterns_survive_without_taking_over_every_word() {
    let started = std::time::Instant::now();
    let mut initial = 0.0;
    let mut final_share = 0.0;
    let mut levelled = 0;
    for seed in 0..40 {
        let mut profile = SoundProfile::by_id("semitic").unwrap();
        profile.grammar = GrammarPrior::fixed(GrammarDesign::default());
        let params = Params {
            pattern_analogy_rate: Params::default().pattern_analogy_rate,
            ..Params::static_society()
        };
        let mut world = World::solo(seed, &profile, params);
        let share = |g: &Grammar| {
            g.markers
                .iter()
                .filter(|m| m.kind == MarkerKind::Pattern)
                .map(|m| g.marker_share(m.id))
                .sum::<f32>()
                / 2.0
        };
        initial += share(&world.varieties[0].grammar);
        world.run(160);
        let grammar = &world.varieties[0].grammar;
        final_share += share(grammar);
        levelled += grammar
            .events
            .iter()
            .filter(|e| matches!(e.event, NoticeKind::Analogy { .. }))
            .count();
        for word in world.varieties[0].lexicon.living() {
            for p in &word.paradigms {
                let active: Vec<_> = p
                    .realizations
                    .iter()
                    .filter(|r| r.retired.is_none())
                    .collect();
                assert!(active.len() <= 3);
                // A word that stops using a category keeps the paradigm's
                // history with every realization retired.
                if active.is_empty() {
                    continue;
                }
                assert!((active.iter().map(|r| r.share).sum::<f32>() - 1.0).abs() < 1e-5);
                for r in active {
                    if let Some(f) = &r.form {
                        assert!(f.vowel_count() > 0);
                    }
                }
            }
        }
    }
    initial /= 40.0;
    final_share /= 40.0;
    assert!((0.15..0.8).contains(&initial), "initial={initial}");
    assert!((0.03..0.75).contains(&final_share), "final={final_share}");
    assert!((50..1500).contains(&levelled), "analogy={levelled}");
    let sample = sample::sample().world_at(160);
    let sample_patterns: usize = sample
        .varieties
        .iter()
        .map(|v| {
            v.grammar
                .markers
                .iter()
                .filter(|m| m.kind == MarkerKind::Pattern)
                .count()
        })
        .sum();
    assert!(
        sample_patterns > 0,
        "the sample must exercise internal inflection"
    );
    println!(
        "Pattern band: 40 seeds x 4000 years; mean share {initial:.3} -> {final_share:.3}; {levelled} analogy events; sample {sample_patterns} inherited pattern markers; {:.1}s",
        started.elapsed().as_secs_f32()
    );
}

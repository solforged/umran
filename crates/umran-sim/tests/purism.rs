#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
#[path = "../examples/support/purism.rs"]
mod support;

#[test]
#[ignore = "40 seeds x 4,000 years plus the full sample recipe; run explicitly"]
fn purism_across_profiles_and_sample_recipe() {
    let started = std::time::Instant::now();
    let mut summary = support::Summary::default();
    for seed in 0..40 {
        let world = support::study(seed);
        assert_eq!(world.generation, 160);
        summary.add(&world);
        for variety in &world.varieties {
            for episode in &variety.purism {
                // Each eligible court gets only one recorded revival. Neither
                // sound change nor continuing contact may fabricate new causes.
                assert_eq!(
                    episode.cause.mechanism,
                    umran_sim::Mechanism::ReligiousRevival
                );
                assert!((41..=64).contains(&episode.since));
                for word in &episode.replaced {
                    let sense = umran_sim::concepts::by_id(word.concept).unwrap();
                    assert_eq!(variety.lexicon.slot(sense).dominant(), Some(word.native));
                    assert!(!matches!(
                        variety.lexicon.get(word.native).origin,
                        umran_sim::Origin::Borrowed { .. }
                    ));
                }
            }
        }
    }
    assert!((16..=40).contains(&summary.worlds), "{summary:?}");
    assert_eq!(
        summary.episodes, summary.worlds,
        "one revival, at most one reform"
    );
    assert!((40..=1800).contains(&summary.replacements), "{summary:?}");
    assert!(
        summary.revived > summary.coined,
        "attested native words should usually be available: {summary:?}"
    );
    let sample = sample::sample();
    assert_eq!(sample.latest().generation, 160);
    let mut sample_summary = support::Summary::default();
    sample_summary.add(sample.latest());
    // The sample is an ordinary world, not an authored revival fixture. Its
    // recorded causal opportunities bound reforms instead of forcing one.
    let opportunities = sample
        .latest()
        .events
        .iter()
        .filter(|(_, event)| {
            matches!(
                event,
                umran_sim::WorldEvent::Rose { .. } | umran_sim::WorldEvent::Schism { .. }
            )
        })
        .count();
    assert!(sample_summary.episodes <= opportunities);
    println!(
        "PURISM BAND: 40 seeds x 4,000 years; {summary:?}; sample {sample_summary:?}; {:.2}s",
        started.elapsed().as_secs_f32()
    );
}

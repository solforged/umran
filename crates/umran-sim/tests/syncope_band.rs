#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
#[path = "../examples/support/syncope.rs"]
mod syncope;

#[test]
#[ignore = "40 seeds × 4000 years across five profiles, plus the workbench sample"]
fn rhythmic_syncope_cluster_band() {
    let started = std::time::Instant::now();
    let reports = syncope::cohort(40);
    let mut large = 0;
    for (profile, report) in &reports {
        let words: usize = report.words.values().sum();
        assert!((4000..20000).contains(&words), "{profile}: {report:?}");
        assert_eq!(report.languages.values().sum::<usize>(), 40);
        // Other laws and compounding can still produce long clusters. The
        // historical band is not a universal prohibition on those forms.
        assert!(report.large_words() < words / 100, "{profile}: {report:?}");
        large += report.large_words();
    }
    println!("all profiles: clusters>=5={large}");
    assert!(
        large < 5,
        "large clusters should remain exceptional: {large}"
    );
    let history = sample::sample();
    let world = history.latest();
    let mut report = syncope::Distribution::default();
    for (variety, spoken) in world.varieties.iter().zip(world.spoken()) {
        if spoken {
            report.add(variety);
        }
    }
    report.print("workbench sample (living varieties, not independent languages)");
    let words: usize = report.words.values().sum();
    assert!(words > 0 && report.large_words() < words / 100);
    println!("elapsed={:.1}s", started.elapsed().as_secs_f32());
}

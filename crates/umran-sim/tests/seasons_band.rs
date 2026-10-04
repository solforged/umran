#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
#[path = "../examples/support/seasons_band.rs"]
mod support;

#[test]
#[ignore = "40 seeds × 4000 years, three founding profiles, plus the complete workbench sample"]
fn seasonal_weather_band() {
    let started = std::time::Instant::now();
    let mut report = support::Report::default();
    for seed in 0..40 {
        report.add(&support::history(seed));
    }
    report.print("40 seeds × 4000 years (one representative year per generation)");
    assert!(
        (0.015..0.12).contains(&report.rate(0)),
        "drought risk outside band"
    );
    assert!(
        (0.003..0.09).contains(&report.rate(1)),
        "winter risk outside band"
    );
    assert!(
        (0.0001..0.04).contains(&report.rate(2)),
        "flood risk outside band"
    );
    assert!(report.quiet_land_years > report.samples * 4 / 5);
    assert!(report.monsoon > 0 && report.monsoon < report.lands);
    assert!(report.floodplains > 0 && report.floodplains < report.lands);
    assert!(report.migrations > 0 && report.adoptions > 0);
    let sample = sample::sample();
    let mut report = support::Report::default();
    report.add(sample.latest());
    report.print("workbench sample, year 4000");
    assert_eq!(sample.latest().generation, 160);
    assert!(report.counts.iter().all(|&n| n > 0));
    assert!(report.quiet_land_years > report.samples * 4 / 5);
    println!(
        "seasonal band elapsed={:.1}s",
        started.elapsed().as_secs_f32()
    );
}

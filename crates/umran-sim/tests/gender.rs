#[path = "../examples/gender.rs"]
mod report;

#[test]
#[ignore = "40 seeds per profile × five independent founder cohorts × 4000 years, plus sample"]
fn gender_sizes_and_clusivity_approach_wals() {
    let start = std::time::Instant::now();
    let founding = report::founding_report();
    assert!(founding.iter().all(|&n| n > 0));
    let counts = report::study(40);
    assert_eq!(counts.families, 200);
    for (i, target) in report::WALS_GENDER.into_iter().enumerate() {
        let share = counts.final_sizes[i] as f64 / counts.families as f64;
        assert!(
            (share - target).abs() < 0.10,
            "gender bucket {i}: {share:.3} vs {target:.3}"
        );
        assert!(
            counts.final_sizes[i] > 0,
            "every class size must remain represented"
        );
    }
    let clusivity = counts.clusivity as f64 / counts.families as f64;
    assert!(
        (0.22..0.42).contains(&clusivity),
        "WALS 39A presence band: {clusivity}"
    );
    assert!(
        counts.distinct_clusivity * 10 >= counts.clusivity * 8,
        "the modeled opposition must usually remain overt"
    );
    report::sample_report();
    println!(
        "gender/clusivity bands passed; elapsed {:.2}s",
        start.elapsed().as_secs_f64()
    );
}

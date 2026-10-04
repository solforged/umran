#[path = "../examples/pronouns.rs"]
mod report;

#[test]
#[ignore = "40 seeds × five profiles × 4,000 years, plus the workbench sample"]
fn pronoun_stability_and_genitive_bands() {
    let start = std::time::Instant::now();
    let mut cells = 0;
    let mut renewed = 0;
    for profile in report::PROFILES {
        let counts = report::study(profile, 40);
        let share = counts.renewed as f32 / counts.cells as f32;
        println!(
            "{profile}: {} cells, renewal={share:.3}, loans={}, genitive={:.3}",
            counts.cells,
            counts.borrowed,
            counts.genitive / 40.0
        );
        assert!(
            (0.02..0.65).contains(&share),
            "pronouns should renew but remain relatively stable: {profile} {share}"
        );
        assert_eq!(counts.borrowed, 0, "isolated pronouns cannot be loans");
        assert!(
            (0.05..0.98).contains(&(counts.genitive / 40.0)),
            "neither universal nor absent genitive contrast"
        );
        cells += counts.cells;
        renewed += counts.renewed;
    }
    let total = renewed as f32 / cells as f32;
    assert!(
        (0.1..0.5).contains(&total),
        "four-millennium renewal band: {total}"
    );
    let sample = report::sample_report();
    assert!(sample.cells >= 6);
    assert!(
        sample.borrowed * 10 <= sample.cells,
        "loans must stay exceptional"
    );
    println!(
        "aggregate renewal={total:.3}; elapsed {:.2}s",
        start.elapsed().as_secs_f32()
    );
}

#[test]
#[ignore = "40 seeds × five court profiles × 4,000 years, plus the workbench sample"]
fn polite_address_coexistence_and_generalisation_bands() {
    let start = std::time::Instant::now();
    let (mut emerged, mut coexisting, mut generalised) = (0, 0, 0);
    for profile in report::PROFILES {
        let counts = report::court_study(profile, 40);
        println!(
            "{profile}: courts=40 emerged={} coexisting={} generalised={}",
            counts.emerged, counts.polite, counts.generalised
        );
        assert!((25..=40).contains(&counts.emerged));
        assert!(counts.polite >= 20, "familiar address normally survives");
        assert_eq!(counts.emerged, counts.polite + counts.generalised);
        emerged += counts.emerged;
        coexisting += counts.polite;
        generalised += counts.generalised;
    }
    let rate = generalised as f32 / emerged as f32;
    assert!((0.02..0.30).contains(&rate), "later generalisation={rate}");
    let sample = report::sample_report();
    assert_eq!(sample.emerged, sample.polite + sample.generalised);
    println!(
        "address: courts=200 emerged={emerged} coexisting={coexisting} generalised={generalised} rate={rate:.3}; elapsed {:.2}s",
        start.elapsed().as_secs_f32()
    );
}

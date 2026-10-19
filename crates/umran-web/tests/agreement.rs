#[path = "../examples/agreement.rs"]
mod report;

#[test]
#[ignore = "40 seeds per profile × five independent founder cohorts × 4000 years, plus sample"]
fn agreement_stages_and_pro_drop_hold_their_bands() {
    let start = std::time::Instant::now();
    let defaults = umran_sim::Params::default();
    let params = report::params(
        defaults.agreement_rate,
        defaults.agreement_fusion_rate,
        defaults.pro_drop_rate,
    );
    let counts = report::study(40, &params);
    assert_eq!(counts.languages, 200);
    let marked = counts.marked() as f64 / counts.languages as f64;
    assert!(
        (0.35..=0.65).contains(&marked),
        "clitics or affixes at year 4000: {marked:.3}"
    );
    let pro_drop = counts.pro_drop as f64 / counts.languages as f64;
    assert!(
        (0.10..=0.40).contains(&pro_drop),
        "pro-drop at year 4000: {pro_drop:.3}"
    );
    assert!(counts.losses >= 1, "at least one loss of the contrast");
    assert!(counts.clitic >= 1 && counts.affix >= 1);
    let sample = report::sample_report();
    assert!(
        sample >= 1,
        "the workbench sample must end with a language marking agreement"
    );
    println!(
        "agreement bands passed; elapsed {:.2}s",
        start.elapsed().as_secs_f64()
    );
}

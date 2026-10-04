#[path = "../examples/support/tone_band.rs"]
mod band;

#[test]
#[ignore = "40 default worlds and the full sample, 4000 years each; run explicitly in release"]
fn forty_seed_four_thousand_year_tone_band() {
    let stats = band::run(40);
    // Tone should emerge in some, not all histories. Neither coda loss nor
    // onset devoicing is a compulsory stage, and tone can subsequently die.
    // About two fifths of the world's languages are tonal (WALS 13A); at
    // revision 41, 22 of 40 founding languages are.
    assert!((8..=39).contains(&stats.gained), "{stats:?}");
    assert!((8..=30).contains(&stats.tonal), "{stats:?}");
    assert!((1..=30).contains(&stats.lost), "{stats:?}");
    assert!((1..=39).contains(&stats.coda), "{stats:?}");
    assert!((1..=39).contains(&stats.register), "{stats:?}");
    assert!(stats.sample_gains > 0, "{stats:?}");
}

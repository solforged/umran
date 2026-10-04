use umran_sim::harmony::Harmony;
use umran_sim::{Params, SoundProfile, World};

#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

#[test]
#[ignore = "40 seeds over 4,000 years plus the full sample recipe"]
fn harmony_statistical_band() {
    let start = std::time::Instant::now();
    let mut gained = 0;
    let mut lost = 0;
    let mut active = 0;
    let mut varieties = 0;
    for seed in 0..40 {
        let profile = ["finnic", "germanic", "semitic", "polynesian"][seed as usize % 4];
        let mut w = World::solo(
            seed,
            &SoundProfile::by_id(profile).unwrap(),
            Params::default(),
        );
        w.run(160);
        for v in &w.varieties {
            gained += v.harmony_events.iter().filter(|e| e.gained).count();
            lost += v.harmony_events.iter().filter(|e| !e.gained).count();
            active += usize::from(v.harmony.as_ref().is_some_and(Harmony::active));
            varieties += 1;
            if let Some(h) = v.harmony.as_ref().filter(|h| h.active()) {
                for word in v
                    .lexicon
                    .living()
                    .filter(|l| !h.disharmonic_loans.contains(&l.id))
                {
                    assert!(!h.feature.disharmonic(&word.form));
                }
            }
        }
    }
    let sample = sample::sample();
    let sample_gains: usize = sample
        .latest()
        .varieties
        .iter()
        .map(|v| v.harmony_events.iter().filter(|e| e.gained).count())
        .sum();
    println!(
        "harmony band: 40 seeds × 4,000 years; {varieties} varieties, {gained} inherited-inclusive gains, {lost} losses, {active} active; sample gains {sample_gains}; elapsed {:.2}s",
        start.elapsed().as_secs_f64()
    );
    // Harmony is a minority trait carried along family lines, as in Uralic
    // and Turkic: 68 of 290 varieties at revision 39.
    let share = active as f32 / varieties as f32;
    assert!(
        (0.10..=0.35).contains(&share),
        "active {active}/{varieties}"
    );
    assert!(lost <= gained);
}

use umran_sim::geography::{GeographyVersion, MapSize, continent_diagnostics};

fn median(values: &[f64]) -> f64 {
    let mut ranked: Vec<_> = values.iter().copied().enumerate().collect();
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    (ranked[(ranked.len() - 1) / 2].1 + ranked[ranked.len() / 2].1) * 0.5
}

#[test]
#[ignore = "40 seeds per size, V6 and V5 physical-area and spherical-convexity surveys"]
fn continental_v6_drift_has_compact_bodies_and_geological_relief() {
    let start = std::time::Instant::now();
    let mut all_old = Vec::new();
    let mut all_new = Vec::new();
    for (size, minimum_islands, convexity_floor) in [
        (MapSize::Small, 2, 0.78),
        (MapSize::Medium, 6, 0.79),
        (MapSize::Large, 6, 0.78),
        (MapSize::Vast, 15, 0.75),
    ] {
        let mut old_convexity = Vec::new();
        let mut new_convexity = Vec::new();
        let mut enclosed = 0;
        let mut land_range = [f64::INFINITY, f64::NEG_INFINITY];
        let mut counts = [usize::MAX, 0];
        let mut largest = [f64::INFINITY, f64::NEG_INFINITY];
        let mut islands = [usize::MAX, 0];
        let mut margin = 100.0_f64;
        for seed in 0..40 {
            let old = continent_diagnostics(seed, size, GeographyVersion::ContinentalV5);
            let d = continent_diagnostics(seed, size, GeographyVersion::ContinentalV6);
            // Measured land ranges S/M/L/V: 29.143–30.508, 29.393–30.838,
            // 29.393–30.838, 29.498–30.663 percent, after coast refinement.
            assert!(
                (0.285..=0.315).contains(&d.land_share),
                "{size:?}/{seed}: {d:?}"
            );
            if size == MapSize::Small {
                // Measured 2–5 continents on the 125,000 km² cutoff.
                assert!(
                    (2..=5).contains(&d.continents.len()),
                    "{size:?}/{seed}: {d:?}"
                );
            }
            if size == MapSize::Medium {
                // Measured 3–5 continents, largest 34.363–62.267% of land.
                assert!(
                    (2..=8).contains(&d.continents.len()),
                    "{size:?}/{seed}: {d:?}"
                );
                assert!(
                    (0.25..=0.65).contains(&d.top_shares[0]),
                    "{size:?}/{seed}: {d:?}"
                );
            }
            // Measured island minima S/M/L/V: 3, 11, 9, 18.
            assert!(d.island_count >= minimum_islands, "{size:?}/{seed}: {d:?}");
            // Measured minimum within two regional borders of an advected
            // convergent margin or suture: 89.474, 92.0, 92.0, 93.791 percent.
            assert!(
                d.mountains_near_margin_pct >= 75.0,
                "{size:?}/{seed}: {d:?}"
            );
            assert!(
                d.convexity
                    .iter()
                    .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            );
            old_convexity.extend(old.convexity);
            new_convexity.extend(d.convexity);
            enclosed += usize::from(d.enclosed_seas > 0);
            land_range[0] = land_range[0].min(d.land_share);
            land_range[1] = land_range[1].max(d.land_share);
            counts[0] = counts[0].min(d.continents.len());
            counts[1] = counts[1].max(d.continents.len());
            largest[0] = largest[0].min(d.top_shares[0]);
            largest[1] = largest[1].max(d.top_shares[0]);
            islands[0] = islands[0].min(d.island_count);
            islands[1] = islands[1].max(d.island_count);
            margin = margin.min(d.mountains_near_margin_pct);
        }
        let old_median = median(&old_convexity);
        let new_median = median(&new_convexity);
        // Centroid-gnomonic solidity is undefined for V5's largest trefoils.
        // The agreed replacement counts great-circle pairs staying on their
        // own landmass, with no excluded bodies. Measured V5→V6 medians:
        // S .683→.893, M .730→.871, L .730→.833, V .715→.779.
        // Each floor above lies between that size's old and new medians.
        // Earth is orientation, not a numerical calibration of this metric:
        // compact Africa scores high; Eurasia's gulfs and seas lower it.
        // https://www.naturalearthdata.com/downloads/110m-physical-vectors/110m-land/
        assert!(
            old_median < convexity_floor && new_median > convexity_floor,
            "{size:?}: V5 {old_median}, V6 {new_median}"
        );
        if size == MapSize::Medium {
            // Measured 28/40 seeds have a sea or gulf with >=70% of its rim
            // on one body; the requested lower band is 10/40 (one quarter).
            assert!(
                enclosed >= 10,
                "Medium enclosed seas in {enclosed}/40 seeds"
            );
        }
        println!(
            "V6 {size:?}: land={land_range:?} continents={counts:?} largest={largest:?} islands={islands:?} margin_min={margin:.3}% convexity V5={old_median:.6} V6={new_median:.6} enclosed={enclosed}/40"
        );
        all_old.extend(old_convexity);
        all_new.extend(new_convexity);
    }
    // All continents, no exclusions: measured V5 .719, V6 .838.
    assert!(median(&all_old) < 0.78 && median(&all_new) > 0.78);
    println!(
        "V6 all-continent convexity: V5={:.6} V6={:.6}; elapsed {:.2}s",
        median(&all_old),
        median(&all_new),
        start.elapsed().as_secs_f64()
    );
}

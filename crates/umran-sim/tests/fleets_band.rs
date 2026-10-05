#[path = "../examples/support/fleets.rs"]
mod fleets;
#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

#[test]
#[ignore = "40 mixed-size six-people worlds over 4,000 years plus the full sample recipe"]
fn fleets_band_40_seeds_4000_years() {
    let start = std::time::Instant::now();
    let mut counts = fleets::Counts::default();
    let mut sizes: [fleets::Counts; 4] = std::array::from_fn(|_| fleets::Counts::default());
    let mut maritime = fleets::Counts::default();
    let mut capacity_limited = 0;
    for seed in 0..40 {
        let size = seed as usize % fleets::SIZES.len();
        let mut world = fleets::world(seed, fleets::SIZES[size]);
        world.run(160);
        counts.add(&world);
        sizes[size].add(&world);
        let (control, limited) = fleets::maritime(seed, fleets::SIZES[size]);
        maritime.add(&control);
        capacity_limited += limited;
    }
    for (size, counts) in fleets::SIZES.iter().zip(&sizes) {
        counts.report(&format!("{size:?}"));
    }
    counts.report("40 seeds × 4,000 years");
    maritime.report("40 authored maritime controls (year zero, not spontaneous migrations)");
    println!("Capacity-limited maritime journeys refused: {capacity_limited}");
    let mut sample_counts = fleets::Counts::default();
    sample_counts.add(sample::sample().latest());
    sample_counts.report("Full sample recipe");
    println!("fleet band elapsed {:.2}s", start.elapsed().as_secs_f64());

    // e12e917: 389 land migrations and no sea movement (fecd8e0: 363).
    // Continental-v4: 368 land, 53 sea (12.6%). Continental-v5 at revision
    // 53: 330 land, 175 sea (34.7%); revision 54: 420 land, 66 sea (13.58%),
    // 220 sea settlements, 149 rented, 40 mixed; by size 13%, 3% (1 of 38),
    // 17%, 12%. Sea movement is a minority overall and never the main way
    // any size of world migrates; one Medium voyage is too few to pin.
    let land = counts.migrations - counts.sea_migrations;
    let sea_share = counts.sea_migrations as f64 / counts.migrations.max(1) as f64;
    // V5 measures 420 land migrations (450 at a 600-km sea cap); leave
    // headroom above the old 453 ceiling without accepting lost land movement.
    assert!((292..=500).contains(&land), "{counts:?}");
    assert!((0.08..=0.20).contains(&sea_share), "{counts:?}");
    for (size, counts) in fleets::SIZES.iter().zip(&sizes) {
        let share = counts.sea_migrations as f64 / counts.migrations.max(1) as f64;
        assert!(share <= 0.30, "{size:?}: {counts:?}");
    }
    assert!((5..=300).contains(&counts.sea_settlements), "{counts:?}");
    assert!((1..=200).contains(&counts.rented), "{counts:?}");
    assert!((1..=100).contains(&counts.mixed), "{counts:?}");
    assert!(
        (500..=3_000).contains(&counts.built) && counts.lost <= counts.built,
        "{counts:?}"
    );
    assert!(
        counts.rented <= counts.sea_migrations + counts.sea_settlements,
        "{counts:?}"
    );
    assert!((20..=1_000).contains(&counts.routes), "{counts:?}");
    // These authored controls test capacity, not the spontaneous rate.
    assert!(
        (120..=200).contains(&maritime.sea_settlements),
        "{maritime:?}"
    );
    assert!((30..=50).contains(&maritime.rented), "{maritime:?}");
    assert!(
        (0.15..=0.35).contains(&(maritime.rented as f64 / maritime.sea_settlements as f64)),
        "{maritime:?}"
    );
    assert!((60..=120).contains(&maritime.routes), "{maritime:?}");
    assert!((30..=50).contains(&capacity_limited), "{capacity_limited}");
}

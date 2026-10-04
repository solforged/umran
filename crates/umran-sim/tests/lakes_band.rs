#[path = "../examples/lakes.rs"]
mod lakes;

#[test]
#[ignore = "40 seeded 4,000-year histories per size plus the authored sample recipe"]
fn lake_basins_and_hydronyms_remain_a_minor_but_persistent_part_of_the_world() {
    for (size, maximum) in [
        (umran_sim::MapSize::Small, 20.0),
        (umran_sim::MapSize::Medium, 60.0),
        (umran_sim::MapSize::Large, 60.0),
        (umran_sim::MapSize::Vast, 200.0),
    ] {
        let report = lakes::report(size, 40, 160);
        let per_world = report.lakes as f64 / report.worlds as f64;
        let area = report.lake_area / report.land_area;
        let closed = report.closed as f64 / report.lakes as f64;
        let named = report.named as f64 / report.lakes as f64;
        assert!(
            (0.1..maximum).contains(&per_world),
            "{size:?}: lakes per world {per_world}"
        );
        assert!(
            (0.0001..0.15).contains(&area),
            "{size:?}: lake-region area share {area}"
        );
        assert!(
            (0.01..0.99).contains(&closed),
            "{size:?}: endorheic share {closed}"
        );
        assert!(
            (0.05..=1.0).contains(&named),
            "{size:?}: remembered lake share {named}"
        );
    }
    let sample = lakes::report_sample();
    assert!(sample.lakes > 0 && sample.named > 0);
}

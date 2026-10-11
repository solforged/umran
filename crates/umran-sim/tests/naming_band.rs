#[path = "support/naming.rs"]
mod naming;

#[test]
fn daughter_names_and_living_forms_stay_unstacked() {
    // A short every-generation invariant band; names_band reports the full
    // forty-seed, two-hundred-generation cohort with the same checks.
    naming::band(8, 200, umran_sim::MapSize::Medium, true);
}

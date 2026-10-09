use umran_sim::{
    GeographyVersion, Livelihood, MapSize, Naming, Params, SoundProfile, World, WorldEvent,
};

#[test]
fn farming_founders_bud_within_a_few_centuries() {
    let mut first_spreads = Vec::new();
    for seed in 0..40 {
        let mut world = World::with_geography(
            seed,
            Params::default(),
            MapSize::Medium,
            GeographyVersion::ContinentalV6,
        );
        let founder = world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            seed,
            0.5,
            0.5,
            None,
            Some(Livelihood::Farming),
            None,
        );
        let mut first = None;
        for _ in 0..40 {
            let start = world.events.len();
            world.step();
            first = world.events[start..].iter().find_map(|(g, event)| {
                matches!(event, WorldEvent::Spread { community, .. } if *community == founder)
                    .then_some(g * 25)
            });
            if first.is_some() {
                break;
            }
        }
        // Keep unspread founders in the sample, above the observed horizon.
        first_spreads.push(first.unwrap_or(1025));
    }
    first_spreads.sort_unstable();
    let median = (first_spreads[19] + first_spreads[20]) as f32 / 2.0;
    let by_400 = first_spreads.iter().filter(|&&year| year <= 400).count();
    println!("40 farming founders: first Spread median={median:.1} years; {by_400}/40 by year 400");
    assert!(
        (25.0..=400.0).contains(&median),
        "first Spread median={median}"
    );
    assert!(by_400 >= 20, "only {by_400}/40 spread by year 400");
}

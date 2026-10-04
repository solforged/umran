use super::*;
use crate::{MapSize, Naming, Params, SoundProfile};

fn world() -> World {
    World::with_map(
        21,
        Params {
            seasons_enabled: true,
            ..Params::static_society()
        },
        MapSize::Medium,
    )
}

#[test]
fn latitude_continentality_elevation_and_hemispheres_shape_the_cycle() {
    let world = world();
    let mut land = world
        .map
        .regions
        .iter()
        .find(|r| r.terrain.is_land())
        .unwrap()
        .clone();
    land.terrain = Terrain::Plains;
    land.elevation = 0.0;
    land.position = [1.0, 0.0, 0.0];
    let equator = SeasonalProfile::of(&land, 0.0, 100_000.0, false);
    land.position = [0.6, 0.0, 0.8];
    let coast = SeasonalProfile::of(&land, 0.0, 100_000.0, false);
    let interior = SeasonalProfile::of(&land, 1_000.0, 100_000.0, false);
    land.elevation = 0.9;
    let highland = SeasonalProfile::of(&land, 1_000.0, 100_000.0, false);
    assert!(equator.amplitude < coast.amplitude);
    assert!(coast.amplitude < interior.amplitude);
    assert!(interior.amplitude < highland.amplitude);
    assert!(coast.warmth_at(0.5, true, 0.5) > coast.warmth_at(0.5, true, 0.0));
    assert_eq!(
        coast.warmth_at(0.5, true, 0.5),
        coast.warmth_at(0.5, false, 0.0)
    );
    assert!((coast.warmth_at(0.5, true, 0.25) - 0.5).abs() < 1e-6);
    land.position[2] = -0.8;
    assert_eq!(
        SeasonalProfile::of(&land, 1_000.0, 100_000.0, false),
        highland
    );
}

#[test]
fn monsoons_need_large_low_latitude_land_and_floods_need_lowland_rivers() {
    let world = world();
    let mut land = world
        .map
        .regions
        .iter()
        .find(|r| r.terrain.is_land())
        .unwrap()
        .clone();
    land.terrain = Terrain::Plains;
    land.position = [0.95, 0.0, 0.3];
    let monsoon = SeasonalProfile::of(&land, 300.0, 1_000_000.0, true);
    assert_eq!(monsoon.wet, WetSeason::Monsoon);
    assert!(monsoon.floods);
    assert!(monsoon.rain_at(true, 0.5) > 1.8);
    assert!(monsoon.rain_at(true, 0.0) < 0.2);
    assert_eq!(monsoon.rain_at(true, 0.5), monsoon.rain_at(false, 0.0));
    assert_ne!(
        SeasonalProfile::of(&land, 300.0, 100_000.0, false).wet,
        WetSeason::Monsoon
    );
    assert!(!SeasonalProfile::of(&land, 300.0, 1_000_000.0, false).floods);
    land.terrain = Terrain::Mountains;
    assert!(!SeasonalProfile::of(&land, 300.0, 1_000_000.0, true).floods);
    land.terrain = Terrain::Sea;
    assert_eq!(SeasonalProfile::of(&land, 0.0, 0.0, false).amplitude, 0.0);
}

#[test]
fn geography_and_weather_do_not_depend_on_chart_coordinates_or_occupation() {
    let mut empty = world();
    let mut occupied = empty.clone();
    occupied.found(&SoundProfile::base(), 0.5, 0.5);
    let mut shifted = (*empty.map).clone();
    for r in &mut shifted.regions {
        r.site = [9999.0, -9999.0];
    }
    assert_eq!(Seasons::new(&shifted), empty.climate.seasons);
    for generation in 1..=20 {
        for w in [&mut empty, &mut occupied] {
            w.generation = generation;
            w.advance_climate();
            w.advance_seasons();
        }
        assert_eq!(
            empty.climate.seasons.impacts,
            occupied.climate.seasons.impacts
        );
        assert_eq!(empty.climate.regions, occupied.climate.regions);
    }
}

#[test]
fn shocks_do_not_compound_and_disabled_weather_restores_food_and_expires_causes() {
    let mut world = world();
    let baseline = world.climate.regions.clone();
    world.generation = 1;
    world.advance_seasons();
    let once = world.climate.regions.clone();
    assert_ne!(baseline, once);
    world.advance_climate();
    world.advance_seasons();
    assert_eq!(world.climate.regions, once);
    world.params.seasons_enabled = false;
    world.advance_climate();
    world.advance_seasons();
    assert_eq!(world.climate.regions, baseline);
    for r in 0..world.map.regions.len() {
        assert!(world.feeding_cause(r, Livelihood::Farming).is_none());
    }
}

#[test]
fn static_society_keeps_seasons_inert() {
    let mut world = World::new(21, Params::static_society());
    let before = world.climate.clone();
    world.run(10);
    assert_eq!(world.climate, before);
    assert!(world.events.is_empty());
}

// Find an actual seeded drought that newly makes herding attractive, rather
// than adjusting draws or food tables to manufacture an adoption.
#[test]
fn drought_drives_adoption_and_the_response_names_its_actual_trigger() {
    let mut world = world();
    world.params.adoption_rate = 1_000.0;
    let mut selected = None;
    for generation in 1..=160 {
        world
            .climate
            .seasons
            .sample(world.seed, generation, &world.map, &world.climate.regions);
        selected = world.map.regions.iter().enumerate().find_map(|(r, _)| {
            let impact = world.climate.seasons.impacts[r]?;
            let base = world.climate.regions[r].feeding;
            let losses = impact.hazard.losses(impact.severity);
            let own = Livelihood::Farming as usize;
            let next = Livelihood::Herding as usize;
            (impact.hazard == SeasonalHazard::Drought
                && base[own] > 0.0
                && base[next] < crate::world::ADOPT_GAIN * base[own]
                && base[next] * (1.0 - losses[next])
                    >= crate::world::ADOPT_GAIN * base[own] * (1.0 - losses[own]))
                .then_some((generation, r))
        });
        if selected.is_some() {
            break;
        }
    }
    let (generation, home) = selected.expect("a seeded drought crosses the adoption threshold");
    let c = world.found_seeded(
        &Naming::People,
        &SoundProfile::base(),
        31,
        0.5,
        0.5,
        Some(home),
        Some(Livelihood::Farming),
        None,
    );
    world.communities[c].size = 1_000.0;
    world.generation = generation - 1;
    world.step();
    assert_eq!(world.communities[c].livelihood, Livelihood::Herding);
    let response = world
        .events
        .iter()
        .position(|(_, e)| matches!(e, WorldEvent::Adopted { community, .. } if *community == c))
        .unwrap();
    let cause = world.causes[&response];
    assert_eq!(cause.mechanism, Mechanism::Climate);
    assert!(cause.event < response);
    assert!(matches!(&world.events[cause.event].1,
        WorldEvent::SeasonalHazard { region, hazard: SeasonalHazard::Drought, peoples, .. }
        if *region == home && peoples.contains(&c)));
    // Remembered hardship expires, even after the people leaves the affected land.
    world.params.seasons_enabled = false;
    world.run(crate::polity::CHALLENGE_SPAN + 1);
    assert_eq!(world.hard_times_cause(c), None);
}

#[test]
fn seasonal_migration_records_the_hazard_not_an_unrelated_epoch() {
    let mut world = world();
    world.params.migration_rate = 1_000.0;
    let mut selected = None;
    for generation in 1..=160 {
        world
            .climate
            .seasons
            .sample(world.seed, generation, &world.map, &world.climate.regions);
        selected = world.map.regions.iter().enumerate().find_map(|(r, land)| {
            let impact = world.climate.seasons.impacts[r]?;
            let loss = impact.hazard.losses(impact.severity)[Livelihood::Farming as usize];
            (loss > 0.4
                && land.neighbours.iter().any(|&n| {
                    world.climate.seasons.impacts[n].is_none()
                        && world.feeds(n, Livelihood::Farming)
                            > world.feeds(r, Livelihood::Farming) * (1.0 - loss)
                        && world.feeds(n, Livelihood::Farming) > 2_000.0
                }))
            .then_some((generation, r))
        });
        if selected.is_some() {
            break;
        }
    }
    let (generation, home) = selected.unwrap();
    let c = world.found_seeded(
        &Naming::People,
        &SoundProfile::base(),
        31,
        0.5,
        0.5,
        Some(home),
        Some(Livelihood::Farming),
        None,
    );
    world.communities[c].size = 1_000.0;
    world.generation = generation - 1;
    world.step();
    let response = world
        .events
        .iter()
        .position(|(_, e)| matches!(e, WorldEvent::Migrated { community, .. } if *community == c))
        .unwrap();
    let cause = world.causes[&response];
    assert!(cause.event < response);
    assert_eq!(cause.mechanism, Mechanism::Climate);
    assert!(
        matches!(world.events[cause.event].1, WorldEvent::SeasonalHazard { region, .. } if region == home)
    );
}

#[test]
fn an_already_better_livelihood_does_not_acquire_a_seasonal_cause() {
    let mut world = world();
    let home = world
        .map
        .regions
        .iter()
        .enumerate()
        .find(|&(r, _)| {
            world.feeds(r, Livelihood::Herding)
                > crate::world::ADOPT_GAIN * world.feeds(r, Livelihood::Farming)
        })
        .unwrap()
        .0;
    let c = world.found_seeded(
        &Naming::People,
        &SoundProfile::base(),
        31,
        0.5,
        0.5,
        Some(home),
        Some(Livelihood::Farming),
        None,
    );
    world.climate.exposure.resize(world.communities.len(), None);
    world.climate.seasons.feeding[home] = world.climate.regions[home].feeding;
    let impact = SeasonalImpact {
        hazard: SeasonalHazard::Drought,
        severity: 1.0,
    };
    world.climate.seasons.impacts[home] = Some(impact);
    world.apply_seasonal_hazard(home, impact);
    assert!(world.feeding_cause(home, Livelihood::Farming).is_some());
    assert_eq!(
        world.adoption_cause(c, Livelihood::Farming, Livelihood::Herding),
        None
    );
}

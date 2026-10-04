use super::*;
use crate::{
    ContactKind, Craft, Livelihood, MapSize, Naming, Params, Revelation, Rise, SoundProfile,
    Terrain,
};

fn settle(world: &mut World, region: usize, power: f32, size: f32) -> usize {
    let c = world.found_seeded(
        &Naming::People,
        &SoundProfile::base(),
        world.communities.len() as u64 + 31,
        power,
        0.5,
        Some(region),
        Some(Livelihood::Farming),
        None,
    );
    world.communities[c].size = size;
    c
}

fn response(world: &World, predicate: impl Fn(&WorldEvent) -> bool) -> usize {
    world
        .events
        .iter()
        .position(|(_, e)| predicate(e))
        .expect("response occurred")
}

fn cause(world: &World, response: usize, mechanism: Mechanism) -> usize {
    let cause = world.causes.get(&response).expect("recorded cause");
    assert_eq!(cause.mechanism, mechanism);
    assert!(cause.event < response);
    assert!(world.events[cause.event].0 <= world.events[response].0);
    cause.event
}

#[test]
fn hardship_causes_rise_and_collapse_but_expires() {
    let mut world = World::new(
        21,
        Params {
            hardship_rate: 1.0,
            state_rate: 100.0,
            ..Params::static_society()
        },
    );
    let home = world
        .map
        .regions
        .iter()
        .position(|r| r.terrain == Terrain::Plains)
        .unwrap();
    settle(&mut world, home, 0.5, 200_000.0);
    world.step();
    let rose = response(&world, |e| matches!(e, WorldEvent::Rose { .. }));
    let hardship = cause(&world, rose, Mechanism::Hardship);
    assert!(
        matches!(world.events[hardship].1, WorldEvent::HardTimes { region, .. } if region == home)
    );
    world.params.hardship_rate = 0.0;
    world.params.collapse_rate = 1.0;
    let mut expired = world.clone();
    expired.generation += crate::polity::CHALLENGE_SPAN + 1;
    expired.hold_states();
    let fell = response(&expired, |e| matches!(e, WorldEvent::Fell { .. }));
    assert!(
        !expired.causes.contains_key(&fell),
        "old hardship must not explain a later collapse"
    );
    world.generation += 1;
    world.hold_states();
    let fell = response(&world, |e| matches!(e, WorldEvent::Fell { .. }));
    assert_eq!(cause(&world, fell, Mechanism::Hardship), hardship);
}

#[test]
fn stronger_state_and_conquest_name_the_actual_events() {
    let mut world = World::new(
        21,
        Params {
            state_rate: 100.0,
            ..Params::static_society()
        },
    );
    let home = world.map.landmasses[0].anchor;
    let ruler = settle(&mut world, home, 0.9, 80_000.0);
    let other = settle(&mut world, home, 0.2, 60_000.0);
    let state = world.raise_state(ruler, None, Rise::Proclaimed);
    let first = world.triggers.states[&state];
    world
        .connect(ruler, other, 1.0, ContactKind::Neighbours)
        .unwrap();
    world.rise_states();
    let rose = response(&world, |e| matches!(e, WorldEvent::Rose { state: 1 }));
    assert_eq!(cause(&world, rose, Mechanism::StrongerNeighbour), first);
    let conquest = world.events.len();
    world.connect(ruler, other, 1.0, ContactKind::Rule).unwrap();
    let fell = response(&world, |e| matches!(e, WorldEvent::Fell { state: 1 }));
    assert_eq!(cause(&world, fell, Mechanism::Conquest), conquest);
}

#[test]
fn conversion_uses_the_selected_contact_and_forgets_replaced_links() {
    let mut world = World::new(
        21,
        Params {
            conversion_rate: 100.0,
            ..Params::static_society()
        },
    );
    let home = world.map.landmasses[0].anchor;
    let teacher = settle(&mut world, home, 0.8, 1000.0);
    let pupil = settle(&mut world, home, 0.5, 1000.0);
    world.found_religion(teacher, Revelation::Proclaimed);
    let meeting = world.events.len();
    world
        .connect(teacher, pupil, 1.0, ContactKind::Religion)
        .unwrap();
    let mut unrecorded = world.clone();
    unrecorded.link(teacher, pupil, 1.0, ContactKind::Religion);
    unrecorded.spread_faiths();
    let converted = response(
        &unrecorded,
        |e| matches!(e, WorldEvent::Converted { community, .. } if *community == pupil),
    );
    assert!(!unrecorded.causes.contains_key(&converted));
    world.spread_faiths();
    let converted = response(
        &world,
        |e| matches!(e, WorldEvent::Converted { community, .. } if *community == pupil),
    );
    assert_eq!(cause(&world, converted, Mechanism::Contact), meeting);
    let split_start = world.events.len();
    world.split(pupil, None, 0.5);
    let split = world.events[split_start..]
        .iter()
        .position(|(_, e)| matches!(e, WorldEvent::Split { .. }))
        .unwrap()
        + split_start;
    assert!(
        !world.causes.contains_key(&split),
        "a split is not caused by an adjacent conversion"
    );
}

#[test]
fn city_responses_link_to_its_recorded_threshold() {
    let mut world = World::new(
        42,
        Params {
            city_rate: 0.25,
            ..Params::static_society()
        },
    );
    let home = world
        .map
        .regions
        .iter()
        .position(|r| r.terrain == Terrain::Plains)
        .unwrap();
    for size in [80_000.0, 70_000.0, 50_000.0] {
        settle(&mut world, home, 0.5, size);
    }
    world.connect(0, 1, 0.8, ContactKind::Rule).unwrap();
    world.connect(0, 2, 0.8, ContactKind::Rule).unwrap();
    for _ in 0..4 {
        world.generation += 1;
        world.grow_cities();
    }
    world.adopt_standard(0);
    let city = response(&world, |e| matches!(e, WorldEvent::City { city: 0 }));
    for response in [
        response(&world, |e| matches!(e, WorldEvent::Koine { city: 0, .. })),
        response(&world, |e| matches!(e, WorldEvent::Standard { state: 0 })),
    ] {
        assert_eq!(cause(&world, response, Mechanism::City), city);
    }
}

#[test]
fn an_overseas_pilgrim_contact_causes_conversion() {
    // This mechanism fixture relies on V3 seed geography.
    let mut world = World::with_geography(
        3,
        Params::static_society(),
        MapSize::Large,
        crate::GeographyVersion::ContinentalV3,
    );
    let (home, remote) = world
        .map
        .regions
        .iter()
        .enumerate()
        .find_map(|(a, r)| {
            if !r.terrain.is_land() {
                return None;
            }
            world.map.regions.iter().enumerate().find_map(|(b, s)| {
                (s.terrain.is_land()
                    && r.landmass != s.landmass
                    && world.map.voyage(a, b) <= world.params.pilgrimage_reach)
                    .then_some((a, b))
            })
        })
        .unwrap();
    let founder = settle(&mut world, home, 0.5, 1000.0);
    let pilgrim = settle(&mut world, remote, 0.5, 1000.0);
    let religion = world.found_religion(founder, Revelation::Proclaimed);
    world.religions[religion].shrine.region = home;
    world.convert(pilgrim, religion, None);
    world.communities[founder].faith = None;
    world.learn(pilgrim, Craft::Seafaring, None);
    world.params.pilgrimage_rate = 100.0;
    world.send_pilgrims();
    let route = response(
        &world,
        |e| matches!(e, WorldEvent::Pilgrimage { community, .. } if *community == pilgrim),
    );
    world.params.conversion_rate = 100.0;
    world.spread_faiths();
    let converted = response(
        &world,
        |e| matches!(e, WorldEvent::Converted { community, .. } if *community == founder),
    );
    assert_eq!(cause(&world, converted, Mechanism::Pilgrimage), route);
}

#[test]
fn holy_conquest_requires_a_recorded_loss_to_that_holder() {
    let mut world = World::new(21, Params::static_society());
    let home = world.map.landmasses[0].anchor;
    let holder = settle(&mut world, home, 0.0, 2000.0);
    let attacker = settle(&mut world, home, 1.0, 1000.0);
    let religion = world.found_religion(holder, Revelation::Proclaimed);
    world.religions[religion].shrine.region = home;
    world.convert(attacker, religion, None);
    world.params.pilgrimage_rate = 1.0;
    world.observe_holy_lands();
    world.communities[holder].faith = None;
    world.params.conquest_rate = 100.0;
    assert_eq!(world.holy_war_cause(attacker, holder), None);
    world.observe_holy_lands();
    let loss = response(&world, |e| {
        matches!(
            e,
            WorldEvent::HolyLand {
                faithful: false,
                ..
            }
        )
    });
    world
        .connect(attacker, holder, 1.0, ContactKind::Neighbours)
        .unwrap();
    world.params.pilgrimage_rate = 0.0;
    world.step();
    let conquest = response(
        &world,
        |e| matches!(e, WorldEvent::Conquered { ruler, ruled } if *ruler == attacker && *ruled == holder),
    );
    assert_eq!(cause(&world, conquest, Mechanism::UnfaithfulHolder), loss);
}

#[test]
fn migration_names_a_stronger_coresident_meeting() {
    for kind in [ContactKind::Neighbours, ContactKind::Rule] {
        let mut world = World::new(
            21,
            Params {
                migration_rate: 100.0,
                ..Params::static_society()
            },
        );
        let home = world
            .map
            .regions
            .iter()
            .position(|r| {
                r.terrain == Terrain::Plains
                    && r.neighbours
                        .iter()
                        .any(|&n| world.feeds(n, Livelihood::Farming) > 2000.0)
            })
            .unwrap();
        let migrant = settle(&mut world, home, 0.0, 1000.0);
        let size = world.feeds(home, Livelihood::Farming) * 2.0;
        let stronger = settle(&mut world, home, 1.0, size);
        let meeting = world.events.len();
        world.connect(migrant, stronger, 1.0, kind).unwrap();
        world.step();
        let moved = response(
            &world,
            |e| matches!(e, WorldEvent::Migrated { community, .. } if *community == migrant),
        );
        assert_eq!(cause(&world, moved, Mechanism::StrongerNeighbour), meeting);
    }
}

#[test]
fn drying_causes_livelihood_change_and_recovery_does_not_linger() {
    let mut world = World::new(
        21,
        Params {
            climate_enabled: true,
            adoption_rate: 1000.0,
            ..Params::static_society()
        },
    );
    let home = world
        .map
        .regions
        .iter()
        .enumerate()
        .find(|&(r, land)| land.terrain == Terrain::Plains && world.map.river_regions[r].is_none())
        .unwrap()
        .0;
    let c = settle(&mut world, home, 0.5, 1000.0);
    for zone in &mut world.climate.zones {
        zone.remaining = 100;
    }
    let zone = world.map.regions[home].climate_zone.unwrap();
    world.climate.zones[zone].target_wetness = -0.65;
    world.climate.zones[zone].cause = crate::ClimateCause::Drought;
    world.run(20);
    let adopted = response(
        &world,
        |e| matches!(e, WorldEvent::Adopted { community, livelihood: Livelihood::Herding, .. } if *community == c),
    );
    let event = cause(&world, adopted, Mechanism::Climate);
    assert!(
        matches!(world.events[event].1, WorldEvent::Climate { zone: z, change: ClimateChange::Onset | ClimateChange::Worsening, .. } if z == zone)
    );
    world.climate.zones[zone].target_wetness = 0.0;
    world.climate.zones[zone].target_warmth = 0.0;
    world.run(30);
    assert!(world.events.iter().any(|(_, e)| matches!(e, WorldEvent::Climate { zone: z, change: ClimateChange::Recovery, .. } if *z == zone)));
    assert_eq!(world.feeding_cause(home, Livelihood::Farming), None);
}

#[test]
fn joining_a_schism_uses_contact_but_the_reformer_does_not() {
    let mut joined = 0;
    for seed in 0..8 {
        let mut world = World::new(seed, Params::static_society());
        let home = world.map.landmasses[0].anchor;
        let founder = settle(&mut world, home, 0.5, 2000.0);
        let religion = world.found_religion(founder, Revelation::Proclaimed);
        let reformer = world.split(founder, None, 0.0);
        world.communities[reformer].lands = vec![home];
        let meeting = world.events.len();
        world
            .connect(founder, reformer, 1.0, ContactKind::Religion)
            .unwrap();
        let branch = world
            .schism(reformer, crate::schisms::SchismCause::Reform)
            .unwrap();
        assert_eq!(world.religions[branch].parent, Some(religion));
        let converted = response(
            &world,
            |e| matches!(e, WorldEvent::Converted { community, religion, .. } if *community == reformer && *religion == branch),
        );
        assert!(
            !world.causes.contains_key(&converted),
            "the reformer did not convert through a contact"
        );
        if world.communities[founder].faith == Some(branch) {
            joined += 1;
            let converted = response(
                &world,
                |e| matches!(e, WorldEvent::Converted { community, religion, .. } if *community == founder && *religion == branch),
            );
            assert_eq!(cause(&world, converted, Mechanism::Contact), meeting);
        }
    }
    assert!(
        joined > 0,
        "the cohort must exercise a contact-based joining"
    );
}

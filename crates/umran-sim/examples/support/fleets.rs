use umran_sim::{Itinerary, MapSize, Naming, Params, SoundProfile, TravelBy, World, WorldEvent};

pub const SIZES: [MapSize; 4] = [
    MapSize::Small,
    MapSize::Medium,
    MapSize::Large,
    MapSize::Vast,
];

#[derive(Debug, Default)]
pub struct Counts {
    pub built: usize,
    pub lost: usize,
    pub migrations: usize,
    pub sea_migrations: usize,
    pub splits: usize,
    pub settlements: usize,
    pub sea_settlements: usize,
    pub rented: usize,
    pub mixed: usize,
    pub routes: usize,
}

impl Counts {
    fn itinerary(&mut self, community: usize, route: &Itinerary) -> bool {
        let sea = route.legs.iter().any(|leg| leg.by == TravelBy::Sea);
        self.rented += usize::from(route.carriers.iter().any(|&c| c != community));
        self.mixed += usize::from(sea && route.legs.iter().any(|leg| leg.by != TravelBy::Sea));
        for leg in &route.legs {
            assert!(leg.km.is_finite() && leg.km > 0.0);
        }
        for pair in route.legs.windows(2) {
            assert_eq!(pair[0].to, pair[1].from);
        }
        sea
    }

    pub fn add(&mut self, world: &World) {
        for (_, event) in &world.events {
            self.splits += usize::from(matches!(event, WorldEvent::Split { .. }));
            match event {
                WorldEvent::FleetBuilt { .. } => self.built += 1,
                WorldEvent::FleetLost { .. } => self.lost += 1,
                WorldEvent::SeaRouteOpened { .. } => self.routes += 1,
                WorldEvent::Migrated {
                    community,
                    itinerary,
                    by_sea,
                    ..
                } => {
                    self.migrations += 1;
                    let sea = self.itinerary(*community, itinerary);
                    assert_eq!(sea, *by_sea);
                    self.sea_migrations += usize::from(sea);
                }
                WorldEvent::Split {
                    community,
                    itinerary: Some(route),
                    travelled: true,
                    by_sea,
                    ..
                } => {
                    self.settlements += 1;
                    let sea = self.itinerary(*community, route);
                    assert_eq!(sea, *by_sea);
                    self.sea_settlements += usize::from(sea);
                }
                WorldEvent::Settlement(record) if !record.plan.routes.is_empty() => {
                    self.settlements += 1;
                    self.sea_settlements += usize::from(
                        record
                            .plan
                            .routes
                            .iter()
                            .any(|route| route.legs.iter().any(|leg| leg.by == TravelBy::Sea)),
                    );
                    self.rented += usize::from(record.plan.routes.iter().any(|route| {
                        route
                            .carriers
                            .iter()
                            .any(|&c| c != record.plan.choice.community)
                    }));
                    self.mixed += usize::from(record.plan.routes.iter().any(|route| {
                        route.legs.iter().any(|leg| leg.by == TravelBy::Sea)
                            && route.legs.iter().any(|leg| leg.by != TravelBy::Sea)
                    }));
                }
                _ => {}
            }
        }
        for people in &world.communities {
            if let Some(fleet) = &people.fleet {
                assert!(people.living());
                assert!((0.2..=1.0).contains(&fleet.strength));
                assert!(!fleet.ports.is_empty());
                assert!(
                    fleet
                        .ports
                        .iter()
                        .all(|port| people.lands.contains(port) && world.map.coastal(*port))
                );
            }
        }
    }

    pub fn sea_share(&self) -> f64 {
        (self.sea_migrations + self.sea_settlements) as f64
            / (self.migrations + self.settlements).max(1) as f64
    }

    pub fn report(&self, label: &str) {
        println!(
            "{label}: fleets built {}, lost {}; land migrations {}; sea migrations {}/{} ({:.2}%); splits {}; sea settlements {}/{} ({:.2}%); all sea {:.2}%; rented-berth journeys {}; mixed journeys {}; sea routes opened {}",
            self.built,
            self.lost,
            self.migrations - self.sea_migrations,
            self.sea_migrations,
            self.migrations,
            100.0 * self.sea_migrations as f64 / self.migrations.max(1) as f64,
            self.splits,
            self.sea_settlements,
            self.settlements,
            100.0 * self.sea_settlements as f64 / self.settlements.max(1) as f64,
            100.0 * self.sea_share(),
            self.rented,
            self.mixed,
            self.routes,
        );
    }
}

/// Six peoples, three sharing a coastal homeland and three freely founded.
/// Only founding is authored: all crafts, fleets, and journeys then emerge
/// under the default parameters. Profiles rotate independently of map size.
pub fn world(seed: u64, size: MapSize) -> World {
    let mut world = World::with_map(seed, Params::default(), size);
    let profiles = SoundProfile::presets();
    let shores: Vec<_> = (0..world.map.regions.len())
        .filter(|&r| world.map.coastal(r))
        .collect();
    let home = shores[(seed as usize * 17) % shores.len()];
    for i in 0..6 {
        world.found_seeded(
            &Naming::People,
            &profiles[(seed as usize + i) % profiles.len()],
            seed.wrapping_mul(6).wrapping_add(i as u64),
            0.5,
            0.5,
            (i < 3).then_some(home),
            None,
            None,
        );
    }
    world
}

/// A separate authored control, not a measurement of spontaneous sea migration:
/// three half-population colonies use their own boats; one small group rents.
/// The authored expedition radius is the selected finite sea corridor's cost.
/// This is intentionally separate from the natural cohort's default radii.
pub fn maritime(seed: u64, size: MapSize) -> (World, usize) {
    use umran_sim::settlement::{SettlementChoice, SettlementIntent};
    use umran_sim::{ContactKind, Craft, Livelihood};
    let mut world = World::with_map(seed, Params::default(), size);
    let (home, destination, effort) = (0..world.map.regions.len())
        .filter(|&r| world.map.coastal(r))
        .find_map(|home| {
            world
                .map
                .voyage_row(home, f32::INFINITY)
                .iter()
                .filter(|&&(r, _)| {
                    world.map.overseas(home, r as usize)
                        && world.feeds(r as usize, Livelihood::Farming) >= 2_000.0
                })
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                .map(|&(r, effort)| (home, r as usize, effort))
        })
        .expect("separate shores joined by a finite sea corridor");
    world.params.colony_reach = effort;
    let profiles = SoundProfile::presets();
    let mut found = |offset: usize| {
        world.found_seeded(
            &Naming::People,
            &profiles[(seed as usize + offset) % profiles.len()],
            seed.wrapping_mul(2).wrapping_add(offset as u64),
            0.5,
            0.5,
            Some(home),
            Some(Livelihood::Farming),
            None,
        )
    };
    let carrier = found(0);
    let passenger = found(1);
    world.learn(carrier, Craft::Seafaring, None);
    world
        .connect(passenger, carrier, 0.5, ContactKind::Neighbours)
        .unwrap();
    let mut choice = SettlementChoice {
        community: passenger,
        intent: SettlementIntent::Settlers,
        destination,
        share: 0.5,
        naming: None,
        intensity: 0.5,
    };
    assert!(
        world
            .plan_settlement(&choice)
            .unwrap_err()
            .contains("enough fleet capacity")
    );
    // The rejected half-population group and accepted small group share the
    // identical origin, destination, contact, and reach: capacity is the limit.
    choice.share = 0.1;
    world.settle(&choice).unwrap();
    choice.community = carrier;
    choice.share = 0.5;
    for _ in 0..3 {
        world.settle(&choice).unwrap();
    }
    (world, 1)
}

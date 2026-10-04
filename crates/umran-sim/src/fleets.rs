//! Coastal holdings sustain ships; known carriers admit passengers at their ports.
//! Distances retain the map's effort-km, including 100 km at each sea crossing.
use crate::{Cause, Craft, Journey, Mechanism, World, WorldEvent};
use serde::Serialize;
use std::{cmp::Ordering, collections::BinaryHeap};

/// Transport over a generation, not a count of hulls.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Fleet {
    pub since: u32,
    pub strength: f32,
    pub ports: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TravelBy {
    Land,
    Sea,
    River,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Leg {
    pub from: usize,
    pub to: usize,
    pub by: TravelBy,
    /// Equivalent plain kilometres, including embarkation and landing.
    pub km: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Itinerary {
    pub path: Vec<usize>,
    pub legs: Vec<Leg>,
    pub carriers: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Visit {
    effort: f64,
    sea: bool,
    region: usize,
}
impl Eq for Visit {}
impl Ord for Visit {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .effort
            .total_cmp(&self.effort)
            .then_with(|| other.sea.cmp(&self.sea))
            .then_with(|| other.region.cmp(&self.region))
    }
}
impl PartialOrd for Visit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Phase-local storage, reused for every traveller. No route graph is replay state.
pub(crate) struct Routes {
    distance: Vec<f64>,
    sea: Vec<bool>,
    capacity: Vec<f32>,
    parent: Vec<usize>,
    carrier: Vec<Option<usize>>,
    ports: Vec<Option<(usize, f32)>>,
    heap: BinaryHeap<Visit>,
    pub(crate) touched: Vec<usize>,
    pub(crate) destinations: Vec<usize>,
}
impl Routes {
    pub(crate) fn new(n: usize) -> Self {
        Self {
            distance: vec![f64::INFINITY; n],
            sea: vec![false; n],
            capacity: vec![f32::INFINITY; n],
            parent: vec![usize::MAX; n],
            carrier: vec![None; n],
            ports: vec![None; n],
            heap: BinaryHeap::new(),
            touched: Vec::new(),
            destinations: Vec::new(),
        }
    }
    pub(crate) fn journey(&self, r: usize) -> Option<Journey> {
        self.distance[r].is_finite().then_some(Journey {
            effort: self.distance[r] as f32,
            by_sea: self.sea[r],
            capacity: self.capacity[r],
        })
    }
    pub(crate) fn itinerary(&self, world: &World, target: usize) -> Option<Itinerary> {
        self.journey(target)?;
        let mut path = vec![target];
        while self.parent[*path.last().unwrap()] != *path.last().unwrap() {
            path.push(self.parent[*path.last().unwrap()]);
        }
        path.reverse();
        let mut legs: Vec<Leg> = Vec::new();
        let mut carriers = Vec::new();
        for edge in path.windows(2) {
            let (a, b) = (edge[0], edge[1]);
            let by = if !world.map.regions[a].terrain.is_land()
                || !world.map.regions[b].terrain.is_land()
            {
                TravelBy::Sea
            } else if world.map.river_edge(a, b) {
                TravelBy::River
            } else {
                TravelBy::Land
            };
            let km = (self.distance[b] - self.distance[a]) as f32;
            if let Some(last) = legs.last_mut().filter(|l| l.by == by) {
                last.to = b;
                last.km += km;
            } else {
                legs.push(Leg {
                    from: a,
                    to: b,
                    by,
                    km,
                });
            }
            if let Some(c) = self.carrier[b]
                && !carriers.contains(&c)
            {
                carriers.push(c);
            }
        }
        Some(Itinerary {
            path,
            legs,
            carriers,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn search(
        &mut self,
        world: &World,
        community: usize,
        sources: &[usize],
        reach: f32,
        passengers: f32,
        target: Option<usize>,
        rent: bool,
    ) {
        for r in self.touched.drain(..) {
            self.distance[r] = f64::INFINITY;
            self.sea[r] = false;
            self.capacity[r] = f32::INFINITY;
            self.parent[r] = usize::MAX;
            self.carrier[r] = None;
        }
        self.heap.clear();
        self.ports.fill(None);
        self.destinations.clear();
        if reach.is_nan()
            || reach < 0.0
            || !passengers.is_finite()
            || passengers < 0.0
            || !world.communities[community].living()
        {
            return;
        }
        // Only the traveller and directly contacted carriers can offer berths.
        // Do not scan every living people and rescan all contacts for each one.
        for c in std::iter::once(community).chain(
            world
                .partners(community)
                .filter_map(|(other, intensity, _)| (rent && intensity > 0.0).then_some(other)),
        ) {
            if !world.communities[c].living() {
                continue;
            }
            let Some(fleet) = &world.communities[c].fleet else {
                continue;
            };
            // Ships scale with the people sustaining them, not a fixed hull
            // count. Even a weak fleet carries a half-population colony;
            // carrying everyone requires strength >= 0.5. Only a quarter of
            // that transport is available to another people's passengers.
            let capacity = world.communities[c].size
                * (0.5 + fleet.strength)
                * if c == community { 1.0 } else { 0.25 };
            if capacity < passengers {
                continue;
            }
            for &port in &fleet.ports {
                if !world.communities[c].lands.contains(&port) {
                    continue;
                }
                if self.ports[port]
                    .is_none_or(|(owner, old)| capacity > old || (capacity == old && c < owner))
                {
                    self.ports[port] = Some((c, capacity));
                }
            }
        }
        for &source in sources {
            if !world.map.regions[source].terrain.is_land() {
                continue;
            }
            self.distance[source] = 0.0;
            self.parent[source] = source;
            self.touched.push(source);
            self.heap.push(Visit {
                effort: 0.0,
                sea: false,
                region: source,
            });
        }
        while let Some(Visit {
            effort,
            sea,
            region: r,
        }) = self.heap.pop()
        {
            if effort != self.distance[r] || sea != self.sea[r] {
                continue;
            }
            if target == Some(r) {
                break;
            }
            let land = world.map.regions[r].terrain.is_land();
            for &n in &world.map.regions[r].neighbours {
                let next_land = world.map.regions[n].terrain.is_land();
                let embark = land && !next_land;
                if embark && self.ports[r].is_none() {
                    continue;
                }
                let by_sea = sea || !next_land;
                let cost = f64::from(world.map.border_effort(r, n).expect("shared border"))
                    + if land != next_land { 100.0 } else { 0.0 };
                let distance = effort + cost;
                if distance as f32 > reach {
                    continue;
                }
                let capacity = if embark {
                    self.capacity[r].min(self.ports[r].unwrap().1)
                } else {
                    self.capacity[r]
                };
                if distance < self.distance[n]
                    || (distance == self.distance[n] && self.sea[n] && !by_sea)
                {
                    if !self.distance[n].is_finite() {
                        self.touched.push(n);
                    }
                    self.distance[n] = distance;
                    self.sea[n] = by_sea;
                    self.capacity[n] = capacity;
                    self.parent[n] = r;
                    self.carrier[n] = if embark {
                        Some(self.ports[r].unwrap().0)
                    } else {
                        None
                    };
                    self.heap.push(Visit {
                        effort: distance,
                        sea: by_sea,
                        region: n,
                    });
                }
            }
        }
        self.touched.sort_unstable();
        self.destinations.extend(
            self.touched
                .iter()
                .copied()
                .filter(|&r| world.map.regions[r].terrain.is_land()),
        );
    }
}

impl World {
    pub(crate) fn transport_available(&self, community: usize) -> bool {
        self.communities[community].fleet.is_some()
            || self
                .partners(community)
                .any(|(c, intensity, _)| intensity > 0.0 && self.communities[c].fleet.is_some())
    }
    /// Seafaring is learned separately; possessing the craft inland grants no ships.
    pub(crate) fn refresh_fleet(&mut self, community: usize, cause: Option<Cause>) {
        let people = &self.communities[community];
        if people.fleet.is_none() && !people.crafts.contains(&Craft::Seafaring) {
            return;
        }
        let mut ports: Vec<_> = if people.living() && people.crafts.contains(&Craft::Seafaring) {
            people
                .lands
                .iter()
                .copied()
                .filter(|&r| self.map.coastal(r))
                .collect()
        } else {
            Vec::new()
        };
        ports.sort_unstable();
        ports.dedup();
        if ports.is_empty() {
            if let Some(old) = self.communities[community].fleet.take() {
                self.record_response(
                    WorldEvent::FleetLost {
                        community,
                        ports: old.ports,
                    },
                    cause,
                );
            }
            return;
        }
        let urban: f32 = self
            .cities
            .iter()
            .enumerate()
            .filter(|(_, city)| ports.contains(&city.region))
            .map(|(id, _)| self.city_size(id))
            .sum();
        let built = people.fleet.is_none();
        let since = people.fleet.as_ref().map_or(self.generation, |f| f.since);
        // Sustained coastal shipbuilding matures over eight generations.
        // Without this, a single-port people can never transport itself
        // unless it first develops a large city. Static societies do not mature.
        let experience = if self.params.craft_rate > 0.0 {
            0.3 * ((self.generation - since) as f32 / 8.0).min(1.0)
        } else {
            0.0
        };
        let strength =
            (0.2 + experience + 0.1 * (ports.len() - 1) as f32 + 0.4 * (urban / 20_000.0).min(1.0))
                .min(1.0);
        self.communities[community].fleet = Some(Fleet {
            since,
            strength,
            ports,
        });
        if built {
            self.record_response(WorldEvent::FleetBuilt { community }, cause);
        }
    }
    pub(crate) fn refresh_fleets(&mut self) {
        for c in 0..self.communities.len() {
            self.refresh_fleet(c, None);
        }
    }
    /// A directed mixed journey from a particular land, optionally requiring room for passengers.
    pub fn itinerary(
        &self,
        community: usize,
        from: usize,
        to: usize,
        reach: f32,
        passengers: f32,
    ) -> Option<Itinerary> {
        if !self.map.regions.get(from)?.terrain.is_land() {
            return None;
        }
        if !self.map.regions.get(to)?.terrain.is_land()
            || !self.communities.get(community)?.living()
        {
            return None;
        }
        let mut routes = Routes::new(self.map.regions.len());
        routes.search(self, community, &[from], reach, passengers, Some(to), true);
        routes.itinerary(self, to)
    }
    /// Record the first used sea corridor, and strengthen a rented carrier's contact.
    pub(crate) fn use_itinerary(
        &mut self,
        community: usize,
        route: &Itinerary,
        cause: Option<Cause>,
    ) {
        for leg in route.legs.iter().filter(|l| l.by == TravelBy::Sea) {
            if self.sea_routes.insert((community, leg.from, leg.to)) {
                self.record_response(
                    WorldEvent::SeaRouteOpened {
                        community,
                        from: leg.from,
                        to: leg.to,
                    },
                    cause,
                );
            }
        }
        for &carrier in &route.carriers {
            if carrier == community {
                continue;
            }
            if let Some(contact) = self.contacts.iter_mut().find(|c| {
                (c.a == community && c.b == carrier) || (c.b == community && c.a == carrier)
            }) {
                contact.intensity = (contact.intensity + 0.05 * (1.0 - contact.intensity)).min(1.0);
            }
        }
    }
    pub(crate) fn sea_contact_factor(&self, a: usize, b: usize) -> f32 {
        if self.nearness(a, b) > 0.0
            || self.communities[a].lands.iter().any(|&r| {
                self.communities[b]
                    .lands
                    .iter()
                    .any(|&s| self.map.regions[r].landmass == self.map.regions[s].landmass)
            })
        {
            return 1.0;
        }
        [a, b]
            .into_iter()
            .flat_map(|c| std::iter::once(c).chain(self.partners(c).map(|(p, _, _)| p)))
            .filter_map(|c| self.communities[c].fleet.as_ref().map(|f| f.strength))
            .fold(0.0, f32::max)
    }
    pub(crate) fn use_contact_route(&mut self, a: usize, b: usize, cause: Option<Cause>) {
        if !self.transport_available(a) && !self.transport_available(b) {
            return;
        }
        let mut routes = Routes::new(self.map.regions.len());
        let mut best: Option<(usize, usize, Journey)> = None;
        for (traveller, other) in [(a, b), (b, a)] {
            routes.search(
                self,
                traveller,
                &self.communities[traveller].lands,
                self.params.trade_reach,
                0.0,
                None,
                true,
            );
            for &r in &self.communities[other].lands {
                if let Some(j) = routes.journey(r)
                    && best.as_ref().is_none_or(|(_, _, old)| {
                        j.effort < old.effort || (j.effort == old.effort && old.by_sea && !j.by_sea)
                    })
                {
                    best = Some((traveller, r, j));
                }
            }
        }
        if let Some((traveller, target, j)) = best
            && j.by_sea
        {
            routes.search(
                self,
                traveller,
                &self.communities[traveller].lands,
                self.params.trade_reach,
                0.0,
                Some(target),
                true,
            );
            let itinerary = routes.itinerary(self, target).expect("selected route");
            self.use_itinerary(traveller, &itinerary, cause);
        }
    }
    pub(crate) fn fleet_craft_cause(&self, community: usize) -> Option<Cause> {
        self.events.iter().rposition(|(_,e)| matches!(e, WorldEvent::Learnt { community:c, craft:Craft::Seafaring, .. } if *c == community))
            .map(|event| Cause { event, mechanism: Mechanism::Craft })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContactKind, Naming, Params, SoundProfile};

    fn shores() -> (World, usize, usize, usize) {
        let mut world = World::new(7, Params::static_society());
        let inland = |r: usize| {
            world.map.regions[r]
                .neighbours
                .iter()
                .copied()
                .find(|&n| world.map.regions[n].terrain.is_land() && !world.map.coastal(n))
        };
        let (port, home, destination) = (0..world.map.regions.len())
            .filter(|&r| world.map.coastal(r))
            .find_map(|port| {
                let home = inland(port)?;
                world
                    .map
                    .voyage_row(port, 1800.0)
                    .iter()
                    .find_map(|&(other, _)| {
                        let other = other as usize;
                        if !world.map.overseas(port, other) {
                            return None;
                        }
                        Some((port, home, inland(other)?))
                    })
            })
            .expect("two shores with inland neighbours");
        let mut found = |region| {
            world.found_seeded(
                &Naming::People,
                &SoundProfile::base(),
                region as u64,
                0.5,
                0.5,
                Some(region),
                None,
                None,
            )
        };
        let carrier = found(port);
        let passenger = found(home);
        (world, carrier, passenger, destination)
    }

    #[test]
    fn contacted_carrier_supports_capacity_limited_mixed_journeys() {
        let (mut world, carrier, passenger, destination) = shores();
        let home = world.communities[passenger].home();
        world.learn(carrier, Craft::Seafaring, None);
        assert!(
            world
                .itinerary(passenger, home, destination, f32::INFINITY, 1.0)
                .is_none()
        );
        world
            .connect(passenger, carrier, 0.5, ContactKind::Neighbours)
            .unwrap();
        let route = world
            .itinerary(passenger, home, destination, f32::INFINITY, 175.0)
            .unwrap();
        assert_eq!(route.path.first(), Some(&home));
        assert_eq!(route.path.last(), Some(&destination));
        assert_eq!(route.carriers, vec![carrier]);
        assert!(route.legs.iter().any(|leg| leg.by == TravelBy::Sea));
        assert_ne!(route.legs.first().unwrap().by, TravelBy::Sea);
        assert_ne!(route.legs.last().unwrap().by, TravelBy::Sea);
        assert!(
            world
                .itinerary(passenger, home, destination, f32::INFINITY, 175.01)
                .is_none()
        );
        let port = world.communities[carrier].home();
        assert!(
            world
                .itinerary(carrier, port, destination, f32::INFINITY, 700.0)
                .is_some()
        );
        assert!(
            world
                .itinerary(carrier, port, destination, f32::INFINITY, 700.01)
                .is_none()
        );
        let remote = world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            19,
            0.5,
            0.5,
            Some(destination),
            None,
            None,
        );
        world.params.conquest_reach = f32::INFINITY;
        assert!(world.journey_between(passenger, remote).is_some());
        assert!(!world.can_rule(passenger, remote));
        let before = world.contacts[0].intensity;
        world.use_itinerary(passenger, &route, None);
        assert!(world.contacts[0].intensity > before);
        let opened = world.sea_routes.len();
        assert!(opened > 0);
        world.use_itinerary(passenger, &route, None);
        assert_eq!(world.sea_routes.len(), opened);
        world.contacts[0].intensity = 0.0;
        assert!(
            world
                .itinerary(passenger, home, destination, f32::INFINITY, 1.0)
                .is_none()
        );
    }

    #[test]
    fn a_sustained_single_port_matures_without_a_city_and_loss_resets_it() {
        let (mut world, carrier, inland, destination) = shores();
        let port = world.communities[carrier].home();
        world.learn(carrier, Craft::Seafaring, None);
        world.generation = 8;
        world.refresh_fleets();
        assert_eq!(
            world.communities[carrier].fleet.as_ref().unwrap().strength,
            0.2
        );
        world.params.craft_rate = Params::default().craft_rate;
        world.refresh_fleets();
        assert_eq!(
            world.communities[carrier].fleet.as_ref().unwrap().strength,
            0.5
        );
        assert!(
            world
                .itinerary(
                    carrier,
                    port,
                    destination,
                    f32::INFINITY,
                    world.communities[carrier].size
                )
                .is_some()
        );
        world.communities[carrier].lands = world.communities[inland].lands.clone();
        world.refresh_fleets();
        assert!(world.communities[carrier].fleet.is_none());
        world.communities[carrier].lands = vec![port];
        world.refresh_fleets();
        let fleet = world.communities[carrier].fleet.as_ref().unwrap();
        assert_eq!(fleet.since, 8);
        assert_eq!(fleet.strength, 0.2);
    }

    #[test]
    fn colonies_fit_weak_fleets_but_whole_peoples_need_stronger_fleets() {
        let (mut world, carrier, _, destination) = shores();
        world.learn(carrier, Craft::Seafaring, None);
        let home = world.communities[carrier].home();
        for population in [1_000.0, 10_000.0, 100_000.0] {
            world.communities[carrier].size = population;
            world.communities[carrier].fleet.as_mut().unwrap().strength = 0.2;
            assert!(
                world
                    .itinerary(carrier, home, destination, f32::INFINITY, population * 0.5)
                    .is_some()
            );
            assert!(
                world
                    .itinerary(carrier, home, destination, f32::INFINITY, population)
                    .is_none()
            );
            world.communities[carrier].fleet.as_mut().unwrap().strength = 0.5;
            assert!(
                world
                    .itinerary(carrier, home, destination, f32::INFINITY, population)
                    .is_some()
            );
            assert!(
                world
                    .itinerary(carrier, home, destination, f32::INFINITY, population + 1.0)
                    .is_none()
            );
        }
    }

    #[test]
    fn fleets_require_held_ports_and_end_once_when_ports_are_lost() {
        let (mut world, carrier, inland, _) = shores();
        world.learn(inland, Craft::Seafaring, None);
        assert!(world.communities[inland].fleet.is_none());
        world.learn(carrier, Craft::Seafaring, None);
        assert_eq!(
            world.communities[carrier].fleet.as_ref().unwrap().strength,
            0.2
        );
        world.communities[carrier].lands = world.communities[inland].lands.clone();
        world.refresh_fleets();
        world.refresh_fleets();
        assert!(world.communities[carrier].fleet.is_none());
        assert_eq!(world.events.iter().filter(|(_, event)| {
            matches!(event, WorldEvent::FleetLost { community, .. } if *community == carrier)
        }).count(), 1);
    }

    #[test]
    fn static_society_does_not_acquire_fleets() {
        let (mut world, _, _, _) = shores();
        world.run(20);
        assert!(world.communities.iter().all(|c| c.fleet.is_none()));
        assert!(world.sea_routes.is_empty());
    }

    #[test]
    fn river_legs_follow_usable_drainage_edges() {
        let (world, carrier, _, _) = shores();
        let (from, to) = world
            .map
            .drainage
            .iter()
            .enumerate()
            .find_map(|(r, &next)| next.filter(|&n| world.map.river_edge(r, n)).map(|n| (r, n)))
            .expect("a flowing river edge");
        let route = world
            .itinerary(carrier, from, to, f32::INFINITY, 0.0)
            .unwrap();
        assert!(route.legs.iter().any(|leg| leg.by == TravelBy::River));
        assert!(route.carriers.is_empty());
        for edge in route.path.windows(2) {
            assert!(world.map.regions[edge[0]].neighbours.contains(&edge[1]));
        }
    }
}

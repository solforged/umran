//! Authored settlement decisions. A preview is a census and an allocation,
//! not a second simulation: committing recomputes it against the same world.

use crate::names::Naming;
use crate::world::{ContactKind, MIN_PEOPLE, Spatial, World, WorldEvent};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SettlementIntent {
    Partition,
    Settlers,
    Migration,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SettlementChoice {
    pub community: usize,
    pub intent: SettlementIntent,
    pub destination: usize,
    /// The fraction departing for a new settlement; ignored by other intents.
    pub share: f32,
    pub naming: Option<Naming>,
    pub intensity: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MapSize, Params, SoundProfile};

    #[test]
    fn disconnected_territory_needs_a_heart_in_each_component() {
        let mut world = World::with_map(7, Params::static_society(), MapSize::Medium);
        world.found(&SoundProfile::base(), 0.5, 0.5);
        let mut isolated = Vec::new();
        for (i, region) in world
            .map
            .regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.terrain.is_land())
        {
            if isolated.iter().all(|r| !region.neighbours.contains(r)) {
                isolated.push(i);
            }
            if isolated.len() == 3 {
                break;
            }
        }
        assert_eq!(isolated.len(), 3);
        world.communities[0].lands = isolated[..2].to_vec();
        assert_eq!(
            world.partition_lands(0, isolated[1]).unwrap(),
            vec![isolated[1]]
        );
        world.communities[0].lands.push(isolated[2]);
        let choice = SettlementChoice {
            community: 0,
            intent: SettlementIntent::Partition,
            destination: isolated[1],
            share: 0.5,
            naming: None,
            intensity: 0.5,
        };
        let before = world.communities.clone();
        assert!(world.settle(&choice).is_err());
        assert_eq!(world.communities, before);
    }

    #[test]
    fn settlement_requires_a_route_from_every_inhabited_land() {
        let mut world = World::with_map(7, Params::static_society(), MapSize::Medium);
        world.found(&SoundProfile::base(), 0.5, 0.5);
        let (a, destination) = world
            .map
            .regions
            .iter()
            .enumerate()
            .find_map(|(a, land)| {
                if !land.terrain.is_land() {
                    return None;
                }
                land.neighbours
                    .iter()
                    .copied()
                    .find(|&r| {
                        world.map.regions[r].terrain.is_land()
                            && world.map.distance(a, r) <= world.params.migration_reach
                    })
                    .map(|destination| (a, destination))
            })
            .expect("a walkable destination");
        let b = world
            .map
            .regions
            .iter()
            .enumerate()
            .find(|(_, land)| {
                land.terrain.is_land() && land.landmass != world.map.regions[a].landmass
            })
            .expect("a disconnected inhabited source")
            .0;
        world.communities[0].lands = vec![a, b];
        assert!(world.journey_to(0, destination).is_some());
        assert!(world.map.distance(b, destination).is_infinite());
        let choice = SettlementChoice {
            community: 0,
            intent: SettlementIntent::Migration,
            destination,
            share: 1.0,
            naming: None,
            intensity: 0.5,
        };
        assert_eq!(
            world.plan_settlement(&choice).unwrap_err(),
            format!(
                "Every inhabited land needs a route within {:.0} effort-km on foot or {:.0} by sea, and enough fleet capacity.",
                world.params.migration_reach,
                world.params.sea_migration_reach(world.map.geography)
            )
        );
    }

    #[test]
    fn sea_migration_and_settlers_use_their_versioned_reaches_without_extending_walks() {
        for geography in [
            crate::GeographyVersion::ContinentalV4,
            crate::GeographyVersion::ContinentalV5,
            crate::GeographyVersion::ContinentalV6,
        ] {
            let mut world =
                World::with_geography(7, Params::static_society(), MapSize::Medium, geography);
            let (home, destination) = (0..world.map.regions.len())
                .find_map(|home| {
                    if !world.map.regions[home]
                        .neighbours
                        .iter()
                        .any(|&r| world.map.regions[r].terrain.is_land())
                    {
                        return None;
                    }
                    world
                        .map
                        .voyage_row(home, 1200.0)
                        .iter()
                        .find(|&&(to, _)| {
                            world.map.overseas(home, to as usize)
                                && world.feeds(to as usize, crate::Livelihood::Farming)
                                    >= MIN_PEOPLE
                        })
                        .map(|&(to, _)| (home, to as usize))
                })
                .expect("separate habitable shores within colony reach");
            let c = world.found_seeded(
                &Naming::People,
                &SoundProfile::base(),
                7,
                0.5,
                0.5,
                Some(home),
                Some(crate::Livelihood::Farming),
                None,
            );
            world.communities[c].size = 2.0 * MIN_PEOPLE;
            world.learn(c, crate::Craft::Seafaring, None);
            world.communities[c].fleet.as_mut().unwrap().strength = 0.5;
            let journey = world.journey_to(c, destination).unwrap();
            assert!(journey.by_sea);
            world.params.migration_reach = journey.effort - 101.0;
            world.params.colony_reach = journey.effort;
            let mut choice = SettlementChoice {
                community: c,
                intent: SettlementIntent::Migration,
                destination,
                share: 1.0,
                naming: None,
                intensity: 0.5,
            };
            if geography.has_modern_continents() {
                assert!(world.plan_settlement(&choice).is_err());
                assert!(world.settle(&choice).is_err());
                assert_eq!(world.communities[c].home(), home);
                world.params.migration_reach += 2.0;
            }
            let plan = world.plan_settlement(&choice).unwrap();
            assert!(plan.routes[0].by_sea);
            assert!(plan.routes[0].effort > world.params.migration_reach);
            world.params.colony_reach = journey.effort - 1.0;
            assert!(world.plan_settlement(&choice).is_err());
            choice.destination = world.map.regions[home]
                .neighbours
                .iter()
                .copied()
                .find(|&r| world.journey_to(c, r).is_some_and(|j| !j.by_sea))
                .expect("a neighbouring walk");
            world.params.migration_reach = 0.0;
            world.params.colony_reach = 1200.0;
            assert!(world.plan_settlement(&choice).is_err());

            // Settlers still get the whole colony radius, not the migration cap.
            choice.destination = destination;
            choice.intent = SettlementIntent::Settlers;
            choice.share = 0.5;
            assert!(world.plan_settlement(&choice).is_ok());
            choice.intent = SettlementIntent::Migration;
            choice.share = 1.0;
            world.params.migration_reach = journey.effort;
            world.settle(&choice).unwrap();
            assert_eq!(world.communities[c].home(), destination);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Allocation {
    pub lands: Vec<usize>,
    pub population: f32,
    /// Includes residents in cities outside the people's holdings.
    pub presence: Vec<(usize, f32)>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SettlementRoute {
    pub from: usize,
    pub to: usize,
    pub population: f32,
    pub effort: f32,
    pub by_sea: bool,
    pub path: Vec<usize>,
    pub legs: Vec<crate::Leg>,
    #[serde(skip)]
    pub carriers: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SettlementPlan {
    pub choice: SettlementChoice,
    pub before: Allocation,
    pub remaining: Allocation,
    pub arriving: Allocation,
    pub routes: Vec<SettlementRoute>,
    pub inhabitants: Vec<(usize, f32)>,
    pub capacity: f32,
    pub room: f32,
    pub falling_states: Vec<usize>,
    pub notes: Vec<String>,
}

impl SettlementPlan {
    /// Historical apparatus uses the recorded outcome, not the preview warning.
    pub fn recorded_notes(&self) -> Vec<String> {
        self.describe(true)
    }

    fn describe(&self, recorded: bool) -> Vec<String> {
        let mut notes = Vec::new();
        match self.choice.intent {
            SettlementIntent::Partition => notes.push("The people divide where they live. No one travels: each holding stays connected to its chosen heart.".into()),
            SettlementIntent::Settlers => notes.push("The departing share comes from every inhabited land, including city residents. The new people's speech can diverge from its parent's.".into()),
            SettlementIntent::Migration => notes.push("Everyone moves, including city residents. The people keeps its identity and language; its old holdings are relinquished.".into()),
        }
        if !self.inhabitants.is_empty() {
            notes.push(if self.choice.intent == SettlementIntent::Partition {
                "Other peoples share these lands. Their homes stay unchanged; the new people can form its own ties with them."
            } else {
                "Other peoples already live here. Arrival can create contact and crowding; it does not remove the inhabitants."
            }.into());
        }
        if self.choice.intent != SettlementIntent::Partition && self.arriving.population > self.room
        {
            notes.push("The arrival exceeds the room this land offers. The people can settle, but crowding may bring later hardship or displacement.".into());
        }
        if !self.falling_states.is_empty() {
            notes.push(if recorded {
                if self.falling_states.len() == 1 {
                    "The rulers gave up their capital, and their state fell."
                } else {
                    "The rulers gave up their capitals, and their states fell."
                }
            } else {
                "The rulers would give up their capital. Their state will fall when this choice is made."
            }.into());
        }
        notes
    }
}

/// Evidence retained with the event even after territory and speech change.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SettlementRecord {
    pub plan: SettlementPlan,
    pub daughter: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SettlementOption {
    pub region: usize,
    pub reason: Option<String>,
    pub effort: Option<f32>,
}

pub(crate) struct Division {
    pub region: usize,
    pub lands: Vec<usize>,
    pub share: Option<f32>,
    pub by_sea: bool,
    pub record: bool,
    pub cause: Option<crate::Cause>,
}

/// A frontier with stable ties: the old heart wins an equal journey.
#[derive(Clone, Copy, PartialEq)]
struct Frontier {
    effort: f32,
    side: usize,
    region: usize,
}
impl Eq for Frontier {}
impl PartialOrd for Frontier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Frontier {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .effort
            .total_cmp(&self.effort)
            .then_with(|| other.side.cmp(&self.side))
            .then_with(|| other.region.cmp(&self.region))
    }
}

impl World {
    /// Two fronts travel only through this people's held lands. Assigning a
    /// region when its front arrives keeps each side connected to its heart.
    pub(crate) fn partition_lands(
        &self,
        community: usize,
        destination: usize,
    ) -> Result<Vec<usize>, String> {
        let people = &self.communities[community];
        if destination == people.home() {
            return Err("Choose another held land as the new people's heart.".into());
        }
        if !people.lands.contains(&destination) {
            return Err(
                "A territorial division needs a new heart within the lands they hold.".into(),
            );
        }
        let held: HashSet<usize> = people.lands.iter().copied().collect();
        let mut owner = vec![None; self.map.regions.len()];
        let mut frontier = BinaryHeap::from([
            Frontier {
                effort: 0.0,
                side: 0,
                region: people.home(),
            },
            Frontier {
                effort: 0.0,
                side: 1,
                region: destination,
            },
        ]);
        while let Some(Frontier {
            effort,
            side,
            region,
        }) = frontier.pop()
        {
            if owner[region].is_some() {
                continue;
            }
            owner[region] = Some(side);
            for &next in &self.map.regions[region].neighbours {
                if held.contains(&next) && owner[next].is_none() {
                    let edge = self
                        .map
                        .border_effort(region, next)
                        .expect("neighbours share an edge");
                    frontier.push(Frontier {
                        effort: effort + edge,
                        side,
                        region: next,
                    });
                }
            }
        }
        if people.lands.iter().any(|&r| owner[r].is_none()) {
            return Err("Some holdings are disconnected from both hearts. Choose hearts covering both areas, or move settlers first.".into());
        }
        let mut leaving: Vec<usize> = people
            .lands
            .iter()
            .copied()
            .filter(|&r| owner[r] == Some(1))
            .collect();
        leaving.sort_unstable_by_key(|&r| (r != destination, r));
        Ok(leaving)
    }

    pub fn settlement_options(
        &self,
        community: usize,
        intent: SettlementIntent,
        share: f32,
    ) -> Result<Vec<SettlementOption>, String> {
        let planner = Planner::new(self, community, intent, share)?;
        Ok(self
            .map
            .regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.terrain.is_land())
            .map(|(region, _)| {
                let choice = SettlementChoice {
                    community,
                    intent,
                    destination: region,
                    share,
                    naming: None,
                    intensity: 0.5,
                };
                match planner.plan(&choice, false) {
                    Ok(plan) => SettlementOption {
                        region,
                        reason: None,
                        effort: plan.routes.iter().map(|r| r.effort).max_by(f32::total_cmp),
                    },
                    Err(reason) => SettlementOption {
                        region,
                        reason: Some(reason),
                        effort: None,
                    },
                }
            })
            .collect())
    }

    pub fn plan_settlement(&self, choice: &SettlementChoice) -> Result<SettlementPlan, String> {
        Planner::new(self, choice.community, choice.intent, choice.share)?.plan(choice, true)
    }

    pub fn settle(&mut self, choice: &SettlementChoice) -> Result<Option<usize>, String> {
        let plan = self.plan_settlement(choice)?;
        let community = choice.community;
        for route in &plan.routes {
            self.use_itinerary(
                community,
                &crate::Itinerary {
                    path: route.path.clone(),
                    legs: route.legs.clone(),
                    carriers: route.carriers.clone(),
                },
                None,
            );
        }
        let daughter = match choice.intent {
            SettlementIntent::Migration => {
                self.leave_city_residence(community);
                self.communities[community].lands = plan.arriving.lands.clone();
                self.refresh_fleet(community, None);
                self.refresh_city_makeup();
                self.reconcile_contacts();
                self.refresh_places();
                None
            }
            SettlementIntent::Partition | SettlementIntent::Settlers => Some(self.divide(
                community,
                choice.naming.as_ref(),
                choice.intensity,
                Division {
                    region: choice.destination,
                    lands: plan.arriving.lands.clone(),
                    share: (choice.intent == SettlementIntent::Settlers).then_some(choice.share),
                    by_sea: plan.routes.iter().any(|r| r.by_sea),
                    record: false,
                    cause: None,
                },
                None,
            )),
        };
        let arriving = daughter.unwrap_or(community);
        // The choice records a deliberate contact intensity. No random draw is
        // needed, so an authored arrival cannot shift an automatic process.
        if choice.intensity > 0.0 {
            for &(other, _) in &plan.inhabitants {
                if other != arriving
                    && self.contact_eligible(arriving, other, ContactKind::Neighbours)
                    && !self.contacts.iter().any(|c| {
                        (c.a == arriving && c.b == other) || (c.b == arriving && c.a == other)
                    })
                {
                    self.link(arriving, other, choice.intensity, ContactKind::Neighbours);
                }
            }
        }
        self.events.push((
            self.generation,
            WorldEvent::Settlement(Box::new(SettlementRecord { plan, daughter })),
        ));
        Ok(daughter)
    }
}

struct SourceRoutes {
    source: usize,
    population: f32,
    routes: crate::fleets::Routes,
}

struct Planner<'a> {
    world: &'a World,
    community: usize,
    spatial: Spatial,
    sources: Vec<SourceRoutes>,
    walking_reach: f32,
    sea_reach: f32,
}

impl<'a> Planner<'a> {
    fn new(
        world: &'a World,
        community: usize,
        intent: SettlementIntent,
        share: f32,
    ) -> Result<Self, String> {
        let people = world
            .communities
            .get(community)
            .filter(|c| c.living())
            .ok_or("Choose a living people.")?;
        if intent == SettlementIntent::Settlers
            && (!share.is_finite() || share <= 0.0 || share >= 1.0)
        {
            return Err("The departing share must be between zero and one.".into());
        }
        let walking_reach = if intent == SettlementIntent::Migration {
            world.params.migration_reach
        } else {
            world.params.colony_reach
        };
        let sea_reach = if intent == SettlementIntent::Migration {
            world.params.sea_migration_reach(world.map.geography)
        } else {
            world.params.colony_reach
        };
        let reach = walking_reach.max(sea_reach);
        let sources = world
            .presence_iter(community)
            .filter(|(_, n)| *n > 0.0)
            .map(|(source, population)| {
                let mut routes = crate::fleets::Routes::new(world.map.regions.len());
                if intent != SettlementIntent::Partition {
                    // Reserve the whole travelling group, not a separate full fleet per source.
                    let passengers = people.size
                        * if intent == SettlementIntent::Migration {
                            1.0
                        } else {
                            share
                        };
                    routes.search(world, community, &[source], reach, passengers, None, true);
                }
                SourceRoutes {
                    source,
                    population,
                    routes,
                }
            })
            .collect();
        if people.lands.is_empty() {
            return Err("This people has no homeland.".into());
        }
        Ok(Self {
            world,
            community,
            spatial: world.spatial(),
            sources,
            walking_reach,
            sea_reach,
        })
    }

    fn plan(&self, choice: &SettlementChoice, paths: bool) -> Result<SettlementPlan, String> {
        let world = self.world;
        let people = &world.communities[self.community];
        let destination = choice.destination;
        if !world
            .map
            .regions
            .get(destination)
            .is_some_and(|r| r.terrain.is_land())
        {
            return Err("Choose a land on the chart.".into());
        }
        if let Some(naming) = &choice.naming {
            naming.validate()?;
        }
        if !choice.intensity.is_finite() || !(0.0..=1.0).contains(&choice.intensity) {
            return Err("Contact intensity must be between zero and one.".into());
        }
        let before = Allocation {
            lands: people.lands.clone(),
            population: people.size,
            presence: world.presence(self.community),
        };
        let (remaining, arriving, routes) = if choice.intent == SettlementIntent::Partition {
            let lands = world.partition_lands(self.community, destination)?;
            let presence: Vec<(usize, f32)> = before
                .presence
                .iter()
                .copied()
                .filter(|(r, _)| lands.contains(r))
                .collect();
            let population: f32 = presence.iter().map(|(_, n)| n).sum();
            let remaining = Allocation {
                lands: people
                    .lands
                    .iter()
                    .copied()
                    .filter(|r| !lands.contains(r))
                    .collect(),
                population: people.size - population,
                presence: before
                    .presence
                    .iter()
                    .copied()
                    .filter(|(r, _)| !lands.contains(r))
                    .collect(),
            };
            (
                remaining,
                Allocation {
                    lands,
                    population,
                    presence,
                },
                Vec::new(),
            )
        } else {
            if people.lands.contains(&destination) {
                return Err("Choose a new land beyond this people's present holdings.".into());
            }
            let share = if choice.intent == SettlementIntent::Migration {
                1.0
            } else {
                choice.share
            };
            let population = people.size * share;
            let mut routes = Vec::new();
            for source in &self.sources {
                let journey = source.routes.journey(destination)
                    .filter(|j| j.effort <= if j.by_sea {
                        self.sea_reach
                    } else {
                        self.walking_reach
                    })
                    .ok_or_else(|| format!(
                        "Every inhabited land needs a route within {:.0} effort-km on foot or {:.0} by sea, and enough fleet capacity.",
                        self.walking_reach, self.sea_reach))?;
                let (effort, by_sea) = (journey.effort, journey.by_sea);
                let itinerary = if paths {
                    source.routes.itinerary(world, destination)
                } else {
                    None
                };
                let (path, legs, carriers) = itinerary.map_or_else(
                    || (Vec::new(), Vec::new(), Vec::new()),
                    |i| (i.path, i.legs, i.carriers),
                );
                routes.push(SettlementRoute {
                    from: source.source,
                    to: destination,
                    population: source.population * share,
                    effort,
                    by_sea,
                    path,
                    legs,
                    carriers,
                });
            }
            let remaining = Allocation {
                lands: if share == 1.0 {
                    Vec::new()
                } else {
                    people.lands.clone()
                },
                population: people.size - population,
                presence: before
                    .presence
                    .iter()
                    .filter_map(|&(r, n)| (share < 1.0).then_some((r, n * (1.0 - share))))
                    .collect(),
            };
            (
                remaining,
                Allocation {
                    lands: vec![destination],
                    population,
                    presence: vec![(destination, population)],
                },
                routes,
            )
        };
        if arriving.population < MIN_PEOPLE
            || (choice.intent != SettlementIntent::Migration && remaining.population < MIN_PEOPLE)
        {
            return Err(format!(
                "Each people needs at least {MIN_PEOPLE:.0} souls to continue. Choose a different share or division."
            ));
        }
        let capacity = world.feeds(destination, people.livelihood);
        let remaining_here = if choice.intent == SettlementIntent::Settlers {
            remaining
                .presence
                .iter()
                .find(|(r, _)| *r == destination)
                .map_or(0.0, |(_, n)| *n)
        } else {
            0.0
        };
        let room = world.free_room(self.community, destination, &self.spatial) - remaining_here;
        if choice.intent != SettlementIntent::Partition && room < arriving.population / 2.0 {
            return Err(format!(
                "This land has room for about {:.0} more; at least {:.0} is needed for this arrival. Try a smaller group or another land.",
                room.max(0.0),
                arriving.population / 2.0
            ));
        }
        let retained = if choice.intent == SettlementIntent::Migration {
            &arriving.lands
        } else {
            &remaining.lands
        };
        let falling_states: Vec<usize> = world
            .states
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.fell.is_none() && s.rulers == self.community && !retained.contains(&s.capital)
            })
            .map(|(i, _)| i)
            .collect();
        let mut counts = BTreeMap::new();
        for &land in &arriving.lands {
            for &(c, n) in &self.spatial.dwellers[land] {
                if c != self.community {
                    *counts.entry(c).or_insert(0.0) += n;
                }
            }
        }
        if remaining_here > 0.0 {
            counts.insert(self.community, remaining_here);
        }
        let inhabitants: Vec<(usize, f32)> = counts.into_iter().collect();
        let mut plan = SettlementPlan {
            choice: choice.clone(),
            before,
            remaining,
            arriving,
            routes,
            inhabitants,
            capacity,
            room,
            falling_states,
            notes: Vec::new(),
        };
        plan.notes = plan.describe(false);
        Ok(plan)
    }
}

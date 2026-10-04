//! Shared weather histories and the food each land currently supplies.
//! Epochs are fictional regional shifts, not forecasts of Earth's climate.

use crate::geography::{Map, REFERENCE_AREA_KM2, Terrain};
use crate::livelihood::Livelihood;
use crate::rng::{index, key, stream};
use crate::{World, WorldEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClimateCause {
    Drought,
    ColdSpell,
    Recovery,
    LongDrying,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClimateChange {
    Onset,
    Worsening,
    Recovery,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ZoneClimate {
    pub epoch: u32,
    pub remaining: u32,
    /// Departures from each land's baseline, not absolute rainfall.
    pub wetness: f32,
    pub warmth: f32,
    pub target_wetness: f32,
    pub target_warmth: f32,
    pub cause: ClimateCause,
    /// Zero is quiet, one is stress, and two is severe stress.
    pub severity: u8,
    pub onset: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionClimate {
    pub wetness: f32,
    pub warmth: f32,
    pub vegetation: Terrain,
    /// Accumulated wet catchment area, in wet square kilometres.
    pub river_flow: f32,
    /// Capacity shares in Livelihood::ALL order, including physical area.
    pub feeding: [f32; 3],
    pub severe: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Climate {
    pub zones: Vec<ZoneClimate>,
    pub regions: Vec<RegionClimate>,
    pub seasons: crate::seasons::Seasons,
    /// Last climate exposure by people, so moving does not erase hardship.
    pub exposure: Vec<Option<u32>>,
    baseline_wetness: Vec<f32>,
    baseline_feeding: Vec<[f32; 3]>,
    pub(crate) flows: Vec<f32>,
    river_flowing: Vec<bool>,
}

impl Climate {
    pub fn new(seed: u64, map: &Map) -> Self {
        let baseline_wetness: Vec<f32> = map
            .regions
            .iter()
            .map(|r| {
                let rain = match r.terrain {
                    Terrain::Sea => 0.0,
                    Terrain::Desert => 0.18,
                    Terrain::Steppe => 0.38,
                    Terrain::Plains => 0.65,
                    Terrain::Forest => 0.8,
                    Terrain::Hills => 0.6,
                    Terrain::Mountains => 0.55,
                };
                if r.terrain.is_land() {
                    (rain + r.moisture * 0.1).clamp(0.1, 0.95)
                } else {
                    0.0
                }
            })
            .collect();
        let zones = map
            .climate_zones
            .iter()
            .enumerate()
            .map(|(id, _)| {
                let mut rng = stream(seed, &[key("climate history"), id as u64, 0]);
                ZoneClimate {
                    epoch: 0,
                    remaining: 8 + index(&mut rng, 12) as u32,
                    wetness: 0.0,
                    warmth: 0.0,
                    target_wetness: 0.0,
                    target_warmth: 0.0,
                    cause: ClimateCause::Recovery,
                    severity: 0,
                    onset: 0,
                }
            })
            .collect();
        let mut climate = Self {
            zones,
            regions: map
                .regions
                .iter()
                .map(|r| RegionClimate {
                    wetness: 0.0,
                    warmth: r.warmth,
                    vegetation: r.terrain,
                    river_flow: 0.0,
                    feeding: [0.0; 3],
                    severe: false,
                })
                .collect(),
            exposure: Vec::new(),
            seasons: crate::seasons::Seasons::new(map),
            baseline_wetness,
            baseline_feeding: vec![[0.0; 3]; map.regions.len()],
            flows: vec![0.0; map.regions.len()],
            river_flowing: vec![false; map.rivers.len()],
        };
        climate.cache(map);
        for (base, current) in climate.baseline_feeding.iter_mut().zip(&climate.regions) {
            *base = current.feeding;
        }
        for (id, river) in map.rivers.iter().enumerate() {
            climate.river_flowing[id] =
                climate.flows[*river.course.last().unwrap()] >= crate::geography::RIVER_TRAVEL_FLOW;
        }
        climate
    }

    fn epoch(seed: u64, id: usize, zone: &mut ZoneClimate) {
        zone.epoch += 1;
        let mut rng = stream(
            seed,
            &[key("climate history"), id as u64, u64::from(zone.epoch)],
        );
        let draw = rng.r#gen::<f32>();
        let (cause, wetness, warmth, duration) = if draw < 0.18 {
            (
                ClimateCause::Drought,
                rng.gen_range(-0.7..-0.4),
                rng.gen_range(0.0..0.1),
                6 + index(&mut rng, 9),
            )
        } else if draw < 0.31 {
            (
                ClimateCause::ColdSpell,
                rng.gen_range(-0.1..0.1),
                rng.gen_range(-0.45..-0.25),
                6 + index(&mut rng, 10),
            )
        } else if draw < 0.48 {
            let target = (zone.target_wetness - rng.gen_range(0.12..0.26)).clamp(-0.6, -0.2);
            (
                ClimateCause::LongDrying,
                target,
                rng.gen_range(-0.05..0.1),
                16 + index(&mut rng, 17),
            )
        } else {
            (
                ClimateCause::Recovery,
                rng.gen_range(-0.08..0.14),
                rng.gen_range(-0.06..0.1),
                10 + index(&mut rng, 15),
            )
        };
        zone.cause = cause;
        zone.target_wetness = wetness;
        zone.target_warmth = warmth;
        zone.remaining = duration as u32;
    }

    fn advance(
        &mut self,
        seed: u64,
        generation: u32,
        map: &Map,
    ) -> Vec<(usize, ClimateChange, ClimateCause)> {
        let mut changes = Vec::new();
        for (id, zone) in self.zones.iter_mut().enumerate() {
            if zone.remaining == 0 {
                Self::epoch(seed, id, zone);
            }
            zone.remaining -= 1;
            zone.wetness += (zone.target_wetness - zone.wetness) * 0.25;
            zone.warmth += (zone.target_warmth - zone.warmth) * 0.25;
            let severity = if zone.wetness <= -0.4 || zone.warmth <= -0.3 {
                2
            } else if zone.wetness <= -0.2 || zone.warmth <= -0.18 {
                1
            } else {
                0
            };
            if severity > zone.severity {
                let change = if zone.severity == 0 {
                    ClimateChange::Onset
                } else {
                    ClimateChange::Worsening
                };
                if zone.severity == 0 {
                    zone.onset = generation;
                }
                changes.push((id, change, zone.cause));
            } else if severity == 0 && zone.severity > 0 {
                changes.push((id, ClimateChange::Recovery, ClimateCause::Recovery));
            }
            zone.severity = severity;
        }
        self.cache(map);
        changes
    }

    fn cache(&mut self, map: &Map) {
        for (r, region) in map.regions.iter().enumerate() {
            let current = &mut self.regions[r];
            if !region.terrain.is_land() {
                continue;
            }
            let zone = &self.zones[region.climate_zone.expect("land has climate")];
            let lake = map.lake_regions[r].is_some();
            let exposure = if lake { 0.65 } else { 1.0 };
            current.wetness =
                (self.baseline_wetness[r] + zone.wetness * exposure).clamp(0.015, 1.0);
            current.warmth = (region.warmth + zone.warmth * exposure).clamp(0.0, 1.0);
            let rain = (current.wetness / self.baseline_wetness[r]).clamp(0.04, 1.35);
            self.flows[r] = map.runoff[r] * rain;
            current.vegetation = match region.terrain {
                Terrain::Plains | Terrain::Forest | Terrain::Steppe | Terrain::Desert => {
                    if current.wetness < 0.23 {
                        Terrain::Desert
                    } else if current.wetness < 0.48 {
                        Terrain::Steppe
                    } else if current.wetness > 0.76 {
                        Terrain::Forest
                    } else {
                        Terrain::Plains
                    }
                }
                terrain => terrain,
            };
        }
        for &r in &map.drainage_order {
            if let Some(downstream) = map.drainage[r]
                && map.regions[downstream].terrain.is_land()
            {
                self.flows[downstream] += self.flows[r];
            }
        }
        for (r, region) in map.regions.iter().enumerate() {
            let current = &mut self.regions[r];
            if !region.terrain.is_land() {
                continue;
            }
            current.river_flow = if map.river_regions[r].is_some() {
                self.flows[r]
            } else {
                0.0
            };
            let rain = (current.wetness / self.baseline_wetness[r]).clamp(0.04, 1.35);
            let cold = ((current.warmth + 0.2) / (region.warmth + 0.2)).clamp(0.2, 1.1);
            let river = current.river_flow / (current.river_flow + 30_000.0);
            // Lakes occupy parts of their regions; shore food attracts settlement
            // without turning existing land IDs into impassable water cells.
            let lake = if map.lake_regions[r].is_some() {
                0.35
            } else {
                0.0
            };
            let relief = match region.terrain {
                Terrain::Mountains => 0.04,
                Terrain::Hills => 0.22,
                _ => 1.0,
            };
            current.feeding = Livelihood::ALL.map(|livelihood| {
                let (weather, water) = match livelihood {
                    Livelihood::Farming => (rain * rain * cold * cold, 0.9),
                    Livelihood::Herding => (rain * cold, 0.065),
                    Livelihood::Foraging => (rain.sqrt() * cold, 0.025),
                };
                (livelihood.feeds(region.terrain) * weather
                    + (river + lake).min(1.0) * water * relief * cold)
                    * region.area_km2
                    / REFERENCE_AREA_KM2
            });
            let base = self.baseline_feeding[r][Livelihood::Farming as usize];
            current.severe =
                base > 0.0 && current.feeding[Livelihood::Farming as usize] < base * 0.6;
        }
    }
}

impl World {
    /// Recorded climate currently reducing this livelihood's food, not baseline aridity.
    pub(crate) fn feeding_cause(
        &self,
        region: usize,
        livelihood: crate::Livelihood,
    ) -> Option<crate::Cause> {
        self.seasonal_feeding_cause(region, livelihood)
            .or_else(|| self.epoch_feeding_cause(region, livelihood))
    }

    fn unseasonal_feeding(&self, region: usize, livelihood: crate::Livelihood) -> f32 {
        if self.climate.seasons.impacts[region].is_some() {
            self.climate.seasons.feeding[region][livelihood as usize]
        } else {
            self.climate.regions[region].feeding[livelihood as usize]
        }
    }

    fn epoch_feeding_cause(
        &self,
        region: usize,
        livelihood: crate::Livelihood,
    ) -> Option<crate::Cause> {
        if self.unseasonal_feeding(region, livelihood)
            >= self.climate.baseline_feeding[region][livelihood as usize]
        {
            return None;
        }
        let zone = self.map.regions[region].climate_zone?;
        self.triggers.climate.get(&zone).map(|&event| crate::Cause {
            event,
            mechanism: crate::Mechanism::Climate,
        })
    }

    pub(crate) fn adoption_cause(
        &self,
        community: usize,
        own: crate::Livelihood,
        next: crate::Livelihood,
    ) -> Option<crate::Cause> {
        let lands = &self.communities[community].lands;
        let unseasonal = |l: crate::Livelihood| {
            lands
                .iter()
                .map(|&r| self.unseasonal_feeding(r, l))
                .sum::<f32>()
        };
        if unseasonal(next) < crate::world::ADOPT_GAIN * unseasonal(own)
            && let Some(cause) = lands
                .iter()
                .find_map(|&r| self.seasonal_feeding_cause(r, own))
        {
            return Some(cause);
        }
        let baseline = |l: crate::Livelihood| {
            lands
                .iter()
                .map(|&r| self.climate.baseline_feeding[r][l as usize])
                .sum::<f32>()
        };
        if baseline(next) >= crate::world::ADOPT_GAIN * baseline(own) {
            return None;
        }
        lands.iter().find_map(|&r| self.epoch_feeding_cause(r, own))
    }

    pub(crate) fn climate_challenged(&self, community: usize) -> bool {
        let exposed = |c: usize| {
            self.communities[c]
                .lands
                .iter()
                .any(|&r| self.climate.regions[r].severe)
                || self
                    .climate
                    .exposure
                    .get(c)
                    .copied()
                    .flatten()
                    .is_some_and(|g| self.generation.saturating_sub(g) <= 6)
        };
        exposed(community)
            || self
                .rules(community)
                .is_some_and(|s| self.states[s].subjects().any(exposed))
    }

    /// Returns whether changed valley routes need contact reconciliation.
    pub(crate) fn advance_climate(&mut self) -> bool {
        if !self.params.climate_enabled {
            // Restore unmodified yields before this year's seasonal sample,
            // including the first step after seasonal weather is switched off.
            if self.params.seasons_enabled
                || self.climate.seasons.impacts.iter().any(Option::is_some)
            {
                self.climate.cache(&self.map);
            }
            return false;
        }
        let changes = self.climate.advance(self.seed, self.generation, &self.map);
        self.climate.exposure.resize(self.communities.len(), None);
        for (zone, change, cause) in changes {
            let lands = self.map.climate_zones[zone].regions.clone();
            let mut peoples: Vec<usize> = self
                .living()
                .filter(|&c| {
                    self.communities[c]
                        .lands
                        .iter()
                        .any(|r| lands.binary_search(r).is_ok())
                })
                .collect();
            for state in self.states.iter().filter(|s| s.standing()) {
                if state.subjects().any(|c| peoples.contains(&c))
                    && !peoples.contains(&state.rulers)
                {
                    peoples.push(state.rulers);
                }
            }
            peoples.sort_unstable();
            peoples.dedup();
            if change != ClimateChange::Recovery {
                for &c in &peoples {
                    self.climate.exposure[c] = Some(self.generation);
                    self.communities[c].ethos_challenged = self.generation;
                    self.hardship_ethos(c, true);
                }
            }
            let current = &self.climate.zones[zone];
            self.record_event(WorldEvent::Climate {
                zone,
                cause,
                change,
                severity: current.severity,
                wetness: current.wetness,
                warmth: current.warmth,
                lands,
                peoples,
            });
        }
        for c in 0..self.communities.len() {
            if self.communities[c].living()
                && self.communities[c]
                    .lands
                    .iter()
                    .any(|&r| self.climate.regions[r].severe)
            {
                self.climate.exposure[c] = Some(self.generation);
                self.communities[c].ethos_challenged = self.generation;
                if let Some(event) = self.communities[c]
                    .lands
                    .iter()
                    .filter(|&&r| self.climate.regions[r].severe)
                    .filter_map(|&r| self.map.regions[r].climate_zone)
                    .find_map(|z| self.triggers.climate.get(&z).copied())
                {
                    self.triggers.exposure.insert(c, event);
                } else {
                    self.triggers.exposure.remove(&c);
                }
            }
        }
        for (id, river) in self.map.rivers.iter().enumerate() {
            let flowing = self.climate.flows[*river.course.last().unwrap()]
                >= crate::geography::RIVER_TRAVEL_FLOW;
            if flowing == self.climate.river_flowing[id] {
                continue;
            }
            self.climate.river_flowing[id] = flowing;
            let lands = river.course.clone();
            let peoples = self
                .living()
                .filter(|&c| self.communities[c].lands.iter().any(|r| lands.contains(r)))
                .collect();
            let cause = if flowing {
                ClimateCause::Recovery
            } else {
                let dry = river
                    .catchment
                    .iter()
                    .copied()
                    .min_by(|&a, &b| {
                        self.climate.zones[self.map.regions[a].climate_zone.unwrap()]
                            .wetness
                            .total_cmp(
                                &self.climate.zones[self.map.regions[b].climate_zone.unwrap()]
                                    .wetness,
                            )
                    })
                    .unwrap();
                self.climate.zones[self.map.regions[dry].climate_zone.unwrap()].cause
            };
            self.events.push((
                self.generation,
                WorldEvent::RiverFlow {
                    river: id,
                    flowing,
                    cause,
                    lands,
                    peoples,
                },
            ));
        }
        // Only threshold crossings need new sparse rows, not every rain change.
        if self.map.valley_flows_changed(&self.climate.flows) {
            std::sync::Arc::make_mut(&mut self.map).set_valley_flows(&self.climate.flows);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MapSize, Naming, Params, SoundProfile};

    #[test]
    fn legacy_maps_match_pre_lake_drainage_rivers_and_climate() {
        use crate::geography::GeographyVersion;
        for geography in [
            GeographyVersion::SphericalV1,
            GeographyVersion::ContinentalV2,
            GeographyVersion::ContinentalV3,
        ] {
            for size in [
                MapSize::Small,
                MapSize::Medium,
                MapSize::Large,
                MapSize::Vast,
            ] {
                for seed in [0, 7, 21] {
                    let map = Map::generate_with_version(seed, size, geography);
                    assert!(map.lakes.is_empty());
                    assert!(map.lake_regions.iter().all(Option::is_none));
                    assert!(map.rivers.iter().all(|river| river.channel.is_empty()));

                    // The pre-lake construction: priority flood directly into
                    // runoff accumulation and courses, with no basin rerouting.
                    let (drainage, order) = crate::rivers::drainage(&map.regions);
                    let runoff: Vec<f32> = map
                        .regions
                        .iter()
                        .enumerate()
                        .map(|(r, region)| {
                            if !region.terrain.is_land() {
                                return 0.0;
                            }
                            let mut rng = stream(seed, &[key("river runoff"), r as u64]);
                            region.area_km2
                                * (0.25 + 1.5 * region.moisture.clamp(0.0, 1.0))
                                * rng.gen_range(0.85..1.15)
                        })
                        .collect();
                    let mut flows = runoff.clone();
                    for &r in &order {
                        if let Some(n) = drainage[r] {
                            flows[n] += flows[r];
                        }
                    }
                    let (rivers, owners) =
                        crate::rivers::courses(&map.regions, &drainage, &order, &flows);
                    assert_eq!(map.drainage, drainage, "{geography:?} {size:?} {seed}");
                    assert_eq!(map.drainage_order, order);
                    assert_eq!(map.runoff, runoff);
                    assert_eq!(map.rivers, rivers);
                    assert_eq!(map.river_regions, owners);
                    let mut before = map.clone();
                    before.drainage = drainage;
                    before.drainage_order = order;
                    before.runoff = runoff;
                    before.rivers = rivers;
                    before.river_regions = owners;
                    let mut climate = Climate::new(seed, &map);
                    assert_eq!(climate, Climate::new(seed, &before));

                    // Independently retain the old lake-free cache arithmetic,
                    // comparing bits both at baseline and during climate epochs.
                    for generation in 0..40 {
                        if generation > 0 {
                            climate.advance(seed, generation, &map);
                        }
                        let mut flows = vec![0.0; before.regions.len()];
                        for (r, region) in before.regions.iter().enumerate() {
                            if !region.terrain.is_land() {
                                continue;
                            }
                            let zone = &climate.zones[region.climate_zone.unwrap()];
                            let base = climate.baseline_wetness[r];
                            let wetness = (base + zone.wetness).clamp(0.015, 1.0);
                            let warmth = (region.warmth + zone.warmth).clamp(0.0, 1.0);
                            assert_eq!(climate.regions[r].wetness.to_bits(), wetness.to_bits());
                            assert_eq!(climate.regions[r].warmth.to_bits(), warmth.to_bits());
                            flows[r] = before.runoff[r] * (wetness / base).clamp(0.04, 1.35);
                        }
                        for &r in &before.drainage_order {
                            if let Some(n) = before.drainage[r]
                                && before.regions[n].terrain.is_land()
                            {
                                flows[n] += flows[r];
                            }
                        }
                        for (r, region) in before.regions.iter().enumerate() {
                            if !region.terrain.is_land() {
                                continue;
                            }
                            let current = &climate.regions[r];
                            let flow = if before.river_regions[r].is_some() {
                                flows[r]
                            } else {
                                0.0
                            };
                            assert_eq!(current.river_flow.to_bits(), flow.to_bits());
                            let river = flow / (flow + 30_000.0);
                            let rain =
                                (current.wetness / climate.baseline_wetness[r]).clamp(0.04, 1.35);
                            let cold =
                                ((current.warmth + 0.2) / (region.warmth + 0.2)).clamp(0.2, 1.1);
                            let relief = match region.terrain {
                                Terrain::Mountains => 0.04,
                                Terrain::Hills => 0.22,
                                _ => 1.0,
                            };
                            let feeding = Livelihood::ALL.map(|livelihood| {
                                let (weather, water) = match livelihood {
                                    Livelihood::Farming => (rain * rain * cold * cold, 0.9),
                                    Livelihood::Herding => (rain * cold, 0.065),
                                    Livelihood::Foraging => (rain.sqrt() * cold, 0.025),
                                };
                                (livelihood.feeds(region.terrain) * weather
                                    + river * water * relief * cold)
                                    * region.area_km2
                                    / REFERENCE_AREA_KM2
                            });
                            assert_eq!(
                                current.feeding.map(f32::to_bits),
                                feeding.map(f32::to_bits)
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn static_society_freezes_weather_flow_and_feeding() {
        let mut world = World::new(17, Params::static_society());
        world.found(&SoundProfile::base(), 0.5, 0.5);
        let before = world.climate.clone();
        world.run(80);
        assert_eq!(world.climate, before);
        assert!(!world.events.iter().any(|(_, event)| matches!(
            event,
            WorldEvent::Climate { .. } | WorldEvent::RiverFlow { .. }
        )));
    }

    #[test]
    fn early_settlers_prefer_valleys_over_dry_interiors() {
        let (mut valley, mut dry, mut valley_available, mut dry_available) = (0, 0, 0, 0);
        for seed in 0..40 {
            let mut world = World::with_map(seed, Params::static_society(), MapSize::Medium);
            let is_dry = |r: usize, map: &Map| {
                map.river_regions[r].is_none()
                    && matches!(map.regions[r].terrain, Terrain::Steppe | Terrain::Desert)
                    && !map.coastal(r)
                    && !map.island(r)
            };
            valley_available += world
                .map
                .regions
                .iter()
                .enumerate()
                .filter(|(r, _)| world.map.river_regions[*r].is_some() && !world.map.island(*r))
                .count();
            dry_available += world
                .map
                .regions
                .iter()
                .enumerate()
                .filter(|(r, _)| is_dry(*r, &world.map))
                .count();
            for _ in 0..6 {
                let c = world.found(&SoundProfile::base(), 0.5, 0.5);
                let home = world.communities[c].home();
                valley += usize::from(world.map.river_regions[home].is_some());
                dry += usize::from(is_dry(home, &world.map));
            }
        }
        let valley_rate = valley as f32 / valley_available as f32;
        let dry_rate = dry as f32 / dry_available as f32;
        assert!(
            valley_rate > dry_rate * 1.5,
            "per available land: valley {valley}/{valley_available}, dry {dry}/{dry_available}"
        );
    }

    #[test]
    fn drying_lowers_food_and_drives_remembered_hardship_and_migration() {
        let (mut cohort, mut dry_moves, mut quiet_moves, mut exposed) = (0, 0, 0, 0);
        for seed in 0..40 {
            let params = Params {
                climate_enabled: true,
                growth_rate: 0.07,
                migration_rate: 0.02,
                ..Params::static_society()
            };
            let mut dry = World::new(seed, params);
            let home = (0..dry.map.regions.len()).find(|&r| {
                let region = &dry.map.regions[r];
                region.terrain == Terrain::Plains
                    && dry.map.river_regions[r].is_none()
                    && region.neighbours.iter().any(|&n| {
                        dry.map.regions[n].terrain == Terrain::Plains
                            && dry.map.regions[n].climate_zone != region.climate_zone
                    })
            });
            let Some(home) = home else { continue };
            cohort += 1;
            let c = dry.found_seeded(
                &Naming::People,
                &SoundProfile::base(),
                seed,
                0.5,
                0.5,
                Some(home),
                Some(Livelihood::Farming),
                None,
            );
            let fed = dry.feeds(home, Livelihood::Farming);
            dry.communities[c].size = fed * 0.85;
            for zone in &mut dry.climate.zones {
                zone.remaining = 100;
            }
            let mut quiet = dry.clone();
            let zone = dry.map.regions[home].climate_zone.unwrap();
            dry.climate.zones[zone].target_wetness = -0.65;
            dry.climate.zones[zone].cause = ClimateCause::Drought;
            for _ in 0..40 {
                let from = dry.communities[c].home();
                let challenged = dry.climate_challenged(c);
                dry.step();
                if dry.communities[c].home() != from && challenged {
                    assert!(
                        dry.climate_challenged(c),
                        "moving must not erase climate exposure"
                    );
                }
            }
            quiet.run(40);
            assert!(dry.feeds(home, Livelihood::Farming) < fed * 0.2);
            dry_moves += usize::from(dry.events.iter().any(|(_, e)| {
                matches!(
                    e, WorldEvent::Migrated { community, .. } if *community == c
                )
            }));
            quiet_moves += usize::from(quiet.events.iter().any(|(_, e)| {
                matches!(
                    e, WorldEvent::Migrated { community, .. } if *community == c
                )
            }));
            exposed += usize::from(dry.events.iter().any(|(_, e)| matches!(
                e, WorldEvent::Climate { change: ClimateChange::Onset, peoples, .. } if peoples.contains(&c)
            )));
            if let Some((generation, _)) = dry.events.iter().find(|(_, e)| {
                matches!(
                    e, WorldEvent::Migrated { community, .. } if *community == c
                )
            }) {
                assert!(
                    dry.climate.exposure[c].is_some_and(|g| g + 6 >= *generation),
                    "migrants must carry their exposure, seed {seed}"
                );
            }
            let newcomer = dry.found_seeded(
                &Naming::People,
                &SoundProfile::base(),
                seed + 100,
                0.5,
                0.5,
                Some(home),
                None,
                None,
            );
            assert_eq!(
                dry.communities[newcomer].livelihood,
                Livelihood::Herding,
                "a late founder must use the dry land's current food"
            );
        }
        assert!(cohort >= 20, "only {cohort} boundary refuges");
        assert!(
            dry_moves >= cohort / 2 && dry_moves > quiet_moves + cohort / 5,
            "dry {dry_moves}, quiet {quiet_moves}, cohort {cohort}"
        );
        assert!(exposed >= cohort * 3 / 4, "exposed {exposed}/{cohort}");
    }

    #[test]
    fn drying_a_valley_ends_inaccessible_trade_before_words_are_borrowed() {
        let mut world = World::new(
            0,
            Params {
                climate_enabled: true,
                sound_change_rate: 0.0,
                innovation_rate: 0.0,
                loan_rate: 0.0,
                ..Params::static_society()
            },
        );
        let course = &world
            .map
            .rivers
            .iter()
            .find(|r| r.course.len() >= 2)
            .unwrap()
            .course;
        let (a, b) = (course[0], course[1]);
        for (seed, region) in [(10, a), (20, b)] {
            world.found_seeded(
                &Naming::People,
                &SoundProfile::base(),
                seed,
                0.5,
                1.0,
                Some(region),
                Some(Livelihood::Farming),
                None,
            );
        }
        world.params.trade_reach = world.map.distance(a, b) * 1.001;
        world.connect(0, 1, 1.0, crate::ContactKind::Trade).unwrap();
        for zone in &mut world.climate.zones {
            zone.remaining = 100;
            zone.target_wetness = -0.7;
            zone.cause = ClimateCause::Drought;
        }
        let borrowed = |world: &World| {
            world.varieties.iter().any(|v| {
                v.lexicon.lexemes.iter().any(|l| {
                    l.born == world.generation && matches!(l.origin, crate::Origin::Borrowed { .. })
                })
            })
        };
        for _ in 0..20 {
            let mut preview = world.clone();
            preview.step();
            if preview.map.distance(a, b) > world.params.trade_reach {
                let mut quiet = world.clone();
                quiet.params.climate_enabled = false;
                quiet.params.loan_rate = 100.0;
                quiet.step();
                assert!(
                    borrowed(&quiet),
                    "reachable trade must carry a loan in this fixture"
                );
                world.params.loan_rate = 100.0;
                world.step();
                assert!(world.contacts.is_empty());
                assert!(
                    !borrowed(&world),
                    "a dried route cannot carry loans during its closing generation"
                );
                return;
            }
            world = preview;
        }
        panic!("the drought did not close the valley route");
    }
    #[test]
    fn lake_shores_buffer_weather_and_add_food_without_advancing_static_climate() {
        let map = Map::generate(5, crate::MapSize::Medium);
        let region = map
            .regions
            .iter()
            .position(|r| r.terrain == Terrain::Plains)
            .unwrap();
        let mut shore = map.clone();
        shore.lake_regions[region] = Some(0);
        let mut inland = map.clone();
        inland.lake_regions[region] = None;
        let mut buffered = Climate::new(5, &shore);
        let mut exposed = Climate::new(5, &inland);
        assert!(buffered.regions[region].feeding[0] > exposed.regions[region].feeding[0]);
        for climate in [&mut buffered, &mut exposed] {
            let zone = map.regions[region].climate_zone.unwrap();
            climate.zones[zone].wetness = -0.4;
            climate.zones[zone].warmth = -0.3;
        }
        buffered.cache(&shore);
        exposed.cache(&inland);
        assert!(buffered.regions[region].wetness > exposed.regions[region].wetness);
        assert!(buffered.regions[region].warmth > exposed.regions[region].warmth);
        assert!(buffered.regions[region].feeding[0] > exposed.regions[region].feeding[0]);
        let mut world = World::new(5, crate::Params::static_society());
        let before = world.climate.clone();
        world.run(8);
        assert_eq!(world.climate, before);
    }
}

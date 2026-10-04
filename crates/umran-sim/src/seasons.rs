//! A representative seasonal year within each generation, not monthly ticks.
//! Food bottlenecks from that year's hazards last until the next weather sample.
use crate::geography::{Map, Region, Terrain};
use crate::rng::{key, stream};
use crate::{Cause, Livelihood, Mechanism, World, WorldEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WetSeason {
    None,
    Summer,
    Winter,
    Monsoon,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeasonalProfile {
    /// Normalized summer-to-winter thermal range, not degrees Celsius.
    pub amplitude: f32,
    pub wet: WetSeason,
    /// Low-relief river land susceptible to damaging overbank floods.
    pub floods: bool,
}

impl SeasonalProfile {
    fn of(region: &Region, coast_km: f32, land_area: f32, river: bool) -> Self {
        if !region.terrain.is_land() {
            return Self {
                amplitude: 0.0,
                wet: WetSeason::None,
                floods: false,
            };
        }
        let latitude = region.position[2].abs() as f32;
        let interior = coast_km / (coast_km + 500.0);
        let amplitude =
            (0.06 + 0.52 * latitude + 0.24 * interior + 0.12 * region.elevation.max(0.0)).min(0.95);
        let wet = if (0.17..0.58).contains(&latitude)
            && land_area >= 500_000.0
            && coast_km < 1_500.0
            && region.terrain != Terrain::Desert
        {
            WetSeason::Monsoon
        } else if (0.4..0.72).contains(&latitude) && interior < 0.35 {
            WetSeason::Winter
        } else if latitude < 0.12 || region.terrain == Terrain::Desert {
            WetSeason::None
        } else {
            WetSeason::Summer
        };
        Self {
            amplitude,
            wet,
            floods: river && !matches!(region.terrain, Terrain::Hills | Terrain::Mountains),
        }
    }

    /// Calendar phase in [0, 1): northern summer at 0.5, southern at 0.
    pub fn warmth_at(self, mean: f32, north: bool, phase: f32) -> f32 {
        let summer = -crate::math::cos(f64::from(phase) * std::f64::consts::TAU) as f32;
        (mean + self.amplitude * 0.5 * if north { summer } else { -summer }).clamp(0.0, 1.0)
    }

    /// Relative seasonal rain: the monsoon has a stronger wet/dry contrast.
    pub fn rain_at(self, north: bool, phase: f32) -> f32 {
        let summer = -crate::math::cos(f64::from(phase) * std::f64::consts::TAU) as f32;
        let summer = if north { summer } else { -summer };
        match self.wet {
            WetSeason::None => 1.0,
            WetSeason::Summer => 1.0 + 0.4 * summer,
            WetSeason::Winter => 1.0 - 0.4 * summer,
            WetSeason::Monsoon => 1.0 + 0.85 * summer,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SeasonalHazard {
    Drought,
    HardWinter,
    Flood,
}

impl SeasonalHazard {
    pub fn id(self) -> &'static str {
        match self {
            Self::Drought => "drought",
            Self::HardWinter => "hard-winter",
            Self::Flood => "flood",
        }
    }

    fn losses(self, severity: f32) -> [f32; 3] {
        // Livelihood::ALL order. Crops are less buffered than mixed gathering.
        Livelihood::ALL.map(|l| {
            severity
                * match (self, l) {
                    (Self::Drought, Livelihood::Farming) => 0.8,
                    (Self::Drought, Livelihood::Herding) => 0.45,
                    (Self::Drought, Livelihood::Foraging) => 0.3,
                    (Self::HardWinter, Livelihood::Farming) => 0.65,
                    (Self::HardWinter, Livelihood::Herding) => 0.5,
                    (Self::HardWinter, Livelihood::Foraging) => 0.35,
                    (Self::Flood, Livelihood::Farming) => 0.6,
                    (Self::Flood, Livelihood::Herding) => 0.2,
                    (Self::Flood, Livelihood::Foraging) => 0.15,
                }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeasonalImpact {
    pub hazard: SeasonalHazard,
    pub severity: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Seasons {
    pub profiles: Vec<SeasonalProfile>,
    /// Strongest hazard in the sampled year, including unoccupied lands.
    pub impacts: Vec<Option<SeasonalImpact>>,
    /// Feeding before this year's hazard, for counterfactual cause checks.
    pub(crate) feeding: Vec<[f32; 3]>,
    /// Current hazard event by region; cleared every sampled year.
    events: Vec<Option<usize>>,
    /// Upstream seasonal concentration, weighted by the runoff it supplies.
    flood_pulse: Vec<f32>,
    /// Sampled hazard land-years by kind (drought, hard winter, flood),
    /// including unrecorded ones on empty land or below severe.
    pub struck: [u64; 3],
}

impl Seasons {
    pub fn new(map: &Map) -> Self {
        // Multi-source spherical land distance, in metres. No chart seam or
        // terrain travel penalty belongs in a continentality measure.
        let mut distance = vec![u64::MAX; map.regions.len()];
        let mut queue = BinaryHeap::new();
        for (r, d) in distance.iter_mut().enumerate() {
            if map.coastal(r) {
                *d = 0;
                queue.push(Reverse((0, r)));
            }
        }
        while let Some(Reverse((d, r))) = queue.pop() {
            if d != distance[r] {
                continue;
            }
            for &n in &map.regions[r].neighbours {
                if !map.regions[n].terrain.is_land() {
                    continue;
                }
                let edge = (crate::sphere::angle(map.regions[r].position, map.regions[n].position)
                    * f64::from(map.radius_km)
                    * 1_000.0) as u64;
                let next = d + edge;
                if next < distance[n] {
                    distance[n] = next;
                    queue.push(Reverse((next, n)));
                }
            }
        }
        let areas: Vec<f32> = map
            .landmasses
            .iter()
            .map(|land| land.regions.iter().map(|&r| map.regions[r].area_km2).sum())
            .collect();
        let profiles: Vec<_> = map
            .regions
            .iter()
            .enumerate()
            .map(|(r, region)| {
                SeasonalProfile::of(
                    region,
                    distance[r] as f32 / 1_000.0,
                    region.landmass.map_or(0.0, |l| areas[l]),
                    map.river_regions[r].is_some(),
                )
            })
            .collect();
        let mut water = map.runoff.clone();
        let mut flood_pulse: Vec<f32> = profiles
            .iter()
            .enumerate()
            .map(|(r, p)| {
                map.runoff[r]
                    * match p.wet {
                        WetSeason::None => 0.1,
                        WetSeason::Summer | WetSeason::Winter => 0.4,
                        WetSeason::Monsoon => 0.85,
                    }
            })
            .collect();
        for &r in &map.drainage_order {
            if let Some(n) = map.drainage[r] {
                water[n] += water[r];
                flood_pulse[n] += flood_pulse[r];
            }
        }
        for (pulse, water) in flood_pulse.iter_mut().zip(water) {
            *pulse /= water.max(1.0);
        }
        Self {
            profiles,
            impacts: vec![None; map.regions.len()],
            feeding: vec![[0.0; 3]; map.regions.len()],
            events: vec![None; map.regions.len()],
            flood_pulse,
            struck: [0; 3],
        }
    }

    fn sample(&mut self, seed: u64, generation: u32, map: &Map, climate: &[crate::RegionClimate]) {
        self.impacts.fill(None);
        self.events.fill(None);
        for (zone, lands) in map.climate_zones.iter().enumerate() {
            let mut rng = stream(
                seed,
                &[key("seasonal weather"), u64::from(generation), zone as u64],
            );
            // Shared anomalies, local susceptibility. Always draw in this order.
            let draws: [f32; 3] = std::array::from_fn(|_| rng.r#gen());
            for &r in &lands.regions {
                let p = self.profiles[r];
                let current = &climate[r];
                let winter = (current.warmth - p.amplitude * 0.5).max(0.0);
                let drought = (0.018
                    + (1.0 - current.wetness) * 0.065
                    + if p.wet == WetSeason::Monsoon {
                        0.025
                    } else {
                        0.0
                    })
                .min(0.12);
                let cold = if winter < 0.38 {
                    0.015 + (0.38 - winter) * 0.18
                } else {
                    0.0
                };
                let flood = if p.floods && current.river_flow > 0.0 {
                    (0.012 + self.flood_pulse[r] * 0.06) * current.wetness.clamp(0.15, 1.0)
                } else {
                    0.0
                };
                for (hazard, draw, chance) in [
                    (SeasonalHazard::Drought, draws[0], drought),
                    (SeasonalHazard::HardWinter, draws[1], cold),
                    (SeasonalHazard::Flood, draws[2], flood),
                ] {
                    if draw < chance {
                        let severity = 0.4 + 0.6 * (1.0 - draw / chance);
                        let impact = SeasonalImpact { hazard, severity };
                        if self.impacts[r].is_none_or(|old| {
                            hazard.losses(severity)[Livelihood::Farming as usize]
                                > old.hazard.losses(old.severity)[Livelihood::Farming as usize]
                        }) {
                            self.impacts[r] = Some(impact);
                        }
                    }
                }
            }
        }
    }
}

impl World {
    pub(crate) fn advance_seasons(&mut self) {
        if !self.params.seasons_enabled {
            self.climate.seasons.impacts.fill(None);
            self.climate.seasons.events.fill(None);
            return;
        }
        self.climate
            .seasons
            .sample(self.seed, self.generation, &self.map, &self.climate.regions);
        self.climate.exposure.resize(self.communities.len(), None);
        for r in 0..self.map.regions.len() {
            self.climate.seasons.feeding[r] = self.climate.regions[r].feeding;
            if let Some(impact) = self.climate.seasons.impacts[r] {
                self.climate.seasons.struck[impact.hazard as usize] += 1;
                self.apply_seasonal_hazard(r, impact);
            }
        }
    }

    fn apply_seasonal_hazard(&mut self, region: usize, impact: SeasonalImpact) {
        let losses = impact.hazard.losses(impact.severity);
        for (food, loss) in self.climate.regions[region].feeding.iter_mut().zip(losses) {
            *food *= 1.0 - loss;
        }
        let severe = losses[Livelihood::Farming as usize] >= 0.4;
        self.climate.regions[region].severe |= severe;
        let peoples: Vec<_> = self
            .living()
            .filter(|&c| self.communities[c].lands.contains(&region))
            .collect();
        // Only a severe hazard where people live enters the record: it is the
        // only kind a response can cite, and recording every dry year on
        // empty land would bury the chronicle.
        if !severe || peoples.is_empty() {
            return;
        }
        for &c in &peoples {
            self.climate.exposure[c] = Some(self.generation);
            self.communities[c].ethos_challenged = self.generation;
            self.hardship_ethos(c, true);
        }
        let event = self.events.len();
        for &c in &peoples {
            self.triggers.exposure.insert(c, event);
        }
        let event = self.record_event(WorldEvent::SeasonalHazard {
            region,
            hazard: impact.hazard,
            severity: impact.severity,
            peoples,
        });
        self.climate.seasons.events[region] = Some(event);
    }

    pub(crate) fn seasonal_feeding_cause(
        &self,
        region: usize,
        livelihood: Livelihood,
    ) -> Option<Cause> {
        let i = livelihood as usize;
        (self.climate.regions[region].feeding[i] < self.climate.seasons.feeding[region][i])
            .then(|| self.climate.seasons.events[region])
            .flatten()
            .map(|event| Cause {
                event,
                mechanism: Mechanism::Climate,
            })
    }
}

#[cfg(test)]
mod tests;

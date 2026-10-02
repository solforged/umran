//! Static drainage, named regional courses, and connected climate districts.
//! These purpose-keyed draws never consume the terrain generator's stream.

use crate::geography::{ClimateZone, Landmass, RIVER_FORMATION_FLOW, Region, River};
use crate::rng::{key, stream};
use rand::Rng;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

pub(crate) struct Hydrology {
    pub drainage: Vec<Option<usize>>,
    pub drainage_order: Vec<usize>,
    pub runoff: Vec<f32>,
    pub rivers: Vec<River>,
    pub river_regions: Vec<Option<usize>>,
    pub flows: Vec<f32>,
}

#[derive(Clone, Copy, PartialEq)]
struct Flood {
    spill: f32,
    region: usize,
}

impl Eq for Flood {}

impl Ord for Flood {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .spill
            .total_cmp(&self.spill)
            .then(other.region.cmp(&self.region))
    }
}

impl PartialOrd for Flood {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A coast-inward priority flood minimizes the highest elevation crossed to
/// sea. A discovered cell points at the already-popped cell that reached it:
/// even a level spill surface cannot cycle. Original elevations stay intact.
/// Different landmasses share the heap but never a land-to-land drain edge.
pub(crate) fn drainage(regions: &[Region]) -> (Vec<Option<usize>>, Vec<usize>) {
    let mut downstream = vec![None; regions.len()];
    let mut heap = BinaryHeap::new();
    let mut order = Vec::new();
    for (r, region) in regions.iter().enumerate() {
        if !region.terrain.is_land() {
            continue;
        }
        if let Some(sea) = region
            .neighbours
            .iter()
            .copied()
            .filter(|&n| !regions[n].terrain.is_land())
            .min()
        {
            downstream[r] = Some(sea);
            heap.push(Flood {
                spill: region.elevation,
                region: r,
            });
        }
    }
    while let Some(Flood { spill, region: r }) = heap.pop() {
        order.push(r);
        for &n in &regions[r].neighbours {
            if !regions[n].terrain.is_land()
                || downstream[n].is_some()
                || regions[n].landmass != regions[r].landmass
            {
                continue;
            }
            downstream[n] = Some(r);
            heap.push(Flood {
                spill: spill.max(regions[n].elevation),
                region: n,
            });
        }
    }
    assert_eq!(
        order.len(),
        regions.iter().filter(|r| r.terrain.is_land()).count(),
        "every landmass must have a sea outlet"
    );
    order.reverse();
    (downstream, order)
}

pub(crate) fn generate(seed: u64, regions: &[Region]) -> Hydrology {
    let (drainage, drainage_order) = drainage(regions);
    let runoff: Vec<f32> = regions
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
    for &r in &drainage_order {
        if let Some(n) = drainage[r] {
            flows[n] += flows[r];
        }
    }
    let (rivers, river_regions) = courses(regions, &drainage, &drainage_order, &flows);
    Hydrology {
        drainage,
        drainage_order,
        runoff,
        rivers,
        river_regions,
        flows,
    }
}

/// The strongest upstream branch continues a course; other branches join it.
/// Physical rainfall area, not polygon count, determines which reaches exist.
/// Course IDs follow their downstream endpoints' region IDs, and remain fixed
/// when climate changes flow. Catchments intentionally overlap at tributaries.
pub(crate) fn courses(
    regions: &[Region],
    drainage: &[Option<usize>],
    order: &[usize],
    flows: &[f32],
) -> (Vec<River>, Vec<Option<usize>>) {
    let mut main_upstream: Vec<Option<usize>> = vec![None; regions.len()];
    let reach = |r: usize| regions[r].terrain.is_land() && flows[r] >= RIVER_FORMATION_FLOW;
    for &r in order {
        if !reach(r) {
            continue;
        }
        if let Some(n) = drainage[r].filter(|&n| regions[n].terrain.is_land()) {
            let replace = main_upstream[n]
                .is_none_or(|old| flows[r] > flows[old] || (flows[r] == flows[old] && r < old));
            if replace {
                main_upstream[n] = Some(r);
            }
        }
    }
    let mut mouths = vec![0; regions.len()];
    for &r in order.iter().rev() {
        let n = drainage[r].expect("land has a downstream region");
        mouths[r] = if regions[n].terrain.is_land() {
            mouths[n]
        } else {
            n
        };
    }
    let mut rivers = Vec::new();
    let mut owner = vec![None; regions.len()];
    let mut stack = Vec::new();
    for end in 0..regions.len() {
        if !reach(end) {
            continue;
        }
        let next = drainage[end].expect("a river drains to land or sea");
        if regions[next].terrain.is_land() && main_upstream[next] == Some(end) {
            continue;
        }
        let id = rivers.len();
        let mut course = vec![end];
        owner[end] = Some(id);
        let mut at = end;
        while let Some(r) = main_upstream[at] {
            course.push(r);
            owner[r] = Some(id);
            at = r;
        }
        course.reverse();
        let mut catchment = Vec::new();
        stack.push(end);
        while let Some(r) = stack.pop() {
            catchment.push(r);
            stack.extend(
                regions[r]
                    .neighbours
                    .iter()
                    .copied()
                    .filter(|&n| drainage[n] == Some(r)),
            );
        }
        catchment.sort_unstable();
        rivers.push(River {
            course,
            mouth: mouths[end],
            catchment,
            joins: None,
        });
    }
    for river in &mut rivers {
        let end = *river.course.last().unwrap();
        river.joins = owner[drainage[end].unwrap()];
    }
    (rivers, owner)
}

pub(crate) fn warmth(seed: u64, region: usize, elevation: f32) -> f32 {
    let mut rng = stream(seed, &[key("climate baseline"), region as u64]);
    (rng.gen_range(0.65..0.95) - 0.25 * elevation).clamp(0.0, 1.0)
}

/// Grow compact connected districts to twelve lands, with seeded frontier
/// ties. Merge small remnants into bordering districts up to twenty lands.
/// Islands and pieces trapped by a narrow land bridge can remain below six;
/// neither disconnected zones nor oversized zones are invented to pad them.
pub(crate) fn climate_zones(
    seed: u64,
    regions: &mut [Region],
    landmasses: &[Landmass],
) -> Vec<ClimateZone> {
    const TARGET: usize = 12;
    const MINIMUM: usize = 6;
    const MAXIMUM: usize = 20;
    let mut rng = stream(seed, &[key("climate zones")]);
    let priorities: Vec<u64> = (0..regions.len()).map(|_| rng.r#gen()).collect();
    let mut seeds: Vec<usize> = landmasses
        .iter()
        .flat_map(|m| m.regions.iter().copied())
        .collect();
    seeds.sort_unstable_by_key(|&r| (priorities[r], r));
    let mut owner = vec![None; regions.len()];
    let mut queued = vec![usize::MAX; regions.len()];
    let mut frontier = BinaryHeap::new();
    let mut zones: Vec<ClimateZone> = Vec::new();
    for seed in seeds {
        if owner[seed].is_some() {
            continue;
        }
        let id = zones.len();
        let mass = regions[seed].landmass;
        let mut members = Vec::with_capacity(TARGET);
        frontier.clear();
        frontier.push(Reverse((0, priorities[seed], seed)));
        queued[seed] = id;
        while let Some(Reverse((distance, _, r))) = frontier.pop() {
            owner[r] = Some(id);
            members.push(r);
            if members.len() == TARGET {
                break;
            }
            for &n in &regions[r].neighbours {
                if regions[n].landmass == mass && owner[n].is_none() && queued[n] != id {
                    queued[n] = id;
                    frontier.push(Reverse((distance + 1, priorities[n], n)));
                }
            }
        }
        zones.push(ClimateZone { regions: members });
    }
    loop {
        let mut merged = false;
        for id in 0..zones.len() {
            let size = zones[id].regions.len();
            if size == 0 || size >= MINIMUM {
                continue;
            }
            let neighbour = zones[id]
                .regions
                .iter()
                .flat_map(|&r| regions[r].neighbours.iter())
                .filter_map(|&n| owner[n])
                .filter(|&z| z != id && zones[z].regions.len() + size <= MAXIMUM)
                .min_by_key(|&z| (zones[z].regions.len(), z));
            if let Some(to) = neighbour {
                let members = std::mem::take(&mut zones[id].regions);
                for &r in &members {
                    owner[r] = Some(to);
                }
                zones[to].regions.extend(members);
                merged = true;
            }
        }
        if !merged {
            break;
        }
    }
    zones.retain(|zone| !zone.regions.is_empty());
    for zone in &mut zones {
        zone.regions.sort_unstable();
    }
    zones.sort_unstable_by_key(|zone| zone.regions[0]);
    for (id, zone) in zones.iter().enumerate() {
        for &r in &zone.regions {
            regions[r].climate_zone = Some(id);
        }
    }
    zones
}

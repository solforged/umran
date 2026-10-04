//! Static lakes in priority-flood depressions, with a baseline water balance.
use crate::geography::{RIVER_FORMATION_FLOW, Region, River};
use crate::sphere;
use std::collections::VecDeque;

/// A water feature within existing land regions, not a replacement terrain.
#[derive(Clone, Debug, PartialEq)]
pub struct Lake {
    /// Regions intersecting the water surface, in increasing order.
    pub regions: Vec<usize>,
    /// Normalized elevation of the static water surface.
    pub surface: f32,
    /// River carrying the overflow; absent in an endorheic basin.
    pub outlet: Option<usize>,
    /// Land reach at the spill saddle, absent when evaporation closes the basin.
    pub spill: Option<usize>,
}

pub(crate) fn basins(
    regions: &[Region],
    drainage: &mut [Option<usize>],
    order: &mut Vec<usize>,
    runoff: &[f32],
) -> (Vec<Lake>, Vec<Option<usize>>) {
    let mut levels = vec![0.0_f32; regions.len()];
    let mut rank = vec![0; regions.len()];
    for (i, &r) in order.iter().rev().enumerate() {
        levels[r] = regions[r]
            .elevation
            .max(drainage[r].map_or(0.0, |n| levels[n]));
        rank[r] = i;
    }
    let mut flow = runoff.to_vec();
    for &r in order.iter() {
        if let Some(n) = drainage[r] {
            flow[n] += flow[r];
        }
    }
    let depressed =
        |r: usize| regions[r].terrain.is_land() && levels[r] > regions[r].elevation + 1e-6;
    let mut basin_ids = vec![None; regions.len()];
    let mut lake_regions = vec![None; regions.len()];
    let mut lakes = Vec::new();
    let mut queue = VecDeque::new();
    for start in 0..regions.len() {
        if !depressed(start) || basin_ids[start].is_some() {
            continue;
        }
        let id = lakes.len();
        let mut members = Vec::new();
        basin_ids[start] = Some(id);
        queue.push_back(start);
        while let Some(r) = queue.pop_front() {
            members.push(r);
            for &n in &regions[r].neighbours {
                if basin_ids[n].is_none() && depressed(n) && levels[n] == levels[start] {
                    basin_ids[n] = Some(id);
                    queue.push_back(n);
                }
            }
        }
        members.sort_unstable();
        let spill = *members.iter().min_by_key(|&&r| rank[r]).unwrap();
        let bottom = *members
            .iter()
            .min_by(|&&a, &&b| {
                regions[a]
                    .elevation
                    .total_cmp(&regions[b].elevation)
                    .then(a.cmp(&b))
            })
            .unwrap();
        let mut supply = 0.0;
        let mut evaporation = 0.0;
        for &r in &members {
            supply += runoff[r];
            evaporation += regions[r].area_km2 * (0.6 + 1.8 * regions[r].warmth);
            for &n in &regions[r].neighbours {
                if basin_ids[n] != Some(id) && drainage[n] == Some(r) {
                    supply += flow[n];
                }
            }
        }
        let open = supply > evaporation && supply >= RIVER_FORMATION_FLOW;
        let root = if open { spill } else { bottom };
        let downstream = if open { drainage[spill] } else { None };
        // One tree through each basin prevents competing equal-level exits.
        // The earliest flood cell has the route below every basin member.
        let mut routed = std::collections::BTreeSet::new();
        routed.insert(root);
        drainage[root] = downstream;
        queue.push_back(root);
        while let Some(r) = queue.pop_front() {
            for &n in &regions[r].neighbours {
                if basin_ids[n] == Some(id) && routed.insert(n) {
                    drainage[n] = Some(r);
                    queue.push_back(n);
                }
            }
        }
        let fraction = if open {
            1.0
        } else {
            (supply / evaporation).clamp(0.05, 0.95)
        };
        let surface =
            regions[bottom].elevation + (levels[start] - regions[bottom].elevation) * fraction;
        members.retain(|&r| regions[r].elevation < surface);
        for &r in &members {
            lake_regions[r] = Some(id);
        }
        lakes.push(Lake {
            regions: members,
            surface,
            outlet: None,
            spill: open.then_some(spill),
        });
    }
    // Basin routing changes the within-lake order, never the region IDs.
    let mut upstream = vec![0_usize; regions.len()];
    for next in drainage.iter().flatten() {
        upstream[*next] += 1;
    }
    order.clear();
    for (r, region) in regions.iter().enumerate() {
        if region.terrain.is_land() && upstream[r] == 0 {
            queue.push_back(r);
        }
    }
    while let Some(r) = queue.pop_front() {
        order.push(r);
        if let Some(n) = drainage[r] {
            upstream[n] -= 1;
            if upstream[n] == 0 && regions[n].terrain.is_land() {
                queue.push_back(n);
            }
        }
    }
    assert_eq!(
        order.len(),
        regions.iter().filter(|r| r.terrain.is_land()).count(),
        "lake drainage cannot cycle"
    );
    (lakes, lake_regions)
}

/// Centre-to-border-to-centre arcs stay inside the spherical star cells.
/// Distinct cells have disjoint interiors, so a drainage tree cannot cross itself.
pub(crate) fn channels(regions: &[Region], drainage: &[Option<usize>], rivers: &mut [River]) {
    for river in rivers {
        let mut points = Vec::with_capacity(river.course.len() * 2 + 1);
        for &r in &river.course {
            points.push(regions[r].position);
            if let Some(n) = drainage[r] {
                // Match Map::shared_midpoint's canonical border traversal.
                let (first, second) = if r < n { (r, n) } else { (n, r) };
                points.push(
                    sphere::shared_midpoint(&regions[first].boundary, &regions[second].boundary)
                        .expect("drainage regions share their spherical boundary"),
                );
            }
        }
        let end = *river.course.last().unwrap();
        if let Some(n) = drainage[end].filter(|&n| regions[n].terrain.is_land()) {
            points.push(regions[n].position);
        }
        river.channel = points;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geography::{Map, MapSize, Terrain};

    #[test]
    fn v4_adds_lakes_without_redrawing_v3_land() {
        use crate::geography::GeographyVersion;
        assert_eq!(GeographyVersion::default(), GeographyVersion::ContinentalV5);
        for size in [
            MapSize::Small,
            MapSize::Medium,
            MapSize::Large,
            MapSize::Vast,
        ] {
            let mut count = 0;
            for seed in [0, 7, 21] {
                let old = Map::generate_with_version(seed, size, GeographyVersion::ContinentalV3);
                let new = Map::generate_with_version(seed, size, GeographyVersion::ContinentalV4);
                assert_eq!(new.geography, GeographyVersion::ContinentalV4);
                assert_eq!(new.regions, old.regions);
                assert_eq!(new.landmasses, old.landmasses);
                assert_eq!(new.climate_zones, old.climate_zones);
                assert_eq!(new.runoff, old.runoff);
                assert!(new.rivers.iter().all(|river| !river.channel.is_empty()));
                count += new.lakes.len();
            }
            assert!(count > 0, "{size:?} should have lakes");
        }
    }

    #[test]
    fn lake_capability_keeps_legacy_versions_dry() {
        use crate::geography::GeographyVersion;
        let (regions, _, _, _) = depression(200_000.0);
        for version in [
            GeographyVersion::SphericalV1,
            GeographyVersion::ContinentalV2,
            GeographyVersion::ContinentalV3,
        ] {
            assert!(!version.has_lakes());
            assert!(
                crate::rivers::generate(7, &regions, version)
                    .lakes
                    .is_empty()
            );
        }
        assert!(GeographyVersion::ContinentalV4.has_lakes());
        assert!(GeographyVersion::ContinentalV5.has_lakes());
        let v4 = crate::rivers::generate(7, &regions, GeographyVersion::ContinentalV4);
        let v5 = crate::rivers::generate(7, &regions, GeographyVersion::ContinentalV5);
        assert!(!v4.lakes.is_empty());
        assert_eq!(v4.lakes, v5.lakes);
        assert_eq!(v4.lake_regions, v5.lake_regions);
        assert_eq!(v4.drainage, v5.drainage);
        assert_eq!(v4.drainage_order, v5.drainage_order);
        assert_eq!(v4.runoff, v5.runoff);
    }

    fn depression(runoff_per_land: f32) -> (Vec<Region>, Vec<Option<usize>>, Vec<usize>, Vec<f32>) {
        let regions = [0.0, 0.6, 0.2, 0.1, 0.8]
            .into_iter()
            .enumerate()
            .map(|(r, elevation)| Region {
                position: [1.0, 0.0, 0.0],
                boundary: Vec::new(),
                site: [0.0; 2],
                area_km2: 100_000.0,
                terrain: if r == 0 {
                    Terrain::Sea
                } else {
                    Terrain::Plains
                },
                elevation,
                moisture: 0.5,
                warmth: 0.5,
                climate_zone: (r != 0).then_some(0),
                neighbours: (0..5).filter(|&n| r.abs_diff(n) == 1).collect(),
                landmass: (r != 0).then_some(0),
            })
            .collect();
        (
            regions,
            vec![None, Some(0), Some(1), Some(2), Some(3)],
            vec![4, 3, 2, 1],
            vec![
                0.0,
                runoff_per_land,
                runoff_per_land,
                runoff_per_land,
                runoff_per_land,
            ],
        )
    }

    #[test]
    fn dry_basins_stop_rivers_and_wet_basins_overflow_without_changing_land() {
        for (runoff, open) in [(10_000.0, false), (200_000.0, true)] {
            let (regions, mut drainage, mut order, runoff) = depression(runoff);
            let original = regions.clone();
            let (lakes, ownership) = basins(&regions, &mut drainage, &mut order, &runoff);
            assert_eq!(lakes.len(), 1);
            let lake = &lakes[0];
            assert_eq!(lake.spill.is_some(), open);
            assert!(lake.surface > 0.1 && lake.surface <= 0.6);
            assert_eq!(ownership[3], Some(0));
            if open {
                assert_eq!(lake.surface, 0.6);
                assert_eq!(drainage[2], Some(1));
            } else {
                assert_eq!(drainage[3], None);
                assert_eq!(drainage[2], Some(3));
            }
            let mut flows = runoff.clone();
            for &r in &order {
                if let Some(n) = drainage[r] {
                    flows[n] += flows[r];
                }
            }
            let (rivers, _) = crate::rivers::courses(&regions, &drainage, &order, &flows);
            assert!(
                rivers
                    .iter()
                    .all(|river| river.mouth == if open { 0 } else { 3 })
            );
            assert_eq!(regions, original);
        }
    }

    #[test]
    fn monotone_slopes_do_not_invent_lakes() {
        let (mut regions, mut drainage, mut order, runoff) = depression(200_000.0);
        for (r, region) in regions.iter_mut().enumerate() {
            region.elevation = r as f32 * 0.1;
        }
        let before = drainage.clone();
        let (lakes, owners) = basins(&regions, &mut drainage, &mut order, &runoff);
        assert!(lakes.is_empty());
        assert!(owners.iter().all(Option::is_none));
        assert_eq!(drainage, before);
    }

    fn inside_arc(p: sphere::Point, a: sphere::Point, b: sphere::Point) -> bool {
        let ap = sphere::angle(a, p);
        let pb = sphere::angle(p, b);
        ap > 1e-8 && pb > 1e-8 && (ap + pb - sphere::angle(a, b)).abs() < 1e-9
    }

    #[test]
    fn channels_are_unit_non_crossing_arcs_stable_under_chart_scale() {
        for size in [
            MapSize::Small,
            MapSize::Medium,
            MapSize::Large,
            MapSize::Vast,
        ] {
            for seed in 0..4 {
                let mut map = Map::generate(seed, size);
                assert_eq!(map.geography, crate::GeographyVersion::ContinentalV5);
                for river in &map.rivers {
                    for &p in &river.channel {
                        assert!((sphere::dot(p, p) - 1.0).abs() < 1e-12);
                    }
                    for (i, a) in river.channel.windows(2).enumerate() {
                        assert!(sphere::angle(a[0], a[1]) > 1e-10);
                        for b in river.channel.windows(2).skip(i + 2) {
                            let line =
                                sphere::cross(sphere::cross(a[0], a[1]), sphere::cross(b[0], b[1]));
                            if sphere::dot(line, line) < 1e-24 {
                                continue;
                            }
                            let p = sphere::unit(line);
                            let q = [-p[0], -p[1], -p[2]];
                            assert!(!(inside_arc(p, a[0], a[1]) && inside_arc(p, b[0], b[1])));
                            assert!(!(inside_arc(q, a[0], a[1]) && inside_arc(q, b[0], b[1])));
                        }
                    }
                    let end = *river.course.last().unwrap();
                    let endpoint = match map.drainage[end] {
                        None => map.regions[end].position,
                        Some(n) if map.regions[n].terrain.is_land() => map.regions[n].position,
                        Some(n) => map.shared_midpoint(end, n).unwrap(),
                    };
                    assert_eq!(river.channel.last(), Some(&endpoint));
                }
                let before = map.rivers.clone();
                map.radius_km *= 10.0;
                map.width *= 10.0;
                for region in &mut map.regions {
                    region.site = [0.0, 0.0];
                }
                channels(&map.regions, &map.drainage, &mut map.rivers);
                assert_eq!(map.rivers, before);
            }
        }
    }
}

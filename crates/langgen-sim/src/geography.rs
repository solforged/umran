//! The land peoples live on: a map of regions drawn from the world's seed.
//!
//! Regions are the cells around jittered points on a hex grid (a Voronoi
//! diagram), so each borders about six others. Elevation and moisture come
//! from smooth noise, and their ranks rather than their raw values decide
//! terrain, so every map has about the same share of sea, mountain, and
//! desert. Generation uses only arithmetic and square roots, never `exp` or
//! `sin`, whose last bits can differ between native code and WASM.

use crate::rng::{key, stream};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// Height of one hex-grid row, with points one unit apart: √3 / 2.
const ROW: f64 = 0.866_025_403_784_438_6;
/// Furthest a point strays from its grid position, in grid units.
const JITTER: f64 = 0.3;
/// Share of regions under water.
const SEA: f64 = 0.4;
/// Shares of land that are mountain and hill, by height.
const MOUNTAINS: f64 = 0.1;
const HILLS: f64 = 0.15;
/// Shares of the remaining land that are desert and steppe (driest
/// first) and forest (wettest).
const DESERT: f64 = 0.12;
const STEPPE: f64 = 0.25;
const FOREST: f64 = 0.3;
/// Closeness of peoples on two bordering open plains.
const PLAIN_CLOSENESS: f32 = 0.8;
/// Closest any two regions can be, short of being the same land.
const MAX_CLOSENESS: f32 = 0.9;

/// How large a world is: how many regions its map has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MapSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl MapSize {
    /// Columns and rows of the hex grid.
    fn grid(self) -> (usize, usize) {
        match self {
            MapSize::Small => (9, 7),
            MapSize::Medium => (13, 10),
            MapSize::Large => (18, 14),
        }
    }
}

/// What a region's land is like, which sets how many it feeds and how hard
/// it is to cross.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Terrain {
    Sea,
    Plains,
    Forest,
    Steppe,
    Hills,
    Mountains,
    Desert,
}

impl Terrain {
    /// How many the land feeds, as a share of what an open plain feeds.
    pub fn fertility(self) -> f32 {
        match self {
            Terrain::Sea => 0.0,
            Terrain::Plains => 1.0,
            Terrain::Forest => 0.6,
            Terrain::Steppe | Terrain::Hills => 0.5,
            Terrain::Mountains => 0.15,
            Terrain::Desert => 0.1,
        }
    }

    /// Effort of crossing it, relative to open plain. Open steppe is
    /// easiest, which is why languages spread so far across it; mountains
    /// are hardest, which is why they shelter so many.
    pub fn travel(self) -> f32 {
        match self {
            Terrain::Steppe => 0.8,
            Terrain::Plains => 1.0,
            Terrain::Forest => 1.5,
            Terrain::Hills => 2.0,
            Terrain::Desert => 2.5,
            Terrain::Sea => 3.0,
            Terrain::Mountains => 4.0,
        }
    }

    pub fn is_land(self) -> bool {
        self != Terrain::Sea
    }
}

/// One region of the map.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    /// The point the region was drawn around, in grid units.
    pub site: [f32; 2],
    /// Its border, as a convex polygon in grid units.
    pub outline: Vec<[f32; 2]>,
    pub terrain: Terrain,
    /// Regions sharing a border with it, in increasing order.
    pub neighbours: Vec<usize>,
}

/// The world's land and sea.
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub size: MapSize,
    pub width: f32,
    pub height: f32,
    pub regions: Vec<Region>,
    /// Least effort of travel between each pair of regions, by land or sea,
    /// row by row.
    distance: Vec<f32>,
}

impl Map {
    /// The map `seed` draws for a world of `size`.
    pub fn generate(seed: u64, size: MapSize) -> Map {
        let (cols, rows) = size.grid();
        let width = cols as f64 + 0.5;
        let height = (rows - 1) as f64 * ROW + 1.0;
        let mut rng = stream(seed, &[key("map")]);
        let mut sites = Vec::with_capacity(cols * rows);
        for r in 0..rows {
            for c in 0..cols {
                let offset = if r % 2 == 1 { 0.5 } else { 0.0 };
                let x = 0.5 + c as f64 + offset + rng.gen_range(-JITTER..JITTER);
                let y = 0.5 + r as f64 * ROW + rng.gen_range(-JITTER..JITTER);
                sites.push([x, y]);
            }
        }
        let broad = Noise::new(&mut rng, width, height, 3.0);
        let fine = Noise::new(&mut rng, width, height, 1.5);
        let wet = Noise::new(&mut rng, width, height, 3.5);
        let cells: Vec<(Vec<[f64; 2]>, Vec<usize>)> = (0..sites.len())
            .map(|i| cell(&sites, i, width, height))
            .collect();
        let neighbours = symmetric(cells.iter().map(|(_, n)| n.clone()).collect());

        // A continent: higher toward the middle of the map, falling to sea
        // at its edges, roughened by noise.
        let elevation: Vec<f64> = sites
            .iter()
            .map(|&[x, y]| {
                let dx = (x - width / 2.0) / (width / 2.0);
                let dy = (y - height / 2.0) / (height / 2.0);
                broad.at(x, y) + 0.5 * fine.at(x, y) - 0.9 * (dx * dx + dy * dy)
            })
            .collect();
        let mut terrain = vec![Terrain::Plains; sites.len()];
        let all: Vec<usize> = (0..sites.len()).collect();
        for &i in ranked(&all, &elevation).iter().take(share(all.len(), SEA)) {
            terrain[i] = Terrain::Sea;
        }
        let land: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&i| terrain[i].is_land())
            .collect();
        let by_height = ranked(&land, &elevation);
        let peaks = share(land.len(), MOUNTAINS);
        let hills = share(land.len(), HILLS);
        for &i in by_height.iter().rev().take(peaks) {
            terrain[i] = Terrain::Mountains;
        }
        for &i in by_height.iter().rev().skip(peaks).take(hills) {
            terrain[i] = Terrain::Hills;
        }
        // The coast is wetter than the interior.
        let moisture: Vec<f64> = sites
            .iter()
            .enumerate()
            .map(|(i, &[x, y])| {
                let coastal = neighbours[i].iter().any(|&n| terrain[n] == Terrain::Sea);
                wet.at(x, y) + if coastal { 0.15 } else { 0.0 }
            })
            .collect();
        let lowland: Vec<usize> = land
            .iter()
            .copied()
            .filter(|&i| terrain[i] == Terrain::Plains)
            .collect();
        let by_wet = ranked(&lowland, &moisture);
        let desert = share(lowland.len(), DESERT);
        let steppe = share(lowland.len(), STEPPE);
        let forest = share(lowland.len(), FOREST);
        for &i in by_wet.iter().take(desert) {
            terrain[i] = Terrain::Desert;
        }
        for &i in by_wet.iter().skip(desert).take(steppe) {
            terrain[i] = Terrain::Steppe;
        }
        for &i in by_wet.iter().rev().take(forest) {
            terrain[i] = Terrain::Forest;
        }

        let regions: Vec<Region> = cells
            .into_iter()
            .zip(neighbours)
            .enumerate()
            .map(|(i, ((outline, _), neighbours))| Region {
                site: [sites[i][0] as f32, sites[i][1] as f32],
                outline: outline.iter().map(|&[x, y]| [x as f32, y as f32]).collect(),
                terrain: terrain[i],
                neighbours,
            })
            .collect();
        let distance = travel_distances(&regions);
        Map {
            size,
            width: width as f32,
            height: height as f32,
            regions,
            distance,
        }
    }

    /// Least effort of travelling from region `a` to region `b`.
    pub fn distance(&self, a: usize, b: usize) -> f32 {
        self.distance[a * self.regions.len() + b]
    }

    /// How readily peoples on regions `a` and `b` meet as neighbours: 1 on
    /// the same land, less across a harder border, and 0 when the two
    /// share no land border.
    pub fn closeness(&self, a: usize, b: usize) -> f32 {
        if a == b {
            return 1.0;
        }
        let (ra, rb) = (&self.regions[a], &self.regions[b]);
        if !ra.terrain.is_land() || !rb.terrain.is_land() || !ra.neighbours.contains(&b) {
            return 0.0;
        }
        let effort = ra.terrain.travel() + rb.terrain.travel();
        (PLAIN_CLOSENESS * 2.0 / effort).min(MAX_CLOSENESS)
    }

    /// Whether region `r` is land that borders the sea.
    pub fn coastal(&self, r: usize) -> bool {
        let region = &self.regions[r];
        region.terrain.is_land()
            && region
                .neighbours
                .iter()
                .any(|&n| self.regions[n].terrain == Terrain::Sea)
    }
}

/// Smooth random values over the plane: random heights at the corners of
/// a coarse lattice, blended between them.
struct Noise {
    cell: f64,
    cols: usize,
    values: Vec<f64>,
}

impl Noise {
    fn new(rng: &mut ChaCha8Rng, width: f64, height: f64, cell: f64) -> Noise {
        let cols = (width / cell) as usize + 2;
        let rows = (height / cell) as usize + 2;
        let values = (0..cols * rows).map(|_| rng.r#gen::<f64>()).collect();
        Noise { cell, cols, values }
    }

    fn at(&self, x: f64, y: f64) -> f64 {
        let (gx, gy) = (x.max(0.0) / self.cell, y.max(0.0) / self.cell);
        let (ix, iy) = (gx as usize, gy as usize);
        let (tx, ty) = (smooth(gx - ix as f64), smooth(gy - iy as f64));
        let v = |c: usize, r: usize| self.values[r * self.cols + c];
        let top = v(ix, iy) + (v(ix + 1, iy) - v(ix, iy)) * tx;
        let bottom = v(ix, iy + 1) + (v(ix + 1, iy + 1) - v(ix, iy + 1)) * tx;
        top + (bottom - top) * ty
    }
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// The Voronoi cell of site `i` within the map's bounds, and the sites
/// whose cells it borders: the bounds clipped by the half-plane nearer `i`
/// than each other site. Each edge of the polygon remembers the site that
/// cut it, or none for the map's edge.
fn cell(sites: &[[f64; 2]], i: usize, width: f64, height: f64) -> (Vec<[f64; 2]>, Vec<usize>) {
    let mut polygon: Vec<([f64; 2], Option<usize>)> = vec![
        ([0.0, 0.0], None),
        ([width, 0.0], None),
        ([width, height], None),
        ([0.0, height], None),
    ];
    let [px, py] = sites[i];
    for (j, &[qx, qy]) in sites.iter().enumerate() {
        if j == i {
            continue;
        }
        // Points p with (p - m)·n <= 0 are nearer site i.
        let (nx, ny) = (qx - px, qy - py);
        let (mx, my) = ((px + qx) / 2.0, (py + qy) / 2.0);
        let side = |[x, y]: [f64; 2]| (x - mx) * nx + (y - my) * ny;
        let mut next = Vec::with_capacity(polygon.len() + 1);
        for k in 0..polygon.len() {
            let (a, label) = polygon[k];
            let (b, _) = polygon[(k + 1) % polygon.len()];
            let (sa, sb) = (side(a), side(b));
            let cross = || {
                let t = sa / (sa - sb);
                [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
            };
            match (sa <= 0.0, sb <= 0.0) {
                (true, true) => next.push((a, label)),
                (true, false) => {
                    next.push((a, label));
                    next.push((cross(), Some(j)));
                }
                (false, true) => next.push((cross(), label)),
                (false, false) => {}
            }
        }
        polygon = next;
    }
    let mut borders: Vec<usize> = (0..polygon.len())
        .filter_map(|k| {
            let (a, label) = polygon[k];
            let (b, _) = polygon[(k + 1) % polygon.len()];
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let long = dx * dx + dy * dy > 1e-6;
            label.filter(|_| long)
        })
        .collect();
    borders.sort_unstable();
    borders.dedup();
    (polygon.into_iter().map(|(p, _)| p).collect(), borders)
}

/// Borders as both sides see them: rounding can leave a sliver of border
/// on one side only.
fn symmetric(mut neighbours: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    for i in 0..neighbours.len() {
        for k in 0..neighbours[i].len() {
            let j = neighbours[i][k];
            if !neighbours[j].contains(&i) {
                neighbours[j].push(i);
            }
        }
    }
    for n in &mut neighbours {
        n.sort_unstable();
    }
    neighbours
}

/// `of` ordered by `value`, lowest first, ties by index.
fn ranked(of: &[usize], value: &[f64]) -> Vec<usize> {
    let mut out = of.to_vec();
    out.sort_by(|&a, &b| value[a].total_cmp(&value[b]).then(a.cmp(&b)));
    out
}

fn share(len: usize, fraction: f64) -> usize {
    (len as f64 * fraction + 0.5) as usize
}

/// Least travel effort between every pair of regions (Floyd–Warshall).
/// Crossing a border costs the mean effort of the two regions it joins.
fn travel_distances(regions: &[Region]) -> Vec<f32> {
    let n = regions.len();
    let mut d = vec![f32::INFINITY; n * n];
    for (i, region) in regions.iter().enumerate() {
        d[i * n + i] = 0.0;
        for &j in &region.neighbours {
            d[i * n + j] = (region.terrain.travel() + regions[j].terrain.travel()) / 2.0;
        }
    }
    for k in 0..n {
        for i in 0..n {
            let ik = d[i * n + k];
            if ik.is_infinite() {
                continue;
            }
            for j in 0..n {
                let through = ik + d[k * n + j];
                if through < d[i * n + j] {
                    d[i * n + j] = through;
                }
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_tile_the_map_and_border_each_other_both_ways() {
        for size in [MapSize::Small, MapSize::Medium, MapSize::Large] {
            let map = Map::generate(7, size);
            let area: f32 = map.regions.iter().map(|r| polygon_area(&r.outline)).sum();
            assert!(
                (area - map.width * map.height).abs() < 1e-2,
                "{size:?}: {area}"
            );
            for (i, r) in map.regions.iter().enumerate() {
                assert!(
                    (2..=9).contains(&r.neighbours.len()),
                    "{i}: {:?}",
                    r.neighbours
                );
                for &j in &r.neighbours {
                    assert!(map.regions[j].neighbours.contains(&i));
                }
            }
        }
    }

    #[test]
    fn every_map_has_sea_mountains_and_open_land_in_proportion() {
        for seed in 0..20 {
            let map = Map::generate(seed, MapSize::Medium);
            let count = |t: Terrain| map.regions.iter().filter(|r| r.terrain == t).count();
            let n = map.regions.len();
            assert_eq!(count(Terrain::Sea), share(n, SEA));
            assert!(count(Terrain::Mountains) > 0 && count(Terrain::Plains) > 0);
            assert!(count(Terrain::Desert) < count(Terrain::Plains));
        }
    }

    #[test]
    fn travel_follows_the_easiest_route() {
        let map = Map::generate(3, MapSize::Medium);
        for (i, r) in map.regions.iter().enumerate() {
            for &j in &r.neighbours {
                let direct = (r.terrain.travel() + map.regions[j].terrain.travel()) / 2.0;
                assert!(map.distance(i, j) <= direct + 1e-6);
                assert_eq!(map.distance(i, j), map.distance(j, i));
            }
        }
    }

    #[test]
    fn the_same_seed_draws_the_same_map() {
        assert_eq!(
            Map::generate(11, MapSize::Small),
            Map::generate(11, MapSize::Small)
        );
        assert_ne!(
            Map::generate(11, MapSize::Small).regions,
            Map::generate(12, MapSize::Small).regions
        );
    }

    fn polygon_area(points: &[[f32; 2]]) -> f32 {
        let n = points.len();
        (0..n)
            .map(|k| {
                let ([ax, ay], [bx, by]) = (points[k], points[(k + 1) % n]);
                ax * by - bx * ay
            })
            .sum::<f32>()
            .abs()
            / 2.0
    }
}

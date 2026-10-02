//! The land peoples live on: a map of regions drawn from the world's seed.
//!
//! Regions are the cells around jittered points on a hex grid (a Voronoi
//! diagram), so each borders about six others. Separated basins contain
//! connected, noisy continental coasts; small islands sit in their ocean
//! moats. Relief and moisture ranks set terrain shares on the finished land
//! mask. Generation uses only arithmetic and square roots, never `exp` or
//! `sin`, whose last bits can differ between native code and WASM.

use crate::livelihood::Livelihood;
use crate::rng::{index, key, stream};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cmp::Ordering, collections::BinaryHeap};

/// Height of one hex-grid row, with points one unit apart: √3 / 2.
const ROW: f64 = 0.866_025_403_784_438_6;
/// Physical length represented by one drawing/grid unit.
pub const KM_PER_UNIT: f32 = 100.0;
/// Plain-kilometre scale used to soften journey preferences.
pub const REFERENCE_TRAVEL_KM: f32 = 100.0;
/// Area of an interior reference hexagon with 100 km centre spacing.
pub const REFERENCE_AREA_KM2: f32 = 8_660.254;
/// Precomputed neighbourhood radius, not a limit on exact journeys.
pub const CACHE_REACH_KM: f32 = 1_800.0;
/// Fixed effort-km charged on each embarkation and landing.
const EMBARK: f32 = 100.0;
/// Furthest a point strays from its grid position, in grid units.
const JITTER: f64 = 0.3;
/// Share of regions under water.
const SEA: f64 = 0.5;
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
/// One row describes a regional theatre, with regions about 100 km across.
/// The half-sea budget is fixed; land beyond the main bodies is small islands.
struct Geography {
    cols: usize,
    rows: usize,
    continents: (usize, usize),
    continent_land: (usize, usize),
    minimum: usize,
    /// Fixed number of one/two-cell islands; None preserves the regional packs.
    island_count: Option<usize>,
}

const GEOGRAPHY: [Geography; 4] = [
    Geography {
        cols: 9,
        rows: 7,
        continents: (1, 1),
        continent_land: (20, 27),
        minimum: 20,
        island_count: None,
    },
    Geography {
        cols: 13,
        rows: 10,
        continents: (1, 2),
        continent_land: (45, 55),
        minimum: 20,
        island_count: None,
    },
    Geography {
        cols: 18,
        rows: 14,
        continents: (2, 3),
        continent_land: (90, 110),
        minimum: 25,
        island_count: None,
    },
    Geography {
        cols: 60,
        rows: 60,
        continents: (2, 2),
        continent_land: (1_794, 1_797),
        minimum: 800,
        island_count: Some(3),
    },
];

/// How large a world is: how many regions its map has.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MapSize {
    Small,
    #[default]
    Medium,
    Large,
    Vast,
}

impl MapSize {
    fn geography(self) -> &'static Geography {
        &GEOGRAPHY[self as usize]
    }

    /// Columns and rows of the hex grid.
    fn grid(self) -> (usize, usize) {
        let g = self.geography();
        (g.cols, g.rows)
    }

    /// Fewest regions a body of land needs to count as a continent, one of
    /// the map's main bodies; smaller ones are islands. A region stands for
    /// land about 100 km across. Regional sizes hold country-sized or
    /// subcontinental fragments; Vast holds two small-continent-sized bodies.
    pub fn continent_minimum(self) -> usize {
        self.geography().minimum
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

    /// How readily a people living on it takes to the road, relative to
    /// farmers on open plain: herders on the steppe and in the desert move
    /// often, as the peoples of the Eurasian steppe did again and again;
    /// mountain folk seldom do.
    pub fn mobility(self) -> f32 {
        match self {
            Terrain::Steppe => 3.0,
            Terrain::Desert => 2.0,
            Terrain::Plains | Terrain::Hills => 1.0,
            Terrain::Forest => 0.8,
            Terrain::Mountains => 0.5,
            Terrain::Sea => 0.0,
        }
    }
}

/// One region of the map.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    /// The point the region was drawn around, in grid units.
    pub site: [f32; 2],
    /// Its border, as a convex polygon in grid units.
    pub outline: Vec<[f32; 2]>,
    /// Physical area, derived before drawing coordinates are rounded.
    pub area_km2: f32,
    pub terrain: Terrain,
    /// Regions sharing a border with it, in increasing order.
    pub neighbours: Vec<usize>,
    /// The body of land it belongs to, an index into `Map::landmasses`;
    /// `None` for sea. Land in different bodies can only be reached by
    /// crossing the sea.
    pub landmass: Option<usize>,
}

/// Whether a body of land is one of the map's main bodies or an island.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LandmassKind {
    Continent,
    Island,
}

/// One connected body of land.
#[derive(Clone, Debug, PartialEq)]
pub struct Landmass {
    pub kind: LandmassKind,
    /// Its regions, in increasing order.
    pub regions: Vec<usize>,
    /// The member region nearest its centre of area, where a chart writes
    /// its name; never sea, even for a crescent of land.
    pub anchor: usize,
}

/// Immutable, ID-sorted adjacency or distance rows in compressed sparse form.
#[derive(Clone, Debug, Default, PartialEq)]
struct RouteRows {
    offsets: Vec<usize>,
    entries: Vec<(u32, f32)>,
}

impl RouteRows {
    fn row(&self, source: usize) -> &[(u32, f32)] {
        &self.entries[self.offsets[source]..self.offsets[source + 1]]
    }

    fn get(&self, source: usize, destination: usize) -> Option<f32> {
        let row = self.row(source);
        row.binary_search_by_key(&(destination as u32), |&(r, _)| r)
            .ok().map(|i| row[i].1)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RouteMode {
    Walking,
    Voyage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RouteVisit {
    effort: f64,
    region: u32,
}

impl Eq for RouteVisit {}

impl Ord for RouteVisit {
    fn cmp(&self, other: &Self) -> Ordering {
        other.effort.total_cmp(&self.effort).then(other.region.cmp(&self.region))
    }
}

impl PartialOrd for RouteVisit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Reused across every cached source: only touched distances need resetting.
struct RouteScratch {
    distance: Vec<f64>,
    touched: Vec<u32>,
    heap: BinaryHeap<RouteVisit>,
}

impl RouteScratch {
    fn new(n: usize) -> Self {
        Self {
            distance: vec![f64::INFINITY; n],
            touched: Vec::new(),
            heap: BinaryHeap::new(),
        }
    }

    fn reset(&mut self) {
        for r in self.touched.drain(..) {
            self.distance[r as usize] = f64::INFINITY;
        }
        self.heap.clear();
    }

    fn relax(&mut self, region: u32, effort: f64, reach: f32) {
        // Radius boundaries use the public f32 effort, including its rounding.
        if effort as f32 > reach || effort >= self.distance[region as usize] {
            return;
        }
        if self.distance[region as usize].is_infinite() {
            self.touched.push(region);
        }
        self.distance[region as usize] = effort;
        self.heap.push(RouteVisit { effort, region });
    }

    fn append_row(&mut self, map: &Map, source: usize, mode: RouteMode, entries: &mut Vec<(u32, f32)>) {
        self.touched.sort_unstable();
        entries.extend(self.touched.iter().copied().filter(|&r| {
            map.regions[r as usize].terrain.is_land()
                && (mode == RouteMode::Walking || r as usize != source)
        }).map(|r| (r, self.distance[r as usize] as f32)));
    }
}

/// The world's land and sea.
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub size: MapSize,
    pub width: f32,
    pub height: f32,
    pub regions: Vec<Region>,
    /// Its bodies of land, numbered as `Region::landmass` numbers them.
    pub landmasses: Vec<Landmass>,
    /// Symmetric centre-to-centre effort through each shared border midpoint.
    edges: RouteRows,
    /// Land-only walking and sea-interior voyage neighbourhoods to 1800 effort-km.
    walking: RouteRows,
    voyages: RouteRows,
    feeding: Vec<[f32; 3]>,
}

impl Map {
    /// The map `seed` draws for a world of `size`.
    pub fn generate(seed: u64, size: MapSize) -> Map {
        let g = size.geography();
        let (cols, rows) = size.grid();
        let width = cols as f64 + 0.5;
        let height = (rows - 1) as f64 * ROW + 1.0;
        let n = cols * rows;
        let mut rng = stream(seed, &[key("map")]);
        // Draw once: fallback geometries use the same offsets, not new worlds.
        let offsets: Vec<[f64; 2]> = (0..n)
            .map(|_| [rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)])
            .collect();
        let broad = Noise::new(&mut rng, width, height, 3.0);
        let fine = Noise::new(&mut rng, width, height, 1.5);
        let wet = Noise::new(&mut rng, width, height, 3.5);
        let mut layout_rng = stream(seed, &[key("continent layout")]);
        let k = g.continents.0 + index(&mut layout_rng, g.continents.1 - g.continents.0 + 1);
        let layouts = layouts(g, k);
        let start = index(&mut layout_rng, layouts.len());
        let mut shares_rng = stream(seed, &[key("continent shares")]);
        let continent_land = g.continent_land.0
            + index(&mut shares_rng, g.continent_land.1 - g.continent_land.0 + 1);
        let weights: Vec<f64> = (0..k).map(|_| shares_rng.gen_range(0.75..1.25)).collect();
        let island_land = n - share(n, SEA) - continent_land;
        let budget = LandBudget {
            k,
            minimum: g.minimum,
            continents: continent_land,
            islands: island_land,
            island_count: g.island_count,
        };
        let mut island_rng = stream(seed, &[key("continent islands")]);
        let island_order: Vec<f64> = (0..n).map(|_| island_rng.r#gen()).collect();
        let island_orientation = index(&mut island_rng, 8);
        let mut coasts_rng = stream(seed, &[key("continent coasts")]);
        let centres: Vec<[f64; 2]> = (0..k)
            .map(|_| {
                [
                    coasts_rng.gen_range(-0.15..0.15),
                    coasts_rng.gen_range(-0.15..0.15),
                ]
            })
            .collect();

        // The family is finite at each amplitude. Zero jitter is the terminal
        // constructor, proven separately for every size/K and land budget.
        let (sites, cells, borders, neighbours, plan) = [JITTER, JITTER / 2.0, 0.0]
            .into_iter()
            .find_map(|jitter| {
                let sites = sites(g, &offsets, jitter);
                let cells = grid_cells(g, &sites, width, height);
                let borders = shared_borders(&cells);
                let neighbours = border_neighbours(n, &borders);
                let plan = if jitter == 0.0 {
                    // A canonical terminal ordering makes feasibility a finite
                    // proof over budgets, not another seed-dependent gamble.
                    choose_plan(&layouts, 0, &neighbours, &budget, &sweep_order(g, 0))
                } else {
                    choose_plan(&layouts, start, &neighbours, &budget, &island_order).or_else(
                        || {
                            (0..8).find_map(|offset| {
                                let order = sweep_order(g, (island_orientation + offset) % 8);
                                choose_plan(&layouts, start, &neighbours, &budget, &order)
                            })
                        },
                    )
                }?;
                Some((sites, cells, borders, neighbours, plan))
            })
            .expect("the zero-jitter layouts hold every prescribed land budget");
        let quotas = apportion(&plan.continents, &weights, g.minimum, continent_land);
        let mut terrain = vec![Terrain::Sea; n];
        let mut elevation = vec![0.0; n];
        let mut scores = Vec::with_capacity(plan.continents.iter().map(Vec::len).max().unwrap());
        for (b, candidates) in plan.continents.iter().enumerate() {
            basin_scores(
                candidates,
                &sites,
                centres[b],
                &broad,
                &fine,
                &mut elevation,
                &mut scores,
            );
            grow(candidates, &scores, &neighbours, quotas[b], &mut terrain);
        }
        for island in &plan.islands {
            basin_scores(
                island,
                &sites,
                [0.0, 0.0],
                &broad,
                &fine,
                &mut elevation,
                &mut scores,
            );
            for &r in island {
                terrain[r] = Terrain::Plains;
            }
        }
        let land: Vec<usize> = (0..n).filter(|&i| terrain[i].is_land()).collect();
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

        let landmass = landmasses(&terrain, &neighbours);
        let edges = travel_edges(&sites, &terrain, &borders);
        let regions: Vec<Region> = cells
            .into_iter()
            .zip(neighbours)
            .enumerate()
            .map(|(i, ((outline, _), neighbours))| Region {
                site: [sites[i][0] as f32, sites[i][1] as f32],
                outline: outline.iter().map(|&[x, y]| [x as f32, y as f32]).collect(),
                area_km2: (area(&outline) * f64::from(KM_PER_UNIT).powi(2)) as f32,
                terrain: terrain[i],
                neighbours,
                landmass: landmass[i],
            })
            .collect();
        let landmasses = describe_landmasses(&regions, size);
        assert_eq!(
            landmasses
                .iter()
                .filter(|m| m.kind == LandmassKind::Continent)
                .count(),
            k
        );
        assert!(landmasses.iter().all(|m| match m.kind {
            LandmassKind::Continent => m.regions.len() >= g.minimum,
            LandmassKind::Island => (1..=if g.island_count.is_some() { 2 } else { 3 }).contains(&m.regions.len()),
        }));
        if let Some(count) = g.island_count {
            assert_eq!(landmasses.iter().filter(|m| m.kind == LandmassKind::Island).count(), count);
        }
        let mut map = Map {
            size,
            width: width as f32,
            height: height as f32,
            feeding: feeding_factors(&regions),
            regions,
            landmasses,
            edges,
            walking: RouteRows::default(),
            voyages: RouteRows::default(),
        };
        (map.walking, map.voyages) = map.cache_routes();
        map
    }

    pub(crate) fn feeding_factor(&self, region: usize, livelihood: Livelihood) -> f32 {
        self.feeding[region][livelihood as usize]
    }

    /// Exact least land-only walking effort-km; sea endpoints are unreachable.
    pub fn distance(&self, a: usize, b: usize) -> f32 {
        if !self.regions[a].terrain.is_land() || !self.regions[b].terrain.is_land()
            || self.regions[a].landmass != self.regions[b].landmass
        {
            return f32::INFINITY;
        }
        self.route_pair(a, b, RouteMode::Walking)
    }

    /// Exact coast-to-coast voyage effort-km, with sea-only interiors.
    /// A voyage needs at least one sea cell; other coasts are never transit ports.
    pub fn voyage(&self, a: usize, b: usize) -> f32 {
        if a == b || !self.coastal(a) || !self.coastal(b) {
            return f32::INFINITY;
        }
        self.route_pair(a, b, RouteMode::Voyage)
    }

    /// ID-sorted reachable land within an inclusive effort-km radius.
    /// Infinity requests the full exact row, not the cached neighbourhood.
    pub fn walking_row(&self, source: usize, reach: f32) -> Cow<'_, [(u32, f32)]> {
        self.route_row(source, reach, RouteMode::Walking)
    }

    /// Full cached walking neighbourhood. Hot callers filter their own radius.
    pub(crate) fn walking_cached(&self, source: usize) -> &[(u32, f32)] {
        self.walking.row(source)
    }

    /// ID-sorted coastal destinations within an inclusive voyage radius.
    pub fn voyage_row(&self, source: usize, reach: f32) -> Cow<'_, [(u32, f32)]> {
        self.route_row(source, reach, RouteMode::Voyage)
    }

    /// Full cached voyage neighbourhood. Hot callers filter their own radius.
    pub(crate) fn voyage_cached(&self, source: usize) -> &[(u32, f32)] {
        self.voyages.row(source)
    }

    /// Stored walking and voyage tuple counts, excluding CSR offsets and graph.
    pub fn route_entry_counts(&self) -> (usize, usize) {
        (self.walking.entries.len(), self.voyages.entries.len())
    }

    fn route_pair(&self, a: usize, b: usize, mode: RouteMode) -> f32 {
        // One orientation gives bit-identical answers even beyond cache coverage.
        let (source, destination) = (a.min(b), a.max(b));
        let rows = match mode { RouteMode::Walking => &self.walking, RouteMode::Voyage => &self.voyages };
        if let Some(effort) = rows.get(source, destination) {
            return effort;
        }
        let mut scratch = RouteScratch::new(self.regions.len());
        self.search_routes(source, f32::INFINITY, mode, Some(destination), &mut scratch);
        scratch.distance[destination] as f32
    }

    fn route_row(&self, source: usize, reach: f32, mode: RouteMode) -> Cow<'_, [(u32, f32)]> {
        let eligible = match mode {
            RouteMode::Walking => self.regions[source].terrain.is_land(),
            RouteMode::Voyage => self.coastal(source),
        };
        if !eligible || !(reach >= 0.0) {
            return Cow::Borrowed(&[]);
        }
        if reach <= CACHE_REACH_KM {
            let row = match mode {
                RouteMode::Walking => self.walking.row(source),
                RouteMode::Voyage => self.voyages.row(source),
            };
            if row.iter().all(|&(_, effort)| effort <= reach) {
                return Cow::Borrowed(row);
            }
            return Cow::Owned(row.iter().copied().filter(|&(_, effort)| effort <= reach).collect());
        }
        let mut scratch = RouteScratch::new(self.regions.len());
        self.search_routes(source, reach, mode, None, &mut scratch);
        let mut row = Vec::new();
        scratch.append_row(self, source, mode, &mut row);
        Cow::Owned(row)
    }

    fn cache_routes(&self) -> (RouteRows, RouteRows) {
        let n = self.regions.len();
        let mut walking = RouteRows { offsets: Vec::with_capacity(n + 1), entries: Vec::new() };
        let mut voyages = RouteRows { offsets: Vec::with_capacity(n + 1), entries: Vec::new() };
        let mut scratch = RouteScratch::new(n);
        walking.offsets.push(0);
        voyages.offsets.push(0);
        for source in 0..n {
            if self.regions[source].terrain.is_land() {
                self.search_routes(source, CACHE_REACH_KM, RouteMode::Walking, None, &mut scratch);
                scratch.append_row(self, source, RouteMode::Walking, &mut walking.entries);
            }
            walking.offsets.push(walking.entries.len());
            if self.coastal(source) {
                self.search_routes(source, CACHE_REACH_KM, RouteMode::Voyage, None, &mut scratch);
                scratch.append_row(self, source, RouteMode::Voyage, &mut voyages.entries);
            }
            voyages.offsets.push(voyages.entries.len());
        }
        (walking, voyages)
    }

    fn search_routes(
        &self,
        source: usize,
        reach: f32,
        mode: RouteMode,
        target: Option<usize>,
        scratch: &mut RouteScratch,
    ) {
        scratch.reset();
        scratch.relax(source as u32, 0.0, reach);
        while let Some(RouteVisit { effort, region }) = scratch.heap.pop() {
            let r = region as usize;
            if effort != scratch.distance[r] {
                continue;
            }
            if target == Some(r) {
                break;
            }
            let land = self.regions[r].terrain.is_land();
            if mode == RouteMode::Voyage && land && r != source {
                continue;
            }
            for &(next, edge_effort) in self.edges.row(r) {
                let n = next as usize;
                let next_land = self.regions[n].terrain.is_land();
                let surcharge = match mode {
                    RouteMode::Walking if !next_land => continue,
                    RouteMode::Walking => 0.0,
                    RouteMode::Voyage if n == source || (land && next_land) => continue,
                    RouteMode::Voyage if land != next_land => f64::from(EMBARK),
                    RouteMode::Voyage => 0.0,
                };
                scratch.relax(next, effort + f64::from(edge_effort) + surcharge, reach);
            }
        }
    }

    /// How readily peoples on regions `a` and `b` meet as neighbours: 1 on
    /// the same land, less across a harder border, and 0 when the two
    /// share no land border.
    pub fn closeness(&self, a: usize, b: usize) -> f32 {
        let (ra, rb) = (&self.regions[a], &self.regions[b]);
        if a == b {
            return if ra.terrain.is_land() { 1.0 } else { 0.0 };
        }
        if !ra.terrain.is_land() || !rb.terrain.is_land() || !ra.neighbours.contains(&b) {
            return 0.0;
        }
        let effort = self.edges.get(a, b).expect("a neighbour has a retained shared border");
        (PLAIN_CLOSENESS * REFERENCE_TRAVEL_KM / effort).min(MAX_CLOSENESS)
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

    /// Whether going from region `a` to region `b` means crossing the sea.
    pub fn overseas(&self, a: usize, b: usize) -> bool {
        self.regions[a].landmass != self.regions[b].landmass
    }

    /// Whether region `r` is land on an island rather than a continent.
    pub fn island(&self, r: usize) -> bool {
        self.regions[r]
            .landmass
            .is_some_and(|m| self.landmasses[m].kind == LandmassKind::Island)
    }
}

fn feeding_factors(regions: &[Region]) -> Vec<[f32; 3]> {
    regions.iter().map(|r| Livelihood::ALL.map(|l| {
        l.feeds(r.terrain) * r.area_km2 / REFERENCE_AREA_KM2
    })).collect()
}

/// Grid rectangles are only candidate basins, never the finished coastline.
/// Their outer sea belt also supplies island sites when there is one basin.
fn layouts(g: &Geography, k: usize) -> Vec<Vec<Option<usize>>> {
    let central = |n: usize| (2 * n / 5)..=(3 * n / 5);
    let membership = |assign: &dyn Fn(usize, usize) -> Option<usize>| {
        (0..g.rows)
            .flat_map(|r| {
                (0..g.cols).map(move |c| {
                    if c == 0 || r == 0 || c + 1 == g.cols || r + 1 == g.rows {
                        None
                    } else {
                        assign(c, r)
                    }
                })
            })
            .collect()
    };
    let mut out = Vec::new();
    if k == 1 {
        out.push(membership(&|_, _| Some(0)));
    } else if k == 2 {
        for vertical in [true, false] {
            for cut in central(if vertical { g.cols } else { g.rows }) {
                out.push(membership(&|c, r| {
                    let p = if vertical { c } else { r };
                    (p != cut).then_some(usize::from(p > cut))
                }));
            }
        }
    } else {
        assert_eq!(k, 3);
        for vertical in [true, false] {
            for main_first in [true, false] {
                for full in central(if vertical { g.cols } else { g.rows }) {
                    for half in central(if vertical { g.rows } else { g.cols }) {
                        out.push(membership(&|c, r| {
                            let (p, q) = if vertical { (c, r) } else { (r, c) };
                            if p == full {
                                None
                            } else if (p < full) == main_first {
                                Some(0)
                            } else {
                                (q != half).then_some(1 + usize::from(q > half))
                            }
                        }));
                    }
                }
            }
        }
    }
    // A two-row basin near the edge can pass topology checks but turns its
    // minimum quota into a rectangular strip. Retain room for shaped coasts.
    out.retain(|owner: &Vec<Option<usize>>| {
        (0..k).all(|b| {
            let (mut x0, mut y0, mut x1, mut y1) = (g.cols, g.rows, 0, 0);
            for (r, &body) in owner.iter().enumerate() {
                if body == Some(b) {
                    x0 = x0.min(r % g.cols);
                    y0 = y0.min(r / g.cols);
                    x1 = x1.max(r % g.cols);
                    y1 = y1.max(r / g.cols);
                }
            }
            x1 >= x0 + 2 && y1 >= y0 + 2
        })
    });
    out
}

/// Eight row/column sweeps form a small, finite packing alternative to the
/// fully noisy island priorities. Reflections and transposition vary the
/// starting shore; the zero-jitter terminal uses the first sweep.
fn sweep_order(g: &Geography, orientation: usize) -> Vec<f64> {
    (0..g.cols * g.rows)
        .map(|r| {
            let (mut x, mut y) = (r % g.cols, r / g.cols);
            if orientation & 1 != 0 {
                x = g.cols - 1 - x;
            }
            if orientation & 2 != 0 {
                y = g.rows - 1 - y;
            }
            if orientation & 4 != 0 {
                (x * g.rows + y) as f64
            } else {
                (y * g.cols + x) as f64
            }
        })
        .collect()
}

fn sites(g: &Geography, offsets: &[[f64; 2]], jitter: f64) -> Vec<[f64; 2]> {
    offsets
        .iter()
        .enumerate()
        .map(|(i, &[dx, dy])| {
            let (c, r) = (i % g.cols, i / g.cols);
            [
                0.5 + c as f64 + if r % 2 == 1 { 0.5 } else { 0.0 } + jitter * dx,
                0.5 + r as f64 * ROW + jitter * dy,
            ]
        })
        .collect()
}

struct LandBudget {
    k: usize,
    minimum: usize,
    continents: usize,
    islands: usize,
    island_count: Option<usize>,
}

struct LandPlan {
    continents: Vec<Vec<usize>>,
    islands: Vec<Vec<usize>>,
}

fn choose_plan(
    layouts: &[Vec<Option<usize>>],
    start: usize,
    neighbours: &[Vec<usize>],
    budget: &LandBudget,
    island_order: &[f64],
) -> Option<LandPlan> {
    for offset in 0..layouts.len() {
        let owner = &layouts[(start + offset) % layouts.len()];
        // A grid separator alone is insufficient on a jittered Voronoi graph.
        // Remove BOTH endpoints of every edge crossing candidate basins.
        let mut moated = owner.clone();
        for (r, adjacent) in neighbours.iter().enumerate() {
            for &n in adjacent {
                if owner[r].is_some() && owner[n].is_some() && owner[r] != owner[n] {
                    moated[r] = None;
                    moated[n] = None;
                }
            }
        }
        let packs = island_packs(&moated, neighbours, island_order);
        let search = IslandSearch {
            owner: &moated,
            neighbours,
            budget,
            packs,
        };
        let mut blocked = vec![0u8; owner.len()];
        let mut islands = Vec::new();
        // Never spend exponential time exhausting an infeasible arrangement.
        // Move to the next finite layout, retaining the exact same budget.
        let mut alternatives = 1024;
        if let Some(continents) = search.find(
            budget.islands,
            0,
            &mut blocked,
            &mut islands,
            &mut alternatives,
        ) {
            return Some(LandPlan {
                continents,
                islands,
            });
        }
    }
    None
}

/// Connected packs of one, two, and three separator/outer-belt cells.
/// Every ordering tie is explicit; no hashing or random redraws are involved.
fn island_packs(
    owner: &[Option<usize>],
    neighbours: &[Vec<usize>],
    order: &[f64],
) -> [Vec<Vec<usize>>; 3] {
    let mut out = [Vec::new(), Vec::new(), Vec::new()];
    for r in 0..owner.len() {
        if owner[r].is_some() {
            continue;
        }
        out[0].push(vec![r]);
        for &n in &neighbours[r] {
            if n <= r || owner[n].is_some() {
                continue;
            }
            out[1].push(vec![r, n]);
            for &p in neighbours[r].iter().chain(&neighbours[n]) {
                if p == r || p == n || owner[p].is_some() {
                    continue;
                }
                let mut pack = vec![r, n, p];
                pack.sort_unstable();
                out[2].push(pack);
            }
        }
    }
    for packs in &mut out {
        packs.sort_unstable();
        packs.dedup();
        // Compact peripheral packs spare basin capacity. Noise breaks this
        // preference enough to vary island locations and shapes by seed.
        let score = |pack: &[usize]| {
            let mut rim = Vec::new();
            for &r in pack {
                rim.extend(
                    neighbours[r]
                        .iter()
                        .copied()
                        .filter(|&n| owner[n].is_some()),
                );
            }
            rim.sort_unstable();
            rim.dedup();
            rim.len() as f64 + 4.0 * pack.iter().map(|&r| order[r]).sum::<f64>() / pack.len() as f64
        };
        let mut scored: Vec<_> = packs.drain(..).map(|p| (score(&p), p)).collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        packs.extend(scored.into_iter().map(|(_, p)| p));
    }
    out
}

struct IslandSearch<'a> {
    owner: &'a [Option<usize>],
    neighbours: &'a [Vec<usize>],
    budget: &'a LandBudget,
    packs: [Vec<Vec<usize>>; 3],
}

impl IslandSearch<'_> {
    /// Bounded backtracking. Regional islands use packs up to three cells;
    /// fixed-count islands distribute their budget into one/two-cell packs.
    /// Equal-size packs have increasing indices, eliminating permutations.
    fn find(
        &self,
        remaining: usize,
        first: usize,
        blocked: &mut [u8],
        islands: &mut Vec<Vec<usize>>,
        alternatives: &mut usize,
    ) -> Option<Vec<Vec<usize>>> {
        if *alternatives == 0 {
            return None;
        }
        *alternatives -= 1;
        let continents = largest_components(self.owner, self.neighbours, blocked, self.budget.k);
        if continents.iter().any(|c| c.len() < self.budget.minimum)
            || continents.iter().map(Vec::len).sum::<usize>() < self.budget.continents
        {
            return None;
        }
        if remaining == 0 {
            if self.budget.island_count.is_some_and(|count| islands.len() != count) {
                return None;
            }
            return ocean_backbone(&continents, islands, self.neighbours).then_some(continents);
        }
        let size = if let Some(count) = self.budget.island_count {
            let slots = count.checked_sub(islands.len())?;
            if slots == 0 || remaining < slots || remaining > 2 * slots {
                return None;
            }
            if remaining > slots { 2 } else { 1 }
        } else {
            remaining.min(3)
        };
        for (p, pack) in self.packs[size - 1].iter().enumerate().skip(first) {
            if *alternatives == 0 {
                break;
            }
            if pack.iter().any(|&r| blocked[r] != 0) {
                continue;
            }
            let mut rim = pack.clone();
            for &r in pack {
                rim.extend(&self.neighbours[r]);
            }
            rim.sort_unstable();
            rim.dedup();
            for &r in &rim {
                blocked[r] += 1;
            }
            islands.push(pack.clone());
            let left = remaining - size;
            let next_size = if left == 0 {
                0
            } else if let Some(count) = self.budget.island_count {
                if left > count - islands.len() { 2 } else { 1 }
            } else {
                left.min(3)
            };
            let next = if next_size == size { p + 1 } else { 0 };
            if let Some(c) = self.find(remaining - size, next, blocked, islands, alternatives) {
                return Some(c);
            }
            islands.pop();
            for &r in &rim {
                blocked[r] -= 1;
            }
        }
        None
    }
}

fn largest_components(
    owner: &[Option<usize>],
    neighbours: &[Vec<usize>],
    blocked: &[u8],
    k: usize,
) -> Vec<Vec<usize>> {
    let mut largest = vec![Vec::new(); k];
    let mut seen = vec![false; owner.len()];
    let mut component = Vec::new();
    let mut stack = Vec::new();
    for start in 0..owner.len() {
        let Some(b) = owner[start] else { continue };
        if seen[start] || blocked[start] != 0 {
            continue;
        }
        component.clear();
        stack.push(start);
        seen[start] = true;
        while let Some(r) = stack.pop() {
            component.push(r);
            for &n in &neighbours[r] {
                if !seen[n] && blocked[n] == 0 && owner[n] == Some(b) {
                    seen[n] = true;
                    stack.push(n);
                }
            }
        }
        component.sort_unstable();
        if component.len() > largest[b].len()
            || (component.len() == largest[b].len() && component[0] < largest[b][0])
        {
            std::mem::swap(&mut component, &mut largest[b]);
        }
    }
    largest
}

/// The reserved sea (including discarded basin fragments) is connected.
/// No island can cut the separator into disconnected ocean pockets.
fn ocean_backbone(
    continents: &[Vec<usize>],
    islands: &[Vec<usize>],
    neighbours: &[Vec<usize>],
) -> bool {
    let mut sea = vec![true; neighbours.len()];
    for &r in continents.iter().chain(islands).flatten() {
        sea[r] = false;
    }
    let Some(start) = sea.iter().position(|&s| s) else {
        return false;
    };
    let mut stack = vec![start];
    sea[start] = false;
    while let Some(r) = stack.pop() {
        for &n in &neighbours[r] {
            if sea[n] {
                sea[n] = false;
                stack.push(n);
            }
        }
    }
    !sea.into_iter().any(|s| s)
}

/// Capped largest-remainder apportionment, reweighted when a basin binds.
fn apportion(
    candidates: &[Vec<usize>],
    weights: &[f64],
    minimum: usize,
    total: usize,
) -> Vec<usize> {
    let mut quotas = vec![minimum; candidates.len()];
    let mut remaining = total - minimum * candidates.len();
    while remaining != 0 {
        let weighted: Vec<f64> = candidates
            .iter()
            .zip(&quotas)
            .zip(weights)
            .map(|((c, &q), &w)| (c.len() - q) as f64 * w)
            .collect();
        let sum: f64 = weighted.iter().sum();
        let ideals: Vec<f64> = weighted
            .iter()
            .map(|&w| remaining as f64 * w / sum)
            .collect();
        if let Some(b) =
            (0..quotas.len()).find(|&b| ideals[b] > (candidates[b].len() - quotas[b]) as f64)
        {
            remaining -= candidates[b].len() - quotas[b];
            quotas[b] = candidates[b].len();
            continue;
        }
        for (q, &ideal) in quotas.iter_mut().zip(&ideals) {
            let whole = ideal as usize;
            *q += whole;
            remaining -= whole;
        }
        let mut order: Vec<usize> = (0..quotas.len()).collect();
        order.sort_by(|&a, &b| {
            let fraction = |r: usize| ideals[r] - (ideals[r] as usize) as f64;
            fraction(b).total_cmp(&fraction(a)).then(a.cmp(&b))
        });
        for b in order {
            if remaining == 0 {
                break;
            }
            if quotas[b] < candidates[b].len() {
                quotas[b] += 1;
                remaining -= 1;
            }
        }
    }
    quotas
}

fn basin_scores(
    candidates: &[usize],
    sites: &[[f64; 2]],
    displacement: [f64; 2],
    broad: &Noise,
    fine: &Noise,
    elevation: &mut [f64],
    scores: &mut Vec<f64>,
) {
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for &r in candidates {
        let [x, y] = sites[r];
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (w, h) = (x1 - x0 + 1.0, y1 - y0 + ROW);
    let (cx, cy) = (
        (x0 + x1) / 2.0 + displacement[0] * w,
        (y0 + y1) / 2.0 + displacement[1] * h,
    );
    scores.clear();
    for &r in candidates {
        let [x, y] = sites[r];
        let (dx, dy) = ((x - cx) / (w / 2.0), (y - cy) / (h / 2.0));
        let shape = 1.0 - dx * dx - dy * dy;
        let (b, f) = (broad.at(x, y), fine.at(x, y));
        scores.push(shape + 0.25 * b + 0.20 * f);
        // Keep relief correlated with the basin without assigning all the
        // mountain quota to the centres of tiny islands.
        elevation[r] = b + 0.7 * f + 0.15 * shape;
    }
}

/// Highest-scored frontier first, with region-ID ties. Connectivity and
/// sufficient candidate capacity guarantee that the quota is reachable.
fn grow(
    candidates: &[usize],
    scores: &[f64],
    neighbours: &[Vec<usize>],
    quota: usize,
    terrain: &mut [Terrain],
) {
    let mut value = vec![None; terrain.len()];
    for (&r, &score) in candidates.iter().zip(scores) {
        value[r] = Some(score);
    }
    let compare = |a: usize, b: usize| {
        value[a]
            .unwrap()
            .total_cmp(&value[b].unwrap())
            .then(b.cmp(&a))
    };
    let start = *candidates
        .iter()
        .max_by(|&&a, &&b| compare(a, b))
        .expect("a basin has candidates");
    let mut frontier = vec![start];
    let mut queued = vec![false; terrain.len()];
    queued[start] = true;
    for _ in 0..quota {
        let best = frontier
            .iter()
            .enumerate()
            .max_by(|a, b| compare(*a.1, *b.1))
            .map(|(i, _)| i)
            .expect("a connected basin can fill its quota");
        let r = frontier.swap_remove(best);
        terrain[r] = Terrain::Plains;
        for &n in &neighbours[r] {
            if value[n].is_some() && !queued[n] {
                frontier.push(n);
                queued[n] = true;
            }
        }
    }
}

/// Each land region's body of land, numbered in order of its lowest
/// region; `None` for sea.
fn landmasses(terrain: &[Terrain], neighbours: &[Vec<usize>]) -> Vec<Option<usize>> {
    let mut out: Vec<Option<usize>> = vec![None; terrain.len()];
    let mut next = 0;
    for start in 0..terrain.len() {
        if !terrain[start].is_land() || out[start].is_some() {
            continue;
        }
        out[start] = Some(next);
        let mut stack = vec![start];
        while let Some(r) = stack.pop() {
            for &n in &neighbours[r] {
                if terrain[n].is_land() && out[n].is_none() {
                    out[n] = Some(next);
                    stack.push(n);
                }
            }
        }
        next += 1;
    }
    out
}

/// Each body of land's kind, regions, and anchor, by its number.
fn describe_landmasses(regions: &[Region], size: MapSize) -> Vec<Landmass> {
    let count = regions
        .iter()
        .filter_map(|r| r.landmass)
        .max()
        .map_or(0, |m| m + 1);
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (i, r) in regions.iter().enumerate() {
        if let Some(m) = r.landmass {
            members[m].push(i);
        }
    }
    members
        .into_iter()
        .map(|regions_of| {
            // Centre of area: each cell's centroid weighted by its area.
            let (mut area, mut cx, mut cy) = (0.0f64, 0.0f64, 0.0f64);
            for &r in &regions_of {
                let (a, x, y) = centroid(&regions[r].outline);
                area += a;
                cx += a * x;
                cy += a * y;
            }
            let (cx, cy) = if area > 0.0 {
                (cx / area, cy / area)
            } else {
                (0.0, 0.0)
            };
            let anchor = regions_of
                .iter()
                .copied()
                .min_by(|&a, &b| {
                    let d = |r: usize| {
                        let [x, y] = regions[r].site;
                        let (dx, dy) = (f64::from(x) - cx, f64::from(y) - cy);
                        dx * dx + dy * dy
                    };
                    d(a).total_cmp(&d(b)).then(a.cmp(&b))
                })
                .expect("a landmass has a region");
            let kind = if regions_of.len() >= size.continent_minimum() {
                LandmassKind::Continent
            } else {
                LandmassKind::Island
            };
            Landmass {
                kind,
                regions: regions_of,
                anchor,
            }
        })
        .collect()
}

/// A polygon's area and centroid, by the shoelace formula.
fn centroid(outline: &[[f32; 2]]) -> (f64, f64, f64) {
    let (mut a, mut x, mut y) = (0.0f64, 0.0f64, 0.0f64);
    for (i, &[x0, y0]) in outline.iter().enumerate() {
        let [x1, y1] = outline[(i + 1) % outline.len()];
        let (x0, y0, x1, y1) = (f64::from(x0), f64::from(y0), f64::from(x1), f64::from(y1));
        let cross = x0 * y1 - x1 * y0;
        a += cross;
        x += (x0 + x1) * cross;
        y += (y0 + y1) * cross;
    }
    if a == 0.0 {
        return (0.0, 0.0, 0.0);
    }
    (a.abs() / 2.0, x / (3.0 * a), y / (3.0 * a))
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

#[derive(Clone, Debug)]
struct CellBorder {
    neighbour: usize,
    ends: [[f64; 2]; 2],
}

#[derive(Clone, Debug)]
struct SharedBorder {
    a: usize,
    b: usize,
    midpoint: [f64; 2],
}

fn area(outline: &[[f64; 2]]) -> f64 {
    outline.iter().enumerate().map(|(i, &[x0, y0])| {
        let [x1, y1] = outline[(i + 1) % outline.len()];
        x0 * y1 - x1 * y0
    }).sum::<f64>().abs() / 2.0
}

/// The unjittered rectangle is covered within sqrt(1^2 + 0.5^2) of a
/// grid site: choose the nearest row, then its nearest point. Jitter adds
/// at most sqrt(2)*0.3, so every true cell lies within 1.55 of its site.
/// A site supplying a positive Voronoi facet is therefore at most 3.1 away.
/// Omitted columns (delta >= 5) are at least 5-0.5-0.6 = 3.9 away;
/// omitted rows are at least 5*ROW-0.6 > 3.7 away. Keeping +/-4 of each
/// includes every true facet, hence exactly the full half-plane intersection.
/// Region-ID clipping order remains the same as the exhaustive clipper.
fn grid_cells(
    g: &Geography,
    sites: &[[f64; 2]],
    width: f64,
    height: f64,
) -> Vec<(Vec<[f64; 2]>, Vec<CellBorder>)> {
    (0..sites.len()).map(|i| {
        let (c, r) = (i % g.cols, i / g.cols);
        let candidates = (r.saturating_sub(4)..=(r + 4).min(g.rows - 1)).flat_map(|row| {
            (c.saturating_sub(4)..=(c + 4).min(g.cols - 1)).map(move |col| row * g.cols + col)
        });
        clipped_cell(sites, i, width, height, candidates)
    }).collect()
}

#[cfg(test)]
fn cell(sites: &[[f64; 2]], i: usize, width: f64, height: f64) -> (Vec<[f64; 2]>, Vec<CellBorder>) {
    clipped_cell(sites, i, width, height, 0..sites.len())
}

/// The Voronoi cell of site `i` within the map's bounds, and the sites
/// whose cells it borders: the bounds clipped by the half-plane nearer `i`
/// than each other site. Each edge of the polygon remembers the site that
/// cut it, or none for the map's edge.
fn clipped_cell(
    sites: &[[f64; 2]],
    i: usize,
    width: f64,
    height: f64,
    candidates: impl Iterator<Item = usize>,
) -> (Vec<[f64; 2]>, Vec<CellBorder>) {
    let mut polygon: Vec<([f64; 2], Option<usize>)> = vec![
        ([0.0, 0.0], None),
        ([width, 0.0], None),
        ([width, height], None),
        ([0.0, height], None),
    ];
    let [px, py] = sites[i];
    let mut next = Vec::with_capacity(12);
    for j in candidates {
        if j == i {
            continue;
        }
        let [qx, qy] = sites[j];
        // Points p with (p - m)·n <= 0 are nearer site i.
        let (nx, ny) = (qx - px, qy - py);
        let (mx, my) = ((px + qx) / 2.0, (py + qy) / 2.0);
        let side = |[x, y]: [f64; 2]| (x - mx) * nx + (y - my) * ny;
        next.clear();
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
        std::mem::swap(&mut polygon, &mut next);
    }
    let borders = (0..polygon.len())
        .filter_map(|k| {
            let (a, label) = polygon[k];
            let (b, _) = polygon[(k + 1) % polygon.len()];
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            // Keep the existing sliver cutoff, but apply it to unrounded
            // labelled edges. A surviving side supplies the canonical border.
            label.filter(|_| dx * dx + dy * dy > 1e-6)
                .map(|neighbour| CellBorder { neighbour, ends: [a, b] })
        })
        .collect();
    (polygon.into_iter().map(|(p, _)| p).collect(), borders)
}

/// Retain one positive border for each unordered pair before f32 rounding.
/// Prefer the lower-ID cell's side; a one-sided clipping sliver still yields
/// exactly one symmetric edge rather than independently recovered geometry.
fn shared_borders(cells: &[(Vec<[f64; 2]>, Vec<CellBorder>)]) -> Vec<SharedBorder> {
    let mut sides: Vec<_> = cells.iter().enumerate().flat_map(|(i, (_, borders))| {
        borders.iter().map(move |border| {
            (i.min(border.neighbour), i.max(border.neighbour), i, border.ends)
        })
    }).collect();
    sides.sort_unstable_by_key(|&(a, b, source, _)| (a, b, source));
    sides.dedup_by_key(|side| (side.0, side.1));
    sides.into_iter().map(|(a, b, _, [p, q])| SharedBorder {
        a, b, midpoint: [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0],
    }).collect()
}

fn border_neighbours(n: usize, borders: &[SharedBorder]) -> Vec<Vec<usize>> {
    let mut neighbours = vec![Vec::new(); n];
    for border in borders {
        neighbours[border.a].push(border.b);
        neighbours[border.b].push(border.a);
    }
    for row in &mut neighbours {
        row.sort_unstable();
    }
    neighbours
}

fn travel_edges(
    sites: &[[f64; 2]],
    terrain: &[Terrain],
    borders: &[SharedBorder],
) -> RouteRows {
    let mut offsets = vec![0; sites.len() + 1];
    for border in borders {
        offsets[border.a + 1] += 1;
        offsets[border.b + 1] += 1;
    }
    for r in 0..sites.len() {
        offsets[r + 1] += offsets[r];
    }
    let mut entries = vec![(0, 0.0); 2 * borders.len()];
    let mut cursor = offsets[..sites.len()].to_vec();
    for border in borders {
        let leg = |r: usize| {
            let [x, y] = sites[r];
            let [mx, my] = border.midpoint;
            ((x - mx).powi(2) + (y - my).powi(2)).sqrt()
        };
        let effort = (f64::from(KM_PER_UNIT)
            * (leg(border.a) * f64::from(terrain[border.a].travel())
                + leg(border.b) * f64::from(terrain[border.b].travel()))) as f32;
        for (from, to) in [(border.a, border.b), (border.b, border.a)] {
            entries[cursor[from]] = (to as u32, effort);
            cursor[from] += 1;
        }
    }
    for r in 0..sites.len() {
        entries[offsets[r]..offsets[r + 1]].sort_unstable_by_key(|&(id, _)| id);
    }
    RouteRows { offsets, entries }
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


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn border_midpoints_define_reference_and_skewed_effort() {
        let g = Geography {
            cols: 5, rows: 5, continents: (1, 1), continent_land: (1, 1), minimum: 1,
            island_count: None,
        };
        let points = sites(&g, &[[0.0, 0.0]; 25], 0.0);
        let cells: Vec<_> = (0..points.len()).map(|i| cell(&points, i, 5.5, 4.0 * ROW + 1.0)).collect();
        let borders = shared_borders(&cells);
        for (terrain, expected) in [(Terrain::Plains, 100.0), (Terrain::Steppe, 80.0)] {
            let edges = travel_edges(&points, &vec![terrain; points.len()], &borders);
            let effort = edges.get(12, 13).unwrap();
            assert!((effort - expected).abs() < 1e-4);
        }

        let points = [[0.5, 0.5], [1.5, 0.5], [1.0, 1.4]];
        let cells: Vec<_> = (0..points.len()).map(|i| cell(&points, i, 2.0, 2.0)).collect();
        let borders = shared_borders(&cells);
        let border = borders.iter().find(|b| (b.a, b.b) == (0, 1)).unwrap();
        let [mx, my] = border.midpoint;
        let leg = ((mx - 0.5).powi(2) + (my - 0.5).powi(2)).sqrt();
        assert!(leg > 0.5);
        let edges = travel_edges(&points, &[Terrain::Plains, Terrain::Hills, Terrain::Sea], &borders);
        let effort = edges.get(0, 1).unwrap();
        assert!((f64::from(effort) - 100.0 * leg * 3.0).abs() < 1e-4);
        assert_eq!(effort, edges.get(1, 0).unwrap());
    }


    #[test]
    fn bounded_grid_clipping_preserves_exhaustive_areas_and_borders() {
        for size in [MapSize::Small, MapSize::Medium, MapSize::Large, MapSize::Vast] {
            let g = size.geography();
            let n = g.cols * g.rows;
            let width = g.cols as f64 + 0.5;
            let height = (g.rows - 1) as f64 * ROW + 1.0;
            for seed in 0..3 {
                let mut rng = stream(seed, &[key("clipper oracle")]);
                let offsets: Vec<_> = (0..n).map(|r| {
                    if seed == 0 {
                        // Extreme opposing offsets exercise the exclusion bound,
                        // including map corners and alternating offset rows.
                        [if r % 2 == 0 { -1.0 } else { 1.0 },
                            if (r / g.cols) % 2 == 0 { 1.0 } else { -1.0 }]
                    } else {
                        [rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)]
                    }
                }).collect();
                for jitter in [JITTER, JITTER / 2.0, 0.0] {
                    let sites = sites(g, &offsets, jitter);
                    let bounded = grid_cells(g, &sites, width, height);
                    let exhaustive: Vec<_> = (0..n).map(|r| cell(&sites, r, width, height)).collect();
                    for r in 0..n {
                        assert!((area(&bounded[r].0) - area(&exhaustive[r].0)).abs() < 1e-9,
                            "{size:?}, seed {seed}, jitter {jitter}, region {r}");
                    }
                    let bounded = shared_borders(&bounded);
                    let exhaustive = shared_borders(&exhaustive);
                    assert_eq!(bounded.len(), exhaustive.len());
                    for (a, b) in bounded.iter().zip(&exhaustive) {
                        assert_eq!((a.a, a.b), (b.a, b.b));
                        assert!((a.midpoint[0] - b.midpoint[0]).abs() < 1e-8);
                        assert!((a.midpoint[1] - b.midpoint[1]).abs() < 1e-8);
                    }
                }
            }
        }
    }

    #[test]
    fn regions_tile_the_map_and_border_each_other_both_ways() {
        for size in [MapSize::Small, MapSize::Medium, MapSize::Large, MapSize::Vast] {
            let map = Map::generate(7, size);
            let area: f64 = map.regions.iter().map(|r| centroid(&r.outline).0).sum();
            let physical_area: f64 = map.regions.iter().map(|r| f64::from(r.area_km2)).sum();
            let rectangle = f64::from(map.width) * f64::from(map.height) * 10_000.0;
            assert!((physical_area - rectangle).abs() / rectangle < 1e-6);
            assert!(map.regions.iter().all(|r| r.area_km2 > 0.0));
            assert!(
                (area - f64::from(map.width) * f64::from(map.height)).abs() < 1e-2,
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
    fn continents_and_islands_obey_the_regional_budgets_and_graph() {
        for (size, k_range, land_range, minimum) in [
            (MapSize::Small, 1..=1, 20..=27, 20),
            (MapSize::Medium, 1..=2, 45..=55, 20),
            (MapSize::Large, 2..=3, 90..=110, 25),
            (MapSize::Vast, 2..=2, 1_794..=1_797, 800),
        ] {
            let mut startup_ms = Vec::new();
            let mut payload_counts = Vec::new();
            let mut body_sizes = Vec::new();
            for seed in 0..128 {
                let start = std::time::Instant::now();
                let map = Map::generate(seed, size);
                startup_ms.push(start.elapsed().as_secs_f64() * 1_000.0);
                payload_counts.push(map.route_entry_counts());
                assert_eq!(map, Map::generate(seed, size), "{size:?}, seed {seed}");
                let continents: Vec<_> = map.landmasses.iter()
                    .filter(|m| m.kind == LandmassKind::Continent).collect();
                body_sizes.extend(continents.iter().map(|m| m.regions.len()));
                let islands: Vec<_> = map.landmasses.iter()
                    .filter(|m| m.kind == LandmassKind::Island).collect();
                assert!(k_range.contains(&continents.len()), "{size:?}, seed {seed}");
                assert!(
                    land_range.contains(&continents.iter().map(|m| m.regions.len()).sum::<usize>())
                );
                assert!(continents.iter().all(|m| m.regions.len() >= minimum));
                assert!(!islands.is_empty());
                assert!(islands.iter().all(|m| (1..=3).contains(&m.regions.len())));
                if size == MapSize::Vast {
                    assert_eq!(map.regions.len(), 3_600);
                    assert_eq!(islands.len(), 3);
                    assert!(islands.iter().all(|m| (1..=2).contains(&m.regions.len())));
                    let area: f64 = map.regions.iter().map(|r| f64::from(r.area_km2)).sum();
                    let rectangle = f64::from(map.width) * f64::from(map.height) * 10_000.0;
                    assert!((area - rectangle).abs() / rectangle < 1e-6);
                }
                // The first sea region is on the reserved outer ocean belt.
                // Every main body must have a coast on that same backbone,
                // not merely border an isolated inland lake.
                let start = map
                    .regions
                    .iter()
                    .position(|r| r.terrain == Terrain::Sea)
                    .unwrap();
                let mut ocean = vec![false; map.regions.len()];
                let mut stack = vec![start];
                ocean[start] = true;
                while let Some(r) = stack.pop() {
                    for &n in &map.regions[r].neighbours {
                        if !ocean[n] && map.regions[n].terrain == Terrain::Sea {
                            ocean[n] = true;
                            stack.push(n);
                        }
                    }
                }
                assert!(continents.iter().all(|m| {
                    m.regions
                        .iter()
                        .any(|&r| map.regions[r].neighbours.iter().any(|&n| ocean[n]))
                }));

                // Independently traverse the final land graph and compare the
                // complete membership, not just the count or stored IDs.
                let mut seen = vec![false; map.regions.len()];
                let mut components = Vec::new();
                for start in 0..map.regions.len() {
                    if seen[start] || !map.regions[start].terrain.is_land() {
                        continue;
                    }
                    let mut stack = vec![start];
                    let mut members = Vec::new();
                    seen[start] = true;
                    while let Some(r) = stack.pop() {
                        members.push(r);
                        for &n in &map.regions[r].neighbours {
                            if !seen[n] && map.regions[n].terrain.is_land() {
                                seen[n] = true;
                                stack.push(n);
                            }
                        }
                    }
                    members.sort_unstable();
                    components.push(members);
                }
                assert_eq!(components.len(), map.landmasses.len());
                for (id, (members, mass)) in components.iter().zip(&map.landmasses).enumerate() {
                    assert_eq!(members, &mass.regions);
                    assert!(members.contains(&mass.anchor));
                    assert!(members.iter().all(|&r| map.regions[r].landmass == Some(id)));
                    assert!(members.iter().any(|&r| map.coastal(r)));
                    for &r in members {
                        assert_eq!(map.island(r), mass.kind == LandmassKind::Island);
                    }
                }
                for (r, region) in map.regions.iter().enumerate() {
                    assert_eq!(region.landmass.is_some(), region.terrain.is_land());
                    assert!(region.neighbours.windows(2).all(|pair| pair[0] < pair[1]));
                    for &n in &region.neighbours {
                        assert!(map.regions[n].neighbours.contains(&r));
                        if region.terrain.is_land() && map.regions[n].terrain.is_land() {
                            assert_eq!(region.landmass, map.regions[n].landmass);
                            assert!(!map.overseas(r, n));
                        }
                    }
                }
                let count = |t| map.regions.iter().filter(|r| r.terrain == t).count();
                let land = map.regions.len() - share(map.regions.len(), SEA);
                let peaks = share(land, MOUNTAINS);
                let hills = share(land, HILLS);
                let lowlands = land - peaks - hills;
                assert_eq!(count(Terrain::Sea), share(map.regions.len(), SEA));
                assert_eq!(count(Terrain::Mountains), peaks);
                assert_eq!(count(Terrain::Hills), hills);
                assert_eq!(count(Terrain::Desert), share(lowlands, DESERT));
                assert_eq!(count(Terrain::Steppe), share(lowlands, STEPPE));
                assert_eq!(count(Terrain::Forest), share(lowlands, FOREST));
                assert_eq!(
                    count(Terrain::Plains),
                    lowlands
                        - share(lowlands, DESERT)
                        - share(lowlands, STEPPE)
                        - share(lowlands, FOREST)
                );
            }
            startup_ms.sort_by(f64::total_cmp);
            eprintln!("{size:?}: 128 seeds, startup median={:.2}ms p95={:.2}ms; \
                body cells={}..{}; walking tuples={}..{}, voyage tuples={}..{}",
                startup_ms[64], startup_ms[121],
                body_sizes.iter().min().unwrap(), body_sizes.iter().max().unwrap(),
                payload_counts.iter().map(|p| p.0).min().unwrap(),
                payload_counts.iter().map(|p| p.0).max().unwrap(),
                payload_counts.iter().map(|p| p.1).min().unwrap(),
                payload_counts.iter().map(|p| p.1).max().unwrap());
        }
    }

    #[test]
    fn continental_shares_reapportion_when_multiple_basins_fill() {
        let candidates = [vec![0; 25], vec![0; 70], vec![0; 30]];
        assert_eq!(
            apportion(&candidates, &[1.25, 0.75, 1.25], 20, 120),
            [25, 65, 30]
        );
    }

    #[test]
    fn zero_jitter_is_terminal_for_every_size_count_and_budget() {
        for size in [MapSize::Small, MapSize::Medium, MapSize::Large, MapSize::Vast] {
            let g = size.geography();
            let n = g.cols * g.rows;
            let sites = sites(g, &vec![[0.0, 0.0]; n], 0.0);
            let width = g.cols as f64 + 0.5;
            let height = (g.rows - 1) as f64 * ROW + 1.0;
            let cells = grid_cells(g, &sites, width, height);
            let neighbours = border_neighbours(n, &shared_borders(&cells));
            for k in g.continents.0..=g.continents.1 {
                let layouts = layouts(g, k);
                for continents in g.continent_land.0..=g.continent_land.1 {
                    let budget = LandBudget {
                        k,
                        minimum: g.minimum,
                        continents,
                        islands: n - share(n, SEA) - continents,
                        island_count: g.island_count,
                    };
                    let order = sweep_order(g, 0);
                    let plan = choose_plan(&layouts, 0, &neighbours, &budget, &order)
                        .unwrap_or_else(|| panic!("{size:?}, K={k}, land={continents}"));
                    let quotas = apportion(&plan.continents, &vec![1.0; k], g.minimum, continents);
                    let mut terrain = vec![Terrain::Sea; n];
                    for (c, &quota) in plan.continents.iter().zip(&quotas) {
                        grow(c, &vec![0.0; c.len()], &neighbours, quota, &mut terrain);
                    }
                    for &r in plan.islands.iter().flatten() {
                        terrain[r] = Terrain::Plains;
                    }
                    assert_eq!(
                        terrain.iter().filter(|&&t| t == Terrain::Sea).count(),
                        share(n, SEA)
                    );
                    let membership = landmasses(&terrain, &neighbours);
                    let mut sizes = vec![0; k + plan.islands.len()];
                    for m in membership.into_iter().flatten() {
                        sizes[m] += 1;
                    }
                    assert_eq!(sizes.iter().filter(|&&s| s >= g.minimum).count(), k);
                    assert_eq!(sizes.iter().filter(|&&s| s >= g.minimum).sum::<usize>(), continents);
                    assert!(sizes.iter().filter(|&&s| s < g.minimum).all(|&s| (1..=3).contains(&s)));
                    assert_eq!(plan.islands.iter().map(Vec::len).sum::<usize>(), budget.islands);
                    if let Some(count) = g.island_count {
                        assert_eq!(plan.islands.len(), count);
                        assert!(plan.islands.iter().all(|island| (1..=2).contains(&island.len())));
                    }
                    assert!(ocean_backbone(&plan.continents, &plan.islands, &neighbours));
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
    fn sparse_routes_match_independent_oracles_and_inclusive_radii() {
        let map = Map::generate(6, MapSize::Small);
        let n = map.regions.len();
        let walk = floyd_oracle(&map, true);
        let sea = floyd_oracle(&map, false);
        for source in 0..n {
            let mut voyages = vec![f32::INFINITY; n];
            if map.coastal(source) {
                for (destination, effort) in voyages.iter_mut().enumerate() {
                    if source == destination || !map.coastal(destination) { continue; }
                    for &(depart, first) in map.edges.row(source) {
                        if map.regions[depart as usize].terrain.is_land() { continue; }
                        for &(arrive, last) in map.edges.row(destination) {
                            if map.regions[arrive as usize].terrain.is_land() { continue; }
                            let total = f64::from(first) + 100.0
                                + sea[depart as usize * n + arrive as usize]
                                + f64::from(last) + 100.0;
                            *effort = effort.min(total as f32);
                        }
                    }
                }
            }
            for destination in 0..n {
                assert_eq!(map.distance(source, destination), walk[source * n + destination] as f32);
                assert_eq!(map.distance(source, destination), map.distance(destination, source));
                assert_eq!(map.voyage(source, destination), voyages[destination]);
                assert_eq!(map.voyage(source, destination), map.voyage(destination, source));
            }
            for reach in [0.0, 400.0, CACHE_REACH_KM, f32::INFINITY] {
                let expected_walk: Vec<_> = (0..n).filter_map(|r| {
                    let effort = walk[source * n + r] as f32;
                    (effort.is_finite() && effort <= reach).then_some((r as u32, effort))
                }).collect();
                let expected_voyage: Vec<_> = voyages.iter().enumerate().filter_map(|(r, &effort)| {
                    (effort.is_finite() && effort <= reach).then_some((r as u32, effort))
                }).collect();
                assert_eq!(map.walking_row(source, reach).as_ref(), expected_walk);
                assert_eq!(map.voyage_row(source, reach).as_ref(), expected_voyage);
            }
        }
    }

    #[test]
    fn walking_takes_the_land_detour_and_exact_fallback_exceeds_cache() {
        let map = route_fixture(
            &[Terrain::Plains, Terrain::Sea, Terrain::Plains, Terrain::Plains,
                Terrain::Plains, Terrain::Plains, Terrain::Plains],
            &[(0, 1, 200.0), (1, 2, 200.0), (0, 3, 100.0), (3, 4, 100.0),
                (4, 5, 100.0), (5, 2, 100.0)],
        );
        assert_eq!(map.distance(0, 2), 400.0);
        assert_eq!(map.regions[0].landmass, map.regions[2].landmass);
        assert_eq!(map.voyage(0, 2), 600.0);
        assert_eq!(map.distance(0, 0), 0.0);
        assert_eq!(map.closeness(1, 1), 0.0);
        for (a, b) in [(0, 1), (1, 1), (1, 2), (0, 6)] {
            assert!(map.distance(a, b).is_infinite());
        }
        assert!(map.walking_row(1, f32::INFINITY).is_empty());

        let map = linear_map(&[Terrain::Plains; 25]);
        assert_eq!(map.distance(0, 24), 2_400.0);
        assert_eq!(map.distance(24, 0), 2_400.0);
        assert!(!map.walking_row(0, CACHE_REACH_KM).iter().any(|&(r, _)| r == 24));
        assert!(map.walking_row(0, 2_400.0).contains(&(24, 2_400.0)));
        assert!(!map.walking_row(0, 2_399.0).iter().any(|&(r, _)| r == 24));
        assert!(map.walking_row(0, f32::INFINITY).contains(&(24, 2_400.0)));
    }

    #[test]
    fn voyages_charge_transitions_and_never_rent_intermediate_ports() {
        for (sea_cells, expected) in [(1, 600.0), (2, 900.0), (3, 1_200.0), (6, 2_100.0)] {
            let mut terrain = vec![Terrain::Sea; sea_cells + 2];
            terrain[0] = Terrain::Plains;
            terrain[sea_cells + 1] = Terrain::Plains;
            let map = linear_map(&terrain);
            let end = terrain.len() - 1;
            assert_eq!(map.voyage(0, end), expected);
            assert_eq!(map.voyage(end, 0), expected);
            assert!(map.distance(0, end).is_infinite());
            assert!(map.voyage(0, 0).is_infinite());
            assert!(map.voyage(1, end).is_infinite());
            assert!(map.voyage_row(0, expected).contains(&(end as u32, expected)));
            assert!(map.voyage_row(0, expected - 1.0).is_empty());
            if expected > CACHE_REACH_KM {
                assert!(map.voyage_row(0, CACHE_REACH_KM).is_empty());
                assert!(map.voyage_row(0, f32::INFINITY).contains(&(end as u32, expected)));
            }
            terrain[0] = Terrain::Mountains;
            assert_eq!(linear_map(&terrain).voyage(0, end), expected + 150.0);
        }
        let port = linear_map(&[
            Terrain::Plains, Terrain::Sea, Terrain::Plains, Terrain::Sea, Terrain::Plains,
        ]);
        assert_eq!(port.voyage(0, 2), 600.0);
        assert!(port.voyage(0, 4).is_infinite());
        let inland = linear_map(&[
            Terrain::Plains, Terrain::Plains, Terrain::Sea, Terrain::Plains,
        ]);
        assert!(inland.voyage(0, 3).is_infinite());
        assert_eq!(inland.voyage(1, 3), 600.0);
    }

    fn floyd_oracle(map: &Map, land: bool) -> Vec<f64> {
        let n = map.regions.len();
        let mut distance = vec![f64::INFINITY; n * n];
        for r in 0..n {
            if map.regions[r].terrain.is_land() != land { continue; }
            distance[r * n + r] = 0.0;
            for &(s, effort) in map.edges.row(r) {
                if map.regions[s as usize].terrain.is_land() == land {
                    distance[r * n + s as usize] = f64::from(effort);
                }
            }
        }
        for k in 0..n {
            for a in 0..n {
                for b in 0..n {
                    distance[a * n + b] = distance[a * n + b]
                        .min(distance[a * n + k] + distance[k * n + b]);
                }
            }
        }
        distance
    }

    fn linear_map(terrain: &[Terrain]) -> Map {
        let sites: Vec<_> = (0..terrain.len()).map(|r| [r as f64 + 0.5, 0.5]).collect();
        let cells: Vec<_> = (0..terrain.len()).map(|r| cell(&sites, r, terrain.len() as f64, 1.0)).collect();
        let borders = shared_borders(&cells);
        let edges = travel_edges(&sites, terrain, &borders);
        let links: Vec<_> = (0..terrain.len()).flat_map(|r| {
            edges.row(r).iter().copied().filter(move |&(s, _)| r < s as usize)
                .map(move |(s, effort)| (r, s as usize, effort))
        }).collect();
        route_fixture(terrain, &links)
    }

    fn route_fixture(terrain: &[Terrain], links: &[(usize, usize, f32)]) -> Map {
        let n = terrain.len();
        let mut adjacent = vec![Vec::new(); n];
        for &(a, b, effort) in links {
            adjacent[a].push((b as u32, effort));
            adjacent[b].push((a as u32, effort));
        }
        for row in &mut adjacent { row.sort_unstable_by_key(|&(r, _)| r); }
        let neighbours: Vec<Vec<_>> = adjacent.iter().map(|row| {
            row.iter().map(|&(r, _)| r as usize).collect()
        }).collect();
        let masses = landmasses(terrain, &neighbours);
        let regions: Vec<_> = (0..n).map(|r| Region {
            site: [r as f32, 0.0],
            outline: vec![[r as f32, 0.0], [r as f32 + 1.0, 0.0],
                [r as f32 + 1.0, 1.0], [r as f32, 1.0]],
            area_km2: REFERENCE_AREA_KM2,
            terrain: terrain[r],
            neighbours: neighbours[r].clone(),
            landmass: masses[r],
        }).collect();
        let mut edges = RouteRows { offsets: vec![0], entries: Vec::new() };
        for row in adjacent {
            edges.entries.extend(row);
            edges.offsets.push(edges.entries.len());
        }
        let mut map = Map {
            size: MapSize::Small, width: n as f32, height: 1.0,
            feeding: feeding_factors(&regions),
            landmasses: describe_landmasses(&regions, MapSize::Small), regions, edges,
            walking: RouteRows::default(), voyages: RouteRows::default(),
        };
        (map.walking, map.voyages) = map.cache_routes();
        map
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

}

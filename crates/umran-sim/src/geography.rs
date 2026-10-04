//! One closed spherical geography, independent of how a chart presents it.
//!
//! Regions are the dual cells of a subdivided icosahedron. Canonical unit
//! vectors determine area, drainage adjacency, landmass anchors and travel;
//! the equirectangular `site` is drawing compatibility only.

use crate::rng::{key, stream};
use crate::sphere::{Point, add, dot, tangent, unit};
use crate::{continental, math, rivers, sphere};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};
use std::{borrow::Cow, cmp::Ordering, collections::BinaryHeap};

/// Equatorial drawing scale only; never use chart lengths for physics.
pub const KM_PER_UNIT: f32 = 100.0;
/// Plain-kilometre scale used to soften journey preferences.
pub const REFERENCE_TRAVEL_KM: f32 = 100.0;
/// Area of an interior reference hexagon with 100 km centre spacing.
pub const REFERENCE_AREA_KM2: f32 = 8_660.254;
/// A named reach needs rain from roughly three wet reference cells.
pub const RIVER_FORMATION_FLOW: f32 = 3.0 * REFERENCE_AREA_KM2;
/// A fixed river valley is usable while both endpoints carry this much water.
pub const RIVER_TRAVEL_FLOW: f32 = 2.0 * REFERENCE_AREA_KM2;
/// Travel effort along a usable valley relative to its shared-border cost.
const VALLEY_EFFORT: f32 = 0.65;
/// Precomputed neighbourhood radius, not a limit on exact journeys.
pub const CACHE_REACH_KM: f32 = 1_800.0;
/// Fixed effort-km charged on each embarkation and landing.
const EMBARK: f32 = 100.0;
/// Classification by physical land area, independent of mesh resolution.
const CONTINENT_AREA_KM2: f64 = 500_000.0;
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

/// The immutable geography algorithm recorded by a world's recipe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeographyVersion {
    SphericalV1,
    ContinentalV2,
    /// ContinentalV2 with zonal wind taken from geographic east.
    ContinentalV3,
    /// ContinentalV3 with lakes and river channels.
    #[default]
    ContinentalV4,
}

impl GeographyVersion {
    /// The geography new worlds are drawn with.
    pub const CURRENT: Self = Self::ContinentalV4;
}

/// Physical world size and bounded spherical region resolution.
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
    fn subdivisions(self) -> u32 {
        match self {
            Self::Small => 3,
            Self::Medium | Self::Large => 4,
            Self::Vast => 5,
        }
    }

    pub fn radius_km(self) -> f32 {
        match self {
            Self::Small => 800.0,
            Self::Medium => 1_600.0,
            Self::Large => 3_200.0,
            Self::Vast => 6_371.0,
        }
    }

    pub fn region_count(self) -> usize {
        10 * 4_usize.pow(self.subdivisions()) + 2
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
    /// Canonical unit vector: x=cos(lat)cos(lon), y=cos(lat)sin(lon), z=sin(lat).
    pub position: [f64; 3],
    /// Canonical unclosed polygon, clockwise when viewed from outside.
    pub boundary: Vec<[f64; 3]>,
    /// Derived equirectangular drawing point, never a physical coordinate.
    pub site: [f32; 2],
    /// Physical area of the canonical spherical polygon.
    pub area_km2: f32,
    pub terrain: Terrain,
    /// Normalized relief and moisture fields, independent of projection.
    pub elevation: f32,
    pub moisture: f32,
    /// Static normalized thermal baseline, separate from climate history.
    pub warmth: f32,
    /// Connected climatic district; sea has no zone.
    pub climate_zone: Option<usize>,
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

/// One named course, upstream to downstream, with no reach owned twice.
#[derive(Clone, Debug, PartialEq)]
pub struct River {
    pub course: Vec<usize>,
    /// Ultimate sea outlet or terminal lake region, including for tributaries.
    pub mouth: usize,
    /// All upstream land supplying the course's final reach, in region order.
    pub catchment: Vec<usize>,
    /// The river owning the next downstream reach; none at the sea or a closed lake.
    pub joins: Option<usize>,
    /// Canonical unit-vector course through shared borders, upstream first.
    pub channel: Vec<[f64; 3]>,
}

/// A connected district sharing one climate history.
#[derive(Clone, Debug, PartialEq)]
pub struct ClimateZone {
    /// Land regions in increasing order.
    pub regions: Vec<usize>,
}

/// Only drainage-linked river lands can become valley travel edges.
#[derive(Clone, Debug, PartialEq)]
struct ValleyEdge {
    regions: [usize; 2],
    entries: [usize; 2],
    dry_effort: f32,
    usable: bool,
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
            .ok()
            .map(|i| row[i].1)
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
        other
            .effort
            .total_cmp(&self.effort)
            .then(other.region.cmp(&self.region))
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
    predecessor: Option<Vec<u32>>,
}

impl RouteScratch {
    fn new(n: usize) -> Self {
        Self {
            distance: vec![f64::INFINITY; n],
            touched: Vec::new(),
            heap: BinaryHeap::new(),
            predecessor: None,
        }
    }

    fn reset(&mut self) {
        for r in self.touched.drain(..) {
            self.distance[r as usize] = f64::INFINITY;
        }
        self.heap.clear();
    }

    fn relax(&mut self, region: u32, effort: f64, reach: f32, previous: u32) {
        // Radius boundaries use the public f32 effort, including its rounding.
        if effort as f32 > reach || effort >= self.distance[region as usize] {
            return;
        }
        if self.distance[region as usize].is_infinite() {
            self.touched.push(region);
        }
        self.distance[region as usize] = effort;
        if let Some(parents) = &mut self.predecessor {
            parents[region as usize] = previous;
        }
        self.heap.push(RouteVisit { effort, region });
    }

    fn append_row(
        &mut self,
        map: &Map,
        source: usize,
        mode: RouteMode,
        entries: &mut Vec<(u32, f32)>,
    ) {
        self.touched.sort_unstable();
        entries.extend(
            self.touched
                .iter()
                .copied()
                .filter(|&r| {
                    map.regions[r as usize].terrain.is_land()
                        && (mode == RouteMode::Walking || r as usize != source)
                })
                .map(|r| (r, self.distance[r as usize] as f32)),
        );
    }
}

/// The world's land and sea.
#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub size: MapSize,
    pub geography: GeographyVersion,
    pub radius_km: f32,
    pub width: f32,
    pub height: f32,
    pub regions: Vec<Region>,
    /// Its bodies of land, numbered as `Region::landmass` numbers them.
    pub landmasses: Vec<Landmass>,
    /// Each land's next bordering region; sea and endorheic sinks have none.
    pub drainage: Vec<Option<usize>>,
    /// Land-only topological order, upstream before downstream.
    pub drainage_order: Vec<usize>,
    /// Local base rainfall supply in wet km², before downstream accumulation.
    pub runoff: Vec<f32>,
    /// Stable river IDs are indices, and each reach belongs to exactly one.
    pub rivers: Vec<River>,
    pub river_regions: Vec<Option<usize>>,
    pub lakes: Vec<crate::lakes::Lake>,
    pub lake_regions: Vec<Option<usize>>,
    pub climate_zones: Vec<ClimateZone>,
    /// Symmetric centre-to-centre effort through each shared border midpoint.
    edges: RouteRows,
    /// Land-only walking and sea-interior voyage neighbourhoods to 1800 effort-km.
    walking: RouteRows,
    voyages: RouteRows,
    valleys: Vec<ValleyEdge>,
}

impl Map {
    /// The closed world `seed` draws at the physical scale of `size`.
    pub fn generate(seed: u64, size: MapSize) -> Map {
        Self::generate_with_version(seed, size, GeographyVersion::CURRENT)
    }

    /// Replays a recorded geography without changing its geometry or streams.
    pub fn generate_with_version(seed: u64, size: MapSize, geography: GeographyVersion) -> Map {
        let radius_km = size.radius_km();
        let width = TAU * f64::from(radius_km) / f64::from(KM_PER_UNIT);
        let height = width / 2.0;
        let mut mesh = sphere::mesh(size.subdivisions());
        let (terrain, elevation, moisture) = match geography {
            GeographyVersion::SphericalV1 => surface(seed, &mesh.cells, radius_km),
            GeographyVersion::ContinentalV2 => {
                continental::surface(seed, &mut mesh, radius_km, continental::Wind::MeshTangent)
            }
            GeographyVersion::ContinentalV3 | GeographyVersion::ContinentalV4 => {
                continental::surface(
                    seed,
                    &mut mesh,
                    radius_km,
                    continental::Wind::GeographicEast,
                )
            }
        };
        let landmass = landmasses(&terrain, &mesh.cells);
        let edges = travel_edges(&mesh.cells, &terrain, &mesh.borders, radius_km);
        let mut regions: Vec<_> = mesh
            .cells
            .into_iter()
            .enumerate()
            .map(|(i, cell)| {
                let site = chart_point(cell.position, width, height);
                Region {
                    position: cell.position,
                    area_km2: (sphere::area(cell.position, &cell.boundary)
                        * f64::from(radius_km)
                        * f64::from(radius_km)) as f32,
                    boundary: cell.boundary,
                    site,
                    terrain: terrain[i],
                    elevation: elevation[i] as f32,
                    moisture: moisture[i] as f32,
                    warmth: rivers::warmth(seed, i, elevation[i] as f32, cell.position[2] as f32),
                    climate_zone: None,
                    neighbours: cell.neighbours,
                    landmass: landmass[i],
                }
            })
            .collect();
        let landmasses = describe_landmasses(&regions);
        let climate_zones = rivers::climate_zones(seed, &mut regions, &landmasses);
        let rivers::Hydrology {
            drainage,
            drainage_order,
            runoff,
            mut rivers,
            river_regions,
            lakes,
            lake_regions,
            flows,
        } = rivers::generate(seed, &regions, geography);
        if geography == GeographyVersion::ContinentalV4 {
            crate::lakes::channels(&regions, &drainage, &mut rivers);
        }
        let mut map = Map {
            size,
            geography,
            radius_km,
            width: width as f32,
            height: height as f32,
            regions,
            landmasses,
            drainage,
            drainage_order,
            runoff,
            rivers,
            river_regions,
            lakes,
            lake_regions,
            climate_zones,
            edges,
            walking: RouteRows::default(),
            voyages: RouteRows::default(),
            valleys: Vec::new(),
        };
        map.initialize_valleys(&flows);
        map.walking = map.cache_routes(RouteMode::Walking);
        map.voyages = map.cache_routes(RouteMode::Voyage);
        map
    }

    /// Half-arclength point of the shared canonical border (possibly curved).
    pub fn shared_midpoint(&self, a: usize, b: usize) -> Option<[f64; 3]> {
        let ra = self.regions.get(a)?;
        let rb = self.regions.get(b)?;
        if ra.neighbours.binary_search(&b).is_err() {
            return None;
        }
        if self.geography != GeographyVersion::SphericalV1 {
            // Canonicalize traversal so asking b→a returns exactly the same point.
            let (first, second) = if a < b { (ra, rb) } else { (rb, ra) };
            return sphere::shared_midpoint(&first.boundary, &second.boundary);
        }
        let mut shared = ra
            .boundary
            .iter()
            .copied()
            .filter(|p| rb.boundary.contains(p));
        Some(unit(add(shared.next()?, shared.next()?)))
    }

    fn initialize_valleys(&mut self, flows: &[f32]) {
        for (r, downstream) in self.drainage.iter().enumerate() {
            let Some(n) = *downstream else { continue };
            if self.river_regions[r].is_none() || self.river_regions[n].is_none() {
                continue;
            }
            let entry = |a, b| {
                self.edges.offsets[a]
                    + self
                        .edges
                        .row(a)
                        .binary_search_by_key(&(b as u32), |&(id, _)| id)
                        .expect("drainage follows a shared border")
            };
            let entries = [entry(r, n), entry(n, r)];
            let dry_effort = self.edges.entries[entries[0]].1;
            let usable = flows[r] >= RIVER_TRAVEL_FLOW && flows[n] >= RIVER_TRAVEL_FLOW;
            if usable {
                for &index in &entries {
                    self.edges.entries[index].1 = dry_effort * VALLEY_EFFORT;
                }
            }
            self.valleys.push(ValleyEdge {
                regions: [r, n],
                entries,
                dry_effort,
                usable,
            });
        }
    }

    /// Check before making a shared map mutable: unchanged usability needs
    /// neither an `Arc` clone nor a walking-cache refresh.
    pub fn valley_flows_changed(&self, flows: &[f32]) -> bool {
        assert_eq!(flows.len(), self.regions.len());
        self.valleys.iter().any(|valley| {
            let [a, b] = valley.regions;
            (flows[a] >= RIVER_TRAVEL_FLOW && flows[b] >= RIVER_TRAVEL_FLOW) != valley.usable
        })
    }

    /// Refresh walking only when a drainage-linked river edge becomes usable
    /// or dries. Flow changes within the same band do not rebuild any routes.
    /// Sea edges, voyage permissions, and voyage caches are never changed.
    pub fn set_valley_flows(&mut self, flows: &[f32]) -> bool {
        assert_eq!(flows.len(), self.regions.len());
        let mut changed = false;
        for valley in &mut self.valleys {
            let [a, b] = valley.regions;
            let usable = flows[a] >= RIVER_TRAVEL_FLOW && flows[b] >= RIVER_TRAVEL_FLOW;
            if usable == valley.usable {
                continue;
            }
            valley.usable = usable;
            changed = true;
            let effort = valley.dry_effort * if usable { VALLEY_EFFORT } else { 1.0 };
            for &index in &valley.entries {
                self.edges.entries[index].1 = effort;
            }
        }
        if changed {
            self.walking = self.cache_routes(RouteMode::Walking);
        }
        changed
    }

    /// Effort across one shared border, for a traversal restricted to held land.
    pub(crate) fn border_effort(&self, a: usize, b: usize) -> Option<f32> {
        self.edges
            .row(a)
            .iter()
            .find(|(r, _)| *r as usize == b)
            .map(|(_, effort)| *effort)
    }

    /// Exact least land-only walking effort-km; sea endpoints are unreachable.
    pub fn distance(&self, a: usize, b: usize) -> f32 {
        if !self.regions[a].terrain.is_land()
            || !self.regions[b].terrain.is_land()
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

    /// Builds a path only for consumers that display an actual journey.
    pub(crate) fn route_path(
        &self,
        source: usize,
        target: usize,
        by_sea: bool,
        reach: f32,
    ) -> Option<Vec<usize>> {
        let mode = if by_sea {
            RouteMode::Voyage
        } else {
            RouteMode::Walking
        };
        let eligible = if by_sea {
            source != target && self.coastal(source) && self.coastal(target)
        } else {
            self.regions[source].terrain.is_land()
                && self.regions[target].terrain.is_land()
                && self.regions[source].landmass == self.regions[target].landmass
        };
        if !eligible || reach.is_nan() || reach < 0.0 {
            return None;
        }
        let mut scratch = RouteScratch::new(self.regions.len());
        scratch.predecessor = Some(vec![u32::MAX; self.regions.len()]);
        self.search_routes(source, reach, mode, Some(target), &mut scratch);
        if !scratch.distance[target].is_finite() {
            return None;
        }
        let parents = scratch.predecessor.as_ref().unwrap();
        let mut path = vec![target];
        let mut at = target;
        while at != source {
            at = parents[at] as usize;
            path.push(at);
        }
        path.reverse();
        Some(path)
    }

    pub fn journey_path(&self, source: usize, target: usize, by_sea: bool) -> Vec<usize> {
        self.route_path(source, target, by_sea, f32::INFINITY)
            .unwrap_or_default()
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
        let rows = match mode {
            RouteMode::Walking => &self.walking,
            RouteMode::Voyage => &self.voyages,
        };
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
        if !eligible || reach.is_nan() || reach < 0.0 {
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
            return Cow::Owned(
                row.iter()
                    .copied()
                    .filter(|&(_, effort)| effort <= reach)
                    .collect(),
            );
        }
        let mut scratch = RouteScratch::new(self.regions.len());
        self.search_routes(source, reach, mode, None, &mut scratch);
        let mut row = Vec::new();
        scratch.append_row(self, source, mode, &mut row);
        Cow::Owned(row)
    }

    fn cache_routes(&self, mode: RouteMode) -> RouteRows {
        let n = self.regions.len();
        let mut rows = RouteRows {
            offsets: Vec::with_capacity(n + 1),
            entries: Vec::new(),
        };
        let mut scratch = RouteScratch::new(n);
        rows.offsets.push(0);
        for source in 0..n {
            let eligible = match mode {
                RouteMode::Walking => self.regions[source].terrain.is_land(),
                RouteMode::Voyage => self.coastal(source),
            };
            if eligible {
                self.search_routes(source, CACHE_REACH_KM, mode, None, &mut scratch);
                scratch.append_row(self, source, mode, &mut rows.entries);
            }
            rows.offsets.push(rows.entries.len());
        }
        rows
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
        scratch.relax(source as u32, 0.0, reach, source as u32);
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
                scratch.relax(
                    next,
                    effort + f64::from(edge_effort) + surcharge,
                    reach,
                    region,
                );
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
        let effort = self
            .edges
            .get(a, b)
            .expect("a neighbour has a retained shared border");
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

/// Longitude east and latitude north, in degrees, from a canonical unit vector.
pub fn geographic(position: [f64; 3]) -> [f64; 2] {
    let [x, y, z] = position;
    [
        math::atan2(y, x) * 180.0 / PI,
        math::atan2(z, (x * x + y * y).sqrt()) * 180.0 / PI,
    ]
}

/// Shortest great-circle angle in radians, including coincident/antipodal points.
pub fn angular_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    sphere::angle(a, b)
}

fn chart_point(position: Point, width: f64, height: f64) -> [f32; 2] {
    let [lon, lat] = geographic(position);
    [
        ((lon + 180.0) / 360.0 * width) as f32,
        ((90.0 - lat) / 180.0 * height) as f32,
    ]
}

pub(crate) fn random_direction(rng: &mut ChaCha8Rng) -> Point {
    let z: f64 = rng.gen_range(-1.0..1.0);
    let longitude = rng.gen_range(-PI..PI);
    let radial = (1.0 - z * z).sqrt();
    [
        radial * math::cos(longitude),
        radial * math::sin(longitude),
        z,
    ]
}

/// Smooth, seam-free fields sampled in 3D rather than in chart coordinates.
struct Wave {
    direction: Point,
    frequency: f64,
    phase: f64,
    amplitude: f64,
}

impl Wave {
    fn value(&self, p: Point) -> f64 {
        self.amplitude * math::sin(dot(p, self.direction) * self.frequency + self.phase)
    }
}

fn waves(rng: &mut ChaCha8Rng) -> [Wave; 6] {
    std::array::from_fn(|i| Wave {
        direction: random_direction(rng),
        frequency: [3.0, 5.0, 9.0, 17.0, 31.0, 47.0][i],
        phase: rng.gen_range(-PI..PI),
        amplitude: [0.38, 0.26, 0.17, 0.10, 0.06, 0.03][i],
    })
}

struct Continent {
    centre: Point,
    along: Point,
    across: Point,
    length: f64,
    width: f64,
    ridge_offset: f64,
    ridge_phase: f64,
}

impl Continent {
    /// Spherical logarithmic coordinates in the body's local tangent basis.
    fn coordinates(&self, p: Point) -> [f64; 2] {
        let x = dot(p, self.along);
        let y = dot(p, self.across);
        let radial = (x * x + y * y).sqrt();
        let scale = if radial > 1e-12 {
            math::atan2(radial, dot(p, self.centre)) / radial
        } else {
            0.0
        };
        [x * scale, y * scale]
    }
}

/// A bounded set of separated broad bodies, perturbed coasts, small volcanic
/// highs, and curved elongated ridges. This is not a plate or weather model.
fn surface(
    seed: u64,
    cells: &[sphere::Cell],
    radius_km: f32,
) -> (Vec<Terrain>, Vec<f64>, Vec<f64>) {
    let mut body_rng = stream(seed, &[key("spherical continental bodies")]);
    let mut continents: Vec<Continent> = Vec::with_capacity(5);
    for _ in 0..5 {
        // Farthest of a fixed candidate set; never retry a generated world.
        let centre = (0..48)
            .map(|_| random_direction(&mut body_rng))
            .min_by(|&a, &b| {
                let nearest = |p| {
                    continents
                        .iter()
                        .map(|c| dot(p, c.centre))
                        .fold(-1.0, f64::max)
                };
                nearest(a).total_cmp(&nearest(b))
            })
            .unwrap();
        let (east, north) = tangent(centre);
        let turn = body_rng.gen_range(-PI..PI);
        let (s, c) = (math::sin(turn), math::cos(turn));
        continents.push(Continent {
            centre,
            along: std::array::from_fn(|i| east[i] * c + north[i] * s),
            across: std::array::from_fn(|i| north[i] * c - east[i] * s),
            length: body_rng.gen_range(0.58..0.88),
            width: body_rng.gen_range(0.34..0.55),
            ridge_offset: body_rng.gen_range(-0.15..0.15),
            ridge_phase: body_rng.gen_range(-PI..PI),
        });
    }
    let mut island_rng = stream(seed, &[key("spherical island highs")]);
    let islands: Vec<_> = (0..32)
        .map(|_| {
            let centre = random_direction(&mut island_rng);
            let radius = island_rng.gen_range(0.06..0.14);
            (centre, radius, math::cos(radius * 1.25))
        })
        .collect();
    let coast_waves = waves(&mut stream(seed, &[key("spherical coastline field")]));
    let relief_waves = waves(&mut stream(seed, &[key("spherical relief field")]));
    let wet_waves = waves(&mut stream(seed, &[key("spherical moisture field")]));
    let mut terrain = vec![Terrain::Sea; cells.len()];
    let mut elevation = Vec::with_capacity(cells.len());
    for (r, cell) in cells.iter().enumerate() {
        let p = cell.position;
        let coast: f64 = coast_waves.iter().map(|wave| wave.value(p)).sum();
        let relief: f64 = relief_waves.iter().map(|wave| wave.value(p)).sum();
        let mut height: f64 = -1.0;
        let mut ridge: f64 = 0.0;
        for body in &continents {
            if dot(p, body.centre) <= 0.0 {
                continue;
            }
            let [along, across] = body.coordinates(p);
            let bend = math::sin(5.0 * along + body.ridge_phase);
            // A broad bend makes non-elliptical peninsulas and bays.
            let bent = across + 0.10 * bend;
            let x = along / body.length;
            let y = bent / body.width;
            height = height.max(1.0 - (x * x + y * y).sqrt() + 0.22 * coast);
            let transverse = (across - body.ridge_offset - 0.07 * bend) / 0.065;
            let longitudinal = along / (0.8 * body.length);
            ridge = ridge.max(math::exp64(
                -transverse * transverse
                    - longitudinal * longitudinal * longitudinal * longitudinal,
            ));
        }
        for &(centre, radius, cutoff) in &islands {
            // Dot-product rejection avoids trig for distant small highs.
            if dot(p, centre) > cutoff {
                height = height.max(1.0 - angular_distance(p, centre) / radius + 0.10 * coast);
            }
        }
        if height > 0.0 {
            terrain[r] = Terrain::Plains;
        }
        elevation.push((0.12 + 0.32 * height + 0.64 * ridge + 0.10 * relief).clamp(0.0, 1.0));
    }

    // Distance to salt water follows the closed graph in physical kilometres.
    // A multi-source pass avoids scanning every coast from every inland cell.
    let coast_distance = coast_distances(cells, &terrain, radius_km);
    let moisture: Vec<_> = cells
        .iter()
        .enumerate()
        .map(|(r, cell)| {
            let latitude = cell.position[2].abs();
            let equatorial = math::exp64(-latitude * latitude / 0.045);
            let subtropical = math::exp64(-(latitude - 0.5) * (latitude - 0.5) / 0.025);
            let field: f64 = wet_waves.iter().map(|wave| wave.value(cell.position)).sum();
            (0.46 + 0.28 * math::exp64(-coast_distance[r] / 650.0) + 0.22 * equatorial
                - 0.25 * subtropical
                - 0.22 * elevation[r]
                + 0.24 * field)
                .clamp(0.0, 1.0)
        })
        .collect();

    let land: Vec<_> = (0..cells.len()).filter(|&r| terrain[r].is_land()).collect();
    let by_height = ranked(&land, &elevation);
    let mountains = share(land.len(), MOUNTAINS);
    let hills = share(land.len(), HILLS);
    let lowland_end = land.len() - mountains - hills;
    for &r in &by_height[lowland_end..land.len() - mountains] {
        terrain[r] = Terrain::Hills;
    }
    for &r in &by_height[land.len() - mountains..] {
        terrain[r] = Terrain::Mountains;
    }
    let by_moisture = ranked(&by_height[..lowland_end], &moisture);
    let deserts = share(lowland_end, DESERT);
    let steppes = share(lowland_end, STEPPE);
    let forests = share(lowland_end, FOREST);
    for &r in &by_moisture[..deserts] {
        terrain[r] = Terrain::Desert;
    }
    for &r in &by_moisture[deserts..deserts + steppes] {
        terrain[r] = Terrain::Steppe;
    }
    for &r in &by_moisture[lowland_end - forests..] {
        terrain[r] = Terrain::Forest;
    }
    (terrain, elevation, moisture)
}

pub(crate) fn coast_distances(
    cells: &[sphere::Cell],
    terrain: &[Terrain],
    radius_km: f32,
) -> Vec<f64> {
    let mut distance = vec![f64::INFINITY; cells.len()];
    let mut heap = BinaryHeap::new();
    for (r, cell) in cells.iter().enumerate() {
        if !terrain[r].is_land() {
            distance[r] = 0.0;
        } else {
            let coast = cell
                .neighbours
                .iter()
                .copied()
                .filter(|&n| !terrain[n].is_land())
                .map(|n| {
                    0.5 * f64::from(radius_km) * angular_distance(cell.position, cells[n].position)
                })
                .fold(f64::INFINITY, f64::min);
            if coast.is_finite() {
                distance[r] = coast;
                heap.push(RouteVisit {
                    effort: coast,
                    region: r as u32,
                });
            }
        }
    }
    while let Some(RouteVisit { effort, region }) = heap.pop() {
        let r = region as usize;
        if effort != distance[r] {
            continue;
        }
        for &n in &cells[r].neighbours {
            if !terrain[n].is_land() {
                continue;
            }
            let candidate = effort
                + f64::from(radius_km) * angular_distance(cells[r].position, cells[n].position);
            if candidate < distance[n] {
                distance[n] = candidate;
                heap.push(RouteVisit {
                    effort: candidate,
                    region: n as u32,
                });
            }
        }
    }
    distance
}

/// Each land region's body of land, numbered in order of its lowest
/// region; `None` for sea.
fn landmasses(terrain: &[Terrain], cells: &[sphere::Cell]) -> Vec<Option<usize>> {
    let mut out: Vec<Option<usize>> = vec![None; terrain.len()];
    let mut next = 0;
    for start in 0..terrain.len() {
        if !terrain[start].is_land() || out[start].is_some() {
            continue;
        }
        out[start] = Some(next);
        let mut stack = vec![start];
        while let Some(r) = stack.pop() {
            for &n in &cells[r].neighbours {
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
fn describe_landmasses(regions: &[Region]) -> Vec<Landmass> {
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
            let centre = regions_of.iter().fold([0.0; 3], |sum, &r| {
                add(sum, sphere::moment(&regions[r].boundary))
            });
            // Maximizing dot product minimizes the angular distance to the
            // spherical surface centroid. A zero moment ties by stable ID.
            let anchor = regions_of
                .iter()
                .copied()
                .max_by(|&a, &b| {
                    dot(regions[a].position, centre)
                        .total_cmp(&dot(regions[b].position, centre))
                        .then(b.cmp(&a))
                })
                .expect("a landmass has a region");
            let area: f64 = regions_of
                .iter()
                .map(|&r| f64::from(regions[r].area_km2))
                .sum();
            let kind = if area >= CONTINENT_AREA_KM2 {
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

fn travel_edges(
    cells: &[sphere::Cell],
    terrain: &[Terrain],
    borders: &[sphere::Border],
    radius_km: f32,
) -> RouteRows {
    let mut offsets = vec![0; cells.len() + 1];
    for border in borders {
        offsets[border.a + 1] += 1;
        offsets[border.b + 1] += 1;
    }
    for r in 0..cells.len() {
        offsets[r + 1] += offsets[r];
    }
    let mut entries = vec![(0, 0.0); 2 * borders.len()];
    let mut cursor = offsets[..cells.len()].to_vec();
    for border in borders {
        let leg = |r: usize| angular_distance(cells[r].position, border.midpoint);
        let effort = (f64::from(radius_km)
            * (leg(border.a) * f64::from(terrain[border.a].travel())
                + leg(border.b) * f64::from(terrain[border.b].travel())))
            as f32;
        for (from, to) in [(border.a, border.b), (border.b, border.a)] {
            entries[cursor[from]] = (to as u32, effort);
            cursor[from] += 1;
        }
    }
    for r in 0..cells.len() {
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
    fn drainage_spills_hollows_through_the_lowest_saddle() {
        let mut map = route_fixture(
            &[
                Terrain::Sea,
                Terrain::Hills,
                Terrain::Mountains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
            ],
            &[
                (0, 1, 100.0),
                (0, 2, 100.0),
                (1, 3, 100.0),
                (2, 4, 100.0),
                (3, 4, 100.0),
                (3, 5, 100.0),
                (4, 5, 100.0),
            ],
        );
        for (r, elevation) in map.regions.iter_mut().zip([0.0, 4.0, 8.0, 1.0, 0.0, 1.0]) {
            r.elevation = elevation;
        }
        let (drainage, order) = rivers::drainage(&map.regions);
        assert_eq!(
            drainage,
            [None, Some(0), Some(0), Some(1), Some(3), Some(3)]
        );
        for &r in &order {
            let n = drainage[r].unwrap();
            if map.regions[n].terrain.is_land() {
                assert!(order.iter().position(|&x| x == r) < order.iter().position(|&x| x == n));
            }
        }
        assert_eq!(map.regions[4].elevation, 0.0);
    }

    #[test]
    fn rivers_use_physical_runoff_not_a_count_of_polygons() {
        let mut terrain = vec![Terrain::Plains; 13];
        terrain[12] = Terrain::Sea;
        let mut map = linear_map(&terrain);
        for r in &mut map.regions {
            r.area_km2 = 0.1 * REFERENCE_AREA_KM2;
        }
        let small = rivers::generate(17, &map.regions, map.geography);
        assert!(small.rivers.is_empty());
        // One broad wet headwater can supply a long course through small
        // polygons. Changing subdivision count is not the formation rule.
        map.regions[0].area_km2 = 4.0 * REFERENCE_AREA_KM2;
        let broad = rivers::generate(17, &map.regions, map.geography);
        assert_eq!(broad.rivers.len(), 1);
        assert_eq!(broad.rivers[0].course, (0..12).collect::<Vec<_>>());
        assert_eq!(broad.rivers[0].catchment, (0..12).collect::<Vec<_>>());
        assert_eq!(broad.rivers[0].mouth, 12);
        assert!(broad.flows[0] >= RIVER_FORMATION_FLOW);
        assert!(broad.runoff[0] > 30.0 * small.runoff[0]);
    }

    #[test]
    fn tributaries_join_without_sharing_courses_and_keep_full_catchments() {
        let map = route_fixture(
            &[
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Sea,
            ],
            &[
                (0, 2, 100.0),
                (1, 2, 100.0),
                (2, 5, 100.0),
                (3, 4, 100.0),
                (4, 5, 100.0),
                (5, 6, 100.0),
                (6, 7, 100.0),
            ],
        );
        let drainage = [
            Some(2),
            Some(2),
            Some(5),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            None,
        ];
        let order = [1, 0, 3, 2, 4, 5, 6];
        let flows = [4.0, 4.0, 9.0, 3.0, 4.0, 14.0, 15.0, 15.0].map(|f| f * REFERENCE_AREA_KM2);
        let (rivers, owner) = rivers::courses(&map.regions, &drainage, &order, &flows);
        assert_eq!(
            rivers,
            [
                River {
                    course: vec![1],
                    mouth: 7,
                    catchment: vec![1],
                    joins: Some(2),
                    channel: Vec::new(),
                },
                River {
                    course: vec![3, 4],
                    mouth: 7,
                    catchment: vec![3, 4],
                    joins: Some(2),
                    channel: Vec::new(),
                },
                River {
                    course: vec![0, 2, 5, 6],
                    mouth: 7,
                    catchment: vec![0, 1, 2, 3, 4, 5, 6],
                    joins: None,
                    channel: Vec::new(),
                },
            ]
        );
        assert_eq!(
            owner,
            [
                Some(2),
                Some(0),
                Some(2),
                Some(1),
                Some(1),
                Some(2),
                Some(2),
                None
            ]
        );
    }

    #[test]
    fn generated_drainage_and_climate_zones_respect_land_boundaries() {
        for size in [
            MapSize::Small,
            MapSize::Medium,
            MapSize::Large,
            MapSize::Vast,
        ] {
            for seed in [3, 11] {
                let mut map = Map::generate(seed, size);
                let n = map.regions.len();
                let mut rank = vec![usize::MAX; n];
                let mut flows = map.runoff.clone();
                for (i, &r) in map.drainage_order.iter().enumerate() {
                    assert_eq!(rank[r], usize::MAX);
                    rank[r] = i;
                    if let Some(next) = map.drainage[r] {
                        flows[next] += flows[r];
                    }
                }
                for &r in &map.drainage_order {
                    if let Some(next) = map.drainage[r] {
                        assert!(rank[r] < rank[next]);
                    }
                }
                let mut course_owner = vec![None; n];
                for (id, river) in map.rivers.iter().enumerate() {
                    let end = *river.course.last().unwrap();
                    assert!(
                        !map.regions[river.mouth].terrain.is_land()
                            || map.lake_regions[river.mouth].is_some()
                    );
                    assert_eq!(
                        river.joins,
                        map.drainage[end].and_then(|n| map.river_regions[n])
                    );
                    for &r in &river.course {
                        assert!(course_owner[r].replace(id).is_none());
                    }
                    for pair in river.course.windows(2) {
                        assert_eq!(map.drainage[pair[0]], Some(pair[1]));
                    }
                    let mut feeds = vec![false; n];
                    feeds[end] = true;
                    // Propagate membership upstream through the independent
                    // topological order instead of walking every source path.
                    for &r in map.drainage_order.iter().rev() {
                        feeds[r] = feeds[r] || map.drainage[r].is_some_and(|n| feeds[n]);
                    }
                    let catchment: Vec<_> = (0..n).filter(|&r| feeds[r]).collect();
                    assert_eq!(river.catchment, catchment);
                    let mut outlet = end;
                    while let Some(next) = map.drainage[outlet] {
                        outlet = next;
                    }
                    assert_eq!(river.mouth, outlet);
                }
                assert_eq!(course_owner, map.river_regions);
                for (r, region) in map.regions.iter().enumerate() {
                    assert!((0.0..=1.0).contains(&region.warmth));
                    assert_eq!(region.climate_zone.is_some(), region.terrain.is_land());
                    assert_eq!(rank[r] != usize::MAX, region.terrain.is_land());
                    assert_eq!(
                        course_owner[r].is_some(),
                        region.terrain.is_land() && flows[r] >= RIVER_FORMATION_FLOW
                    );
                    if let Some(next) = map.drainage[r] {
                        assert!(region.neighbours.contains(&next));
                        if map.regions[next].terrain.is_land() {
                            assert_eq!(region.landmass, map.regions[next].landmass);
                        }
                    } else {
                        assert!(!region.terrain.is_land() || map.lake_regions[r].is_some());
                    }
                }
                for (id, zone) in map.climate_zones.iter().enumerate() {
                    assert!((1..=20).contains(&zone.regions.len()));
                    assert!(zone.regions.windows(2).all(|p| p[0] < p[1]));
                    let start = zone.regions[0];
                    let mut visited = vec![false; n];
                    let mut stack = vec![start];
                    visited[start] = true;
                    while let Some(r) = stack.pop() {
                        for &next in &map.regions[r].neighbours {
                            if !visited[next] && map.regions[next].climate_zone == Some(id) {
                                visited[next] = true;
                                stack.push(next);
                            }
                        }
                    }
                    assert!(zone.regions.iter().all(|&r| visited[r]
                        && map.regions[r].climate_zone == Some(id)
                        && map.regions[r].landmass == map.regions[start].landmass));
                }
                assert!(!map.valley_flows_changed(&flows));
                assert!(!map.set_valley_flows(&flows));
            }
        }
    }

    #[test]
    fn valleys_refresh_only_connected_reaches_when_flow_crosses_the_threshold() {
        let mut map = route_fixture(
            &[
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Sea,
                Terrain::Plains,
            ],
            &[
                (0, 1, 200.0),
                (1, 2, 200.0),
                (0, 3, 500.0),
                (3, 2, 200.0),
                (0, 2, 1_000.0),
                (2, 4, 200.0),
                (4, 5, 200.0),
            ],
        );
        map.drainage = vec![Some(1), Some(2), Some(4), Some(2), None, Some(4)];
        map.river_regions = vec![Some(0), Some(0), Some(0), Some(1), None, None];
        let mut flows = vec![RIVER_TRAVEL_FLOW; 6];
        map.initialize_valleys(&flows);
        map.walking = map.cache_routes(RouteMode::Walking);
        let wet = map.clone();
        assert_eq!(map.distance(0, 2), 260.0);
        assert_eq!(map.closeness(0, 1), 80.0 / 130.0);
        assert_eq!(map.edges.get(0, 3), Some(500.0));
        assert_eq!(map.edges.get(0, 2), Some(1_000.0));
        assert_eq!(map.voyage(2, 5), 600.0);
        assert!(map.distance(2, 5).is_infinite());
        // A dry non-river neighbour and fluctuations above the threshold
        // cannot invalidate walking routes.
        flows[5] = 0.0;
        flows[0] *= 1.1;
        assert!(!map.valley_flows_changed(&flows));
        assert!(!map.set_valley_flows(&flows));
        assert_eq!(map, wet);
        flows[1] = RIVER_TRAVEL_FLOW - 1.0;
        assert!(map.valley_flows_changed(&flows));
        assert!(map.set_valley_flows(&flows));
        assert_eq!(map.distance(0, 2), 400.0);
        assert_eq!(map.closeness(0, 1), 0.4);
        assert_eq!(map.voyages, wet.voyages);
        assert_eq!(map.voyage(2, 5), 600.0);
        assert!(!map.set_valley_flows(&flows));
        flows[1] = RIVER_TRAVEL_FLOW;
        assert!(map.set_valley_flows(&flows));
        assert_eq!(map, wet);
    }

    #[test]
    fn geographic_coordinates_use_east_longitude_and_north_latitude_degrees() {
        for (position, expected) in [
            ([1.0, 0.0, 0.0], [0.0, 0.0]),
            ([0.0, 1.0, 0.0], [90.0, 0.0]),
            ([0.0, -1.0, 0.0], [-90.0, 0.0]),
            ([-1.0, 0.0, 0.0], [180.0, 0.0]),
            ([0.0, 0.0, 1.0], [0.0, 90.0]),
            ([0.0, 0.0, -1.0], [0.0, -90.0]),
        ] {
            let actual = geographic(position);
            assert!((actual[0] - expected[0]).abs() < 1e-12);
            assert!((actual[1] - expected[1]).abs() < 1e-12);
        }
    }

    #[test]
    fn spherical_border_routes_cross_the_seam_and_scale_with_radius() {
        let mesh = sphere::mesh(3);
        let terrain: Vec<_> = (0..mesh.cells.len())
            .map(|r| {
                if r % 2 == 0 {
                    Terrain::Plains
                } else {
                    Terrain::Hills
                }
            })
            .collect();
        let edges = travel_edges(&mesh.cells, &terrain, &mesh.borders, 800.0);
        let larger = travel_edges(&mesh.cells, &terrain, &mesh.borders, 1_600.0);
        let mut seam_edges = 0;
        for border in &mesh.borders {
            let a = mesh.cells[border.a].position;
            let b = mesh.cells[border.b].position;
            let expected = 800.0
                * (angular_distance(a, border.midpoint) * f64::from(terrain[border.a].travel())
                    + angular_distance(b, border.midpoint) * f64::from(terrain[border.b].travel()));
            let effort = edges.get(border.a, border.b).unwrap();
            assert!((f64::from(effort) - expected).abs() < 0.0001);
            assert_eq!(edges.get(border.b, border.a), Some(effort));
            assert_eq!(larger.get(border.a, border.b), Some(2.0 * effort));
            assert!(effort.is_finite() && effort > 0.0);
            let ga = geographic(a);
            let gb = geographic(b);
            if (ga[0] - gb[0]).abs() > 180.0 && ga[1].abs() < 70.0 {
                seam_edges += 1;
                // Neighbours separated by nearly a chart circumference are
                // still ordinary ~100 km cells on the closed globe.
                assert!(angular_distance(a, b) * 800.0 < 200.0);
                assert!(effort < 400.0);
            }
        }
        assert!(seam_edges > 0);
    }

    #[test]
    fn physical_sphere_and_connected_landmasses_ignore_chart_coordinates() {
        for size in [
            MapSize::Small,
            MapSize::Medium,
            MapSize::Large,
            MapSize::Vast,
        ] {
            let mut map = Map::generate(7, size);
            assert_eq!(map.regions.len(), size.region_count());
            let area: f64 = map.regions.iter().map(|r| f64::from(r.area_km2)).sum();
            let expected = 4.0 * PI * f64::from(map.radius_km) * f64::from(map.radius_km);
            assert!((area - expected).abs() / expected < 1e-6);
            assert!(
                (f64::from(map.width) * f64::from(KM_PER_UNIT) - TAU * f64::from(map.radius_km))
                    .abs()
                    < 0.01
            );
            assert_eq!(map.width, 2.0 * map.height);
            let mut seen = vec![false; map.regions.len()];
            for (id, mass) in map.landmasses.iter().enumerate() {
                let mut stack = vec![mass.anchor];
                let mut members = Vec::new();
                seen[mass.anchor] = true;
                while let Some(r) = stack.pop() {
                    members.push(r);
                    for &n in &map.regions[r].neighbours {
                        if map.regions[n].terrain.is_land() && !seen[n] {
                            seen[n] = true;
                            stack.push(n);
                        }
                    }
                }
                members.sort_unstable();
                assert_eq!(members, mass.regions);
                assert!(members.iter().all(|&r| map.regions[r].landmass == Some(id)));
                assert!(members.iter().any(|&r| map.coastal(r)));
                let area: f64 = members
                    .iter()
                    .map(|&r| f64::from(map.regions[r].area_km2))
                    .sum();
                assert_eq!(
                    mass.kind == LandmassKind::Continent,
                    area >= CONTINENT_AREA_KM2
                );
            }
            for (id, region) in map.regions.iter().enumerate() {
                assert_eq!(seen[id], region.terrain.is_land());
                assert!(region.area_km2 > 0.0);
                assert!(region.neighbours.windows(2).all(|p| p[0] < p[1]));
                let expected =
                    chart_point(region.position, f64::from(map.width), f64::from(map.height));
                assert!((expected[0] - region.site[0]).abs() < 0.0001);
                assert!((expected[1] - region.site[1]).abs() < 0.0001);
                for &other in &region.neighbours {
                    assert!(map.shared_midpoint(id, other).is_some());
                    let effort = map.edges.get(id, other).unwrap();
                    assert!(effort.is_finite() && effort > 0.0);
                }
            }
            let source = map
                .landmasses
                .iter()
                .max_by_key(|m| m.regions.len())
                .unwrap()
                .anchor;
            let routes = map.walking_row(source, f32::INFINITY).into_owned();
            assert_eq!(
                routes.len(),
                map.landmasses[map.regions[source].landmass.unwrap()]
                    .regions
                    .len()
            );
            assert!(
                routes
                    .iter()
                    .all(|&(_, distance)| distance.is_finite() && distance >= 0.0)
            );
            let bounded: Vec<_> = routes
                .iter()
                .copied()
                .filter(|&(_, d)| d <= CACHE_REACH_KM)
                .collect();
            assert_eq!(map.walking_cached(source), bounded);
            let coast = map
                .regions
                .iter()
                .enumerate()
                .find(|&(r, _)| map.coastal(r))
                .unwrap()
                .0;
            let voyages = map.voyage_row(coast, f32::INFINITY);
            let bounded: Vec<_> = voyages
                .iter()
                .copied()
                .filter(|&(_, d)| d <= CACHE_REACH_KM)
                .collect();
            assert_eq!(map.voyage_cached(coast), bounded);
            for region in &mut map.regions {
                region.site = [f32::NAN; 2];
            }
            assert_eq!(map.walking_row(source, f32::INFINITY).as_ref(), routes);
            assert_eq!(describe_landmasses(&map.regions), map.landmasses);
            let hydrology = rivers::generate(7, &map.regions, map.geography);
            assert_eq!(hydrology.drainage, map.drainage);
            assert_eq!(hydrology.drainage_order, map.drainage_order);
            assert_eq!(hydrology.lakes, map.lakes);
        }
    }

    #[test]
    fn representative_surfaces_have_coherent_land_relief_and_islands() {
        let mesh = sphere::mesh(4);
        for seed in 0..12 {
            let (terrain, elevation, moisture) = surface(seed, &mesh.cells, 1_600.0);
            let members = landmasses(&terrain, &mesh.cells);
            let mut body_areas = vec![0.0; members.iter().flatten().max().unwrap() + 1];
            let mut total_land = 0.0;
            for (r, cell) in mesh.cells.iter().enumerate() {
                assert!((0.0..=1.0).contains(&elevation[r]));
                assert!((0.0..=1.0).contains(&moisture[r]));
                if let Some(body) = members[r] {
                    let area = sphere::area(cell.position, &cell.boundary) * 1_600.0 * 1_600.0;
                    body_areas[body] += area;
                    total_land += area;
                }
            }
            let share = total_land / (4.0 * PI * 1_600.0 * 1_600.0);
            assert!((0.20..0.65).contains(&share), "seed {seed}: {share}");
            assert!(
                body_areas
                    .iter()
                    .filter(|&&a| a >= CONTINENT_AREA_KM2)
                    .count()
                    >= 2
            );
            assert!(body_areas.iter().any(|&a| a < CONTINENT_AREA_KM2));
            let land_edges = mesh
                .borders
                .iter()
                .filter(|edge| terrain[edge.a].is_land() && terrain[edge.b].is_land())
                .count();
            let mixed_edges = mesh
                .borders
                .iter()
                .filter(|edge| terrain[edge.a].is_land() != terrain[edge.b].is_land())
                .count();
            assert!(
                land_edges > mixed_edges,
                "coherent bodies, not scattered land noise"
            );
            let peaks: Vec<_> = (0..terrain.len())
                .filter(|&r| terrain[r] == Terrain::Mountains)
                .collect();
            let connected_peaks = peaks
                .iter()
                .filter(|&&r| {
                    mesh.cells[r]
                        .neighbours
                        .iter()
                        .any(|&n| matches!(terrain[n], Terrain::Mountains | Terrain::Hills))
                })
                .count();
            assert!(
                connected_peaks * 4 >= peaks.len() * 3,
                "mountains form connected relief"
            );
            assert!(terrain.contains(&Terrain::Plains) && terrain.contains(&Terrain::Forest));
            let (drainage, order) = {
                let regions: Vec<_> = mesh
                    .cells
                    .iter()
                    .enumerate()
                    .map(|(r, cell)| Region {
                        position: cell.position,
                        boundary: cell.boundary.clone(),
                        site: [0.0; 2],
                        area_km2: 1.0,
                        terrain: terrain[r],
                        elevation: elevation[r] as f32,
                        moisture: moisture[r] as f32,
                        warmth: 0.5,
                        climate_zone: None,
                        neighbours: cell.neighbours.clone(),
                        landmass: members[r],
                    })
                    .collect();
                rivers::drainage(&regions)
            };
            let mut rank = vec![usize::MAX; terrain.len()];
            for (i, &r) in order.iter().enumerate() {
                rank[r] = i;
            }
            for (r, t) in terrain.iter().enumerate() {
                assert_eq!(drainage[r].is_some(), t.is_land());
                if let Some(next) = drainage[r] {
                    assert!(mesh.cells[r].neighbours.contains(&next));
                    assert!(
                        rank[r] < rank[next],
                        "acyclic drainage eventually reaches sea"
                    );
                }
            }
        }
    }

    #[test]
    fn landmass_anchor_is_surface_area_weighted_across_the_chart_seam() {
        let mut map = route_fixture(&[Terrain::Plains; 3], &[(0, 1, 100.0), (1, 2, 100.0)]);
        let point = |lon: f64, lat: f64| {
            let lon = lon * PI / 180.0;
            let lat = lat * PI / 180.0;
            [
                math::cos(lat) * math::cos(lon),
                math::cos(lat) * math::sin(lon),
                math::sin(lat),
            ]
        };
        for (region, (lon, half)) in
            map.regions
                .iter_mut()
                .zip([(175.0, 3.0), (-175.0, 1.0), (0.0, 0.1)])
        {
            region.position = point(lon, 0.0);
            region.boundary = vec![
                point(lon - half, -half),
                point(lon - half, half),
                point(lon + half, half),
                point(lon + half, -half),
            ];
            region.area_km2 =
                (sphere::area(region.position, &region.boundary) * 1_600.0 * 1_600.0) as f32;
        }
        assert_eq!(describe_landmasses(&map.regions)[0].anchor, 0);
        map.regions.swap(0, 1);
        assert_eq!(describe_landmasses(&map.regions)[0].anchor, 1);
        // The same number of regions can change class as physical area grows.
        for r in &mut map.regions {
            r.area_km2 = 150_000.0;
        }
        assert_eq!(
            describe_landmasses(&map.regions)[0].kind,
            LandmassKind::Island
        );
        map.regions[0].area_km2 = 200_000.0;
        assert_eq!(
            describe_landmasses(&map.regions)[0].kind,
            LandmassKind::Continent
        );
    }

    #[test]
    fn sparse_routes_match_independent_oracles_and_inclusive_radii() {
        // Exhaustive all-pairs oracle stays deliberately tiny. Generated
        // sphere geometry and seam costs are covered independently above.
        let map = route_fixture(
            &[
                Terrain::Plains,
                Terrain::Hills,
                Terrain::Plains,
                Terrain::Sea,
                Terrain::Sea,
                Terrain::Plains,
                Terrain::Sea,
                Terrain::Plains,
                Terrain::Mountains,
                Terrain::Sea,
                Terrain::Plains,
                Terrain::Sea,
            ],
            &[
                (0, 1, 137.0),
                (1, 2, 213.0),
                (0, 2, 900.0),
                (0, 3, 182.0),
                (2, 4, 201.0),
                (3, 4, 328.0),
                (4, 5, 175.0),
                (5, 6, 201.0),
                (6, 7, 302.0),
                (7, 8, 2_000.0),
                (3, 9, 330.0),
                (9, 10, 222.0),
                (4, 9, 89.0),
                (9, 11, 2_100.0),
                (8, 11, 370.0),
            ],
        );
        let n = map.regions.len();
        let walk = floyd_oracle(&map, true);
        let sea = floyd_oracle(&map, false);
        for source in 0..n {
            let mut voyages = vec![f32::INFINITY; n];
            if map.coastal(source) {
                for (destination, effort) in voyages.iter_mut().enumerate() {
                    if source == destination || !map.coastal(destination) {
                        continue;
                    }
                    for &(depart, first) in map.edges.row(source) {
                        if map.regions[depart as usize].terrain.is_land() {
                            continue;
                        }
                        for &(arrive, last) in map.edges.row(destination) {
                            if map.regions[arrive as usize].terrain.is_land() {
                                continue;
                            }
                            let total = f64::from(first)
                                + 100.0
                                + sea[depart as usize * n + arrive as usize]
                                + f64::from(last)
                                + 100.0;
                            *effort = effort.min(total as f32);
                        }
                    }
                }
            }
            for destination in 0..n {
                assert_eq!(
                    map.distance(source, destination),
                    walk[source * n + destination] as f32
                );
                assert_eq!(
                    map.distance(source, destination),
                    map.distance(destination, source)
                );
                assert_eq!(map.voyage(source, destination), voyages[destination]);
                assert_eq!(
                    map.voyage(source, destination),
                    map.voyage(destination, source)
                );
                for (by_sea, expected) in [
                    (false, walk[source * n + destination] as f32),
                    (true, voyages[destination]),
                ] {
                    let path = map.route_path(source, destination, by_sea, CACHE_REACH_KM);
                    if !expected.is_finite() || expected > CACHE_REACH_KM {
                        assert!(path.is_none());
                        continue;
                    }
                    let path = path.unwrap();
                    assert_eq!(path.first(), Some(&source));
                    assert_eq!(path.last(), Some(&destination));
                    if by_sea {
                        assert!(
                            path[1..path.len() - 1]
                                .iter()
                                .all(|&r| !map.regions[r].terrain.is_land())
                        );
                    } else {
                        assert!(path.iter().all(|&r| map.regions[r].terrain.is_land()));
                    }
                    let cost: f64 = path
                        .windows(2)
                        .map(|p| {
                            let embark = if map.regions[p[0]].terrain.is_land()
                                != map.regions[p[1]].terrain.is_land()
                            {
                                f64::from(EMBARK)
                            } else {
                                0.0
                            };
                            f64::from(map.edges.get(p[0], p[1]).unwrap()) + embark
                        })
                        .sum();
                    assert_eq!(cost as f32, expected);
                }
            }
            for reach in [0.0, 400.0, CACHE_REACH_KM, f32::INFINITY] {
                let expected_walk: Vec<_> = (0..n)
                    .filter_map(|r| {
                        let effort = walk[source * n + r] as f32;
                        (effort.is_finite() && effort <= reach).then_some((r as u32, effort))
                    })
                    .collect();
                let expected_voyage: Vec<_> = voyages
                    .iter()
                    .enumerate()
                    .filter_map(|(r, &effort)| {
                        (effort.is_finite() && effort <= reach).then_some((r as u32, effort))
                    })
                    .collect();
                assert_eq!(map.walking_row(source, reach).as_ref(), expected_walk);
                assert_eq!(map.voyage_row(source, reach).as_ref(), expected_voyage);
            }
        }
    }

    #[test]
    fn walking_takes_the_land_detour_and_exact_fallback_exceeds_cache() {
        let map = route_fixture(
            &[
                Terrain::Plains,
                Terrain::Sea,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
                Terrain::Plains,
            ],
            &[
                (0, 1, 200.0),
                (1, 2, 200.0),
                (0, 3, 100.0),
                (3, 4, 100.0),
                (4, 5, 100.0),
                (5, 2, 100.0),
            ],
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
        assert!(
            !map.walking_row(0, CACHE_REACH_KM)
                .iter()
                .any(|&(r, _)| r == 24)
        );
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
            assert!(
                map.voyage_row(0, expected)
                    .contains(&(end as u32, expected))
            );
            assert!(map.voyage_row(0, expected - 1.0).is_empty());
            if expected > CACHE_REACH_KM {
                assert!(map.voyage_row(0, CACHE_REACH_KM).is_empty());
                assert!(
                    map.voyage_row(0, f32::INFINITY)
                        .contains(&(end as u32, expected))
                );
            }
            terrain[0] = Terrain::Mountains;
            assert_eq!(linear_map(&terrain).voyage(0, end), expected + 150.0);
        }
        let port = linear_map(&[
            Terrain::Plains,
            Terrain::Sea,
            Terrain::Plains,
            Terrain::Sea,
            Terrain::Plains,
        ]);
        assert_eq!(port.voyage(0, 2), 600.0);
        assert!(port.voyage(0, 4).is_infinite());
        let inland = linear_map(&[
            Terrain::Plains,
            Terrain::Plains,
            Terrain::Sea,
            Terrain::Plains,
        ]);
        assert!(inland.voyage(0, 3).is_infinite());
        assert_eq!(inland.voyage(1, 3), 600.0);
    }

    fn floyd_oracle(map: &Map, land: bool) -> Vec<f64> {
        let n = map.regions.len();
        let mut distance = vec![f64::INFINITY; n * n];
        for r in 0..n {
            if map.regions[r].terrain.is_land() != land {
                continue;
            }
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
                    distance[a * n + b] =
                        distance[a * n + b].min(distance[a * n + k] + distance[k * n + b]);
                }
            }
        }
        distance
    }

    fn linear_map(terrain: &[Terrain]) -> Map {
        // Abstract 100 km cells isolate route semantics from mesh generation.
        let links: Vec<_> = (1..terrain.len())
            .map(|r| {
                (
                    r - 1,
                    r,
                    50.0 * (terrain[r - 1].travel() + terrain[r].travel()),
                )
            })
            .collect();
        route_fixture(terrain, &links)
    }

    fn route_fixture(terrain: &[Terrain], links: &[(usize, usize, f32)]) -> Map {
        let n = terrain.len();
        let mut adjacent = vec![Vec::new(); n];
        for &(a, b, effort) in links {
            adjacent[a].push((b as u32, effort));
            adjacent[b].push((a as u32, effort));
        }
        for row in &mut adjacent {
            row.sort_unstable_by_key(|&(r, _)| r);
        }
        let mut mesh = sphere::mesh(1);
        assert!(n <= mesh.cells.len());
        mesh.cells.truncate(n);
        for (cell, row) in mesh.cells.iter_mut().zip(&adjacent) {
            cell.neighbours = row.iter().map(|&(r, _)| r as usize).collect();
        }
        let masses = landmasses(terrain, &mesh.cells);
        let regions: Vec<_> = (0..n)
            .map(|r| Region {
                position: mesh.cells[r].position,
                boundary: mesh.cells[r].boundary.clone(),
                site: chart_point(mesh.cells[r].position, 2.0 * PI, PI),
                area_km2: REFERENCE_AREA_KM2,
                terrain: terrain[r],
                elevation: 0.5,
                moisture: 0.5,
                warmth: 0.6,
                climate_zone: None,
                neighbours: mesh.cells[r].neighbours.clone(),
                landmass: masses[r],
            })
            .collect();
        let mut edges = RouteRows {
            offsets: vec![0],
            entries: Vec::new(),
        };
        for row in adjacent {
            edges.entries.extend(row);
            edges.offsets.push(edges.entries.len());
        }
        let mut map = Map {
            size: MapSize::Small,
            geography: GeographyVersion::CURRENT,
            radius_km: 100.0,
            width: TAU as f32,
            height: PI as f32,
            landmasses: describe_landmasses(&regions),
            regions,
            drainage: vec![None; n],
            drainage_order: Vec::new(),
            runoff: vec![0.0; n],
            rivers: Vec::new(),
            river_regions: vec![None; n],
            lakes: Vec::new(),
            lake_regions: vec![None; n],
            climate_zones: Vec::new(),
            edges,
            walking: RouteRows::default(),
            voyages: RouteRows::default(),
            valleys: Vec::new(),
        };
        map.walking = map.cache_routes(RouteMode::Walking);
        map.voyages = map.cache_routes(RouteMode::Voyage);
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

//! A history as a seed plus the ordered actions someone took. Any past
//! generation is recovered by replaying those actions, which the engine's
//! determinism makes exact; cached checkpoints keep scrubbing quick.
//!
//! Every telling has a permanent identity. Writing from an earlier reading
//! creates a child; reading or activating another telling never replaces one.

use crate::design::LanguageDesign;
use crate::ethos::{Axis, FoundingEthos};
use crate::geography::{GeographyVersion, MapSize};
use crate::ideas::{Craft, Revelation};
use crate::livelihood::Livelihood;
use crate::names::Naming;
use crate::polity::Rise;
use crate::world::{ContactKind, Params, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bumped whenever an engine change would make an existing recipe replay
/// differently. Saves record it so a mismatch can be reported.
/// Revision 34 adds versioned continental geography and related founding.
/// Revision-33 recipes retain their original spherical geography.
/// Revision 35 draws new worlds with ContinentalV3's geographic wind.
/// Revision 36 adds future and progressive marking.
/// Revision 37 adds pronouns and the genitive.
/// Revision 38 adds noun classes with determiner agreement.
/// Revision 39 adds vowel harmony and two assimilation laws.
/// Revision 40 adds internal vowel-pattern inflection.
pub const ENGINE_REVISION: u32 = 40;
/// Revision 33 replaces flat geography and its region identities with a sphere.
/// Earlier region-targeted actions cannot be replayed on the spherical mesh.
pub const SPHERICAL_GEOGRAPHY_REVISION: u32 = 33;
/// Identifies saved recipes. Kept from the project's first name, langgen,
/// so files saved before the rename still load.
pub const FORMAT: &str = "langgen-sim-recipe";
/// Generations between cached checkpoints.
const CHECKPOINT_EVERY: u32 = 10;
/// Longest single run, as a guard against typos.
const MAX_RUN: u32 = 2000;

/// One thing a person did to the world.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Action {
    Settle {
        #[serde(flatten)]
        choice: crate::settlement::SettlementChoice,
    },
    Found {
        /// What the people call themselves, built from their own words.
        naming: Naming,
        design: LanguageDesign,
        /// The language's own seed: the one its design was previewed with.
        seed: u64,
        power: f32,
        openness: f32,
        /// The land they settle; `None` lets the world choose.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        region: Option<usize>,
        /// How they live; `None` lets their land decide.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        livelihood: Option<Livelihood>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ethos: Option<FoundingEthos>,
    },
    /// A year-zero people whose speech descends from a living source.
    FoundRelated {
        source: usize,
        region: usize,
        naming: Naming,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        livelihood: Option<Livelihood>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ethos: Option<FoundingEthos>,
    },
    Connect {
        a: usize,
        b: usize,
        intensity: f32,
        contact: ContactKind,
    },
    Shift {
        community: usize,
        toward: usize,
    },
    /// A people organizes itself at `capital`, one of its lands, or its
    /// best-fed held land if no capital is supplied.
    State {
        community: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capital: Option<usize>,
    },
    /// A founder arises among a people, and a religion with him.
    Religion {
        community: usize,
    },
    /// A people comes upon a craft by itself.
    Craft {
        community: usize,
        craft: Craft,
    },
    /// Applies a catalog sound law to a living language at this reading.
    Law {
        variety: usize,
        law: String,
    },
    Temper {
        community: usize,
        axis: Axis,
        amount: f32,
    },
    Run {
        generations: u32,
    },
}

/// Document-local identities, independent of simulation random streams.
pub type TellingId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadingRef {
    pub telling: TellingId,
    pub point: HistoryPoint,
}

/// A complete history and its provenance. Full logs keep replay self-contained;
/// ancestry describes where the reader made another choice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Telling {
    pub id: TellingId,
    pub name: String,
    pub parent: Option<ReadingRef>,
    pub actions: Vec<Action>,
}

/// A saved history: enough to replay it exactly on the same engine revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub format: String,
    pub revision: u32,
    pub seed: u64,
    /// How large a map the seed draws. Recipes from before maps get the
    /// default size.
    #[serde(default)]
    pub map: MapSize,
    /// Missing geography identifies recipes made with the original spherical map.
    #[serde(default = "spherical_geography")]
    pub geography: GeographyVersion,
    pub active: TellingId,
    pub tellings: Vec<Telling>,
}

fn spherical_geography() -> GeographyVersion {
    GeographyVersion::SphericalV1
}

/// Where a replay has got to: the next action, and how far into it if it
/// is a run.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Cursor {
    action: usize,
    done: u32,
}

/// An exact reading in a history. `action` actions have happened, followed
/// by `offset` generations of the next run. Unlike a year this can distinguish
/// two decisions made without advancing time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub action: usize,
    pub offset: u32,
}

#[derive(Clone, Debug)]
pub struct Chronicle {
    pub seed: u64,
    pub map: MapSize,
    pub geography: GeographyVersion,
    active: TellingId,
    tellings: Vec<Telling>,
    latest: World,
    checkpoints: BTreeMap<u32, (Cursor, World)>,
}

impl Chronicle {
    pub fn new(seed: u64, map: MapSize) -> Self {
        Self::with_geography(seed, map, GeographyVersion::CURRENT)
    }

    /// A history whose map recipe remains fixed through every telling and reading.
    pub fn with_geography(seed: u64, map: MapSize, geography: GeographyVersion) -> Self {
        Self {
            seed,
            map,
            geography,
            active: 0,
            tellings: vec![Telling {
                id: 0,
                name: "The first telling".into(),
                parent: None,
                actions: Vec::new(),
            }],
            latest: World::with_geography(seed, Params::default(), map, geography),
            checkpoints: BTreeMap::new(),
        }
    }

    pub fn actions(&self) -> &[Action] {
        &self
            .telling(self.active)
            .expect("the active telling exists")
            .actions
    }

    fn actions_mut(&mut self) -> &mut Vec<Action> {
        &mut self
            .tellings
            .iter_mut()
            .find(|t| t.id == self.active)
            .expect("the active telling exists")
            .actions
    }

    pub fn active(&self) -> TellingId {
        self.active
    }

    pub fn telling(&self, id: TellingId) -> Result<&Telling, String> {
        self.tellings
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("there is no telling {id}"))
    }

    pub fn rename(&mut self, id: TellingId, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 120 {
            return Err("Give the telling a name of 1 to 120 characters.".into());
        }
        self.tellings
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| format!("there is no telling {id}"))?
            .name = name.into();
        Ok(())
    }

    /// Every telling, including the active one, in creation order.
    pub fn tellings(&self) -> &[Telling] {
        &self.tellings
    }

    /// The world after every action.
    pub fn latest(&self) -> &World {
        &self.latest
    }

    pub fn end(&self) -> HistoryPoint {
        match self.actions().last() {
            Some(Action::Run { generations }) => HistoryPoint {
                action: self.actions().len() - 1,
                offset: *generations,
            },
            _ => HistoryPoint {
                action: self.actions().len(),
                offset: 0,
            },
        }
    }

    /// The reading after every authored decision in `generation`.
    pub fn point_at(&self, generation: u32) -> HistoryPoint {
        let mut at = 0;
        for (index, action) in self.actions().iter().enumerate() {
            if let Action::Run { generations } = action {
                if at + generations > generation {
                    return HistoryPoint {
                        action: index,
                        offset: generation.saturating_sub(at),
                    };
                }
                at += generations;
            }
        }
        self.end()
    }

    fn prefix(&self, point: HistoryPoint) -> Result<Vec<Action>, String> {
        Self::prefix_of(self.actions(), point)
    }

    fn prefix_of(log: &[Action], point: HistoryPoint) -> Result<Vec<Action>, String> {
        if point.action > log.len() {
            return Err("that reading is beyond the end of this telling".into());
        }
        let mut actions = log[..point.action].to_vec();
        if point.offset > 0 {
            match log.get(point.action) {
                Some(Action::Run { generations }) if point.offset <= *generations => {
                    actions.push(Action::Run {
                        generations: point.offset,
                    });
                }
                _ => return Err("that reading is not within a recorded run".into()),
            }
        }
        Ok(actions)
    }

    /// An action boundary immediately after a run is also that run's full
    /// offset. Use the latter when comparing prefixes: a descendant can extend
    /// its final run, while the parent's next action remains at the boundary.
    fn canonical_point(log: &[Action], point: HistoryPoint) -> HistoryPoint {
        if point.offset == 0
            && point.action > 0
            && let Some(Action::Run { generations }) = log.get(point.action - 1)
        {
            return HistoryPoint {
                action: point.action - 1,
                offset: *generations,
            };
        }
        point
    }

    /// Publish a new history only after both the rewind and action succeed.
    /// A refusal must not set aside later years or change the active world.
    pub fn act_at(&mut self, point: HistoryPoint, action: Action) -> Result<(), String> {
        if point == self.end() {
            return self.act(action);
        }
        let mut candidate = self.clone();
        candidate.branch_at_point(point)?;
        candidate.act(action)?;
        *self = candidate;
        Ok(())
    }

    pub fn world_at_point(&self, point: HistoryPoint) -> Result<World, String> {
        if point == self.end() {
            return Ok(self.latest.clone());
        }
        let actions = self.prefix(point)?;
        Self::replay(self.seed, self.map, self.geography, &actions)
    }

    /// Each action with the generation it happened at.
    pub fn timeline(&self) -> Vec<(u32, &Action)> {
        let mut generation = 0;
        self.actions()
            .iter()
            .map(|action| {
                let at = generation;
                if let Action::Run { generations } = action {
                    generation += generations;
                }
                (at, action)
            })
            .collect()
    }

    /// Applies `action` to the latest world and records it, or explains why
    /// it cannot happen and changes nothing. A run straight after another
    /// run extends it, so playing generation by generation stays one action.
    pub fn act(&mut self, action: Action) -> Result<(), String> {
        let index = self.actions().len();
        apply(&mut self.latest, &action, index)?;
        match (self.actions_mut().last_mut(), &action) {
            (Some(Action::Run { generations }), Action::Run { generations: more })
                if *generations <= MAX_RUN - more =>
            {
                *generations += more;
            }
            _ => self.actions_mut().push(action),
        }
        Ok(())
    }

    /// Runs one generation at a time until a world event happens, or
    /// `limit` generations pass; returns how many ran.
    pub fn run_until_event(&mut self, limit: u32) -> u32 {
        let before = self.latest.events.len();
        let mut ran = 0;
        while ran < limit && self.latest.events.len() == before {
            self.act(Action::Run { generations: 1 })
                .expect("a one-generation run is always valid");
            ran += 1;
        }
        ran
    }

    /// Move a reading before its previous action, without changing any telling.
    pub fn previous(&self, point: HistoryPoint) -> Result<HistoryPoint, String> {
        self.prefix(point)?;
        Ok(HistoryPoint {
            action: if point.offset > 0 {
                point.action
            } else {
                point.action.saturating_sub(1)
            },
            offset: 0,
        })
    }

    /// Sets everything after `generation` aside, so new actions continue
    /// from there. A run that crosses it is shortened.
    pub fn branch_at(&mut self, generation: u32) {
        self.branch_at_point(self.point_at(generation))
            .expect("a reading obtained from this history is valid");
    }

    pub fn branch_at_point(&mut self, point: HistoryPoint) -> Result<(), String> {
        let kept = self.prefix(point)?;
        if kept == self.actions() {
            return Ok(());
        }
        let unchanged = self
            .actions()
            .iter()
            .zip(&kept)
            .take_while(|(a, b)| a == b)
            .count();
        let id = self
            .tellings
            .iter()
            .map(|t| t.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Too many tellings in this world")?;
        let parent = ReadingRef {
            telling: self.active,
            point,
        };
        self.tellings.push(Telling {
            id,
            name: format!("Telling {}", u64::from(id) + 1),
            parent: Some(parent),
            actions: kept,
        });
        self.active = id;
        self.checkpoints
            .retain(|_, (cursor, _)| cursor.action < unchanged);
        self.latest = self.replay_to(u32::MAX);
        Ok(())
    }

    /// Activate a telling by identity. Its peers and metadata remain intact.
    pub fn restore(&mut self, id: TellingId) -> Result<(), String> {
        if self.active == id {
            return Ok(());
        }
        let latest = Self::replay(
            self.seed,
            self.map,
            self.geography,
            &self.telling(id)?.actions,
        )?;
        self.active = id;
        self.latest = latest;
        self.checkpoints.clear();
        Ok(())
    }

    fn replay(
        seed: u64,
        map: MapSize,
        geography: GeographyVersion,
        actions: &[Action],
    ) -> Result<World, String> {
        let mut world = World::with_geography(seed, Params::default(), map, geography);
        for (i, action) in actions.iter().enumerate() {
            apply(&mut world, action, i).map_err(|e| format!("action {}: {e}", i + 1))?;
        }
        Ok(world)
    }

    pub fn reading(&self, id: TellingId) -> Result<Self, String> {
        let mut reading = self.clone();
        reading.restore(id)?;
        Ok(reading)
    }

    /// Last provably shared reading, before either history creates distinct
    /// entities. Equal numeric IDs after this point do not imply identity.
    pub fn common_reading(&self, a: TellingId, b: TellingId) -> Result<ReadingRef, String> {
        let ancestors = |mut id| -> Result<BTreeMap<TellingId, HistoryPoint>, String> {
            let mut path = BTreeMap::new();
            let log = &self.telling(id)?.actions;
            let mut through = match log.last() {
                Some(Action::Run { generations }) => HistoryPoint {
                    action: log.len() - 1,
                    offset: *generations,
                },
                _ => HistoryPoint {
                    action: log.len(),
                    offset: 0,
                },
            };
            loop {
                path.insert(id, through);
                match self.telling(id)?.parent {
                    Some(parent) => {
                        let cap = Self::canonical_point(
                            &self.telling(parent.telling)?.actions,
                            parent.point,
                        );
                        through = through.min(cap);
                        id = parent.telling;
                    }
                    None => return Ok(path),
                }
            }
        };
        let (left, right) = (ancestors(a)?, ancestors(b)?);
        let (telling, point) = left
            .iter()
            .rev()
            .find_map(|(id, point)| right.get(id).map(|other| (*id, (*point).min(*other))))
            .ok_or("These tellings have no shared founding")?;
        Ok(ReadingRef { telling, point })
    }

    /// The world as it stood at `generation`, after any actions taken then.
    pub fn world_at(&mut self, generation: u32) -> World {
        if generation >= self.latest.generation {
            return self.latest.clone();
        }
        self.replay_to(generation)
    }

    fn replay_to(&mut self, target: u32) -> World {
        let (mut cursor, mut world) = self
            .checkpoints
            .range(..=target)
            .next_back()
            .map(|(_, (c, w))| (*c, w.clone()))
            .unwrap_or_else(|| {
                let fresh =
                    World::with_geography(self.seed, Params::default(), self.map, self.geography);
                (Cursor::default(), fresh)
            });
        while cursor.action < self.actions().len() {
            match self.actions()[cursor.action].clone() {
                Action::Run { generations } => {
                    while cursor.done < generations && world.generation < target {
                        world.step();
                        cursor.done += 1;
                        if world.generation % CHECKPOINT_EVERY == 0 {
                            self.checkpoints
                                .entry(world.generation)
                                .or_insert_with(|| (cursor, world.clone()));
                        }
                    }
                    if cursor.done < generations {
                        return world;
                    }
                    cursor = Cursor {
                        action: cursor.action + 1,
                        done: 0,
                    };
                }
                other => {
                    apply(&mut world, &other, cursor.action)
                        .expect("recorded actions were valid when taken");
                    cursor.action += 1;
                }
            }
        }
        world
    }

    pub fn recipe(&self) -> Recipe {
        Recipe {
            format: FORMAT.into(),
            revision: ENGINE_REVISION,
            seed: self.seed,
            map: self.map,
            geography: self.geography,
            active: self.active,
            tellings: self.tellings.clone(),
        }
    }

    /// Rebuilds a history from a recipe. Pre-spherical recipes are refused.
    /// Other revision mismatches still load, but their histories may differ;
    /// callers should say so.
    pub fn from_recipe(recipe: &Recipe) -> Result<Self, String> {
        if recipe.format != FORMAT {
            return Err(format!("not a {FORMAT} file"));
        }
        if recipe.revision < SPHERICAL_GEOGRAPHY_REVISION {
            return Err("This save uses the earlier flat geography. Its original is still available for recovery; it cannot be replayed on a spherical world.".into());
        }
        let ids: std::collections::BTreeSet<_> = recipe.tellings.iter().map(|t| t.id).collect();
        if ids.len() != recipe.tellings.len() {
            return Err("Telling identities must be unique".into());
        }
        let active = recipe
            .tellings
            .iter()
            .find(|t| t.id == recipe.active)
            .ok_or("The active telling is missing")?;
        for telling in &recipe.tellings {
            if let Some(parent) = telling.parent {
                if parent.telling >= telling.id {
                    return Err("Telling ancestry must point to an earlier telling".into());
                }
                let log = &recipe
                    .tellings
                    .iter()
                    .find(|t| t.id == parent.telling)
                    .ok_or("A parent telling is missing")?
                    .actions;
                let prefix = Self::prefix_of(log, parent.point)?;
                if !holds(&telling.actions, &prefix) {
                    return Err("A telling does not share its recorded parent's history".into());
                }
            }
        }
        Ok(Self {
            seed: recipe.seed,
            map: recipe.map,
            geography: recipe.geography,
            active: recipe.active,
            tellings: recipe.tellings.clone(),
            latest: Self::replay(recipe.seed, recipe.map, recipe.geography, &active.actions)?,
            checkpoints: BTreeMap::new(),
        })
    }
}

/// Whether the history `longer` already tells all of `shorter`: the same
/// actions, with a final run at least as long.
fn holds(longer: &[Action], shorter: &[Action]) -> bool {
    let Some((last, rest)) = shorter.split_last() else {
        return true;
    };
    longer.len() >= shorter.len()
        && longer.starts_with(rest)
        && match (&longer[rest.len()], last) {
            (Action::Run { generations: a }, Action::Run { generations: b }) => a >= b,
            (a, b) => a == b,
        }
}

fn community(world: &World, index: usize) -> Result<(), String> {
    match world.communities.get(index) {
        None => Err(format!("there is no community {index}")),
        Some(c) if !c.living() => Err(format!("the {} are no more", world.community_name(index))),
        Some(_) => Ok(()),
    }
}

/// Checks `action` and only then changes `world`, so a refused action
/// leaves it as it was.
fn apply(world: &mut World, action: &Action, index: usize) -> Result<(), String> {
    let start = world.events.len();
    match action {
        Action::Settle { choice } => {
            world.settle(choice)?;
        }
        Action::Found {
            naming,
            design,
            seed,
            power,
            openness,
            region,
            livelihood,
            ethos,
        } => {
            design.validate()?;
            naming.validate()?;
            if let Some(ethos) = ethos {
                ethos.validate()?;
            }
            if *naming == Naming::Land {
                return Err("a founding people has no land name to be called by yet".into());
            }
            if let Some(r) = *region
                && !world
                    .map
                    .regions
                    .get(r)
                    .is_some_and(|r| r.terrain.is_land())
            {
                return Err(format!("region {r} is not land a people can settle"));
            }
            world.found_seeded(
                naming,
                &design.profile(),
                *seed,
                *power,
                *openness,
                *region,
                *livelihood,
                ethos.as_ref(),
            );
        }
        Action::FoundRelated {
            source,
            region,
            naming,
            livelihood,
            ethos,
        } => {
            world.found_related(*source, *region, naming, *livelihood, ethos.as_ref())?;
        }
        Action::Connect {
            a,
            b,
            intensity,
            contact,
        } => {
            community(world, *a)?;
            community(world, *b)?;
            if a == b {
                return Err("a community cannot be in contact with itself".into());
            }
            world.connect(*a, *b, *intensity, *contact)?;
        }
        Action::Shift {
            community: c,
            toward,
        } => {
            community(world, *c)?;
            community(world, *toward)?;
            if world.communities[*c].variety == world.communities[*toward].variety {
                return Err("they already speak the same language".into());
            }
            world.shift(*c, *toward);
        }
        Action::State {
            community: c,
            capital,
        } => {
            community(world, *c)?;
            if let Some(s) = world.state_of(*c) {
                return Err(format!(
                    "the {} already belong to {}",
                    world.community_name(*c),
                    world.states[s].name.meaning
                ));
            }
            if let Some(r) = *capital
                && !world.communities[*c].lands.contains(&r)
            {
                return Err(format!(
                    "the {} do not hold region {r}",
                    world.community_name(*c)
                ));
            }
            world.raise_state(*c, *capital, Rise::Proclaimed);
        }
        Action::Religion { community: c } => {
            community(world, *c)?;
            world.found_religion(*c, Revelation::Proclaimed);
        }
        Action::Craft {
            community: c,
            craft,
        } => {
            community(world, *c)?;
            if world.communities[*c].crafts.contains(craft) {
                return Err(format!(
                    "the {} already know {}",
                    world.community_name(*c),
                    craft.label()
                ));
            }
            world.learn(*c, *craft, None);
        }
        Action::Law { variety, law } => {
            let law = world
                .law_catalog()
                .iter()
                .find(|candidate| candidate.id == law)
                .ok_or_else(|| format!("there is no sound law {law}"))?;
            if law.id == "koine-levelling" {
                return Err("koine levelling is not a portable sound law".into());
            }
            if !world
                .communities
                .iter()
                .any(|c| c.living() && c.variety == *variety)
            {
                return Err(format!("language {variety} has no living speakers"));
            }
            let speech = &world.varieties[*variety];
            if law
                .assess_weighted(
                    speech.spoken_forms(),
                    &speech.profile.inventory,
                    speech.minimal,
                    speech.stress(),
                )
                .is_none()
            {
                return Err("the sound law would change no living forms".into());
            }
            let law = law.clone();
            world.apply_law(*variety, &law);
            world
                .authored_laws
                .insert((*variety, world.generation, law.id), index);
        }
        Action::Temper {
            community,
            axis,
            amount,
        } => {
            world.temper(*community, *axis, *amount)?;
        }
        Action::Run { generations } => {
            if *generations == 0 || *generations > MAX_RUN {
                return Err(format!("run between 1 and {MAX_RUN} generations"));
            }
            world.run(*generations);
        }
    }
    if !matches!(action, Action::Run { .. }) {
        world.decisions.push(crate::world::Decision {
            action: index,
            events: start..world.events.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Pole, TemperCause, WorldEvent};

    fn found(name: &str, preset: &str) -> Action {
        Action::Found {
            naming: Naming::People,
            design: LanguageDesign::preset(preset, 0)
                .unwrap_or_else(|| LanguageDesign::by_frequency(0, 14, 5)),
            seed: name.len() as u64,
            power: 0.5,
            openness: 0.5,
            region: None,
            livelihood: None,
            ethos: None,
        }
    }

    fn same(a: &World, b: &World) -> bool {
        a.generation == b.generation
            && a.map == b.map
            && a.climate == b.climate
            && a.places == b.places
            && a.river_names == b.river_names
            && a.continent_names == b.continent_names
            && a.cities == b.cities
            && a.events == b.events
            && a.causes == b.causes
            && a.triggers == b.triggers
            && a.contacts == b.contacts
            && a.communities == b.communities
            && a.states == b.states
            && a.religions == b.religions
            && a.varieties.len() == b.varieties.len()
            && a.varieties
                .iter()
                .zip(&b.varieties)
                .all(|(x, y)| x.lexicon == y.lexicon)
    }

    #[test]
    fn related_founders_inherit_the_living_language_without_migration() {
        let mut world = World::new(7, Params::static_society());
        apply(&mut world, &found("First", "familiar"), 0).unwrap();
        apply(&mut world, &found("Second", "semitic"), 1).unwrap();
        // A source can already have changed speech: community and variety
        // indices are not interchangeable, nor is the original design current.
        apply(
            &mut world,
            &Action::Shift {
                community: 0,
                toward: 1,
            },
            2,
        )
        .unwrap();
        let founding_size = world.communities[1].size;
        world.communities[0].size = founding_size * 2.3;
        let source = world.communities[0].clone();
        let parent = source.variety;
        let speech = world.varieties[parent].clone();
        let homeland = world.known_place(parent, source.home()).unwrap().clone();
        let region = (0..world.map.regions.len())
            .find(|&r| {
                world.map.regions[r].terrain.is_land()
                    && world.communities.iter().all(|c| !c.lands.contains(&r))
            })
            .unwrap();
        let action: Action = serde_json::from_value(serde_json::json!({
            "kind": "found-related",
            "source": 0,
            "region": region,
            "naming": { "kind": "epithet", "epithet": "new" },
            "livelihood": "foraging",
            "ethos": { "martial": 0.8 }
        }))
        .unwrap();
        let events = world.events.len();
        let contacts = world.contacts.clone();
        apply(&mut world, &action, 3).unwrap();
        let people = &world.communities[2];
        let daughter = people.variety;
        let inherited = &world.varieties[daughter];
        assert_eq!(world.generation, 0);
        assert_eq!(world.communities[0], source);
        assert_eq!(people.parents, vec![0]);
        assert_eq!(people.size, founding_size, "use a full founding population");
        assert_eq!(people.lands, vec![region]);
        assert_eq!(world.presence(2), vec![(region, founding_size)]);
        assert_eq!(
            (people.power, people.openness),
            (source.power, source.openness)
        );
        assert_eq!(people.prestige, source.power);
        assert_eq!(people.livelihood, Livelihood::Foraging);
        assert_eq!(people.ethos.martial, 0.8);
        assert_eq!(people.faith, None);
        assert!(people.crafts.is_empty());
        assert!(people.living());
        assert_eq!(
            inherited.parent,
            Some(crate::variety::Fork {
                variety: parent,
                generation: 0,
                inherited: speech.lexicon.lexemes.len() as u32,
            })
        );
        assert_eq!(world.family(daughter), world.family(parent));
        assert_eq!(inherited.lexicon, speech.lexicon);
        assert_eq!(inherited.grammar, speech.grammar);
        assert_eq!(inherited.morphology, speech.morphology);
        assert_eq!(inherited.profile, speech.profile);
        assert_eq!(inherited.given, speech.given);
        for word in &speech.lexicon.lexemes {
            assert_eq!(
                world.root_of(daughter, word.id),
                world.root_of(parent, word.id)
            );
        }
        assert_eq!(world.known_place(daughter, source.home()), Some(&homeland));
        assert_eq!(world.places[region].last().unwrap().variety, daughter);
        assert_eq!(people.name.coined, 0);
        assert_eq!(inherited.name.coined, 0);
        assert_eq!(world.contacts, contacts);
        assert_eq!(
            &world.events[events..],
            &[(0, WorldEvent::Found { community: 2 })],
            "authored kinship is founding, not a migration or a split"
        );
        assert_eq!(
            world.decisions.last(),
            Some(&crate::world::Decision {
                action: 3,
                events: events..events + 1,
            })
        );
    }

    #[test]
    fn related_founders_evolve_independently_and_replay() {
        let mut history = Chronicle::new(7, MapSize::Small);
        history.act(found("First", "familiar")).unwrap();
        let region = (0..history.latest().map.regions.len())
            .find(|&r| {
                history.latest().map.regions[r].terrain.is_land()
                    && !history.latest().communities[0].lands.contains(&r)
            })
            .unwrap();
        history
            .act(
                serde_json::from_value(serde_json::json!({
                    "kind": "found-related",
                    "source": 0,
                    "region": region,
                    "naming": { "kind": "speakers" }
                }))
                .unwrap(),
            )
            .unwrap();
        let parent = history.latest().communities[0].variety;
        let daughter = history.latest().communities[1].variety;
        history.act(Action::Run { generations: 1 }).unwrap();
        let world = history.latest();
        let speech = &world.varieties[daughter];
        let law = world
            .law_catalog()
            .iter()
            .find(|law| {
                law.id != "koine-levelling"
                    && speech.lexicon.living().any(|word| {
                        law.changes(
                            &word.form,
                            &law.apply(&word.form, speech.minimal, speech.stress()),
                            speech.stress(),
                        )
                    })
            })
            .unwrap()
            .id;
        let source_lexicon = world.varieties[parent].lexicon.clone();
        let before = speech.lexicon.clone();
        history
            .act(Action::Law {
                variety: daughter,
                law: law.into(),
            })
            .unwrap();
        let world = history.latest();
        assert_eq!(world.varieties[parent].lexicon, source_lexicon);
        assert_ne!(world.varieties[daughter].lexicon, before);
        assert_eq!(world.varieties[daughter].laws.last(), Some(&(1, law)));
        assert_eq!(world.family(daughter), world.family(parent));
        assert_eq!(
            world.varieties[daughter].parent.unwrap().generation,
            0,
            "independent change must not invent earlier founding history"
        );
        let recipe: Recipe =
            serde_json::from_str(&serde_json::to_string(&history.recipe()).unwrap()).unwrap();
        let replay = Chronicle::from_recipe(&recipe).unwrap();
        assert!(same(world, replay.latest()));
        assert_eq!(world.decisions, replay.latest().decisions);
        for (a, b) in world.varieties.iter().zip(&replay.latest().varieties) {
            assert_eq!(a.parent, b.parent);
            assert_eq!(a.grammar, b.grammar);
            assert_eq!(a.laws, b.laws);
        }
    }

    #[test]
    fn related_founding_refusals_preserve_world_and_recipe() {
        fn refuse(history: &mut Chronicle, action: Action) {
            let before = history.latest().clone();
            let recipe = history.recipe();
            assert!(history.act(action).is_err());
            let world = history.latest();
            assert_eq!(history.recipe(), recipe);
            assert!(same(world, &before));
            assert_eq!(world.decisions, before.decisions);
            assert_eq!(world.authored_laws, before.authored_laws);
            assert_eq!(world.places, before.places);
            assert_eq!(world.river_names, before.river_names);
            assert_eq!(world.continent_names, before.continent_names);
            assert_eq!(world.cities, before.cities);
            for (a, b) in world.varieties.iter().zip(&before.varieties) {
                assert_eq!(a.parent, b.parent);
                assert_eq!(a.grammar, b.grammar);
                assert_eq!(a.laws, b.laws);
                assert_eq!(a.exonyms, b.exonyms);
                assert_eq!(a.river_exonyms, b.river_exonyms);
            }
        }
        let mut history = Chronicle::new(7, MapSize::Small);
        history.act(found("First", "familiar")).unwrap();
        let home = history.latest().communities[0].home();
        let sea = (0..history.latest().map.regions.len())
            .find(|&r| !history.latest().map.regions[r].terrain.is_land())
            .unwrap();
        for (source, region, naming, ethos) in [
            (usize::MAX, home, Naming::People, None),
            (0, usize::MAX, Naming::People, None),
            (0, sea, Naming::People, None),
            (0, home, Naming::Land, None),
            (
                0,
                home,
                Naming::Place {
                    place: "moon".into(),
                },
                None,
            ),
            (
                0,
                home,
                Naming::Epithet {
                    epithet: "moon".into(),
                },
                None,
            ),
            (
                0,
                home,
                Naming::People,
                Some(FoundingEthos {
                    martial: Some(f32::NAN),
                    ..FoundingEthos::default()
                }),
            ),
            (
                0,
                home,
                Naming::People,
                Some(FoundingEthos {
                    open: Some(1.01),
                    ..FoundingEthos::default()
                }),
            ),
        ] {
            refuse(
                &mut history,
                Action::FoundRelated {
                    source,
                    region,
                    naming,
                    livelihood: None,
                    ethos,
                },
            );
        }
        let valid = Action::FoundRelated {
            source: 0,
            region: home,
            naming: Naming::People,
            livelihood: None,
            ethos: None,
        };
        history.latest.communities[0].ended = Some(0);
        refuse(&mut history, valid.clone());
        history.latest.communities[0].ended = None;
        history.act(Action::Run { generations: 1 }).unwrap();
        refuse(&mut history, valid);
    }

    #[test]
    fn authored_laws_refuse_invalid_or_unspoken_inputs_without_decisions() {
        let mut world = World::solo(0, &crate::SoundProfile::base(), Params::static_society());
        let stress = world.varieties[0].stress();
        let noop = world
            .law_catalog()
            .iter()
            .find(|law| {
                law.id != "koine-levelling"
                    && law
                        .assess_weighted(
                            world.varieties[0]
                                .grammar
                                .forms(&world.varieties[0].lexicon),
                            &world.varieties[0].profile.inventory,
                            world.varieties[0].minimal,
                            stress,
                        )
                        .is_none()
            })
            .unwrap()
            .id;
        for (variety, law) in [
            (0, "missing-law"),
            (0, "koine-levelling"),
            (0, noop),
            (usize::MAX, "w-fortition"),
        ] {
            let before = world.clone();
            assert!(
                apply(
                    &mut world,
                    &Action::Law {
                        variety,
                        law: law.into()
                    },
                    0
                )
                .is_err()
            );
            assert!(same(&world, &before));
            assert_eq!(world.decisions, before.decisions);
            assert_eq!(world.authored_laws, before.authored_laws);
            assert_eq!(world.varieties[0].laws, before.varieties[0].laws);
        }
        world.communities[0].ended = Some(0);
        let before = world.clone();
        assert!(
            apply(
                &mut world,
                &Action::Law {
                    variety: 0,
                    law: "w-fortition".into()
                },
                0
            )
            .is_err()
        );
        assert!(same(&world, &before));
        assert!(world.decisions.is_empty());
        assert!(world.authored_laws.is_empty());
    }

    #[test]
    fn authored_laws_spread_as_waves_and_can_repeat_without_a_quiet_span() {
        let params = Params {
            wave_rate: 1_000_000.0,
            sound_change_rate: 0.0,
            loan_rate: 0.0,
            innovation_rate: 0.0,
            ..Params::static_society()
        };
        let mut world = World::solo(0, &crate::SoundProfile::base(), params);
        let daughter = world.split(0, None, 0.0);
        let target = world.communities[daughter].variety;
        world.communities[daughter].lands = world.communities[0].lands.clone();
        world
            .connect(0, daughter, 1.0, ContactKind::Neighbours)
            .unwrap();
        let word = world.varieties[0].lexicon.living().next().unwrap().id;
        let before = crate::Form::from_ipa("wawa").unwrap();
        let after = crate::Form::from_ipa("vava").unwrap();
        world.varieties[0].lexicon.get_mut(word).form = before.clone();
        world.varieties[target].lexicon.get_mut(word).form = before.clone();
        let action = Action::Law {
            variety: 0,
            law: "w-fortition".into(),
        };
        apply(&mut world, &action, 0).unwrap();
        assert_eq!(world.varieties[0].lexicon.get(word).form, after);
        assert_eq!(world.varieties[target].lexicon.get(word).form, before);
        world.step();
        assert_eq!(world.varieties[target].lexicon.get(word).form, after);
        assert_eq!(world.varieties[target].waves, vec![(1, "w-fortition", 0)]);
        assert!(
            !world
                .authored_laws
                .contains_key(&(target, 1, "w-fortition"))
        );
        world.varieties[0].lexicon.get_mut(word).form = before;
        apply(&mut world, &action, 1).unwrap();
        assert_eq!(world.varieties[0].lexicon.get(word).form, after);
        assert_eq!(
            world.varieties[0].laws,
            vec![(0, "w-fortition"), (1, "w-fortition")]
        );
        assert_eq!(
            world.decisions.iter().map(|d| d.action).collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    fn settlement(world: &World, intensity: f32) -> Action {
        use crate::settlement::{SettlementChoice, SettlementIntent};
        for intent in [SettlementIntent::Partition, SettlementIntent::Settlers] {
            if let Some(option) = world
                .settlement_options(0, intent, 0.5)
                .unwrap()
                .into_iter()
                .find(|o| o.reason.is_none())
            {
                return Action::Settle {
                    choice: SettlementChoice {
                        community: 0,
                        destination: option.region,
                        intent,
                        share: 0.5,
                        naming: None,
                        intensity,
                    },
                };
            }
        }
        panic!("the sample needs somewhere to settle")
    }

    #[test]
    fn neutral_ethos_recipe_is_identical_with_multipliers_enabled() {
        let zero = FoundingEthos {
            martial: Some(0.0),
            open: Some(0.0),
            pious: Some(0.0),
            hierarchical: Some(0.0),
            roving: Some(0.0),
            seaward: Some(0.0),
        };
        let mut baseline = World::new(
            7,
            Params {
                ethos_enabled: false,
                ..Params::default()
            },
        );
        let mut neutral = World::new(
            7,
            Params {
                ethos_shifts: false,
                ..Params::default()
            },
        );
        let home = (0..baseline.map.regions.len())
            .find(|&r| baseline.map.regions[r].terrain.is_land() && !baseline.map.island(r))
            .unwrap();
        let mut actions = vec![
            found("Hill", "familiar"),
            found("Coast", "polynesian"),
            found("Court", "semitic"),
        ];
        for action in &mut actions {
            if let Action::Found { ethos, region, .. } = action {
                *ethos = Some(zero);
                *region = Some(home);
            }
        }
        actions.extend([
            Action::Connect {
                a: 0,
                b: 1,
                intensity: 0.8,
                contact: ContactKind::Trade,
            },
            Action::Connect {
                a: 2,
                b: 1,
                intensity: 0.8,
                contact: ContactKind::Rule,
            },
            Action::Religion { community: 1 },
            Action::Craft {
                community: 0,
                craft: Craft::Seafaring,
            },
            Action::Run { generations: 12 },
        ]);
        for (index, action) in actions.iter().enumerate() {
            apply(&mut baseline, action, index).unwrap();
            apply(&mut neutral, action, index).unwrap();
        }
        for (offset, action) in [
            settlement(&baseline, 0.5),
            Action::Shift {
                community: 1,
                toward: 0,
            },
            Action::Run { generations: 148 },
        ]
        .into_iter()
        .enumerate()
        {
            apply(&mut baseline, &action, actions.len() + offset).unwrap();
            apply(&mut neutral, &action, actions.len() + offset).unwrap();
        }
        assert!(same(&baseline, &neutral));
        assert_eq!(baseline.events, neutral.events);
        assert_eq!(baseline.contacts, neutral.contacts);
        assert_eq!(baseline.cities, neutral.cities);
        assert_eq!(baseline.places, neutral.places);
        for (a, b) in baseline.varieties.iter().zip(&neutral.varieties) {
            assert_eq!(a.given, b.given);
            assert_eq!(a.laws, b.laws);
            assert_eq!(a.waves, b.waves);
        }
    }

    #[test]
    fn ethos_recipe_validates_before_mutation_and_replays_twice() {
        let mut chronicle = Chronicle::new(7, MapSize::Medium);
        let mut action = found("Hill", "familiar");
        if let Action::Found { ethos, .. } = &mut action {
            *ethos = Some(FoundingEthos {
                martial: Some(1.1),
                ..FoundingEthos::default()
            });
        }
        assert!(chronicle.act(action.clone()).is_err());
        assert!(chronicle.latest().communities.is_empty());
        if let Action::Found { ethos, .. } = &mut action {
            *ethos = Some(FoundingEthos {
                martial: Some(0.4),
                ..FoundingEthos::default()
            });
        }
        chronicle.act(action).unwrap();
        chronicle.act(Action::Run { generations: 1 }).unwrap();
        chronicle
            .act(Action::Temper {
                community: 0,
                axis: Axis::Martial,
                amount: 0.4,
            })
            .unwrap();
        for amount in [-0.4, -0.1, 0.25] {
            chronicle
                .act(Action::Temper {
                    community: 0,
                    axis: Axis::Martial,
                    amount,
                })
                .unwrap();
        }
        let transitions: Vec<_> = chronicle
            .latest()
            .events
            .iter()
            .filter_map(|(_, event)| match event {
                WorldEvent::Temper {
                    pole,
                    entered,
                    cause: TemperCause::Fate,
                    ..
                } => Some((*pole, *entered)),
                _ => None,
            })
            .collect();
        assert_eq!(
            transitions,
            [(Pole::High, true), (Pole::High, false), (Pole::High, true)]
        );
        chronicle.act(Action::Run { generations: 20 }).unwrap();
        let recipe = chronicle.recipe();
        let a = Chronicle::from_recipe(&recipe).unwrap();
        let b = Chronicle::from_recipe(&recipe).unwrap();
        assert!(same(a.latest(), b.latest()));
        assert_eq!(a.latest().events, b.latest().events);
        let before = chronicle.latest().communities.clone();
        assert!(
            chronicle
                .act(Action::Temper {
                    community: 0,
                    axis: Axis::Martial,
                    amount: 1.1
                })
                .is_err()
        );
        assert_eq!(chronicle.latest().communities, before);
        assert_eq!(chronicle.recipe(), recipe);
        assert_eq!(chronicle.world_at(0).communities[0].ethos.martial, 0.4);
    }

    fn sample() -> Chronicle {
        let mut c = Chronicle::new(7, MapSize::default());
        c.act(found("Hill", "familiar")).unwrap();
        let mut coast = found("Coast", "polynesian");
        if let Action::Found { region, .. } = &mut coast {
            *region = Some(c.latest().communities[0].home());
        }
        c.act(coast).unwrap();
        c.act(Action::Run { generations: 15 }).unwrap();
        c.act(Action::Connect {
            a: 0,
            b: 1,
            intensity: 0.7,
            contact: ContactKind::Trade,
        })
        .unwrap();
        c.act(settlement(c.latest(), 0.3)).unwrap();
        c.act(Action::State {
            community: 0,
            capital: None,
        })
        .unwrap();
        c.act(Action::Run { generations: 22 }).unwrap();
        c
    }

    #[test]
    fn past_generations_replay_exactly() {
        let mut c = sample();
        let mut direct = World::new(7, Params::default());
        let design = |p: &str| LanguageDesign::preset(p, 0).unwrap().profile();
        direct.found_seeded(
            &Naming::People,
            &design("familiar"),
            4,
            0.5,
            0.5,
            None,
            None,
            None,
        );
        direct.found_seeded(
            &Naming::People,
            &design("polynesian"),
            5,
            0.5,
            0.5,
            Some(direct.communities[0].home()),
            None,
            None,
        );
        direct.run(12);
        assert!(same(&c.world_at(12), &direct));
        // Actions taken at a generation are part of that generation's view.
        assert_eq!(c.world_at(15).communities.len(), 3);
        assert!(same(&c.world_at(37), c.latest()));
        // Checkpoints give the same answer as a cold replay.
        let warm = c.world_at(31);
        let mut cold = sample();
        cold.checkpoints.clear();
        assert!(same(&warm, &cold.replay_to(31)));
        assert_eq!(warm.decisions, cold.replay_to(31).decisions);
    }

    #[test]
    fn branching_discards_the_later_history() {
        let mut c = sample();
        c.branch_at(20);
        assert_eq!(c.latest().generation, 20);
        assert_eq!(
            c.timeline().last().map(|(_, a)| (*a).clone()),
            Some(Action::Run { generations: 5 })
        );
        c.act(Action::Shift {
            community: 1,
            toward: 0,
        })
        .unwrap();
        c.act(Action::Run { generations: 5 }).unwrap();
        assert_eq!(c.latest().generation, 25);
        assert!(
            c.latest()
                .events
                .iter()
                .any(|(_, e)| matches!(e, crate::WorldEvent::Shift { .. }))
        );
    }

    #[test]
    fn named_tellings_survive_reading_restoration_and_equal_continuations() {
        let mut c = sample();
        let original = c.recipe();
        let before = c.previous(c.previous(c.end()).unwrap()).unwrap();
        assert_eq!(c.recipe(), original, "undo only moves a reading");
        c.act_at(before, Action::Run { generations: 1 }).unwrap();
        let child = c.active();
        c.rename(child, "The quieter coast").unwrap();
        assert_eq!(c.tellings().len(), 2);
        assert_eq!(c.tellings()[0], original.tellings[0]);
        c.restore(0).unwrap();
        assert_eq!(c.recipe().tellings.len(), 2);
        c.act_at(before, Action::Run { generations: 1 }).unwrap();
        assert_ne!(c.active(), child);
        assert_eq!(
            c.tellings().len(),
            3,
            "equal actions do not erase identities"
        );
        assert_eq!(
            c.common_reading(child, c.active()).unwrap().point,
            Chronicle::canonical_point(&original.tellings[0].actions, before)
        );
        c.restore(child).unwrap();
        assert_eq!(c.telling(child).unwrap().name, "The quieter coast");
        let back = Chronicle::from_recipe(&c.recipe()).unwrap();
        assert_eq!(back.recipe(), c.recipe());
    }

    #[test]
    fn shared_reading_caps_an_extended_run_at_its_parent_boundary() {
        let mut c = Chronicle::new(7, MapSize::default());
        c.act(found("X", "familiar")).unwrap();
        c.act(Action::Run { generations: 3 }).unwrap();
        let before = HistoryPoint {
            action: 2,
            offset: 0,
        };
        c.act(found("Y", "polynesian")).unwrap();
        c.act_at(before, Action::Run { generations: 1 }).unwrap();
        let shared = c.common_reading(0, c.active()).unwrap();
        let parent = c.reading(shared.telling).unwrap();
        let world = parent.world_at_point(shared.point).unwrap();
        assert_eq!(world.generation, 3);
        assert_eq!(world.communities.len(), 1);
        assert_eq!(c.latest().generation, 4);
        assert_eq!(c.reading(0).unwrap().latest().communities.len(), 2);
    }

    #[test]
    fn undo_and_invalid_actions() {
        let mut c = sample();
        let previous = c.previous(c.end()).unwrap();
        assert_eq!(c.world_at_point(previous).unwrap().generation, 15);
        assert_eq!(c.latest().generation, 37);
        let before = c.actions().len();
        assert!(
            c.act(Action::Shift {
                community: 0,
                toward: 9
            })
            .is_err()
        );
        let mut unnamed = found("X", "familiar");
        if let Action::Found { naming, .. } = &mut unnamed {
            *naming = Naming::Place {
                place: "moon".into(),
            };
        }
        assert!(c.act(unnamed).is_err());
        let mut bad = found("X", "familiar");
        if let Action::Found { design, .. } = &mut bad {
            design.sounds.clear();
        }
        assert!(c.act(bad).is_err());
        let sea = (0..c.latest().map.regions.len())
            .find(|&r| !c.latest().map.regions[r].terrain.is_land())
            .unwrap();
        let mut drowned = found("X", "familiar");
        if let Action::Found { region, .. } = &mut drowned {
            *region = Some(sea);
        }
        assert!(c.act(drowned).is_err());
        assert_eq!(c.actions().len(), before, "rejected actions change nothing");
    }

    #[test]
    fn physically_refused_contact_keeps_world_and_action_history_unchanged() {
        let mut c = Chronicle::new(7, MapSize::Medium);
        let map = &c.latest().map;
        let shores: Vec<_> = (0..map.regions.len()).filter(|&r| map.coastal(r)).collect();
        let (a, b) = shores
            .iter()
            .find_map(|&a| {
                shores.iter().find_map(|&b| {
                    (map.overseas(a, b) && map.voyage(a, b).is_finite()).then_some((a, b))
                })
            })
            .unwrap();
        for region_id in [a, b] {
            let mut action = found("Coast", "familiar");
            if let Action::Found { region, .. } = &mut action {
                *region = Some(region_id);
            }
            c.act(action).unwrap();
        }
        let before = c.latest().clone();
        let recipe = c.recipe();
        assert!(
            c.act(Action::Connect {
                a: 0,
                b: 1,
                intensity: 0.8,
                contact: ContactKind::Rule,
            })
            .is_err()
        );
        assert_eq!(c.recipe(), recipe);
        assert_eq!(c.latest().communities, before.communities);
        assert_eq!(c.latest().events, before.events);
        assert_eq!(c.latest().contacts, before.contacts);
        assert_eq!(c.latest().states, before.states);
    }

    #[test]
    fn a_people_settles_the_land_it_is_founded_on() {
        let mut c = sample();
        let world = c.latest();
        let open = (0..world.map.regions.len())
            .rev()
            .find(|&r| {
                world.map.regions[r].terrain.is_land()
                    && world.communities.iter().all(|p| !p.lands.contains(&r))
            })
            .unwrap();
        let mut chosen = found("X", "familiar");
        if let Action::Found { region, .. } = &mut chosen {
            *region = Some(open);
        }
        c.act(chosen.clone()).unwrap();
        assert_eq!(c.latest().communities.last().unwrap().home(), open);
        // The choice is part of the recipe, so it replays.
        assert_eq!(c.actions().last(), Some(&chosen));
    }

    #[test]
    fn consecutive_runs_merge_and_events_stop_a_run() {
        let mut c = sample();
        let before = c.actions().len();
        c.act(Action::Run { generations: 1 }).unwrap();
        c.act(Action::Run { generations: 1 }).unwrap();
        assert_eq!(c.actions().len(), before, "runs extend the last run");
        assert_eq!(c.actions().last(), Some(&Action::Run { generations: 24 }));
        // Replaying the merged run gives the same world as stepping did.
        let mut cold = Chronicle::from_recipe(&c.recipe()).unwrap();
        assert!(same(&cold.world_at(39), c.latest()));

        let mut world = Chronicle::new(1, MapSize::default());
        world.act(found("Hill", "familiar")).unwrap();
        let ran = world.run_until_event(400);
        assert!(ran < 400, "a growing community eventually splits");
        assert_eq!(world.latest().events.len(), 2, "the founding and the split");
    }

    #[test]
    fn recipes_round_trip() {
        let c = sample();
        let json = serde_json::to_string(&c.recipe()).unwrap();
        let back: Recipe = serde_json::from_str(&json).unwrap();
        let loaded = Chronicle::from_recipe(&back).unwrap();
        assert!(same(loaded.latest(), c.latest()));
        assert_eq!(loaded.latest().decisions, c.latest().decisions);
        let mut wrong = back.clone();
        wrong.format = "something-else".into();
        assert!(Chronicle::from_recipe(&wrong).is_err());
    }

    #[test]
    fn pre_spherical_recipes_cannot_reinterpret_valid_region_actions() {
        let mut source =
            Chronicle::with_geography(7, MapSize::Small, GeographyVersion::SphericalV1);
        let home = source.latest().map.landmasses[0].anchor;
        let mut action = found("First", "familiar");
        if let Action::Found { region, .. } = &mut action {
            *region = Some(home);
        }
        source.act(action).unwrap();
        let mut saved = serde_json::to_value(source.recipe()).unwrap();
        saved.as_object_mut().unwrap().remove("geography");
        for revision in [0, SPHERICAL_GEOGRAPHY_REVISION - 1] {
            saved["revision"] = revision.into();
            let recipe: Recipe = serde_json::from_value(saved.clone()).unwrap();
            assert_eq!(recipe.geography, GeographyVersion::SphericalV1);
            assert!(Chronicle::from_recipe(&recipe).is_err());
        }
        for revision in [SPHERICAL_GEOGRAPHY_REVISION, ENGINE_REVISION + 1] {
            saved["revision"] = revision.into();
            let recipe: Recipe = serde_json::from_value(saved.clone()).unwrap();
            let restored = Chronicle::from_recipe(&recipe).unwrap();
            assert!(same(restored.latest(), source.latest()));
            assert_eq!(restored.latest().communities[0].home(), home);
        }
    }

    #[test]
    fn geography_survives_legacy_migration_readings_checkpoints_and_tellings() {
        for geography in [
            GeographyVersion::SphericalV1,
            GeographyVersion::ContinentalV2,
            GeographyVersion::ContinentalV3,
        ] {
            let mut source = if geography == GeographyVersion::CURRENT {
                Chronicle::new(7, MapSize::Small)
            } else {
                Chronicle::with_geography(7, MapSize::Small, geography)
            };
            let empty = World::with_geography(7, Params::default(), MapSize::Small, geography);
            assert!(same(source.latest(), &empty));

            // The authored region is chosen on the recorded geography, not redrawn
            // on whatever geography new worlds now use.
            let home = empty
                .map
                .regions
                .iter()
                .position(|r| r.terrain.is_land())
                .unwrap();
            let mut action = found("First", "familiar");
            if let Action::Found { region, .. } = &mut action {
                *region = Some(home);
            }
            source.act(action).unwrap();
            let founding = source.latest().clone();
            let after_founding = source.end();
            source.act(Action::Run { generations: 25 }).unwrap();
            let original = source.latest().clone();

            let mut saved = serde_json::to_value(source.recipe()).unwrap();
            if geography == GeographyVersion::SphericalV1 {
                // Revision-33 saves predate the field. Loading and saving one
                // must make its old map explicit, never adopt the new generator.
                saved["revision"] = serde_json::json!(33);
                saved.as_object_mut().unwrap().remove("geography");
            }
            let migrated: Recipe = serde_json::from_value(saved).unwrap();
            assert_eq!(migrated.geography, geography);
            let mut loaded = Chronicle::from_recipe(&migrated).unwrap();
            assert!(same(loaded.latest(), &original));
            let explicit = serde_json::to_value(loaded.recipe()).unwrap();
            assert_eq!(
                explicit["geography"],
                serde_json::to_value(geography).unwrap()
            );
            let resaved: Recipe = serde_json::from_value(explicit).unwrap();
            loaded = Chronicle::from_recipe(&resaved).unwrap();
            assert!(same(loaded.latest(), &original));

            // Exact zero-time readings and undo also regenerate the original map.
            assert!(same(
                &loaded.world_at_point(HistoryPoint::default()).unwrap(),
                &empty
            ));
            let previous = loaded.previous(loaded.end()).unwrap();
            assert_eq!(previous, after_founding);
            assert!(same(&loaded.world_at_point(previous).unwrap(), &founding));
            let mut at_fourteen = founding.clone();
            at_fourteen.run(14);
            assert!(same(
                &loaded
                    .world_at_point(HistoryPoint {
                        action: 1,
                        offset: 14,
                    })
                    .unwrap(),
                &at_fourteen
            ));

            // Scrub forward to populate checkpoints, then backward through one.
            let mut at_twenty_one = founding.clone();
            at_twenty_one.run(21);
            assert!(same(&loaded.world_at(21), &at_twenty_one));
            assert!(loaded.checkpoints.contains_key(&10));
            assert!(loaded.checkpoints.contains_key(&20));
            assert!(same(&loaded.world_at(14), &at_fourteen));

            let mut at_twenty = founding.clone();
            at_twenty.run(20);
            loaded.branch_at(20);
            assert!(same(loaded.latest(), &at_twenty));
            let child = loaded.active();
            loaded.act(Action::Run { generations: 3 }).unwrap();
            let mut continued = at_twenty;
            continued.run(3);
            assert!(same(loaded.latest(), &continued));
            loaded.restore(0).unwrap();
            assert!(same(loaded.latest(), &original));
            assert!(same(loaded.reading(child).unwrap().latest(), &continued));

            // Exporting while another telling is active must keep both histories
            // replayable on the same geography after a second load.
            let branched: Recipe =
                serde_json::from_str(&serde_json::to_string(&loaded.recipe()).unwrap()).unwrap();
            assert_eq!(branched.geography, geography);
            let mut reloaded = Chronicle::from_recipe(&branched).unwrap();
            assert!(same(reloaded.latest(), &original));
            reloaded.restore(child).unwrap();
            assert!(same(reloaded.latest(), &continued));
            reloaded
                .act_at(after_founding, Action::Run { generations: 1 })
                .unwrap();
            let mut at_one = founding;
            at_one.run(1);
            assert!(same(reloaded.latest(), &at_one));
            assert_eq!(reloaded.recipe().geography, geography);
        }
    }

    #[test]
    fn exact_readings_and_refused_interventions_preserve_the_history() {
        let mut history = Chronicle::new(7, MapSize::Small);
        history.act(found("First", "familiar")).unwrap();
        let before_second = history.end();
        history.act(found("Second", "airy")).unwrap();
        history.act(Action::Run { generations: 3 }).unwrap();
        let saved = history.recipe();
        let world = history.latest().clone();
        assert_eq!(
            history
                .world_at_point(before_second)
                .unwrap()
                .communities
                .len(),
            1
        );
        assert!(
            history
                .act_at(
                    before_second,
                    Action::Shift {
                        community: 0,
                        toward: 99
                    }
                )
                .is_err()
        );
        assert_eq!(history.recipe(), saved);
        assert!(same(history.latest(), &world));
        history
            .act_at(before_second, found("Different", "familiar"))
            .unwrap();
        assert_eq!(history.latest().generation, 0);
        assert_eq!(history.latest().communities.len(), 2);
        assert_eq!(history.tellings()[0].actions, saved.tellings[0].actions);
        assert!(
            history
                .world_at_point(HistoryPoint {
                    action: 0,
                    offset: 1
                })
                .is_err()
        );
        history.act(Action::Run { generations: 3 }).unwrap();
        let reading = history.end();
        history.act(Action::Run { generations: 1 }).unwrap();
        assert_eq!(history.world_at_point(reading).unwrap().generation, 3);
        history.branch_at_point(reading).unwrap();
        assert_eq!(history.latest().generation, 3);
    }

    #[test]
    fn decisions_follow_exact_prefixes_and_branch_local_indices() {
        let mut c = sample();
        let original = c.latest().decisions.clone();
        assert_eq!(
            original.iter().map(|d| d.action).collect::<Vec<_>>(),
            vec![0, 1, 3, 4, 5]
        );
        for decision in &original {
            let before = c
                .world_at_point(HistoryPoint {
                    action: decision.action,
                    offset: 0,
                })
                .unwrap();
            let after = c
                .world_at_point(HistoryPoint {
                    action: decision.action + 1,
                    offset: 0,
                })
                .unwrap();
            assert_eq!(decision.events, before.events.len()..after.events.len());
            assert_eq!(after.decisions.last(), Some(decision));
            assert!(before.decisions.iter().all(|d| d.action < decision.action));
        }
        let prefix = HistoryPoint {
            action: 2,
            offset: 10,
        };
        c.act_at(
            prefix,
            Action::Temper {
                community: 0,
                axis: Axis::Martial,
                amount: 0.01,
            },
        )
        .unwrap();
        assert_eq!(c.latest().decisions[..2], original[..2]);
        assert_eq!(c.latest().decisions.last().unwrap().action, 3);
        let branch = c.active();
        let branch_decisions = c.latest().decisions.clone();
        let before_refusal = c.recipe();
        assert!(
            c.act(Action::Shift {
                community: 0,
                toward: usize::MAX
            })
            .is_err()
        );
        assert_eq!(c.recipe(), before_refusal);
        assert_eq!(c.latest().decisions, branch_decisions);
        c.restore(0).unwrap();
        assert_eq!(c.latest().decisions, original);
        c.restore(branch).unwrap();
        assert_eq!(c.latest().decisions, branch_decisions);
    }
}

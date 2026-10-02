//! A history as a seed plus the ordered actions someone took. Any past
//! generation is recovered by replaying those actions, which the engine's
//! determinism makes exact; cached checkpoints keep scrubbing quick.
//!
//! Every telling has a permanent identity. Writing from an earlier reading
//! creates a child; reading or activating another telling never replaces one.

use crate::design::LanguageDesign;
use crate::ethos::{Axis, FoundingEthos};
use crate::geography::MapSize;
use crate::ideas::{Craft, Revelation};
use crate::livelihood::Livelihood;
use crate::names::Naming;
use crate::polity::Rise;
use crate::world::{ContactKind, Params, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bumped whenever an engine change would make an existing recipe replay
/// differently. Saves record it so a mismatch can be reported.
pub const ENGINE_REVISION: u32 = 31;
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
    pub active: TellingId,
    pub tellings: Vec<Telling>,
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
    active: TellingId,
    tellings: Vec<Telling>,
    latest: World,
    checkpoints: BTreeMap<u32, (Cursor, World)>,
}

impl Chronicle {
    pub fn new(seed: u64, map: MapSize) -> Self {
        Self {
            seed,
            map,
            active: 0,
            tellings: vec![Telling {
                id: 0,
                name: "The first telling".into(),
                parent: None,
                actions: Vec::new(),
            }],
            latest: World::with_map(seed, Params::default(), map),
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
        Self::replay(self.seed, self.map, &actions)
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
        apply(&mut self.latest, &action)?;
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
        let latest = Self::replay(self.seed, self.map, &self.telling(id)?.actions)?;
        self.active = id;
        self.latest = latest;
        self.checkpoints.clear();
        Ok(())
    }

    fn replay(seed: u64, map: MapSize, actions: &[Action]) -> Result<World, String> {
        let mut world = World::with_map(seed, Params::default(), map);
        for (i, action) in actions.iter().enumerate() {
            apply(&mut world, action).map_err(|e| format!("action {}: {e}", i + 1))?;
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
                let fresh = World::with_map(self.seed, Params::default(), self.map);
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
                    apply(&mut world, &other).expect("recorded actions were valid when taken");
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
            active: self.active,
            tellings: self.tellings.clone(),
        }
    }

    /// Rebuilds a history from a recipe. A recipe from another engine
    /// revision still loads, but its words may differ from when it was
    /// saved; callers should say so.
    pub fn from_recipe(recipe: &Recipe) -> Result<Self, String> {
        if recipe.format != FORMAT {
            return Err(format!("not a {FORMAT} file"));
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
        let mut chronicle = Self::new(recipe.seed, recipe.map);
        chronicle.latest = Self::replay(recipe.seed, recipe.map, &active.actions)?;
        chronicle.active = recipe.active;
        chronicle.tellings = recipe.tellings.clone();
        Ok(chronicle)
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
fn apply(world: &mut World, action: &Action) -> Result<(), String> {
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
        let mut actions = vec![
            found("Hill", "familiar"),
            found("Coast", "polynesian"),
            found("Court", "semitic"),
        ];
        for action in &mut actions {
            if let Action::Found { ethos, .. } = action {
                *ethos = Some(zero);
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
        for action in &actions {
            apply(&mut baseline, action).unwrap();
            apply(&mut neutral, action).unwrap();
        }
        for action in [
            settlement(&baseline, 0.5),
            Action::Shift {
                community: 1,
                toward: 0,
            },
            Action::Run { generations: 148 },
        ] {
            apply(&mut baseline, &action).unwrap();
            apply(&mut neutral, &action).unwrap();
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
        let mut wrong = back.clone();
        wrong.format = "something-else".into();
        assert!(Chronicle::from_recipe(&wrong).is_err());
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
}

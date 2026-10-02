//! A history as a seed plus the ordered actions someone took. Any past
//! generation is recovered by replaying those actions, which the engine's
//! determinism makes exact; cached checkpoints keep scrubbing quick.
//!
//! Nothing written is thrown away. Undoing, or writing on from an earlier
//! year, sets the abandoned actions aside as another telling, which the
//! book shows struck through and can return to.

use crate::design::LanguageDesign;
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
pub const ENGINE_REVISION: u32 = 25;
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
    },
    Connect {
        a: usize,
        b: usize,
        intensity: f32,
        contact: ContactKind,
    },
    Split {
        community: usize,
        /// `None` lets the new community choose its own name.
        #[serde(default)]
        naming: Option<Naming>,
        intensity: f32,
    },
    Shift {
        community: usize,
        toward: usize,
    },
    /// A people organizes itself into a state, with its court at
    /// `capital`, one of its lands, or its heart land if `None`.
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
    Run {
        generations: u32,
    },
}

/// Why a telling was set aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SetAside {
    /// Taken back with undo.
    Undone,
    /// Left when the history was written on from an earlier year, or when
    /// another telling was taken up.
    Rewritten,
}

/// A history set aside: every action it had, from the founding on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Telling {
    pub why: SetAside,
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
    pub actions: Vec<Action>,
    /// Tellings set aside, oldest first. They never affect the replay.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tellings: Vec<Telling>,
}

/// Where a replay has got to: the next action, and how far into it if it
/// is a run.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Cursor {
    action: usize,
    done: u32,
}

#[derive(Clone, Debug)]
pub struct Chronicle {
    pub seed: u64,
    pub map: MapSize,
    actions: Vec<Action>,
    tellings: Vec<Telling>,
    latest: World,
    checkpoints: BTreeMap<u32, (Cursor, World)>,
}

impl Chronicle {
    pub fn new(seed: u64, map: MapSize) -> Self {
        Self {
            seed,
            map,
            actions: Vec::new(),
            tellings: Vec::new(),
            latest: World::with_map(seed, Params::default(), map),
            checkpoints: BTreeMap::new(),
        }
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    /// Tellings set aside, oldest first.
    pub fn tellings(&self) -> &[Telling] {
        &self.tellings
    }

    /// The world after every action.
    pub fn latest(&self) -> &World {
        &self.latest
    }

    /// Each action with the generation it happened at.
    pub fn timeline(&self) -> Vec<(u32, &Action)> {
        let mut generation = 0;
        self.actions
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
        match (self.actions.last_mut(), &action) {
            (Some(Action::Run { generations }), Action::Run { generations: more }) => {
                *generations += more;
            }
            _ => self.actions.push(action),
        }
        // A telling the history has caught up with is no longer set aside.
        let actions = &self.actions;
        self.tellings.retain(|t| !holds(actions, &t.actions));
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

    /// Takes back the last action, keeping it in a telling set aside.
    /// Undoing again extends that same telling rather than starting another.
    pub fn undo(&mut self) -> Option<Action> {
        let before = self.actions.clone();
        let action = self.actions.pop()?;
        self.set_aside(before, SetAside::Undone);
        let len = self.actions.len();
        self.checkpoints
            .retain(|_, (cursor, _)| cursor.action < len);
        self.latest = self.replay_to(u32::MAX);
        Some(action)
    }

    /// Sets everything after `generation` aside, so new actions continue
    /// from there. A run that crosses it is shortened.
    pub fn branch_at(&mut self, generation: u32) {
        let mut kept = Vec::new();
        let mut at = 0;
        for action in &self.actions {
            match action {
                Action::Run { generations } if at + generations > generation => {
                    if generation > at {
                        kept.push(Action::Run {
                            generations: generation - at,
                        });
                    }
                    break;
                }
                Action::Run { generations } => {
                    at += generations;
                    kept.push(action.clone());
                }
                other => kept.push(other.clone()),
            }
        }
        let unchanged = self
            .actions
            .iter()
            .zip(&kept)
            .take_while(|(a, b)| a == b)
            .count();
        let before = std::mem::replace(&mut self.actions, kept);
        self.set_aside(before, SetAside::Rewritten);
        self.checkpoints
            .retain(|_, (cursor, _)| cursor.action < unchanged);
        self.latest = self.replay_to(u32::MAX);
    }

    /// Takes up the telling at `index` again, setting the present one
    /// aside in its place.
    pub fn restore(&mut self, index: usize) -> Result<(), String> {
        let telling = self
            .tellings
            .get(index)
            .ok_or_else(|| format!("there is no telling {index}"))?;
        let mut taken = Self::new(self.seed, self.map);
        for (i, action) in telling.actions.iter().enumerate() {
            taken
                .act(action.clone())
                .map_err(|e| format!("action {}: {e}", i + 1))?;
        }
        self.tellings.remove(index);
        let before = std::mem::replace(&mut self.actions, taken.actions);
        self.latest = taken.latest;
        self.checkpoints = taken.checkpoints;
        self.set_aside(before, SetAside::Rewritten);
        Ok(())
    }

    /// Keeps `actions` as a telling, unless the present history or another
    /// telling already holds all of them. Tellings they contain are merged
    /// into them.
    fn set_aside(&mut self, actions: Vec<Action>, why: SetAside) {
        if actions.is_empty()
            || holds(&self.actions, &actions)
            || self.tellings.iter().any(|t| holds(&t.actions, &actions))
        {
            return;
        }
        self.tellings.retain(|t| !holds(&actions, &t.actions));
        self.tellings.push(Telling { why, actions });
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
        while cursor.action < self.actions.len() {
            match &self.actions[cursor.action] {
                Action::Run { generations } => {
                    while cursor.done < *generations && world.generation < target {
                        world.step();
                        cursor.done += 1;
                        if world.generation % CHECKPOINT_EVERY == 0 {
                            self.checkpoints
                                .entry(world.generation)
                                .or_insert_with(|| (cursor, world.clone()));
                        }
                    }
                    if cursor.done < *generations {
                        return world;
                    }
                    cursor = Cursor {
                        action: cursor.action + 1,
                        done: 0,
                    };
                }
                other => {
                    apply(&mut world, other).expect("recorded actions were valid when taken");
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
            actions: self.actions.clone(),
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
        let mut chronicle = Self::new(recipe.seed, recipe.map);
        for (i, action) in recipe.actions.iter().enumerate() {
            chronicle
                .act(action.clone())
                .map_err(|e| format!("action {}: {e}", i + 1))?;
        }
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
        Action::Found {
            naming,
            design,
            seed,
            power,
            openness,
            region,
            livelihood,
        } => {
            design.validate()?;
            naming.validate()?;
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
            world.connect(*a, *b, *intensity, *contact);
        }
        Action::Split {
            community: c,
            naming,
            intensity,
        } => {
            community(world, *c)?;
            if let Some(naming) = naming {
                naming.validate()?;
            }
            world.split(*c, naming.as_ref(), *intensity);
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
        }
    }

    fn same(a: &World, b: &World) -> bool {
        a.generation == b.generation
            && a.communities == b.communities
            && a.states == b.states
            && a.religions == b.religions
            && a.varieties.len() == b.varieties.len()
            && a.varieties
                .iter()
                .zip(&b.varieties)
                .all(|(x, y)| x.lexicon == y.lexicon)
    }

    fn sample() -> Chronicle {
        let mut c = Chronicle::new(7, MapSize::default());
        c.act(found("Hill", "familiar")).unwrap();
        c.act(found("Coast", "polynesian")).unwrap();
        c.act(Action::Run { generations: 15 }).unwrap();
        c.act(Action::Connect {
            a: 0,
            b: 1,
            intensity: 0.7,
            contact: ContactKind::Trade,
        })
        .unwrap();
        c.act(Action::Split {
            community: 0,
            naming: None,
            intensity: 0.3,
        })
        .unwrap();
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
        );
        direct.found_seeded(
            &Naming::People,
            &design("polynesian"),
            5,
            0.5,
            0.5,
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
    fn undone_actions_are_set_aside_and_can_return() {
        let mut c = sample();
        let full = c.actions().to_vec();
        let end = c.latest().clone();
        c.undo();
        c.undo();
        assert_eq!(c.tellings().len(), 1, "consecutive undos make one telling");
        assert_eq!(c.tellings()[0].why, SetAside::Undone);
        assert_eq!(c.tellings()[0].actions, full);

        c.restore(0).unwrap();
        assert_eq!(c.actions(), &full[..]);
        assert!(same(c.latest(), &end));
        assert!(
            c.tellings().is_empty(),
            "the present held nothing the restored telling lacks"
        );

        c.branch_at(5);
        c.act(Action::Split {
            community: 0,
            naming: None,
            intensity: 0.2,
        })
        .unwrap();
        assert_eq!(c.tellings().len(), 1);
        assert_eq!(c.tellings()[0].why, SetAside::Rewritten);
        let branched = c.actions().to_vec();
        c.restore(0).unwrap();
        assert_eq!(c.actions(), &full[..]);
        assert_eq!(
            c.tellings()[0].actions,
            branched,
            "the present is set aside"
        );

        let json = serde_json::to_string(&c.recipe()).unwrap();
        let back = Chronicle::from_recipe(&serde_json::from_str(&json).unwrap()).unwrap();
        assert_eq!(back.tellings(), c.tellings());
    }

    #[test]
    fn running_on_past_a_telling_takes_it_up() {
        let mut c = sample();
        c.undo();
        assert_eq!(c.tellings().len(), 1);
        c.act(Action::Run { generations: 30 }).unwrap();
        assert!(
            c.tellings().is_empty(),
            "running on tells everything the undone run told"
        );
    }

    #[test]
    fn undo_and_invalid_actions() {
        let mut c = sample();
        assert_eq!(c.undo(), Some(Action::Run { generations: 22 }));
        assert_eq!(c.latest().generation, 15);
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
}

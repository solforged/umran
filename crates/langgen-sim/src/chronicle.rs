//! A history as a seed plus the ordered actions someone took. Any past
//! generation is recovered by replaying those actions, which the engine's
//! determinism makes exact; cached checkpoints keep scrubbing quick.

use crate::design::LanguageDesign;
use crate::world::{ContactKind, Params, World};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bumped whenever an engine change would make an existing recipe replay
/// differently. Saves record it so a mismatch can be reported.
pub const ENGINE_REVISION: u32 = 5;
/// Identifies saved recipes.
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
        name: String,
        design: LanguageDesign,
        /// The language's own seed: the one its design was previewed with.
        seed: u64,
        power: f32,
        openness: f32,
    },
    Connect {
        a: usize,
        b: usize,
        intensity: f32,
        contact: ContactKind,
    },
    Split {
        community: usize,
        name: String,
        intensity: f32,
    },
    Shift {
        community: usize,
        toward: usize,
    },
    Run {
        generations: u32,
    },
}

/// A saved history: enough to replay it exactly on the same engine revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    pub format: String,
    pub revision: u32,
    pub seed: u64,
    pub actions: Vec<Action>,
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
    actions: Vec<Action>,
    latest: World,
    checkpoints: BTreeMap<u32, (Cursor, World)>,
}

impl Chronicle {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            actions: Vec::new(),
            latest: World::new(seed, Params::default()),
            checkpoints: BTreeMap::new(),
        }
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
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
        let mut next = self.latest.clone();
        apply(&mut next, &action)?;
        self.latest = next;
        match (self.actions.last_mut(), &action) {
            (Some(Action::Run { generations }), Action::Run { generations: more }) => {
                *generations += more;
            }
            _ => self.actions.push(action),
        }
        Ok(())
    }

    /// Runs one generation at a time until a split or shift happens, or
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

    /// Removes the last action.
    pub fn undo(&mut self) -> Option<Action> {
        let action = self.actions.pop()?;
        let len = self.actions.len();
        self.checkpoints
            .retain(|_, (cursor, _)| cursor.action < len);
        self.latest = self.replay_to(u32::MAX);
        Some(action)
    }

    /// Discards everything after `generation`, so new actions continue from
    /// there. A run that crosses it is shortened.
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
        self.actions = kept;
        self.checkpoints
            .retain(|_, (cursor, _)| cursor.action < unchanged);
        self.latest = self.replay_to(u32::MAX);
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
            .unwrap_or_else(|| (Cursor::default(), World::new(self.seed, Params::default())));
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
            actions: self.actions.clone(),
        }
    }

    /// Rebuilds a history from a recipe. A recipe from another engine
    /// revision still loads, but its words may differ from when it was
    /// saved; callers should say so.
    pub fn from_recipe(recipe: &Recipe) -> Result<Self, String> {
        if recipe.format != FORMAT {
            return Err(format!("not a {FORMAT} file"));
        }
        let mut chronicle = Self::new(recipe.seed);
        for (i, action) in recipe.actions.iter().enumerate() {
            chronicle
                .act(action.clone())
                .map_err(|e| format!("action {}: {e}", i + 1))?;
        }
        Ok(chronicle)
    }
}

fn community(world: &World, index: usize) -> Result<(), String> {
    if index < world.communities.len() {
        Ok(())
    } else {
        Err(format!("there is no community {index}"))
    }
}

fn apply(world: &mut World, action: &Action) -> Result<(), String> {
    match action {
        Action::Found {
            name,
            design,
            seed,
            power,
            openness,
        } => {
            design.validate()?;
            if name.trim().is_empty() {
                return Err("a community needs a name".into());
            }
            world.found_seeded(name.trim(), &design.profile(), *seed, *power, *openness);
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
            name,
            intensity,
        } => {
            community(world, *c)?;
            if name.trim().is_empty() {
                return Err("the new community needs a name".into());
            }
            world.split(*c, name.trim(), *intensity);
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
            name: name.into(),
            design: LanguageDesign::preset(preset, 0)
                .unwrap_or_else(|| LanguageDesign::typical(0, 14, 5)),
            seed: name.len() as u64,
            power: 0.5,
            openness: 0.5,
        }
    }

    fn same(a: &World, b: &World) -> bool {
        a.generation == b.generation
            && a.communities == b.communities
            && a.varieties.len() == b.varieties.len()
            && a.varieties
                .iter()
                .zip(&b.varieties)
                .all(|(x, y)| x.lexicon == y.lexicon)
    }

    fn sample() -> Chronicle {
        let mut c = Chronicle::new(7);
        c.act(found("Hill", "typical")).unwrap();
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
            name: "Upland".into(),
            intensity: 0.3,
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
        direct.found_seeded("Hill", &design("typical"), 4, 0.5, 0.5);
        direct.found_seeded("Coast", &design("polynesian"), 5, 0.5, 0.5);
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
        assert!(c.act(found("", "typical")).is_err());
        let mut bad = found("X", "typical");
        if let Action::Found { design, .. } = &mut bad {
            design.sounds.clear();
        }
        assert!(c.act(bad).is_err());
        assert_eq!(c.actions().len(), before, "rejected actions change nothing");
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

        let mut world = Chronicle::new(1);
        world.act(found("Hill", "typical")).unwrap();
        let ran = world.run_until_event(400);
        assert!(ran < 400, "a growing community eventually splits");
        assert_eq!(world.latest().events.len(), 1);
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

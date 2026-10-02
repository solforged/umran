//! Diglossia: two forms of one language for different purposes, a high
//! form for writing, law, and learning, and a low form everyone speaks. A
//! state's written standard is fixed as a classical form, frozen as it
//! stood, when grammarians settle it after centuries of writing, as Panini
//! did for Sanskrit, or when its state falls and leaves it behind, as Rome
//! left Latin. From then on its speakers and their daughters write the
//! classical form, not their speech, so their speech changes at the
//! ordinary pace again and counts as unwritten; the classical form keeps
//! its standing and lends them learned words, as Latin gave French
//! fragile beside the inherited frail. Readers of other languages that
//! deal with them borrow from it too, as Japanese did from Classical
//! Chinese. Diglossia ends for a language when it is written in its own
//! right: when a state takes it as its standard, as Alfonso X did with
//! Castilian, or when a faith that translates its scripture reaches its
//! speakers, as Luther's Bible did German. The classical form then lends
//! to it more slowly, as Latin still lends to English.

use crate::ideas::Craft;
use crate::rng::{key, stream};
use crate::world::{World, WorldEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Generations a standard must have been written before grammarians may
/// fix it, and the chance per generation that they then do.
const CLASSICAL_AGE: u32 = 12;
const FIXING: f32 = 0.1;
/// Generations a standard must have been written for its state's fall to
/// leave it classical.
const FALL_TRADITION: u32 = 6;
/// Standing of a classical form's words, however few still write it, and
/// how far it stands above any people that learns from it: the learned
/// form outranks even its own rulers' speech.
pub(crate) const CLASSICAL_PRESTIGE: f32 = 0.8;
const LEARNED_EDGE: f32 = 0.3;
/// How closely those who write a classical form deal with it, doubled for
/// a people that reads; the share left once their own speech is written;
/// and the share for readers of other languages that deal with them.
const CLASSICAL_INTENSITY: f32 = 0.2;
const VERNACULAR_SHARE: f32 = 0.5;
const FOREIGN_SHARE: f32 = 0.5;

/// How a standard came to be fixed as a classical form.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fixing {
    /// Grammarians settled it after centuries of writing.
    Age,
    /// Its state fell and left it behind.
    Fall,
}

/// A state's standard, frozen as it stood when fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Classical {
    /// The frozen variety.
    pub variety: usize,
    pub fixed: u32,
    pub how: Fixing,
}

/// What brought a language to be written in its own right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vernacular {
    /// A state took it as its standard.
    Standard { state: usize },
    /// A faith that translates its written teaching reached its speakers.
    Scripture { religion: usize },
}

impl World {
    /// Whether `variety`'s speakers write a classical form rather than
    /// their own speech.
    pub fn diglossic(&self, variety: usize) -> bool {
        let v = &self.varieties[variety];
        v.high.is_some() && v.vernacular.is_none()
    }

    /// `variety` is written from now, unless its speakers write a
    /// classical form instead.
    pub(crate) fn begin_writing(&mut self, variety: usize) {
        if !self.diglossic(variety) {
            self.varieties[variety]
                .written
                .get_or_insert(self.generation);
        }
    }

    /// Grammarians may fix a standing state's standard once it has been
    /// written for `CLASSICAL_AGE` generations.
    pub(crate) fn fix_classics(&mut self) {
        for s in 0..self.states.len() {
            let state = &self.states[s];
            let (true, None, Some(standard)) = (state.standing(), state.classical, state.standard)
            else {
                continue;
            };
            let v = self.communities[state.rulers].variety;
            let Some(written) = self.varieties[v].written else {
                continue;
            };
            if self.generation < written.max(standard) + CLASSICAL_AGE {
                continue;
            }
            let mut rng = stream(
                self.seed,
                &[
                    key("step"),
                    u64::from(self.generation),
                    key("classical"),
                    s as u64,
                ],
            );
            if rng.r#gen::<f32>() < FIXING {
                self.fix(s, Fixing::Age);
            }
        }
    }

    /// A falling state leaves its written standard behind as a classical
    /// form, if it was not fixed already and has been written long enough
    /// to have a tradition (`FALL_TRADITION`).
    pub(crate) fn fix_at_fall(&mut self, s: usize) {
        let state = &self.states[s];
        let written = self.varieties[self.communities[state.rulers].variety].written;
        if let (None, Some(standard), Some(written)) = (state.classical, state.standard, written)
            && self.generation >= written.max(standard) + FALL_TRADITION
        {
            self.fix(s, Fixing::Fall);
        }
    }

    /// Freezes state `s`'s standard as a classical form, which its
    /// speakers and the speakers of every language descended from it write
    /// from now on in place of their own speech.
    pub(crate) fn fix(&mut self, s: usize, how: Fixing) {
        self.refresh_places();
        let generation = self.generation;
        let v = self.communities[self.states[s].rulers].variety;
        let mut classical = self.varieties[v].fork(v, generation);
        self.inherit_places(v, &mut classical);
        classical.name = self.varieties[v].name.clone();
        let high = self.varieties.len();
        self.varieties.push(classical);
        let spoken = self.spoken();
        let writers: Vec<usize> = (0..high)
            .filter(|&x| spoken[x] && (x == v || self.descends(x, v)))
            .collect();
        for x in writers {
            let speech = &mut self.varieties[x];
            speech.high = Some(high);
            speech.vernacular = None;
            speech.written = None;
        }
        self.states[s].classical = Some(Classical {
            variety: high,
            fixed: generation,
            how,
        });
        self.events
            .push((generation, WorldEvent::Fixed { state: s }));
    }

    /// `variety`'s speakers begin to write their own speech, ending its
    /// diglossia.
    pub(crate) fn write_vernacular(&mut self, variety: usize, by: Vernacular) {
        if !self.diglossic(variety) {
            return;
        }
        let generation = self.generation;
        let v = &mut self.varieties[variety];
        v.vernacular = Some(generation);
        v.written = Some(generation);
        self.events
            .push((generation, WorldEvent::Vernacular { variety, by }));
    }

    /// The classical forms each living people learns words from, how
    /// closely, and with what standing: the one its language is written in
    /// or was, and, for a people that reads, those of the peoples it deals
    /// with.
    pub(crate) fn classical_sources(&self) -> Vec<(usize, usize, f32, f32)> {
        let mut out = Vec::new();
        for c in self.living() {
            let speech = &self.varieties[self.communities[c].variety];
            let reads = self.communities[c].crafts.contains(&Craft::Writing);
            let standing = CLASSICAL_PRESTIGE.max(self.communities[c].prestige + LEARNED_EDGE);
            let mut own = None;
            if let Some(high) = speech.high {
                let share = if speech.vernacular.is_some() {
                    VERNACULAR_SHARE
                } else {
                    1.0
                };
                let reading = if reads { 2.0 } else { 1.0 };
                out.push((c, high, CLASSICAL_INTENSITY * reading * share, standing));
                own = Some(high);
            }
            if !reads {
                continue;
            }
            let mut seen: Vec<usize> = own.into_iter().collect();
            for (o, _, _) in self.partners(c) {
                let Some(high) = self.varieties[self.communities[o].variety].high else {
                    continue;
                };
                if !seen.contains(&high) {
                    seen.push(high);
                    out.push((c, high, CLASSICAL_INTENSITY * FOREIGN_SHARE, standing));
                }
            }
        }
        out
    }

    /// The state whose classical form `variety` is, if it is one.
    pub fn classical_of(&self, variety: usize) -> Option<usize> {
        self.states
            .iter()
            .position(|s| s.classical.is_some_and(|k| k.variety == variety))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::Origin;
    use crate::polity::{Fall, Rise};
    use crate::profile::SoundProfile;
    use crate::world::{ContactKind, Params};

    /// A people ruling a dialect split from it, with a written standard;
    /// fixed as a classical form at once if `fix`.
    fn realm(seed: u64, fix: bool) -> (World, usize, usize, usize) {
        let mut world = World::new(seed, Params::static_society());
        let rulers = world.found(&SoundProfile::base(), 0.9, 0.5);
        let subjects = world.split(rulers, None, 0.0);
        world.connect(rulers, subjects, 0.8, ContactKind::Rule);
        let s = world.rules(rulers).expect("rule raises a state");
        world.adopt_standard(s);
        world.learn(rulers, Craft::Writing, None);
        if fix {
            world.fix(s, Fixing::Age);
        }
        (world, s, rulers, subjects)
    }

    #[test]
    fn a_fixed_standard_freezes_while_speech_goes_free() {
        let mut drifted = 0;
        for seed in 0..8 {
            let (mut world, s, rulers, subjects) = realm(seed, true);
            let high = world.states[s].classical.unwrap().variety;
            for c in [rulers, subjects] {
                let v = world.communities[c].variety;
                assert_eq!(world.varieties[v].high, Some(high));
                assert!(world.diglossic(v));
                assert_eq!(
                    world.varieties[v].written, None,
                    "they write the classical form"
                );
            }
            let v = world.communities[rulers].variety;
            assert_eq!(world.pace(v), 1.0, "the brake is off their speech");
            world.learn(subjects, Craft::Writing, None);
            assert_eq!(world.variety_of(subjects).written, None);
            let frozen: Vec<_> = world.varieties[high].lexicon.lexemes.clone();
            world.run(120);
            assert_eq!(world.varieties[high].lexicon.lexemes, frozen);
            drifted += usize::from(world.varieties[v].lexicon.lexemes != frozen);
        }
        assert!(drifted >= 6, "speech changed in only {drifted} of 8");
    }

    /// Writers borrow words back from the classical form, so an old word
    /// returns beside its own worn descendant, as Latin fragile beside
    /// French frail.
    #[test]
    fn a_classical_form_lends_learned_doublets() {
        let (mut learned, mut doublets) = (0, 0);
        for seed in 0..6 {
            let (mut world, s, rulers, _) = realm(seed, true);
            world.run(200);
            let high = world.states[s].classical.unwrap().variety;
            let v = world.communities[rulers].variety;
            let lexicon = &world.varieties[v].lexicon;
            for l in lexicon.living() {
                let Origin::Borrowed { from, source } = l.origin else {
                    continue;
                };
                if from != high {
                    continue;
                }
                learned += 1;
                let root = world.root_of(high, source);
                doublets += usize::from(
                    lexicon
                        .living()
                        .any(|n| n.form != l.form && world.root_of(v, n.id) == root),
                );
            }
        }
        assert!(
            learned > 0 && doublets > 0,
            "{learned} learned loans, {doublets} doublets"
        );
    }

    #[test]
    fn a_falling_state_leaves_its_written_standard_classical() {
        let (mut world, s, _, _) = realm(1, false);
        world.fall(s, Fall::Collapsed);
        assert_eq!(world.states[s].classical, None, "too young a tradition");

        let (mut world, s, rulers, subjects) = realm(1, false);
        world.run(FALL_TRADITION);
        world.fall(s, Fall::Collapsed);
        let classical = world.states[s].classical.expect("fixed at the fall");
        assert_eq!(classical.how, Fixing::Fall);
        for c in [rulers, subjects] {
            assert!(world.diglossic(world.communities[c].variety));
        }
    }

    #[test]
    fn a_new_standard_or_a_translated_scripture_ends_diglossia() {
        let (mut world, s, rulers, subjects) = realm(2, true);
        let high = world.states[s].classical.unwrap().variety;
        let own = world.raise_state(subjects, None, Rise::Proclaimed);
        world.adopt_standard(own);
        let v = world.communities[subjects].variety;
        assert!(!world.diglossic(v));
        assert_eq!(world.varieties[v].written, Some(world.generation));
        assert_eq!(world.varieties[v].high, Some(high), "it still lends");
        let lending = |w: &World, c: usize| {
            w.classical_sources()
                .into_iter()
                .find(|&(r, h, _, _)| (r, h) == (c, high))
                .map(|(_, _, i, _)| i)
        };
        assert!(lending(&world, subjects) < lending(&world, rulers));

        let r = world.found_religion(subjects, crate::ideas::Revelation::Proclaimed);
        world.religions[r].translates = true;
        world.religions[r].scripture = true;
        world.convert(rulers, r, None);
        assert!(!world.diglossic(world.communities[rulers].variety));
    }

    #[test]
    fn cultural_classical_forks_snapshot_local_names_not_later_discoveries() {
        let (mut world, state, rulers, _) = realm(13, false);
        let parent = world.communities[rulers].variety;
        let home = world.communities[rulers].home();
        let local = world.known_place(parent, home).unwrap().clone();
        world.fix(state, Fixing::Age);
        let high = world.states[state].classical.unwrap().variety;
        assert_eq!(world.known_place(high, home), Some(&local));
        let unknown = (0..world.map.regions.len())
            .find(|&r| {
                world.map.regions[r].terrain.is_land()
                    && world.known_place(parent, r).is_none()
                    && world.known_place(high, r).is_none()
            })
            .unwrap();
        world.varieties[parent].exonyms.push((unknown, local));
        assert!(world.known_place(high, unknown).is_none());
    }
}

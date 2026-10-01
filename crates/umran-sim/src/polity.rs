//! States: a ruling people, the peoples it rules, a capital whose city
//! the state's tribute feeds, and in time a standard language. States
//! arise by conquest, or when a large farming people under pressure
//! organizes itself, as Toynbee saw Egypt answer the drying of the Sahara.
//! A standard pulls the state's kindred speech toward it (dialect
//! levelling), draws its subjects of other families to shift to it, and
//! itself changes slowly; when the state falls, the pull stops, and its
//! dialects drift apart again, as Latin did into the Romance languages.
//!
//! Rule is carried by contacts: every subject has a rule contact with its
//! rulers, so borrowing, waves, and shift work through it as through any
//! contact. When that contact ends, the subject has thrown off the rule.

use crate::diglossia::{Classical, Vernacular};
use crate::ideas::WRITTEN_PACE;
use crate::names::{MAX_PEOPLE_NAME, Name, clipped};
use crate::rng::{key, stream};
use crate::world::{ContactKind, World, WorldEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Share of its subjects' number a state's tribute feeds at its capital.
const TRIBUTE: f32 = 0.1;
/// Share of what their own lands feed them that ruling adds for the
/// rulers: stores, works, and order.
const COURT: f32 = 0.1;
/// Most of the rulers that live in the city, however great the tribute.
const CITY_MOST: f32 = 0.5;
/// Generations a state must stand, and the city it must have, before its
/// court speech may become its standard, and the chance per generation
/// that it then does.
const STANDARD_AGE: u32 = 8;
const STANDARD_CITY: f32 = 10000.0;
const STANDARD_CHANCE: f32 = 0.2;
/// Smallest people that may organize itself into a state.
const STATE_SIZE: f32 = 30000.0;
/// Pressure counted for a people that has met no challenge lately:
/// comfort alone rarely makes a state.
const COMFORT: f32 = 0.1;
/// Generations a challenge is felt for.
pub(crate) const CHALLENGE_SPAN: u32 = 6;
/// How much likelier a state is to collapse after bad times struck its
/// rulers' lands.
const HARD_COLLAPSE: f32 = 3.0;
/// Intensity of rule over a people split off within a state.
pub(crate) const RULE_INTENSITY: f32 = 0.6;
/// How much more slowly a standard takes up sound laws and new words.
pub(crate) const STANDARD_PACE: f32 = 0.5;
/// How much more readily a standard's sound changes and words reach the
/// kindred speech of its subjects (dialect levelling), and the least
/// borrowability a word has when levelled, so even basic words give way.
pub(crate) const LEVELLING: f32 = 3.0;
pub(crate) const LEVEL_FLOOR: f32 = 0.5;
/// How much readier subjects of another family are to shift to a
/// standard than to mere rulers' speech.
pub(crate) const STANDARD_SHIFT: f32 = 2.0;
/// Prestige a standard's words carry beyond its speakers' standing.
pub(crate) const STANDARD_PRESTIGE: f32 = 0.5;
/// How much longer rule holds under a standard, and how much shorter
/// after bad times struck the rulers' lands.
pub(crate) const STANDARD_HOLD: f32 = 1.5;
pub(crate) const HARD_HOLD: f32 = 0.5;
/// For a wholly purist standard: the usage disadvantage of its loans,
/// and the extra innovation hazard of a concept held by a loan.
pub(crate) const PURIST_COST: f32 = 0.5;
pub(crate) const PURIST_PRESSURE: f32 = 6.0;

/// A state: one people ruling itself and others.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// Coined in the rulers' language, after the rulers ("the realm of the
    /// Ivo"), and changing with it while the state stands.
    pub name: Name,
    /// Its first ruler, as his name was said then; one of the names in
    /// fashion among the rulers.
    pub founder: Name,
    pub rulers: usize,
    /// Every people it has ruled, in the order they came under it.
    pub members: Vec<Member>,
    /// The land of its court and city: the rulers' heart when it arose.
    pub capital: usize,
    pub rose: u32,
    pub how: Rise,
    /// When and how it fell, if it has.
    pub fell: Option<(u32, Fall)>,
    /// When its court speech became its standard, if it has.
    pub standard: Option<u32>,
    /// 0–1: how closely its standard is guarded against foreign words.
    pub purism: f32,
    /// Its standard frozen as a classical form, once fixed.
    pub classical: Option<Classical>,
}

/// A people under a state's rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Member {
    pub community: usize,
    pub joined: u32,
    /// When it threw off the rule, came to an end, or the state fell.
    pub left: Option<u32>,
}

impl State {
    pub fn standing(&self) -> bool {
        self.fell.is_none()
    }

    /// The peoples it rules now, besides the rulers.
    pub fn subjects(&self) -> impl Iterator<Item = usize> + '_ {
        self.members
            .iter()
            .filter(|m| m.left.is_none())
            .map(|m| m.community)
    }
}

/// How a state arose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rise {
    /// Its rulers conquered a people they dealt with.
    Conquest,
    /// A large farming people organized itself in answer to a challenge.
    Challenge(Challenge),
    /// By an author's hand.
    Proclaimed,
}

/// What pressed a people to organize itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Challenge {
    /// Famine, plague, or drought on its lands.
    HardTimes,
    /// Being crowded off land it held.
    Crowded,
    /// A stronger state on its borders.
    Neighbour,
    /// None lately: it grew into a state in comfort.
    Comfort,
}

/// How a state fell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum Fall {
    /// Its rulers came to an end.
    RulersEnded,
    /// Its rulers no longer hold its capital.
    CapitalLost,
    /// Another people conquered its rulers and took its subjects.
    Conquered { by: usize },
    /// It broke down of itself, likelier after bad times on its rulers'
    /// lands.
    Collapsed,
}

impl World {
    /// The standing state `community` rules, if any.
    pub fn rules(&self, community: usize) -> Option<usize> {
        self.states
            .iter()
            .position(|s| s.standing() && s.rulers == community)
    }

    /// The standing state `community` belongs to, as ruler or subject.
    pub fn state_of(&self, community: usize) -> Option<usize> {
        self.states.iter().position(|s| {
            s.standing() && (s.rulers == community || s.subjects().any(|m| m == community))
        })
    }

    /// The standing state ruling `community`, if it is a subject.
    pub fn ruled_by(&self, community: usize) -> Option<usize> {
        self.states
            .iter()
            .position(|s| s.standing() && s.subjects().any(|m| m == community))
    }

    /// For each variety, the standing state whose standard it is.
    pub fn standards(&self) -> Vec<Option<usize>> {
        let mut out = vec![None; self.varieties.len()];
        for (i, s) in self.states.iter().enumerate() {
            if s.standing() && s.standard.is_some() {
                out[self.communities[s.rulers].variety] = Some(i);
            }
        }
        out
    }

    /// What ruling adds to what the rulers of a state are fed: their
    /// court's share of their own lands, and their subjects' tribute.
    /// Zero for a people that rules no state.
    pub(crate) fn tribute(&self, community: usize) -> f32 {
        let Some(s) = self.rules(community) else {
            return 0.0;
        };
        let k = &self.communities[community];
        let own: f32 = k.lands.iter().map(|&r| self.feeds(r, k.livelihood)).sum();
        let subjects: f32 = self.states[s]
            .subjects()
            .map(|m| self.communities[m].size)
            .sum();
        COURT * own + TRIBUTE * subjects
    }

    /// How many live in a standing state's city: the rulers its tribute
    /// feeds, gathered at the capital.
    pub fn city(&self, state: usize) -> f32 {
        let s = &self.states[state];
        if !s.standing() {
            return 0.0;
        }
        let rulers = self.communities[s.rulers].size;
        self.tribute(s.rulers).min(rulers * CITY_MOST)
    }

    /// Whether `subject` is ruled by `rulers`.
    pub(crate) fn rules_over(&self, rulers: usize, subject: usize) -> bool {
        self.ruled_by(subject)
            .is_some_and(|s| self.states[s].rulers == rulers)
    }

    /// Whether `subject` is ruled by `rulers` and their speech is the
    /// state's standard.
    pub(crate) fn under_standard(&self, subject: usize, rulers: usize) -> bool {
        self.ruled_by(subject)
            .is_some_and(|s| self.states[s].rulers == rulers && self.states[s].standard.is_some())
    }

    /// How fast variety `v` takes up sound laws and new words: slower if
    /// it is a standing state's standard, and slower still if written; at
    /// the ordinary pace again once its speakers write a classical form
    /// instead, since the brake is then on the classical form.
    pub(crate) fn pace(&self, v: usize) -> f32 {
        match self.standard_state(v) {
            Some(_) if self.diglossic(v) => 1.0,
            Some(_) if self.varieties[v].written.is_some() => STANDARD_PACE * WRITTEN_PACE,
            Some(_) => STANDARD_PACE,
            None => 1.0,
        }
    }

    /// How closely variety `v` is guarded against foreign words: its
    /// state's purism if it is a standard, else none.
    pub(crate) fn purism(&self, v: usize) -> f32 {
        self.standard_state(v)
            .map_or(0.0, |s| self.states[s].purism)
    }

    fn standard_state(&self, v: usize) -> Option<usize> {
        self.states.iter().position(|s| {
            s.standing() && s.standard.is_some() && self.communities[s.rulers].variety == v
        })
    }

    /// How much longer than plain rule the rule between `a` and `b`
    /// holds: longer under a standard, shorter after bad times struck the
    /// rulers' lands (`struck`).
    pub(crate) fn rule_hold(&self, a: usize, b: usize, struck: &HashSet<usize>) -> f32 {
        let rulers = if self.rules_over(a, b) {
            a
        } else if self.rules_over(b, a) {
            b
        } else {
            return 1.0;
        };
        let s = self
            .rules(rulers)
            .expect("rulers of a subject rule a state");
        let standard = if self.states[s].standard.is_some() {
            STANDARD_HOLD
        } else {
            1.0
        };
        let hard = if self.communities[rulers]
            .lands
            .iter()
            .any(|r| struck.contains(r))
        {
            HARD_HOLD
        } else {
            1.0
        };
        standard * hard
    }

    /// `rulers` raise a state with its court at `capital`, or their heart
    /// land; returns its index.
    pub fn raise_state(&mut self, rulers: usize, capital: Option<usize>, how: Rise) -> usize {
        let index = self.states.len();
        let mut rng = stream(self.seed, &[key("state"), index as u64]);
        // Most standards take words freely; a few guard against them.
        let purism = rng.r#gen::<f32>().powi(2);
        let stock = &self.varieties[self.communities[rulers].variety].given;
        let founder = match stock.len() {
            0 => self.communities[rulers].name.clone(),
            n => stock[crate::rng::index(&mut rng, n)].name.clone(),
        };
        self.states.push(State {
            name: self.state_name(rulers),
            founder: Name {
                coined: self.generation,
                log: Vec::new(),
                ..founder
            },
            rulers,
            members: Vec::new(),
            capital: capital.unwrap_or_else(|| self.communities[rulers].home()),
            rose: self.generation,
            how,
            fell: None,
            standard: None,
            purism,
            classical: None,
        });
        self.events
            .push((self.generation, WorldEvent::Rose { state: index }));
        index
    }

    /// The name of a state its rulers raise: theirs with the belonging
    /// affix, as Francia was the Franks', or compounded with their word
    /// for a chief when that would sound like their language's name.
    fn state_name(&self, rulers: usize) -> Name {
        let c = &self.communities[rulers];
        let variety = &self.varieties[c.variety];
        let morphology = &variety.morphology;
        let mut form = morphology.belonging(&c.name.form);
        if form == variety.name.form
            && let Some(chief) = crate::concepts::by_id("chief")
                .and_then(|concept| variety.lexicon.word_for(concept))
        {
            form = morphology.compound(&c.name.form, &chief.form);
        }
        Name {
            form: clipped(form, MAX_PEOPLE_NAME + 1),
            meaning: format!("the realm of the {}", self.community_name(rulers)),
            coined: self.generation,
            log: Vec::new(),
        }
    }

    /// `rulers` come to rule `ruled`, at `intensity`: the ruled join the
    /// rulers' state, which they raise if they have none. Conquered rulers
    /// bring their whole state, which falls, its subjects passing to the
    /// conquerors. Rulers who were themselves subjects break away first.
    pub(crate) fn subject(&mut self, rulers: usize, ruled: usize, intensity: f32) {
        if let Some(s) = self.ruled_by(rulers) {
            self.leave(s, rulers);
        }
        if let Some(s) = self.ruled_by(ruled) {
            self.leave(s, ruled);
        }
        let state = match self.rules(rulers) {
            Some(s) => s,
            None => self.raise_state(rulers, None, Rise::Conquest),
        };
        let taken: Vec<usize> = match self.rules(ruled) {
            Some(old) => {
                let subjects: Vec<usize> = self.states[old].subjects().collect();
                self.fall(old, Fall::Conquered { by: rulers });
                subjects
            }
            None => Vec::new(),
        };
        for c in std::iter::once(ruled).chain(taken) {
            if c == rulers {
                continue;
            }
            self.link(rulers, c, intensity, ContactKind::Rule);
            self.states[state].members.push(Member {
                community: c,
                joined: self.generation,
                left: None,
            });
        }
    }

    /// A people split off from `parent` stays in its parent's state, under
    /// the same rulers.
    pub(crate) fn inherit_state(&mut self, parent: usize, daughter: usize) {
        let Some(s) = self.state_of(parent) else {
            return;
        };
        let rulers = self.states[s].rulers;
        self.link(rulers, daughter, RULE_INTENSITY, ContactKind::Rule);
        self.states[s].members.push(Member {
            community: daughter,
            joined: self.generation,
            left: None,
        });
    }

    /// `community` is no longer under state `s`; its rule contact, if any,
    /// leaves them neighbours, as the end of any rule does.
    fn leave(&mut self, s: usize, community: usize) {
        let generation = self.generation;
        let rulers = self.states[s].rulers;
        for m in &mut self.states[s].members {
            if m.community == community && m.left.is_none() {
                m.left = Some(generation);
            }
        }
        if let Some(i) = self.contacts.iter().position(|k| {
            k.kind == ContactKind::Rule
                && ((k.a, k.b) == (rulers, community) || (k.a, k.b) == (community, rulers))
        }) {
            let intensity = self.contacts[i].intensity;
            self.contacts.remove(i);
            if self.communities[community].living() && self.communities[rulers].living() {
                self.link(rulers, community, intensity / 2.0, ContactKind::Neighbours);
            }
        }
    }

    /// State `s` falls: its subjects are freed, and its written standard
    /// is left behind as a classical form.
    pub(crate) fn fall(&mut self, s: usize, how: Fall) {
        let subjects: Vec<usize> = self.states[s].subjects().collect();
        for c in subjects {
            self.leave(s, c);
        }
        self.states[s].fell = Some((self.generation, how));
        self.events
            .push((self.generation, WorldEvent::Fell { state: s }));
        self.fix_at_fall(s);
    }

    /// Keeps states true to the peoples they hold: a subject that came to
    /// an end or whose rule contact ended has left; a state whose rulers
    /// ended, or no longer hold its capital, falls. Any state may also
    /// collapse (`collapse_rate`), likelier after bad times struck its
    /// rulers' lands.
    pub(crate) fn hold_states(&mut self) {
        let (struck, _) = self.recent_challenges();
        for s in 0..self.states.len() {
            if !self.states[s].standing() {
                continue;
            }
            let rulers = self.states[s].rulers;
            if !self.communities[rulers].living() {
                self.fall(s, Fall::RulersEnded);
                continue;
            }
            if !self.communities[rulers]
                .lands
                .contains(&self.states[s].capital)
            {
                self.fall(s, Fall::CapitalLost);
                continue;
            }
            let mut hazard = self.params.collapse_rate;
            if self.communities[rulers]
                .lands
                .iter()
                .any(|r| struck.contains(r))
            {
                hazard *= HARD_COLLAPSE;
            }
            let mut rng = stream(
                self.seed,
                &[
                    key("step"),
                    u64::from(self.generation),
                    key("collapse"),
                    s as u64,
                ],
            );
            if rng.r#gen::<f32>() < hazard {
                self.fall(s, Fall::Collapsed);
                continue;
            }
            let gone: Vec<usize> = self.states[s]
                .subjects()
                .filter(|&m| {
                    !self.communities[m].living()
                        || !self.contacts.iter().any(|k| {
                            k.kind == ContactKind::Rule
                                && ((k.a, k.b) == (rulers, m) || (k.a, k.b) == (m, rulers))
                        })
                })
                .collect();
            for m in gone {
                self.leave(s, m);
            }
        }
    }

    /// Lands struck by bad times, and peoples crowded off land, in the
    /// last `CHALLENGE_SPAN` generations.
    pub(crate) fn recent_challenges(&self) -> (HashSet<usize>, HashSet<usize>) {
        let since = self.generation.saturating_sub(CHALLENGE_SPAN);
        let (mut struck, mut crowded) = (HashSet::new(), HashSet::new());
        for (g, event) in self.events.iter().rev() {
            if *g < since {
                break;
            }
            match *event {
                WorldEvent::HardTimes { region, .. } => {
                    struck.insert(region);
                }
                WorldEvent::Displaced { community, .. } => {
                    crowded.insert(community);
                }
                _ => {}
            }
        }
        (struck, crowded)
    }

    /// Large farming peoples under no state may organize themselves into
    /// one: rarely in comfort, more readily in answer to bad times, being
    /// crowded off land, or a stronger state beside them.
    pub(crate) fn rise_states(&mut self) {
        if self.params.state_rate <= 0.0 {
            return;
        }
        let (struck, crowded) = self.recent_challenges();
        for c in self.living().collect::<Vec<_>>() {
            let k = &self.communities[c];
            if k.livelihood != crate::Livelihood::Farming
                || k.size < STATE_SIZE
                || self.state_of(c).is_some()
            {
                continue;
            }
            let neighbour = self.contacts.iter().any(|contact| {
                let other = match (contact.a == c, contact.b == c) {
                    (true, _) => contact.b,
                    (_, true) => contact.a,
                    _ => return false,
                };
                self.state_of(other).is_some()
                    && self.communities[other].prestige > self.communities[c].prestige
            });
            let challenge = if k.lands.iter().any(|r| struck.contains(r)) {
                Challenge::HardTimes
            } else if crowded.contains(&c) {
                Challenge::Crowded
            } else if neighbour {
                Challenge::Neighbour
            } else {
                Challenge::Comfort
            };
            let pressure = match challenge {
                Challenge::Comfort => COMFORT,
                _ => 1.0,
            };
            let mut rng = self.community_rng(c, "rise");
            if rng.r#gen::<f32>() < self.params.state_rate * pressure {
                self.raise_state(c, None, Rise::Challenge(challenge));
            }
        }
    }

    /// A state that has stood long enough, with a city large enough, may
    /// take its court speech as its standard.
    pub(crate) fn standardize(&mut self) {
        for s in 0..self.states.len() {
            let state = &self.states[s];
            if !state.standing()
                || state.standard.is_some()
                || self.generation < state.rose + STANDARD_AGE
                || self.city(s) < STANDARD_CITY
            {
                continue;
            }
            let mut rng = stream(
                self.seed,
                &[
                    key("step"),
                    u64::from(self.generation),
                    key("standard"),
                    s as u64,
                ],
            );
            if rng.r#gen::<f32>() < STANDARD_CHANCE {
                self.adopt_standard(s);
            }
        }
    }

    /// State `s` takes its court speech as its standard. If its speakers
    /// wrote a classical form, they now write their own speech.
    pub(crate) fn adopt_standard(&mut self, s: usize) {
        self.states[s].standard = Some(self.generation);
        self.events
            .push((self.generation, WorldEvent::Standard { state: s }));
        let v = self.communities[self.states[s].rulers].variety;
        self.write_vernacular(v, Vernacular::Standard { state: s });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concepts::CONCEPTS;
    use crate::profile::SoundProfile;
    use crate::world::Params;

    /// A people and a dialect split from it, the first ruling the second,
    /// with sound changes spreading between them; the realm has a
    /// standard if `standard`.
    fn kingdom(seed: u64, standard: bool, generations: u32) -> (World, usize, usize) {
        let params = Params {
            wave_rate: 1.0,
            ..Params::static_society()
        };
        let mut world = World::new(seed, params);
        let rulers = world.found(&SoundProfile::base(), 0.9, 0.5);
        let subjects = world.split(rulers, None, 0.0);
        world.connect(rulers, subjects, 0.8, ContactKind::Rule);
        let s = world.rules(rulers).expect("rule raises a state");
        if standard {
            world.states[s].standard = Some(0);
        }
        world.run(generations);
        (world, rulers, subjects)
    }

    /// Share of concepts the two peoples say alike.
    fn alike(world: &World, a: usize, b: usize) -> f32 {
        let (va, vb) = (world.variety_of(a), world.variety_of(b));
        let same = (0..CONCEPTS.len())
            .filter(|&i| {
                match (
                    va.lexicon.slots[i].dominant(),
                    vb.lexicon.slots[i].dominant(),
                ) {
                    (Some(x), Some(y)) => va.lexicon.get(x).form == vb.lexicon.get(y).form,
                    _ => false,
                }
            })
            .count();
        same as f32 / CONCEPTS.len() as f32
    }

    #[test]
    fn conquered_rulers_bring_their_subjects() {
        let mut world = World::new(3, Params::static_society());
        let profile = SoundProfile::base();
        let (a, b, c) = (
            world.found(&profile, 0.9, 0.5),
            world.found(&profile, 0.6, 0.5),
            world.found(&profile, 0.3, 0.5),
        );
        world.connect(b, c, 0.8, ContactKind::Rule);
        let old = world.rules(b).expect("b rules c");
        world.connect(a, b, 0.8, ContactKind::Rule);
        let new = world.rules(a).expect("a rules b");
        assert_eq!(
            world.states[old].fell.map(|(_, f)| f),
            Some(Fall::Conquered { by: a })
        );
        assert_eq!(world.states[new].subjects().collect::<Vec<_>>(), vec![b, c]);
        assert!(world.rules_over(a, c));
        world.run(5);
        assert_eq!(world.state_of(c), Some(new), "the rule holds");
    }

    #[test]
    fn a_standard_levels_its_kindred_dialects() {
        let (mut levelled, mut apart) = (0.0, 0.0);
        for seed in 0..12 {
            let (w, a, b) = kingdom(seed, true, 60);
            levelled += alike(&w, a, b);
            let (w, a, b) = kingdom(seed, false, 60);
            apart += alike(&w, a, b);
        }
        assert!(
            levelled > 1.15 * apart,
            "levelled {levelled:.2}, apart {apart:.2}"
        );
    }

    #[test]
    fn a_standard_changes_slowly() {
        let (mut standard, mut plain) = (0, 0);
        for seed in 0..12 {
            let (w, a, _) = kingdom(seed, true, 60);
            standard += w.variety_of(a).laws.len();
            let (w, a, _) = kingdom(seed, false, 60);
            plain += w.variety_of(a).laws.len();
        }
        assert!(
            (standard as f32) < 0.8 * plain as f32,
            "standard {standard}, plain {plain}"
        );
    }
}

//! Heritable worldview, acculturation, and bounded challenge responses.
use crate::livelihood::Livelihood;
use crate::rng::{key, stream};
use crate::world::{Community, ContactKind, World, WorldEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Magnitude at which an axis enters a notable pole.
const STRONG: f32 = 0.5;
/// Minimum magnitude that retains a previously recorded pole.
const SETTLED: f32 = 0.35;
/// Piety's rise per hard year, as a share of the room left toward +1.
const HARDSHIP_PIETY: f32 = 0.08;
/// Roving's rise per hardship on full lands, as a share of the room toward
/// +1, and its fall per hardship with room to spare, as a share of the room
/// toward -1.
const HARDSHIP_ROVING: f32 = 0.06;
const HARDSHIP_ROOTING: f32 = 0.04;
/// Piety's fall per fed, untroubled generation, as a share of what lies
/// above neutral.
const EASE: f32 = 0.15;
/// Roving's fall per generation settled in place, as a share of what lies
/// above the way of life's founding prior.
const SETTLING: f32 = 0.06;
/// Martial's fall per generation after `QUIET_SPAN` without a challenge,
/// as a share of the room toward -1.
const COMFORT: f32 = 0.003;
const QUIET_SPAN: u32 = 12;
/// Hierarchical's rise per generation of ruling a subject past
/// `QUIET_SPAN`, as a share of the room toward +1.
const LONG_RULE: f32 = 0.004;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Axis {
    Martial,
    Open,
    Pious,
    Hierarchical,
    Roving,
    Seaward,
}

impl Axis {
    pub const ALL: [Self; 6] = [
        Self::Martial,
        Self::Open,
        Self::Pious,
        Self::Hierarchical,
        Self::Roving,
        Self::Seaward,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Martial => "martial",
            Self::Open => "open",
            Self::Pious => "pious",
            Self::Hierarchical => "hierarchical",
            Self::Roving => "roving",
            Self::Seaward => "seaward",
        }
    }
}

/// Zero is neutral. Positive and negative values are equally valid biases.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ethos {
    pub martial: f32,
    pub open: f32,
    pub pious: f32,
    pub hierarchical: f32,
    pub roving: f32,
    pub seaward: f32,
}

/// Absent axes retain their independent founding draw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundingEthos {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub martial: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pious: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchical: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roving: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seaward: Option<f32>,
}

impl FoundingEthos {
    fn get(self, axis: Axis) -> Option<f32> {
        match axis {
            Axis::Martial => self.martial,
            Axis::Open => self.open,
            Axis::Pious => self.pious,
            Axis::Hierarchical => self.hierarchical,
            Axis::Roving => self.roving,
            Axis::Seaward => self.seaward,
        }
    }

    pub fn validate(self) -> Result<(), String> {
        for axis in Axis::ALL {
            if let Some(value) = self.get(axis)
                && (!value.is_finite() || !(-1.0..=1.0).contains(&value))
            {
                return Err(format!("ethos {} must be between -1 and 1", axis.id()));
            }
        }
        Ok(())
    }
}

/// A single multiplier at each use, never a compounded feedback formula.
#[derive(Clone, Copy, Debug)]
pub enum Effect {
    Conquest,
    Contact,
    Borrowing,
    Purism,
    Conversion,
    Faith,
    Schism,
    Pilgrimage,
    State,
    Standard,
    Migration,
    Spread,
    Cohesion,
    Seafaring,
    Colony,
}

impl Effect {
    pub const ALL: [Self; 15] = [
        Self::Conquest,
        Self::Contact,
        Self::Borrowing,
        Self::Purism,
        Self::Conversion,
        Self::Faith,
        Self::Schism,
        Self::Pilgrimage,
        Self::State,
        Self::Standard,
        Self::Migration,
        Self::Spread,
        Self::Cohesion,
        Self::Seafaring,
        Self::Colony,
    ];

    pub fn axis(self) -> Axis {
        match self {
            Self::Conquest => Axis::Martial,
            Self::Contact | Self::Borrowing | Self::Purism => Axis::Open,
            Self::Conversion | Self::Faith | Self::Schism | Self::Pilgrimage => Axis::Pious,
            Self::State | Self::Standard => Axis::Hierarchical,
            Self::Migration | Self::Spread | Self::Cohesion => Axis::Roving,
            Self::Seafaring | Self::Colony => Axis::Seaward,
        }
    }

    pub fn k(self) -> f32 {
        match self {
            Self::Purism => -0.5,
            Self::Cohesion => 0.35,
            Self::Seafaring | Self::Colony => 0.75,
            _ => 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemperCause {
    Drift,
    Inheritance,
    Contact,
    Hardship,
    Freed,
    LongRule,
    Comfort,
    Ease,
    Settled,
    Seafaring,
    Faith,
    Fate,
}

impl TemperCause {
    pub fn id(self) -> &'static str {
        match self {
            Self::Drift => "drift",
            Self::Inheritance => "inheritance",
            Self::Contact => "contact",
            Self::Hardship => "hardship",
            Self::Freed => "freed",
            Self::LongRule => "long-rule",
            Self::Comfort => "comfort",
            Self::Ease => "ease",
            Self::Settled => "settled",
            Self::Seafaring => "seafaring",
            Self::Faith => "faith",
            Self::Fate => "fate",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Pole {
    High,
    Low,
}

impl Pole {
    pub fn id(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Low => "low",
        }
    }
}

impl Ethos {
    pub fn get(self, axis: Axis) -> f32 {
        match axis {
            Axis::Martial => self.martial,
            Axis::Open => self.open,
            Axis::Pious => self.pious,
            Axis::Hierarchical => self.hierarchical,
            Axis::Roving => self.roving,
            Axis::Seaward => self.seaward,
        }
    }

    fn set(&mut self, axis: Axis, value: f32) {
        *match axis {
            Axis::Martial => &mut self.martial,
            Axis::Open => &mut self.open,
            Axis::Pious => &mut self.pious,
            Axis::Hierarchical => &mut self.hierarchical,
            Axis::Roving => &mut self.roving,
            Axis::Seaward => &mut self.seaward,
        } = value.clamp(-1.0, 1.0);
    }

    /// Initial notable poles, without announcing the founding worldview.
    pub(crate) fn temper_marks(self) -> [i8; 6] {
        Axis::ALL.map(|axis| {
            let value = self.get(axis);
            if value >= STRONG {
                1
            } else if value <= -STRONG {
                -1
            } else {
                0
            }
        })
    }

    pub fn factor(self, effect: Effect) -> f32 {
        multiplier(self.get(effect.axis()), effect.k())
    }

    pub(crate) fn name_weight(self, meaning: &str) -> f32 {
        let axis = match meaning {
            "spear" | "shield" | "war" | "fight" | "bow" => Axis::Martial,
            "god" => Axis::Pious,
            "chief" => Axis::Hierarchical,
            "sea" | "fish" => Axis::Seaward,
            "horse" | "bird" => Axis::Roving,
            "friend" | "people" => Axis::Open,
            _ => return 1.0,
        };
        multiplier(self.get(axis), 0.75)
    }

    pub(crate) fn mean(values: impl Iterator<Item = (Self, f32)>) -> Self {
        let mut total = 0.0;
        let mut sums = [0.0; 6];
        for (ethos, weight) in values {
            total += weight;
            for (i, axis) in Axis::ALL.into_iter().enumerate() {
                sums[i] += ethos.get(axis) * weight;
            }
        }
        let mut out = Self::default();
        if total > 0.0 {
            for (i, axis) in Axis::ALL.into_iter().enumerate() {
                out.set(axis, sums[i] / total);
            }
        }
        out
    }
}

pub fn multiplier(axis: f32, k: f32) -> f32 {
    (1.0 + k * axis).clamp(0.25, 2.0)
}

impl Community {
    /// The end-of-generation worldview, frozen after this people ends.
    pub fn ethos_at(&self, generation: u32) -> Ethos {
        let end = self
            .ethos_history
            .partition_point(|(g, _)| *g <= generation);
        self.ethos_history
            .get(end.saturating_sub(1))
            .map_or(self.ethos, |(_, e)| *e)
    }
}

impl Ethos {
    /// The founding prior of a way of life and a homeland, before the
    /// founding draw's noise; the rest a worldview relaxes toward.
    pub(crate) fn prior(livelihood: Livelihood, coastal: bool) -> Self {
        let (martial, hierarchical, roving) = match livelihood {
            Livelihood::Foraging => (-0.1, -0.3, 0.15),
            Livelihood::Herding => (0.3, 0.0, 0.4),
            Livelihood::Farming => (0.0, 0.3, -0.35),
        };
        Self {
            martial,
            hierarchical,
            roving,
            seaward: if coastal { 0.4 } else { -0.25 },
            ..Self::default()
        }
    }
}

impl World {
    /// Recompute founding temper from independent purpose-keyed streams.
    /// Passing no authored axes recovers the draw without changing the world.
    pub fn founding_ethos(
        &self,
        community: usize,
        region: usize,
        livelihood: Livelihood,
        authored: Option<&FoundingEthos>,
    ) -> Ethos {
        if !self.params.ethos_enabled {
            return Ethos::default();
        }
        let mut ethos = Ethos::prior(livelihood, self.map.coastal(region));
        for axis in Axis::ALL {
            let mut rng = stream(self.seed, &[key("ethos"), community as u64, key(axis.id())]);
            let drawn = ethos.get(axis) + rng.gen_range(-0.3..0.3);
            ethos.set(axis, authored.and_then(|e| e.get(axis)).unwrap_or(drawn));
        }
        ethos
    }

    /// Authorial nudges are validated before any mutation, and never override.
    pub fn temper(&mut self, community: usize, axis: Axis, amount: f32) -> Result<(), String> {
        match self.communities.get(community) {
            None => return Err(format!("there is no community {community}")),
            Some(c) if !c.living() => {
                return Err(format!(
                    "the {} are no more",
                    self.community_name(community)
                ));
            }
            _ => {}
        }
        if !amount.is_finite() || !(-1.0..=1.0).contains(&amount) {
            return Err("temper amount must be between -1 and 1".into());
        }
        self.nudge_ethos(community, axis, amount, TemperCause::Fate);
        Ok(())
    }

    pub(crate) fn nudge_ethos(
        &mut self,
        community: usize,
        axis: Axis,
        amount: f32,
        cause: TemperCause,
    ) {
        if !self.params.ethos_enabled || !self.params.ethos_shifts {
            return;
        }
        let c = &mut self.communities[community];
        let before = c.ethos.get(axis);
        c.ethos.set(axis, before + amount);
        let after = c.ethos.get(axis);
        if before == after {
            return;
        }
        match c.ethos_history.last_mut() {
            Some((g, value)) if *g == self.generation => *value = c.ethos,
            _ => c.ethos_history.push((self.generation, c.ethos)),
        }
        let mark = c.temper_marks[axis as usize];
        let next = if after >= STRONG {
            1
        } else if after <= -STRONG {
            -1
        } else if mark == 1 && after >= SETTLED {
            1
        } else if mark == -1 && after <= -SETTLED {
            -1
        } else {
            0
        };
        if next != mark {
            for (value, entered) in [(mark, false), (next, true)] {
                if value == 0 {
                    continue;
                }
                self.events.push((
                    self.generation,
                    WorldEvent::Temper {
                        community,
                        axis,
                        pole: if value == 1 { Pole::High } else { Pole::Low },
                        entered,
                        cause,
                    },
                ));
            }
            c.temper_marks[axis as usize] = next;
        }
    }

    pub(crate) fn inherit_ethos(&mut self, daughter: usize, moving: bool) {
        let c = &mut self.communities[daughter];
        c.ethos_history.clear();
        c.ethos_history.push((self.generation, c.ethos));
        if !self.params.ethos_enabled || !self.params.ethos_shifts {
            return;
        }
        for axis in Axis::ALL {
            let mut rng = stream(
                self.seed,
                &[key("ethos"), key("split"), daughter as u64, key(axis.id())],
            );
            let drift = rng.gen_range(-0.04..0.04)
                + if axis == Axis::Roving && moving {
                    0.06
                } else {
                    0.0
                };
            self.nudge_ethos(daughter, axis, drift, TemperCause::Inheritance);
        }
    }

    /// A challenge response: `amount` of the room left toward the end it
    /// pushes to, so repeated pushes approach that end without reaching it.
    pub(crate) fn respond(
        &mut self,
        community: usize,
        axis: Axis,
        amount: f32,
        cause: TemperCause,
    ) {
        let value = self.communities[community].ethos.get(axis);
        let room = if amount >= 0.0 {
            1.0 - value
        } else {
            1.0 + value
        };
        self.nudge_ethos(community, axis, amount * room, cause);
    }

    /// Whether a response already moved this people's temper this
    /// generation. Before the end-of-generation drift, only a founding,
    /// a parting, or a response records a snapshot at the current
    /// generation, so a people's first year counts as answered.
    fn responded(&self, community: usize) -> bool {
        self.communities[community]
            .ethos_history
            .last()
            .is_some_and(|(g, _)| *g == self.generation)
    }

    /// One hard year is one challenge, however many of a people's lands
    /// it struck; the first hardship of a generation is the response.
    pub(crate) fn hardship_ethos(&mut self, community: usize, crowded: bool) {
        self.communities[community].ethos_challenged = self.generation;
        if self.responded(community) {
            return;
        }
        self.respond(
            community,
            Axis::Pious,
            HARDSHIP_PIETY,
            TemperCause::Hardship,
        );
        self.respond(
            community,
            Axis::Roving,
            if crowded {
                HARDSHIP_ROVING
            } else {
                -HARDSHIP_ROOTING
            },
            TemperCause::Hardship,
        );
    }

    pub(crate) fn freed_ethos(&mut self, community: usize) {
        self.communities[community].ethos_challenged = self.generation;
        self.nudge_ethos(community, Axis::Martial, 0.12, TemperCause::Freed);
    }

    /// One simultaneous contact update, closing at most 2% of each axis gap.
    pub(crate) fn acculturate(&mut self) {
        let mut pulls = vec![([0.0; 6], 0.0_f32); self.communities.len()];
        for contact in &self.contacts {
            let kind = match contact.kind {
                ContactKind::Intermarriage => 1.0,
                ContactKind::Rule => 0.9,
                ContactKind::Religion => 0.6,
                ContactKind::Neighbours => 0.45,
                ContactKind::Trade => 0.2,
            };
            for (recipient, source) in [(contact.a, contact.b), (contact.b, contact.a)] {
                let (a, b) = (&self.communities[recipient], &self.communities[source]);
                if !a.living() || !b.living() {
                    continue;
                }
                let w = contact.intensity * kind * (0.5 + b.prestige);
                pulls[recipient].1 += w;
                for (i, axis) in Axis::ALL.into_iter().enumerate() {
                    pulls[recipient].0[i] += w * (b.ethos.get(axis) - a.ethos.get(axis));
                }
            }
        }
        for (c, (sums, weight)) in pulls.into_iter().enumerate() {
            if weight == 0.0 {
                continue;
            }
            for (i, axis) in Axis::ALL.into_iter().enumerate() {
                self.nudge_ethos(
                    c,
                    axis,
                    0.02 * sums[i] / weight.max(1.0),
                    TemperCause::Contact,
                );
            }
        }
    }

    /// Peoples that left or lost a home land this generation.
    fn moved_this_generation(&self) -> Vec<bool> {
        let mut moved = vec![false; self.communities.len()];
        for (_, event) in self
            .events
            .iter()
            .rev()
            .take_while(|(g, _)| *g == self.generation)
        {
            if let WorldEvent::Migrated { community, .. }
            | WorldEvent::Displaced { community, .. } = event
            {
                moved[*community] = true;
            }
        }
        moved
    }

    /// Farming in place, or keeping a city: the life a people settles into.
    fn settled(&self, community: usize) -> bool {
        self.communities[community].livelihood == Livelihood::Farming
            || self
                .cities
                .iter()
                .any(|city| city.townsfolk == Some(community))
    }

    /// A generation of plenty: the people's lands feed it, and no
    /// response (hardship, liberation, faith, the sea, a parting) moved
    /// its temper, nor did a conqueror arrive. A land still short of its
    /// old yield does not count against a people its lands feed: the
    /// drought's challenge was answered when it struck.
    fn untroubled(&self, community: usize, occupied: &[f32]) -> bool {
        let (fed, crowd) = self.fed_and_crowd(community, occupied);
        crowd < fed
            && !self.responded(community)
            && !self.contacts.iter().any(|k| {
                k.kind == ContactKind::Rule && k.b == community && k.since == self.generation
            })
    }

    /// Drift, long rule, and the responses that run the other way once
    /// the challenge has passed. Being ruled is not a challenge of
    /// itself, so a long-ruled people grows comfortable like any other.
    pub(crate) fn temper_generation(&mut self) {
        if !self.params.ethos_enabled || !self.params.ethos_shifts {
            return;
        }
        let occupied = self.occupation();
        let untroubled: Vec<bool> = (0..self.communities.len())
            .map(|c| self.communities[c].living() && self.untroubled(c, &occupied))
            .collect();
        let moved = self.moved_this_generation();
        self.acculturate();
        for c in 0..self.communities.len() {
            if !self.communities[c].living() {
                continue;
            }
            for axis in Axis::ALL {
                let mut rng = stream(
                    self.seed,
                    &[
                        key("temper"),
                        self.generation as u64,
                        c as u64,
                        key(axis.id()),
                    ],
                );
                self.nudge_ethos(c, axis, rng.gen_range(-0.012..0.012), TemperCause::Drift);
            }
            let long_rule = self.rules(c).is_some_and(|s| {
                self.states[s]
                    .members
                    .iter()
                    .any(|m| m.left.is_none() && self.generation >= m.joined + QUIET_SPAN)
            });
            if long_rule {
                self.respond(c, Axis::Hierarchical, LONG_RULE, TemperCause::LongRule);
            }
            if untroubled[c] {
                let people = &self.communities[c];
                let (ethos, livelihood, home) = (people.ethos, people.livelihood, people.home());
                let rest = Ethos::prior(livelihood, self.map.coastal(home));
                if ethos.pious > rest.pious {
                    self.nudge_ethos(
                        c,
                        Axis::Pious,
                        -EASE * (ethos.pious - rest.pious),
                        TemperCause::Ease,
                    );
                }
                if !moved[c] && self.settled(c) && ethos.roving > rest.roving {
                    self.nudge_ethos(
                        c,
                        Axis::Roving,
                        -SETTLING * (ethos.roving - rest.roving),
                        TemperCause::Settled,
                    );
                }
            }
            if self.generation >= self.communities[c].ethos_challenged + QUIET_SPAN {
                self.respond(c, Axis::Martial, -COMFORT, TemperCause::Comfort);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Craft, Naming, Params, Revelation, SoundProfile};

    fn found(world: &mut World, ethos: FoundingEthos) -> usize {
        world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            world.communities.len() as u64,
            0.5,
            0.5,
            world.communities.first().map(|c| c.home()),
            None,
            Some(&ethos),
        )
    }

    fn all(value: f32) -> FoundingEthos {
        FoundingEthos {
            martial: Some(value),
            open: Some(value),
            pious: Some(value),
            hierarchical: Some(value),
            roving: Some(value),
            seaward: Some(value),
        }
    }

    #[test]
    fn multipliers_have_neutral_identity_and_bounded_direction() {
        for effect in Effect::ALL {
            assert_eq!(Ethos::default().factor(effect).to_bits(), 1.0_f32.to_bits());
            let mut ethos = Ethos::default();
            ethos.set(effect.axis(), 1.0);
            assert_eq!(ethos.factor(effect), 1.0 + effect.k());
            ethos.set(effect.axis(), -1.0);
            assert_eq!(ethos.factor(effect), 1.0 - effect.k());
        }
        assert_eq!(multiplier(-1.0, 5.0), 0.25);
        assert_eq!(multiplier(1.0, 5.0), 2.0);
    }

    #[test]
    fn intense_contact_converges_simultaneously_without_order_bias() {
        let mut world = World::new(
            17,
            Params {
                ethos_shifts: true,
                ..Params::static_society()
            },
        );
        found(&mut world, all(-0.8));
        found(&mut world, all(0.8));
        world
            .connect(0, 1, 1.0, ContactKind::Intermarriage)
            .unwrap();
        for _ in 0..120 {
            world.acculturate();
        }
        for axis in Axis::ALL {
            let a = world.communities[0].ethos.get(axis);
            let b = world.communities[1].ethos.get(axis);
            assert!(a < 0.0 && b > 0.0 && b - a < 0.02, "{axis:?}: {a} {b}");
            assert_eq!(a, -b);
        }
    }

    #[test]
    fn throwing_off_rule_hardens_the_ruled_not_the_rulers() {
        let mut world = World::new(
            3,
            Params {
                ethos_shifts: true,
                contact_turnover: 1000.0,
                ..Params::static_society()
            },
        );
        found(&mut world, all(0.4));
        found(&mut world, all(0.4));
        world.connect(0, 1, 0.8, ContactKind::Rule).unwrap();
        for _ in 0..24 {
            let before = world.communities[1].ethos.martial;
            let events = world.events.len();
            world.step();
            if world.events[events..].iter().any(|(_, e)| {
                matches!(
                    e,
                    WorldEvent::Parted {
                        b: 1,
                        kind: ContactKind::Rule,
                        ..
                    }
                )
            }) {
                assert!(world.communities[1].ethos.martial > before + 0.09);
                assert!(world.ruled_by(1).is_none());
                return;
            }
        }
        panic!("the subject never threw off rule");
    }

    #[test]
    fn static_society_freezes_ethos_through_contact_faith_crafts_and_splits() {
        let mut world = World::new(42, Params::static_society());
        found(&mut world, all(0.7));
        found(&mut world, all(-0.7));
        let original: Vec<_> = world.communities.iter().map(|c| c.ethos).collect();
        world
            .connect(0, 1, 1.0, ContactKind::Intermarriage)
            .unwrap();
        world.learn(0, Craft::Seafaring, None);
        let faith = world.found_religion(0, Revelation::Proclaimed);
        world.convert(1, faith, Some(0));
        world.temper(0, Axis::Martial, -1.0).unwrap();
        let daughter = world.split(0, None, 1.0);
        world.shift(1, 0);
        world.run(30);
        assert_eq!(world.communities[0].ethos, original[0]);
        assert_eq!(world.communities[1].ethos, original[1]);
        assert_eq!(world.communities[daughter].ethos, original[0]);
        assert!(
            !world
                .events
                .iter()
                .any(|(_, e)| matches!(e, WorldEvent::Temper { .. }))
        );
    }

    #[test]
    fn inheritance_drifts_but_language_shift_keeps_worldview() {
        let mut world = World::new(
            4,
            Params {
                ethos_shifts: true,
                ..Params::static_society()
            },
        );
        found(&mut world, all(0.0));
        found(&mut world, all(-0.7));
        let parent = world.communities[0].ethos;
        let daughter = world.split(0, None, 0.5);
        let split = world.communities[daughter].ethos;
        for axis in Axis::ALL {
            let difference = split.get(axis) - parent.get(axis);
            if axis == Axis::Roving
                && world.communities[daughter].home() != world.communities[0].home()
            {
                assert!((0.02..0.1).contains(&difference));
            } else {
                assert!(difference.abs() <= 0.04);
            }
        }
        world.shift(daughter, 1);
        assert_eq!(world.communities[daughter].ethos, split);
    }

    #[test]
    fn founders_start_recorded_and_daughters_record_their_own_inheritance_crossings() {
        let mut world = World::new(
            4,
            Params {
                ethos_shifts: true,
                ..Params::static_society()
            },
        );
        let home = world
            .map
            .regions
            .iter()
            .position(|r| {
                r.terrain == crate::Terrain::Plains
                    && r.neighbours
                        .iter()
                        .any(|&n| world.feeds(n, crate::Livelihood::Farming) > 0.0)
            })
            .unwrap();
        world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            0,
            0.5,
            0.5,
            Some(home),
            Some(crate::Livelihood::Farming),
            Some(&all(STRONG)),
        );
        found(&mut world, all(-STRONG));
        world.communities[0].size = world.feeds(home, crate::Livelihood::Farming);
        world.temper(0, Axis::Martial, -0.1).unwrap();
        world.temper(0, Axis::Roving, -0.01).unwrap();
        world.temper(1, Axis::Martial, 0.1).unwrap();
        assert!(
            !world
                .events
                .iter()
                .any(|(_, event)| matches!(event, WorldEvent::Temper { .. }))
        );
        world.temper(1, Axis::Martial, 0.06).unwrap();
        assert!(matches!(
            world.events.last().unwrap().1,
            WorldEvent::Temper {
                community: 1,
                axis: Axis::Martial,
                pole: Pole::Low,
                entered: false,
                cause: TemperCause::Fate,
            }
        ));
        let daughter = world.split(0, None, 0.0);
        assert_ne!(
            world.communities[daughter].home(),
            world.communities[0].home()
        );
        assert!(world.events.iter().any(|(_, event)| matches!(
            event,
            WorldEvent::Temper {
                community,
                axis: Axis::Roving,
                pole: Pole::High,
                entered: true,
                cause: TemperCause::Inheritance,
            } if *community == daughter
        )));
        let before = world.events.len();
        world.temper(daughter, Axis::Martial, -0.1).unwrap();
        assert_eq!(world.events.len(), before);
    }

    #[test]
    fn threshold_reversals_history_and_invalid_nudges_are_exact() {
        let mut world = World::new(
            9,
            Params {
                ethos_shifts: true,
                ..Params::static_society()
            },
        );
        found(&mut world, all(0.0));
        world.generation = 1;
        world.temper(0, Axis::Martial, 0.5).unwrap();
        world.temper(0, Axis::Martial, -1.0).unwrap();
        world.temper(0, Axis::Martial, 1.0).unwrap();
        let transitions: Vec<_> = world
            .events
            .iter()
            .filter_map(|(_, e)| match e {
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
            [
                (Pole::High, true),
                (Pole::High, false),
                (Pole::Low, true),
                (Pole::Low, false),
                (Pole::High, true),
            ]
        );
        assert_eq!(world.communities[0].ethos_at(0).martial, 0.0);
        assert_eq!(world.communities[0].ethos_at(1).martial, 0.5);
        assert_eq!(world.communities[0].ethos_history.len(), 2);
        for value in [f32::NAN, f32::INFINITY, 1.01, -1.01] {
            assert!(world.temper(0, Axis::Martial, value).is_err());
            assert_eq!(world.communities[0].ethos.martial, 0.5);
        }
        world.communities[0].ended = Some(1);
        assert!(world.temper(0, Axis::Martial, 0.1).is_err());
        assert_eq!(world.communities[0].ethos_at(100).martial, 0.5);
    }

    #[test]
    fn drifting_near_a_pole_stays_recorded_until_it_leaves_the_settled_band() {
        for (sign, pole) in [(1.0, Pole::High), (-1.0, Pole::Low)] {
            let mut world = World::new(
                9,
                Params {
                    ethos_shifts: true,
                    ..Params::static_society()
                },
            );
            found(&mut world, all(0.4 * sign));
            world.events.clear();
            for target in [0.55, 0.4, 0.55, 0.4, 0.55, SETTLED] {
                let amount = target * sign - world.communities[0].ethos.martial;
                world.nudge_ethos(0, Axis::Martial, amount, TemperCause::Drift);
                assert_eq!(
                    world.events,
                    [(
                        0,
                        WorldEvent::Temper {
                            community: 0,
                            axis: Axis::Martial,
                            pole,
                            entered: true,
                            cause: TemperCause::Drift,
                        }
                    )]
                );
            }
            world.nudge_ethos(0, Axis::Martial, -0.01 * sign, TemperCause::Drift);
            assert_eq!(
                world.events[1..],
                [(
                    0,
                    WorldEvent::Temper {
                        community: 0,
                        axis: Axis::Martial,
                        pole,
                        entered: false,
                        cause: TemperCause::Drift,
                    }
                )]
            );
        }
    }

    #[test]
    fn hardship_faith_and_seafaring_have_bounded_distinct_responses() {
        let mut world = World::new(
            8,
            Params {
                ethos_shifts: true,
                ..Params::static_society()
            },
        );
        found(&mut world, all(0.0));
        found(&mut world, all(0.0));
        // Founding is the year's response; hardship comes the next year.
        world.generation = 1;
        world.hardship_ethos(0, false);
        world.hardship_ethos(1, true);
        assert!(world.communities[0].ethos.roving < 0.0);
        assert!(world.communities[1].ethos.roving > 0.0);
        let hardship_piety = world.communities[0].ethos.pious;
        assert!(hardship_piety > 0.0);
        world.learn(0, Craft::Seafaring, None);
        assert!(world.communities[0].ethos.seaward > 0.0);
        let faith = world.found_religion(0, Revelation::Proclaimed);
        assert!(world.communities[0].ethos.pious > hardship_piety);
        world.convert(1, faith, Some(0));
        let converted_piety = world.communities[1].ethos.pious;
        assert!(converted_piety > hardship_piety);
        world.convert(1, faith, Some(0));
        assert_eq!(world.communities[1].ethos.pious, converted_piety);
        let mut last = (
            world.communities[1].ethos.pious,
            world.communities[1].ethos.roving,
        );
        let mut steps = Vec::new();
        for g in 2..=101 {
            world.generation = g;
            world.hardship_ethos(1, true);
            // A second hardship in the same year is no second response.
            world.hardship_ethos(1, true);
            let now = (
                world.communities[1].ethos.pious,
                world.communities[1].ethos.roving,
            );
            steps.push((now.0 - last.0, now.1 - last.1));
            last = now;
        }
        assert!((0.6..1.0).contains(&last.0), "pious {}", last.0);
        assert!((0.6..1.0).contains(&last.1), "roving {}", last.1);
        assert!(
            steps
                .windows(2)
                .all(|w| w[1].0 > 0.0 && w[1].0 < w[0].0 && w[1].1 > 0.0 && w[1].1 < w[0].1)
        );
    }

    /// A farming people founded on a plain, so that it can settle.
    fn farmers(seed: u64, ethos: FoundingEthos, livelihood: Livelihood) -> World {
        let mut world = World::new(
            seed,
            Params {
                ethos_shifts: true,
                ..Params::static_society()
            },
        );
        let home = world
            .map
            .regions
            .iter()
            .position(|r| r.terrain == crate::Terrain::Plains)
            .unwrap();
        world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            0,
            0.5,
            0.5,
            Some(home),
            Some(livelihood),
            Some(&ethos),
        );
        world
    }

    /// Twin worlds share every drift draw; one is crowded past what its
    /// lands feed and marked challenged every generation, so their
    /// difference is the relaxation alone.
    fn twins(seed: u64, ethos: FoundingEthos, livelihood: Livelihood) -> (World, World) {
        let quiet = farmers(seed, ethos, livelihood);
        let mut troubled = quiet.clone();
        let home = troubled.communities[0].home();
        troubled.communities[0].size = 2.0 * troubled.feeds(home, livelihood);
        (quiet, troubled)
    }

    fn relax(quiet: &mut World, troubled: &mut World, generations: u32) {
        for g in 1..=generations {
            quiet.generation = g;
            troubled.generation = g;
            troubled.communities[0].ethos_challenged = g;
            quiet.temper_generation();
            troubled.temper_generation();
        }
    }

    #[test]
    fn plenty_eases_piety_settles_farmers_and_comforts_the_unchallenged() {
        let start = FoundingEthos {
            martial: Some(0.5),
            pious: Some(0.8),
            roving: Some(0.5),
            ..all(0.0)
        };
        let (mut quiet, mut troubled) = twins(11, start, Livelihood::Farming);
        relax(&mut quiet, &mut troubled, 30);
        let (q, t) = (quiet.communities[0].ethos, troubled.communities[0].ethos);
        assert!(t.pious - q.pious > 0.5, "pious {} vs {}", q.pious, t.pious);
        assert!(q.pious > -0.05, "ease stops at neutral: {}", q.pious);
        assert!(
            t.roving - q.roving > 0.5,
            "roving {} vs {}",
            q.roving,
            t.roving
        );
        assert!(
            q.roving > -0.4,
            "settling stops at the farming prior: {}",
            q.roving
        );
        let comfort = t.martial - q.martial;
        assert!((0.04..0.12).contains(&comfort), "comfort {comfort}");
        assert_eq!(
            (q.open, q.hierarchical, q.seaward),
            (t.open, t.hierarchical, t.seaward)
        );
        let left = |world: &World, axis: Axis| {
            world.events.iter().any(|(_, e)| {
                matches!(e, WorldEvent::Temper { axis: a, pole: Pole::High, entered: false, .. } if *a == axis)
            })
        };
        assert!(left(&quiet, Axis::Pious) && left(&quiet, Axis::Roving));
        assert!(!left(&troubled, Axis::Pious) && !left(&troubled, Axis::Roving));

        // Just above the settled band, the relaxations themselves cross it:
        // the generation's drift draw is known, so each axis is placed
        // where the drift leaves it inside the band and the relaxation
        // takes it out.
        let drift = |axis: Axis| {
            stream(11, &[key("temper"), 1, 0, key(axis.id())]).gen_range(-0.012..0.012)
        };
        let mut world = farmers(
            11,
            FoundingEthos {
                pious: Some(0.5),
                roving: Some(0.5),
                ..all(0.0)
            },
            Livelihood::Farming,
        );
        world
            .temper(0, Axis::Pious, 0.367 - drift(Axis::Pious) - 0.5)
            .unwrap();
        world
            .temper(0, Axis::Roving, 0.37 - drift(Axis::Roving) - 0.5)
            .unwrap();
        assert!(
            !world
                .events
                .iter()
                .any(|(_, e)| matches!(e, WorldEvent::Temper { .. }))
        );
        world.generation = 1;
        world.temper_generation();
        let crossings: Vec<_> = world
            .events
            .iter()
            .filter_map(|(_, e)| match e {
                WorldEvent::Temper {
                    axis, pole, cause, ..
                } => Some((*axis, *pole, *cause)),
                _ => None,
            })
            .collect();
        assert_eq!(
            crossings,
            [
                (Axis::Pious, Pole::High, TemperCause::Ease),
                (Axis::Roving, Pole::High, TemperCause::Settled),
            ]
        );

        // Herders are never settled: their roving only drifts, while their
        // piety still eases.
        let (mut quiet, mut troubled) = twins(11, start, Livelihood::Herding);
        relax(&mut quiet, &mut troubled, 30);
        let (q, t) = (quiet.communities[0].ethos, troubled.communities[0].ethos);
        assert_eq!(q.roving, t.roving);
        assert!(t.pious - q.pious > 0.5);

        // A people that moved this generation does not settle that generation.
        let mut quiet = farmers(11, start, Livelihood::Farming);
        let mut moving = quiet.clone();
        quiet.generation = 1;
        moving.generation = 1;
        moving.events.push((
            1,
            WorldEvent::Migrated {
                community: 0,
                from: 0,
                to: 0,
                by_sea: false,
                itinerary: crate::fleets::Itinerary {
                    path: Vec::new(),
                    legs: Vec::new(),
                    carriers: Vec::new(),
                },
            },
        ));
        quiet.temper_generation();
        moving.temper_generation();
        let (q, m) = (quiet.communities[0].ethos, moving.communities[0].ethos);
        assert_eq!(q.pious, m.pious);
        assert!(m.roving - q.roving > 0.04);
    }

    #[test]
    fn rule_no_longer_blocks_comfort() {
        let mut world = World::new(
            5,
            Params {
                ethos_shifts: true,
                contact_turnover: 1000.0,
                ..Params::static_society()
            },
        );
        found(&mut world, all(0.5));
        found(&mut world, all(0.5));
        world.connect(0, 1, 0.8, ContactKind::Rule).unwrap();
        let mut twin = world.clone();
        for g in 1..=QUIET_SPAN + 60 {
            world.generation = g;
            twin.generation = g;
            twin.communities[1].ethos_challenged = g;
            world.temper_generation();
            twin.temper_generation();
        }
        assert!(world.ruled_by(1).is_some());
        let comfort = twin.communities[1].ethos.martial - world.communities[1].ethos.martial;
        assert!((0.15..0.35).contains(&comfort), "comfort {comfort}");
        let left = |world: &World| {
            world.events.iter().any(|(_, e)| {
                matches!(
                    e,
                    WorldEvent::Temper {
                        community: 1,
                        axis: Axis::Martial,
                        pole: Pole::High,
                        entered: false,
                        ..
                    }
                )
            })
        };
        assert!(left(&world) && !left(&twin));
    }

    #[test]
    fn band_keeps_every_axis_spread_and_unsaturated() {
        let mut pooled: Vec<Vec<f32>> = vec![Vec::new(); 6];
        let mut left_pole = [0; 6];
        for seed in 0..20 {
            let mut world = World::new(seed, Params::default());
            let home = world
                .map
                .regions
                .iter()
                .position(|r| r.terrain == crate::Terrain::Plains)
                .unwrap();
            let presets = SoundProfile::presets();
            let n = presets.len() as u64;
            for (i, power, openness) in [(0, 0.5, 0.4), (1, 0.4, 0.6), (2, 0.85, 0.3)] {
                world.found_seeded(
                    &Naming::People,
                    &presets[((((seed % n) * 3 + i) * 7) % n) as usize],
                    seed.wrapping_add(i),
                    power,
                    openness,
                    Some(home),
                    None,
                    None,
                );
            }
            world.connect(0, 1, 0.5, ContactKind::Trade).unwrap();
            world.connect(2, 1, 0.8, ContactKind::Rule).unwrap();
            world.connect(2, 0, 0.3, ContactKind::Neighbours).unwrap();
            world.run(160);
            for c in world.living() {
                for (i, axis) in Axis::ALL.into_iter().enumerate() {
                    pooled[i].push(world.communities[c].ethos.get(axis));
                }
            }
            let mut left = vec![[false; 6]; world.communities.len()];
            for (_, event) in &world.events {
                if let WorldEvent::Temper {
                    community,
                    axis,
                    entered: false,
                    ..
                } = event
                {
                    left[*community][*axis as usize] = true;
                }
            }
            for axes in left {
                for (i, left) in axes.into_iter().enumerate() {
                    left_pole[i] += usize::from(left);
                }
            }
        }
        for (i, axis) in Axis::ALL.into_iter().enumerate() {
            let values = &pooled[i];
            let n = values.len() as f32;
            let mean = values.iter().sum::<f32>() / n;
            let sd = (values.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n).sqrt();
            let saturated = values.iter().filter(|v| v.abs() > 0.9).count() as f32 / n;
            eprintln!("{axis:?}: mean {mean:+.4} sd {sd:.4} beyond 0.9 {saturated:.3}");
            // Hierarchical has no response of its own beyond long rule; its
            // spread is the founding draw after acculturation, about 0.11.
            let floor = if axis == Axis::Hierarchical {
                0.1
            } else {
                0.12
            };
            assert!(sd >= floor, "{axis:?}: pooled sd {sd}");
            assert!(mean.abs() <= 0.35, "{axis:?}: mean {mean}");
            assert!(saturated < 0.15, "{axis:?}: {saturated} beyond 0.9");
        }
        assert!(left_pole[Axis::Pious as usize] > 0 && left_pole[Axis::Roving as usize] > 0);
    }
}

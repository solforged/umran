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

impl World {
    pub(crate) fn founding_ethos(
        &self,
        community: usize,
        region: usize,
        livelihood: Livelihood,
        authored: Option<&FoundingEthos>,
    ) -> Ethos {
        if !self.params.ethos_enabled {
            return Ethos::default();
        }
        let (martial, hierarchical, roving) = match livelihood {
            Livelihood::Foraging => (-0.1, -0.3, 0.15),
            Livelihood::Herding => (0.3, 0.0, 0.4),
            Livelihood::Farming => (0.0, 0.3, -0.35),
        };
        let mut ethos = Ethos {
            martial,
            hierarchical,
            roving,
            seaward: if self.map.coastal(region) { 0.4 } else { -0.25 },
            ..Ethos::default()
        };
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

    pub(crate) fn hardship_ethos(&mut self, community: usize, crowded: bool) {
        self.communities[community].ethos_challenged = self.generation;
        self.nudge_ethos(community, Axis::Pious, 0.06, TemperCause::Hardship);
        self.nudge_ethos(
            community,
            Axis::Roving,
            if crowded { 0.06 } else { -0.04 },
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

    pub(crate) fn temper_generation(&mut self) {
        if !self.params.ethos_enabled || !self.params.ethos_shifts {
            return;
        }
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
                    .any(|m| m.left.is_none() && self.generation >= m.joined + 12)
            });
            if long_rule {
                self.nudge_ethos(c, Axis::Hierarchical, 0.004, TemperCause::LongRule);
            }
            let challenged = self.ruled_by(c).is_some()
                || self
                    .contacts
                    .iter()
                    .any(|k| k.kind == ContactKind::Rule && (k.a == c || k.b == c));
            if challenged {
                self.communities[c].ethos_challenged = self.generation;
            }
            if self.generation >= self.communities[c].ethos_challenged + 12 {
                self.nudge_ethos(c, Axis::Martial, -0.003, TemperCause::Comfort);
                self.nudge_ethos(c, Axis::Pious, -0.003, TemperCause::Comfort);
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
            None,
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
        world.connect(0, 1, 1.0, ContactKind::Intermarriage);
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
        world.connect(0, 1, 0.8, ContactKind::Rule);
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
        world.connect(0, 1, 1.0, ContactKind::Intermarriage);
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
        found(&mut world, all(STRONG));
        found(&mut world, all(-STRONG));
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
        for _ in 0..100 {
            world.hardship_ethos(1, true);
        }
        assert_eq!(world.communities[1].ethos.pious, 1.0);
        assert_eq!(world.communities[1].ethos.roving, 1.0);
    }
}

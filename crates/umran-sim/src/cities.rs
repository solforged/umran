//! Cities are residential subsets of peoples until their mixed speech becomes
//! a community. Reservations move people on the map, not into a second census;
//! forming townsfolk debits those same people exactly once.

use crate::change::{Env, Matcher, Rewrite, SoundChange};
use crate::ethos::{Effect, Ethos};
use crate::laws::Law;
use crate::lexicon::{Entry, Event, LexemeId, Origin, Variant};
use crate::names::{Name, Naming};
use crate::phoneme::PhonemeId;
use crate::provenance::LoanCause;
use crate::rng::{key, stream};
use crate::variety::Variety;
use crate::world::{Community, ContactKind, World, WorldEvent};
use rand::Rng;
use std::collections::BTreeMap;

pub const GREAT_CITY: f32 = 10_000.0;
const MIXED_SHARE: f32 = 0.15;
const MIXED_GENERATIONS: u32 = 4;
const TOWN: f32 = 2_500.0;

#[derive(Clone, Debug, PartialEq)]
pub struct City {
    pub state: usize,
    pub region: usize,
    pub since: u32,
    pub townsfolk: Option<usize>,
    /// Current speech shares, largest first. Formation ancestry is kept on
    /// the variety separately, even if these people later shift language.
    pub makeup: Vec<(usize, f32)>,
    /// Before formation, shares of source communities resident in the city.
    residents: Vec<(usize, f32)>,
    mixed: u32,
}

impl World {
    pub fn city_size(&self, city: usize) -> f32 {
        let city = &self.cities[city];
        if let Some(c) = city.townsfolk {
            let people = &self.communities[c];
            return if people.living() && people.lands.contains(&city.region) {
                self.presence(c)
                    .iter()
                    .find(|(r, _)| *r == city.region)
                    .map_or(0.0, |(_, n)| *n)
            } else {
                0.0
            };
        }
        city.residents
            .iter()
            .filter(|(c, _)| self.communities[*c].living())
            .map(|(c, share)| self.communities[*c].size * share)
            .sum()
    }

    /// City names are the capital's name in the state's standard (or court).
    pub fn city_name(&self, city: usize) -> Name {
        let city = &self.cities[city];
        let v = self.standard_variety(city.state);
        self.heard_place(city.region, v, &self.varieties[v])
            .expect("a state's inhabited capital has a name")
    }

    pub fn standard_community(&self, state: usize) -> usize {
        self.states[state]
            .standard_speakers
            .unwrap_or(self.states[state].rulers)
    }

    pub fn standard_variety(&self, state: usize) -> usize {
        self.communities[self.standard_community(state)].variety
    }

    pub(crate) fn city_residence(&self, community: usize) -> Option<(usize, f32)> {
        self.cities
            .iter()
            .filter(|city| city.townsfolk.is_none())
            .find_map(|city| {
                city.residents
                    .iter()
                    .find(|(c, _)| *c == community)
                    .map(|(_, share)| (city.region, self.communities[community].size * share))
            })
    }

    /// A territorial division keeps each city's actual residents on its land.
    /// Settlers are drawn proportionally instead, so the parent's reservation
    /// fraction already describes the smaller population remaining there.
    pub(crate) fn divide_city_residents(
        &mut self,
        parent: usize,
        daughter: usize,
        before: f32,
        territorial: bool,
    ) {
        if !territorial {
            self.refresh_city_makeup();
            return;
        }
        for city in &mut self.cities {
            let goes = self.communities[daughter].lands.contains(&city.region);
            if city.townsfolk == Some(parent) && goes {
                city.townsfolk = Some(daughter);
            }
            for (community, share) in &mut city.residents {
                if *community == parent {
                    let residents = before * *share;
                    if goes {
                        *community = daughter;
                    }
                    *share = residents / self.communities[*community].size;
                }
            }
        }
        self.refresh_city_makeup();
    }

    pub(crate) fn leave_city_residence(&mut self, community: usize) {
        for city in &mut self.cities {
            city.residents.retain(|(c, _)| *c != community);
        }
        self.refresh_city_makeup();
    }

    pub(crate) fn refresh_city_makeup(&mut self) {
        for city in &mut self.cities {
            if let Some(c) = city.townsfolk {
                let people = &self.communities[c];
                city.makeup = if people.living() && people.lands.contains(&city.region) {
                    vec![(people.variety, 1.0)]
                } else {
                    Vec::new()
                };
            } else {
                let mut counts = BTreeMap::new();
                for &(c, share) in &city.residents {
                    let people = &self.communities[c];
                    if people.living() {
                        *counts.entry(people.variety).or_default() += people.size * share;
                    }
                }
                city.makeup = shares(counts);
            }
        }
    }

    /// Tribute is moved from the court's food budget to its townsfolk,
    /// never added twice. Townsfolk themselves pay no circular tribute.
    pub(crate) fn urban_food(&self, state: usize) -> Option<(usize, f32)> {
        self.cities
            .iter()
            .find(|city| city.state == state)
            .and_then(|city| {
                city.townsfolk
                    .filter(|&c| {
                        self.communities[c].living()
                            && self.communities[c].lands.contains(&city.region)
                    })
                    .map(|c| (c, self.communities[c].size))
            })
    }

    pub(crate) fn city_wave_weight(&self, a: usize, b: usize) -> f32 {
        let urban = |c| {
            self.cities.iter().any(|city| {
                city.townsfolk == Some(c) && self.communities[c].lands.contains(&city.region)
            })
        };
        if !urban(a) && !urban(b) {
            return 1.0;
        }
        let d = 1.0 + self.apart(a, b) / crate::geography::REFERENCE_TRAVEL_KM;
        let gravity = (self.communities[a].size / GREAT_CITY)
            * (self.communities[b].size / GREAT_CITY)
            / (d * d);
        1.0 + gravity.min(3.0)
    }
    fn city_sources(&self, state: usize, city: Option<usize>) -> Vec<(usize, f32)> {
        let region = self.states[state].capital;
        let town = city.and_then(|i| self.cities[i].townsfolk);
        std::iter::once(self.states[state].rulers)
            .chain(self.states[state].subjects())
            .filter(|&c| Some(c) != town && self.communities[c].living())
            .filter(|&c| {
                !self.cities.iter().enumerate().any(|(other, town)| {
                    Some(other) != city
                        && town.townsfolk.is_none()
                        && town.residents.iter().any(|(source, _)| *source == c)
                })
            })
            .map(|c| {
                let d = self.communities[c]
                    .lands
                    .iter()
                    .map(|&r| self.map.distance(r, region))
                    .fold(f32::INFINITY, f32::min);
                (
                    c,
                    self.communities[c].size / (1.0 + d / crate::geography::REFERENCE_TRAVEL_KM),
                )
            })
            .collect()
    }

    pub(crate) fn grow_cities(&mut self) {
        if self.params.city_rate <= 0.0 {
            return;
        }
        for state in 0..self.states.len() {
            if self.states[state].standing()
                && self.city_capacity(state) >= GREAT_CITY
                && !self.cities.iter().any(|city| city.state == state)
            {
                let sources = self.city_sources(state, None);
                let total: f32 = sources.iter().map(|(_, w)| w).sum();
                let capacity = self.city_capacity(state);
                let actual: f32 = sources
                    .iter()
                    .map(|(c, w)| (capacity * w / total).min(self.communities[*c].size * 0.5))
                    .sum();
                if actual < GREAT_CITY {
                    continue;
                }
                let i = self.cities.len();
                self.cities.push(City {
                    state,
                    region: self.states[state].capital,
                    since: self.generation,
                    townsfolk: None,
                    makeup: Vec::new(),
                    residents: Vec::new(),
                    mixed: 0,
                });
                self.record_event(WorldEvent::City { city: i });
            }
        }
        for i in 0..self.cities.len() {
            let state = self.cities[i].state;
            if !self.states[state].standing() {
                self.disperse_city(i);
                continue;
            }
            let region = self.cities[i].region;
            let town = self.cities[i].townsfolk;
            let capacity = self.city_capacity(state);
            let sources = self.city_sources(state, Some(i));
            let total: f32 = sources.iter().map(|(_, w)| w).sum();
            if total <= 0.0 {
                continue;
            }
            if let Some(town) = town {
                if !self.communities[town].living()
                    || !self.communities[town].lands.contains(&region)
                {
                    self.cities[i].makeup.clear();
                    continue;
                }
                let mut counts = BTreeMap::new();
                counts.insert(self.communities[town].variety, self.communities[town].size);
                let gap = (capacity - self.city_size(i)).max(0.0);
                for (c, weight) in sources {
                    let n = (gap * weight / total * self.params.city_rate)
                        .min(self.communities[c].size * 0.05);
                    self.communities[c].size -= n;
                    self.communities[town].size += n;
                    *counts.entry(self.communities[c].variety).or_default() += n;
                }
                self.cities[i].makeup = shares(counts);
            } else {
                let first = self.cities[i].residents.is_empty();
                let mut residents = Vec::with_capacity(sources.len());
                let mut counts = BTreeMap::new();
                for (c, weight) in sources {
                    let size = self.communities[c].size;
                    let target = (capacity * weight / total / size).min(0.5);
                    let old = self.cities[i]
                        .residents
                        .iter()
                        .find(|(source, _)| *source == c)
                        .map_or(0.0, |(_, share)| *share);
                    let share = if first {
                        target
                    } else {
                        old + (target - old) * self.params.city_rate.min(1.0)
                    };
                    residents.push((c, share));
                    *counts.entry(self.communities[c].variety).or_default() += size * share;
                }
                self.cities[i].residents = residents;
                self.cities[i].makeup = shares(counts);
                let mixed = self.cities[i]
                    .makeup
                    .iter()
                    .filter(|(_, share)| *share >= MIXED_SHARE)
                    .count()
                    >= 2;
                self.cities[i].mixed = if mixed { self.cities[i].mixed + 1 } else { 0 };
                if self.cities[i].mixed >= MIXED_GENERATIONS && self.city_size(i) >= GREAT_CITY {
                    self.form_koine(i);
                }
            }
        }
        self.link_cities();
    }

    fn disperse_city(&mut self, i: usize) {
        let Some(town) = self.cities[i].townsfolk else {
            // Reservations return to their source people's ordinary lands.
            self.cities[i].residents.clear();
            self.cities[i].makeup.clear();
            return;
        };
        if !self.communities[town].living()
            || !self.communities[town]
                .lands
                .contains(&self.cities[i].region)
        {
            self.cities[i].makeup.clear();
            return;
        }
        let state = self.cities[i].state;
        let destination = std::iter::once(self.states[state].rulers)
            .chain(self.states[state].members.iter().map(|m| m.community))
            .find(|&c| c != town && self.communities[c].living());
        if let Some(c) = destination {
            let leaving = (self.city_size(i) - TOWN).max(0.0) * 0.2;
            self.communities[town].size -= leaving;
            self.communities[c].size += leaving;
        }
        self.cities[i].makeup = vec![(self.communities[town].variety, 1.0)];
    }

    fn form_koine(&mut self, city: usize) {
        let parent = self.cities[city].makeup[0].0;
        let cause = LoanCause::City {
            city,
            community: self.communities.len(),
        };
        let mut variety = self.levelled_variety(&self.cities[city].makeup, cause);
        self.inherit_places(parent, &mut variety);
        let land = self
            .heard_place(self.cities[city].region, parent, &variety)
            .expect("a capital has a land name");
        let name = self.coin(&variety, &Naming::Land, Some((&land, &variety)));
        variety.name = self.fresh_language_name(&variety, &name);
        let v = self.varieties.len();
        self.varieties.push(variety);
        let source = self.cities[city]
            .residents
            .iter()
            .filter(|(c, _)| self.communities[*c].variety == parent)
            .max_by(|(a, x), (b, y)| {
                (self.communities[*a].size * x).total_cmp(&(self.communities[*b].size * y))
            })
            .expect("largest contributor has residents")
            .0;
        let size = self.city_size(city);
        let ethos = Ethos::mean(
            self.cities[city]
                .residents
                .iter()
                .map(|&(c, share)| (self.communities[c].ethos, self.communities[c].size * share)),
        );
        for &(c, share) in &self.cities[city].residents {
            self.communities[c].size -= self.communities[c].size * share;
        }
        let town = self.communities.len();
        let source = &self.communities[source];
        self.communities.push(Community {
            parents: self.cities[city]
                .residents
                .iter()
                .map(|(c, _)| *c)
                .collect(),
            name,
            variety: v,
            size,
            lands: vec![self.cities[city].region],
            ended: None,
            prestige: source.prestige,
            power: source.power,
            openness: source.openness,
            livelihood: source.livelihood,
            faith: source.faith,
            crafts: source.crafts.clone(),
            ethos,
            temper_marks: ethos.temper_marks(),
            ethos_history: vec![(self.generation, ethos)],
            ethos_challenged: self.generation,
        });
        let state = self.cities[city].state;
        let rulers = self.states[state].rulers;
        self.cities[city].townsfolk = Some(town);
        self.cities[city].residents.clear();
        self.inherit_state(rulers, town);
        let contributors: Vec<usize> = self.states[state]
            .subjects()
            .filter(|&c| c != town)
            .collect();
        for c in contributors {
            let kind = if self.contact_eligible(town, c, ContactKind::Neighbours) {
                ContactKind::Neighbours
            } else {
                ContactKind::Trade
            };
            if self.contact_eligible(town, c, kind) {
                self.connect(town, c, 0.5, kind)
                    .expect("city contributors are reachable");
            }
        }
        let law = levelling_law(&self.varieties[v].koine_mergers);
        if !law.rules.is_empty() {
            self.apply_law(v, &law);
        }
        let cause = self.triggers.cities.get(&city).map(|&event| crate::Cause {
            event,
            mechanism: crate::Mechanism::City,
        });
        self.record_response(
            WorldEvent::Koine {
                city,
                community: town,
                variety: v,
            },
            cause,
        );
    }

    fn levelled_variety(&self, makeup: &[(usize, f32)], cause: LoanCause) -> Variety {
        let parent = makeup[0].0;
        let mut out = self.varieties[parent].fork(parent, self.generation);
        out.koine_of = makeup.to_vec();
        let mut sounds = BTreeMap::<u16, f32>::new();
        for &(v, share) in makeup {
            let (consonants, vowels) = self.varieties[v].inventory();
            for p in consonants.into_iter().chain(vowels) {
                *sounds.entry(p.0).or_default() += share;
            }
        }
        out.koine_mergers = sounds
            .iter()
            .filter(|(_, share)| **share < 0.5)
            .filter_map(|(&from, _)| {
                sounds
                    .iter()
                    .filter(|(_, share)| **share >= 0.5)
                    .map(|(&to, _)| (to, crate::adapt::distance(PhonemeId(from), PhonemeId(to))))
                    .filter(|(_, d)| d.is_finite())
                    .min_by(|(a, x), (b, y)| x.total_cmp(y).then(a.cmp(b)))
                    .map(|(to, _)| (PhonemeId(from), PhonemeId(to)))
            })
            .collect();
        for i in 0..out.lexicon.slots.len() {
            if out.lexicon.slots[i].concept.stability.is_none()
                || crate::pronouns::is_pronoun(out.lexicon.slots[i].concept)
            {
                continue;
            }
            let mut cognates = BTreeMap::<(usize, LexemeId), (f32, usize, LexemeId, f32)>::new();
            for &(v, share) in makeup {
                for variant in &self.varieties[v].lexicon.slots[i].variants {
                    let usage = share * variant.weight;
                    let group = cognates.entry(self.root_of(v, variant.lexeme)).or_insert((
                        0.0,
                        v,
                        variant.lexeme,
                        -1.0,
                    ));
                    group.0 += usage;
                    if usage > group.3 {
                        (group.1, group.2, group.3) = (v, variant.lexeme, usage);
                    }
                }
            }
            let Some((_, &(_, from, source, _))) = cognates
                .iter()
                .max_by(|(a, x), (b, y)| x.0.total_cmp(&y.0).then_with(|| b.cmp(a)))
            else {
                continue;
            };
            let id = if from == parent {
                source
            } else {
                let word = self.varieties[from].lexicon.get(source);
                let id = out.lexicon.coin(
                    word.form.clone(),
                    Origin::Borrowed {
                        from,
                        source,
                        cause,
                    },
                    out.lexicon.slots[i].concept,
                    self.generation,
                );
                out.lexicon.get_mut(id).log.push(Entry {
                    generation: self.generation,
                    event: Event::Borrowed {
                        from,
                        source: word.form.clone(),
                        cause,
                    },
                });
                id
            };
            let slot = &mut out.lexicon.slots[i];
            for old in &slot.variants {
                if old.lexeme != id {
                    out.lexicon.lexemes[old.lexeme.0 as usize].log.push(Entry {
                        generation: self.generation,
                        event: Event::Lost {
                            sense: slot.concept,
                        },
                    });
                }
            }
            slot.variants = vec![Variant {
                lexeme: id,
                weight: 1.0,
            }];
        }
        let used: std::collections::BTreeSet<_> = out
            .lexicon
            .slots
            .iter()
            .flat_map(|slot| slot.variants.iter().map(|v| v.lexeme))
            .collect();
        for word in &mut out.lexicon.lexemes {
            if word.obsolete.is_none() && !used.contains(&word.id) {
                word.obsolete = Some(self.generation);
                word.log.push(Entry {
                    generation: self.generation,
                    event: Event::Obsolete,
                });
            }
        }
        let rare =
            |relation| {
                out.lexicon.living().filter(|l|
            matches!(l.origin, Origin::Derived { relation: r, .. } if r == relation)).count() < 3
            };
        out.morphology.affixes.retain(|(relation, _)| {
            !rare(*relation)
                || makeup
                    .iter()
                    .filter(|(v, _)| {
                        self.varieties[*v]
                            .morphology
                            .affixes
                            .iter()
                            .any(|(r, _)| r == relation)
                    })
                    .map(|(_, share)| share)
                    .sum::<f32>()
                    >= 0.5
        });
        out.morphology.patterns.retain(|(relation, _)| {
            !rare(*relation)
                || makeup
                    .iter()
                    .filter(|(v, _)| {
                        self.varieties[*v]
                            .morphology
                            .patterns
                            .iter()
                            .any(|(r, _)| r == relation)
                    })
                    .map(|(_, share)| share)
                    .sum::<f32>()
                    >= 0.5
        });
        // New loans receive native marking before the city's sound mergers.
        out.sync_grammar(self.generation);
        out
    }

    fn link_cities(&mut self) {
        let towns: Vec<usize> = self
            .cities
            .iter()
            .enumerate()
            .filter(|(i, _)| self.city_size(*i) >= GREAT_CITY)
            .filter_map(|(_, city)| city.townsfolk)
            .collect();
        for (i, &a) in towns.iter().enumerate() {
            for &b in &towns[i + 1..] {
                if a == b
                    || self
                        .contacts
                        .iter()
                        .any(|c| (c.a == a && c.b == b) || (c.a == b && c.b == a))
                    || !self.contact_eligible(a, b, ContactKind::Trade)
                {
                    continue;
                }
                let mut rng = stream(
                    self.seed,
                    &[key("city"), self.generation as u64, a as u64, b as u64],
                );
                let openness = Ethos {
                    open: (self.communities[a].ethos.open + self.communities[b].ethos.open) / 2.0,
                    ..Ethos::default()
                };
                let pull =
                    (self.city_wave_weight(a, b) - 1.0) * 0.08 * openness.factor(Effect::Contact);
                if rng.r#gen::<f32>() < pull {
                    self.connect(a, b, 0.5, ContactKind::Trade)
                        .expect("the cities have physical access");
                }
            }
        }
    }
}

fn shares(counts: BTreeMap<usize, f32>) -> Vec<(usize, f32)> {
    let total: f32 = counts.values().sum();
    let mut out: Vec<_> = counts
        .into_iter()
        .filter(|(_, n)| *n > 0.0)
        .map(|(v, n)| (v, n / total))
        .collect();
    out.sort_by(|(a, x), (b, y)| y.total_cmp(x).then(a.cmp(b)));
    out
}

fn levelling_law(mergers: &[(PhonemeId, PhonemeId)]) -> Law {
    Law {
        id: "koine-levelling",
        label: "Minority sounds merge into majority sounds",
        commonness: 0.0,
        rules: mergers
            .iter()
            .map(|&(from, to)| SoundChange {
                id: "koine-levelling".into(),
                target: Matcher::Phone(from),
                result: Rewrite::Phone(to),
                left: Env::Any,
                right: Env::Any,
            })
            .collect(),
        stress: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fall, Form, Livelihood, Params, SoundProfile};

    #[test]
    fn settlement_census_includes_cities_and_preserves_each_regions_residents() {
        use crate::settlement::{SettlementChoice, SettlementIntent};
        let mut base = World::new(7, Params::static_society());
        base.found(&SoundProfile::base(), 0.5, 0.5);
        let home = base.communities[0].home();
        let next = base.map.regions[home]
            .neighbours
            .iter()
            .copied()
            .filter(|&r| base.map.regions[r].terrain.is_land())
            .max_by(|&a, &b| {
                base.feeds(a, base.communities[0].livelihood)
                    .total_cmp(&base.feeds(b, base.communities[0].livelihood))
            })
            .unwrap();
        base.communities[0].lands.push(next);
        base.communities[0].size = 5_000.0;
        base.raise_state(0, Some(home), crate::Rise::Proclaimed);
        for city_region in [home, next] {
            let mut world = base.clone();
            // The same resident transfer must work on either side of the new boundary.
            world.cities.push(City {
                state: 0,
                region: city_region,
                since: 0,
                townsfolk: None,
                makeup: vec![(0, 1.0)],
                residents: vec![(0, 0.3)],
                mixed: 0,
            });
            let choice = SettlementChoice {
                community: 0,
                intent: SettlementIntent::Partition,
                destination: next,
                share: 0.5,
                naming: None,
                intensity: 0.5,
            };
            let plan = world.plan_settlement(&choice).unwrap();
            let city_before = world.city_size(0);
            let daughter = world.settle(&choice).unwrap().unwrap();
            for (c, allocation) in [(0, &plan.remaining), (daughter, &plan.arriving)] {
                assert!((world.communities[c].size - allocation.population).abs() < 0.01);
                for (region, n) in world.presence(c) {
                    let expected = allocation
                        .presence
                        .iter()
                        .find(|(r, _)| *r == region)
                        .unwrap()
                        .1;
                    assert!(
                        (n - expected).abs() < 0.01,
                        "resident allocation changed at land {region}"
                    );
                }
            }
            assert!((world.city_size(0) - city_before).abs() < 0.01);
            let speakers = if city_region == home { 0 } else { daughter };
            assert_eq!(
                world.cities[0].makeup,
                vec![(world.communities[speakers].variety, 1.0)]
            );
        }
        let mut world = base;
        world.cities.push(City {
            state: 0,
            region: home,
            since: 0,
            townsfolk: None,
            makeup: vec![(0, 1.0)],
            residents: vec![(0, 0.3)],
            mixed: 0,
        });
        let destination = world
            .settlement_options(0, SettlementIntent::Migration, 1.0)
            .unwrap()
            .into_iter()
            .find(|o| o.reason.is_none())
            .unwrap()
            .region;
        let choice = SettlementChoice {
            community: 0,
            intent: SettlementIntent::Migration,
            destination,
            share: 1.0,
            naming: None,
            intensity: 0.5,
        };
        let plan = world.plan_settlement(&choice).unwrap();
        assert_eq!(plan.falling_states, vec![0]);
        let language = world.communities[0].variety;
        world.settle(&choice).unwrap();
        assert_eq!(world.communities[0].variety, language);
        assert_eq!(world.presence(0), plan.arriving.presence);
        assert!(world.states[0].fell.is_some());
        assert_eq!(world.city_size(0), 0.0);
        assert!(world.cities[0].makeup.is_empty());
    }

    #[test]
    fn settlers_count_parent_residents_remaining_in_an_external_city() {
        use crate::settlement::{SettlementChoice, SettlementIntent};
        let mut world = World::new(7, Params::static_society());
        world.found(&SoundProfile::base(), 0.5, 0.5);
        world.found(&SoundProfile::base(), 0.7, 0.5);
        let home = world.communities[0].home();
        let city = world.map.regions[home]
            .neighbours
            .iter()
            .copied()
            .find(|&r| world.map.regions[r].terrain.is_land())
            .unwrap();
        world.communities[1].lands = vec![city];
        world.raise_state(1, Some(city), crate::Rise::Proclaimed);
        world.cities.push(City {
            state: 0,
            region: city,
            since: 0,
            townsfolk: None,
            makeup: vec![(0, 0.75), (1, 0.25)],
            residents: vec![(0, 0.3), (1, 0.1)],
            mixed: 0,
        });
        let choice = SettlementChoice {
            community: 0,
            intent: SettlementIntent::Settlers,
            destination: city,
            share: 0.5,
            naming: None,
            intensity: 0.5,
        };
        let plan = world.plan_settlement(&choice).unwrap();
        assert!(plan.inhabitants.contains(&(0, 150.0)));
        assert!((plan.room - (world.free_room(0, city, &world.spatial()) - 150.0)).abs() < 0.01);
        let daughter = world.settle(&choice).unwrap().unwrap();
        assert_eq!(world.city_residence(0), Some((city, 150.0)));
        assert_eq!(world.presence(daughter), plan.arriving.presence);
        assert_eq!(world.cities[0].makeup, vec![(0, 0.6), (1, 0.4)]);
    }

    fn realm() -> World {
        let mut world = World::new(
            42,
            Params {
                city_rate: 0.25,
                ..Params::static_society()
            },
        );
        for (i, size) in [80_000.0, 70_000.0, 50_000.0].into_iter().enumerate() {
            let c = world.found(&SoundProfile::base(), 0.5, 0.5);
            world.communities[c].size = size;
            world.communities[c].livelihood = Livelihood::Farming;
            if i > 0 {
                world.communities[c].lands = world.communities[0].lands.clone();
            }
        }
        // Choose a fertile capital so its court has the intended food budget.
        let capital = world
            .map
            .regions
            .iter()
            .position(|r| r.terrain == crate::Terrain::Plains)
            .unwrap();
        for c in &mut world.communities {
            c.lands = vec![capital];
        }
        world.connect(0, 1, 0.8, ContactKind::Rule).unwrap();
        world.connect(0, 2, 0.8, ContactKind::Rule).unwrap();
        world.refresh_places();
        world
    }

    fn city_steps(world: &mut World, n: u32) {
        for _ in 0..n {
            world.generation += 1;
            world.grow_cities();
        }
    }

    fn population(world: &World) -> f32 {
        world.living().map(|c| world.communities[c].size).sum()
    }

    #[test]
    fn one_speech_never_forms_a_koine_and_static_society_stays_silent() {
        let mut world = realm();
        for c in &mut world.communities {
            c.variety = 0;
        }
        city_steps(&mut world, 30);
        assert_eq!(world.cities.len(), 1);
        assert_eq!(world.cities[0].townsfolk, None);
        assert_eq!(world.cities[0].makeup, vec![(0, 1.0)]);
        let mut fixed = realm();
        fixed.params = Params::static_society();
        city_steps(&mut fixed, 30);
        assert!(fixed.cities.is_empty());
    }

    #[test]
    fn reservations_formation_and_fall_conserve_the_census() {
        let mut world = realm();
        let before = population(&world);
        city_steps(&mut world, 3);
        assert_eq!(world.cities[0].townsfolk, None, "mixture must persist");
        for c in world.living() {
            let counted: f32 = world.presence(c).iter().map(|(_, n)| n).sum();
            assert!((counted - world.communities[c].size).abs() < 0.02);
        }
        city_steps(&mut world, 1);
        let town = world.cities[0].townsfolk.expect("four mixed generations");
        assert!((population(&world) - before).abs() < 0.1);
        let size = world.communities[town].size;
        assert_eq!(world.communities[town].lands, vec![world.states[0].capital]);
        city_steps(&mut world, 8);
        assert!((population(&world) - before).abs() < 0.2);
        world.fall(0, Fall::Collapsed);
        city_steps(&mut world, 30);
        assert!((population(&world) - before).abs() < 0.5);
        assert!(world.communities[town].living());
        assert!(world.city_size(0) < size * 0.3);
        assert!(world.city_size(0) >= TOWN);
    }

    #[test]
    fn townsfolk_inherit_the_residents_size_weighted_ethos() {
        let mut world = realm();
        for (c, value) in world.communities.iter_mut().zip([1.0, -1.0, 0.2]) {
            c.ethos = Ethos {
                martial: value,
                open: -value,
                pious: value,
                hierarchical: -value,
                roving: value,
                seaward: -value,
            };
        }
        city_steps(&mut world, 4);
        let town = world.cities[0].townsfolk.unwrap();
        let ethos = world.communities[town].ethos;
        for axis in crate::Axis::ALL {
            let expected = if matches!(
                axis,
                crate::Axis::Open | crate::Axis::Hierarchical | crate::Axis::Seaward
            ) {
                -0.1
            } else {
                0.1
            };
            assert!(
                (ethos.get(axis) - expected).abs() < 1e-6,
                "{axis:?}: {ethos:?}"
            );
        }
        assert_eq!(world.communities[town].ethos_at(4), ethos);
    }

    #[test]
    fn minority_mergers_are_regular_and_leave_obsolete_words_frozen() {
        let mut world = realm();
        for (i, v) in world.varieties.iter_mut().enumerate() {
            let form = Form::from_ipa(if i == 0 { "θaθa" } else { "tata" }).unwrap();
            for word in &mut v.lexicon.lexemes {
                word.form = form.clone();
                word.paradigms.clear();
            }
            // The hypothetical dialect includes its spoken grammatical forms.
            for marker in &mut v.grammar.markers {
                marker.form = match marker.kind {
                    crate::grammar::MarkerKind::Bound | crate::grammar::MarkerKind::Pattern => {
                        Form::from_ipa("a").unwrap()
                    }
                    crate::grammar::MarkerKind::Particle => form.clone(),
                    crate::grammar::MarkerKind::None => Form::default(),
                };
            }
            v.sync_grammar(0);
        }
        let sense = crate::concepts::by_id("water").unwrap();
        let old = world.varieties[0].lexicon.coin(
            Form::from_ipa("aθa").unwrap(),
            Origin::Expressive,
            sense,
            0,
        );
        world.varieties[0].lexicon.get_mut(old).obsolete = Some(0);
        city_steps(&mut world, 4);
        let town = world.cities[0].townsfolk.unwrap();
        let v = world.variety_of(town);
        assert_eq!(v.parent.unwrap().variety, 0);
        for word in v.lexicon.living() {
            assert_eq!(word.form.ipa(), "tata");
            assert!(word.log.iter().any(|e| matches!(&e.event,
                Event::SoundLaw { law: "koine-levelling", before } if before.ipa() == "θaθa")));
        }
        for (form, _) in v.grammar.forms(&v.lexicon) {
            assert!(
                form.phones()
                    .all(|phone| matches!(crate::CATALOG.get(phone).ipa(), "t" | "a"))
            );
        }
        assert_eq!(v.lexicon.get(old).form.ipa(), "aθa");
        assert_eq!(v.laws.last(), Some(&(4, "koine-levelling")));
    }

    #[test]
    fn cognate_usage_votes_together_against_a_larger_single_dialect() {
        let mut world = realm();
        world.varieties[2] = world.varieties[1].fork(1, 0);
        let sense = crate::concepts::by_id("water").unwrap();
        let expected = world.varieties[1]
            .lexicon
            .word_for(sense)
            .unwrap()
            .form
            .clone();
        let koine =
            world.levelled_variety(&[(0, 0.45), (1, 0.30), (2, 0.25)], LoanCause::Unrecorded);
        assert_eq!(koine.lexicon.word_for(sense).unwrap().form, expected);
        assert_eq!(koine.parent.unwrap().variety, 0);
    }

    #[test]
    fn town_standard_and_gravity_use_the_city_not_the_court() {
        let mut world = realm();
        city_steps(&mut world, 4);
        let town = world.cities[0].townsfolk.unwrap();
        world.adopt_standard(0);
        let speech = world.communities[town].variety;
        assert_eq!(world.standards()[speech], Some(0));
        assert_eq!(world.standards()[0], None);
        assert!(world.under_standard(1, town));
        assert!(!world.under_standard(1, 0));
        assert_eq!(
            world.city_wave_weight(0, 1),
            1.0,
            "rural waves are unchanged"
        );
        let near = world.city_wave_weight(town, 1);
        world.communities[1].size *= 0.1;
        assert!(world.city_wave_weight(town, 1) < near);
        assert!((1.0..=4.0).contains(&near));
    }

    #[test]
    fn cities_replay_with_identical_makeup_words_and_waves() {
        let run = || {
            let mut world = realm();
            city_steps(&mut world, 4);
            world.params.sound_change_rate = 0.4;
            world.params.wave_rate = 1.0;
            world.run(12);
            world
        };
        let a = run();
        let b = run();
        assert_eq!(a.cities, b.cities);
        assert_eq!(a.events, b.events);
        assert_eq!(a.communities, b.communities);
        for (a, b) in a.varieties.iter().zip(&b.varieties) {
            assert_eq!(a.lexicon, b.lexicon);
            assert_eq!(a.laws, b.laws);
            assert_eq!(a.waves, b.waves);
            assert_eq!(a.koine_of, b.koine_of);
        }
    }
}

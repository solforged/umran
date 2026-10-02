//! Rival faiths, roads to their shrines, and changes of holy-land allegiance.
use crate::compare::intelligibility;
use crate::ethos::{Effect, Ethos};
use crate::ideas::Religion;
use crate::names::{MAX_PEOPLE_NAME, Name, Naming, clipped, given_name};
use crate::rng::{index, key, stream, weighted_index};
use crate::world::{ContactKind, World, WorldEvent};
use rand::Rng;
use serde::Serialize;

const PILGRIM_REACH: f32 = 18.0;
const SCHISM_QUIET: u32 = 12;
const MAX_DESCENDANTS: usize = 6;
const REFORM_SHARING: f32 = 0.55;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SchismCause {
    Distance,
    Rule,
    Reform,
    Succession,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BranchNaming {
    Leader,
    Land,
    Epithet,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Pilgrimage {
    pub people: usize,
    pub from: usize,
    pub to: usize,
    pub path: Vec<usize>,
    pub since: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HolyLand {
    pub region: usize,
    pub held_by: Option<usize>,
    pub faithful: bool,
}

impl World {
    pub fn faith_root(&self, mut religion: usize) -> usize {
        while let Some(parent) = self.religions[religion].parent {
            religion = parent;
        }
        religion
    }

    pub fn faith_name(&self, religion: usize) -> String {
        let r = &self.religions[religion];
        self.varieties[r.name_variety].title(&r.name.form)
    }

    /// The largest population actually living on a land, not merely its namers.
    pub fn land_holder(&self, region: usize) -> Option<usize> {
        self.living()
            .filter_map(|c| {
                let k = &self.communities[c];
                if !k.lands.contains(&region) {
                    return None;
                }
                let total: f32 = k.lands.iter().map(|&r| self.feeds(r, k.livelihood)).sum();
                let share = if total > 0.0 {
                    self.feeds(region, k.livelihood) / total
                } else {
                    1.0 / k.lands.len() as f32
                };
                Some((c, k.size * share))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
            .map(|(c, _)| c)
    }

    pub fn holy_lands(&self, religion: usize) -> Vec<HolyLand> {
        self.religions[religion]
            .shrines()
            .map(|shrine| {
                let held_by = self.land_holder(shrine.region);
                HolyLand {
                    region: shrine.region,
                    held_by,
                    faithful: held_by.is_some_and(|c| self.communities[c].faith == Some(religion)),
                }
            })
            .collect()
    }

    pub(crate) fn observe_holy_lands(&mut self) {
        if self.params.schism_rate <= 0.0 && self.params.pilgrimage_rate <= 0.0 {
            return;
        }
        for religion in 0..self.religions.len() {
            let now = self.holy_lands(religion);
            for (old, new) in self.religions[religion].holy_land.iter().zip(&now) {
                if old.faithful != new.faithful {
                    self.events.push((
                        self.generation,
                        WorldEvent::HolyLand {
                            religion,
                            region: new.region,
                            was_held_by: old.held_by,
                            held_by: new.held_by,
                            faithful: new.faithful,
                        },
                    ));
                }
            }
            self.religions[religion].holy_land = now;
        }
    }

    /// A bounded hazard bonus, never a new conquest draw. A branch also
    /// reveres its ancestors' shrines, but does not attack its own faithful.
    pub(crate) fn holy_war_target(&self, attacker: usize, target: usize) -> bool {
        if self.params.conquest_rate <= 0.0 {
            return false;
        }
        let Some(faith) = self.communities[attacker].faith else {
            return false;
        };
        let held_faith = self.communities[target].faith;
        if held_faith == Some(faith) {
            return false;
        }
        let mut ancestor = Some(faith);
        while let Some(f) = ancestor {
            if held_faith != Some(f)
                && self.religions[f]
                    .shrines()
                    .any(|s| self.land_holder(s.region) == Some(target))
            {
                return true;
            }
            ancestor = self.religions[f].parent;
        }
        false
    }

    pub(crate) fn divide_faiths(&mut self) {
        if self.params.schism_rate <= 0.0 {
            return;
        }
        // New branches cannot split again in the generation they appear.
        for parent in 0..self.religions.len() {
            let root = self.faith_root(parent);
            let descendants = self
                .religions
                .iter()
                .enumerate()
                .filter(|&(r, _)| r != root && self.faith_root(r) == root)
                .count();
            if descendants >= MAX_DESCENDANTS
                || self.religions.iter().enumerate().any(|(r, f)| {
                    self.faith_root(r) == root
                        && f.split.is_some_and(|g| self.generation < g + SCHISM_QUIET)
                })
            {
                continue;
            }
            let mut rng = stream(
                self.seed,
                &[key("schism"), u64::from(self.generation), parent as u64],
            );
            let (pious, size) = self
                .living()
                .filter(|&c| self.communities[c].faith == Some(parent))
                .fold((0.0, 0.0), |(pious, size), c| {
                    let people = &self.communities[c];
                    (pious + people.ethos.pious * people.size, size + people.size)
                });
            let devotion = Ethos {
                pious: if size > 0.0 { pious / size } else { 0.0 },
                ..Ethos::default()
            };
            if rng.r#gen::<f32>()
                >= self.params.schism_rate / (1.0 + descendants as f32 * 0.4)
                    * devotion.factor(Effect::Schism)
            {
                continue;
            }
            let followers: Vec<_> = self
                .living()
                .filter(|&c| self.communities[c].faith == Some(parent))
                .collect();
            if followers.len() < 2 {
                continue;
            }
            let religion = &self.religions[parent];
            let age = self.generation.saturating_sub(religion.founded);
            let centre = self
                .land_holder(religion.shrine.region)
                .and_then(|c| self.state_of(c));
            let mut candidates = Vec::new();
            for &c in &followers {
                let people = &self.communities[c];
                let state = self.state_of(c);
                if age >= 8
                    && self.rules(c).is_some()
                    && centre.is_some()
                    && state != centre
                    && followers.iter().any(|&o| self.state_of(o) != state)
                {
                    candidates.push((c, SchismCause::Rule, 1.0));
                }
                let distant = self.map.regions[people.home()].landmass
                    != self.map.regions[religion.shrine.region].landmass;
                let apart_states = state.zip(centre).is_some_and(|(a, b)| {
                    a != b
                        && self.generation >= self.states[a].rose + 16
                        && self.generation >= self.states[b].rose + 16
                });
                if age >= 16 && (distant || apart_states) {
                    candidates.push((c, SchismCause::Distance, 0.8));
                }
                if age >= 12
                    && !religion.translates
                    && intelligibility(
                        &self.varieties[religion.sacred].lexicon,
                        &self.varieties[people.variety].lexicon,
                    ) < REFORM_SHARING
                {
                    candidates.push((c, SchismCause::Reform, 1.0));
                }
                if religion.parent.is_none() && (1..=6).contains(&age) {
                    candidates.push((c, SchismCause::Succession, 0.12 / followers.len() as f32));
                }
            }
            let total: f32 = candidates.iter().map(|x| x.2).sum();
            if candidates.is_empty() || rng.r#gen::<f32>() >= total.min(1.0) {
                continue;
            }
            let (community, cause, _) =
                candidates[weighted_index(&mut rng, candidates.iter().map(|x| x.2))];
            self.schism(community, cause);
        }
    }

    /// Found a branch among adherents of an existing faith. Used by the
    /// endogenous process; explicit calls can study one cause in isolation.
    pub fn schism(&mut self, community: usize, cause: SchismCause) -> Option<usize> {
        let people = self.communities.get(community)?;
        if !people.living() {
            return None;
        }
        let parent = people.faith?;
        let state = self.state_of(community);
        if cause == SchismCause::Rule && self.rules(community).is_none() {
            return None;
        }
        if cause == SchismCause::Succession
            && (self.religions[parent].parent.is_some()
                || !(1..=6).contains(
                    &self
                        .generation
                        .saturating_sub(self.religions[parent].founded),
                ))
        {
            return None;
        }
        self.refresh_places();
        let id = self.religions.len();
        let mut rng = stream(
            self.seed,
            &[
                key("schism"),
                key("found"),
                id as u64,
                community as u64,
                u64::from(self.generation),
            ],
        );
        let (name, named) = self.branch_name(community, parent, cause, &mut rng);
        let old = &self.religions[parent];
        let mut branch = Religion {
            name,
            name_variety: self.communities[community].variety,
            founder: old.founder.clone(),
            people: community,
            land: self.communities[community].home(),
            founded: self.generation,
            how: old.how,
            sacred: old.sacred,
            shrine: old.shrine.clone(),
            extra_shrines: old.extra_shrines.clone(),
            parent: Some(parent),
            split: Some(self.generation),
            cause: Some(cause),
            named: Some(named),
            converts: old.converts,
            translates: old.translates || cause == SchismCause::Reform,
            scripture: old.scripture,
            pilgrims: Vec::new(),
            pilgrim_landmasses: Vec::new(),
            holy_land: Vec::new(),
        };
        if cause == SchismCause::Distance && rng.r#gen::<f32>() < 0.7 {
            let shrine = self.sacred_place(community, true, &mut rng);
            if !branch.shrines().any(|s| s.region == shrine.region) {
                branch.extra_shrines.push(shrine);
            }
        }
        self.religions.push(branch);
        let followers: Vec<_> = self
            .living()
            .filter(|&c| self.communities[c].faith == Some(parent))
            .collect();
        let v = self.communities[community].variety;
        let joins: Vec<_> = followers
            .into_iter()
            .filter(|&c| {
                if c == community {
                    return true;
                }
                if cause == SchismCause::Rule {
                    return self.state_of(c) == state;
                }
                let contact = self
                    .partners(c)
                    .find(|&(o, _, _)| o == community)
                    .map_or(0.0, |(_, i, _)| i);
                let kin = self.kinship(v, self.communities[c].variety);
                let chance = (contact * (0.25 + 0.75 * kin)).min(0.85);
                let mut joining = stream(
                    self.seed,
                    &[key("schism"), key("join"), id as u64, c as u64],
                );
                joining.r#gen::<f32>() < chance
            })
            .collect();
        self.events.push((
            self.generation,
            WorldEvent::Schism {
                religion: id,
                parent,
                community,
                cause,
            },
        ));
        for c in joins {
            let mut conversion = stream(
                self.seed,
                &[key("schism"), key("conversion"), id as u64, c as u64],
            );
            self.convert_with_rng(c, id, Some(community), &mut conversion);
        }
        self.religions[id].holy_land = self.holy_lands(id);
        Some(id)
    }

    fn branch_name(
        &self,
        community: usize,
        parent: usize,
        cause: SchismCause,
        rng: &mut impl Rng,
    ) -> (Name, BranchNaming) {
        let people = &self.communities[community];
        let variety = &self.varieties[people.variety];
        let named = [
            BranchNaming::Leader,
            BranchNaming::Land,
            BranchNaming::Epithet,
        ][index(rng, 3)];
        let name = match named {
            BranchNaming::Leader => {
                let leader = if variety.given.is_empty() {
                    given_name(
                        variety,
                        people.livelihood,
                        true,
                        people.ethos,
                        rng,
                        self.generation,
                    )
                    .expect("a living language has words for a given name")
                } else {
                    variety.given[index(rng, variety.given.len())].name.clone()
                };
                Name {
                    form: clipped(
                        variety.morphology.belonging(&leader.form),
                        MAX_PEOPLE_NAME + 2,
                    ),
                    meaning: format!("the followers of {}", variety.title(&leader.form)),
                    coined: self.generation,
                    log: Vec::new(),
                }
            }
            BranchNaming::Land => {
                let land = self
                    .known_place(people.variety, people.home())
                    .expect("held land is known");
                Naming::Land
                    .coin(
                        variety,
                        Some((land, &variety.title(&land.form))),
                        self.generation,
                    )
                    .expect("a named homeland")
            }
            BranchNaming::Epithet => {
                let old = &self.religions[parent].name;
                let base = Name {
                    form: self.ear(people.variety).adapt(&old.form, 0.0, rng),
                    ..old.clone()
                };
                let epithet = if cause == SchismCause::Reform {
                    "new"
                } else {
                    "old"
                };
                Naming::Epithet {
                    epithet: epithet.into(),
                }
                .coin(
                    variety,
                    Some((&base, &self.faith_name(parent))),
                    self.generation,
                )
                .expect("old and new are core words")
            }
        };
        (name, named)
    }

    /// Cheapest permitted route. The precomputed all-pairs distances both
    /// gate reach and reconstruct the usual path. A land-only detour is
    /// needed only when the unconstrained cheapest path takes a sea shortcut.
    pub fn pilgrim_path(&self, community: usize, to: usize) -> Option<Vec<usize>> {
        let from = self.communities[community].home();
        if self.map.distance(from, to) > PILGRIM_REACH {
            return None;
        }
        let sails = self.sails(community);
        if !sails && self.map.regions[from].landmass != self.map.regions[to].landmass {
            return None;
        }
        let mut path = vec![from];
        let mut at = from;
        while at != to {
            let next = self.map.regions[at]
                .neighbours
                .iter()
                .copied()
                .min_by(|&a, &b| {
                    let cost = |n: usize| {
                        (self.map.regions[at].terrain.travel()
                            + self.map.regions[n].terrain.travel())
                            / 2.0
                            + self.map.distance(n, to)
                    };
                    cost(a).total_cmp(&cost(b)).then(a.cmp(&b))
                })?;
            if !sails && !self.map.regions[next].terrain.is_land() {
                return self.pilgrim_land_path(from, to);
            }
            path.push(next);
            at = next;
        }
        Some(path)
    }

    fn pilgrim_land_path(&self, from: usize, to: usize) -> Option<Vec<usize>> {
        let n = self.map.regions.len();
        let mut distances = vec![f32::INFINITY; n];
        let mut previous = vec![None; n];
        let mut visited = vec![false; n];
        distances[from] = 0.0;
        loop {
            let at = (0..n)
                .filter(|&r| !visited[r])
                .min_by(|&a, &b| distances[a].total_cmp(&distances[b]).then(a.cmp(&b)))?;
            if distances[at] > PILGRIM_REACH {
                return None;
            }
            if at == to {
                break;
            }
            visited[at] = true;
            for &next in &self.map.regions[at].neighbours {
                if !self.map.regions[next].terrain.is_land() || visited[next] {
                    continue;
                }
                let cost = distances[at]
                    + (self.map.regions[at].terrain.travel()
                        + self.map.regions[next].terrain.travel())
                        / 2.0;
                if cost < distances[next] {
                    distances[next] = cost;
                    previous[next] = Some(at);
                }
            }
        }
        let mut path = vec![to];
        let mut at = to;
        while at != from {
            at = previous[at]?;
            path.push(at);
        }
        path.reverse();
        Some(path)
    }

    pub(crate) fn send_pilgrims(&mut self) {
        if self.params.pilgrimage_rate <= 0.0 {
            return;
        }
        for religion in 0..self.religions.len() {
            let mut routes = std::mem::take(&mut self.religions[religion].pilgrims);
            routes.retain(|p| {
                self.communities[p.people].living()
                    && self.communities[p.people].faith == Some(religion)
                    && self.communities[p.people].home() == p.from
            });
            let shrines: Vec<_> = self.religions[religion]
                .shrines()
                .map(|s| s.region)
                .collect();
            for c in self
                .living()
                .filter(|&c| self.communities[c].faith == Some(religion))
            {
                let from = self.communities[c].home();
                for &to in &shrines {
                    if from == to || routes.iter().any(|p| p.people == c && p.to == to) {
                        continue;
                    }
                    let mut rng = stream(
                        self.seed,
                        &[
                            key("pilgrimage"),
                            u64::from(self.generation),
                            religion as u64,
                            c as u64,
                            to as u64,
                        ],
                    );
                    if rng.r#gen::<f32>()
                        >= self.params.pilgrimage_rate
                            * self.communities[c].ethos.factor(Effect::Pilgrimage)
                    {
                        continue;
                    }
                    if let Some(path) = self.pilgrim_path(c, to) {
                        routes.push(Pilgrimage {
                            people: c,
                            from,
                            to,
                            path,
                            since: self.generation,
                        });
                    }
                }
            }
            for route in &routes {
                let landmass = self.map.regions[route.from]
                    .landmass
                    .expect("people live on land");
                if self.map.regions[route.to].landmass != Some(landmass)
                    && !self.religions[religion]
                        .pilgrim_landmasses
                        .contains(&landmass)
                {
                    self.religions[religion].pilgrim_landmasses.push(landmass);
                    self.events.push((
                        self.generation,
                        WorldEvent::Pilgrimage {
                            religion,
                            community: route.people,
                            landmass,
                            from: route.from,
                            to: route.to,
                        },
                    ));
                }
                let Some(holder) = self.land_holder(route.to) else {
                    continue;
                };
                if holder == route.people {
                    continue;
                }
                let mut rng = stream(
                    self.seed,
                    &[
                        key("pilgrimage"),
                        key("contact"),
                        religion as u64,
                        route.people as u64,
                        holder as u64,
                        route.to as u64,
                    ],
                );
                let intensity = rng.gen_range(0.25..0.4);
                if let Some(contact) = self.contacts.iter_mut().find(|k| {
                    (k.a, k.b) == (route.people, holder) || (k.b, k.a) == (route.people, holder)
                }) {
                    // Preserve rule and other established dealings, and never
                    // reset their age: ordinary contact turnover still applies.
                    contact.intensity = contact.intensity.max(intensity);
                } else {
                    self.connect(route.people, holder, intensity, ContactKind::Religion);
                }
            }
            self.religions[religion].pilgrims = routes;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Craft, Livelihood, MapSize, Params, Revelation, SoundProfile};

    fn settled(world: &mut World, region: usize) -> usize {
        world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            world.communities.len() as u64 + 50,
            0.5,
            0.5,
            Some(region),
            Some(Livelihood::Farming),
            None,
        )
    }

    fn faith_world() -> (World, usize, usize, usize) {
        let mut world = World::new(8, Params::static_society());
        let home = world.map.landmasses[0].anchor;
        let other = world.map.regions[home]
            .neighbours
            .iter()
            .copied()
            .find(|&r| world.map.regions[r].terrain.is_land())
            .unwrap();
        let founder = settled(&mut world, home);
        let follower = settled(&mut world, other);
        let religion = world.found_religion(founder, Revelation::Proclaimed);
        // A known local shrine makes the geographic boundary explicit.
        world.religions[religion].shrine.region = home;
        world.religions[religion].shrine.name = world
            .known_place(world.communities[founder].variety, home)
            .unwrap()
            .clone();
        world.convert(follower, religion, None);
        world.religions[religion].holy_land = world.holy_lands(religion);
        (world, founder, follower, religion)
    }

    #[test]
    fn rule_schism_takes_the_whole_state_but_not_outsiders_or_other_faiths() {
        let (mut world, founder, ruler, parent) = faith_world();
        let home = world.communities[ruler].home();
        let subject = settled(&mut world, home);
        let outside = world.communities[founder].home();
        let outsider = settled(&mut world, outside);
        let other_faith = settled(&mut world, home);
        world.convert(subject, parent, None);
        world.convert(outsider, parent, None);
        world.connect(ruler, subject, 0.6, ContactKind::Rule);
        world.connect(ruler, other_faith, 0.6, ContactKind::Rule);
        let branch = world.schism(ruler, SchismCause::Rule).unwrap();
        assert_eq!(world.communities[ruler].faith, Some(branch));
        assert_eq!(world.communities[subject].faith, Some(branch));
        assert_eq!(world.communities[founder].faith, Some(parent));
        assert_eq!(world.communities[outsider].faith, Some(parent));
        assert_eq!(world.communities[other_faith].faith, None);
        assert_eq!(
            world.religions[branch].founder,
            world.religions[parent].founder
        );
        assert_eq!(
            world.religions[branch].shrine,
            world.religions[parent].shrine
        );
    }

    #[test]
    fn reform_translates_and_writes_the_vernacular_without_replacing_sacred_speech() {
        let (mut world, _, follower, parent) = faith_world();
        world.religions[parent].translates = false;
        world.religions[parent].scripture = true;
        let v = world.communities[follower].variety;
        world.varieties[v].high = Some(world.religions[parent].sacred);
        let branch = world.schism(follower, SchismCause::Reform).unwrap();
        assert!(world.religions[branch].translates);
        assert!(!world.religions[parent].translates);
        assert_eq!(
            world.religions[branch].sacred,
            world.religions[parent].sacred
        );
        assert!(world.varieties[v].vernacular.is_some());
        assert_eq!(world.religions[branch].name_variety, v);
    }

    #[test]
    fn pilgrim_contacts_age_and_fade_after_conversion() {
        let (mut world, founder, follower, religion) = faith_world();
        world.params.pilgrimage_rate = 1.0;
        world.send_pilgrims();
        let contact = *world
            .contacts
            .iter()
            .find(|c| c.kind == ContactKind::Religion)
            .unwrap();
        assert_eq!((contact.a, contact.b), (follower, founder));
        world.generation = 7;
        world.send_pilgrims();
        assert_eq!(world.contacts[0].since, contact.since);
        assert_eq!(world.religions[religion].pilgrims[0].since, 0);
        let rival = world.found_religion(follower, Revelation::Proclaimed);
        assert_ne!(rival, religion);
        world.religions[rival].shrine.region = world.communities[follower].home();
        world.send_pilgrims();
        assert!(world.religions[religion].pilgrims.is_empty());
        // Turnover certain once settled: the contact was not made immortal
        // by repeated pilgrimages, and a rival with a shrine at home sends none.
        world.params.contact_turnover = 100.0;
        world.run(15);
        assert!(
            !world
                .contacts
                .iter()
                .any(|c| (c.a, c.b) == (follower, founder) || (c.b, c.a) == (follower, founder))
        );
    }

    #[test]
    fn overseas_pilgrims_need_seafaring_and_are_announced_once_per_landmass() {
        let mut world = World::with_map(3, Params::static_society(), MapSize::Large);
        let (home, remote) = world
            .map
            .regions
            .iter()
            .enumerate()
            .find_map(|(a, r)| {
                if !r.terrain.is_land() {
                    return None;
                }
                world.map.regions.iter().enumerate().find_map(|(b, s)| {
                    (s.terrain.is_land()
                        && r.landmass != s.landmass
                        && world.map.distance(a, b) < PILGRIM_REACH)
                        .then_some((a, b))
                })
            })
            .unwrap();
        let founder = settled(&mut world, home);
        let follower = settled(&mut world, remote);
        let religion = world.found_religion(founder, Revelation::Proclaimed);
        world.religions[religion].shrine.region = home;
        world.convert(follower, religion, None);
        world.params.pilgrimage_rate = 1.0;
        world.send_pilgrims();
        assert!(world.religions[religion].pilgrims.is_empty());
        assert!(world.contacts.is_empty());
        world.learn(follower, Craft::Seafaring, None);
        world.send_pilgrims();
        let route = &world.religions[religion].pilgrims[0];
        assert_eq!(route.path.first(), Some(&remote));
        assert_eq!(route.path.last(), Some(&home));
        let mut cost = 0.0;
        for pair in route.path.windows(2) {
            assert!(world.map.regions[pair[0]].neighbours.contains(&pair[1]));
            cost += (world.map.regions[pair[0]].terrain.travel()
                + world.map.regions[pair[1]].terrain.travel())
                / 2.0;
        }
        assert!((cost - world.map.distance(remote, home)).abs() < 0.0001);
        world.send_pilgrims();
        world.religions[religion].pilgrims.clear();
        world.send_pilgrims();
        assert_eq!(world.events.iter().filter(|(_, e)| matches!(e, WorldEvent::Pilgrimage { religion: f, .. } if *f == religion)).count(), 1);
    }

    #[test]
    fn holy_lands_record_loss_and_recovery_not_changes_between_unfaithful_holders() {
        let (mut world, founder, follower, religion) = faith_world();
        world.params.pilgrimage_rate = 1.0;
        world.communities[founder].faith = None;
        world.observe_holy_lands();
        let home = world.communities[founder].home();
        world.communities[follower].lands = vec![home];
        world.communities[follower].size = world.communities[founder].size * 2.0;
        world.communities[follower].faith = None;
        world.observe_holy_lands();
        world.convert(follower, religion, None);
        world.observe_holy_lands();
        let changes: Vec<_> = world
            .events
            .iter()
            .filter_map(|(_, e)| match e {
                WorldEvent::HolyLand {
                    religion: r,
                    faithful,
                    held_by,
                    ..
                } if *r == religion => Some((*faithful, *held_by)),
                _ => None,
            })
            .collect();
        assert_eq!(
            changes,
            vec![(false, Some(founder)), (true, Some(follower))]
        );
    }

    #[test]
    fn holy_war_pressure_excludes_cofaithful_and_includes_an_inherited_shrine() {
        let (mut world, founder, follower, religion) = faith_world();
        world.params.conquest_rate = 0.05;
        assert!(!world.holy_war_target(follower, founder));
        world.communities[founder].faith = None;
        assert!(world.holy_war_target(follower, founder));
        let branch = world.schism(follower, SchismCause::Reform).unwrap();
        assert_eq!(world.religions[branch].parent, Some(religion));
        assert!(world.holy_war_target(follower, founder));
        world.convert(founder, branch, None);
        assert!(!world.holy_war_target(follower, founder));
    }

    #[test]
    fn succession_is_limited_to_the_original_founders_first_six_generations() {
        let (mut world, founder, follower, _) = faith_world();
        assert_eq!(world.schism(follower, SchismCause::Succession), None);
        world.generation = 6;
        let branch = world.schism(follower, SchismCause::Succession).unwrap();
        world.generation = 7;
        assert_eq!(world.schism(founder, SchismCause::Succession), None);
        assert_eq!(world.schism(follower, SchismCause::Succession), None);
        assert_eq!(world.religions[branch].cause, Some(SchismCause::Succession));
    }

    #[test]
    fn static_society_never_divides_or_sends_pilgrims() {
        let (mut world, _, _, religion) = faith_world();
        world.run(24);
        assert_eq!(world.religions.len(), 1);
        assert!(world.religions[religion].pilgrims.is_empty());
        assert!(!world.events.iter().any(|(_, e)| matches!(
            e,
            WorldEvent::Schism { .. } | WorldEvent::Pilgrimage { .. } | WorldEvent::HolyLand { .. }
        )));
    }

    #[test]
    fn faiths_routes_and_language_replay_identically() {
        let replay = || {
            let (mut world, _, follower, _) = faith_world();
            world.params.schism_rate = 1.0;
            world.params.pilgrimage_rate = 1.0;
            world.generation = 16;
            world.schism(follower, SchismCause::Reform);
            world.run(12);
            world
        };
        let (a, b) = (replay(), replay());
        assert_eq!(a.religions, b.religions);
        assert_eq!(a.communities, b.communities);
        assert_eq!(a.contacts, b.contacts);
        assert_eq!(a.events, b.events);
        assert_eq!(a.varieties.len(), b.varieties.len());
        for (a, b) in a.varieties.iter().zip(&b.varieties) {
            assert_eq!(a.lexicon, b.lexicon);
            assert_eq!(a.name, b.name);
            assert_eq!(a.given, b.given);
            assert_eq!(a.exonyms, b.exonyms);
            assert_eq!(a.laws, b.laws);
            assert_eq!(a.waves, b.waves);
        }
    }
}

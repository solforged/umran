//! Living hydronyms use the same records and memories as land names, but
//! their identities and holders follow a whole lake, not one shore region.

use crate::adapt::Adapter;
use crate::names::{Name, PlaceName, PlaceOrigin, lake_name};
use crate::rng::{key, stream};
use crate::world::{PLACE_HOLD, PLACE_KEEP_KNOWN, PLACE_KEEP_UNKNOWN};
use crate::{Variety, World};
use rand::Rng;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

impl World {
    /// The lake's local naming record at `generation`. Its spoken form at
    /// that date is `record.name.form_at(generation)`, not its current form.
    pub fn lake_name_at(&self, lake: usize, generation: u32) -> Option<&PlaceName> {
        self.lake_names
            .get(lake)?
            .iter()
            .rev()
            .find(|p| p.since <= generation)
    }

    /// This language's own remembered hydronym, never another language's
    /// latest name substituted for a surviving local alternative.
    pub fn known_lake(&self, variety: usize, lake: usize) -> Option<&Name> {
        self.varieties[variety]
            .lake_exonyms
            .iter()
            .find(|(r, _)| *r == lake)
            .map(|(_, name)| name)
            .or_else(|| {
                self.lake_names[lake]
                    .iter()
                    .rev()
                    .find(|p| p.variety == variety)
                    .map(|p| &p.name)
            })
    }

    /// Forks remember their parent's local hydronyms in addition to the
    /// exonyms already cloned by `Variety::fork`.
    pub fn inherit_lake_names(&self, parent: usize, daughter: &mut Variety) {
        for (lake, names) in self.lake_names.iter().enumerate() {
            if !daughter.lake_exonyms.iter().any(|(r, _)| *r == lake)
                && let Some(place) = names.iter().rev().find(|p| p.variety == parent)
            {
                daughter.lake_exonyms.push((lake, place.name.clone()));
            }
        }
    }

    fn remember_lake_name(&mut self, lake: usize) {
        if let Some(place) = self.lake_names[lake].last()
            && !self.varieties[place.variety]
                .lake_exonyms
                .iter()
                .any(|(r, _)| *r == lake)
        {
            self.varieties[place.variety]
                .lake_exonyms
                .push((lake, place.name.clone()));
        }
    }

    /// Capture local forms before absent speakers undergo further sound
    /// laws. The deserted attestation freezes; their remembered word lives.
    pub fn preserve_lake_names(&mut self) {
        let dwellers = self.dwellers();
        self.preserve_lake_names_indexed(&dwellers);
    }

    pub(crate) fn preserve_lake_names_indexed(&mut self, dwellers: &[Vec<(usize, f32)>]) {
        for lake in 0..self.lake_names.len() {
            if self.lake_names[lake].last().is_some_and(|p| {
                !self.map.lakes[lake].regions.iter().any(|&region| {
                    dwellers[region]
                        .iter()
                        .any(|&(c, _)| self.communities[c].variety == p.variety)
                })
            }) {
                self.remember_lake_name(lake);
            }
        }
    }

    pub fn refresh_lake_names(&mut self) {
        let dwellers = self.dwellers();
        self.refresh_lake_names_indexed(&dwellers);
    }

    pub(crate) fn refresh_lake_names_indexed(&mut self, dwellers: &[Vec<(usize, f32)>]) {
        if self.map.lakes.is_empty() {
            return;
        }
        self.preserve_lake_names_indexed(dwellers);
        for lake in 0..self.map.lakes.len() {
            // All shores count by their actual local populations.
            let mut holders = BTreeMap::<usize, f32>::new();
            for &region in &self.map.lakes[lake].regions {
                for &(community, size) in &dwellers[region] {
                    *holders.entry(community).or_default() += size;
                }
            }
            let largest = holders.iter().fold(None, |best, (&c, &size)| match best {
                Some((_, n)) if n >= size => best,
                _ => Some((c, size)),
            });
            let Some((holder, size)) = largest else {
                continue;
            };
            let variety = self.communities[holder].variety;
            let before = self.lake_names[lake].last();
            if before.is_some_and(|p| p.variety == variety) {
                continue;
            }
            let mut rng = stream(
                self.seed,
                &[
                    key("lake succession"),
                    lake as u64,
                    variety as u64,
                    u64::from(self.generation),
                ],
            );
            let origin = match before {
                Some(p) => {
                    let namers: f32 = holders
                        .iter()
                        .filter(|(c, _)| self.communities[**c].variety == p.variety)
                        .map(|(_, n)| *n)
                        .sum();
                    if size < namers * PLACE_HOLD {
                        continue;
                    }
                    if self.descends(variety, p.variety) {
                        Some(PlaceOrigin::Inherited)
                    } else if self.known_lake(variety, lake).is_some() {
                        Some(PlaceOrigin::Borrowed)
                    } else {
                        let knew = namers > 0.0
                            || self.contacts.iter().any(|k| {
                                (k.a == holder && self.communities[k.b].variety == p.variety)
                                    || (k.b == holder && self.communities[k.a].variety == p.variety)
                            });
                        let keep = if knew {
                            PLACE_KEEP_KNOWN
                        } else {
                            PLACE_KEEP_UNKNOWN
                        };
                        (rng.r#gen::<f32>() < keep).then_some(PlaceOrigin::Borrowed)
                    }
                }
                None => None,
            };
            let name = if let Some(known) = self.known_lake(variety, lake) {
                known.clone()
            } else if let (Some(origin), Some(p)) = (origin, before) {
                if origin == PlaceOrigin::Inherited {
                    p.name.clone()
                } else {
                    let ear = self.ear(variety);
                    Name {
                        form: ear.adapt(&p.name.form, 0.0, &mut rng),
                        meaning: p.name.meaning.clone(),
                        coined: self.generation,
                        log: Vec::new(),
                    }
                }
            } else {
                let mut naming =
                    stream(self.seed, &[key("lake name"), lake as u64, variety as u64]);
                let Some(name) = lake_name(&self.varieties[variety], &mut naming, self.generation)
                else {
                    continue;
                };
                name
            };
            self.remember_lake_name(lake);
            self.lake_names[lake].push(PlaceName {
                variety,
                since: self.generation,
                name,
                origin: origin.unwrap_or(PlaceOrigin::Coined { community: holder }),
            });
        }
        self.hear_lake_names();
    }

    fn hear_lake_names(&mut self) {
        let mut known = HashSet::new();
        for (variety, speech) in self.varieties.iter().enumerate() {
            known.extend(speech.lake_exonyms.iter().map(|(lake, _)| (variety, *lake)));
        }
        for (lake, names) in self.lake_names.iter().enumerate() {
            known.extend(names.iter().map(|p| (p.variety, lake)));
        }
        let mut ears = HashMap::new();
        let mut heard = BTreeMap::new();
        for c in self.living() {
            let community = &self.communities[c];
            let variety = community.variety;
            let nearby: BTreeSet<usize> = community
                .lands
                .iter()
                .flat_map(|&land| {
                    std::iter::once(land).chain(self.map.regions[land].neighbours.iter().copied())
                })
                .filter_map(|land| self.map.lake_regions[land])
                .collect();
            for lake in nearby {
                if known.contains(&(variety, lake)) || heard.contains_key(&(variety, lake)) {
                    continue;
                }
                let Some(place) = self.lake_names[lake].last() else {
                    continue;
                };
                let mut rng = stream(
                    self.seed,
                    &[key("lake exonym"), lake as u64, variety as u64],
                );
                let name =
                    self.place_heard(variety, place.variety, &place.name, &mut rng, &mut ears);
                heard.insert((variety, lake), name);
            }
        }
        for ((variety, lake), name) in heard {
            self.varieties[variety].lake_exonyms.push((lake, name));
        }
    }

    /// Regular laws affect locally spoken names and every living memory,
    /// never the lake's superseded or deserted local attestations.
    pub(crate) fn change_lake_names(
        &mut self,
        variety: usize,
        held: &[usize],
        change: &mut impl FnMut(&mut Name),
    ) {
        let mut lakes: Vec<usize> = held
            .iter()
            .filter_map(|&r| self.map.lake_regions[r])
            .collect();
        lakes.sort_unstable();
        lakes.dedup();
        for lake in lakes {
            if let Some(place) = self.lake_names[lake]
                .last_mut()
                .filter(|p| p.variety == variety)
            {
                change(&mut place.name);
            }
        }
        for (_, name) in &mut self.varieties[variety].lake_exonyms {
            change(name);
        }
    }

    /// Shifters keep their own hydronyms rather than adopting the target's
    /// competing exonyms. Fit the old forms to the new speech's actual sounds.
    pub(crate) fn shift_lake_names(&self, community: usize, old: usize, new: &mut Variety) {
        let mut ear = None;
        for lake in 0..self.lake_names.len() {
            let Some(name) = self.known_lake(old, lake) else {
                continue;
            };
            let adapter = ear.get_or_insert_with(|| {
                Adapter::new(
                    new.lexicon.living().map(|l| &l.form),
                    &new.profile.inventory,
                )
            });
            let mut rng = stream(
                self.seed,
                &[
                    key("lake memory shift"),
                    community as u64,
                    lake as u64,
                    u64::from(self.generation),
                ],
            );
            let kept = Name {
                form: adapter.adapt(&name.form, 0.0, &mut rng),
                meaning: name.meaning.clone(),
                coined: self.generation,
                log: Vec::new(),
            };
            if let Some((_, memory)) = new.lake_exonyms.iter_mut().find(|(r, _)| *r == lake) {
                *memory = kept;
            } else {
                new.lake_exonyms.push((lake, kept));
            }
        }
    }

    /// Once the community speaks its new variety, record the local handover
    /// separately from the earlier speakers' frozen attestation.
    pub(crate) fn keep_lake_names(&mut self, community: usize, old: usize) {
        let variety = self.communities[community].variety;
        let mut lakes: Vec<usize> = self.communities[community]
            .lands
            .iter()
            .filter_map(|&r| self.map.lake_regions[r])
            .collect();
        lakes.sort_unstable();
        lakes.dedup();
        for lake in lakes {
            if !self.lake_names[lake]
                .last()
                .is_some_and(|p| p.variety == old)
            {
                continue;
            }
            let name = self.known_lake(variety, lake).unwrap().clone();
            self.remember_lake_name(lake);
            self.lake_names[lake].push(PlaceName {
                variety,
                since: self.generation,
                name,
                origin: PlaceOrigin::Kept,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lakes::Lake;
    use crate::{Env, Form, Law, Matcher, Params, Rewrite, SoundChange, SoundProfile};
    use std::sync::Arc;

    #[test]
    fn remembered_lake_names_survive_shift_and_desertion_but_follow_living_sound_laws() {
        let mut world = World::new(5, Params::static_society());
        world.found(&SoundProfile::by_id("familiar").unwrap(), 0.2, 0.5);
        world.found(&SoundProfile::by_id("polynesian").unwrap(), 0.9, 0.5);
        let bank = world
            .map
            .regions
            .iter()
            .position(|r| r.terrain.is_land())
            .unwrap();
        let away = world
            .map
            .regions
            .iter()
            .enumerate()
            .find(|&(r, land)| {
                land.terrain.is_land() && r != bank && !land.neighbours.contains(&bank)
            })
            .unwrap()
            .0;
        let map = Arc::make_mut(&mut world.map);
        map.lakes = vec![Lake {
            regions: vec![bank],
            surface: 0.4,
            outlet: None,
            spill: None,
        }];
        map.lake_regions.fill(None);
        map.lake_regions[bank] = Some(0);
        world.lake_names = vec![Vec::new()];
        for variety in &mut world.varieties {
            variety.lake_exonyms.clear();
        }
        world.communities[0].lands = vec![bank];
        world.communities[1].lands = vec![away];
        world.generation = 1;
        world.refresh_lake_names();
        let old = world.communities[1].variety;
        let local = world.lake_name_at(0, 1).unwrap().name.clone();
        assert_eq!(
            world.lake_name_at(0, 1).unwrap().origin,
            PlaceOrigin::Coined { community: 0 }
        );
        assert!(world.lake_name_at(0, 0).is_none());
        world.communities[1].lands = vec![bank];
        world.communities[1].size = world.communities[0].size * 0.5;
        world.refresh_lake_names();
        let heard = world.known_lake(old, 0).unwrap().clone();
        world.communities[1].size *= 8.0;
        world.generation = 2;
        world.refresh_lake_names();
        assert_eq!(
            world.lake_name_at(0, 2).unwrap().origin,
            PlaceOrigin::Borrowed
        );
        assert_eq!(world.lake_name_at(0, 2).unwrap().name, heard);
        world.communities[0].ended = Some(2);
        let rulers = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.9, 0.5);
        world.communities[rulers].lands = vec![away];
        let target = world.communities[rulers].variety;
        let mut rival = world.varieties[target].name.clone();
        rival.meaning = "rival water".into();
        world.varieties[target]
            .lake_exonyms
            .retain(|(l, _)| *l != 0);
        world.varieties[target].lake_exonyms.push((0, rival));
        world.generation = 3;
        let shifted = world.shift(1, rulers);
        let kept = world.lake_name_at(0, 3).unwrap();
        assert_eq!((kept.variety, kept.origin), (shifted, PlaceOrigin::Kept));
        assert_eq!(kept.name.meaning, local.meaning);
        assert_eq!(world.known_lake(target, 0).unwrap().meaning, "rival water");
        assert_eq!(world.lake_name_at(0, 2).unwrap().name, heard);
        let before = kept.name.form.clone();
        let target = if before.segs[0].phone == Form::from_ipa("a").unwrap().segs[0].phone {
            Form::from_ipa("i").unwrap()
        } else {
            Form::from_ipa("a").unwrap()
        };
        let law = Law {
            id: "test-lake-merger",
            label: "Lake merger",
            rules: vec![SoundChange {
                id: "test-lake-merger".into(),
                target: Matcher::Phone(before.segs[0].phone),
                result: Rewrite::Phone(target.segs[0].phone),
                left: Env::Any,
                right: Env::Any,
            }],
            commonness: 1.0,
            stress: None,
        };
        let speech = &world.varieties[shifted];
        let expected = law.apply(&before, speech.minimal, speech.stress());
        assert_ne!(expected, before);
        world.generation = 4;
        world.apply_law(shifted, &law);
        assert_eq!(world.lake_name_at(0, 4).unwrap().name.form, expected);
        assert_eq!(world.known_lake(shifted, 0).unwrap().form, expected);
        assert_eq!(world.lake_name_at(0, 3).unwrap().name.form_at(3), &before);
        assert_eq!(world.lake_name_at(0, 1).unwrap().name, local);
        let daughter = world.split(1, None, 0.0);
        assert_eq!(
            world
                .known_lake(world.communities[daughter].variety, 0)
                .unwrap()
                .form,
            expected
        );
        world.communities[1].lands = vec![away];
        world.communities[daughter].lands = vec![away];
        world.refresh_lake_names();
        let frozen = world.lake_name_at(0, 4).unwrap().name.clone();
        world.generation = 5;
        let reverse = Law {
            rules: vec![SoundChange {
                id: "test-return".into(),
                target: Matcher::Phone(target.segs[0].phone),
                result: Rewrite::Phone(before.segs[0].phone),
                left: Env::Any,
                right: Env::Any,
            }],
            ..law
        };
        world.apply_law(shifted, &reverse);
        assert_eq!(world.lake_name_at(0, 5).unwrap().name, frozen);
        assert_ne!(world.known_lake(shifted, 0).unwrap().form, frozen.form);
    }
}

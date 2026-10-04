//! Living hydronyms use the same records and memories as land names, but
//! their identities and holders follow whole river courses, not one region.

use crate::adapt::Adapter;
use crate::names::{Name, PlaceName, PlaceOrigin, river_name};
use crate::rng::{key, stream};
use crate::world::{PLACE_HOLD, PLACE_KEEP_KNOWN, PLACE_KEEP_UNKNOWN};
use crate::{Variety, World};
use rand::Rng;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

impl World {
    /// The river's local naming record at `generation`. Its spoken form at
    /// that date is `record.name.form_at(generation)`, not its current form.
    pub fn river_name_at(&self, river: usize, generation: u32) -> Option<&PlaceName> {
        self.river_names
            .get(river)?
            .iter()
            .rev()
            .find(|p| p.since <= generation)
    }

    /// This language's own remembered hydronym, never another language's
    /// latest name substituted for a surviving local alternative.
    pub fn known_river(&self, variety: usize, river: usize) -> Option<&Name> {
        self.varieties[variety]
            .river_exonyms
            .iter()
            .find(|(r, _)| *r == river)
            .map(|(_, name)| name)
            .or_else(|| {
                self.river_names[river]
                    .iter()
                    .rev()
                    .find(|p| p.variety == variety)
                    .map(|p| &p.name)
            })
    }

    /// Forks remember their parent's local hydronyms in addition to the
    /// exonyms already cloned by `Variety::fork`.
    pub fn inherit_river_names(&self, parent: usize, daughter: &mut Variety) {
        for (river, names) in self.river_names.iter().enumerate() {
            if !daughter.river_exonyms.iter().any(|(r, _)| *r == river)
                && let Some(place) = names.iter().rev().find(|p| p.variety == parent)
            {
                daughter.river_exonyms.push((river, place.name.clone()));
            }
        }
    }

    fn remember_river_name(&mut self, river: usize) {
        if let Some(place) = self.river_names[river].last()
            && !self.varieties[place.variety]
                .river_exonyms
                .iter()
                .any(|(r, _)| *r == river)
        {
            self.varieties[place.variety]
                .river_exonyms
                .push((river, place.name.clone()));
        }
    }

    /// Capture local forms before absent speakers undergo further sound
    /// laws. The deserted attestation freezes; their remembered word lives.
    pub fn preserve_river_names(&mut self) {
        let dwellers = self.dwellers();
        self.preserve_river_names_indexed(&dwellers);
    }

    pub(crate) fn preserve_river_names_indexed(&mut self, dwellers: &[Vec<(usize, f32)>]) {
        for river in 0..self.river_names.len() {
            if self.river_names[river].last().is_some_and(|p| {
                !self.map.rivers[river].course.iter().any(|&region| {
                    dwellers[region]
                        .iter()
                        .any(|&(c, _)| self.communities[c].variety == p.variety)
                })
            }) {
                self.remember_river_name(river);
            }
        }
    }

    pub fn refresh_river_names(&mut self) {
        let dwellers = self.dwellers();
        self.refresh_river_names_indexed(&dwellers);
    }

    pub(crate) fn refresh_river_names_indexed(&mut self, dwellers: &[Vec<(usize, f32)>]) {
        self.preserve_river_names_indexed(dwellers);
        for river in 0..self.map.rivers.len() {
            // A headwater or middle reach counts as much as a mouth: use
            // actual local populations along the entire course.
            let mut holders = BTreeMap::<usize, f32>::new();
            for &region in &self.map.rivers[river].course {
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
            let before = self.river_names[river].last();
            if before.is_some_and(|p| p.variety == variety) {
                continue;
            }
            let mut rng = stream(
                self.seed,
                &[
                    key("river succession"),
                    river as u64,
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
                    } else if self.known_river(variety, river).is_some() {
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
            let name = if let Some(known) = self.known_river(variety, river) {
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
                let mut naming = stream(
                    self.seed,
                    &[key("river name"), river as u64, variety as u64],
                );
                let Some(name) = river_name(&self.varieties[variety], &mut naming, self.generation)
                else {
                    continue;
                };
                name
            };
            self.remember_river_name(river);
            self.river_names[river].push(PlaceName {
                variety,
                since: self.generation,
                name,
                origin: origin.unwrap_or(PlaceOrigin::Coined { community: holder }),
            });
        }
        self.hear_river_names();
    }

    /// A nearby river in this language, for land and departing people names.
    /// Prefer a course through the land over a neighbouring valley.
    pub(crate) fn nearby_river_name(&self, variety: usize, region: usize) -> Option<&Name> {
        std::iter::once(region)
            .chain(self.map.regions[region].neighbours.iter().copied())
            .filter_map(|r| self.map.river_regions[r].map(|river| (r != region, river)))
            .filter_map(|order| self.known_river(variety, order.1).map(|name| (order, name)))
            .min_by_key(|(order, _)| *order)
            .map(|(_, name)| name)
    }

    fn hear_river_names(&mut self) {
        // Joins, not shared sea outlets or mere proximity, define a network.
        // Every tributary keeps its own identity and naming history.
        let networks: Vec<usize> = (0..self.map.rivers.len())
            .map(|river| {
                let mut downstream = river;
                while let Some(next) = self.map.rivers[downstream].joins {
                    downstream = next;
                }
                downstream
            })
            .collect();
        let mut network_rivers = vec![Vec::new(); networks.len()];
        for (river, &network) in networks.iter().enumerate() {
            network_rivers[network].push(river);
        }
        let mut known = HashSet::new();
        for (variety, speech) in self.varieties.iter().enumerate() {
            known.extend(
                speech
                    .river_exonyms
                    .iter()
                    .map(|(river, _)| (variety, *river)),
            );
        }
        for (river, names) in self.river_names.iter().enumerate() {
            known.extend(names.iter().map(|p| (p.variety, river)));
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
                .filter_map(|land| self.map.river_regions[land])
                .map(|river| networks[river])
                .collect();
            for river in nearby
                .iter()
                .flat_map(|&network| network_rivers[network].iter().copied())
            {
                if known.contains(&(variety, river)) || heard.contains_key(&(variety, river)) {
                    continue;
                }
                let Some(place) = self.river_names[river].last() else {
                    continue;
                };
                let mut rng = stream(
                    self.seed,
                    &[key("river exonym"), river as u64, variety as u64],
                );
                let name =
                    self.place_heard(variety, place.variety, &place.name, &mut rng, &mut ears);
                heard.insert((variety, river), name);
            }
        }
        for ((variety, river), name) in heard {
            self.varieties[variety].river_exonyms.push((river, name));
        }
    }

    /// Regular laws affect locally spoken names and every living memory,
    /// never the river's superseded or deserted local attestations.
    pub(crate) fn change_river_names(
        &mut self,
        variety: usize,
        // Shared with productive harmony; callers supply the recorded change.
        held: &[usize],
        change: &mut impl FnMut(&mut crate::Name),
    ) {
        let mut rivers: Vec<usize> = held
            .iter()
            .filter_map(|&r| self.map.river_regions[r])
            .collect();
        rivers.sort_unstable();
        rivers.dedup();
        for river in rivers {
            if let Some(place) = self.river_names[river]
                .last_mut()
                .filter(|p| p.variety == variety)
            {
                change(&mut place.name);
            }
        }
        for (_, name) in &mut self.varieties[variety].river_exonyms {
            change(name);
        }
    }

    /// Shifters keep their own hydronyms rather than adopting the target's
    /// competing exonyms. Fit the old forms to the new speech's actual sounds.
    pub(crate) fn shift_river_names(&self, community: usize, old: usize, new: &mut Variety) {
        let mut ear = None;
        for river in 0..self.river_names.len() {
            let Some(name) = self.known_river(old, river) else {
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
                    key("river memory shift"),
                    community as u64,
                    river as u64,
                    u64::from(self.generation),
                ],
            );
            let kept = Name {
                form: adapter.adapt(&name.form, 0.0, &mut rng),
                meaning: name.meaning.clone(),
                coined: self.generation,
                log: Vec::new(),
            };
            if let Some((_, memory)) = new.river_exonyms.iter_mut().find(|(r, _)| *r == river) {
                *memory = kept;
            } else {
                new.river_exonyms.push((river, kept));
            }
        }
    }

    /// Once the community speaks its new variety, record the local handover
    /// separately from the earlier speakers' frozen attestation.
    pub(crate) fn keep_river_names(&mut self, community: usize, old: usize) {
        let variety = self.communities[community].variety;
        let mut rivers: Vec<usize> = self.communities[community]
            .lands
            .iter()
            .filter_map(|&r| self.map.river_regions[r])
            .collect();
        rivers.sort_unstable();
        rivers.dedup();
        for river in rivers {
            if !self.river_names[river]
                .last()
                .is_some_and(|p| p.variety == old)
            {
                continue;
            }
            let name = self.known_river(variety, river).unwrap().clone();
            self.remember_river_name(river);
            self.river_names[river].push(PlaceName {
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
    use crate::geography::River;
    use crate::{Env, Form, Law, Matcher, Params, Rewrite, SoundChange, SoundProfile};
    use std::sync::Arc;

    // A main course a->b and a tributary c->a. b and c do not border, so
    // hearing across them must follow the declared connection, not proximity.
    fn valley() -> (World, [usize; 4]) {
        let mut world = World::new(5, Params::static_society());
        world.found(&SoundProfile::by_id("familiar").unwrap(), 0.2, 0.5);
        world.found(&SoundProfile::by_id("polynesian").unwrap(), 0.9, 0.5);
        let regions = &world.map.regions;
        let (a, b, c) = regions
            .iter()
            .enumerate()
            .find_map(|(a, region)| {
                if !region.terrain.is_land() {
                    return None;
                }
                region
                    .neighbours
                    .iter()
                    .copied()
                    .filter(|&b| regions[b].terrain.is_land())
                    .find_map(|b| {
                        region
                            .neighbours
                            .iter()
                            .copied()
                            .find(|&c| {
                                c != b
                                    && regions[c].terrain.is_land()
                                    && !regions[b].neighbours.contains(&c)
                            })
                            .map(|c| (a, b, c))
                    })
            })
            .unwrap();
        let away = regions
            .iter()
            .enumerate()
            .find(|(r, land)| {
                land.terrain.is_land()
                    && ![a, b, c].contains(r)
                    && [a, b, c]
                        .iter()
                        .all(|&near| !regions[near].neighbours.contains(r))
            })
            .unwrap()
            .0;
        let mouth = regions.iter().position(|r| !r.terrain.is_land()).unwrap();
        Arc::make_mut(&mut world.map).rivers = vec![
            River {
                course: vec![a, b],
                mouth,
                catchment: vec![a, b, c],
                joins: None,
            },
            River {
                course: vec![c],
                mouth,
                catchment: vec![c],
                joins: Some(0),
            },
        ];
        let map = Arc::make_mut(&mut world.map);
        map.river_regions.fill(None);
        map.river_regions[a] = Some(0);
        map.river_regions[b] = Some(0);
        map.river_regions[c] = Some(1);
        world.river_names = vec![Vec::new(); 2];
        for variety in &mut world.varieties {
            variety.river_exonyms.clear();
        }
        world.communities[0].lands = vec![b];
        world.communities[1].lands = vec![away];
        (world, [a, b, c, away])
    }

    fn merger(from: &Form, to: &Form) -> Law {
        Law {
            id: "test-river-merger",
            label: "River merger",
            rules: vec![SoundChange {
                id: "test-river-merger".into(),
                target: Matcher::Phone(from.segs[0].phone),
                result: Rewrite::Phone(to.segs[0].phone),
                left: Env::Any,
                right: Env::Any,
            }],
            commonness: 1.0,
            stress: None,
        }
    }

    #[test]
    fn a_borrowed_hydronym_survives_shift_over_the_targets_competing_name() {
        let (mut world, [_, bank, _, away]) = valley();
        world.generation = 1;
        world.refresh_river_names();
        let native = world.communities[0].variety;
        let old = world.communities[1].variety;
        let local = world.river_names[0].last().unwrap().name.clone();
        assert_eq!(
            world.river_names[0][0].origin,
            PlaceOrigin::Coined { community: 0 }
        );
        assert!(world.river_name_at(0, 0).is_none());

        // Newcomers hear the native name before they outnumber its coiners.
        world.communities[1].lands = vec![bank];
        world.communities[1].size = world.communities[0].size * 0.5;
        world.refresh_river_names();
        let heard = world.known_river(old, 0).unwrap().clone();
        world.generation = 2;
        world.communities[1].size = world.communities[0].size * 4.0;
        world.refresh_river_names();
        let borrowed = world.river_name_at(0, 2).unwrap();
        assert_eq!(borrowed.origin, PlaceOrigin::Borrowed);
        assert_eq!(borrowed.name, heard);
        assert_eq!(borrowed.name.meaning, local.meaning);
        world.communities[0].ended = Some(2);

        let rulers = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.9, 0.5);
        world.communities[rulers].lands = vec![away];
        let target = world.communities[rulers].variety;
        let mut competing = world.varieties[target].name.clone();
        competing.meaning = "the rulers' river".into();
        world.varieties[target]
            .river_exonyms
            .retain(|(r, _)| *r != 0);
        world.varieties[target].river_exonyms.push((0, competing));
        world.generation = 3;
        let shifted = world.shift(1, rulers);
        let kept = world.river_name_at(0, 3).unwrap();
        assert_eq!((kept.variety, kept.origin), (shifted, PlaceOrigin::Kept));
        assert_eq!(kept.name.meaning, local.meaning);
        assert_eq!(world.known_river(shifted, 0), Some(&kept.name));
        assert_eq!(world.river_name_at(0, 2).unwrap().name, heard);
        assert_eq!(world.river_name_at(0, 1).unwrap().name, local);
        assert_eq!(
            world.known_river(target, 0).unwrap().meaning,
            "the rulers' river"
        );
        assert!(!world.spoken()[native] && !world.spoken()[old]);

        // A constructed regular merger changes both current local usage
        // and memory while leaving both earlier attestations untouched.
        let before = kept.name.form.clone();
        let replacement = if before.segs[0].phone == Form::from_ipa("a").unwrap().segs[0].phone {
            Form::from_ipa("i").unwrap()
        } else {
            Form::from_ipa("a").unwrap()
        };
        let law = merger(&before, &replacement);
        let speech = &world.varieties[shifted];
        let expected = law.apply(&before, speech.minimal, speech.stress());
        assert_ne!(expected, before);
        world.generation = 4;
        world.apply_law(shifted, &law);
        assert_eq!(world.river_name_at(0, 4).unwrap().name.form, expected);
        assert_eq!(world.known_river(shifted, 0).unwrap().form, expected);
        assert_eq!(world.river_name_at(0, 3).unwrap().name.form_at(3), &before);
        assert_eq!(world.river_name_at(0, 1).unwrap().name, local);

        let daughter = world.split(1, None, 0.0);
        let fork = world.communities[daughter].variety;
        assert_eq!(world.known_river(fork, 0).unwrap().form, expected);
        world.communities[1].lands = vec![away];
        world.communities[daughter].lands = vec![away];
        world.refresh_river_names();
        let frozen = world.river_name_at(0, 4).unwrap().name.clone();
        let later = merger(&replacement, &Form::from_ipa("u").unwrap());
        world.generation = 5;
        world.apply_law(shifted, &later);
        assert_eq!(world.river_name_at(0, 5).unwrap().name, frozen);
        assert_ne!(world.known_river(shifted, 0).unwrap().form, frozen.form);
    }

    #[test]
    fn course_holders_hear_connected_tributaries_without_erasing_local_forms() {
        let (mut world, [a, b, c, _]) = valley();
        // Neither speaker is at the first region of the main course.
        world.communities[0].lands = vec![b];
        world.communities[1].lands = vec![c];
        world.refresh_river_names();
        let main = world.communities[0].variety;
        let upstream = world.communities[1].variety;
        assert_eq!(world.river_names[0][0].variety, main);
        assert_eq!(world.river_names[1][0].variety, upstream);
        let main_local = world.known_river(main, 0).unwrap().clone();
        let tributary_local = world.known_river(upstream, 1).unwrap().clone();
        let upstream_heard = world.known_river(upstream, 0).unwrap().clone();
        assert_eq!(upstream_heard.meaning, main_local.meaning);
        assert_eq!(
            world.known_river(main, 1).unwrap().meaning,
            tributary_local.meaning
        );
        world.communities[1].lands = vec![a, b, c];
        world.communities[1].size = world.communities[0].size * 20.0;
        world.generation = 1;
        world.refresh_river_names();
        assert_eq!(world.river_names[0].last().unwrap().variety, upstream);
        assert_eq!(world.known_river(main, 0), Some(&main_local));
        assert_eq!(world.known_river(upstream, 0), Some(&upstream_heard));
        assert_eq!(world.known_river(upstream, 1), Some(&tributary_local));
        assert_eq!(world.river_names[1].last().unwrap().name, tributary_local);
    }
}

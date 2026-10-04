//! Response causes are recorded at decisions, never reconstructed from log order.
use crate::{ClimateChange, World, WorldEvent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mechanism {
    Hardship,
    Crowding,
    StrongerNeighbour,
    Climate,
    Craft,
    Conquest,
    City,
    Contact,
    Pilgrimage,
    UnfaithfulHolder,
    Court,
    WordNeed,
    ForeignPrestige,
    ReligiousRevival,
    PuristNorm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cause {
    /// Index in this telling's World::events; the facade calls it world:<event>.
    pub event: usize,
    pub mechanism: Mechanism,
}

/// Event identities attached when the corresponding state is changed.
/// No event is invented for an unrecorded contact, climate, or city.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Triggers {
    pub hardship: BTreeMap<usize, usize>,
    pub climate: BTreeMap<usize, usize>,
    pub exposure: BTreeMap<usize, usize>,
    pub states: BTreeMap<usize, usize>,
    pub cities: BTreeMap<usize, usize>,
    pub holy_lands: BTreeMap<(usize, usize), usize>,
    pub pilgrimages: BTreeMap<(usize, usize, usize, usize), usize>,
    pub word_needs: BTreeMap<(usize, crate::Need), usize>,
}

impl World {
    /// Append an event and retain its identity at the point its state changes.
    pub(crate) fn record_event(&mut self, event: WorldEvent) -> usize {
        let id = self.events.len();
        match &event {
            WorldEvent::HardTimes { region, .. } => {
                self.triggers.hardship.insert(*region, id);
            }
            WorldEvent::Climate {
                zone,
                change,
                peoples,
                ..
            } => {
                if *change == ClimateChange::Recovery {
                    self.triggers.climate.remove(zone);
                } else {
                    self.triggers.climate.insert(*zone, id);
                    for &c in peoples {
                        self.triggers.exposure.insert(c, id);
                    }
                }
            }
            WorldEvent::Rose { state } => {
                self.triggers.states.insert(*state, id);
            }
            WorldEvent::City { city } => {
                self.triggers.cities.insert(*city, id);
            }
            WorldEvent::HolyLand {
                religion, region, ..
            } => {
                self.triggers.holy_lands.insert((*religion, *region), id);
            }
            WorldEvent::Pilgrimage {
                religion,
                community,
                from,
                to,
                ..
            } => {
                self.triggers
                    .pilgrimages
                    .insert((*religion, *community, *from, *to), id);
            }
            WorldEvent::Learnt {
                community, craft, ..
            } => {
                self.triggers
                    .word_needs
                    .insert((*community, crate::Need::Craft(*craft)), id);
            }
            WorldEvent::Adopted {
                community,
                livelihood,
                ..
            } => {
                self.triggers
                    .word_needs
                    .insert((*community, crate::Need::Livelihood(*livelihood)), id);
            }
            WorldEvent::Revealed { religion } => {
                self.triggers
                    .word_needs
                    .insert((self.religions[*religion].people, crate::Need::Faith), id);
            }
            WorldEvent::Converted { community, .. } => {
                self.triggers
                    .word_needs
                    .insert((*community, crate::Need::Faith), id);
            }
            _ => {}
        }
        self.events.push((self.generation, event));
        id
    }

    pub(crate) fn record_response(&mut self, event: WorldEvent, cause: Option<Cause>) -> usize {
        let id = self.record_event(event);
        if let Some(cause) = cause {
            debug_assert!(cause.event < id);
            debug_assert!(self.events[cause.event].0 <= self.generation);
            self.causes.insert(id, cause);
        }
        id
    }
    pub(crate) fn word_need_cause(&self, community: usize, need: crate::Need) -> Option<Cause> {
        self.triggers
            .word_needs
            .get(&(community, need))
            .map(|&event| Cause {
                event,
                mechanism: Mechanism::WordNeed,
            })
    }

    pub(crate) fn contact_cause(&self, a: usize, b: usize) -> Option<Cause> {
        self.contacts
            .iter()
            .find(|k| (k.a == a && k.b == b) || (k.a == b && k.b == a))
            .and_then(|k| k.cause)
    }

    pub(crate) fn neighbour_cause(&self, a: usize, b: usize) -> Option<Cause> {
        let cause = self.contact_cause(a, b)?;
        // A split's implicit link or a conquest is not a recorded meeting.
        matches!(self.events[cause.event].1, WorldEvent::Met { .. }).then_some(Cause {
            event: cause.event,
            mechanism: Mechanism::StrongerNeighbour,
        })
    }

    /// The same recent-land and remembered-exposure conditions used by polities.
    pub(crate) fn hard_times_cause(&self, community: usize) -> Option<Cause> {
        let exposed = |c: usize| {
            let k = &self.communities[c];
            let ongoing = k
                .lands
                .iter()
                .filter(|&&r| self.climate.regions[r].severe)
                .filter_map(|&r| self.map.regions[r].climate_zone)
                .find_map(|z| self.triggers.climate.get(&z).copied());
            ongoing.or_else(|| {
                self.climate
                    .exposure
                    .get(c)
                    .copied()
                    .flatten()
                    .filter(|&g| self.generation.saturating_sub(g) <= crate::polity::CHALLENGE_SPAN)
                    .and_then(|_| self.triggers.exposure.get(&c).copied())
            })
        };
        if let Some(event) = exposed(community).or_else(|| {
            self.rules(community)
                .and_then(|s| self.states[s].subjects().find_map(exposed))
        }) {
            return Some(Cause {
                event,
                mechanism: Mechanism::Climate,
            });
        }
        self.communities[community]
            .lands
            .iter()
            .filter_map(|r| self.triggers.hardship.get(r).copied())
            .filter(|&e| {
                self.generation.saturating_sub(self.events[e].0) <= crate::polity::CHALLENGE_SPAN
            })
            .max()
            .map(|event| Cause {
                event,
                mechanism: Mechanism::Hardship,
            })
    }
}

#[cfg(test)]
mod tests;

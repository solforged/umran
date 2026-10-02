//! Reading-local provenance links and subject timelines. No prose matching.
use crate::annals::{Annal, annals, world_event_id};
use crate::notebook::Subject;
use crate::{Bench, form_after, to_json};
use serde::Serialize;
use std::collections::HashSet;
use umran_sim::{ContactKind, Event, Lexeme, LoanCause, Origin, World, WorldEvent};

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum LoanCauseView {
    Contact {
        contact: ContactKind,
        donor: usize,
        recipient: usize,
        since: u32,
        event: Option<String>,
    },
    Rule {
        ruler: usize,
        ruled: usize,
        state: Option<usize>,
        event: Option<String>,
    },
    Faith {
        religion: usize,
        teacher: Option<usize>,
        recipient: usize,
        event: Option<String>,
    },
    Shift {
        community: usize,
        from_variety: usize,
        event: Option<String>,
    },
    City {
        city: usize,
        community: usize,
        event: Option<String>,
    },
    Coinage {
        peoples: [usize; 2],
        event: Option<String>,
    },
    Classical {
        classical: usize,
        recipient: usize,
        event: Option<String>,
    },
    Unrecorded,
}

/// Only recorded participants and times identify a triggering annal. Inherited
/// faith, classical learning, and word-finding need not have a matching annal.
fn event_position(world: &World, cause: LoanCause, generation: u32) -> Option<usize> {
    if matches!(cause, LoanCause::Unrecorded | LoanCause::Coinage { .. }) {
        return None;
    }
    world.events.iter().rposition(|(at, event)| {
        if *at > generation {
            return false;
        }
        match (cause, event) {
            (
                LoanCause::Contact {
                    donor,
                    recipient,
                    kind,
                    since,
                },
                WorldEvent::Met { a, b, kind: met },
            ) => {
                *at == since
                    && kind == *met
                    && ((donor == *a && recipient == *b) || (donor == *b && recipient == *a))
            }
            (
                LoanCause::Contact {
                    donor,
                    recipient,
                    kind: ContactKind::Neighbours | ContactKind::Trade,
                    since,
                },
                WorldEvent::Split {
                    community,
                    daughter,
                    ..
                },
            ) => {
                *at == since
                    && ((donor == *community && recipient == *daughter)
                        || (donor == *daughter && recipient == *community))
            }
            (
                LoanCause::Contact {
                    donor,
                    recipient,
                    kind: ContactKind::Neighbours | ContactKind::Trade,
                    since,
                },
                WorldEvent::Settlement(record),
            ) => {
                *at == since
                    && record.daughter.is_some_and(|daughter| {
                        (donor == record.plan.choice.community && recipient == daughter)
                            || (donor == daughter && recipient == record.plan.choice.community)
                    })
            }
            (
                LoanCause::Rule { ruler, ruled, .. },
                WorldEvent::Conquered { ruler: a, ruled: b },
            ) => ruler == *a && ruled == *b,
            (
                LoanCause::Faith {
                    religion,
                    recipient,
                    teacher,
                },
                WorldEvent::Converted {
                    community,
                    religion: r,
                    from,
                },
            ) => {
                religion == *r
                    && recipient == *community
                    && teacher.is_none_or(|t| Some(t) == *from)
            }
            (
                LoanCause::Faith {
                    religion,
                    recipient,
                    teacher: None,
                },
                WorldEvent::Revealed { religion: r },
            ) => religion == *r && world.religions[religion].people == recipient,
            (
                LoanCause::Shift {
                    community,
                    from_variety,
                },
                WorldEvent::Shift {
                    community: c, from, ..
                },
            ) => *at == generation && community == *c && from_variety == *from,
            (
                LoanCause::City { city, community },
                WorldEvent::Koine {
                    city: c,
                    community: p,
                    ..
                },
            ) => *at == generation && city == *c && community == *p,
            (LoanCause::Classical { classical, .. }, WorldEvent::Fixed { state }) => world.states
                [*state]
                .classical
                .is_some_and(|c| c.variety == classical),
            _ => false,
        }
    })
}

pub(crate) fn cause_view(world: &World, cause: LoanCause, generation: u32) -> LoanCauseView {
    let event = event_position(world, cause, generation).map(world_event_id);
    match cause {
        LoanCause::Contact {
            donor,
            recipient,
            kind,
            since,
        } => LoanCauseView::Contact {
            contact: kind,
            donor,
            recipient,
            since,
            event,
        },
        LoanCause::Rule {
            ruler,
            ruled,
            state,
        } => LoanCauseView::Rule {
            ruler,
            ruled,
            state,
            event,
        },
        LoanCause::Faith {
            religion,
            teacher,
            recipient,
        } => LoanCauseView::Faith {
            religion,
            teacher,
            recipient,
            event,
        },
        LoanCause::Shift {
            community,
            from_variety,
        } => LoanCauseView::Shift {
            community,
            from_variety,
            event,
        },
        LoanCause::City { city, community } => LoanCauseView::City {
            city,
            community,
            event,
        },
        LoanCause::Coinage { donor, recipient } => LoanCauseView::Coinage {
            peoples: [donor, recipient],
            event,
        },
        LoanCause::Classical {
            classical,
            recipient,
        } => LoanCauseView::Classical {
            classical,
            recipient,
            event,
        },
        LoanCause::Unrecorded => LoanCauseView::Unrecorded,
    }
}

pub(crate) fn origin_cause(world: &World, word: &Lexeme) -> Option<LoanCauseView> {
    match word.origin {
        Origin::Borrowed { cause, .. } => Some(cause_view(world, cause, word.born)),
        _ => None,
    }
}

/// An inherited log entry belongs to the ancestor that recorded it, not every
/// daughter that later carries it. Same-generation daughter changes stay local.
fn entry_variety(world: &World, mut variety: usize, word: &Lexeme, index: usize) -> usize {
    while let Some(fork) = world.varieties[variety].parent {
        if word.id.0 >= fork.inherited || word.log[index].generation > fork.generation {
            break;
        }
        let inherited = &world.varieties[fork.variety].lexicon.get(word.id).log;
        if inherited.get(index) != Some(&word.log[index]) {
            break;
        }
        variety = fork.variety;
    }
    variety
}

fn word_events(world: &World, variety: usize, concept: &str) -> Result<HashSet<String>, String> {
    let concept = umran_sim::concepts::by_id(concept).ok_or("Unknown concept.")?;
    let speech = world
        .varieties
        .get(variety)
        .ok_or("No such language variety.")?;
    let mut ids = HashSet::new();
    for variant in &speech.lexicon.slot(concept).variants {
        let word = speech.lexicon.get(variant.lexeme);
        for (i, entry) in word.log.iter().enumerate() {
            let owner = entry_variety(world, variety, word, i);
            match entry.event {
                Event::Borrowed { cause, .. } => {
                    if let Some(position) = event_position(world, cause, entry.generation) {
                        ids.insert(world_event_id(position));
                    }
                }
                Event::SoundLaw { law, ref before } => {
                    let after = form_after(word, i);
                    let v = &world.varieties[owner];
                    // Logged stress changes touched this word even when a
                    // second shift in the same generation restored its accent.
                    if (before != after
                        || v.stress_history.iter().any(|(g, _)| *g == entry.generation))
                        && v.laws.contains(&(entry.generation, law))
                    {
                        ids.insert(format!("sounds:{owner}:{}", entry.generation));
                        // Formation laws are represented by the shift/koiné annal.
                        for (position, (generation, event)) in world.events.iter().enumerate() {
                            if *generation == entry.generation
                                && matches!(event,
                                WorldEvent::Shift { variety: v, .. } | WorldEvent::Koine { variety: v, .. } if *v == owner)
                            {
                                ids.insert(world_event_id(position));
                            }
                        }
                    }
                }
                Event::Extended { .. } | Event::Lost { .. } => {
                    for (position, (generation, event)) in world.events.iter().enumerate() {
                        if *generation == entry.generation
                            && matches!(event,
                            WorldEvent::Pejorated { variety: v, word: w, .. } if *v == owner && *w == word.id)
                        {
                            ids.insert(world_event_id(position));
                        }
                    }
                }
                Event::Obsolete => {}
            }
        }
    }
    Ok(ids)
}

#[derive(Serialize)]
struct Story {
    annals: Vec<String>,
}

impl Bench {
    pub fn story(&mut self, generation: u32, subject: &str) -> Result<String, String> {
        let subject: Subject = serde_json::from_str(subject).map_err(|e| e.to_string())?;
        let world = self.world(generation);
        let word = match &subject {
            Subject::Word { variety, concept } => word_events(world, *variety, concept)?,
            _ => HashSet::new(),
        };
        let all = annals(world);
        let matches = |a: &Annal| match &subject {
            Subject::World | Subject::History => true,
            Subject::People { id } => a.peoples.contains(id),
            Subject::State { id } => a.states.contains(id),
            Subject::Religion { id } => a.religions.contains(id),
            Subject::Craft { id } => a.crafts.contains(id),
            Subject::Language { variety } => a.languages.contains(variety),
            Subject::Word { .. } => word.contains(&a.id),
            Subject::Law { id } => a.laws.contains(&id.as_str()),
            Subject::Land { region } => a.lands.contains(region),
            Subject::Continent { landmass } => a
                .lands
                .iter()
                .any(|r| world.map.regions[*r].landmass == Some(*landmass)),
            Subject::River { id } => a.rivers.contains(id),
            Subject::Zone { id } => a.zones.contains(id),
            Subject::Event { id } => {
                a.id == *id
                    || all
                        .iter()
                        .any(|group| group.id == *id && group.members.iter().any(|m| m.id == a.id))
            }
        };
        fn collect(entries: &[Annal], matches: &impl Fn(&Annal) -> bool, ids: &mut Vec<String>) {
            for a in entries {
                if a.members.is_empty() {
                    if matches(a) {
                        ids.push(a.id.clone());
                    }
                } else {
                    collect(&a.members, matches, ids);
                }
            }
        }
        let mut ids = Vec::new();
        collect(&all, &matches, &mut ids);
        to_json(&Story { annals: ids })
    }
}

#[cfg(test)]
mod tests;

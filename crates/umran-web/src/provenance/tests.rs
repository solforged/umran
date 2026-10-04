use super::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use umran_sim::{Contact, Field, HistoryPoint, Naming, Params, Revelation, SoundProfile};

#[path = "../../examples/support/sample.rs"]
mod sample;

#[derive(Default)]
struct Band {
    causes: HashMap<&'static str, usize>,
    sacred_faith: usize,
    sacred_contact: usize,
}

fn leaves(entries: &[Annal]) -> HashMap<&str, &Annal> {
    entries
        .iter()
        .flat_map(|a| {
            if a.members.is_empty() {
                vec![(a.id.as_str(), a)]
            } else {
                leaves(&a.members).into_iter().collect()
            }
        })
        .collect()
}

struct Snapshot {
    contacts: Vec<Contact>,
    faiths: Vec<Option<usize>>,
}
fn snapshot(world: &World) -> Snapshot {
    Snapshot {
        contacts: world.contacts.clone(),
        faiths: world.communities.iter().map(|c| c.faith).collect(),
    }
}

fn audit(world: &World, snapshots: &[Snapshot], band: &mut Band) {
    let entries = annals(world);
    let entries = leaves(&entries);
    let mut seen = HashSet::new();
    let spreads = world
        .events
        .iter()
        .any(|(_, e)| matches!(e, WorldEvent::Converted { .. }));
    for (v, variety) in world.varieties.iter().enumerate() {
        for word in &variety.lexicon.lexemes {
            let Origin::Borrowed {
                from,
                source,
                cause,
            } = word.origin
            else {
                continue;
            };
            assert!(from < world.varieties.len());
            assert!((source.0 as usize) < world.varieties[from].lexicon.lexemes.len());
            assert!(
                matches!(word.log.first(), Some(entry) if entry.generation == word.born
                && matches!(entry.event, Event::Borrowed { from: f, cause: c, .. } if f == from && c == cause))
            );
            let g = word.born;
            let nearby = &snapshots[g.saturating_sub(1) as usize..=g as usize];
            let person = |c: usize| assert!(c < world.communities.len());
            let mut peoples = Vec::new();
            let kind = match cause {
                LoanCause::Contact {
                    donor,
                    recipient,
                    kind,
                    since,
                } => {
                    person(donor);
                    person(recipient);
                    peoples.extend([donor, recipient]);
                    assert!(since <= g);
                    assert!(
                        nearby
                            .iter()
                            .any(|s| s.contacts.iter().any(|k| k.since == since
                                && k.kind == kind
                                && ((k.a == donor && k.b == recipient)
                                    || (k.b == donor && k.a == recipient)))),
                        "contact absent at loan: seed {} g {g} {cause:?}",
                        world.seed
                    );
                    "contact"
                }
                LoanCause::Rule {
                    ruler,
                    ruled,
                    state,
                } => {
                    person(ruler);
                    person(ruled);
                    peoples.extend([ruler, ruled]);
                    assert!(nearby.iter().any(|s| {
                        s.contacts
                            .iter()
                            .any(|k| k.a == ruler && k.b == ruled && k.kind == ContactKind::Rule)
                    }));
                    if let Some(state) = state {
                        let s = &world.states[state];
                        assert_eq!(s.rulers, ruler);
                        assert!(s.rose <= g && s.fell.is_none_or(|(at, _)| at >= g));
                        assert!(s.members.iter().any(|m| m.community == ruled
                            && m.joined <= g
                            && m.left.is_none_or(|at| at >= g)));
                    }
                    "rule"
                }
                LoanCause::Faith {
                    religion,
                    teacher,
                    recipient,
                } => {
                    person(recipient);
                    peoples.push(recipient);
                    if let Some(t) = teacher {
                        person(t);
                        peoples.push(t);
                    }
                    assert!(world.religions[religion].founded <= g);
                    assert!(
                        nearby
                            .iter()
                            .any(|s| s.faiths.get(recipient) == Some(&Some(religion))),
                        "faith absent at loan: seed {} g {g} {cause:?}",
                        world.seed
                    );
                    "faith"
                }
                LoanCause::Shift {
                    community,
                    from_variety,
                } => {
                    person(community);
                    peoples.push(community);
                    assert_eq!(from, from_variety);
                    assert!(world.events.iter().any(|(at, e)| *at == g && matches!(e, WorldEvent::Shift { community: c, from: f, .. } if *c == community && *f == from_variety)));
                    "shift"
                }
                LoanCause::City { city, community } => {
                    person(community);
                    peoples.push(community);
                    assert_eq!(world.cities[city].townsfolk, Some(community));
                    "city"
                }
                LoanCause::Coinage { donor, recipient } => {
                    person(donor);
                    person(recipient);
                    assert!(nearby.iter().any(|s| {
                        s.contacts.iter().any(|k| {
                            (k.a == donor && k.b == recipient) || (k.b == donor && k.a == recipient)
                        })
                    }));
                    "coinage"
                }
                LoanCause::Classical {
                    classical,
                    recipient,
                } => {
                    person(recipient);
                    assert_eq!(from, classical);
                    // A high form is a state's classical language or, for a
                    // faith that keeps its scripture untranslated, its sacred one.
                    assert!(
                        world
                            .states
                            .iter()
                            .any(|s| s.classical.is_some_and(|c| c.variety == classical))
                            || world.religions.iter().any(|r| r.sacred == classical)
                    );
                    "classical"
                }
                LoanCause::Unrecorded => panic!("all current loan writers know their mechanism"),
            };
            let view = serde_json::to_value(cause_view(world, cause, g)).unwrap();
            if let Some(id) = view["event"].as_str() {
                let a = entries
                    .get(id)
                    .expect("cause event resolves, including grouped constituents");
                assert!(a.generation <= g);
                for c in peoples {
                    assert!(a.peoples.contains(&c), "{id} lacks {c}: {cause:?}");
                }
                if let LoanCause::Faith { religion, .. } = cause {
                    assert!(a.religions.contains(&religion));
                }
                if let LoanCause::Classical { classical, .. } = cause {
                    assert!(a.languages.contains(&classical));
                }
            } else if let LoanCause::Contact {
                donor,
                recipient,
                kind,
                since,
            } = cause
            {
                assert!(!world.events.iter().any(|(at, e)| *at == since
                    && matches!(e,
                    WorldEvent::Met { a, b, kind: k } if *k == kind &&
                    ((*a == donor && *b == recipient) || (*b == donor && *a == recipient)))));
            } else if match cause {
                LoanCause::Shift { .. } | LoanCause::City { .. } => true,
                // A sacred high form has no fixing event to point to.
                LoanCause::Classical { classical, .. } => world.classical_of(classical).is_some(),
                _ => false,
            } {
                panic!("recorded cause must resolve: {cause:?}");
            }
            if seen.insert(world.root_of(v, word.id)) {
                *band.causes.entry(kind).or_default() += 1;
                if spreads && word.first_sense.field == Field::Religion {
                    band.sacred_faith += usize::from(kind == "faith");
                    band.sacred_contact += usize::from(kind == "contact");
                }
            }
        }
    }
}

#[test]
fn provenance_band_40_seeds_4000_years_and_sample() {
    let presets = SoundProfile::presets();
    let n = presets.len() as u64;
    let mut band = Band::default();
    for seed in 0..40 {
        let mut world = World::new(seed, Params::default());
        let pick = |i: u64| &presets[((((seed % n) * 3 + i) * 7) % n) as usize];
        world.found(pick(0), 0.5, 0.4);
        let home = world.communities[0].home();
        for (i, power, open) in [(1, 0.4, 0.6), (2, 0.85, 0.3)] {
            world.found_seeded(
                &Naming::People,
                pick(i),
                seed + i,
                power,
                open,
                Some(home),
                None,
                None,
            );
        }
        world.connect(0, 1, 0.5, ContactKind::Trade).unwrap();
        world.connect(2, 1, 0.8, ContactKind::Rule).unwrap();
        world.connect(2, 0, 0.3, ContactKind::Neighbours).unwrap();
        let faith = world.found_religion(1, Revelation::Proclaimed);
        world.convert(0, faith, Some(1));
        world.convert(2, faith, Some(1));
        let mut snapshots = vec![snapshot(&world)];
        for _ in 0..160 {
            world.step();
            snapshots.push(snapshot(&world));
        }
        audit(&world, &snapshots, &mut band);
    }
    let mut history = sample::sample();
    let snapshots: Vec<_> = (0..=160).map(|g| snapshot(&history.world_at(g))).collect();
    audit(history.latest(), &snapshots, &mut band);
    let mut counts: Vec<_> = band.causes.into_iter().collect();
    counts.sort_unstable();
    println!("PROVENANCE 40 seeds x 4000 years + exact sample: {counts:?}");
    println!(
        "SACRED loans in spreading-faith worlds: faith={} contact={}",
        band.sacred_faith, band.sacred_contact
    );
    // The measurement is descriptive; the provenance change must never tune rates.
}

fn fixed(world: World) -> Bench {
    let mut bench = Bench::new(world.seed as u32, "medium").unwrap();
    bench.fixed = Some((HistoryPoint::default(), world));
    bench
}
fn ids(bench: &mut Bench, subject: Value) -> Vec<String> {
    let value: Value =
        serde_json::from_str(&bench.story(160, &subject.to_string()).unwrap()).unwrap();
    value["annals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_owned())
        .collect()
}
fn founders() -> World {
    let mut world = World::new(21, Params::static_society());
    for seed in 0..4 {
        let home = world.communities.first().map(|c| c.home());
        world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            seed,
            0.5,
            0.5,
            home,
            None,
            None,
        );
    }
    world
}

#[test]
fn grouped_subject_story_keeps_only_matching_constituents_and_time_correct_speech() {
    let mut world = founders();
    world.connect(0, 1, 0.5, ContactKind::Neighbours).unwrap();
    world.connect(2, 3, 0.5, ContactKind::Neighbours).unwrap();
    let own = world
        .events
        .iter()
        .position(|(_, e)| matches!(e, WorldEvent::Met { a: 0, b: 1, .. }))
        .unwrap();
    let other = world
        .events
        .iter()
        .position(|(_, e)| matches!(e, WorldEvent::Met { a: 2, b: 3, .. }))
        .unwrap();
    world.generation = 1;
    let shifted = world.shift(0, 2);
    let home = world.communities[0].home();
    world.split(1, None, 0.5);
    let split = world
        .events
        .iter()
        .position(|(_, e)| matches!(e, WorldEvent::Split { community: 1, .. }))
        .unwrap();
    let mut bench = fixed(world);
    let people = ids(&mut bench, json!({"kind":"people", "id":0}));
    assert!(people.contains(&world_event_id(own)));
    assert!(!people.contains(&world_event_id(other)));
    assert!(!people.iter().any(|id| id.starts_with("neighbours:")));
    assert!(
        ids(&mut bench, json!({"kind":"language", "variety":0})).contains(&world_event_id(own))
    );
    assert!(
        !ids(&mut bench, json!({"kind":"language", "variety":shifted}))
            .contains(&world_event_id(own))
    );
    let land = ids(&mut bench, json!({"kind":"land", "region":home}));
    assert!(land.contains(&world_event_id(split)));
    assert!(
        !land.contains(&world_event_id(own)),
        "contact annals do not invent a location"
    );
}

#[test]
fn word_story_follows_inherited_changes_but_not_untouched_laws() {
    let mut world = founders();
    let concept = umran_sim::concepts::by_id("water").unwrap();
    let word_id = world.varieties[0].lexicon.slot(concept).dominant().unwrap();
    let law = umran_sim::catalog()[0].id;
    world.generation = 1;
    let word = world.varieties[0].lexicon.get_mut(word_id);
    let before = word.form.clone();
    word.form = umran_sim::Form::from_ipa("e").unwrap();
    word.log.push(umran_sim::Entry {
        generation: 1,
        event: Event::SoundLaw { law, before },
    });
    world.varieties[0].laws.push((1, law));
    let daughter = world.split(0, None, 0.5);
    let v = world.communities[daughter].variety;
    world.generation = 2;
    world.varieties[v].laws.push((2, law)); // another word, not water, changed
    let mut bench = fixed(world);
    assert_eq!(
        ids(
            &mut bench,
            json!({"kind":"word", "variety":v, "concept":"water"})
        ),
        vec!["sounds:0:1"]
    );
}

#[test]
fn cause_event_resolution_rejects_wrong_endpoints_kind_and_future_records() {
    let mut world = founders();
    world.generation = 3;
    world.connect(0, 1, 0.5, ContactKind::Trade).unwrap();
    let cause = LoanCause::Contact {
        donor: 1,
        recipient: 0,
        kind: ContactKind::Trade,
        since: 3,
    };
    let exact = event_position(&world, cause, 3).unwrap();
    assert_eq!(event_position(&world, cause, 2), None);
    world.generation = 4;
    world.connect(0, 1, 0.5, ContactKind::Religion).unwrap();
    world.connect(2, 3, 0.5, ContactKind::Trade).unwrap();
    assert_eq!(event_position(&world, cause, 4), Some(exact));
    assert_eq!(
        event_position(
            &world,
            LoanCause::Contact {
                donor: 1,
                recipient: 0,
                kind: ContactKind::Trade,
                since: 4
            },
            4
        ),
        None
    );
}

#[test]
fn story_and_word_provenance_stay_in_the_requested_telling_and_exact_reading() {
    let history = sample::sample();
    let mut bench = Bench::load(&serde_json::to_string(&history.recipe()).unwrap()).unwrap();
    let active = bench.chronicle.active();
    let subject = json!({"kind":"religion", "id":0});
    let original = ids(&mut bench, subject.clone());
    assert!(!original.is_empty());
    bench.branch(99);
    let mut preserved = bench.read(active, "").unwrap();
    assert_eq!(ids(&mut preserved, subject.clone()), original);
    assert!(ids(&mut bench, subject.clone()).is_empty());
    let point = serde_json::to_string(&history.point_at(99)).unwrap();
    let mut early = preserved.read(active, &point).unwrap();
    assert!(ids(&mut early, subject).is_empty());
    assert_eq!(
        early.story(160, r#"{"kind":"history"}"#).unwrap(),
        early.story(0, r#"{"kind":"history"}"#).unwrap()
    );
}

#[test]
fn daughter_contact_links_resolve_to_exact_split_and_settlement_records() {
    use umran_sim::settlement::{SettlementChoice, SettlementIntent};
    let mut world = founders();
    world.generation = 7;
    let daughter = world.split(0, None, 0.5);
    let contact = *world
        .contacts
        .iter()
        .find(|c| c.a == 0 && c.b == daughter)
        .unwrap();
    let cause = LoanCause::Contact {
        donor: daughter,
        recipient: 0,
        kind: contact.kind,
        since: contact.since,
    };
    let position = event_position(&world, cause, 8).unwrap();
    assert!(
        matches!(world.events[position].1, WorldEvent::Split { community: 0, daughter: d, .. } if d == daughter)
    );
    assert_eq!(event_position(&world, cause, 6), None);

    world.generation = 9;
    let destination = world
        .settlement_options(1, SettlementIntent::Settlers, 0.5)
        .unwrap()
        .into_iter()
        .find(|o| {
            o.reason.is_none()
                && world.map.regions[world.communities[1].home()]
                    .neighbours
                    .contains(&o.region)
        })
        .unwrap()
        .region;
    let daughter = world
        .settle(&SettlementChoice {
            community: 1,
            intent: SettlementIntent::Settlers,
            destination,
            share: 0.5,
            naming: None,
            intensity: 0.5,
        })
        .unwrap()
        .unwrap();
    let contact = *world
        .contacts
        .iter()
        .find(|c| c.a == 1 && c.b == daughter)
        .unwrap();
    let cause = LoanCause::Contact {
        donor: daughter,
        recipient: 1,
        kind: contact.kind,
        since: contact.since,
    };
    let position = event_position(&world, cause, 10).unwrap();
    assert!(
        matches!(&world.events[position].1, WorldEvent::Settlement(record) if record.daughter == Some(daughter) && record.plan.choice.community == 1)
    );
    assert_eq!(
        event_position(
            &world,
            LoanCause::Contact {
                donor: daughter,
                recipient: 0,
                kind: contact.kind,
                since: contact.since
            },
            10
        ),
        None
    );
}

#[test]
fn word_story_keeps_stress_changes_that_cancel_within_one_generation() {
    let mut world = founders();
    let concept = umran_sim::concepts::by_id("water").unwrap();
    let word_id = world.varieties[0].lexicon.slot(concept).dominant().unwrap();
    let laws: Vec<_> = umran_sim::catalog()
        .into_iter()
        .filter(|law| law.stress.is_some())
        .take(2)
        .collect();
    let speech = &mut world.varieties[0];
    speech.profile.stress = Some(umran_sim::StressRule::Initial);
    speech.stress_history = vec![
        (1, umran_sim::StressRule::Initial),
        (1, umran_sim::StressRule::Penult),
    ];
    let word = speech.lexicon.get_mut(word_id);
    word.form = umran_sim::Form::from_ipa("pataka").unwrap();
    for law in laws {
        word.log.push(umran_sim::Entry {
            generation: 1,
            event: Event::SoundLaw {
                law: law.id,
                before: word.form.clone(),
            },
        });
        speech.laws.push((1, law.id));
    }
    world.generation = 1;
    let mut bench = fixed(world);
    assert_eq!(
        ids(
            &mut bench,
            json!({"kind":"word", "variety":0, "concept":"water"})
        ),
        vec!["sounds:0:1"]
    );
}

//! The world's history told as a chronicle. As in a critical edition, each
//! entry has two voices: its text is the annalist's, in the past tense and
//! varied in wording, and its notes are the linguist's apparatus. Wording
//! is drawn from the annals' own stream, so it never moves the world's
//! draws, and the same history is always told the same way.

use crate::{
    GrammarFormView, SpecimenWord, grammar_form_at, grammar_form_view, language_label,
    livelihood_noun, specimen, substrate_label,
};
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::BTreeMap;
use umran_sim::climate::{ClimateCause, ClimateChange};
use umran_sim::concepts::Concept;
use umran_sim::grammar::{Category, Marker, MarkerKind, MarkerOrigin, NoticeKind, Side};
use umran_sim::ideas::{NEEDS, Need};
use umran_sim::names::PlaceOrigin;
use umran_sim::rng::{index, key, stream};
use umran_sim::world::{ContactKind, Hardship};
use umran_sim::{
    Axis, Challenge, Craft, Event, Fall, Fixing, Form, Lexeme, Livelihood, Origin, Pole,
    Revelation, Rise, TemperCause, Variety, Vernacular, World, WorldEvent,
};

#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct Annal {
    /// Source-backed identity within one telling and engine revision.
    pub id: String,
    /// Original entries behind a compact yearly account.
    pub members: Vec<Annal>,
    /// Languages involved at the recorded time, not the speakers' present speech.
    pub languages: Vec<usize>,
    pub generation: u32,
    /// "found", "split", "migration", "shift", "contact", "parted",
    /// "neighbours", "conquest", "spread", "displaced", "hardship",
    /// "livelihood", "ended", "rose", "fell", "standard", "classical",
    /// "vernacular", "craft", "faith", "conversion", "meaning",
    /// "respelling", "schism", "pilgrimage", "holy-land", "temper", "grammar",
    /// "pronoun-renewed", "pronoun-polite", "pronoun-generalised",
    /// "pronoun-borrowed", "class-emerged",
    /// "class-merged", "class-lost", "harmony-gained", "harmony-lost",
    /// "tone-gained", "tone-lost", "climate", "river-flow", "purist-reform",
    /// "purist-replacement", or "law".
    pub kind: &'static str,
    /// The annalist's words. Words of the language are marked `*thus*`.
    pub text: String,
    /// The apparatus: what a linguist would note, such as the sound laws
    /// behind a change.
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<umran_sim::Cause>,
    /// The variety a sound law changed, so a view can show one language's.
    pub variety: Option<usize>,
    /// The peoples it tells of, so a view can link to them.
    pub peoples: Vec<usize>,
    /// The lands it tells of: where peoples went, and where from.
    pub lands: Vec<usize>,
    /// The sound laws it tells of, by id.
    pub laws: Vec<&'static str>,
    /// For a sound change, the language's specimen words after it, with
    /// how those it reached sounded before.
    pub specimen: Vec<SpecimenWord>,
    /// The states it tells of.
    pub states: Vec<usize>,
    /// The religions it tells of.
    pub religions: Vec<usize>,
    /// The crafts it tells of.
    pub crafts: Vec<Craft>,
    /// For a change of temper, what turned and why.
    pub temper: Option<Temper>,
    /// Structured grammatical change; absent for all other annal kinds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar: Option<GrammarAnnal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doctrine: Option<DoctrineAnnal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taboo: Option<TabooAnnal>,
    /// Climate zones and rivers it tells of, independently of land ids.
    pub zones: Vec<usize>,
    pub rivers: Vec<usize>,
    /// The climate transition, without inferring deaths from yield loss.
    pub climate: Option<Climate>,
    #[serde(rename = "riverFlow")]
    pub river_flow: Option<RiverFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settlement: Option<umran_sim::settlement::SettlementRecord>,
    /// The authored action that produced this entry, within its telling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<umran_sim::HistoryPoint>,
}

/// A people's temper turning: a leaning reaching one of its ends
/// (`entered`) or falling back from it.
#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct Temper {
    pub axis: Axis,
    pub pole: Pole,
    pub entered: bool,
    pub cause: TemperCause,
}

#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct DoctrineAnnal {
    tenet: umran_sim::doctrine::Tenet,
    previous: Option<f32>,
    stance: f32,
}

#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct TabooAnnal {
    concept: &'static str,
    old: u32,
    word: u32,
}

#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct GrammarAnnal {
    category: &'static str,
    #[serde(flatten)]
    event: GrammarAnnalEvent,
}

#[derive(Clone, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
enum GrammarAnnalEvent {
    NewMarker {
        marker: u32,
    },
    Fusion {
        particle: u32,
        marker: u32,
    },
    ContrastLoss,
    Analogy {
        lexeme: u32,
        before: GrammarFormView,
        after: GrammarFormView,
    },
    ImportedPair {
        lexeme: u32,
        from: usize,
    },
    MarkerTransfer {
        marker: u32,
        from: usize,
    },
}

#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct Climate {
    pub zone: usize,
    pub cause: ClimateCause,
    pub change: ClimateChange,
    pub severity: u8,
    pub wetness: f32,
    pub warmth: f32,
}

#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct RiverFlow {
    pub river: usize,
    pub cause: ClimateCause,
    pub flowing: bool,
}

const TEMPER_ENTERED: &[&str] = &["{c}, the {p} grew {w}.", "{c}, the {p} turned {w}."];
const TEMPER_LEFT: &[&str] = &[
    "{c}, the {p} were {w} no longer.",
    "{c}, the {p} ceased to be {w}.",
];

/// What turned a people's temper, as the start of a sentence.
fn temper_cause(cause: TemperCause) -> &'static str {
    match cause {
        TemperCause::Drift => "Over the generations",
        TemperCause::Inheritance => "Parting from their kin",
        TemperCause::Contact => "Living among others",
        TemperCause::Hardship => "In the hard years",
        TemperCause::Freed => "Free of foreign rule",
        TemperCause::LongRule => "After long rule over others",
        TemperCause::Comfort => "After long years of ease",
        TemperCause::Seafaring => "Taking to the sea",
        TemperCause::Faith => "Moved by their faith",
        TemperCause::Fate => "By fate's hand",
    }
}

/// One end of a leaning, as a word for a people; the workbench's
/// `ETHOS_POLES` uses the same words.
pub(crate) fn temper_word(axis: Axis, pole: Pole) -> &'static str {
    let (low, high) = match axis {
        Axis::Martial => ("peaceable", "warlike"),
        Axis::Open => ("insular", "welcoming"),
        Axis::Pious => ("worldly", "devout"),
        Axis::Hierarchical => ("egalitarian", "hierarchical"),
        Axis::Roving => ("rooted", "restless"),
        Axis::Seaward => ("landbound", "seagoing"),
    };
    match pole {
        Pole::Low => low,
        Pole::High => high,
    }
}

const FOUND: &[&str] = &[
    "The {p} first appeared, calling themselves {p}, “{m}”, and their speech {l}.",
    "The {p} first appeared under that name, meaning “{m}”; they called their tongue {l}.",
    "There was a people who named themselves {p}, that is, “{m}”; their speech they called {l}.",
];

const SPLIT: &[&str] =
    &["The {p} divided into two peoples; the new people called themselves {d}, “{m}”."];

const SPLIT_OVERSEAS: &[&str] = &[
    "Some of the {p} put out to sea and settled in {to}, calling themselves {d}, “{m}”.",
    "A part of the {p} sailed away to {to} and called themselves {d}, “{m}”.",
];

const MIGRATION: &[&str] = &[
    "The {p} left {from} and settled in {to}.",
    "The {p} took to the road, leaving {from} for {to}.",
    "The {p} went out from {from} and made {to} their home.",
];

const MIGRATION_OVERSEAS: &[&str] = &[
    "The {p} took to their boats and crossed the sea from {from} to {to}.",
    "The {p} sailed from {from} and made a new home in {to}.",
];

const SHIFT: &[&str] = &[
    "The {p} forsook their old speech for that of the {t}, and called it {l}.",
    "The {p} gave up their own tongue and spoke as the {t} did; they called it {l}.",
];

const LAW: &[&str] = &[
    "That year {p} began to say *{after}* for “{g}”, where their elders had said *{before}*.",
    "In those days the speech of {p} shifted, and *{before}*, “{g}”, became *{after}*.",
    "The young among {p} no longer said *{before}* but *{after}*, “{g}”.",
    "A change came over the tongue of {p}: “{g}” was now *{after}*, no longer *{before}*.",
];

const LAW_UNSEEN: &[&str] = &[
    "The speech of {p} changed a little.",
    "Something in the way {p} spoke shifted.",
];

fn contact_wording(contact: ContactKind) -> &'static [&'static str] {
    match contact {
        ContactKind::Neighbours => &["The {a} and the {b} came into contact as neighbours."],
        ContactKind::Trade => &[
            "The {a} and the {b} began to trade.",
            "Traders began to go between the {a} and the {b}.",
        ],
        ContactKind::Rule => &[
            "The {a} and the {b} were joined under one rule.",
            "One ruler came to hold both the {a} and the {b}.",
        ],
        ContactKind::Religion => &[
            "The {a} and the {b} came to share their gods.",
            "The {a} and the {b} began to keep the same feasts.",
        ],
        ContactKind::Intermarriage => &[
            "The {a} and the {b} began to marry one another.",
            "Many of the {a} took husbands and wives from among the {b}.",
        ],
    }
}

fn parting_wording(contact: ContactKind) -> &'static [&'static str] {
    match contact {
        ContactKind::Neighbours => &[
            "The {a} and the {b} drifted apart and had little more to do with one another.",
            "The {a} and the {b} ceased to be neighbours.",
        ],
        ContactKind::Trade => &["Trade between the {a} and the {b} ended."],
        ContactKind::Rule => &["The rule joining the {a} and the {b} came to an end."],
        ContactKind::Religion => &[
            "The {a} and the {b} no longer kept the same feasts.",
            "The {a} and the {b} went their own ways in worship.",
        ],
        ContactKind::Intermarriage => &[
            "The {a} and the {b} ceased to marry one another.",
            "Marriages between the {a} and the {b} grew rare, then stopped.",
        ],
    }
}

const CONQUEST: &[&str] = &[
    "The {a} conquered the {b} and ruled over them.",
    "The {b} fell under the rule of the {a}.",
    "The {a} made themselves masters of the {b}.",
];

/// One of `options` for the entry `keys` identify, filled in.
fn tell(world: &World, keys: &[u64], options: &[&str], fill: &[(&str, &str)]) -> String {
    let mut all = vec![key("annal")];
    all.extend_from_slice(keys);
    let mut rng = stream(world.seed, &all);
    let mut text = options[index(&mut rng, options.len())].to_string();
    for (name, value) in fill {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

pub(crate) fn world_event_id(position: usize) -> String {
    format!("world:{position}")
}

/// Everything that happened in `world`, as chronicle entries in order.
/// Names are written as they were said at the time of each entry.
pub(crate) fn annals(world: &World) -> Vec<Annal> {
    let meaning = |c: usize| world.communities[c].name.meaning.as_str();
    let mut out: Vec<Annal> = Vec::new();
    let shifts = Shifts::of(world);
    // Peoples coming to live beside one another and drifting apart, by
    // generation, told together so they do not crowd out the rest.
    let mut neighbours: BTreeMap<u32, Neighbours> = BTreeMap::new();
    // Peoples spreading into new land, by generation, told together too.
    let mut spreads: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    let mut grouped: BTreeMap<(&str, u32), Vec<Annal>> = BTreeMap::new();
    let entry = |generation, kind, text, peoples: &[usize], lands: &[usize]| Annal {
        id: String::new(),
        members: Vec::new(),
        languages: Vec::new(),
        generation,
        kind,
        text,
        notes: Vec::new(),
        cause: None,
        variety: None,
        peoples: peoples.to_vec(),
        lands: lands.to_vec(),
        laws: Vec::new(),
        specimen: Vec::new(),
        states: Vec::new(),
        religions: Vec::new(),
        crafts: Vec::new(),
        temper: None,
        doctrine: None,
        taboo: None,
        grammar: None,
        zones: Vec::new(),
        rivers: Vec::new(),
        climate: None,
        river_flow: None,
        settlement: None,
        decision: None,
        before: None,
    };
    for (position, &(generation, ref event)) in world.events.iter().enumerate() {
        let name = |c: usize| world.community_name_at(c, generation);
        let pair = |kind: &'static str, options: &[&str], a: usize, b: usize| {
            let mut all = vec![key(kind), u64::from(generation)];
            all.extend([a as u64, b as u64]);
            entry(
                generation,
                kind,
                tell(world, &all, options, &[("a", &name(a)), ("b", &name(b))]),
                &[a, b],
                &[],
            )
        };
        let tongue =
            |c: usize| world.language_title_at(shifts.spoken_after(world, c, position), generation);
        let g = u64::from(generation);
        let mut annal = match *event {
            WorldEvent::Tone {
                variety,
                gained,
                law,
            } => {
                let language = world.language_title_at(variety, generation);
                let (text, note) = match (gained, law) {
                    (true, Some("coda-tonogenesis")) => (
                        format!(
                            "In {language}, sounds at the ends of words fell silent and left their mark as pitch: words came to be told apart by tone."
                        ),
                        "Tonogenesis from lost final laryngeals, as Haudricourt (1954) showed for Vietnamese.",
                    ),
                    (true, _) => (
                        format!(
                            "In {language}, voiced and voiceless sounds before a vowel fell together and left high and low pitch behind: words came to be told apart by tone."
                        ),
                        "A register split, as in Thai and the Chinese languages (Matisoff, 1973).",
                    ),
                    (false, _) => (
                        format!("In {language}, pitch no longer told words apart."),
                        "Tone can be lost as its contrasts merge, as Swahili lost the tones of its Bantu ancestors.",
                    ),
                };
                let mut annal = entry(
                    generation,
                    if gained { "tone-gained" } else { "tone-lost" },
                    text,
                    &[],
                    &[],
                );
                annal.notes = vec![note.into()];
                annal.variety = Some(variety);
                annal.laws.extend(law);
                annal
            }
            WorldEvent::Settlement(ref record) => {
                use umran_sim::settlement::SettlementIntent;
                let plan = &record.plan;
                let c = plan.choice.community;
                let d = record.daughter;
                let to = place(world, plan.choice.destination, generation);
                let text = match plan.choice.intent {
                    SettlementIntent::Partition => format!(
                        "The *{}* divided their lands. The *{}* took *{}* as their heart, with {:.0} souls.",
                        name(c),
                        name(d.unwrap()),
                        to,
                        plan.arriving.population
                    ),
                    SettlementIntent::Settlers => format!(
                        "{:.0} of the *{}* went to settle *{}*, becoming the *{}*.",
                        plan.arriving.population,
                        name(c),
                        to,
                        name(d.unwrap())
                    ),
                    SettlementIntent::Migration => format!(
                        "The *{}* moved together to *{}*, {:.0} souls keeping their language.",
                        name(c),
                        to,
                        plan.arriving.population
                    ),
                };
                let mut peoples = vec![c];
                peoples.extend(d);
                let mut lands = plan.before.lands.clone();
                lands.extend(&plan.arriving.lands);
                lands.sort_unstable();
                lands.dedup();
                let mut annal = entry(generation, "settlement", text, &peoples, &lands);
                annal.notes = plan.recorded_notes();
                annal.states = plan.falling_states.clone();
                annal.settlement = Some(record.as_ref().clone());
                annal
            }
            WorldEvent::FleetBuilt { community } => entry(
                generation,
                "fleet-built",
                format!("The *{}* built ships and took to the sea.", name(community)),
                &[community],
                &[],
            ),
            WorldEvent::FleetLost {
                community,
                ref ports,
            } => entry(
                generation,
                "fleet-lost",
                format!(
                    "The *{}* lost their harbours, and their ships with them.",
                    name(community)
                ),
                &[community],
                ports,
            ),
            WorldEvent::SeaRouteOpened {
                community,
                from,
                to,
            } => entry(
                generation,
                "sea-route-opened",
                format!(
                    "The *{}* opened a sea road from *{}* to *{}*.",
                    name(community),
                    place(world, from, generation),
                    place(world, to, generation)
                ),
                &[community],
                &[from, to],
            ),
            WorldEvent::Found { community } => entry(
                generation,
                "found",
                tell(
                    world,
                    &[key("found"), g, community as u64],
                    FOUND,
                    &[
                        ("p", &name(community)),
                        ("m", meaning(community)),
                        ("l", &tongue(community)),
                    ],
                ),
                &[community],
                &[],
            ),
            WorldEvent::Split {
                community,
                daughter,
                from,
                to,
                by_sea,
                ..
            } => {
                let mut annal = entry(
                    generation,
                    "split",
                    tell(
                        world,
                        &[key("split"), g, daughter as u64],
                        if by_sea { SPLIT_OVERSEAS } else { SPLIT },
                        &[
                            ("p", &name(community)),
                            ("d", &name(daughter)),
                            ("m", meaning(daughter)),
                            ("to", &place(world, to, generation)),
                        ],
                    ),
                    &[community, daughter],
                    &[from, to],
                );
                if from != to {
                    annal.notes.extend(place_note(world, to, generation));
                }
                annal
            }
            WorldEvent::Migrated {
                community,
                from,
                to,
                by_sea,
                ..
            } => {
                let mut annal = entry(
                    generation,
                    "migration",
                    tell(
                        world,
                        &[key("migration"), g, community as u64],
                        if by_sea {
                            MIGRATION_OVERSEAS
                        } else {
                            MIGRATION
                        },
                        &[
                            ("p", &name(community)),
                            ("from", &place_before(world, from, generation)),
                            ("to", &place(world, to, generation)),
                        ],
                    ),
                    &[community],
                    &[from, to],
                );
                annal.notes.extend(place_note(world, to, generation));
                annal
            }
            WorldEvent::Shift {
                community,
                toward,
                variety,
                ..
            } => entry(
                generation,
                "shift",
                tell(
                    world,
                    &[key("shift"), g, community as u64],
                    SHIFT,
                    &[
                        ("p", &name(community)),
                        ("t", &name(toward)),
                        ("l", &world.language_title_at(variety, generation)),
                    ],
                ),
                &[community, toward],
                &[],
            ),
            WorldEvent::Met {
                a,
                b,
                kind: ContactKind::Neighbours,
            } => {
                neighbours.entry(generation).or_default().met.push((a, b));
                pair("neighbours", contact_wording(ContactKind::Neighbours), a, b)
            }
            WorldEvent::Parted {
                a,
                b,
                kind: ContactKind::Neighbours,
            } => {
                neighbours
                    .entry(generation)
                    .or_default()
                    .parted
                    .push((a, b));
                pair("neighbours", parting_wording(ContactKind::Neighbours), a, b)
            }
            WorldEvent::Met { a, b, kind } => pair("contact", contact_wording(kind), a, b),
            WorldEvent::Parted { a, b, kind } => pair("parted", parting_wording(kind), a, b),
            WorldEvent::Conquered { ruler, ruled } => pair("conquest", CONQUEST, ruler, ruled),
            WorldEvent::Spread { community, to } => {
                spreads.entry(generation).or_default().push((community, to));
                spread_annal(world, generation, &[(community, to)])
            }
            WorldEvent::Displaced {
                community,
                region,
                by,
            } => entry(
                generation,
                "displaced",
                tell(
                    world,
                    &[key("displaced"), g, community as u64],
                    DISPLACED,
                    &[
                        ("p", &name(community)),
                        ("b", &name(by)),
                        ("land", &place(world, region, generation)),
                    ],
                ),
                &[community, by],
                &[region],
            ),
            WorldEvent::HardTimes {
                region,
                kind,
                share,
            } => entry(
                generation,
                "hardship",
                tell(
                    world,
                    &[key("hardship"), g, region as u64],
                    match kind {
                        Hardship::Famine => FAMINE,
                        Hardship::Plague => PLAGUE,
                    },
                    &[
                        ("land", &place(world, region, generation)),
                        ("share", share_in_words(share)),
                    ],
                ),
                &[],
                &[region],
            ),
            WorldEvent::SeasonalHazard {
                region,
                hazard,
                ref peoples,
                ..
            } => {
                // Only severe hazards on inhabited land are recorded, so
                // there is always a people to name.
                let who = match peoples.as_slice() {
                    [one] => format!("the {}", name(*one)),
                    [first, ..] => format!("the {} and their neighbours", name(*first)),
                    [] => "its people".into(),
                };
                let land = place(world, region, generation);
                let text = match hazard {
                    umran_sim::seasons::SeasonalHazard::Drought => {
                        format!("The rains failed in {land}, and {who} went hungry.")
                    }
                    umran_sim::seasons::SeasonalHazard::HardWinter => {
                        format!("A hard winter fell on {land}, and {who} went hungry.")
                    }
                    umran_sim::seasons::SeasonalHazard::Flood => {
                        format!("The river rose over the fields of {land}, and {who} went hungry.")
                    }
                };
                let mut annal = entry(generation, hazard.id(), text, peoples, &[region]);
                if let Some(zone) = world.map.regions[region].climate_zone {
                    annal.zones.push(zone);
                }
                if hazard == umran_sim::seasons::SeasonalHazard::Flood
                    && let Some(river) = world.map.river_regions[region]
                {
                    annal.rivers.push(river);
                }
                annal
            }
            WorldEvent::Climate {
                zone,
                cause,
                change,
                severity,
                wetness,
                warmth,
                ref lands,
                ref peoples,
            } => {
                let text = match change {
                    ClimateChange::Onset => {
                        "The climate changed, reducing what the lands could feed."
                    }
                    ClimateChange::Worsening => {
                        "The lands could feed fewer people as conditions worsened."
                    }
                    ClimateChange::Recovery => "The lands began to recover their feeding capacity.",
                };
                let mut annal = entry(generation, "climate", text.into(), peoples, lands);
                annal.zones.push(zone);
                annal.climate = Some(Climate {
                    zone,
                    cause,
                    change,
                    severity,
                    wetness,
                    warmth,
                });
                annal
            }
            WorldEvent::RiverFlow {
                river,
                flowing,
                cause,
                ref lands,
                ref peoples,
            } => {
                let text = if flowing {
                    "The river's flow recovered."
                } else {
                    "The river's flow weakened."
                };
                let mut annal = entry(generation, "river-flow", text.into(), peoples, lands);
                annal.rivers.push(river);
                annal.river_flow = Some(RiverFlow {
                    river,
                    cause,
                    flowing,
                });
                annal
            }
            WorldEvent::Adopted {
                community,
                livelihood,
                from,
            } => {
                let noun = livelihood_noun(livelihood);
                let (options, peoples): (&[&str], Vec<usize>) = match (from, livelihood) {
                    (Some(t), _) => (LEARNED, vec![community, t]),
                    (None, Livelihood::Farming) => (BEGAN_FARMING, vec![community]),
                    (None, Livelihood::Herding) => (BEGAN_HERDING, vec![community]),
                    (None, Livelihood::Foraging) => (BEGAN_FORAGING, vec![community]),
                };
                let teacher = from.map(name).unwrap_or_default();
                entry(
                    generation,
                    "livelihood",
                    tell(
                        world,
                        &[key("livelihood"), g, community as u64],
                        options,
                        &[("p", &name(community)), ("t", &teacher), ("way", noun)],
                    ),
                    &peoples,
                    &[],
                )
            }
            WorldEvent::Ended { community, into } => {
                let host = into.map(name).unwrap_or_default();
                entry(
                    generation,
                    "ended",
                    tell(
                        world,
                        &[key("ended"), g, community as u64],
                        if into.is_some() { MERGED } else { DIED_OUT },
                        &[("p", &name(community)), ("i", &host)],
                    ),
                    &[Some(community), into]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                    &[],
                )
            }
            WorldEvent::Rose { state } => state_annal(world, generation, state, "rose"),
            WorldEvent::Fell { state } => state_annal(world, generation, state, "fell"),
            WorldEvent::Standard { state } => state_annal(world, generation, state, "standard"),
            WorldEvent::City { city } => {
                let city = &world.cities[city];
                let s = &world.states[city.state];
                let realm = world.varieties[world.communities[s.rulers].variety]
                    .title(s.name.form_at(generation));
                let mut annal = entry(
                    generation,
                    "city",
                    format!(
                        "{}, where {realm} kept its court, grew into a great city, drawing people from across the realm.",
                        place(world, city.region, generation)
                    ),
                    &[s.rulers],
                    &[city.region],
                );
                annal.states.push(city.state);
                annal
            }
            WorldEvent::Koine {
                city,
                community,
                variety,
            } => {
                let city = &world.cities[city];
                let mut annal = entry(
                    generation,
                    "koine",
                    format!(
                        "In {} the speech of many peoples ran together, and its townsfolk, the {}, came to speak a tongue of their own, {}.",
                        place(world, city.region, generation),
                        name(community),
                        world.language_title_at(variety, generation),
                    ),
                    &[world.states[city.state].rulers, community],
                    &[city.region],
                );
                annal.notes.push(format!(
                    "A koiné of {}.",
                    world.varieties[variety]
                        .koine_of
                        .iter()
                        .map(|&(v, share)| format!(
                            "{} ({:.0}%)",
                            language_label(world, v),
                            share * 100.0
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                annal.states.push(city.state);
                annal.variety = Some(variety);
                annal
            }
            WorldEvent::Learnt {
                community,
                craft,
                from,
            } => {
                let teacher = from.map(name).unwrap_or_default();
                let options = match from {
                    Some(_) => learnt_from(craft),
                    None => came_upon(craft),
                };
                let mut annal = entry(
                    generation,
                    "craft",
                    tell(
                        world,
                        &[key("craft"), g, community as u64, craft as u64],
                        options,
                        &[("p", &name(community)), ("t", &teacher)],
                    ),
                    &[Some(community), from]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                    &[],
                );
                annal.notes = new_words(
                    world,
                    variety_at(world, community, position),
                    Need::Craft(craft),
                    generation,
                );
                annal.crafts = vec![craft];
                annal
            }
            WorldEvent::Revealed { religion } => faith_annal(world, generation, religion),
            WorldEvent::Schism {
                religion,
                parent,
                community,
                cause,
                doctrine,
            } => {
                let r = &world.religions[religion];
                let (people, branch, elder) = (
                    name(community),
                    world.faith_name(religion),
                    world.faith_name(parent),
                );
                use umran_sim::schisms::SchismCause;
                let text = match cause {
                    SchismCause::Distance => format!(
                        "Far from the first faithful, the {people} came to keep {elder} their own way, and called it {branch}."
                    ),
                    SchismCause::Rule => format!(
                        "The {people} would answer to no church beyond their own realm, and {branch} broke from {elder}."
                    ),
                    SchismCause::Reform => format!(
                        "Among the {people}, who no longer understood the sacred speech of {elder}, reformers began {branch} and taught in their own tongue."
                    ),
                    SchismCause::Succession => format!(
                        "Over who should follow the founder, {branch} broke from {elder} among the {people}."
                    ),
                };
                let mut annal = entry(generation, "schism", text, &[community], &[r.land]);
                annal.religions = vec![religion, parent];
                annal.notes = vec![format!("{branch} means “{}”.", r.name.meaning)];
                annal.doctrine = doctrine.map(|d| DoctrineAnnal {
                    tenet: d.tenet,
                    previous: Some(d.previous),
                    stance: d.stance,
                });
                annal
            }
            WorldEvent::TenetAdopted {
                religion,
                tenet,
                previous,
                stance,
            } => {
                let faith = world.faith_name(religion);
                let held = tenet_held(tenet, stance);
                let mut annal = entry(
                    generation,
                    "tenet-adopted",
                    match previous {
                        None => format!("From its beginning, {faith} {held}."),
                        Some(_) => format!("By this time, {faith} {held}."),
                    },
                    &[],
                    &[],
                );
                annal.religions = vec![religion];
                annal.doctrine = Some(DoctrineAnnal {
                    tenet,
                    previous,
                    stance,
                });
                annal
            }
            WorldEvent::TenetDisputed {
                religion,
                parent,
                tenet,
                previous,
                stance,
            } => {
                let mut annal = entry(
                    generation,
                    "tenet-disputed",
                    format!(
                        "{} parted from {} over {}: it {}.",
                        world.faith_name(religion),
                        world.faith_name(parent),
                        tenet_noun(tenet),
                        tenet_held(tenet, stance)
                    ),
                    &[],
                    &[],
                );
                annal.religions = vec![religion, parent];
                annal.doctrine = Some(DoctrineAnnal {
                    tenet,
                    previous: Some(previous),
                    stance,
                });
                annal
            }
            WorldEvent::TabooReplaced {
                religion,
                community,
                variety,
                concept,
                old,
                word,
            } => {
                let v = &world.varieties[variety];
                let mut annal = entry(
                    generation,
                    "taboo-replacement",
                    format!(
                        "Among the {}, {} forbade the old word for '{}', *{}*, and they said *{}* instead.",
                        name(community),
                        world.faith_name(religion),
                        concept.gloss,
                        v.spell(v.lexicon.get(old).form_at(generation)),
                        v.spell(v.lexicon.get(word).form_at(generation))
                    ),
                    &[community],
                    &[],
                );
                annal.religions = vec![religion];
                annal.variety = Some(variety);
                annal.taboo = Some(TabooAnnal {
                    concept: concept.id,
                    old: old.0,
                    word: word.0,
                });
                annal
            }
            WorldEvent::Pilgrimage {
                religion,
                community,
                from,
                to,
                ..
            } => {
                let mut annal = entry(
                    generation,
                    "pilgrimage",
                    format!(
                        "Pilgrims of {} first came from the lands of the {} to {}.",
                        world.faith_name(religion),
                        name(community),
                        place(world, to, generation)
                    ),
                    &[community],
                    &[from, to],
                );
                annal.religions = vec![religion];
                annal.notes.push("They crossed the sea to reach it.".into());
                annal
            }
            WorldEvent::HolyLand {
                religion,
                region,
                was_held_by,
                held_by,
                faithful,
            } => {
                let mut peoples: Vec<_> = was_held_by.into_iter().chain(held_by).collect();
                peoples.sort_unstable();
                peoples.dedup();
                let mut annal = entry(
                    generation,
                    "holy-land",
                    if faithful {
                        format!(
                            "{} returned to the keeping of the faithful of {}.",
                            place(world, region, generation),
                            world.faith_name(religion)
                        )
                    } else {
                        format!(
                            "{} passed out of the keeping of the faithful of {}.",
                            place(world, region, generation),
                            world.faith_name(religion)
                        )
                    },
                    &peoples,
                    &[region],
                );
                annal.religions = vec![religion];
                annal
                    .notes
                    .push("A land keeps the faith of the largest people living there.".into());
                annal
            }
            WorldEvent::Converted {
                community,
                religion,
                from,
            } => {
                let faith = world.faith_name(religion);
                let teacher = from.map(name).unwrap_or_default();
                let mut annal = entry(
                    generation,
                    "conversion",
                    tell(
                        world,
                        &[key("conversion"), g, community as u64],
                        if from.is_some() {
                            CONVERTED_BY
                        } else {
                            CONVERTED
                        },
                        &[("p", &name(community)), ("t", &teacher), ("r", &faith)],
                    ),
                    &[Some(community), from]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>(),
                    &[],
                );
                annal.notes = new_words(
                    world,
                    variety_at(world, community, position),
                    Need::Faith,
                    generation,
                );
                annal.religions = vec![religion];
                annal
            }
            WorldEvent::Coined {
                community,
                variety,
                word,
                from,
            } => {
                let v = &world.varieties[variety];
                let word = v.lexicon.get(word);
                let mut annal = entry(
                    generation,
                    if from.is_some() { "calque" } else { "coinage" },
                    match from {
                        Some(donor) => format!(
                            "The {} made *{}* for '{}', translating the word of {} part by part.",
                            name(community),
                            v.spell(word.form_at(generation)),
                            word.first_sense.gloss,
                            world.language_title_at(donor, generation)
                        ),
                        None => format!(
                            "The {} made a word of their own for '{}': *{}*.",
                            name(community),
                            word.first_sense.gloss,
                            v.spell(word.form_at(generation))
                        ),
                    },
                    &[community],
                    &[],
                );
                annal.variety = Some(variety);
                annal.languages.extend(from);
                if let Some(coined) = &word.coined {
                    annal.notes.push(format!(
                        "Built from {}.",
                        coined
                            .parts
                            .iter()
                            .map(|p| format!("*{}* '{}'", v.spell(&p.form), p.concept.gloss))
                            .collect::<Vec<_>>()
                            .join(" and ")
                    ));
                }
                annal
            }
            WorldEvent::Pejorated {
                community,
                variety,
                word,
                from,
                to,
            } => {
                let v = &world.varieties[variety];
                let said = v.spell(v.lexicon.get(word).form_at(generation));
                let mut annal = entry(
                    generation,
                    "meaning",
                    tell(
                        world,
                        &[key("meaning"), g, community as u64, key(from.id)],
                        PEJORATED,
                        &[
                            ("p", &name(community)),
                            ("w", &said),
                            ("from", from.gloss),
                            ("to", to.gloss),
                        ],
                    ),
                    &[community],
                    &[],
                );
                annal.variety = Some(variety);
                annal.notes = vec![match from.id {
                    "god" => "With the new faith the old gods became demons, as Greek daimōn, a divine power, became the Christian demon, and Iranian daēva became a devil while Sanskrit deva still means a god.".into(),
                    _ => "The old priests became sorcerers, as the Persian magi, priests of the old faith, gave the Greeks their word for magic.".into(),
                }];
                annal.religions = world.communities[community].faith.into_iter().collect();
                annal
            }
            WorldEvent::Respelled { variety } => {
                let tongue = world.language_title_at(variety, generation);
                let mut annal = entry(
                    generation,
                    "respelling",
                    tell(
                        world,
                        &[key("respelling"), g, variety as u64],
                        RESPELLED,
                        &[("l", &tongue)],
                    ),
                    &[],
                    &[],
                );
                annal.variety = Some(variety);
                annal.notes = vec![
                    "Its spelling had fallen far behind its speech; now words are written as they are said, until speech moves on again.".into(),
                ];
                annal
            }
            WorldEvent::Fixed { state } => state_annal(world, generation, state, "classical"),
            WorldEvent::Vernacular { variety, by } => {
                vernacular_annal(world, generation, variety, by)
            }
            WorldEvent::PuristReform { variety, episode } => {
                let reform = &world.varieties[variety].purism[episode];
                let mut annal = entry(
                    generation,
                    "purist-reform",
                    format!(
                        "The keepers of written {} struck out {} borrowed {} and wrote native ones in their place.",
                        world.language_title_at(variety, generation),
                        reform.replaced.len(),
                        if reform.replaced.len() == 1 {
                            "word"
                        } else {
                            "words"
                        }
                    ),
                    &[],
                    &[],
                );
                annal.variety = Some(variety);
                annal.notes = reform
                    .replaced
                    .iter()
                    .take(3)
                    .map(|w| format!("*{}* for *{}*", w.native_form, w.loan_form))
                    .collect();
                annal.states.extend(world.classical_of(variety));
                annal
            }
            WorldEvent::PuristReplacement {
                variety,
                episode,
                replacement,
            } => {
                let word = &world.varieties[variety].purism[episode].replaced[replacement];
                let mut annal = entry(
                    generation,
                    "purist-replacement",
                    format!(
                        "In written {}, *{}* took the place of the borrowed *{}* for '{}'.",
                        world.language_title_at(variety, generation),
                        word.native_form,
                        word.loan_form,
                        umran_sim::concepts::by_id(word.concept).map_or(word.concept, |c| c.gloss)
                    ),
                    &[],
                    &[],
                );
                annal.variety = Some(variety);
                annal
            }
            WorldEvent::Temper {
                community,
                axis,
                pole,
                entered,
                cause,
            } => {
                let mut annal = entry(
                    generation,
                    "temper",
                    tell(
                        world,
                        &[key("temper"), g, community as u64, key(axis.id())],
                        if entered { TEMPER_ENTERED } else { TEMPER_LEFT },
                        &[
                            ("c", temper_cause(cause)),
                            ("p", &name(community)),
                            ("w", temper_word(axis, pole)),
                        ],
                    ),
                    &[community],
                    &[],
                );
                annal.temper = Some(Temper {
                    axis,
                    pole,
                    entered,
                    cause,
                });
                annal
            }
        };
        annal.id = world_event_id(position);
        annal.cause = world.causes.get(&position).copied();
        let next = world
            .decisions
            .partition_point(|d| d.events.end <= position);
        annal.decision = world
            .decisions
            .get(next)
            .filter(|d| d.events.contains(&position))
            .map(|d| d.action);
        if matches!(annal.kind, "neighbours" | "spread") {
            grouped
                .entry((annal.kind, generation))
                .or_default()
                .push(annal);
        } else {
            out.push(annal);
        }
    }
    out.extend(neighbours.into_iter().map(|(generation, n)| {
        let mut annal = neighbours_annal(world, generation, &n);
        annal.id = format!("neighbours:{generation}");
        annal.members = grouped
            .remove(&("neighbours", generation))
            .unwrap_or_default();
        annal
    }));
    out.extend(spreads.into_iter().map(|(generation, s)| {
        let mut annal = spread_annal(world, generation, &s);
        annal.id = format!("spread:{generation}");
        annal.members = grouped.remove(&("spread", generation)).unwrap_or_default();
        annal
    }));
    out.extend(sound_changes(world, &shifts));
    out.extend(grammar_changes(world, &shifts));
    out.extend(pronoun_changes(world, &shifts));
    out.extend(class_changes(world, &shifts));
    out.extend(harmony_changes(world, &shifts));
    fn languages(a: &mut Annal, world: &World, shifts: &Shifts) {
        for member in &mut a.members {
            languages(member, world, shifts);
        }
        a.languages = if !a.members.is_empty() {
            a.members
                .iter()
                .flat_map(|m| m.languages.iter().copied())
                .collect()
        } else if let Some(position) =
            a.id.strip_prefix("world:")
                .and_then(|p| p.parse::<usize>().ok())
        {
            let mut languages: Vec<_> = a
                .peoples
                .iter()
                .map(|&c| shifts.spoken_after(world, c, position))
                .collect();
            if let WorldEvent::Shift { from, variety, .. } = world.events[position].1 {
                languages.extend([from, variety]);
            }
            if let WorldEvent::Fixed { state } = world.events[position].1 {
                languages.extend(world.states[state].classical.map(|c| c.variety));
            }
            languages
        } else {
            a.peoples
                .iter()
                .map(|&c| shifts.spoken_by(world, c, a.generation))
                .collect()
        };
        a.languages.extend(a.variety);
        a.languages.sort_unstable();
        a.languages.dedup();
    }
    for a in &mut out {
        languages(a, world, &shifts);
        if let Some(first) = a.members.first() {
            a.decision = first.decision;
        }
        a.before = a
            .decision
            .map(|action| umran_sim::HistoryPoint { action, offset: 0 });
        for member in &mut a.members {
            member.before = member
                .decision
                .map(|action| umran_sim::HistoryPoint { action, offset: 0 });
        }
    }
    out.sort_by_key(|a| (a.generation, a.kind == "law"));
    out
}

const RISE_CONQUEST: &[&str] = &[
    "With that conquest began {n}, “{m}”, with its court at {c}.",
    "So the {r} came to rule a realm, which they called {n}, “{m}”.",
];
const RISE_HARD_TIMES: &[&str] = &[
    "After the bad years the {r} set a ruler over themselves, and called their realm {n}, “{m}”.",
    "After hard times, the {r} gathered under one ruler and called their realm {n}, “{m}”.",
];
const RISE_CROWDED: &[&str] = &[
    "Pressed off their lands, the {r} gathered under one rule and called their realm {n}, “{m}”.",
];
const RISE_NEIGHBOUR: &[&str] = &[
    "Seeing the strength of the realms beside them, the {r} too took a ruler, and called their realm {n}, “{m}”.",
    "The {r} answered their mighty neighbours by becoming a realm themselves: {n}, “{m}”, with its court at {c}.",
];
const RISE_COMFORT: &[&str] =
    &["In years of plenty the {r} raised a court at {c} and called their realm {n}, “{m}”."];
const RISE_PROCLAIMED: &[&str] =
    &["The {r} were made one realm, {n}, “{m}”, with its court at {c}."];
const FELL_RULERS_ENDED: &[&str] = &["{n} ended with the {r} who had ruled it."];
const FELL_CAPITAL_LOST: &[&str] = &[
    "The {r} lost {c}, and {n} fell with it.",
    "When {c} was lost to the {r}, {n} was no more.",
];
const FELL_CONQUERED: &[&str] =
    &["The {b} conquered the {r}, and {n} was no more; its peoples passed under the {b}."];
const FELL_COLLAPSED: &[&str] = &[
    "{n} came apart, and its peoples went their own ways.",
    "The court at {c} lost its hold, and {n} broke apart.",
];
const STANDARD: &[&str] = &["In {n}, {l} became the standard."];

/// A state rising, falling, or taking a standard, as `kind` says.
fn state_annal(world: &World, generation: u32, state: usize, kind: &'static str) -> Annal {
    let s = &world.states[state];
    let rulers = world.community_name_at(s.rulers, generation);
    let variety = world.communities[s.rulers].variety;
    let realm = world.varieties[variety].title(s.name.form_at(generation));
    let court = world
        .place_at(s.capital, generation)
        .unwrap_or_else(|| "their heart land".into());
    let tongue = world.language_title_at(world.standard_variety(state), generation);
    let mut notes = Vec::new();
    let mut peoples = vec![s.rulers];
    let options = match kind {
        "rose" => match s.how {
            Rise::Conquest => RISE_CONQUEST,
            Rise::Challenge(Challenge::HardTimes) => RISE_HARD_TIMES,
            Rise::Challenge(Challenge::Crowded) => RISE_CROWDED,
            Rise::Challenge(Challenge::Neighbour) => RISE_NEIGHBOUR,
            Rise::Challenge(Challenge::Comfort) => RISE_COMFORT,
            Rise::Proclaimed => RISE_PROCLAIMED,
        },
        "fell" => match s.fell.map(|(_, how)| how) {
            Some(Fall::RulersEnded) => FELL_RULERS_ENDED,
            Some(Fall::CapitalLost) => FELL_CAPITAL_LOST,
            Some(Fall::Conquered { by }) => {
                peoples.push(by);
                FELL_CONQUERED
            }
            _ => FELL_COLLAPSED,
        },
        "classical" => {
            let classical = s.classical.expect("a fixed state has a classical form");
            notes.push(format!(
                "{} stays as it stood; everyday {tongue} now changes freely, and its speakers borrow learned words from the written form.",
                crate::language_label(world, classical.variety)
            ));
            match classical.how {
                Fixing::Age => FIXED_AGE,
                Fixing::Fall => FIXED_FALL,
                Fixing::Purism => &["The keepers fixed {l} for writing."],
            }
        }
        _ => {
            notes.push(format!(
                "{tongue} now changes slowly, and the kindred speech of the realm levels toward it."
            ));
            if s.purism >= 0.5 {
                notes.push(
                    "A purist standard: it keeps foreign words out and makes its own.".into(),
                );
            }
            STANDARD
        }
    };
    let conqueror = peoples
        .get(1)
        .map(|&b| world.community_name_at(b, generation))
        .unwrap_or_default();
    Annal {
        id: String::new(),
        members: Vec::new(),
        languages: Vec::new(),
        generation,
        kind,
        text: tell(
            world,
            &[key(kind), u64::from(generation), state as u64],
            options,
            &[
                ("r", &rulers),
                ("n", &realm),
                ("m", &s.name.meaning),
                ("c", &court),
                ("l", &tongue),
                ("b", &conqueror),
            ],
        ),
        notes,
        cause: None,
        variety: None,
        peoples,
        lands: vec![s.capital],
        laws: Vec::new(),
        specimen: Vec::new(),
        states: vec![state],
        religions: Vec::new(),
        crafts: Vec::new(),
        temper: None,
        doctrine: None,
        taboo: None,
        grammar: None,
        zones: Vec::new(),
        rivers: Vec::new(),
        climate: None,
        river_flow: None,
        settlement: None,
        decision: None,
        before: None,
    }
}

const FIXED_AGE: &[&str] = &[
    "The grammarians of {c} fixed how {l} should be written; from then on men wrote as their forefathers had spoken.",
    "In {n}, the schools settled {l} as it stood, and taught it so ever after.",
];
const FIXED_FALL: &[&str] = &[
    "{n} had fallen, but its writing outlived it: {l} was still written as the court at {c} had spoken it.",
    "With {n} gone, its tongue lived on in writing, unchanging, while speech went its own ways.",
];
const VERNACULAR_STANDARD: &[&str] = &[
    "The {p} began to write {l} as they spoke it, and no longer {k}.",
    "In {n}, men put aside {k} and wrote {l}, the speech of the court.",
];
const VERNACULAR_SCRIPTURE: &[&str] = &[
    "The {p} read {f} in their own speech, and began to write {l} as it was spoken.",
    "The teaching of {f} was put into {l}, and the {p} began to write their own tongue.",
];

/// Speakers of `variety` beginning to write their own speech in place of
/// a classical form.
fn vernacular_annal(world: &World, generation: u32, variety: usize, by: Vernacular) -> Annal {
    let tongue = world.language_title_at(variety, generation);
    let high = world.varieties[variety]
        .high
        .map(|h| crate::language_label(world, h))
        .unwrap_or_default();
    let peoples: Vec<usize> = (0..world.communities.len())
        .filter(|&c| world.communities[c].variety == variety)
        .collect();
    let people = peoples
        .first()
        .map(|&c| world.community_name_at(c, generation))
        .unwrap_or_default();
    let (options, states, religions, realm, faith) = match by {
        Vernacular::Standard { state } => {
            let s = &world.states[state];
            let realm = world.varieties[world.communities[s.rulers].variety]
                .title(s.name.form_at(generation));
            (
                VERNACULAR_STANDARD,
                vec![state],
                Vec::new(),
                realm,
                String::new(),
            )
        }
        Vernacular::Scripture { religion } => {
            let faith = world.faith_name(religion);
            (
                VERNACULAR_SCRIPTURE,
                Vec::new(),
                vec![religion],
                String::new(),
                faith,
            )
        }
    };
    Annal {
        id: String::new(),
        members: Vec::new(),
        languages: Vec::new(),
        generation,
        kind: "vernacular",
        text: tell(
            world,
            &[key("vernacular"), u64::from(generation), variety as u64],
            options,
            &[
                ("p", &people),
                ("l", &tongue),
                ("k", &high),
                ("n", &realm),
                ("f", &faith),
            ],
        ),
        notes: vec![format!(
            "{tongue} is written in its own right from now on; {high} still lends it learned words, more slowly."
        )],
        cause: None,
        variety: Some(variety),
        peoples,
        lands: Vec::new(),
        laws: Vec::new(),
        specimen: Vec::new(),
        states,
        religions,
        crafts: Vec::new(),
        temper: None,
        doctrine: None,
        taboo: None,
        grammar: None,
        zones: Vec::new(),
        rivers: Vec::new(),
        climate: None,
        river_flow: None,
        settlement: None,
        decision: None,
        before: None,
    }
}

fn learnt_from(craft: Craft) -> &'static [&'static str] {
    match craft {
        Craft::Metalworking => &[
            "The {p} learnt from the {t} to work bronze and iron.",
            "Smiths of the {t} taught the {p} their craft.",
        ],
        Craft::Riding => &[
            "The {p} learnt from the {t} to ride.",
            "The {p} took horses and riding from the {t}.",
        ],
        Craft::Seafaring => &[
            "The {p} learnt from the {t} to build ships and sail.",
            "Sailors of the {t} taught the {p} the ways of the sea.",
        ],
        Craft::Writing => &[
            "The {p} learnt their letters from the {t}.",
            "Scribes of the {t} taught the {p} to write.",
        ],
    }
}

fn came_upon(craft: Craft) -> &'static [&'static str] {
    match craft {
        Craft::Metalworking => &["The {p} learnt to work metal."],
        Craft::Riding => &["The {p} broke horses and began to ride."],
        Craft::Seafaring => &["The {p} built ships and put out to sea."],
        Craft::Writing => &["Among the {p}, words were first set down in signs."],
    }
}

const FOUNDED_TROUBLES: &[&str] = &[
    "In the hard years {f} of the {p} began to teach, and the teaching was called {r}, “{m}”.",
    "Out of the troubles of those days came {f}, one of the {p}, whose followers named their faith {r}, “{m}”.",
];
const FOUNDED_QUIET: &[&str] =
    &["Among the {p} there arose a teacher, {f}, whose teaching was called {r}, “{m}”."];
const FOUNDED_PROCLAIMED: &[&str] = &["{f} of the {p} taught a new faith, {r}, “{m}”."];
const CONVERTED_BY: &[&str] = &[
    "The {p} took up {r} from the {t}.",
    "Teachers from among the {t} brought {r} to the {p}.",
];
const CONVERTED: &[&str] = &["The {p} took up {r}."];
const PEJORATED: &[&str] = &[
    "Among the {p}, *{w}*, once “{from}”, came to mean “{to}”.",
    "Among the {p}, *{w}* no longer meant “{from}” but “{to}”.",
];
const RESPELLED: &[&str] = &[
    "{l} was written anew, as it was then spoken.",
    "The scribes set aside the old spellings of {l} and wrote it as it was said.",
];

/// A religion's founding.
fn faith_annal(world: &World, generation: u32, religion: usize) -> Annal {
    let r = &world.religions[religion];
    let sacred = &world.varieties[r.sacred];
    let faith = world.faith_name(religion);
    let founder = sacred.title(&r.founder.form);
    let people = world.community_name_at(r.people, generation);
    let options = match r.how {
        Revelation::Troubles => FOUNDED_TROUBLES,
        Revelation::Quiet => FOUNDED_QUIET,
        Revelation::Proclaimed => FOUNDED_PROCLAIMED,
    };
    let mut notes = vec![
        format!("The name {founder} means “{}”.", r.founder.meaning),
        if r.converts {
            "It seeks converts.".into()
        } else {
            "It keeps to its own people.".into()
        },
        if r.translates {
            "Converts say its words in their own speech, built from their own words.".into()
        } else {
            format!(
                "Converts take its words from {}, which it keeps as its sacred language.",
                language_label(world, r.sacred)
            )
        },
    ];
    if r.scripture {
        notes.push("Its teaching is written down.".into());
    }
    notes.extend(new_words(world, r.sacred, Need::Faith, generation));
    Annal {
        id: String::new(),
        members: Vec::new(),
        languages: Vec::new(),
        generation,
        kind: "faith",
        text: tell(
            world,
            &[key("faith"), u64::from(generation), religion as u64],
            options,
            &[
                ("f", &founder),
                ("p", &people),
                ("r", &faith),
                ("m", &r.name.meaning),
            ],
        ),
        notes,
        cause: None,
        variety: None,
        peoples: vec![r.people],
        lands: vec![r.land],
        laws: Vec::new(),
        specimen: Vec::new(),
        states: Vec::new(),
        religions: vec![religion],
        crafts: Vec::new(),
        temper: None,
        doctrine: None,
        taboo: None,
        grammar: None,
        zones: Vec::new(),
        rivers: Vec::new(),
        climate: None,
        river_flow: None,
        settlement: None,
        decision: None,
        before: None,
    }
}

/// The language `community` spoke when event `position` happened: the one
/// its next shift left, or the one it speaks now.
fn variety_at(world: &World, community: usize, position: usize) -> usize {
    world.events[position..]
        .iter()
        .find_map(|(_, e)| match *e {
            WorldEvent::Shift {
                community: c, from, ..
            } if c == community => Some(from),
            _ => None,
        })
        .unwrap_or(world.communities[community].variety)
}

/// The first word `variety` had for `concept`, and when it took that
/// meaning on.
fn first_word<'a>(variety: &'a Variety, concept: &Concept) -> Option<(&'a Lexeme, u32)> {
    variety
        .lexicon
        .lexemes
        .iter()
        .filter_map(|l| {
            let since = if l.first_sense.id == concept.id {
                Some(l.born)
            } else {
                l.log.iter().find_map(|e| match e.event {
                    Event::Extended { to } if to.id == concept.id => Some(e.generation),
                    _ => None,
                })
            };
            since.map(|g| (l, g))
        })
        .min_by_key(|&(_, g)| g)
}

/// The words `variety` came by for `need`'s meanings when its speakers
/// took the idea up at `generation`, and how each came: a note, or none if
/// the language already had them.
fn new_words(world: &World, variety: usize, need: Need, generation: u32) -> Vec<String> {
    let v = &world.varieties[variety];
    let words: Vec<String> = NEEDS
        .iter()
        .filter(|(_, n)| *n == need)
        .filter_map(|(id, _)| umran_sim::concepts::by_id(id))
        .filter_map(|concept| {
            let (word, since) = first_word(v, concept)?;
            if since < generation || since > generation + 1 {
                return None;
            }
            let said = v.spell(word.form_at(since));
            let how = if word.first_sense.id != concept.id {
                format!("stretched from “{}”", word.first_sense.gloss)
            } else {
                match word.origin {
                    Origin::Borrowed { from, .. } => {
                        format!("from {}", language_label(world, from))
                    }
                    Origin::Derived { base, .. } => {
                        format!("built on *{}*", v.spell(v.lexicon.get(base).form_at(since)))
                    }
                    _ => "a new word".into(),
                }
            };
            Some(format!("“{}” *{said}*, {how}", concept.gloss))
        })
        .collect();
    if words.is_empty() {
        Vec::new()
    } else {
        vec![format!("New words: {}.", words.join("; "))]
    }
}

const DISPLACED: &[&str] = &[
    "The {p} were driven from {land} by the {b}.",
    "The {b} crowded the {p} out of {land}.",
];

const FAMINE: &[&str] = &[
    "Famine came upon {land}, and {share} of those who lived there perished.",
    "The harvests failed in {land}; {share} of its people starved.",
];

const PLAGUE: &[&str] = &[
    "A plague swept through {land}, and {share} of its people died.",
    "Sickness came to {land} and carried off {share} of those who lived there.",
];

const LEARNED: &[&str] = &[
    "The {p} learned {way} from the {t}.",
    "The {p} took up {way}, as the {t} did.",
];

const BEGAN_FARMING: &[&str] = &[
    "The {p} began to till the soil and sow.",
    "Among the {p}, some first planted seed and waited for the harvest.",
];

const BEGAN_HERDING: &[&str] = &["The {p} took up herding."];

const BEGAN_FORAGING: &[&str] =
    &["The {p} gave up their old ways and lived by hunting and gathering."];

const DIED_OUT: &[&str] = &["The {p} dwindled and were no longer counted as a separate people."];

const MERGED: &[&str] = &[
    "The {p} were absorbed among the {i}.",
    "The {p} merged into the {i} and were no longer counted apart.",
];

/// A share of a people, as the annalist would say it.
fn share_in_words(share: f32) -> &'static str {
    match share {
        s if s < 0.23 => "a fifth",
        s if s < 0.29 => "a quarter",
        s if s < 0.38 => "a third",
        s if s < 0.46 => "nearly half",
        _ => "half",
    }
}

/// Most lands an entry on spreading names before only counting them.
const SPREADS_NAMED: usize = 3;

/// One entry for every people spreading into new land in `generation`,
/// told together, and when there are many only counted, with each in the
/// apparatus.
fn spread_annal(world: &World, generation: u32, spreads: &[(usize, usize)]) -> Annal {
    let name = |c: usize| world.community_name_at(c, generation);
    let land = |r: usize| place(world, r, generation);
    let mut notes = Vec::new();
    let text = if spreads.len() > SPREADS_NAMED {
        notes.extend(
            spreads
                .iter()
                .map(|&(c, r)| format!("The {} into {}.", name(c), land(r))),
        );
        format!(
            "That year peoples spread into {} new lands.",
            number(spreads.len())
        )
    } else {
        let told: Vec<String> = spreads
            .iter()
            .enumerate()
            .map(|(i, &(c, r))| {
                let verb = if i == 0 { "spread into" } else { "into" };
                format!("the {} {verb} {}", name(c), land(r))
            })
            .collect();
        let joined = match told.split_last() {
            Some((last, init)) if !init.is_empty() => format!("{}, and {last}", init.join(", ")),
            _ => told.concat(),
        };
        format!("That year {joined}.")
    };
    let mut peoples: Vec<usize> = Vec::new();
    for &(c, _) in spreads {
        if !peoples.contains(&c) {
            peoples.push(c);
        }
    }
    Annal {
        id: String::new(),
        members: Vec::new(),
        languages: Vec::new(),
        generation,
        kind: "spread",
        text,
        notes,
        cause: None,
        variety: None,
        peoples,
        lands: spreads.iter().map(|&(_, r)| r).collect(),
        laws: Vec::new(),
        specimen: Vec::new(),
        states: Vec::new(),
        religions: Vec::new(),
        crafts: Vec::new(),
        temper: None,
        doctrine: None,
        taboo: None,
        grammar: None,
        zones: Vec::new(),
        rivers: Vec::new(),
        climate: None,
        river_flow: None,
        settlement: None,
        decision: None,
        before: None,
    }
}

/// Pairs of peoples that came to live beside one another, and that
/// drifted apart, in one generation.
#[derive(Default)]
struct Neighbours {
    met: Vec<(usize, usize)>,
    parted: Vec<(usize, usize)>,
}

/// Most pairs an entry on neighbours names before only counting them.
const NEIGHBOURS_NAMED: usize = 3;

/// One entry for every change among neighbours in `generation`. A single
/// change is told as any other dealing is; several are told together, and
/// many only counted, with each pair in the apparatus.
fn neighbours_annal(world: &World, generation: u32, n: &Neighbours) -> Annal {
    let name = |c: usize| world.community_name_at(c, generation);
    let g = u64::from(generation);
    let single = |kind: &str, options: &[&str], (a, b): (usize, usize)| {
        tell(
            world,
            &[key(kind), g, a as u64, b as u64],
            options,
            &[("a", &name(a)), ("b", &name(b))],
        )
    };
    // "the A came to live beside the B, and the C beside the D"
    let clause = |pairs: &[(usize, usize)], first: &str, rest: &str, counted: &str| {
        if pairs.len() > NEIGHBOURS_NAMED {
            return format!("{} pairs of peoples {counted}", number(pairs.len()));
        }
        let told: Vec<String> = pairs
            .iter()
            .enumerate()
            .map(|(i, &(a, b))| {
                let verb = if i == 0 { first } else { rest };
                format!("the {} {verb} the {}", name(a), name(b))
            })
            .collect();
        match told.split_last() {
            Some((last, init)) if !init.is_empty() => format!("{}, and {last}", init.join(", ")),
            _ => told.concat(),
        }
    };
    let mut notes = Vec::new();
    let text =
        match (n.met.as_slice(), n.parted.as_slice()) {
            (&[pair], []) => single("contact", contact_wording(ContactKind::Neighbours), pair),
            ([], &[pair]) => single("parted", parting_wording(ContactKind::Neighbours), pair),
            (met, parted) => {
                let mut clauses = Vec::new();
                if !met.is_empty() {
                    clauses.push(clause(
                        met,
                        "came to live beside",
                        "beside",
                        "came to live as neighbours",
                    ));
                }
                if !parted.is_empty() {
                    clauses.push(clause(
                        parted,
                        "drifted apart from",
                        "from",
                        "drifted apart",
                    ));
                }
                for (pairs, what) in [(met, "Neighbours"), (parted, "Apart")] {
                    if pairs.len() > NEIGHBOURS_NAMED {
                        notes.extend(pairs.iter().map(|&(a, b)| {
                            format!("{what}: the {} and the {}.", name(a), name(b))
                        }));
                    }
                }
                format!("That year {}.", clauses.join("; "))
            }
        };
    let mut peoples: Vec<usize> = Vec::new();
    for &(a, b) in n.met.iter().chain(&n.parted) {
        for c in [a, b] {
            if !peoples.contains(&c) {
                peoples.push(c);
            }
        }
    }
    Annal {
        id: String::new(),
        members: Vec::new(),
        languages: Vec::new(),
        generation,
        kind: "neighbours",
        text,
        notes,
        cause: None,
        variety: None,
        peoples,
        lands: Vec::new(),
        laws: Vec::new(),
        specimen: Vec::new(),
        states: Vec::new(),
        religions: Vec::new(),
        crafts: Vec::new(),
        temper: None,
        doctrine: None,
        taboo: None,
        grammar: None,
        zones: Vec::new(),
        rivers: Vec::new(),
        climate: None,
        river_flow: None,
        settlement: None,
        decision: None,
        before: None,
    }
}

/// `n` in words, as the annalist writes small numbers.
fn number(n: usize) -> String {
    const WORDS: [&str; 13] = [
        "no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        "eleven", "twelve",
    ];
    WORDS
        .get(n)
        .map_or_else(|| n.to_string(), |w| (*w).to_string())
}

/// What `region` was called at the end of `generation`.
fn place(world: &World, region: usize, generation: u32) -> String {
    world
        .place_at(region, generation)
        .unwrap_or_else(|| "a land without a name".into())
}

/// What `region` was called before anything that happened in
/// `generation`, or at its end if it had no name before.
fn place_before(world: &World, region: usize, generation: u32) -> String {
    match generation.checked_sub(1) {
        Some(g) if world.place_at(region, g).is_some() => place(world, region, g),
        _ => place(world, region, generation),
    }
}

/// The linguist's note on how `region` came by the name it took in
/// `generation`: what it meant when coined, or what it was borrowed from.
fn place_note(world: &World, region: usize, generation: u32) -> Option<String> {
    let names = &world.places[region];
    let i = names.iter().rposition(|p| p.since == generation)?;
    let now = &names[i];
    let spelled = world.varieties[now.variety].title(now.name.form_at(generation));
    match now.origin {
        PlaceOrigin::Coined { .. } => Some(format!("*{spelled}*: “{}”.", now.name.meaning)),
        PlaceOrigin::Borrowed => {
            let before = &names[i.checked_sub(1)?];
            Some(format!(
                "*{spelled}*: from {} *{}*, “{}”.",
                world.language_title_at(before.variety, generation),
                world.varieties[before.variety].title(before.name.form_at(generation)),
                before.name.meaning
            ))
        }
        PlaceOrigin::Inherited | PlaceOrigin::Kept => None,
    }
}

/// One entry per language and generation in which its sounds changed,
/// told through a word that changed, with the laws in the apparatus.
fn sound_changes(world: &World, shifts: &Shifts) -> Vec<Annal> {
    let laws = world.law_catalog();
    let label = |id: &str| {
        laws.iter()
            .find(|l| l.id == id)
            .map_or_else(|| substrate_label(id), |l| l.label.to_string())
    };
    let mut out = Vec::new();
    for (v, variety) in world.varieties.iter().enumerate() {
        // A daughter's inherited laws are told in its parent's annals.
        let from = variety.parent.map_or(0, |f| f.generation + 1);
        let mut by_generation: BTreeMap<u32, Vec<&'static str>> = BTreeMap::new();
        for &(generation, id) in variety
            .laws
            .iter()
            .filter(|(g, id)| *g >= from || world.authored_laws.contains_key(&(v, *g, *id)))
        {
            // Authored laws at a daughter's founding year are not inherited.
            by_generation.entry(generation).or_default().push(id);
        }
        for (generation, ids) in by_generation {
            let people = speakers(world, shifts, v, generation);
            let keys = [key("law"), u64::from(generation), v as u64];
            let text = match example(variety, generation) {
                Some((before, after, gloss)) => tell(
                    world,
                    &keys,
                    LAW,
                    &[
                        ("p", &people),
                        ("before", &before),
                        ("after", &after),
                        ("g", gloss),
                    ],
                ),
                None => tell(world, &keys, LAW_UNSEEN, &[("p", &people)]),
            };
            // A change that spread from a neighbour says where it came from.
            let note = |id: &str| match variety
                .waves
                .iter()
                .find(|&&(g, w, _)| (g, w) == (generation, id))
            {
                Some(&(_, _, source)) => format!(
                    "{}, spreading from {}",
                    label(id),
                    speakers(world, shifts, source, generation)
                ),
                None => label(id),
            };
            let decision = ids
                .iter()
                .filter_map(|&id| world.authored_laws.get(&(v, generation, id)))
                .copied()
                .max();
            out.push(Annal {
                id: format!("sounds:{v}:{generation}"),
                members: Vec::new(),
                languages: Vec::new(),
                generation,
                kind: "law",
                text,
                notes: ids.iter().map(|id| note(id)).collect(),
                cause: None,
                variety: Some(v),
                peoples: (0..world.communities.len())
                    .filter(|&c| {
                        shifts.alive_at(world, c, generation)
                            && shifts.spoken_by(world, c, generation) == v
                    })
                    .collect(),
                lands: Vec::new(),
                laws: ids,
                specimen: specimen(variety, generation),
                states: Vec::new(),
                religions: Vec::new(),
                crafts: Vec::new(),
                temper: None,
                doctrine: None,
                taboo: None,
                grammar: None,
                zones: Vec::new(),
                rivers: Vec::new(),
                climate: None,
                river_flow: None,
                settlement: None,
                decision,
                before: None,
            });
        }
    }
    out
}

/// Grammatical changes are told from recorded notices, not inferred from
/// the language's current markers or how its words happen to look.
fn grammar_changes(world: &World, shifts: &Shifts) -> Vec<Annal> {
    let mut out = Vec::new();
    for (v, variety) in world.varieties.iter().enumerate() {
        // Inherited events belong to the parent's annals. A daughter can
        // also acquire grammar during the generation of its own fork.
        for (position, notice) in variety
            .grammar
            .events
            .iter()
            .enumerate()
            .filter(|(_, notice)| {
                variety.parent.is_none_or(|fork| {
                    notice.generation > fork.generation
                        || (notice.generation == fork.generation
                            && !world.varieties[fork.variety]
                                .grammar
                                .events
                                .contains(*notice))
                })
            })
        {
            let generation = notice.generation;
            let people = speakers(world, shifts, v, generation);
            let category_id = notice.category.id();
            let category = notice.category.label();
            let (text, notes, event, donor) = match &notice.event {
                NoticeKind::NewMarker { marker } => {
                    let marker = variety.grammar.marker(*marker);
                    let phrase = grammatical_marker(variety, marker, generation);
                    let text = match marker.kind {
                        MarkerKind::None => format!(
                            "In {}, {category} had no overt grammatical marker.",
                            world.language_title_at(v, generation)
                        ),
                        _ => format!(
                            "In those days {people} began marking {category} with {phrase}."
                        ),
                    };
                    let note = match &marker.origin {
                        MarkerOrigin::Founding => match marker.kind {
                            MarkerKind::None => {
                                "No overt marker is a founding choice, not a defective grammar.".into()
                            }
                            _ => "This marker was part of the founding grammar.".into(),
                        },
                        MarkerOrigin::Grammaticalized {
                            concept,
                            source_form,
                            ..
                        } => format!(
                            "The word *{}*, '{}', /{}/, acquired a grammatical job while its lexical use remained. This development is called grammaticalization.",
                            variety.spell(source_form),
                            concept.gloss,
                            source_form.ipa_stressed(variety.stress_at(marker.born))
                        ),
                        MarkerOrigin::Fused { particle } => format!(
                            "The separate grammatical word became attached to its neighbour: {}. This development is called fusion.",
                            grammatical_marker(
                                variety,
                                variety.grammar.marker(*particle),
                                generation
                            )
                        ),
                        MarkerOrigin::Imported { from, source, .. } => format!(
                            "Imported from {}, where it was heard as /{}/.",
                            world.language_title_at(*from, generation),
                            source.ipa_stressed(world.varieties[*from].stress_at(marker.born))
                        ),
                    };
                    let donor = match &marker.origin {
                        MarkerOrigin::Imported { from, .. } => Some(*from),
                        _ => None,
                    };
                    (
                        text,
                        vec![note],
                        GrammarAnnalEvent::NewMarker { marker: marker.id },
                        donor,
                    )
                }
                NoticeKind::Fusion { particle, marker } => {
                    let old = grammatical_marker(
                        variety,
                        variety.grammar.marker(*particle),
                        generation,
                    );
                    let new = grammatical_marker(
                        variety,
                        variety.grammar.marker(*marker),
                        generation,
                    );
                    (
                        format!(
                            "Among {people}, {old} became attached to its neighbour as {new}, marking {category} within one word."
                        ),
                        vec![
                            "Fusion joins a formerly separate grammatical word to its neighbour. The attached form and the separate form may continue to compete.".into(),
                        ],
                        GrammarAnnalEvent::Fusion {
                            particle: *particle,
                            marker: *marker,
                        },
                        None,
                    )
                }
                NoticeKind::ContrastLoss => (
                    if notice.category == Category::Object {
                        format!("Among {people}, the object's mark weakened; word order carried more of the distinction between subject and object.")
                    } else {
                        format!(
                            "Among {people}, {category} forms increasingly sounded the same as their unmarked words."
                        )
                    },
                    vec![
                        "A grammatical contrast is an audible difference between the base and the marked form. An ending can disappear without losing that contrast if the stem still sounds different; separate grammatical words can preserve it too.".into(),
                    ],
                    GrammarAnnalEvent::ContrastLoss,
                    None,
                ),
                NoticeKind::Analogy {
                    lexeme,
                    before,
                    after,
                } => {
                    let word = variety.lexicon.get(*lexeme);
                    (
                        format!(
                            "The {category_id} form for '{}' was reshaped among {people}, from *{}* to *{}*, to follow the pattern used for new words.",
                            word.first_sense.gloss,
                            variety.spell(before),
                            variety.spell(after)
                        ),
                        vec![format!(
                            "Analogy means reshaping a form to match a productive pattern: /{}/ → /{}/. Its unmarked word was not changed.",
                            before.ipa_stressed(variety.stress_at(generation)),
                            after.ipa_stressed(variety.stress_at(generation))
                        )],
                        GrammarAnnalEvent::Analogy {
                            lexeme: lexeme.0,
                            before: grammar_form_view(variety, before, generation),
                            after: grammar_form_view(variety, after, generation),
                        },
                        None,
                    )
                }
                NoticeKind::ImportedPair { lexeme, from } => {
                    let word = variety.lexicon.get(*lexeme);
                    let language = world.language_title_at(*from, generation);
                    (
                        format!(
                            "The word for '{}' came to {people} from {language} together with its {category_id} form.",
                            word.first_sense.gloss
                        ),
                        vec![
                            "Both the base and its marked form were acquired together. A borrowed marked form does not by itself make the foreign pattern productive on native words.".into(),
                        ],
                        GrammarAnnalEvent::ImportedPair {
                            lexeme: lexeme.0,
                            from: *from,
                        },
                        Some(*from),
                    )
                }
                NoticeKind::MarkerTransfer { marker, from } => {
                    let language = world.language_title_at(*from, generation);
                    let phrase = grammatical_marker(
                        variety,
                        variety.grammar.marker(*marker),
                        generation,
                    );
                    (
                        format!(
                            "In those days {people} took up {phrase} from {language} as a pattern for marking {category} on new words."
                        ),
                        vec![
                            "Productive marker transfer spreads a grammatical pattern beyond the individual words first borrowed with it.".into(),
                        ],
                        GrammarAnnalEvent::MarkerTransfer {
                            marker: *marker,
                            from: *from,
                        },
                        Some(*from),
                    )
                }
            };
            out.push(Annal {
                id: format!("grammar:{v}:{position}"),
                members: Vec::new(),
                languages: Vec::new(),
                generation,
                kind: "grammar",
                text,
                notes,
                cause: None,
                variety: Some(v),
                peoples: (0..world.communities.len())
                    .filter(|&c| {
                        let spoken = shifts.spoken_by(world, c, generation);
                        shifts.alive_at(world, c, generation)
                            && (spoken == v || donor == Some(spoken))
                    })
                    .collect(),
                lands: Vec::new(),
                laws: Vec::new(),
                specimen: Vec::new(),
                states: Vec::new(),
                religions: Vec::new(),
                crafts: Vec::new(),
                temper: None,
                doctrine: None,
                taboo: None,
                grammar: Some(GrammarAnnal {
                    category: category_id,
                    event,
                }),
                zones: Vec::new(),
                rivers: Vec::new(),
                climate: None,
                river_flow: None,
                settlement: None,
                decision: None,
                before: None,
            });
        }
    }
    out
}

fn pronoun_changes(world: &World, shifts: &Shifts) -> Vec<Annal> {
    use umran_sim::pronouns::NoticeKind;
    let mut out = Vec::new();
    for (v, variety) in world.varieties.iter().enumerate() {
        for (position, notice) in variety.pronouns.events.iter().enumerate() {
            if variety.parent.is_some_and(|fork| {
                notice.generation < fork.generation
                    || (notice.generation == fork.generation
                        && world.varieties[fork.variety]
                            .pronouns
                            .events
                            .contains(notice))
            }) {
                continue;
            }
            let generation = notice.generation;
            let people = speakers(world, shifts, v, generation);
            let gloss = |id: &str| umran_sim::concepts::by_id(id).map_or("", |c| c.gloss);
            let (before, after) = (variety.spell(&notice.before), variety.spell(&notice.after));
            let meaning = gloss(notice.cell);
            let (kind, text, note, donor, state) = match &notice.event {
                NoticeKind::Renewed { concept, merger, .. } => (
                    "pronoun-renewed",
                    format!(
                        "Among {people}, *{after}*, once their word for '{}', took the place of *{before}* for '{meaning}'.",
                        gloss(concept)
                    ),
                    format!(
                        "{} Languages renew short, worn pronouns from nouns, as Malay saya, 'I', came from a word for 'servant'.",
                        if *merger {
                            "The old pronoun had come to sound like another."
                        } else {
                            "The old pronoun had worn too short to stand alone."
                        }
                    ),
                    None,
                    None,
                ),
                NoticeKind::Polite { state } => (
                    "pronoun-polite",
                    format!("At court, {people} began to use *{after}* respectfully, keeping *{before}* for familiar singular address."),
                    "Plural address becomes respectful singular address alongside the familiar form, as in French tu and vous.".into(),
                    None,
                    Some(*state),
                ),
                NoticeKind::Generalised => (
                    "pronoun-generalised",
                    format!("Among {people}, respectful *{after}* replaced familiar *{before}* in singular address."),
                    "The former respectful form now serves ordinary address, as English you displaced thou.".into(),
                    None,
                    None,
                ),
                NoticeKind::Borrowed { from, .. } => (
                    "pronoun-borrowed",
                    format!(
                        "In those days {people} took *{after}* for '{meaning}' from {}, in place of *{before}*.",
                        world.language_title_at(*from, generation)
                    ),
                    "Pronouns are rarely borrowed; it takes long and intense contact, as English they came from Norse.".into(),
                    Some(*from),
                    None,
                ),
            };
            out.push(Annal {
                id: format!("pronoun:{v}:{position}"),
                members: Vec::new(),
                languages: Vec::new(),
                generation,
                kind,
                text,
                notes: vec![note],
                cause: notice.cause,
                variety: Some(v),
                peoples: (0..world.communities.len())
                    .filter(|&c| {
                        let spoken = shifts.spoken_by(world, c, generation);
                        shifts.alive_at(world, c, generation)
                            && (spoken == v || donor == Some(spoken))
                    })
                    .collect(),
                states: state.into_iter().collect(),
                lands: Vec::new(),
                laws: Vec::new(),
                specimen: Vec::new(),
                religions: Vec::new(),
                crafts: Vec::new(),
                temper: None,
                doctrine: None,
                taboo: None,
                grammar: None,
                zones: Vec::new(),
                rivers: Vec::new(),
                climate: None,
                river_flow: None,
                settlement: None,
                decision: None,
                before: None,
            });
        }
    }
    out
}

fn harmony_changes(world: &World, shifts: &Shifts) -> Vec<Annal> {
    use umran_sim::harmony::{Feature, Trigger};
    let mut out = Vec::new();
    for (v, variety) in world.varieties.iter().enumerate() {
        for (position, notice) in variety
            .harmony_events
            .iter()
            .enumerate()
            .filter(|(_, notice)| {
                variety.parent.is_none_or(|fork| {
                    notice.generation > fork.generation
                        || (notice.generation == fork.generation
                            && !world.varieties[fork.variety]
                                .harmony_events
                                .contains(notice))
                })
            })
        {
            let generation = notice.generation;
            let people = speakers(world, shifts, v, generation);
            let agree = match notice.feature {
                Feature::Backness => "front with front and back with back",
                Feature::Rounding => "rounded with rounded",
                Feature::Atr => "tense with tense and lax with lax",
            };
            let (text, note, laws, cause, donor) = match notice.trigger {
                Trigger::Assimilation { law, .. } => (
                    format!(
                        "Among {people}, the vowels of a word came to agree, {agree}, and endings changed to match the words they joined."
                    ),
                    "Vowel harmony: a change between neighbouring syllables became a rule for the whole word, as harmony is thought to have grown in Finnic and Turkic.".into(),
                    vec![law],
                    None,
                    None,
                ),
                Trigger::ContrastMerger { law } => (
                    format!(
                        "Among {people}, vowel harmony faded once the vowels it paired had fallen together."
                    ),
                    "Harmony needs the contrast it rests on; Estonian lost most of the Finnic harmony this way.".into(),
                    vec![law],
                    None,
                    None,
                ),
                Trigger::LexicalAttrition => (
                    format!("Among {people}, vowel harmony faded as the words that set its vowels apart fell out of use."),
                    "The pairing wore away through the vocabulary, not through any one sound law.".into(),
                    Vec::new(),
                    None,
                    None,
                ),
                Trigger::Contact { donor, cause } => (
                    format!(
                        "Among {people}, vowel harmony gave way after long contact with {}, which had none.",
                        world.language_title_at(donor, generation)
                    ),
                    "Urban Uzbek lost its palatal harmony this way, under long contact with Persian.".into(),
                    Vec::new(),
                    cause,
                    Some(donor),
                ),
            };
            out.push(Annal {
                id: format!("harmony:{v}:{position}"),
                members: Vec::new(),
                languages: Vec::new(),
                generation,
                kind: if notice.gained {
                    "harmony-gained"
                } else {
                    "harmony-lost"
                },
                text,
                notes: vec![note],
                cause,
                variety: Some(v),
                peoples: (0..world.communities.len())
                    .filter(|&c| {
                        let spoken = shifts.spoken_by(world, c, generation);
                        shifts.alive_at(world, c, generation)
                            && (spoken == v || donor == Some(spoken))
                    })
                    .collect(),
                lands: Vec::new(),
                laws,
                specimen: Vec::new(),
                states: Vec::new(),
                religions: Vec::new(),
                crafts: Vec::new(),
                temper: None,
                doctrine: None,
                taboo: None,
                grammar: None,
                zones: Vec::new(),
                rivers: Vec::new(),
                climate: None,
                river_flow: None,
                settlement: None,
                decision: None,
                before: None,
            });
        }
    }
    out
}

fn class_changes(world: &World, shifts: &Shifts) -> Vec<Annal> {
    use umran_sim::gender::{Basis, ClassChange};
    let mut out = Vec::new();
    for (v, variety) in world.varieties.iter().enumerate() {
        let classes = &variety.gender.classes;
        for (position, notice) in variety
            .gender
            .events
            .iter()
            .enumerate()
            .filter(|(_, notice)| {
                variety.parent.is_none_or(|fork| {
                    notice.generation > fork.generation
                        || (notice.generation == fork.generation
                            && !world.varieties[fork.variety].gender.events.contains(notice))
                })
            })
        {
            let generation = notice.generation;
            let people = speakers(world, shifts, v, generation);
            let said = |class: u32, at: u32| {
                format!(
                    "*{}*",
                    variety.spell(classes[class as usize].agreement_at(at))
                )
            };
            let (kind, text, note) = match &notice.change {
                ClassChange::Emerged { classes: new } => {
                    let by = match new.first().map(|&c| classes[c as usize].basis) {
                        Some(Basis::Sex) => "masculine, feminine, and the rest",
                        Some(Basis::Animacy) => "people, animals, and things",
                        _ => "kind and shape",
                    };
                    let forms: Vec<_> = new.iter().map(|&c| said(c, generation)).collect();
                    (
                        "class-emerged",
                        format!(
                            "Among {people}, the word for 'this' began to agree with its noun, sorting nouns by {by}: {}.",
                            forms.join(", ")
                        ),
                        "Noun classes grow when classifiers or demonstratives become agreement markers (Greenberg, 1978). Agreement is what makes a class visible.",
                    )
                }
                ClassChange::Merged { class, into } => {
                    let before = generation.saturating_sub(1);
                    (
                        "class-merged",
                        format!(
                            "Among {people}, {} and {} came to sound alike, and two noun classes became one.",
                            said(*into, before),
                            said(*class, before)
                        ),
                        "Classes merge when sound change erases the endings that told them apart, as Dutch merged masculine and feminine into common gender.",
                    )
                }
                ClassChange::Lost => (
                    "class-lost",
                    format!(
                        "Among {people}, the last difference between noun classes wore away; 'this' no longer agreed with its noun."
                    ),
                    "Gender is often lost as its endings erode, as English lost it after Old English.",
                ),
            };
            out.push(Annal {
                id: format!("classes:{v}:{position}"),
                members: Vec::new(),
                languages: Vec::new(),
                generation,
                kind,
                text,
                notes: vec![note.into()],
                cause: None,
                variety: Some(v),
                peoples: (0..world.communities.len())
                    .filter(|&c| {
                        shifts.alive_at(world, c, generation)
                            && shifts.spoken_by(world, c, generation) == v
                    })
                    .collect(),
                lands: Vec::new(),
                laws: notice.cause.map(|c| vec![c.law]).unwrap_or_default(),
                specimen: Vec::new(),
                states: Vec::new(),
                religions: Vec::new(),
                crafts: Vec::new(),
                temper: None,
                doctrine: None,
                taboo: None,
                grammar: None,
                zones: Vec::new(),
                rivers: Vec::new(),
                climate: None,
                river_flow: None,
                settlement: None,
                decision: notice.cause.and_then(|cause| {
                    world
                        .authored_laws
                        .get(&(v, cause.generation, cause.law))
                        .copied()
                }),
                before: None,
            });
        }
    }
    out
}

/// What a faith held on a tenet, in the annalist's past tense. Stances
/// within 0.15 of neutral read as undecided.
fn tenet_held(tenet: umran_sim::doctrine::Tenet, stance: f32) -> String {
    use umran_sim::doctrine::Tenet;
    if stance.abs() < 0.15 {
        return format!("was of two minds about {}", tenet_noun(tenet));
    }
    let (yes, no) = match tenet {
        Tenet::SacredLanguage => (
            "held that scripture must be read in its sacred tongue",
            "taught scripture in the speech of its faithful",
        ),
        Tenet::Images => ("venerated images", "forbade images"),
        Tenet::Hierarchy => ("kept an order of priests", "kept no order of priests"),
        Tenet::Purity => (
            "kept strict rules of purity",
            "set little store by rules of purity",
        ),
        Tenet::Pilgrimage => (
            "sent its faithful on pilgrimage",
            "held pilgrimage needless",
        ),
        Tenet::Monasticism => (
            "honoured those who withdrew from the world",
            "frowned on withdrawal from the world",
        ),
    };
    (if stance > 0.0 { yes } else { no }).to_owned()
}

fn tenet_noun(tenet: umran_sim::doctrine::Tenet) -> &'static str {
    use umran_sim::doctrine::Tenet;
    match tenet {
        Tenet::SacredLanguage => "the sacred tongue",
        Tenet::Images => "images",
        Tenet::Hierarchy => "the priesthood",
        Tenet::Purity => "purity",
        Tenet::Pilgrimage => "pilgrimage",
        Tenet::Monasticism => "withdrawal from the world",
    }
}

fn grammatical_marker(variety: &Variety, marker: &Marker, generation: u32) -> String {
    let form = grammar_form_at(&marker.form, &marker.history, generation);
    let spelled = variety.spell(form);
    match marker.kind {
        MarkerKind::None => "no overt marker".into(),
        MarkerKind::Particle => format!("the separate word *{spelled}*"),
        MarkerKind::Pattern => format!("the internal vowel pattern *{spelled}*"),
        MarkerKind::Bound if form.phones().next().is_none() => "a silent attached marker".into(),
        MarkerKind::Bound => match marker.side {
            Side::Prefix => format!("the prefix *{spelled}-*"),
            Side::Suffix => format!("the ending *-{spelled}*"),
        },
    }
}

/// Who spoke variety `v` in `generation`: its people if one people spoke
/// it then, otherwise its speakers, named by the language as it was then
/// called.
fn speakers(world: &World, shifts: &Shifts, v: usize, generation: u32) -> String {
    let mut speaking = (0..world.communities.len()).filter(|&c| {
        shifts.alive_at(world, c, generation) && shifts.spoken_by(world, c, generation) == v
    });
    match (speaking.next(), speaking.next()) {
        // Named as they were called going into the year's changes.
        (Some(c), None) => format!(
            "the {}",
            world.community_name_at(c, generation.saturating_sub(1))
        ),
        _ => format!(
            "those who spoke {}",
            world.language_title_at(v, generation.saturating_sub(1))
        ),
    }
}

/// Each community's language shifts, oldest first, gathered once so that
/// asking who spoke what in a year does not search the whole history.
struct Shifts(Vec<Vec<(u32, usize, usize)>>, Vec<u32>);

impl Shifts {
    fn of(world: &World) -> Shifts {
        let mut by = vec![Vec::new(); world.communities.len()];
        let mut born = vec![0; world.communities.len()];
        for (position, &(g, ref event)) in world.events.iter().enumerate() {
            if let WorldEvent::Shift {
                community, from, ..
            } = *event
            {
                by[community].push((g, from, position));
            }
        }
        for &(g, ref event) in &world.events {
            let community = match event {
                WorldEvent::Found { community } | WorldEvent::Koine { community, .. } => {
                    Some(*community)
                }
                WorldEvent::Split { daughter, .. } => Some(*daughter),
                WorldEvent::Settlement(record) => record.daughter,
                _ => None,
            };
            if let Some(c) = community {
                born[c] = g;
            }
        }
        Shifts(by, born)
    }

    fn alive_at(&self, world: &World, c: usize, generation: u32) -> bool {
        self.1[c] <= generation && world.communities[c].ended.is_none_or(|g| g >= generation)
    }

    /// The variety community `c` spoke during `generation`'s changes: the
    /// one it later shifted away from, if it shifted then or afterwards, or
    /// the one it speaks now. A year's sound changes come before anything
    /// done that year.
    fn spoken_by(&self, world: &World, c: usize, generation: u32) -> usize {
        self.0[c]
            .iter()
            .find(|(g, _, _)| *g >= generation)
            .map_or(world.communities[c].variety, |&(_, from, _)| from)
    }

    /// World events have an order even within one year. The next shift's
    /// source is the speech used here, after any shift at this position.
    fn spoken_after(&self, world: &World, c: usize, position: usize) -> usize {
        self.0[c]
            .iter()
            .find(|(_, _, p)| *p > position)
            .map_or(world.communities[c].variety, |&(_, from, _)| from)
    }
}

/// A word that changed audibly in `generation`, spelled before and after,
/// with its meaning: preferring words changed by more laws, then the most
/// basic meanings.
fn example(variety: &Variety, generation: u32) -> Option<(String, String, &'static str)> {
    let mut best = None;
    let mut rank = None;
    for word in variety.lexicon.living() {
        let Some((laws, before, after)) = change_in(word, generation) else {
            continue;
        };
        let key = (
            laws,
            Reverse(word.first_sense.stability.unwrap_or(u8::MAX)),
            Reverse(word.id.0),
        );
        // A lower-ranked word cannot replace the best audible example.
        // Equal ranks still take the last item, just like Iterator::max_by_key.
        if rank.is_some_and(|rank| key < rank) {
            continue;
        }
        let (before, after) = (variety.spell(before), variety.spell(after));
        if before != after {
            rank = Some(key);
            best = Some((before, after, word.first_sense.gloss));
        }
    }
    best
}

/// How many sound laws changed `word` in `generation`, and its forms
/// before the first and after the last.
fn change_in(word: &Lexeme, generation: u32) -> Option<(usize, &Form, &Form)> {
    let befores = |from: usize| {
        word.log[from..]
            .iter()
            .enumerate()
            .filter_map(move |(i, e)| match &e.event {
                Event::SoundLaw { before, .. } => Some((from + i, e.generation, before)),
                _ => None,
            })
    };
    let mut then = befores(0).filter(|(_, g, _)| *g == generation);
    let (mut last, _, before) = then.next()?;
    let mut count = 1;
    for (i, _, _) in then {
        last = i;
        count += 1;
    }
    let after = befores(last + 1).next().map_or(&word.form, |(_, _, f)| f);
    Some((count, before, after))
}

#[cfg(test)]
mod cause_tests {
    use super::*;
    use umran_sim::{Naming, Params, SoundProfile};

    #[test]
    fn ranked_examples_match_exhaustive_spelling() {
        fn exhaustive(
            variety: &Variety,
            generation: u32,
        ) -> Option<(String, String, &'static str)> {
            variety
                .lexicon
                .living()
                .filter_map(|word| {
                    let befores: Vec<_> = word
                        .log
                        .iter()
                        .enumerate()
                        .filter_map(|(i, e)| match &e.event {
                            Event::SoundLaw { before, .. } => Some((i, e.generation, before)),
                            _ => None,
                        })
                        .collect();
                    let then: Vec<_> = befores
                        .iter()
                        .filter(|(_, g, _)| *g == generation)
                        .collect();
                    let first = then.first()?;
                    let last = then.last()?;
                    let after = befores
                        .iter()
                        .find(|(i, _, _)| *i > last.0)
                        .map_or(&word.form, |(_, _, form)| *form);
                    let before = variety.spell(first.2);
                    let after = variety.spell(after);
                    (before != after).then_some((then.len(), word, before, after))
                })
                .max_by_key(|(laws, word, ..)| {
                    (
                        *laws,
                        Reverse(word.first_sense.stability.unwrap_or(u8::MAX)),
                        Reverse(word.id.0),
                    )
                })
                .map(|(_, word, before, after)| (before, after, word.first_sense.gloss))
        }

        for preset in ["familiar", "semitic", "bantu"] {
            let mut world = World::solo(
                7,
                &SoundProfile::by_id(preset).unwrap(),
                Params::static_society(),
            );
            for _ in 0..2 {
                world.run(20);
                for variety in &world.varieties {
                    for &(generation, _) in &variety.laws {
                        assert_eq!(
                            example(variety, generation),
                            exhaustive(variety, generation),
                            "{preset}, generation {generation}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn founding_keeps_its_original_language_after_a_shift() {
        for shift_generation in [0, 10] {
            let mut world = World::new(21, Params::static_society());
            let home = world.map.landmasses[0].anchor;
            for seed in 0..2 {
                world.found_seeded(
                    &Naming::People,
                    &SoundProfile::base(),
                    seed,
                    0.5,
                    0.5,
                    Some(home),
                    None,
                    None,
                );
            }
            let original = world.communities[0].variety;
            let before = annals(&world)
                .into_iter()
                .find(|a| a.kind == "found" && a.peoples == [0])
                .unwrap();
            assert!(before.text.contains(&world.language_title_at(original, 0)));
            world.generation = shift_generation;
            let adopted = world.shift(0, 1);
            assert_ne!(original, adopted);
            let after = annals(&world)
                .into_iter()
                .find(|a| a.id == before.id)
                .unwrap();
            assert_eq!(after.text, before.text);
            assert_eq!(after.languages, vec![original]);
        }
    }

    #[test]
    fn recorded_settlement_tells_the_fall_but_keeps_the_preview_conditional() {
        use umran_sim::polity::Rise;
        use umran_sim::settlement::{SettlementChoice, SettlementIntent};

        let mut world = World::solo(5, &SoundProfile::base(), Params::static_society());
        let state = world.raise_state(0, None, Rise::Proclaimed);
        let destination = world
            .settlement_options(0, SettlementIntent::Migration, 1.0)
            .unwrap()
            .into_iter()
            .find(|option| option.reason.is_none())
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
        let preview = world.plan_settlement(&choice).unwrap();
        assert_eq!(preview.falling_states, vec![state]);
        assert_eq!(
            preview.notes.last().unwrap(),
            "The rulers would give up their capital. Their state will fall when this choice is made."
        );
        world.settle(&choice).unwrap();
        assert!(world.states[state].fell.is_some());
        let recorded = annals(&world)
            .into_iter()
            .find(|a| a.kind == "settlement")
            .unwrap();
        assert_eq!(recorded.states, vec![state]);
        assert_eq!(
            recorded.notes.last().unwrap(),
            "The rulers gave up their capital, and their state fell."
        );
        assert!(
            recorded
                .notes
                .iter()
                .all(|note| !note.contains("will fall"))
        );
        let plan = &recorded.settlement.unwrap().plan;
        assert_eq!(plan.notes, preview.notes);
        let mut no_fall = plan.clone();
        no_fall.falling_states.clear();
        assert!(
            no_fall
                .recorded_notes()
                .iter()
                .all(|note| !note.contains("state fell")),
            "the recorded state IDs, not retained preview prose, determine the outcome"
        );
    }

    #[test]
    fn conversion_cause_resolves_inside_grouped_neighbours_and_unknown_is_absent() {
        let mut world = World::new(
            21,
            Params {
                conversion_rate: 100.0,
                ..Params::static_society()
            },
        );
        let home = world.map.landmasses[0].anchor;
        for seed in 0..3 {
            world.found_seeded(
                &Naming::People,
                &SoundProfile::base(),
                seed,
                0.5,
                0.5,
                Some(home),
                None,
                None,
            );
        }
        world.found_religion(0, Revelation::Proclaimed);
        let meeting = world.events.len();
        world.connect(0, 1, 1.0, ContactKind::Neighbours).unwrap();
        world.connect(0, 2, 1.0, ContactKind::Neighbours).unwrap();
        world.step();
        let entries = annals(&world);
        let converted = entries
            .iter()
            .find(|a| a.kind == "conversion" && a.peoples[0] == 1)
            .unwrap();
        let recorded = converted.cause.unwrap();
        assert_eq!(
            recorded,
            umran_sim::Cause {
                event: meeting,
                mechanism: umran_sim::Mechanism::Contact
            }
        );
        let group = entries
            .iter()
            .find(|a| {
                a.members
                    .iter()
                    .any(|m| m.id == world_event_id(recorded.event))
            })
            .unwrap();
        assert!(group.generation <= converted.generation);
        let json = serde_json::to_value(converted).unwrap();
        assert_eq!(json["cause"]["event"], meeting);
        assert_eq!(json["cause"]["mechanism"], "contact");
        let found = entries.iter().find(|a| a.kind == "found").unwrap();
        assert!(serde_json::to_value(found).unwrap().get("cause").is_none());
    }
}

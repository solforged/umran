//! The world's history told as a chronicle. As in a critical edition, each
//! entry has two voices: its text is the annalist's, in the past tense and
//! varied in wording, and its notes are the linguist's apparatus. Wording
//! is drawn from the annals' own stream, so it never moves the world's
//! draws, and the same history is always told the same way.

use crate::substrate_label;
use langgen_sim::rng::{index, key, stream};
use langgen_sim::world::ContactKind;
use langgen_sim::{Event, Form, Lexeme, Variety, World, WorldEvent, catalog};
use serde::Serialize;
use std::cmp::Reverse;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct Annal {
    pub generation: u32,
    /// "found", "split", "shift", "contact", or "law".
    pub kind: &'static str,
    /// The annalist's words. Words of the language are marked `*thus*`.
    pub text: String,
    /// The apparatus: what a linguist would note, such as the sound laws
    /// behind a change.
    pub notes: Vec<String>,
    /// The variety a sound law changed, so a view can show one language's.
    pub variety: Option<usize>,
}

const FOUND: &[&str] = &[
    "The {p} first appeared, calling themselves {p}, “{m}”, and their speech {l}.",
    "Here begins the account of the {p}, whose name means “{m}”, and of their tongue, {l}.",
    "There was a people who named themselves {p}, that is, “{m}”; their speech they called {l}.",
];

const SPLIT: &[&str] = &[
    "Some of the {p} went out from among them and took the name {d}, “{m}”.",
    "A part of the {p} went away and called themselves {d}, “{m}”.",
    "That year the {p} were divided, and those who left were called {d}, “{m}”.",
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
        ContactKind::Neighbours => &[
            "The {a} and the {b} came to live as neighbours.",
            "The {a} settled within reach of the {b}.",
        ],
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

/// Everything that happened in `world`, as chronicle entries in order.
/// Names are written as they were said at the time of each entry.
pub(crate) fn annals(
    world: &World,
    connections: &[(u32, usize, usize, ContactKind)],
) -> Vec<Annal> {
    let meaning = |c: usize| world.communities[c].name.meaning.as_str();
    let mut out: Vec<Annal> = Vec::new();
    let entry = |generation, kind, text| Annal {
        generation,
        kind,
        text,
        notes: Vec::new(),
        variety: None,
    };
    for &(generation, ref event) in &world.events {
        let name = |c: usize| world.community_name_at(c, generation);
        let tongue = |c: usize| world.language_title_at(world.communities[c].variety, generation);
        let g = u64::from(generation);
        out.push(match *event {
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
            ),
            WorldEvent::Split {
                community,
                daughter,
            } => entry(
                generation,
                "split",
                tell(
                    world,
                    &[key("split"), g, daughter as u64],
                    SPLIT,
                    &[
                        ("p", &name(community)),
                        ("d", &name(daughter)),
                        ("m", meaning(daughter)),
                    ],
                ),
            ),
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
            ),
        });
    }
    for &(generation, a, b, contact) in connections {
        if generation > world.generation || a.max(b) >= world.communities.len() {
            continue;
        }
        out.push(entry(
            generation,
            "contact",
            tell(
                world,
                &[key("contact"), u64::from(generation), a as u64, b as u64],
                contact_wording(contact),
                &[
                    ("a", &world.community_name_at(a, generation)),
                    ("b", &world.community_name_at(b, generation)),
                ],
            ),
        ));
    }
    out.extend(sound_changes(world));
    out.sort_by_key(|a| (a.generation, a.kind == "law"));
    out
}

/// One entry per language and generation in which its sounds changed,
/// told through a word that changed, with the laws in the apparatus.
fn sound_changes(world: &World) -> Vec<Annal> {
    let laws = catalog();
    let label = |id: &str| {
        laws.iter()
            .find(|l| l.id == id)
            .map_or_else(|| substrate_label(id), |l| l.label.to_string())
    };
    let mut out = Vec::new();
    for (v, variety) in world.varieties.iter().enumerate() {
        // A daughter's inherited laws are told in its parent's annals.
        let from = variety.parent.map_or(0, |f| f.generation + 1);
        let mut by_generation: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
        for &(generation, id) in variety.laws.iter().filter(|(g, _)| *g >= from) {
            by_generation.entry(generation).or_default().push(id);
        }
        for (generation, ids) in by_generation {
            let people = speakers(world, v, generation);
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
            out.push(Annal {
                generation,
                kind: "law",
                text,
                notes: ids.iter().map(|id| label(id)).collect(),
                variety: Some(v),
            });
        }
    }
    out
}

/// Who spoke variety `v` in `generation`: its people if one people spoke
/// it then, otherwise its speakers, named by the language as it was then
/// called.
fn speakers(world: &World, v: usize, generation: u32) -> String {
    let mut speaking =
        (0..world.communities.len()).filter(|&c| spoken_by(world, c, generation) == v);
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

/// The variety community `c` spoke during `generation`'s changes: the
/// one it later shifted away from, if it shifted then or afterwards, or
/// the one it speaks now.
fn spoken_by(world: &World, c: usize, generation: u32) -> usize {
    world
        .events
        .iter()
        .filter_map(|(g, event)| match *event {
            WorldEvent::Shift {
                community, from, ..
            // A year's sound changes come before anything done that year.
            } if community == c && *g >= generation => Some((*g, from)),
            _ => None,
        })
        .min_by_key(|(g, _)| *g)
        .map_or(world.communities[c].variety, |(_, from)| from)
}

/// A word that changed audibly in `generation`, spelled before and after,
/// with its meaning: preferring words changed by more laws, then the most
/// basic meanings.
fn example(variety: &Variety, generation: u32) -> Option<(String, String, &'static str)> {
    variety
        .lexicon
        .living()
        .filter_map(|word| {
            let (laws, before, after) = change_in(word, generation)?;
            let (before, after) = (variety.spell(before), variety.spell(after));
            (before != after).then_some((laws, word, before, after))
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
    let then: Vec<(usize, &Form)> = befores(0)
        .filter(|(_, g, _)| *g == generation)
        .map(|(i, _, before)| (i, before))
        .collect();
    let &(_, before) = then.first()?;
    let &(last, _) = then.last()?;
    let after = befores(last + 1).next().map_or(&word.form, |(_, _, f)| f);
    Some((then.len(), before, after))
}

//! What peoples call themselves, their speech, and their land. A name is
//! coined from the language's own words when a people forms or first
//! holds a land, then lives on as a word of its own: sound laws reshape
//! it like any other, even after the words it was built from have changed
//! or gone (English from Engle "Angles"). Place names outlast their
//! coiners: newcomers mostly take over the name of the land they come to,
//! fitted to their own sounds, as English kept the Celtic Thames.

use crate::concepts::{Relation, by_id};
use crate::form::Form;
use crate::geography::Terrain;
use crate::lexicon::{Entry, Event};
use crate::livelihood::Livelihood;
use crate::rng::{index, weighted_index};
use crate::variety::Variety;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// What a people's name means, which decides how it is built.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Naming {
    /// "The people": the commonest source of self-names (Inuit, Diné,
    /// and Deutsch, from a word for the people).
    People,
    /// "Those who speak": the ones who talk sense, as against foreigners
    /// (the Slavs, by one account, from slovo "word").
    Speakers,
    /// "People of the hill": from the land they live in.
    Place { place: String },
    /// An epithet on an older name: "the far Goths", "Little Poland". On a
    /// founding, it qualifies "the people".
    Epithet { epithet: String },
    /// "The people of Opuw": from the name of the land they settle, as the
    /// Northumbrians were named for the land north of the Humber. Only a
    /// people moving off from another takes it; founders have no land
    /// name yet.
    Land,
}

/// Landscape a people can be named for.
pub const PLACES: [&str; 5] = ["hill", "mountain", "river", "sea", "island"];
/// Words a people's name can be qualified with.
pub const EPITHETS: [&str; 7] = ["new", "far", "small", "big", "old", "red", "black"];
/// Syllables beyond which a name takes no further epithet, so epithets
/// do not stack over many splits: there were Ostrogoths and Visigoths,
/// never "far East Goths".
pub(crate) const MAX_EPITHET_BASE: usize = 2;
/// Most syllables a people's name keeps, and a language's. Names said
/// every day are short, and long ones are clipped in use: Deutsch,
/// English, Magyar, Suomi, Kiswahili.
pub(crate) const MAX_PEOPLE_NAME: usize = 3;
const MAX_LANGUAGE_NAME: usize = 4;
/// Epithets a group that moves off tends to take.
const DAUGHTER_EPITHETS: [&str; 3] = ["new", "far", "small"];
/// Most syllables a place name keeps.
const MAX_PLACE_NAME: usize = 4;
/// How often a land is named, in turn: for what it is ("the hill"), for
/// what it is like ("the black hill"), as the place of something, or for
/// the people who hold it ("the land of the Angles").
const PLACE_KINDS: [f32; 4] = [0.15, 0.55, 0.1, 0.2];
/// Qualities any land can be named for.
const PLACE_QUALITIES: [&str; 10] = [
    "big", "small", "old", "new", "long", "wide", "black", "red", "dark", "good",
];
/// Things the coast is named for.
const COAST_THINGS: [&str; 3] = ["sea", "fish", "salt"];

/// A name: its current form, what it meant when coined, and the sound
/// laws it has undergone since.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Name {
    pub form: Form,
    /// What it meant when coined, e.g. "the people of the river".
    pub meaning: String,
    pub coined: u32,
    pub log: Vec<Entry>,
}

impl Name {
    /// Applies a sound law; records it if it changed anything.
    pub fn change(
        &mut self,
        law: &crate::Law,
        minimal: crate::MinimalWord,
        stress: crate::prosody::StressRule,
        generation: u32,
    ) {
        let after = law.apply(&self.form, minimal, stress);
        if law.changes(&self.form, &after, stress) {
            let before = std::mem::replace(&mut self.form, after);
            self.log.push(Entry {
                generation,
                event: Event::SoundLaw {
                    law: law.id,
                    before,
                },
            });
        }
    }

    /// The form the name had at `generation`, before any later sound law.
    pub fn form_at(&self, generation: u32) -> &Form {
        self.log
            .iter()
            .find(|e| e.generation > generation)
            .and_then(|e| match &e.event {
                Event::SoundLaw { before, .. } => Some(before),
                _ => None,
            })
            .unwrap_or(&self.form)
    }
}

/// A fixed chart attestation, not a living name subject to sound laws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinentName {
    pub spelled: String,
    pub ipa: String,
    pub meaning: String,
    pub variety: usize,
    pub people: usize,
    /// A region on this continent known to the naming language.
    pub witness: usize,
    pub since: u32,
}

impl Naming {
    /// Why this naming is not one the engine offers, if it is not.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Naming::Place { place } if !PLACES.contains(&place.as_str()) => {
                Err(format!("'{place}' is not a place a people is named for"))
            }
            Naming::Epithet { epithet } if !EPITHETS.contains(&epithet.as_str()) => {
                Err(format!("'{epithet}' is not an epithet for a people"))
            }
            _ => Ok(()),
        }
    }

    /// Every choice offered when founding a people.
    pub fn choices() -> Vec<Naming> {
        let mut out = vec![Naming::People, Naming::Speakers];
        out.extend(PLACES.iter().map(|p| Naming::Place { place: (*p).into() }));
        out.extend(EPITHETS.iter().map(|e| Naming::Epithet {
            epithet: (*e).into(),
        }));
        out
    }

    /// What a split-off people may call itself, given its parent's name:
    /// epithets common for emigrants, and places not already in the name.
    /// Names already long take no further epithet, so "the far new small
    /// X" does not pile up over many splits.
    pub fn for_daughter(parent: &Name) -> Vec<(Naming, f32)> {
        let short = parent.form.vowel_count() <= MAX_EPITHET_BASE;
        let epithets = DAUGHTER_EPITHETS.iter().filter(|_| short).map(|e| {
            let naming = Naming::Epithet {
                epithet: (*e).into(),
            };
            (naming, 0.55 / DAUGHTER_EPITHETS.len() as f32)
        });
        let places = PLACES
            .iter()
            .filter(|p| !parent.meaning.contains(*p))
            .map(|p| {
                (
                    Naming::Place { place: (*p).into() },
                    0.45 / PLACES.len() as f32,
                )
            });
        epithets.chain(places).collect()
    }

    /// The people's name in `variety`'s words, clipped as names are in
    /// everyday use. `base` is the older name an epithet qualifies, or the
    /// land a people is named for, spelled for the gloss.
    pub fn coin(
        &self,
        variety: &Variety,
        base: Option<(&Name, &str)>,
        generation: u32,
    ) -> Result<Name, String> {
        let mut name = self.coin_whole(variety, base, generation)?;
        name.form = clipped(name.form, MAX_PEOPLE_NAME);
        Ok(name)
    }

    /// The name in full, unclipped, for when the clipped name would be
    /// another people's: "the new Tinea" where "Tinea" is taken.
    pub fn coin_whole(
        &self,
        variety: &Variety,
        base: Option<(&Name, &str)>,
        generation: u32,
    ) -> Result<Name, String> {
        let word = |id: &str| -> Result<Form, String> {
            let concept = by_id(id).ok_or_else(|| format!("no concept '{id}'"))?;
            variety
                .lexicon
                .word_for(concept)
                .map(|l| l.form.clone())
                .ok_or_else(|| format!("the language has no word for {id}"))
        };
        let morphology = &variety.morphology;
        let (form, meaning) = match self {
            Naming::People => (word("people")?, "the people".to_string()),
            Naming::Speakers => {
                let say = word("say")?;
                let form = morphology
                    .derive(&say, None, Relation::Agent)
                    .unwrap_or_else(|| {
                        morphology.compound(&say, &word("people").unwrap_or_default())
                    });
                (form, "those who speak".into())
            }
            Naming::Place { place } => {
                if !PLACES.contains(&place.as_str()) {
                    return Err(format!("'{place}' is not a place a people is named for"));
                }
                let head = word("people")?;
                (
                    morphology.compound(&word(place)?, &head),
                    format!("the people of the {place}"),
                )
            }
            Naming::Epithet { epithet } => {
                if !EPITHETS.contains(&epithet.as_str()) {
                    return Err(format!("'{epithet}' is not an epithet for a people"));
                }
                let (head, of) = match base {
                    Some((name, spelled)) => (name.form.clone(), spelled.to_string()),
                    None => (word("people")?, "people".into()),
                };
                (
                    morphology.compound(&word(epithet)?, &head),
                    format!("the {epithet} {of}"),
                )
            }
            Naming::Land => {
                let (land, spelled) =
                    base.ok_or("a people is named for its land only once it has one")?;
                // "Of" the land, as Northumbrians are of Northumbria: the
                // land comes first whatever the word order, so clipping
                // keeps it.
                (
                    morphology.belonging(&land.form),
                    format!("the people of {spelled}"),
                )
            }
        };
        Ok(Name {
            form,
            meaning,
            coined: generation,
            log: Vec::new(),
        })
    }
}

/// `form` cut after its `max`th vowel if it has more, as long names are
/// clipped in use. Ending on a vowel keeps the clipped name pronounceable
/// in any language.
pub(crate) fn clipped(form: Form, max: usize) -> Form {
    let Some(&end) = form.syllables().get(max).map(|s| &s.onset.start) else {
        return form;
    };
    let nucleus = form.syllables()[max - 1].nucleus;
    let end = end.min(nucleus + 1);
    Form {
        boundaries: form
            .boundaries
            .iter()
            .copied()
            .filter(|&b| b < end)
            .collect(),
        segs: form.segs[..end].to_vec(),
        stress: form.stress.map(|s| s.min(max - 1)),
    }
}

/// A language's name from its speakers' name: with the belonging affix,
/// or compounded with a word for speech unless that would make it too
/// long to say every day.
pub fn language_name(variety: &Variety, people: &Name, spelled: &str, generation: u32) -> Name {
    language_names(variety, people, spelled, generation).swap_remove(0)
}

/// Every way a language can be named from its speakers' name, the usual
/// one first: the other way of forming it (a language that names by the
/// belonging affix can still say "the X tongue"), then each said in full
/// rather than clipped, for when the usual name is another language's.
pub fn language_names(
    variety: &Variety,
    people: &Name,
    spelled: &str,
    generation: u32,
) -> Vec<Name> {
    let morphology = &variety.morphology;
    let compound = |id: &str| {
        let concept = by_id(id)?;
        let word = variety.lexicon.word_for(concept)?.form.clone();
        Some((
            morphology.compound(&people.form, &word),
            format!("the {spelled} {}", concept.gloss),
        ))
    };
    let belonging = (
        morphology.belonging(&people.form),
        format!("of the {spelled}"),
    );
    let ways: Vec<(Form, String)> = match morphology.names.speech.and_then(compound) {
        Some(c) if c.0.vowel_count() <= MAX_LANGUAGE_NAME => vec![c, belonging],
        Some(c) => vec![belonging, c],
        None => [Some(belonging), compound("tongue")]
            .into_iter()
            .flatten()
            .collect(),
    };
    let name = |form: Form, meaning: &String| Name {
        form,
        meaning: meaning.clone(),
        coined: generation,
        log: Vec::new(),
    };
    let short = ways
        .iter()
        .map(|(form, meaning)| name(clipped(form.clone(), MAX_LANGUAGE_NAME), meaning));
    let whole = ways
        .iter()
        .map(|(form, meaning)| name(form.clone(), meaning));
    short.chain(whole).collect()
}

/// What a land is like, which decides what it can be named for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Landscape {
    pub terrain: Terrain,
    pub coastal: bool,
    pub island: bool,
}

impl Landscape {
    /// What the land itself is called: the head of its name.
    fn heads(self) -> &'static [&'static str] {
        if self.island {
            return &["island"];
        }
        match self.terrain {
            Terrain::Plains | Terrain::Steppe => &["field", "soil"],
            Terrain::Forest => &["wood", "tree"],
            Terrain::Hills => &["hill"],
            Terrain::Mountains => &["mountain", "stone"],
            Terrain::Desert => &["sand"],
            Terrain::Sea => &["sea"],
        }
    }

    /// Things found there that it can be named for.
    fn things(self) -> Vec<&'static str> {
        let inland: &[&str] = match self.terrain {
            Terrain::Plains => &["grain", "cattle", "horse", "river", "water", "stone"],
            Terrain::Forest => &["bird", "shadow", "tree", "river"],
            Terrain::Steppe => &["horse", "wind", "cattle", "salt", "sun"],
            Terrain::Hills => &["stone", "cattle", "wind", "river"],
            Terrain::Mountains => &["stone", "iron", "gold", "god", "sky", "wind"],
            Terrain::Desert => &["salt", "sun", "stone"],
            Terrain::Sea => &[],
        };
        let coast: &[&str] = if self.coastal { &COAST_THINGS } else { &[] };
        inland.iter().chain(coast).copied().collect()
    }
}

/// What `variety`'s speakers call a land like `land` on first holding it.
/// `settlers` is their own name, spelled for the gloss, for lands named
/// after their people. `None` if the language has no word for any kind
/// of land.
pub fn place_name(
    variety: &Variety,
    land: Landscape,
    settlers: (&Name, &str),
    rng: &mut impl Rng,
    generation: u32,
) -> Option<Name> {
    let word = |id: &'static str| Some((id, variety.lexicon.word_for(by_id(id)?)?.form.clone()));
    let mut heads: Vec<(&str, Form)> = land.heads().iter().filter_map(|&id| word(id)).collect();
    if heads.is_empty() {
        heads.extend(word("soil"));
    }
    if heads.is_empty() {
        return None;
    }
    let (head_id, head) = heads.swap_remove(index(rng, heads.len()));
    let things: Vec<(&str, Form)> = land
        .things()
        .into_iter()
        .filter(|&id| id != head_id)
        .filter_map(word)
        .collect();
    let qualities: Vec<(&str, Form)> = PLACE_QUALITIES.iter().filter_map(|&id| word(id)).collect();
    let morphology = &variety.morphology;
    let (form, meaning) = match weighted_index(rng, PLACE_KINDS.iter().copied()) {
        1 if !things.is_empty() || !qualities.is_empty() => {
            let all: Vec<&(&str, Form)> = things.iter().chain(&qualities).collect();
            let (id, modifier) = all[index(rng, all.len())];
            (
                morphology.compound(modifier, &head),
                format!("the {id} {head_id}"),
            )
        }
        2 if !things.is_empty() => {
            let (id, thing) = &things[index(rng, things.len())];
            match morphology.derive(thing, None, Relation::Place) {
                Some(form) => (form, format!("the place of {id}")),
                None => (
                    morphology.compound(thing, &head),
                    format!("the {id} {head_id}"),
                ),
            }
        }
        // A land named for its people only if the name stays whole: clipped,
        // "the hill of the Hifis" would be just "Hifis" again.
        3 if morphology.compound(&settlers.0.form, &head).vowel_count() <= MAX_PLACE_NAME => (
            morphology.compound(&settlers.0.form, &head),
            format!("the {head_id} of the {}", settlers.1),
        ),
        _ => (head, format!("the {head_id}")),
    };
    Some(Name {
        form: clipped(form, MAX_PLACE_NAME),
        meaning,
        coined: generation,
        log: Vec::new(),
    })
}

/// A continent heading in a witness language's own words. Local headings
/// may name a place or people; foreign headings may call the land far or new.
pub fn continent_name(
    variety: &Variety,
    place: &Name,
    people: &Name,
    foreign: bool,
    rng: &mut impl Rng,
    generation: u32,
) -> Option<Name> {
    let word = |id| variety.lexicon.word_for(by_id(id)?).map(|l| &l.form);
    let head = word("land").or_else(|| word("soil"))?;
    let qualities: &[&str] = if foreign {
        &["wide", "big", "far", "new"]
    } else {
        &["wide", "big"]
    };
    let mut modifiers: Vec<(&Form, String)> = qualities
        .iter()
        .filter_map(|&id| word(id).map(|form| (form, format!("the {id} land"))))
        .collect();
    modifiers.push((
        &place.form,
        format!("the land of {}", variety.title(&place.form)),
    ));
    if !foreign {
        modifiers.push((
            &people.form,
            format!("the land of the {}", variety.title(&people.form)),
        ));
    }
    let (modifier, meaning) = modifiers.swap_remove(index(rng, modifiers.len()));
    Some(Name {
        form: clipped(variety.morphology.compound(modifier, head), MAX_PLACE_NAME),
        meaning,
        coined: generation,
        log: Vec::new(),
    })
}

/// How a land came by one of its names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceOrigin {
    /// Given by `community` when its people came to hold the land.
    Coined { community: usize },
    /// Came down with the language from an ancestor that held the land.
    Inherited,
    /// Kept by the people holding the land when their speech changed.
    Kept,
    /// Learned from the land's earlier name, fitted to the newcomers'
    /// sounds.
    Borrowed,
}

/// One language's name for a land, from when its speakers came to hold
/// it. Each name after a land's first comes from the one before it, unless
/// it was coined afresh.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceName {
    pub variety: usize,
    pub since: u32,
    pub name: Name,
    pub origin: PlaceOrigin,
}

/// `text` with its first letter capitalized, for names.
pub fn title(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// How a language builds its people's given names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NameStyle {
    /// One meaningful word: "Bright", "Wolf".
    Single,
    /// Two words joined, as Germanic Wulfstan ("wolf-stone"), Slavic
    /// Vladimir ("rule-peace"), and Greek Philippos ("lover of horses")
    /// are built.
    Double,
}

/// A given name in a language's stock of names, and the variety it was
/// taken from, if it came with a faith.
#[derive(Clone, Debug, PartialEq)]
pub struct GivenName {
    pub name: Name,
    pub from: Option<usize>,
}

/// Meanings given names are made of, weighted for foragers, herders, and
/// farmers. Herders name children for horses and cattle, as the Greeks'
/// Philippos and the Iranians' Vishtaspa ("having horses") show, and
/// warriors for spears and battle, as Germanic Gunther ("battle-army")
/// does; farmers name them for grain and fields. "God" makes a name
/// theophoric, as Theodore and Elijah are.
const NAME_ELEMENTS: &[(&str, [f32; 3])] = &[
    ("good", [1.0, 1.0, 1.0]),
    ("big", [1.0, 1.0, 1.0]),
    ("light", [1.0, 1.0, 1.0]),
    ("sun", [1.0, 1.0, 1.0]),
    ("moon", [0.7, 0.7, 0.7]),
    ("star", [0.7, 0.7, 0.7]),
    ("fire", [1.0, 1.0, 1.0]),
    ("stone", [1.0, 1.0, 1.0]),
    ("gold", [0.3, 0.8, 1.0]),
    ("red", [0.7, 0.7, 0.7]),
    ("hard", [0.7, 0.7, 0.7]),
    ("heart", [0.7, 0.7, 0.7]),
    ("life", [0.5, 0.5, 0.5]),
    ("sky", [0.5, 0.8, 0.5]),
    ("sea", [0.5, 0.2, 0.5]),
    ("river", [0.7, 0.3, 0.7]),
    ("mountain", [0.5, 0.5, 0.3]),
    ("tree", [1.0, 0.3, 0.6]),
    ("bird", [1.5, 0.6, 0.6]),
    ("dog", [0.8, 1.0, 0.4]),
    ("fish", [1.0, 0.2, 0.3]),
    ("bow", [1.0, 1.2, 0.5]),
    ("horse", [0.1, 3.0, 0.5]),
    ("cattle", [0.1, 2.5, 0.7]),
    ("grain", [0.1, 0.2, 2.0]),
    ("field", [0.1, 0.2, 1.5]),
    ("seed", [0.1, 0.1, 1.0]),
    ("spear", [1.0, 1.5, 0.8]),
    ("shield", [0.3, 1.0, 0.8]),
    ("war", [0.3, 1.5, 1.0]),
    ("fight", [0.5, 1.2, 0.7]),
    ("friend", [1.0, 1.0, 1.0]),
    ("people", [0.7, 0.7, 0.7]),
    ("chief", [0.3, 1.0, 1.0]),
    ("god", [1.0, 1.0, 1.0]),
];
/// Given names a language has in fashion at once.
pub(crate) const GIVEN_STOCK: usize = 8;
/// Most syllables a given name keeps.
const MAX_GIVEN: usize = 3;
/// How much likelier a people of a founded faith names children for its
/// god.
const DEVOUT: f32 = 4.0;

/// A given name in `variety`'s words, built its way, from meanings a
/// people living by `livelihood` favours, and for its god the more if it
/// is `devout`. `None` if the language has no word for any of them.
pub fn given_name(
    variety: &Variety,
    livelihood: Livelihood,
    devout: bool,
    rng: &mut impl Rng,
    generation: u32,
) -> Option<Name> {
    let way = match livelihood {
        Livelihood::Foraging => 0,
        Livelihood::Herding => 1,
        Livelihood::Farming => 2,
    };
    let elements: Vec<(&str, &Form, f32)> = NAME_ELEMENTS
        .iter()
        .filter_map(|(id, weights)| {
            let word = variety.lexicon.word_for(by_id(id)?)?;
            let devotion = if devout && *id == "god" { DEVOUT } else { 1.0 };
            Some((*id, &word.form, weights[way] * devotion))
        })
        .collect();
    if elements.is_empty() {
        return None;
    }
    let a = weighted_index(rng, elements.iter().map(|e| e.2));
    let b = match variety.style {
        NameStyle::Single => None,
        NameStyle::Double => {
            let b = weighted_index(rng, elements.iter().map(|e| e.2));
            (b != a).then_some(b)
        }
    };
    let (form, meaning) = match b {
        Some(b) => (
            variety.morphology.compound(elements[a].1, elements[b].1),
            format!("{}-{}", elements[a].0, elements[b].0),
        ),
        None => (elements[a].1.clone(), elements[a].0.to_string()),
    };
    Some(Name {
        form: clipped(form, MAX_GIVEN),
        meaning,
        coined: generation,
        log: Vec::new(),
    })
}

/// A founding stock of given names for `variety`, drawn from `rng`.
pub fn given_stock(
    variety: &Variety,
    livelihood: Livelihood,
    rng: &mut impl Rng,
) -> Vec<GivenName> {
    let mut stock: Vec<GivenName> = Vec::new();
    for _ in 0..GIVEN_STOCK * 3 {
        if stock.len() == GIVEN_STOCK {
            break;
        }
        let Some(name) = given_name(variety, livelihood, false, rng, 0) else {
            break;
        };
        if !stock.iter().any(|g| g.name.form == name.form) {
            stock.push(GivenName { name, from: None });
        }
    }
    stock
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::SoundProfile;

    fn variety(preset: &str, seed: u64) -> Variety {
        Variety::found(
            seed,
            &SoundProfile::by_id(preset).unwrap(),
            Livelihood::Farming,
        )
    }

    #[test]
    fn names_are_built_from_the_languages_own_words() {
        let v = variety("familiar", 3);
        let people = Naming::People.coin(&v, None, 0).unwrap();
        let word = |id| v.lexicon.word_for(by_id(id).unwrap()).unwrap().form.clone();
        assert_eq!(people.form, word("people"));
        let river = Naming::Place {
            place: "river".into(),
        }
        .coin(&v, None, 0)
        .unwrap();
        // The compound starts or ends with the word for people.
        let (whole, head) = (&river.form.segs, &word("people").segs);
        assert!(
            whole.len() > head.len()
                && (whole.starts_with(&head[..1]) || whole.ends_with(&head[head.len() - 1..]))
        );
        assert_eq!(river.meaning, "the people of the river");
    }

    #[test]
    fn epithets_qualify_an_older_name() {
        let v = variety("germanic", 5);
        let old = Naming::People.coin(&v, None, 0).unwrap();
        let far = Naming::Epithet {
            epithet: "far".into(),
        }
        .coin(&v, Some((&old, "Theudo")), 4)
        .unwrap();
        assert_eq!(far.meaning, "the far Theudo");
        assert!(far.form.segs.len() > old.form.segs.len());
        assert!(
            Naming::Epithet {
                epithet: "purple".into()
            }
            .coin(&v, None, 0)
            .is_err()
        );
    }

    #[test]
    fn languages_are_named_for_their_speakers() {
        let mut affixed = 0;
        for seed in 0..40 {
            let v = variety("familiar", seed);
            let people = Naming::People.coin(&v, None, 0).unwrap();
            let language = language_name(&v, &people, "Kawa", 0);
            assert!(language.form.segs.len() > people.form.segs.len());
            if v.morphology.names.speech.is_none() {
                affixed += 1;
                assert_eq!(language.meaning, "of the Kawa");
            }
        }
        assert!((12..=36).contains(&affixed), "{affixed} of 40 affixed");
    }

    #[test]
    fn root_and_pattern_languages_put_heads_first() {
        let first = (0..40)
            .filter(|s| variety("semitic", *s).morphology.names.head_first)
            .count();
        assert!(first >= 30, "{first} of 40");
    }

    #[test]
    fn names_remember_their_older_forms() {
        let mut name = Name {
            form: Form::from_ipa("pataka").unwrap(),
            ..Name::default()
        };
        let laws = crate::catalog();
        let voicing = laws
            .iter()
            .find(|l| l.id == "intervocalic-voicing")
            .unwrap();
        let apocope = laws.iter().find(|l| l.id == "apocope").unwrap();
        let first = name.form.clone();
        name.change(
            voicing,
            crate::MinimalWord::Syllable,
            crate::StressRule::Initial,
            4,
        );
        let second = name.form.clone();
        assert_eq!(second.ipa(), "padaga");
        name.change(
            apocope,
            crate::MinimalWord::Syllable,
            crate::StressRule::Initial,
            9,
        );
        let third = name.form.clone();
        assert_eq!(third.ipa(), "padag");
        assert_eq!(name.form_at(0), &first);
        assert_eq!(name.form_at(3), &first);
        assert_eq!(name.form_at(4), &second);
        assert_eq!(name.form_at(8), &second);
        assert_eq!(name.form_at(9), &third);
    }

    #[test]
    fn long_names_are_clipped_after_a_vowel() {
        let form = |ipa| Form::from_ipa(ipa).unwrap();
        assert_eq!(clipped(form("kawatenulo"), 3).ipa(), "kawate");
        assert_eq!(clipped(form("kastanpurla"), 2).ipa(), "kasta");
        assert_eq!(clipped(form("kawa"), 3).ipa(), "kawa");
        let mut joined = form("kawatenu");
        joined.boundaries = vec![2, 6];
        assert_eq!(clipped(joined, 3).boundaries, vec![2]);
    }

    #[test]
    fn titles_capitalize_unicode() {
        assert_eq!(title("ɛwe"), "Ɛwe");
        assert_eq!(title("āna"), "Āna");
        assert_eq!(title(""), "");
    }

    #[test]
    fn cultural_continent_headings_use_own_compounds_and_foreign_qualities() {
        for seed in 0..40 {
            let speech = variety("familiar", seed);
            let people = Naming::People.coin(&speech, None, 0).unwrap();
            let place = Naming::Place {
                place: "river".into(),
            }
            .coin(&speech, None, 0)
            .unwrap();
            let head = &speech
                .lexicon
                .word_for(by_id("soil").unwrap())
                .unwrap()
                .form;
            for foreign in [false, true] {
                let mut rng = crate::rng::stream(seed, &[crate::rng::key("continent name")]);
                let name = continent_name(&speech, &place, &people, foreign, &mut rng, 0).unwrap();
                let mut modifiers: Vec<&Form> = ["wide", "big", "far", "new"]
                    .into_iter()
                    .filter(|id| foreign || !["far", "new"].contains(id))
                    .filter_map(|id| speech.lexicon.word_for(by_id(id).unwrap()).map(|l| &l.form))
                    .collect();
                modifiers.push(&place.form);
                if !foreign {
                    modifiers.push(&people.form);
                }
                assert!(modifiers.into_iter().any(|modifier| clipped(
                    speech.morphology.compound(modifier, head),
                    MAX_PLACE_NAME
                ) == name.form));
                assert!(name.form.vowel_count() <= MAX_PLACE_NAME);
                if !foreign {
                    assert!(!["the far land", "the new land"].contains(&name.meaning.as_str()));
                }
            }
        }
    }
}

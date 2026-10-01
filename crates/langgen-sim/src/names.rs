//! What peoples call themselves and their speech. A name is coined from
//! the language's own words when a people forms, then lives on as a word
//! of its own: sound laws reshape it like any other, even after the words
//! it was built from have changed or gone (English from Engle "Angles").

use crate::concepts::{Relation, by_id};
use crate::form::Form;
use crate::lexicon::{Entry, Event};
use crate::variety::Variety;
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
}

/// Landscape a people can be named for.
pub const PLACES: [&str; 5] = ["hill", "mountain", "river", "sea", "island"];
/// Words a people's name can be qualified with.
pub const EPITHETS: [&str; 7] = ["new", "far", "small", "big", "old", "red", "black"];
/// Syllables beyond which a name takes no further epithet.
const MAX_EPITHET_BASE: usize = 3;
/// Epithets a group that moves off tends to take.
const DAUGHTER_EPITHETS: [&str; 3] = ["new", "far", "small"];

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
    pub fn change(&mut self, after: Form, law: &'static str, generation: u32) {
        if after != self.form {
            let before = std::mem::replace(&mut self.form, after);
            self.log.push(Entry {
                generation,
                event: Event::SoundLaw { law, before },
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

    /// The people's name in `variety`'s words. `base` is the older name an
    /// epithet qualifies, spelled for the gloss.
    pub fn coin(
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
        };
        Ok(Name {
            form,
            meaning,
            coined: generation,
            log: Vec::new(),
        })
    }
}

/// A language's name from its speakers' name: with the belonging affix,
/// or compounded with a word for speech.
pub fn language_name(variety: &Variety, people: &Name, spelled: &str, generation: u32) -> Name {
    let morphology = &variety.morphology;
    let speech = morphology.names.speech.and_then(|id| {
        let concept = by_id(id)?;
        Some((
            variety.lexicon.word_for(concept)?.form.clone(),
            concept.gloss,
        ))
    });
    let (form, meaning) = match speech {
        Some((word, gloss)) => (
            morphology.compound(&people.form, &word),
            format!("the {spelled} {gloss}"),
        ),
        None => (
            morphology.belonging(&people.form),
            format!("of the {spelled}"),
        ),
    };
    Name {
        form,
        meaning,
        coined: generation,
        log: Vec::new(),
    }
}

/// `text` with its first letter capitalized, for names.
pub fn title(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::SoundProfile;

    fn variety(preset: &str, seed: u64) -> Variety {
        Variety::found(seed, &SoundProfile::by_id(preset).unwrap())
    }

    #[test]
    fn names_are_built_from_the_languages_own_words() {
        let v = variety("typical", 3);
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
            let v = variety("typical", seed);
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
        let v = variety("typical", 3);
        let mut name = Naming::People.coin(&v, None, 0).unwrap();
        let first = name.form.clone();
        let mut second = first.clone();
        second.segs.pop();
        name.change(second.clone(), "a law", 4);
        let mut third = second.clone();
        third.segs.reverse();
        name.change(third.clone(), "another law", 9);
        assert_eq!(name.form_at(0), &first);
        assert_eq!(name.form_at(3), &first);
        assert_eq!(name.form_at(4), &second);
        assert_eq!(name.form_at(8), &second);
        assert_eq!(name.form_at(9), &third);
    }

    #[test]
    fn titles_capitalize_unicode() {
        assert_eq!(title("ɛwe"), "Ɛwe");
        assert_eq!(title("āna"), "Āna");
        assert_eq!(title(""), "");
    }
}

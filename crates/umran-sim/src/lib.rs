//! Contact-aware language change simulator: communities, their sound
//! preferences, and their contacts drive sound change, borrowing, word
//! competition, splits, and language shift. See AGENTS.md for the model
//! and the design doc it links for plans and results.

#![deny(clippy::disallowed_methods)]

pub mod adapt;
pub mod causes;
pub mod change;
pub mod chronicle;
pub mod cities;
pub mod climate;
pub mod compare;
pub mod concepts;
mod continental;
pub mod design;
pub mod diglossia;
pub mod ethos;
pub mod flavor;
pub mod form;
pub mod gender;
pub mod geography;
pub mod grammar;
pub mod harmony;
pub mod ideas;
pub mod inflection;
pub mod inventory;
pub mod laws;
pub mod lexicon;
pub mod livelihood;
mod math;
pub mod morphology;
pub mod names;
pub mod palettes;
pub mod phoneme;
pub mod phonotactics;
pub mod polity;
pub mod profile;
pub mod pronouns;
pub mod prosody;
pub mod provenance;
pub mod river_names;
mod rivers;
pub mod rng;
pub mod root;
pub mod schisms;
pub mod settlement;
mod spelling;
mod sphere;
pub mod typology;
pub mod variety;
pub mod wold;
pub mod world;

pub use causes::{Cause, Mechanism};
pub use change::{Env, Matcher, Rewrite, SoundChange, apply_all};
pub use chronicle::{
    Action, Chronicle, ENGINE_REVISION, FORMAT, HistoryPoint, ReadingRef, Recipe, Telling,
    TellingId,
};
pub use climate::{Climate, ClimateCause, ClimateChange, RegionClimate, ZoneClimate};
pub use concepts::{CONCEPTS, Class, Concept, FAMILIES, Field, Iconic, Relation, Tier};
pub use design::{LanguageDesign, Sound};
pub use diglossia::{Classical, Fixing, Vernacular};
pub use ethos::{Axis, Effect, Ethos, FoundingEthos, Pole, TemperCause};
pub use flavor::Flavor;
pub use form::{Form, Seg, Syllable};
pub use geography::{
    ClimateZone, GeographyVersion, Landmass, LandmassKind, Map, MapSize, Region, River, Terrain,
};
pub use grammar::{Category, Grammar, GrammarChoice, GrammarDesign, GrammarPrior};
pub use ideas::{Craft, Need, Religion, Revelation, SacredKind, SacredPlace};
pub use inventory::Inventory;
pub use laws::{Law, catalog};
pub use lexicon::{Entry, Event, Lexeme, LexemeId, Lexicon, Origin, Slot};
pub use livelihood::Livelihood;
pub use names::{ContinentName, GivenName, Name, NameStyle, Naming, PlaceName, PlaceOrigin};
pub use phoneme::{CATALOG, PhonemeId, Segment};
pub use phonotactics::Phonotactics;
pub use polity::{Challenge, Fall, Member, Rise, State};
pub use profile::{LongVowel, MorphologyKind, MorphologyPrior, SoundProfile, Spelling};
pub use prosody::{MinimalWord, StressRule};
pub use provenance::LoanCause;
pub use variety::{Fork, Variety};
pub use world::{Community, Contact, ContactKind, Journey, Params, World, WorldEvent};

//! Contact-aware language change simulator: communities, their sound
//! preferences, and their contacts drive sound change, borrowing, word
//! competition, splits, and language shift. See AGENTS.md for the model
//! and the design doc it links for plans and results.

pub mod adapt;
pub mod change;
pub mod chronicle;
pub mod compare;
pub mod concepts;
pub mod design;
pub mod flavor;
pub mod form;
pub mod geography;
pub mod inventory;
pub mod laws;
pub mod lexicon;
pub mod livelihood;
pub mod morphology;
pub mod names;
pub mod palettes;
pub mod phoneme;
pub mod phonotactics;
pub mod profile;
pub mod prosody;
pub mod rng;
pub mod root;
mod spelling;
pub mod typology;
pub mod variety;
pub mod wold;
pub mod world;

pub use change::{Env, Matcher, Rewrite, SoundChange, apply_all};
pub use chronicle::{Action, Chronicle, ENGINE_REVISION, FORMAT, Recipe, SetAside, Telling};
pub use concepts::{CONCEPTS, Class, Concept, FAMILIES, Field, Iconic, Relation, Tier};
pub use design::{LanguageDesign, Sound};
pub use flavor::Flavor;
pub use form::{Form, Seg, Syllable};
pub use geography::{Map, MapSize, Region, Terrain};
pub use inventory::Inventory;
pub use laws::{Law, catalog};
pub use lexicon::{Entry, Event, Lexeme, LexemeId, Lexicon, Origin, Slot};
pub use livelihood::Livelihood;
pub use names::{Name, Naming};
pub use phoneme::{CATALOG, PhonemeId, Segment};
pub use phonotactics::Phonotactics;
pub use profile::{LongVowel, MorphologyKind, MorphologyPrior, SoundProfile, Spelling};
pub use prosody::MinimalWord;
pub use variety::{Fork, Variety};
pub use world::{Community, Contact, ContactKind, Params, World, WorldEvent};

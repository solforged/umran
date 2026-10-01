//! Contact-aware language change simulator.
//!
//! Replaces `langgen-core` milestone by milestone; see the design doc linked
//! from AGENTS.md. Milestone 1 covers sounds, spelling, concepts, and
//! founding roots.

pub mod change;
pub mod concepts;
pub mod flavor;
pub mod form;
pub mod inventory;
pub mod laws;
pub mod lexicon;
pub mod phoneme;
pub mod phonotactics;
pub mod profile;
pub mod rng;
pub mod root;
pub mod sim;
mod spelling;
pub mod variety;

pub use change::{Env, Matcher, Rewrite, SoundChange, apply_all};
pub use concepts::{CONCEPTS, Class, Concept, Field, Iconic};
pub use flavor::Flavor;
pub use form::{Form, Seg, Syllable};
pub use inventory::Inventory;
pub use laws::{Law, catalog};
pub use lexicon::{Entry, Event, Lexeme, LexemeId, Lexicon, Origin, Slot};
pub use phoneme::{CATALOG, PhonemeId, Segment};
pub use phonotactics::Phonotactics;
pub use profile::{LongVowel, SoundProfile, Spelling};
pub use sim::{Params, Sim};
pub use variety::Variety;

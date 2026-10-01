//! Contact-aware language change simulator.
//!
//! Replaces `langgen-core` milestone by milestone; see the design doc linked
//! from AGENTS.md. Milestone 1 covers sounds, spelling, concepts, and
//! founding roots.

pub mod change;
pub mod concepts;
pub mod form;
pub mod inventory;
pub mod phoneme;
pub mod phonotactics;
pub mod preset;
pub mod rng;
pub mod root;
mod spelling;
pub mod variety;

pub use change::{Env, Matcher, Rewrite, SoundChange, apply_all};
pub use concepts::{CONCEPTS, Class, Concept, Field, Iconic};
pub use form::{Form, Seg, Syllable};
pub use inventory::Inventory;
pub use phoneme::{CATALOG, PhonemeId, Segment};
pub use phonotactics::Phonotactics;
pub use preset::{LongVowel, Preset, Spelling};
pub use variety::{Variety, Word};

//! Reader annotations are document metadata, independent of simulation replay.
use serde::{Deserialize, Serialize};
use umran_sim::{Craft, ReadingRef, Recipe};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum Subject {
    World,
    History,
    People { id: usize },
    State { id: usize },
    Religion { id: usize },
    Craft { id: Craft },
    Language { variety: usize },
    Word { variety: usize, concept: String },
    Law { id: String },
    Land { region: usize },
    Continent { landmass: usize },
    River { id: usize },
    Zone { id: usize },
    Event { id: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Destination {
    pub reading: ReadingRef,
    pub subject: Subject,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum NoteKind {
    Observation,
    Question,
    Year,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub kind: NoteKind,
    pub target: Option<Destination>,
    /// A caption retained even when the original reference cannot be read.
    pub label: String,
    pub generation: u32,
    pub revision: u32,
    pub archived: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Document {
    #[serde(flatten)]
    pub recipe: Recipe,
    #[serde(default)]
    pub notebook: Vec<Note>,
}

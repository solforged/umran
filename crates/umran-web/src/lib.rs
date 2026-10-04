//! Browser facade over `umran-sim`: the history lives here, and the
//! browser asks for presentation-ready views of any generation as JSON.

mod annals;
mod notebook;
mod provenance;

use annals::{Annal, annals};
use notebook::{Document, Note, Subject};
use provenance::{LoanCauseView, cause_view, origin_cause};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::{cell::RefCell, rc::Rc};
use umran_sim::climate::ClimateCause;
use umran_sim::compare::intelligibility;
use umran_sim::concepts::{Concept, by_id, related};
use umran_sim::geography::{
    GeographyVersion, KM_PER_UNIT, LandmassKind, Map, RIVER_TRAVEL_FLOW, River, angular_distance,
    geographic,
};
use umran_sim::grammar::{
    Category, GrammarChoice, GrammarEntry, GrammarEvent, Marker as GrammarMarker, MarkerKind,
    MarkerOrigin, PossessorOrder, Side, WordOrder,
};
use umran_sim::ideas::{NEEDS, Need, SacredKind};
use umran_sim::morphology::Slot;
use umran_sim::names::{Name, PlaceName, PlaceOrigin};
use umran_sim::phoneme::{Backness, Manner, Secondary};
use umran_sim::schisms::{BranchNaming, HolyLand, Pilgrimage, SchismCause};
use umran_sim::{
    Action, CATALOG, CONCEPTS, Challenge, Chronicle, Craft, ENGINE_REVISION, Event, Fall, Flavor,
    Form, HistoryPoint, Lexeme, LexemeId, Livelihood, MapSize, NameStyle, Origin, PhonemeId,
    ReadingRef, Revelation, Rise, StressRule, TellingId, Terrain, World, WorldEvent, catalog,
};
use umran_sim::{LanguageDesign, MorphologyKind, Naming, Segment, Variety};
use wasm_bindgen::prelude::*;

/// The browser-facing history; `Bench` holds the logic so it can be tested
/// natively, where `JsValue` is unavailable.
#[wasm_bindgen]
pub struct Workbench {
    bench: Bench,
}

#[wasm_bindgen]
impl Workbench {
    /// A new history whose map `size` ("small", "medium", "large", or "vast") is
    /// drawn from `seed`.
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, size: &str) -> Result<Workbench, JsValue> {
        Ok(Workbench {
            bench: Bench::new(seed, size).map_err(fail)?,
        })
    }

    #[wasm_bindgen(js_name = withGeography)]
    pub fn with_geography(seed: u32, size: &str, geography: &str) -> Result<Workbench, JsValue> {
        Ok(Workbench {
            bench: Bench::with_geography(seed, size, geography).map_err(fail)?,
        })
    }

    pub fn load(json: &str) -> Result<Workbench, JsValue> {
        Ok(Workbench {
            bench: Bench::load(json).map_err(fail)?,
        })
    }

    pub fn save(&self) -> Result<String, JsValue> {
        self.bench.save().map_err(fail)
    }

    pub fn decisions(&self) -> Result<String, JsValue> {
        self.bench.decisions().map_err(fail)
    }

    pub fn title(&self) -> Option<String> {
        self.bench.title().map(str::to_owned)
    }

    pub fn author(&self) -> Option<String> {
        self.bench.author().map(str::to_owned)
    }

    #[wasm_bindgen(js_name = setTitle)]
    pub fn set_title(&mut self, title: &str) {
        self.bench.set_title(title);
    }

    #[wasm_bindgen(js_name = setAuthor)]
    pub fn set_author(&mut self, author: &str) {
        self.bench.set_author(author);
    }

    pub fn notebook(&self) -> Result<String, JsValue> {
        to_json(&self.bench.notebook).map_err(fail)
    }

    #[wasm_bindgen(js_name = saveNote)]
    pub fn save_note(&mut self, note: &str) -> Result<(), JsValue> {
        self.bench.save_note(note).map_err(fail)
    }

    #[wasm_bindgen(js_name = resolveNote)]
    pub fn resolve_note(&self, id: &str) -> Result<String, JsValue> {
        self.bench.resolve_note(id).map_err(fail)
    }

    #[wasm_bindgen(js_name = removeNote)]
    pub fn remove_note(&mut self, id: &str) {
        self.bench.remove_note(id);
    }

    pub fn catalog() -> Result<String, JsValue> {
        Bench::catalog().map_err(fail)
    }

    /// A preset resolved into an editable design.
    pub fn design(preset: &str, seed: u32) -> Result<String, JsValue> {
        Bench::design(preset, seed).map_err(fail)
    }

    #[wasm_bindgen(js_name = frequencyDesign)]
    pub fn frequency_design(seed: u32, consonants: u8, vowels: u8) -> Result<String, JsValue> {
        Bench::frequency_design(seed, consonants, vowels).map_err(fail)
    }

    /// Sample words from a design, and the names its people would take,
    /// without founding anything.
    pub fn preview(design: &str, seed: u32, naming: &str) -> Result<String, JsValue> {
        Bench::preview(design, seed, naming).map_err(fail)
    }

    #[wasm_bindgen(js_name = foundingPreview)]
    pub fn founding_preview(&self) -> Result<String, JsValue> {
        self.bench.founding_preview().map_err(fail)
    }

    #[wasm_bindgen(js_name = foundingSites)]
    pub fn founding_sites(&self, region: f64, count: f64) -> Result<String, JsValue> {
        if !region.is_finite()
            || region < 0.0
            || region.fract() != 0.0
            || region >= self.bench.chronicle.latest().map.regions.len() as f64
        {
            return Err(fail("Choose a valid homeland region.".into()));
        }
        if !count.is_finite() || count.fract() != 0.0 || !(1.0..=12.0).contains(&count) {
            return Err(fail("Choose between 1 and 12 founding sites.".into()));
        }
        self.bench
            .founding_sites(region as usize, count as usize)
            .map_err(fail)
    }

    pub fn act(&mut self, action: &str) -> Result<(), JsValue> {
        self.bench.act(action).map_err(fail)
    }

    #[wasm_bindgen(js_name = actAt)]
    pub fn act_at(&mut self, point: &str, mutation: u32, action: &str) -> Result<(), JsValue> {
        self.bench.act_at(point, mutation, action).map_err(fail)
    }

    pub fn previous(&self, reading: &str) -> Result<String, JsValue> {
        self.bench.previous(reading).map_err(fail)
    }

    pub fn read(&self, telling: u32, point: &str) -> Result<ReadView, JsValue> {
        Ok(ReadView {
            bench: self.bench.scope(telling, point).map_err(fail)?,
        })
    }

    pub fn rename(&mut self, telling: u32, name: &str) -> Result<(), JsValue> {
        self.bench.rename(telling, name).map_err(fail)
    }

    pub fn compare(&self, left: u32, right: u32, generation: u32) -> Result<String, JsValue> {
        self.bench.compare(left, right, generation).map_err(fail)
    }

    #[wasm_bindgen(js_name = untilAt)]
    pub fn until_at(&mut self, reading: &str, mutation: u32, limit: u32) -> Result<u32, JsValue> {
        self.bench.until_at(reading, mutation, limit).map_err(fail)
    }

    #[wasm_bindgen(js_name = runUntilEvent)]
    pub fn run_until_event(&mut self, limit: u32) -> u32 {
        self.bench.run_until_event(limit)
    }

    pub fn branch(&mut self, generation: u32) {
        self.bench.branch(generation)
    }

    /// Takes up a telling set aside, setting the present one aside.
    pub fn restore(&mut self, index: TellingId) -> Result<(), JsValue> {
        self.bench.restore(index).map_err(fail)
    }

    pub fn latest(&self) -> u32 {
        self.bench.latest()
    }

    pub fn overview(&mut self, generation: u32) -> Result<String, JsValue> {
        self.bench.overview(generation).map_err(fail)
    }

    #[wasm_bindgen(js_name = overviewAt)]
    pub fn overview_at(&mut self, point: &str) -> Result<String, JsValue> {
        self.bench.overview_at(point).map_err(fail)
    }

    pub fn settlement(
        &self,
        point: &str,
        community: f64,
        intent: &str,
        share: f32,
        destination: i32,
    ) -> Result<String, JsValue> {
        self.bench
            .settlement(
                point,
                entity_id(community).map_err(fail)?,
                intent,
                share,
                destination,
            )
            .map_err(fail)
    }

    #[wasm_bindgen(js_name = lawChoices)]
    pub fn law_choices(&self, point: &str, variety: f64) -> Result<String, JsValue> {
        self.bench
            .law_choices(point, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }

    pub fn lexicon(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .lexicon(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }

    pub fn kin(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .kin(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }

    pub fn word(
        &mut self,
        generation: u32,
        variety: f64,
        concept: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .word(generation, entity_id(variety).map_err(fail)?, concept)
            .map_err(fail)
    }

    pub fn story(&mut self, generation: u32, subject: &str) -> Result<String, JsValue> {
        self.bench.story(generation, subject).map_err(fail)
    }

    /// The land: regions with their spherical boundaries and terrain.
    pub fn map(&self) -> Result<String, JsValue> {
        self.bench.map().map_err(fail)
    }

    /// The climate and feeding capacities at `generation`.
    pub fn climate(&mut self, generation: u32) -> Result<String, JsValue> {
        self.bench.climate(generation).map_err(fail)
    }

    /// A river's names and local forms at `generation`.
    pub fn river(&mut self, generation: u32, id: f64) -> Result<String, JsValue> {
        self.bench
            .river(generation, entity_id(id).map_err(fail)?)
            .map_err(fail)
    }

    /// Every living people's word for `concept`, grouped by common root.
    #[wasm_bindgen(js_name = wordMap)]
    pub fn word_map(&mut self, generation: u32, concept: &str) -> Result<String, JsValue> {
        self.bench.word_map(generation, concept).map_err(fail)
    }
}

/// A scoped read capability. Every view resolves IDs within this telling and,
/// when supplied, at the same exact position within a year.
#[wasm_bindgen]
pub struct ReadView {
    bench: Rc<RefCell<Bench>>,
}
#[wasm_bindgen]
impl ReadView {
    pub fn latest(&self) -> u32 {
        self.bench.borrow().latest()
    }
    pub fn decisions(&self) -> Result<String, JsValue> {
        self.bench.borrow().decisions().map_err(fail)
    }
    pub fn overview(&mut self, generation: u32) -> Result<String, JsValue> {
        self.bench.borrow_mut().overview(generation).map_err(fail)
    }
    #[wasm_bindgen(js_name = overviewAt)]
    pub fn overview_at(&mut self, point: &str) -> Result<String, JsValue> {
        self.bench.borrow_mut().overview_at(point).map_err(fail)
    }
    pub fn settlement(
        &self,
        point: &str,
        community: f64,
        intent: &str,
        share: f32,
        destination: i32,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow()
            .settlement(
                point,
                entity_id(community).map_err(fail)?,
                intent,
                share,
                destination,
            )
            .map_err(fail)
    }
    #[wasm_bindgen(js_name = lawChoices)]
    pub fn law_choices(&self, point: &str, variety: f64) -> Result<String, JsValue> {
        self.bench
            .borrow()
            .law_choices(point, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
    pub fn lexicon(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .lexicon(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
    pub fn kin(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .kin(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
    pub fn word(
        &mut self,
        generation: u32,
        variety: f64,
        concept: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .word(generation, entity_id(variety).map_err(fail)?, concept)
            .map_err(fail)
    }
    pub fn story(&mut self, generation: u32, subject: &str) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .story(generation, subject)
            .map_err(fail)
    }
    pub fn climate(&mut self, generation: u32) -> Result<String, JsValue> {
        self.bench.borrow_mut().climate(generation).map_err(fail)
    }
    pub fn river(&mut self, generation: u32, id: f64) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .river(generation, entity_id(id).map_err(fail)?)
            .map_err(fail)
    }
    pub fn map(&self) -> Result<String, JsValue> {
        self.bench.borrow().map().map_err(fail)
    }
    #[wasm_bindgen(js_name = wordMap)]
    pub fn word_map(&mut self, generation: u32, concept: &str) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .word_map(generation, concept)
            .map_err(fail)
    }
}

struct CachedReading {
    telling: TellingId,
    point: String,
    mutation: u32,
    bench: Rc<RefCell<Bench>>,
}

/// One authored constraint, with links to its original (ungrouped) annals.
#[derive(Serialize)]
pub struct DecisionView {
    pub index: usize,
    pub generation: u32,
    pub kind: &'static str,
    pub text: String,
    pub people: Vec<usize>,
    /// The resulting language for founding, settlement, or shift.
    pub variety: Option<usize>,
    pub annals: Vec<String>,
}

pub struct Bench {
    chronicle: Chronicle,
    notebook: Vec<Note>,
    title: Option<String>,
    author: Option<String>,
    /// Invalidates previews even when two decisions happen in the same year.
    mutation: u32,
    /// The last world requested, so repeated views of one generation do not
    /// replay it again.
    cached: Option<World>,
    /// Engine revision a loaded recipe was saved with, if it differs.
    saved_revision: Option<u32>,
    /// An exact, read-only view. All language and word queries share it.
    fixed: Option<(HistoryPoint, World)>,
    readings: RefCell<Vec<CachedReading>>,
}

impl Bench {
    pub fn new(seed: u32, size: &str) -> Result<Bench, String> {
        Self::new_with_geography(seed, size, GeographyVersion::CURRENT)
    }

    /// Generate the recipe's geography without replaying its authored history.
    pub fn with_geography(seed: u32, size: &str, geography: &str) -> Result<Bench, String> {
        let geography: GeographyVersion =
            serde_json::from_value(serde_json::Value::String(geography.into()))
                .map_err(|_| format!("Unknown geography version: {geography}."))?;
        Self::new_with_geography(seed, size, geography)
    }

    fn new_with_geography(
        seed: u32,
        size: &str,
        geography: GeographyVersion,
    ) -> Result<Bench, String> {
        let map: MapSize = serde_json::from_value(serde_json::Value::String(size.into()))
            .map_err(|_| format!("Unknown world size: {size}."))?;
        Ok(Bench {
            chronicle: Chronicle::with_geography(u64::from(seed), map, geography),
            notebook: Vec::new(),
            title: None,
            author: None,
            mutation: 0,
            cached: None,
            saved_revision: None,
            fixed: None,
            readings: RefCell::default(),
        })
    }

    /// Restores a saved recipe.
    pub fn load(json: &str) -> Result<Bench, String> {
        let document: Document =
            serde_json::from_str(json).map_err(|e| format!("Not an Umran save: {e}"))?;
        let recipe = document.recipe;
        let chronicle = Chronicle::from_recipe(&recipe)?;
        Ok(Bench {
            chronicle,
            notebook: document.notebook,
            title: document.title,
            author: document.author,
            mutation: 0,
            cached: None,
            saved_revision: (recipe.revision != ENGINE_REVISION).then_some(recipe.revision),
            fixed: None,
            readings: RefCell::default(),
        })
    }

    pub fn save(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&Document {
            recipe: self.chronicle.recipe(),
            notebook: self.notebook.clone(),
            title: self.title.clone(),
            author: self.author.clone(),
        })
        .map_err(|e| e.to_string())
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = (!title.is_empty()).then(|| title.to_owned());
        self.readings.get_mut().clear();
    }

    pub fn set_author(&mut self, author: &str) {
        self.author = (!author.is_empty()).then(|| author.to_owned());
        self.readings.get_mut().clear();
    }

    /// Authored decisions through this reading, including empty event ranges.
    pub fn decisions(&self) -> Result<String, String> {
        let world = self
            .fixed
            .as_ref()
            .map_or_else(|| self.chronicle.latest(), |(_, world)| world);
        let mut generation = 0;
        let mut actions = self.chronicle.actions().iter().enumerate();
        let mut views = Vec::with_capacity(world.decisions.len());
        for decision in &world.decisions {
            let (_, action) = actions
                .find(|(index, action)| {
                    if let Action::Run { generations } = action {
                        generation += generations;
                    }
                    *index == decision.action
                })
                .expect("a recorded decision has an action");
            let name = |c| world.community_name_at(c, generation);
            let spoken = |c: usize| {
                world.events[decision.events.end..]
                    .iter()
                    .find_map(|(_, event)| match event {
                        WorldEvent::Shift {
                            community, from, ..
                        } if *community == c => Some(*from),
                        _ => None,
                    })
                    .unwrap_or(world.communities[c].variety)
            };
            let (kind, text, people, variety) = match action {
                Action::Found { .. } | Action::FoundRelated { .. } => {
                    let c = decision
                        .events
                        .clone()
                        .find_map(|position| match world.events[position].1 {
                            WorldEvent::Found { community } => Some(community),
                            _ => None,
                        })
                        .expect("founding records its people");
                    match action {
                        Action::FoundRelated { source, .. } => (
                            "found-related",
                            format!(
                                "The *{}* were founded with speech inherited from the *{}*.",
                                name(c),
                                name(*source)
                            ),
                            vec![*source, c],
                            Some(spoken(c)),
                        ),
                        _ => (
                            "found",
                            format!("The *{}* were founded.", name(c)),
                            vec![c],
                            Some(spoken(c)),
                        ),
                    }
                }
                Action::Settle { choice } => {
                    use umran_sim::settlement::SettlementIntent;
                    let destination = choice.destination + 1;
                    let text = match choice.intent {
                        SettlementIntent::Partition => format!(
                            "The *{}* divided their lands, assigning {:.0}% of their people to land {destination}.",
                            name(choice.community),
                            choice.share * 100.0
                        ),
                        SettlementIntent::Settlers => format!(
                            "{:.0}% of the *{}* were sent to settle land {destination}.",
                            choice.share * 100.0,
                            name(choice.community)
                        ),
                        SettlementIntent::Migration => format!(
                            "The *{}* were moved together to land {destination}.",
                            name(choice.community)
                        ),
                    };
                    let daughter = decision.events.clone().find_map(|position| {
                        match &world.events[position].1 {
                            WorldEvent::Settlement(record) => record.daughter,
                            _ => None,
                        }
                    });
                    let mut people = vec![choice.community];
                    people.extend(daughter);
                    (
                        "settle",
                        text,
                        people,
                        Some(spoken(daughter.unwrap_or(choice.community))),
                    )
                }
                Action::Connect { a, b, contact, .. } => {
                    use umran_sim::world::ContactKind;
                    let relation = match contact {
                        ContactKind::Neighbours => "neighbourhood",
                        ContactKind::Trade => "trade",
                        ContactKind::Rule => "rule",
                        ContactKind::Religion => "religious contact",
                        ContactKind::Intermarriage => "intermarriage",
                    };
                    (
                        "connect",
                        format!(
                            "The *{}* and the *{}* were joined by {relation}.",
                            name(*a),
                            name(*b)
                        ),
                        vec![*a, *b],
                        None,
                    )
                }
                Action::Shift { community, toward } => (
                    "shift",
                    format!(
                        "The *{}* took up the tongue of the *{}*.",
                        name(*community),
                        name(*toward)
                    ),
                    vec![*community, *toward],
                    Some(spoken(*community)),
                ),
                Action::State { community, .. } => (
                    "state",
                    format!("The *{}* were made a state.", name(*community)),
                    vec![*community],
                    None,
                ),
                Action::Religion { community } => (
                    "religion",
                    format!("A faith was founded among the *{}*.", name(*community)),
                    vec![*community],
                    None,
                ),
                Action::Craft { community, craft } => (
                    "craft",
                    format!("The *{}* were taught {}.", name(*community), craft.label()),
                    vec![*community],
                    None,
                ),
                Action::Law { variety, law } => {
                    let label = &world
                        .law_catalog()
                        .iter()
                        .find(|l| l.id == law)
                        .expect("authored laws are catalog laws")
                        .label;
                    let mut chars = label.chars();
                    let clause: String = match chars.next() {
                        Some(first) => first.to_lowercase().chain(chars).collect(),
                        None => String::new(),
                    };
                    (
                    "law",
                    format!(
                        "In *{}*, {clause}.",
                        world.varieties[*variety].title(world.varieties[*variety].name.form_at(generation)),
                    ),
                    world.events[..decision.events.start]
                        .iter()
                        .filter_map(|(_, event)| match event {
                            WorldEvent::Found { community } => Some(*community),
                            WorldEvent::Split { daughter, .. } => Some(*daughter),
                            _ => None,
                        })
                        .filter(|&c| {
                            spoken(c) == *variety
                                && !world.events[..decision.events.start].iter().any(
                                    |(_, event)| matches!(event, WorldEvent::Ended { community, .. } if *community == c),
                                )
                        })
                        .collect(),
                    Some(*variety),
                    )
                }
                Action::Temper {
                    community,
                    axis,
                    amount,
                } => (
                    "temper",
                    format!(
                        "The {} leaning of the *{}* was turned by {amount}.",
                        axis.id(),
                        name(*community)
                    ),
                    vec![*community],
                    None,
                ),
                Action::Run { .. } => unreachable!("runs are not decisions"),
            };
            views.push(DecisionView {
                index: decision.action,
                generation,
                kind,
                text,
                people,
                variety,
                annals: if let Action::Law { variety, .. } = action {
                    vec![format!("sounds:{variety}:{generation}")]
                } else {
                    decision
                        .events
                        .clone()
                        .map(annals::world_event_id)
                        .collect()
                },
            });
        }
        to_json(&views)
    }

    pub fn save_note(&mut self, json: &str) -> Result<(), String> {
        let mut note: Note =
            serde_json::from_str(json).map_err(|e| format!("Unreadable note: {e}"))?;
        note.title = note.title.trim().into();
        if note.id.is_empty() || note.title.is_empty() {
            return Err("Give this notebook entry a title.".into());
        }
        // Editing an imported note must remain possible even if its original
        // reading cannot replay. Only a newly selected reference is validated.
        let previous = self.notebook.iter().position(|n| n.id == note.id);
        if previous.is_none_or(|i| {
            self.notebook[i].target != note.target || self.notebook[i].revision != note.revision
        }) {
            self.validate_note(&note)?;
        }
        if let Some(i) = previous {
            self.notebook[i] = note;
        } else {
            self.notebook.push(note);
        }
        Ok(())
    }

    fn validate_note(&self, note: &Note) -> Result<(), String> {
        let Some(target) = &note.target else {
            return Ok(());
        };
        if note.revision != ENGINE_REVISION {
            return Err("The original reference needs checking: this world now uses a different engine revision. The note is still kept.".into());
        }
        let scope = self.scope(target.reading.telling, &to_json(&target.reading.point)?)?;
        let scope = scope.borrow();
        let world = &scope
            .fixed
            .as_ref()
            .ok_or("This note has no exact reading")?
            .1;
        let present = match &target.subject {
            Subject::World | Subject::History => true,
            Subject::People { id } => world.communities.get(*id).is_some(),
            Subject::State { id } => world.states.get(*id).is_some(),
            Subject::Religion { id } => world.religions.get(*id).is_some(),
            Subject::Craft { .. } => true,
            Subject::Language { variety } => world.varieties.get(*variety).is_some(),
            Subject::Word { variety, concept } => {
                world.varieties.get(*variety).is_some() && by_id(concept).is_some()
            }
            Subject::Land { region } => world.map.regions.get(*region).is_some(),
            Subject::Continent { landmass } => world.map.landmasses.get(*landmass).is_some(),
            Subject::River { id } => world.map.rivers.get(*id).is_some(),
            Subject::Zone { id } => world.map.climate_zones.get(*id).is_some(),
            Subject::Law { id } => world
                .varieties
                .iter()
                .any(|v| v.laws.iter().any(|(_, law)| law == id)),
            Subject::Event { id } => annals(world)
                .iter()
                .any(|a| a.id == *id || a.members.iter().any(|m| m.id == *id)),
        };
        if present {
            Ok(())
        } else {
            Err(
                "The original subject is unavailable at this reading. The note is still kept."
                    .into(),
            )
        }
    }

    pub fn resolve_note(&self, id: &str) -> Result<String, String> {
        let note = self
            .notebook
            .iter()
            .find(|n| n.id == id)
            .ok_or("That note is not in this notebook")?;
        self.validate_note(note)?;
        to_json(&note.target)
    }

    pub fn remove_note(&mut self, id: &str) {
        self.notebook.retain(|note| note.id != id);
    }

    /// Map sizes, sound profiles, flavors, and contact kinds to offer in forms.
    pub fn catalog() -> Result<String, String> {
        to_json(&CatalogView {
            revision: ENGINE_REVISION,
            map_sizes: [
                ("small", "small", "642 regions; radius 800 km, circumference about 5,030 km. A compact spherical world."),
                ("medium", "middling", "2,562 regions; radius 1,600 km, circumference about 10,050 km. The default spherical world."),
                ("large", "wide", "2,562 regions; radius 3,200 km, circumference about 20,110 km. A larger globe at coarser regional resolution."),
                ("vast", "vast", "10,242 regions; radius 6,371 km, circumference about 40,030 km. An Earth-sized globe at coarser regional resolution."),
            ]
            .into_iter()
            .map(|(id, name, description)| Choice {
                id: id.into(),
                name: name.into(),
                description: description.into(),
            })
            .collect(),
            sounds: CATALOG
                .segments
                .iter()
                .enumerate()
                .map(|(i, seg)| sound_view(PhonemeId(i as u16), *seg))
                .collect(),
            places: PLACES.to_vec(),
            manners: MANNERS.to_vec(),
            heights: HEIGHTS.to_vec(),
            presets: Flavor::examples()
                .into_iter()
                .map(|f| Choice {
                    id: f.id,
                    name: f.name,
                    description: f.brief,
                })
                .collect(),
            grammar_choices: [
                GrammarChoice::Suffix,
                GrammarChoice::Prefix,
                GrammarChoice::Particle,
                GrammarChoice::None,
            ]
            .into_iter()
            .map(grammar_choice)
            .collect(),
            contacts: [
                (
                    "neighbours",
                    "Neighbours",
                    "Plain proximity; every field equally exposed.",
                ),
                ("trade", "Trade", "Goods, tools, food, and seafaring."),
                (
                    "rule",
                    "Rule",
                    "Government, law, and war; can lead to language shift.",
                ),
                (
                    "religion",
                    "Religion",
                    "Belief, ritual, and the vocabulary of thought.",
                ),
                (
                    "intermarriage",
                    "Intermarriage",
                    "Households, kin, and food; can lead to shift.",
                ),
            ]
            .into_iter()
            .map(|(id, name, description)| Choice {
                id: id.into(),
                name: name.into(),
                description: description.into(),
            })
            .collect(),
            name_places: umran_sim::names::PLACES.to_vec(),
            name_epithets: umran_sim::names::EPITHETS.to_vec(),
            crafts: Craft::ALL
                .iter()
                .map(|&craft| Choice {
                    id: craft.label().into(),
                    name: craft_name(craft).into(),
                    description: craft_description(craft).into(),
                })
                .collect(),
            meanings: CONCEPTS.len(),
        })
    }

    pub fn design(preset: &str, seed: u32) -> Result<String, String> {
        let design = LanguageDesign::preset(preset, u64::from(seed))
            .ok_or_else(|| format!("unknown preset '{preset}'"))?;
        to_json(&design)
    }

    pub fn frequency_design(seed: u32, consonants: u8, vowels: u8) -> Result<String, String> {
        to_json(&LanguageDesign::by_frequency(
            u64::from(seed),
            consonants,
            vowels,
        ))
    }

    /// Founds a throwaway language from `design` and returns a sample of its
    /// words, its word families, and a few numbers.
    pub fn preview(design: &str, seed: u32, naming: &str) -> Result<String, String> {
        let design: LanguageDesign =
            serde_json::from_str(design).map_err(|e| format!("Malformed design: {e}"))?;
        design.validate()?;
        let naming: Naming =
            serde_json::from_str(naming).map_err(|e| format!("Malformed naming: {e}"))?;
        naming.validate()?;
        let variety = Variety::found(
            u64::from(seed),
            &design.profile(),
            Livelihood::Farming,
            Default::default(),
        );
        let people = naming.coin(&variety, None, 0)?;
        let spelled = variety.title(&people.form);
        let language = umran_sim::names::language_name(&variety, &people, &spelled, 0);
        let lexicon = &variety.lexicon;
        let row = |concept: &'static umran_sim::Concept| {
            let word = lexicon.word_for(concept)?;
            let from = match word.origin {
                Origin::Derived { base, relation } => Some(format!(
                    "{} ({})",
                    lexicon.get(base).first_sense.gloss,
                    relation.label()
                )),
                _ => None,
            };
            Some(PreviewWord {
                gloss: concept.gloss,
                spelled: variety.spell(&word.form),
                ipa: word.form.ipa_stressed(variety.stress()),
                from,
            })
        };
        let words = PREVIEW
            .iter()
            .filter_map(|id| umran_sim::concepts::by_id(id))
            .filter_map(row)
            .collect();
        let families = umran_sim::FAMILIES
            .iter()
            .filter_map(|(_, word, _)| umran_sim::concepts::by_id(word))
            .filter_map(row)
            .filter(|w| w.from.is_some())
            .collect();
        let forms: Vec<&Form> = lexicon
            .slots
            .iter()
            .filter_map(|s| Some(&lexicon.word_for(s.concept)?.form))
            .collect();
        let homophones = forms.len() - forms.iter().collect::<std::collections::HashSet<_>>().len();
        let syllables = forms.iter().map(|f| f.syllables().len()).sum::<usize>() as f32
            / forms.len().max(1) as f32;
        to_json(&Preview {
            people: NameView {
                name: spelled,
                ipa: people.form.ipa_stressed(variety.stress()),
                meaning: people.meaning,
            },
            language: NameView {
                name: variety.title(&language.form),
                ipa: language.form.ipa_stressed(variety.stress()),
                meaning: language.meaning,
            },
            words,
            families,
            homophones,
            syllables,
        })
    }

    /// Applies an action (JSON, as `umran_sim::Action`) at the latest
    /// generation. Rejected actions change nothing.
    pub fn act(&mut self, action: &str) -> Result<(), String> {
        let action: Action =
            serde_json::from_str(action).map_err(|e| format!("Malformed action: {e}"))?;
        self.chronicle.act(action)?;
        self.mutation = self.mutation.wrapping_add(1);
        self.cached = None;
        Ok(())
    }

    pub fn act_at(&mut self, point: &str, mutation: u32, action: &str) -> Result<(), String> {
        if mutation != self.mutation {
            return Err(
                "The history has changed since this preview. Read it again before deciding.".into(),
            );
        }
        let reading: ReadingRef = serde_json::from_str(point).map_err(|e| e.to_string())?;
        let action: Action =
            serde_json::from_str(action).map_err(|e| format!("Malformed action: {e}"))?;
        if reading.telling == self.chronicle.active() {
            self.chronicle.act_at(reading.point, action)?;
        } else {
            let mut candidate = self.chronicle.reading(reading.telling)?;
            candidate.act_at(reading.point, action)?;
            self.chronicle = candidate;
        }
        self.mutation = self.mutation.wrapping_add(1);
        self.cached = None;
        Ok(())
    }

    pub fn previous(&self, reading: &str) -> Result<String, String> {
        let mut reading: ReadingRef = serde_json::from_str(reading).map_err(|e| e.to_string())?;
        let history = self.chronicle.reading(reading.telling)?;
        reading.point = history.previous(reading.point)?;
        to_json(&reading)
    }

    fn scope(&self, telling: TellingId, point: &str) -> Result<Rc<RefCell<Bench>>, String> {
        {
            let mut cache = self.readings.borrow_mut();
            cache.retain(|r| r.mutation == self.mutation);
            if let Some(index) = cache
                .iter()
                .position(|r| r.telling == telling && r.point == point)
            {
                let reading = cache.remove(index);
                let handle = Rc::clone(&reading.bench);
                cache.push(reading);
                return Ok(handle);
            }
        }
        let bench = Rc::new(RefCell::new(self.read(telling, point)?));
        let mut cache = self.readings.borrow_mut();
        if cache.len() >= 4 {
            cache.remove(0);
        }
        cache.push(CachedReading {
            telling,
            point: point.into(),
            mutation: self.mutation,
            bench: Rc::clone(&bench),
        });
        Ok(bench)
    }

    pub fn read(&self, telling: TellingId, point: &str) -> Result<Bench, String> {
        let chronicle = self.chronicle.reading(telling)?;
        let fixed = if point.is_empty() {
            None
        } else {
            let point: HistoryPoint = serde_json::from_str(point).map_err(|e| e.to_string())?;
            Some((point, chronicle.world_at_point(point)?))
        };
        Ok(Bench {
            chronicle,
            notebook: self.notebook.clone(),
            title: self.title.clone(),
            author: self.author.clone(),
            mutation: self.mutation,
            cached: None,
            saved_revision: self.saved_revision,
            fixed,
            readings: RefCell::default(),
        })
    }

    pub fn rename(&mut self, telling: TellingId, name: &str) -> Result<(), String> {
        self.chronicle.rename(telling, name)?;
        self.mutation = self.mutation.wrapping_add(1);
        Ok(())
    }

    pub fn until_at(&mut self, reading: &str, mutation: u32, limit: u32) -> Result<u32, String> {
        if limit == 0 {
            return Ok(0);
        }
        self.act_at(reading, mutation, r#"{"kind":"run","generations":1}"#)?;
        // Stop if that first generation already produced an event.
        let generation = self.latest();
        if self
            .chronicle
            .latest()
            .events
            .iter()
            .any(|(g, _)| *g == generation)
        {
            return Ok(1);
        }
        Ok(1 + self.run_until_event(limit - 1))
    }

    pub fn compare(
        &self,
        left: TellingId,
        right: TellingId,
        generation: u32,
    ) -> Result<String, String> {
        let a = self.scope(left, "")?;
        let b = self.scope(right, "")?;
        if generation > a.borrow().latest().min(b.borrow().latest()) {
            return Err("Choose a year already recorded in both tellings.".into());
        }
        let common = self.chronicle.common_reading(left, right)?;
        let shared = self.scope(
            common.telling,
            &serde_json::to_string(&common.point).map_err(|e| e.to_string())?,
        )?;
        let (diverged, people_count, language_count) = {
            let mut shared = shared.borrow_mut();
            let world = shared.world(generation);
            (
                world.generation,
                world.communities.len(),
                world.varieties.len(),
            )
        };
        let a: serde_json::Value = serde_json::from_str(&a.borrow_mut().overview(generation)?)
            .map_err(|e| e.to_string())?;
        let b: serde_json::Value = serde_json::from_str(&b.borrow_mut().overview(generation)?)
            .map_err(|e| e.to_string())?;
        let people_count = if generation < diverged {
            a["communities"].as_array().unwrap().len()
        } else {
            people_count
        };
        let language_count = if generation < diverged {
            a["varieties"].as_array().unwrap().len()
        } else {
            language_count
        };
        to_json(
            &serde_json::json!({ "generation": generation, "common": common, "diverged": diverged,
            "sharedPeoples": (0..people_count).collect::<Vec<_>>(),
            "sharedLanguages": (0..language_count).collect::<Vec<_>>(),
            "left": a, "right": b }),
        )
    }

    pub fn run_until_event(&mut self, limit: u32) -> u32 {
        self.mutation = self.mutation.wrapping_add(1);
        self.cached = None;
        self.chronicle.run_until_event(limit)
    }

    /// Sets everything after `generation` aside as another telling.
    pub fn branch(&mut self, generation: u32) {
        self.mutation = self.mutation.wrapping_add(1);
        self.cached = None;
        self.chronicle.branch_at(generation);
    }

    pub fn restore(&mut self, index: TellingId) -> Result<(), String> {
        self.chronicle.restore(index)?;
        self.mutation = self.mutation.wrapping_add(1);
        self.cached = None;
        Ok(())
    }

    /// Metadata never depends on matching annal prose or successful replay.
    fn tellings(&self) -> Vec<TellingView> {
        self.chronicle
            .tellings()
            .iter()
            .map(|t| {
                let latest = t
                    .actions
                    .iter()
                    .filter_map(|a| {
                        if let Action::Run { generations } = a {
                            Some(*generations)
                        } else {
                            None
                        }
                    })
                    .sum();
                let tip = match t.actions.last() {
                    Some(Action::Run { generations }) => HistoryPoint {
                        action: t.actions.len() - 1,
                        offset: *generations,
                    },
                    _ => HistoryPoint {
                        action: t.actions.len(),
                        offset: 0,
                    },
                };
                let from = t.parent.map(|p| {
                    self.chronicle
                        .telling(p.telling)
                        .expect("validated parent")
                        .actions[..p.point.action]
                        .iter()
                        .filter_map(|a| {
                            if let Action::Run { generations } = a {
                                Some(*generations)
                            } else {
                                None
                            }
                        })
                        .sum::<u32>()
                        + p.point.offset
                });
                TellingView {
                    id: t.id,
                    name: t.name.clone(),
                    parent: t.parent,
                    from,
                    latest,
                    tip,
                    actions: t.actions.len(),
                }
            })
            .collect()
    }

    pub fn latest(&self) -> u32 {
        self.chronicle.latest().generation
    }

    /// Possible first encounters, using the same nearness and merchant reach
    /// as contact formation. This is a query on the authored year-zero world.
    pub fn founding_preview(&self) -> Result<String, String> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Pair {
            a: usize,
            b: usize,
            walk: Option<f32>,
            voyage: Option<f32>,
            reach: &'static str,
            same_landmass: bool,
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct People {
            community: usize,
            coastal: bool,
            landmass: usize,
            nearest_other: Option<usize>,
        }
        #[derive(Serialize)]
        struct Preview {
            pairs: Vec<Pair>,
            peoples: Vec<People>,
        }
        let world = self.chronicle.latest();
        if world.generation != 0 {
            return Err("The founding preview belongs to year zero.".into());
        }
        let finite = |distance: f32| distance.is_finite().then_some(distance);
        let mut pairs = Vec::new();
        for a in world.living() {
            for b in world.living().filter(|&b| b > a) {
                let mut walk = f32::INFINITY;
                let mut voyage = f32::INFINITY;
                for &source in &world.communities[a].lands {
                    for &destination in &world.communities[b].lands {
                        walk = walk.min(world.map.distance(source, destination));
                        voyage = voyage.min(world.map.voyage(source, destination));
                    }
                }
                let reach = if world.nearness(a, b) > 0.0 {
                    "neighbours"
                } else if walk <= world.params.trade_reach {
                    "walking"
                } else if voyage <= world.params.trade_reach {
                    "sea"
                } else {
                    "apart"
                };
                pairs.push(Pair {
                    a,
                    b,
                    walk: finite(walk),
                    voyage: finite(voyage),
                    reach,
                    same_landmass: world.map.regions[world.communities[a].home()].landmass
                        == world.map.regions[world.communities[b].home()].landmass,
                });
            }
        }
        let peoples = world
            .living()
            .map(|community| People {
                community,
                coastal: world.communities[community]
                    .lands
                    .iter()
                    .any(|&r| world.map.coastal(r)),
                landmass: world.map.regions[world.communities[community].home()]
                    .landmass
                    .expect("founders inhabit land"),
                nearest_other: pairs
                    .iter()
                    .filter(|pair| pair.a == community || pair.b == community)
                    .filter_map(|pair| {
                        let other = if pair.a == community { pair.b } else { pair.a };
                        let effort = pair
                            .walk
                            .unwrap_or(f32::INFINITY)
                            .min(pair.voyage.unwrap_or(f32::INFINITY));
                        effort.is_finite().then_some((other, effort))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                    .map(|(other, _)| other),
            })
            .collect();
        to_json(&Preview { pairs, peoples })
    }

    /// Nearby year-zero homelands, ordered by land-only walking effort.
    /// The anchor may be occupied; all returned sites must be unoccupied and fed.
    pub fn founding_sites(&self, region: usize, count: usize) -> Result<String, String> {
        let world = self.chronicle.latest();
        if world.generation != 0 {
            return Err("Founding sites belong to year zero.".into());
        }
        if !(1..=12).contains(&count) {
            return Err("Choose between 1 and 12 founding sites.".into());
        }
        let anchor = world
            .map
            .regions
            .get(region)
            .ok_or("Choose a valid homeland region.")?;
        if !anchor.terrain.is_land() {
            return Err("A homeland must be on land.".into());
        }
        let occupied: HashSet<usize> = world
            .living()
            .flat_map(|c| world.communities[c].lands.iter().copied())
            .collect();
        let feeds = |r| Livelihood::ALL.iter().any(|&way| world.feeds(r, way) > 0.0);
        let mut nearby: Vec<(usize, f32)> = Vec::with_capacity(count);
        for &(r, effort) in world
            .map
            .walking_row(region, world.params.settle_apart)
            .iter()
        {
            let r = r as usize;
            if !effort.is_finite() || occupied.contains(&r) || !feeds(r) {
                continue;
            }
            let at = nearby
                .binary_search_by(|&(id, distance)| distance.total_cmp(&effort).then(id.cmp(&r)))
                .unwrap_or_else(|at| at);
            if at < count {
                if nearby.len() == count {
                    nearby.pop();
                }
                nearby.insert(at, (r, effort));
            }
        }
        let mut sites = [0; 12];
        for (site, &(r, _)) in sites.iter_mut().zip(&nearby) {
            *site = r;
        }
        to_json(&sites[..nearby.len()])
    }

    pub fn settlement(
        &self,
        point: &str,
        community: usize,
        intent: &str,
        share: f32,
        destination: i32,
    ) -> Result<String, String> {
        use umran_sim::settlement::{
            SettlementChoice, SettlementIntent, SettlementOption, SettlementPlan,
        };
        #[derive(Serialize)]
        struct Preview {
            telling: TellingId,
            point: HistoryPoint,
            mutation: u32,
            options: Vec<SettlementOption>,
            plan: Option<SettlementPlan>,
            reason: Option<String>,
        }
        let point: HistoryPoint = serde_json::from_str(point).map_err(|e| e.to_string())?;
        let intent: SettlementIntent =
            serde_json::from_value(serde_json::Value::String(intent.into()))
                .map_err(|e| e.to_string())?;
        let world = self.chronicle.world_at_point(point)?;
        let options = world.settlement_options(community, intent, share)?;
        let (plan, reason) = if destination < 0 {
            (None, None)
        } else {
            match world.plan_settlement(&SettlementChoice {
                community,
                intent,
                destination: destination as usize,
                share,
                naming: None,
                intensity: 0.5,
            }) {
                Ok(plan) => (Some(plan), None),
                Err(reason) => (None, Some(reason)),
            }
        };
        to_json(&Preview {
            telling: self.chronicle.active(),
            point,
            mutation: self.mutation,
            options,
            plan,
            reason,
        })
    }

    /// Eligible catalog laws at an exact reading, without changing the world.
    /// Specimen `was`/`wasIpa` compare immediately before this choice, unlike
    /// a sound-change annal's comparison with the previous generation.
    pub fn law_choices(&self, point: &str, variety: usize) -> Result<String, String> {
        let point: HistoryPoint = serde_json::from_str(point).map_err(|e| e.to_string())?;
        let world = self.chronicle.world_at_point(point)?;
        if !world
            .communities
            .iter()
            .any(|c| c.living() && c.variety == variety)
        {
            return Err(format!("language {variety} has no living speakers"));
        }
        let speech = &world.varieties[variety];
        let stress = speech.stress();
        let mut choices: Vec<LawChoice> = world
            .law_catalog()
            .iter()
            .filter(|law| law.id != "koine-levelling")
            .filter_map(|law| {
                law.assess_weighted(
                    speech.spoken_forms(),
                    &speech.profile.inventory,
                    speech.minimal,
                    stress,
                )?;
                let words = speech
                    .lexicon
                    .living()
                    .filter(|word| {
                        let after = law.apply(&word.form, speech.minimal, stress);
                        law.changes(&word.form, &after, stress)
                    })
                    .count();
                Some(LawChoice {
                    id: law.id,
                    label: law.label,
                    words,
                    specimen: law_specimen(speech, law),
                    recent: speech.laws.iter().rev().any(|&(generation, id)| {
                        id == law.id
                            && world.generation.saturating_sub(generation)
                                < umran_sim::world::LAW_RECURRENCE
                    }),
                })
            })
            .collect();
        // Stable sorting preserves catalog order when coverage is tied.
        choices.sort_by_key(|choice| std::cmp::Reverse(choice.words));
        to_json(&choices)
    }

    /// Communities, varieties, contacts, and the timeline at `generation`.
    pub fn overview(&mut self, generation: u32) -> Result<String, String> {
        self.overview_reading(generation, self.fixed.clone())
    }

    pub fn overview_at(&mut self, point: &str) -> Result<String, String> {
        let point: HistoryPoint = serde_json::from_str(point).map_err(|e| e.to_string())?;
        let world = self.chronicle.world_at_point(point)?;
        self.overview_reading(world.generation, Some((point, world)))
    }

    fn overview_reading(
        &mut self,
        generation: u32,
        exact: Option<(HistoryPoint, World)>,
    ) -> Result<String, String> {
        let latest = self.latest();
        let point = exact
            .as_ref()
            .map_or_else(|| self.chronicle.point_at(generation), |(p, _)| *p);
        let mutation = self.mutation;
        let timeline = self.timeline();
        let seed = self.chronicle.seed;
        let saved_revision = self.saved_revision;
        let tellings = self.tellings();
        let telling = self.chronicle.active();
        let at_tip = self.chronicle.end() == point;
        let world = match &exact {
            Some((_, world)) => world,
            None => self.world(generation),
        };
        let mut annals = annals(world);
        // Authored decisions happen after the year's simulation. Keep their
        // exact order so the chronicle's latest moment is the choice just made.
        annals.sort_by_key(|a| (a.generation, a.before.map(|p| p.action)));
        let spoken = world.spoken();
        // When each language arose: a daughter when it parted, a founding
        // language when its people was founded. A people's first language
        // is the one its first shift left, or the one it speaks if none.
        let mut born: Vec<u32> = world
            .varieties
            .iter()
            .map(|v| v.parent.map_or(0, |f| f.generation))
            .collect();
        for (g, event) in &world.events {
            if let WorldEvent::Found { community } = event {
                let first = world
                    .events
                    .iter()
                    .find_map(|(_, e)| match e {
                        WorldEvent::Shift {
                            community: c, from, ..
                        } if c == community => Some(*from),
                        _ => None,
                    })
                    .unwrap_or(world.communities[*community].variety);
                born[first] = *g;
            }
        }
        let laws = catalog();
        let law_label = |id: &str| {
            laws.iter()
                .find(|l| l.id == id)
                .map_or_else(|| substrate_label(id), |l| l.label.to_string())
        };
        // Every language that hears another people's name, built once:
        // building an ear reads its whole lexicon.
        let mut ears: Vec<Option<umran_sim::adapt::Adapter>> = vec![None; world.varieties.len()];
        for k in &world.contacts {
            for c in [k.a, k.b] {
                let v = world.communities[c].variety;
                if ears[v].is_none() {
                    ears[v] = Some(world.ear(v));
                }
            }
        }
        // Which state, if any, each language is the standard of.
        let standards = world.standards();
        let view = Overview {
            telling,
            at_tip,
            seed,
            point,
            mutation,
            generation: world.generation,
            latest,
            revision: ENGINE_REVISION,
            saved_revision,
            timeline,
            communities: world
                .communities
                .iter()
                .enumerate()
                .map(|(id, c)| CommunityView {
                    id,
                    parents: c.parents.clone(),
                    name: world.community_name(id),
                    meaning: c.name.meaning.clone(),
                    ipa: c.name.form.ipa_stressed(
                        world
                            .variety_of(id)
                            .stress_at(c.ended.unwrap_or(world.generation)),
                    ),
                    coined: c.name.coined,
                    faith: c.faith,
                    crafts: c.crafts.clone(),
                    // Only when it was written differently, not just said so.
                    once: c
                        .name
                        .log
                        .first()
                        .and_then(|e| match &e.event {
                            Event::SoundLaw { before, .. } => {
                                Some(world.variety_of(id).title(before))
                            }
                            _ => None,
                        })
                        .filter(|once| *once != world.community_name(id)),
                    exonyms: world
                        .contacts
                        .iter()
                        .filter_map(|k| match (k.a == id, k.b == id) {
                            (true, _) => Some(k.b),
                            (_, true) => Some(k.a),
                            _ => None,
                        })
                        .map(|by| Exonym {
                            by,
                            name: world.exonym_heard(
                                ears[world.communities[by].variety]
                                    .as_ref()
                                    .expect("every contact's languages have ears"),
                                id,
                                by,
                            ),
                        })
                        // A neighbour that says the name as they do adds nothing.
                        .filter(|e| e.name != world.community_name(id))
                        .collect(),
                    variety: c.variety,
                    size: c.size,
                    prestige: c.prestige,
                    power: c.power,
                    openness: c.openness,
                    ethos: c.ethos_at(generation),
                    region: c.home(),
                    lands: c.lands.clone(),
                    livelihood: c.livelihood,
                    ended: c.ended,
                    ended_into: world.events.iter().find_map(|(_, e)| match *e {
                        WorldEvent::Ended { community, into } if community == id => into,
                        _ => None,
                    }),
                })
                .collect(),
            varieties: world
                .varieties
                .iter()
                .enumerate()
                .map(|(id, v)| {
                    let (consonants, vowels) = v.inventory();
                    VarietyView {
                        id,
                        name: language_label(world, id),
                        meaning: v.name.meaning.clone(),
                        parent: v.parent.map(|f| f.variety),
                        forked_at: v.parent.map(|f| f.generation),
                        koine_of: (!v.koine_of.is_empty()).then(|| makeup_views(&v.koine_of)),
                        family: world.family(id),
                        spoken: spoken[id],
                        born: born[id],
                        // When its last speakers took up another language
                        // or came to an end.
                        silent_since: (!spoken[id])
                            .then(|| {
                                world.events.iter().rev().find_map(|(g, e)| match e {
                                    WorldEvent::Shift { from, .. } if *from == id => Some(*g),
                                    WorldEvent::Ended { community, .. }
                                        if world.communities[*community].variety == id =>
                                    {
                                        Some(*g)
                                    }
                                    _ => None,
                                })
                            })
                            .flatten(),
                        consonants: ipas(&consonants),
                        vowels: ipas(&vowels),
                        laws: v
                            .laws
                            .iter()
                            .map(|&(generation, law)| LawView {
                                generation,
                                id: law,
                                label: law_label(law),
                                from: v
                                    .waves
                                    .iter()
                                    .find(|&&(g, w, _)| (g, w) == (generation, law))
                                    .map(|&(_, _, f)| f),
                                decision: world.authored_laws.get(&(id, generation, law)).copied(),
                            })
                            .collect(),
                        words: v.lexicon.living().count(),
                        word_building: word_building(v),
                        builders: builders(v),
                        minimal_word: v.minimal.label(),
                        stress: v.stress().id(),
                        tones: v.tones(),
                        tonal: v.tonal,
                        geminates: v.lexicon.living().any(|w| {
                            w.form
                                .segs
                                .iter()
                                .any(|s| s.long && !CATALOG.get(s.phone).is_vowel())
                        }),
                        specimen: specimen(v, world.generation),
                        how_synthetic: v.grammar.summary.how_synthetic,
                        contrast_retention: v.grammar.summary.contrast_retention,
                        grammar: grammar_view(world, id),
                        pronouns: pronoun_view(v),
                        harmony: v.harmony.as_ref().map(|h| HarmonyView {
                            feature: h.feature,
                            since: h.since,
                            lost: h.lost,
                        }),
                        standard_of: standards[id],
                        own_words: own_words(world, id),
                        names: v
                            .given
                            .iter()
                            .map(|g| GivenView {
                                name: v.title(&g.name.form),
                                ipa: g.name.form.ipa_stressed(v.stress()),
                                meaning: g.name.meaning.clone(),
                                from: g.from,
                            })
                            .collect(),
                        name_style: v.style,
                        written: v.written,
                        sacred_of: world.religions.iter().position(|r| r.sacred == id),
                        classical_of: world.classical_of(id),
                        high: v.high,
                        vernacular: v.vernacular,
                        // How much of the core its written form still
                        // shares with its speech, while it is spoken.
                        kept_from_high: v.high.filter(|_| spoken[id]).map(|h| {
                            umran_sim::compare::intelligibility(
                                &world.varieties[h].lexicon,
                                &v.lexicon,
                            )
                        }),
                        known_lands: world
                            .known_lands(id)
                            .into_iter()
                            .filter_map(|region| {
                                let name = world.known_place(id, region)?;
                                Some(KnownLandView {
                                    region,
                                    spelled: v.title(&name.form),
                                    ipa: name.form.ipa_stressed(v.stress()),
                                })
                            })
                            .collect(),
                    }
                })
                .collect(),
            states: state_views(world),
            cities: world
                .cities
                .iter()
                .enumerate()
                .map(|(id, city)| CityView {
                    id,
                    state: city.state,
                    region: city.region,
                    name: NameView::new(
                        &world.varieties[world.standard_variety(city.state)],
                        &world.city_name(id),
                    ),
                    size: world.city_size(id),
                    since: city.since,
                    townsfolk: city.townsfolk,
                    makeup: makeup_views(&city.makeup),
                })
                .collect(),
            religions: religion_views(world),
            crafts: craft_views(world),
            continents: continent_views(world),
            contacts: world
                .contacts
                .iter()
                .map(|c| ContactView {
                    a: c.a,
                    b: c.b,
                    intensity: c.intensity,
                    kind: kebab(&format!("{:?}", c.kind)),
                })
                .collect(),
            places: place_views(world),
            moves: move_views(world),
            annals,
            tellings,
        };
        to_json(&view)
    }

    /// How much of its core vocabulary `variety` shares with each other
    /// spoken language at `generation`, the closest first. One language's
    /// row rather than the whole table, which grows with the square of the
    /// languages and would slow every year of a long history.
    pub fn kin(&mut self, generation: u32, variety: usize) -> Result<String, String> {
        let world = self.world(generation);
        let own = &world
            .varieties
            .get(variety)
            .ok_or("No such language variety.")?
            .lexicon;
        let spoken = world.spoken();
        let mut rows: Vec<KinView> = (0..world.varieties.len())
            .filter(|&v| v != variety && spoken[v])
            .map(|other| KinView {
                other,
                score: intelligibility(own, &world.varieties[other].lexicon),
            })
            .collect();
        rows.sort_by(|a, b| b.score.total_cmp(&a.score));
        to_json(&rows)
    }

    /// Every concept's current word in `variety` at `generation`.
    pub fn lexicon(&mut self, generation: u32, variety: usize) -> Result<String, String> {
        let world = self.world(generation);
        let v = world
            .varieties
            .get(variety)
            .ok_or("No such language variety.")?;
        let rows: Vec<LexiconRow> = v
            .lexicon
            .slots
            .iter()
            .filter_map(|slot| {
                let word = v.lexicon.get(slot.dominant()?);
                let (spelled, said) = written_and_said(v, word);
                Some(LexiconRow {
                    concept: slot.concept.id,
                    gloss: slot.concept.gloss,
                    field: slot.concept.field.label(),
                    rank: slot.concept.stability,
                    class: v.gender.class_of(word.id),
                    spelled,
                    said,
                    ipa: word.form.ipa_stressed(v.stress()),
                    origin: origin_view(world, variety, word),
                    changes: word
                        .log
                        .iter()
                        .filter(|e| matches!(e.event, Event::SoundLaw { .. }))
                        .count(),
                    competitors: slot.variants.len() - 1,
                })
            })
            .collect();
        to_json(&rows)
    }

    /// The words competing for `concept` in `variety`, their histories, and
    /// cognates in related languages.
    pub fn word(
        &mut self,
        generation: u32,
        variety: usize,
        concept: &str,
    ) -> Result<String, String> {
        let concept = umran_sim::concepts::by_id(concept).ok_or("Unknown concept.")?;
        let world = self.world(generation);
        let v = world
            .varieties
            .get(variety)
            .ok_or("No such language variety.")?;
        let slot = v.lexicon.slot(concept);
        let mut variants: Vec<VariantView> = slot
            .variants
            .iter()
            .map(|var| {
                let word = v.lexicon.get(var.lexeme);
                let (spelled, said) = written_and_said(v, word);
                VariantView {
                    spelled,
                    said,
                    ipa: word.form.ipa_stressed(v.stress()),
                    share: var.weight,
                    class: v.gender.class_of(word.id),
                    origin: origin_view(world, variety, word),
                    senses: v.lexicon.senses(word.id).map(|c| c.gloss).collect(),
                    history: history(world, variety, word),
                    paradigms: paradigm_views(world, variety, word),
                }
            })
            .collect();
        variants.sort_by(|a, b| b.share.total_cmp(&a.share));
        let cognates = (0..world.varieties.len())
            .filter(|&other| other != variety && world.spoken()[other])
            .filter(|&other| world.cognate(variety, other, concept))
            .filter_map(|other| {
                let o = &world.varieties[other];
                let word = o.lexicon.word_for(concept)?;
                Some(Cognate {
                    variety: other,
                    name: world.language_title(other),
                    spelled: o.spell(&word.form),
                    ipa: word.form.ipa_stressed(o.stress()),
                })
            })
            .collect();
        to_json(&WordView {
            concept: concept.id,
            gloss: concept.gloss,
            field: concept.field.label(),
            rank: concept.stability,
            related: related(concept).map(|c| c.gloss).collect(),
            variants,
            cognates,
        })
    }

    /// The land the history plays out on. It never changes, so the
    /// browser asks once per book.
    pub fn map(&self) -> Result<String, String> {
        let map = &self.chronicle.latest().map;
        to_json(&MapView {
            size: map.size,
            geography: map.geography,
            radius_km: map.radius_km,
            width: map.width,
            height: map.height,
            km_per_unit: KM_PER_UNIT,
            regions: map
                .regions
                .iter()
                .enumerate()
                .map(|(id, r)| RegionView {
                    id,
                    terrain: r.terrain,
                    area_km2: r.area_km2,
                    elevation: r.elevation,
                    moisture: r.moisture,
                    warmth: r.warmth,
                    climate_zone: r.climate_zone,
                    landmass: r.landmass,
                    center: geographic(r.position),
                    boundary: r.boundary.iter().copied().map(geographic).collect(),
                    site: r.site,
                    coastal: map.coastal(id),
                    island: map.island(id),
                    neighbours: r.neighbours.clone(),
                })
                .collect(),
            landmasses: map
                .landmasses
                .iter()
                .enumerate()
                .map(|(id, landmass)| LandmassView {
                    id,
                    kind: landmass.kind,
                    regions: landmass.regions.clone(),
                    anchor: landmass.anchor,
                })
                .collect(),
            rivers: map
                .rivers
                .iter()
                .enumerate()
                .map(|(id, river)| river_view(map, id, river))
                .collect::<Result<Vec<_>, _>>()?,
            climate_zones: map
                .climate_zones
                .iter()
                .enumerate()
                .map(|(id, zone)| ClimateZoneView {
                    id,
                    regions: &zone.regions,
                })
                .collect(),
        })
    }

    /// Climate and literal feeding capacities from the requested world's
    /// replay, not from the present day's conditions.
    pub fn climate(&mut self, generation: u32) -> Result<String, String> {
        let world = self.world(generation);
        to_json(&ClimateView {
            generation: world.generation,
            zones: world
                .climate
                .zones
                .iter()
                .enumerate()
                .map(|(id, zone)| ZoneClimateView {
                    id,
                    epoch: zone.epoch,
                    remaining: zone.remaining,
                    wetness: zone.wetness,
                    warmth: zone.warmth,
                    target_wetness: zone.target_wetness,
                    target_warmth: zone.target_warmth,
                    cause: zone.cause,
                    severity: zone.severity,
                })
                .collect(),
            regions: world
                .climate
                .regions
                .iter()
                .enumerate()
                .map(|(id, region)| RegionClimateView {
                    id,
                    zone: world.map.regions[id].climate_zone,
                    wetness: region.wetness,
                    warmth: region.warmth,
                    vegetation: region.vegetation,
                    river_flow: region.river_flow,
                    feeding: FeedingView {
                        foraging: world.feeds(id, Livelihood::Foraging),
                        herding: world.feeds(id, Livelihood::Herding),
                        farming: world.feeds(id, Livelihood::Farming),
                    },
                    severe: region.severe,
                })
                .collect(),
            rivers: world
                .map
                .rivers
                .iter()
                .enumerate()
                .map(|(id, river)| {
                    let flow = river
                        .course
                        .last()
                        .map_or(0.0, |&region| world.climate.regions[region].river_flow);
                    RiverFlowView {
                        id,
                        flow,
                        flowing: flow >= RIVER_TRAVEL_FLOW,
                    }
                })
                .collect(),
        })
    }

    /// All names and living local forms of a static river identity as
    /// they stood at `generation`.
    pub fn river(&mut self, generation: u32, id: usize) -> Result<String, String> {
        let world = self.world(generation);
        if id >= world.map.rivers.len() {
            return Err("No such river.".into());
        }
        let names = &world.river_names[id];
        let spoken = world.spoken();
        to_json(&RiverNamesView {
            river: id,
            names: names
                .iter()
                .map(|name| place_name_view(world, name))
                .collect(),
            exonyms: world
                .varieties
                .iter()
                .enumerate()
                .filter(|(v, _)| spoken[*v] && names.last().is_some_and(|p| p.variety != *v))
                .filter_map(|(v, speech)| {
                    let (_, name) = speech
                        .river_exonyms
                        .iter()
                        .find(|(river, _)| *river == id)?;
                    Some(place_exonym_view(world, v, name))
                })
                .collect(),
        })
    }

    /// What each living people says for `concept` at `generation`, as a
    /// dialect atlas shows it: words descended from one root share a
    /// group, numbered in order of first appearance.
    pub fn word_map(&mut self, generation: u32, concept: &str) -> Result<String, String> {
        let concept = umran_sim::concepts::by_id(concept).ok_or("Unknown concept.")?;
        let world = self.world(generation);
        let mut roots = Vec::new();
        let words = world
            .communities
            .iter()
            .enumerate()
            .filter_map(|(community, c)| {
                let v = &world.varieties[c.variety];
                let id = v.lexicon.slot(concept).dominant()?;
                let word = v.lexicon.get(id);
                let root = world.root_of(c.variety, id);
                let group = roots.iter().position(|r| *r == root).unwrap_or_else(|| {
                    roots.push(root);
                    roots.len() - 1
                });
                Some(MapWord {
                    community,
                    spelled: v.spell(&word.form),
                    ipa: word.form.ipa_stressed(v.stress()),
                    group,
                    origin: origin_view(world, c.variety, word),
                })
            })
            .collect();
        to_json(&WordMapView {
            concept: concept.id,
            gloss: concept.gloss,
            words,
        })
    }
}

impl Bench {
    fn world(&mut self, generation: u32) -> &World {
        if let Some((_, world)) = &self.fixed {
            return world;
        }
        let target = generation.min(self.latest());
        // The present is always at hand; only the past is replayed.
        if target == self.latest() {
            return self.chronicle.latest();
        }
        if self.cached.as_ref().is_none_or(|w| w.generation != target) {
            self.cached = Some(self.chronicle.world_at(target));
        }
        self.cached.as_ref().expect("just cached")
    }

    /// Actions and world events, in order, with readable labels.
    fn timeline(&self) -> Vec<Marker> {
        let latest = self.chronicle.latest();
        let name = |c: usize| {
            if c < latest.communities.len() {
                latest.community_name(c)
            } else {
                "?".into()
            }
        };
        let mut out: Vec<Marker> = Vec::new();
        for (generation, action) in self.chronicle.timeline() {
            let label = match action {
                Action::Run { generations } => format!("Ran {generations} generations"),
                // Everything else appears as world events below.
                Action::Found { .. }
                | Action::FoundRelated { .. }
                | Action::Settle { .. }
                | Action::Connect { .. }
                | Action::Shift { .. }
                | Action::State { .. }
                | Action::Religion { .. }
                | Action::Temper { .. }
                | Action::Law { .. }
                | Action::Craft { .. } => continue,
            };
            let kind = match action {
                Action::Run { .. } => "run",
                _ => "action",
            };
            out.push(Marker {
                generation,
                kind,
                label,
            });
        }
        for (generation, event) in &latest.events {
            let label = match event {
                WorldEvent::Settlement(record) => format!(
                    "{}: {}",
                    name(record.plan.choice.community),
                    match record.plan.choice.intent {
                        umran_sim::settlement::SettlementIntent::Partition => "lands divided",
                        umran_sim::settlement::SettlementIntent::Settlers => "settlers departed",
                        umran_sim::settlement::SettlementIntent::Migration => "people relocated",
                    }
                ),
                WorldEvent::Found { community } => {
                    out.push(Marker {
                        generation: *generation,
                        kind: "action",
                        label: format!("{} founded", name(*community)),
                    });
                    continue;
                }
                WorldEvent::Split {
                    community,
                    daughter,
                    ..
                } => {
                    format!("{} split from {}", name(*daughter), name(*community))
                }
                WorldEvent::Migrated { community, to, .. } => {
                    let place = latest.place_at(*to, *generation);
                    format!(
                        "{} moved to {}",
                        name(*community),
                        place.as_deref().unwrap_or("new land")
                    )
                }
                WorldEvent::Shift {
                    community, toward, ..
                } => {
                    format!(
                        "{} shifted to {}'s language",
                        name(*community),
                        name(*toward)
                    )
                }
                WorldEvent::Met { a, b, kind } => format!(
                    "{} and {} in {} contact",
                    name(*a),
                    name(*b),
                    kebab(&format!("{kind:?}"))
                ),
                WorldEvent::Parted { a, b, kind } => format!(
                    "{} and {} ended their {} contact",
                    name(*a),
                    name(*b),
                    kebab(&format!("{kind:?}"))
                ),
                WorldEvent::Conquered { ruler, ruled } => {
                    format!("{} conquered {}", name(*ruler), name(*ruled))
                }
                WorldEvent::HardTimes { region, kind, .. } => {
                    let place = latest.place_at(*region, *generation);
                    format!(
                        "{} in {}",
                        kebab(&format!("{kind:?}")),
                        place.as_deref().unwrap_or("a land without a name")
                    )
                }
                WorldEvent::Climate { zone, change, .. } => {
                    format!("Climate {} in zone {zone}", kebab(&format!("{change:?}")))
                }
                WorldEvent::RiverFlow { river, flowing, .. } => {
                    format!(
                        "River {river} {}",
                        if *flowing {
                            "flow returned"
                        } else {
                            "flow fell"
                        }
                    )
                }
                WorldEvent::Adopted {
                    community,
                    livelihood,
                    ..
                } => format!(
                    "{} took to {}",
                    name(*community),
                    livelihood_noun(*livelihood)
                ),
                WorldEvent::Ended { community, into } => match into {
                    Some(into) => format!("{} merged into {}", name(*community), name(*into)),
                    None => format!("{} died out", name(*community)),
                },
                WorldEvent::Rose { state } => {
                    format!("{} arose", latest.states[*state].name.meaning)
                }
                WorldEvent::Fell { state } => {
                    format!("{} fell", latest.states[*state].name.meaning)
                }
                WorldEvent::Standard { state } => {
                    format!("{} took a standard", latest.states[*state].name.meaning)
                }
                WorldEvent::City { city } => {
                    format!("A city grew on land {}", latest.cities[*city].region)
                }
                WorldEvent::Koine { community, .. } => {
                    format!("{} formed their own speech", name(*community))
                }
                WorldEvent::Learnt {
                    community,
                    craft,
                    from: None,
                } => format!("{} came upon {}", name(*community), craft.label()),
                WorldEvent::Revealed { religion } => {
                    let r = &latest.religions[*religion];
                    format!(
                        "{} taught {}",
                        latest.varieties[r.sacred].title(&r.founder.form),
                        r.name.meaning
                    )
                }
                WorldEvent::Schism {
                    religion, parent, ..
                } => {
                    format!(
                        "{} broke from {}",
                        latest.faith_name(*religion),
                        latest.faith_name(*parent)
                    )
                }
                WorldEvent::Fixed { state } => {
                    format!("{} fixed in writing", latest.states[*state].name.meaning)
                }
                // Too frequent to mark: told in the annals instead.
                WorldEvent::Spread { .. }
                | WorldEvent::Displaced { .. }
                | WorldEvent::Learnt { .. }
                | WorldEvent::Converted { .. }
                | WorldEvent::Pejorated { .. }
                | WorldEvent::Respelled { .. }
                | WorldEvent::Pilgrimage { .. }
                | WorldEvent::HolyLand { .. }
                | WorldEvent::Temper { .. }
                | WorldEvent::Tone { .. }
                | WorldEvent::Vernacular { .. } => continue,
            };
            out.push(Marker {
                generation: *generation,
                kind: "event",
                label,
            });
        }
        out.sort_by_key(|m| m.generation);
        out
    }
}

fn word_building(v: &Variety) -> String {
    match v.morphology.kind {
        MorphologyKind::RootPattern => "vowel patterns over consonant roots".into(),
        MorphologyKind::Concatenative => {
            let suffixes = v
                .morphology
                .affixes
                .iter()
                .filter(|(_, a)| a.suffix)
                .count();
            match (suffixes, v.morphology.affixes.len() - suffixes) {
                (_, 0) => "suffixes".into(),
                (0, _) => "prefixes".into(),
                (s, p) if s >= p => "suffixes and some prefixes".into(),
                _ => "prefixes and some suffixes".into(),
            }
        }
    }
}

/// A few basic meanings shown wherever a language appears, so each one can
/// be recognized at a glance and families compared side by side, as in a
/// linguist's comparative word list.
const SPECIMEN: [&str; 6] = ["water", "fire", "stone", "eye", "hand", "night"];

#[derive(Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpecimenWord {
    concept: &'static str,
    gloss: &'static str,
    spelled: String,
    ipa: String,
    /// How it was spelled before `generation`'s sound changes, if they
    /// changed it.
    was: Option<String>,
    stress: Option<usize>,
    was_ipa: Option<String>,
}

/// `variety`'s specimen as it stood after `generation`'s sound changes,
/// through the words it uses now. Words it took up only later are left
/// out.
pub(crate) fn specimen(variety: &Variety, generation: u32) -> Vec<SpecimenWord> {
    SPECIMEN
        .iter()
        .filter_map(|id| {
            let concept = by_id(id).expect("specimen meanings are concepts");
            let word = variety
                .lexicon
                .word_for(concept)
                .filter(|w| w.born <= generation)?;
            let form = word.form_at(generation);
            let spelled = variety.spell(form);
            let before = word.form_at(generation.saturating_sub(1));
            let was = variety.spell(before);
            let stress = variety.stress_at(generation);
            let ipa = form.ipa_stressed(stress);
            let was_ipa = before.ipa_stressed(variety.stress_at(generation.saturating_sub(1)));
            Some(SpecimenWord {
                concept: concept.id,
                gloss: concept.gloss,
                was: (was != spelled).then_some(was),
                was_ipa: (was_ipa != ipa).then_some(was_ipa),
                ipa,
                stress: (form.vowel_count() > 1)
                    .then(|| form.stressed_syllable(stress))
                    .flatten(),
                spelled,
            })
        })
        .collect()
}

/// Preview only this law, including a same-year choice after earlier changes.
fn law_specimen(variety: &Variety, law: &umran_sim::laws::Law) -> Vec<SpecimenWord> {
    let stress = variety.stress();
    let next_stress = law.stress.unwrap_or(stress);
    SPECIMEN
        .iter()
        .filter_map(|id| {
            let concept = by_id(id).expect("specimen meanings are concepts");
            let word = variety.lexicon.word_for(concept)?;
            let after = law.apply(&word.form, variety.minimal, stress);
            let spelled = variety.spell(&after);
            let ipa = after.ipa_stressed(next_stress);
            let was = variety.spell(&word.form);
            let was_ipa = word.form.ipa_stressed(stress);
            Some(SpecimenWord {
                concept: concept.id,
                gloss: concept.gloss,
                was: (was != spelled).then_some(was),
                was_ipa: (was_ipa != ipa).then_some(was_ipa),
                spelled,
                ipa,
                stress: (after.vowel_count() > 1)
                    .then(|| after.stressed_syllable(next_stress))
                    .flatten(),
            })
        })
        .collect()
}

/// Affixes as "-ka" or "ma-"; patterns with C1 C2 C3 for root consonants.
/// Each relation's affix or pattern, then the renewing affix.
fn builders(v: &Variety) -> Vec<Builder> {
    let affix = |a: &umran_sim::morphology::Affix| {
        let written = v.spell(&a.form);
        if a.suffix {
            format!("-{written}")
        } else {
            format!("{written}-")
        }
    };
    let mut out: Vec<Builder> = match v.morphology.kind {
        MorphologyKind::Concatenative => v
            .morphology
            .affixes
            .iter()
            .map(|(r, a)| Builder {
                relation: r.label(),
                shape: affix(a),
            })
            .collect(),
        MorphologyKind::RootPattern => v
            .morphology
            .patterns
            .iter()
            .map(|(r, p)| Builder {
                relation: r.label(),
                shape: p
                    .0
                    .iter()
                    .map(|slot| match slot {
                        Slot::Root(i) => format!("C{}", i + 1),
                        Slot::Fixed(id) => CATALOG.get(*id).ipa().to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(""),
            })
            .collect(),
    };
    out.push(Builder {
        relation: "renewing",
        shape: affix(&v.morphology.renewing),
    });
    out
}

fn continent_views(world: &World) -> Vec<ContinentView> {
    world
        .map
        .landmasses
        .iter()
        .enumerate()
        .filter(|(_, landmass)| landmass.kind == LandmassKind::Continent)
        .map(|(landmass, _)| ContinentView {
            landmass,
            name: world.continent_names[landmass]
                .as_ref()
                .map(|name| ContinentNameView {
                    name: NameView {
                        name: name.spelled.clone(),
                        ipa: name.ipa.clone(),
                        meaning: name.meaning.clone(),
                    },
                    variety: name.variety,
                    people: name.people,
                    witness: name.witness,
                    since: name.since,
                }),
            peoples: world
                .living()
                .filter(|&c| {
                    world.communities[c]
                        .lands
                        .iter()
                        .any(|&r| world.map.regions[r].landmass == Some(landmass))
                })
                .collect(),
            religions: world
                .religions
                .iter()
                .enumerate()
                .filter(|(_, r)| world.map.regions[r.shrine.region].landmass == Some(landmass))
                .map(|(id, _)| id)
                .collect(),
        })
        .collect()
}

/// Every land that has been held, with all its names.
fn place_views(world: &World) -> Vec<PlaceView> {
    let spoken = world.spoken();
    world
        .places
        .iter()
        .enumerate()
        .filter(|(_, names)| !names.is_empty())
        .map(|(region, names)| PlaceView {
            region,
            exonyms: (0..world.varieties.len())
                .filter(|&v| spoken[v] && names.last().is_some_and(|p| p.variety != v))
                .filter_map(|v| {
                    let speech = &world.varieties[v];
                    let (_, name) = speech.exonyms.iter().find(|(r, _)| *r == region)?;
                    Some(place_exonym_view(world, v, name))
                })
                .collect(),
            names: names.iter().map(|p| place_name_view(world, p)).collect(),
        })
        .collect()
}

fn place_exonym_view(world: &World, variety: usize, name: &Name) -> PlaceExonymView {
    let speech = &world.varieties[variety];
    let spelled = speech.title(&name.form);
    PlaceExonymView {
        variety,
        language: language_label(world, variety),
        ipa: name.form.ipa_stressed(speech.stress()),
        heard: name.coined,
        once: Some(speech.title(name.form_at(name.coined))).filter(|once| *once != spelled),
        spelled,
    }
}

fn place_name_view(world: &World, place: &PlaceName) -> PlaceNameView {
    let speech = &world.varieties[place.variety];
    let spelled = speech.title(&place.name.form);
    let until = place
        .name
        .log
        .last()
        .map_or(place.since, |entry| entry.generation.max(place.since));
    PlaceNameView {
        since: place.since,
        variety: place.variety,
        language: world.language_title_at(place.variety, place.since),
        ipa: place.name.form.ipa_stressed(speech.stress_at(until)),
        meaning: place.name.meaning.clone(),
        origin: match place.origin {
            PlaceOrigin::Coined { .. } => "coined",
            PlaceOrigin::Inherited => "inherited",
            PlaceOrigin::Kept => "kept",
            PlaceOrigin::Borrowed => "borrowed",
        },
        by: match place.origin {
            PlaceOrigin::Coined { community } => Some(community),
            _ => None,
        },
        once: Some(speech.title(place.name.form_at(place.since))).filter(|once| *once != spelled),
        spelled,
    }
}

/// Every people's going to new land, in order.
fn move_views(world: &World) -> Vec<MoveView> {
    world
        .events
        .iter()
        .flat_map(|&(generation, ref event)| {
            if let WorldEvent::Settlement(record) = event {
                return record
                    .plan
                    .routes
                    .iter()
                    .filter(|r| r.from != r.to)
                    .map(|r| MoveView {
                        generation,
                        community: record.daughter.unwrap_or(record.plan.choice.community),
                        from: r.from,
                        to: r.to,
                        kind: if record.daughter.is_some() {
                            "split"
                        } else {
                            "migration"
                        },
                        by_sea: r.by_sea,
                        path: r.path.clone(),
                    })
                    .collect::<Vec<_>>();
            }
            let (community, from, to, kind, by_sea) = match *event {
                WorldEvent::Migrated {
                    community,
                    from,
                    to,
                    by_sea,
                } => (community, from, to, "migration", by_sea),
                WorldEvent::Split {
                    daughter,
                    from,
                    to,
                    by_sea,
                    travelled: true,
                    ..
                } if from != to => (daughter, from, to, "split", by_sea),
                _ => return Vec::new(),
            };
            vec![MoveView {
                generation,
                community,
                from,
                to,
                kind,
                by_sea,
                path: world.map.journey_path(from, to, by_sea),
            }]
        })
        .collect()
}

/// Whether the borrowing record explicitly names substrate retention.
fn kept_through_shift(word: &Lexeme) -> bool {
    matches!(
        word.origin,
        Origin::Borrowed {
            cause: umran_sim::LoanCause::Shift { .. },
            ..
        }
    )
}

fn origin_view(world: &World, variety: usize, word: &Lexeme) -> OriginView {
    if kept_through_shift(word) {
        let Origin::Borrowed { from, .. } = word.origin else {
            unreachable!()
        };
        return OriginView {
            kind: "kept",
            from: Some(language_label(world, from)),
            generation: word.born,
            from_variety: Some(from),
            cause: origin_cause(world, word),
        };
    }
    match word.origin {
        Origin::Founding => OriginView {
            kind: "inherited",
            from: None,
            generation: word.born,
            from_variety: None,
            cause: None,
        },
        Origin::Expressive => OriginView {
            kind: "coined",
            from: None,
            generation: word.born,
            from_variety: None,
            cause: None,
        },
        Origin::Derived { base, relation } => OriginView {
            kind: "derived",
            from: Some(format!(
                "{} ({})",
                world.varieties[variety].lexicon.get(base).first_sense.gloss,
                relation.label()
            )),
            generation: word.born,
            from_variety: None,
            cause: None,
        },
        Origin::Renewed { base, with } => {
            let gloss = |id| world.varieties[variety].lexicon.get(id).first_sense.gloss;
            OriginView {
                kind: "derived",
                from: Some(match with {
                    Some(with) => format!("{} + {}", gloss(with), gloss(base)),
                    None => format!("{} (renewed)", gloss(base)),
                }),
                generation: word.born,
                from_variety: None,
                cause: None,
            }
        }
        Origin::Borrowed { from, .. } => OriginView {
            kind: "borrowed",
            from: Some(language_label(world, from)),
            generation: word.born,
            from_variety: Some(from),
            cause: origin_cause(world, word),
        },
    }
}

/// The form a word had just after log entry `i`.
fn form_after(word: &Lexeme, i: usize) -> &Form {
    word.log[i + 1..]
        .iter()
        .find_map(|e| match &e.event {
            Event::SoundLaw { before, .. } => Some(before),
            _ => None,
        })
        .unwrap_or(&word.form)
}

fn history(world: &World, variety: usize, word: &Lexeme) -> Vec<HistoryLine> {
    let laws = catalog();
    let mut out = vec![HistoryLine {
        generation: word.born,
        cause: origin_cause(world, word),
        text: match word.origin {
            Origin::Founding => format!(
                "A root of the founding language, for '{}'",
                word.first_sense.gloss
            ),
            Origin::Expressive => format!("Coined for '{}'", word.first_sense.gloss),
            Origin::Derived { base, relation } => {
                let v = &world.varieties[variety];
                let base = v.lexicon.get(base);
                let how = match v.morphology.kind {
                    MorphologyKind::RootPattern => format!("its {} pattern", relation.label()),
                    MorphologyKind::Concatenative => v
                        .morphology
                        .affixes
                        .iter()
                        .find(|(r, _)| *r == relation)
                        .map_or_else(
                            || format!("its {} ending", relation.label()),
                            |(_, a)| {
                                let written = v.spell(&a.form);
                                if a.suffix {
                                    format!("the {} suffix -{written}", relation.label())
                                } else {
                                    format!("the {} prefix {written}-", relation.label())
                                }
                            },
                        ),
                };
                format!(
                    "Built from '{}' with {how}, for '{}'",
                    base.first_sense.gloss, word.first_sense.gloss
                )
            }
            Origin::Renewed { base, with } => {
                let v = &world.varieties[variety];
                let old = v.lexicon.get(base);
                let worn = v.spell(old.form_at(word.born));
                match with {
                    Some(with) => format!(
                        "Compounded from '{}' and the old word {worn}, for '{}'",
                        v.lexicon.get(with).first_sense.gloss,
                        word.first_sense.gloss
                    ),
                    None => {
                        let affix = &v.morphology.renewing;
                        let written = v.spell(&affix.form);
                        let affix = if affix.suffix {
                            format!("-{written}")
                        } else {
                            format!("{written}-")
                        };
                        format!(
                            "The old word {worn} made fuller with {affix}, for '{}'",
                            word.first_sense.gloss
                        )
                    }
                }
            }
            Origin::Borrowed { from, .. } if kept_through_shift(word) => format!(
                "Kept from {} when its speakers changed language, for '{}'",
                world.language_title(from),
                word.first_sense.gloss
            ),
            Origin::Borrowed { from, .. } => format!(
                "Borrowed from {} for '{}'",
                world.language_title(from),
                word.first_sense.gloss
            ),
        },
    }];
    let kept = kept_through_shift(word);
    let (passing, folded) = passing_senses(word);
    for (i, entry) in word.log.iter().enumerate() {
        if folded.contains(&i) {
            if let Some(&(concept, times)) = passing.get(&i) {
                out.push(HistoryLine {
                    generation: entry.generation,
                    cause: None,
                    text: if times == 1 {
                        format!("For a few generations also used for '{}'", concept.gloss)
                    } else {
                        format!(
                            "Now and then used for '{}' too, never for long ({times} times)",
                            concept.gloss
                        )
                    },
                });
            }
            continue;
        }
        let text = match &entry.event {
            // A kept word was never heard as foreign.
            Event::Borrowed { .. } if kept => continue,
            Event::Borrowed { source, from, .. } => format!(
                "Heard as /{}/, adapted to /{}/",
                source.ipa_stressed(world.varieties[*from].stress_at(entry.generation)),
                form_after(word, i)
                    .ipa_stressed(world.varieties[variety].stress_at(entry.generation))
            ),
            Event::SoundLaw { law, before } => {
                let label = laws
                    .iter()
                    .find(|l| l.id == *law)
                    .map_or_else(|| substrate_label(law), |l| l.label.to_string());
                format!(
                    "{label}: /{}/ → /{}/",
                    before.ipa_stressed(
                        world.varieties[variety].stress_at(entry.generation.saturating_sub(1))
                    ),
                    form_after(word, i)
                        .ipa_stressed(world.varieties[variety].stress_at(entry.generation))
                )
            }
            Event::Extended { to } => format!("Also came to mean '{}'", to.gloss),
            Event::Lost { sense } => format!("No longer used for '{}'", sense.gloss),
            Event::Obsolete => "Fell out of use".into(),
        };
        out.push(HistoryLine {
            generation: entry.generation,
            text,
            cause: match entry.event {
                Event::Borrowed { cause, .. } => Some(cause_view(world, cause, entry.generation)),
                _ => None,
            },
        });
    }
    out
}

/// Longest a word can hold a sense, in generations, for taking it up and
/// dropping it again to be told as a passing use rather than a change of
/// meaning, as a dictionary records only the senses that took hold.
const PASSING_SENSE: u32 = 10;

/// Senses `word` took up and dropped again within `PASSING_SENSE`
/// generations: per concept, how many times, keyed by the log index of its
/// first such use; and every log index those uses span, to fold away.
fn passing_senses(word: &Lexeme) -> (BTreeMap<usize, (&'static Concept, usize)>, HashSet<usize>) {
    let mut first: HashMap<&'static str, usize> = HashMap::new();
    let mut passing = BTreeMap::new();
    let mut folded = HashSet::new();
    for (i, entry) in word.log.iter().enumerate() {
        let Event::Extended { to } = entry.event else {
            continue;
        };
        let lost = word.log[i + 1..]
            .iter()
            .position(|e| matches!(e.event, Event::Lost { sense } if sense.id == to.id))
            .map(|k| i + 1 + k);
        let Some(j) = lost else { continue };
        if word.log[j].generation - entry.generation > PASSING_SENSE {
            continue;
        }
        folded.extend([i, j]);
        let head = *first.entry(to.id).or_insert(i);
        passing.entry(head).or_insert((to, 0)).1 += 1;
    }
    (passing, folded)
}

/// A way of life as the annalist names it: "farming", "herding", or
/// "foraging".
pub(crate) fn livelihood_noun(livelihood: Livelihood) -> &'static str {
    match livelihood {
        Livelihood::Farming => "farming",
        Livelihood::Herding => "herding",
        Livelihood::Foraging => "foraging",
    }
}

pub(crate) fn substrate_label(id: &str) -> String {
    if id == "substrate" {
        "Speakers' old accent merged a sound".into()
    } else {
        id.into()
    }
}

fn ipas(ids: &[PhonemeId]) -> Vec<&'static str> {
    ids.iter().map(|id| CATALOG.get(*id).ipa()).collect()
}

fn kebab(debug: &str) -> String {
    let mut out = String::new();
    for (i, ch) in debug.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('-');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// Preserve the original JS number until it has been checked against the same
/// entity-ID domain on native and wasm32 targets. Bench checks existence.
fn entity_id(value: f64) -> Result<usize, String> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > u32::MAX as f64 {
        return Err("Choose a valid entity ID.".into());
    }
    Ok(value as usize)
}

fn fail(message: String) -> JsValue {
    JsValue::from_str(&message)
}

fn to_json<T: Serialize + ?Sized>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct Choice {
    id: String,
    name: String,
    description: String,
}

fn grammar_choice(choice: GrammarChoice) -> Choice {
    let (name, description) = match choice {
        GrammarChoice::Suffix => (
            "Suffix",
            "A grammatical ending follows the word: one word marks plural, past, or the object.",
        ),
        GrammarChoice::Prefix => (
            "Prefix",
            "A grammatical beginning precedes the word: one word marks plural, past, or the object.",
        ),
        GrammarChoice::Particle => (
            "Separate word",
            "A separate grammatical word marks plural, past, or the object.",
        ),
        GrammarChoice::None => (
            "No marker",
            "The category has no overt marker at founding; marking may develop later.",
        ),
    };
    Choice {
        id: choice.id().into(),
        name: name.into(),
        description: description.into(),
    }
}

#[derive(Serialize)]
struct NameView {
    name: String,
    ipa: String,
    /// What it meant when coined.
    meaning: String,
}

impl NameView {
    fn new(variety: &Variety, name: &Name) -> Self {
        Self {
            name: variety.title(&name.form),
            ipa: name.form.ipa_stressed(
                variety.stress_at(name.log.last().map_or(name.coined, |e| e.generation)),
            ),
            meaning: name.meaning.clone(),
        }
    }
}

#[derive(Serialize)]
struct Exonym {
    /// The community that uses it.
    by: usize,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogView {
    map_sizes: Vec<Choice>,
    sounds: Vec<SoundView>,
    /// Chart columns and rows, in display order.
    places: Vec<&'static str>,
    manners: Vec<&'static str>,
    heights: Vec<&'static str>,
    presets: Vec<Choice>,
    grammar_choices: Vec<Choice>,
    contacts: Vec<Choice>,
    /// What a people can be named for, and the epithets it can take.
    name_places: Vec<&'static str>,
    name_epithets: Vec<&'static str>,
    /// The crafts a people can be taught.
    crafts: Vec<Choice>,
    /// How many meanings the engine knows: the most any language has words for.
    meanings: usize,
    /// The engine revision, for the colophon.
    revision: u32,
}

/// One catalog segment, placed for the sound chart.
/// Concepts shown in a design preview: basics, nursery words, nature, and
/// a few actions and qualities.
const PREVIEW: &[&str] = &[
    "mother", "father", "child", "person", "1sg", "2sg", "fire", "water", "sun", "moon", "star",
    "night", "stone", "tree", "river", "mountain", "eye", "hand", "heart", "blood", "bird", "dog",
    "fish", "go", "see", "eat", "die", "say", "big", "small", "red", "good",
];

#[derive(Serialize)]
struct PreviewWord {
    gloss: &'static str,
    spelled: String,
    ipa: String,
    from: Option<String>,
}

#[derive(Serialize)]
struct Preview {
    people: NameView,
    language: NameView,
    words: Vec<PreviewWord>,
    families: Vec<PreviewWord>,
    /// Concepts sharing a form with another concept.
    homophones: usize,
    /// Mean syllables per word.
    syllables: f32,
}

#[derive(Serialize)]
struct SoundView {
    ipa: &'static str,
    roman: &'static str,
    vowel: bool,
    /// Consonants: place and manner; vowels: backness and height.
    column: &'static str,
    row: &'static str,
    voiced: bool,
    /// "plain", "aspirated", "breathy", "labialized", or "rounded" for
    /// rounded vowels.
    secondary: &'static str,
    /// Share of the world's languages that have it.
    share: f32,
}

const PLACES: [&str; 11] = [
    "bilabial",
    "labiodental",
    "dental",
    "alveolar",
    "postalveolar",
    "retroflex",
    "palatal",
    "velar",
    "uvular",
    "pharyngeal",
    "glottal",
];
const MANNERS: [&str; 12] = [
    "stop",
    "affricate",
    "fricative",
    "nasal",
    "trill",
    "tap",
    "lateral",
    "lateral-fricative",
    "lateral-affricate",
    "approximant",
    "ejective",
    "implosive",
];
const HEIGHTS: [&str; 7] = [
    "close",
    "near-close",
    "close-mid",
    "mid",
    "open-mid",
    "near-open",
    "open",
];

fn sound_view(id: PhonemeId, seg: Segment) -> SoundView {
    let share = umran_sim::typology::share(id);
    match seg {
        Segment::Consonant(c) => SoundView {
            ipa: c.ipa,
            roman: c.roman,
            vowel: false,
            column: PLACES[c.place as usize],
            row: match c.manner {
                Manner::Stop => "stop",
                Manner::Affricate => "affricate",
                Manner::Fricative => "fricative",
                Manner::Nasal => "nasal",
                Manner::Trill => "trill",
                Manner::Tap => "tap",
                Manner::Lateral => "lateral",
                Manner::LateralFricative => "lateral-fricative",
                Manner::LateralAffricate => "lateral-affricate",
                Manner::Approximant => "approximant",
                Manner::Ejective => "ejective",
                Manner::Implosive => "implosive",
            },
            voiced: c.voiced,
            secondary: match c.secondary {
                Secondary::Plain => "plain",
                Secondary::Aspirated => "aspirated",
                Secondary::Breathy => "breathy",
                Secondary::Labialized => "labialized",
            },
            share,
        },
        Segment::Vowel(v) => SoundView {
            ipa: v.ipa,
            roman: v.roman,
            vowel: true,
            column: match v.backness {
                Backness::Front => "front",
                Backness::Central => "central",
                Backness::Back => "back",
            },
            row: HEIGHTS[v.height as usize],
            voiced: true,
            secondary: if v.rounded { "rounded" } else { "plain" },
            share,
        },
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Overview {
    telling: TellingId,
    at_tip: bool,
    seed: u64,
    point: HistoryPoint,
    mutation: u32,
    generation: u32,
    latest: u32,
    revision: u32,
    saved_revision: Option<u32>,
    timeline: Vec<Marker>,
    communities: Vec<CommunityView>,
    varieties: Vec<VarietyView>,
    contacts: Vec<ContactView>,
    /// Every state that has stood, in the order they arose.
    states: Vec<StateView>,
    cities: Vec<CityView>,
    /// Every religion founded, in order.
    religions: Vec<ReligionView>,
    /// Each continent, as it was known in this generation.
    continents: Vec<ContinentView>,
    /// The crafts, where each began, and who holds it now.
    crafts: Vec<CraftView>,
    /// What each land that has been held is called, through history.
    places: Vec<PlaceView>,
    /// Peoples going to new land: migrations, and split-offs settling
    /// away from home.
    moves: Vec<MoveView>,
    /// What happened, in order, told as a chronicle.
    annals: Vec<Annal>,
    /// Tellings set aside, with what they told that this one does not.
    tellings: Vec<TellingView>,
}

#[derive(Serialize)]
struct CityView {
    id: usize,
    state: usize,
    region: usize,
    name: NameView,
    size: f32,
    since: u32,
    townsfolk: Option<usize>,
    makeup: Vec<MakeupView>,
}

#[derive(Serialize)]
struct MakeupView {
    variety: usize,
    share: f32,
}

fn makeup_views(makeup: &[(usize, f32)]) -> Vec<MakeupView> {
    makeup
        .iter()
        .map(|&(variety, share)| MakeupView { variety, share })
        .collect()
}

#[derive(Serialize)]
struct ContinentView {
    landmass: usize,
    name: Option<ContinentNameView>,
    peoples: Vec<usize>,
    religions: Vec<usize>,
}

#[derive(Serialize)]
struct ContinentNameView {
    #[serde(flatten)]
    name: NameView,
    variety: usize,
    people: usize,
    witness: usize,
    since: u32,
}

#[derive(Serialize)]
struct TellingView {
    id: TellingId,
    name: String,
    parent: Option<ReadingRef>,
    from: Option<u32>,
    latest: u32,
    tip: HistoryPoint,
    actions: usize,
}

#[derive(Serialize)]
struct PlaceView {
    region: usize,
    /// Its names, oldest first; the last is its name now.
    names: Vec<PlaceNameView>,
    /// What speakers of other living languages call it now, each heard
    /// from its holders once and changed since by its own sound laws.
    exonyms: Vec<PlaceExonymView>,
}

#[derive(Serialize)]
struct RiverNamesView {
    river: usize,
    names: Vec<PlaceNameView>,
    exonyms: Vec<PlaceExonymView>,
}

#[derive(Serialize)]
struct PlaceExonymView {
    variety: usize,
    language: String,
    spelled: String,
    ipa: String,
    /// The generation its speakers first heard of the land.
    heard: u32,
    /// How it was spelled when they heard it, if it has changed.
    once: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaceNameView {
    /// The generation its speakers came to hold the land.
    since: u32,
    variety: usize,
    /// The language it is a name in, as that language was called then.
    language: String,
    spelled: String,
    ipa: String,
    meaning: String,
    /// "coined", "inherited", "kept", or "borrowed".
    origin: &'static str,
    /// Who coined it, for a coined name.
    by: Option<usize>,
    /// How it was spelled when its speakers took it up, if it has changed.
    once: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveView {
    path: Vec<usize>,
    generation: u32,
    community: usize,
    from: usize,
    to: usize,
    /// "migration" or "split".
    kind: &'static str,
    /// Whether they crossed the sea.
    by_sea: bool,
}

#[derive(Serialize)]
struct Marker {
    generation: u32,
    kind: &'static str,
    label: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommunityView {
    parents: Vec<usize>,
    id: usize,
    /// What it calls itself, spelled in its language.
    name: String,
    meaning: String,
    ipa: String,
    /// Generation the name was coined.
    coined: u32,
    /// How the name was spelled when coined, if sound change has since
    /// altered it.
    once: Option<String>,
    /// What its contacts call it.
    exonyms: Vec<Exonym>,
    variety: usize,
    size: f32,
    prestige: f32,
    power: f32,
    openness: f32,
    ethos: umran_sim::Ethos,
    /// Its heart land, where its name is written on the map.
    region: usize,
    /// Every land it holds, its heart first; for a people that has ended,
    /// the lands it last held.
    lands: Vec<usize>,
    /// How it gets its food: "foraging", "herding", or "farming".
    livelihood: Livelihood,
    /// The generation it ended, if it has.
    ended: Option<u32>,
    /// The people it merged into, if it ended so.
    ended_into: Option<usize>,
    /// The founded religion it holds, if any; else its own folk religion.
    faith: Option<usize>,
    /// The crafts it holds.
    crafts: Vec<Craft>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VarietyView {
    id: usize,
    /// What its speakers call it.
    name: String,
    meaning: String,
    parent: Option<usize>,
    forked_at: Option<u32>,
    koine_of: Option<Vec<MakeupView>>,
    family: usize,
    spoken: bool,
    /// The generation it arose: founded, parted from its parent, or taken
    /// up in a shift.
    born: u32,
    /// When its last speakers took up another language, if they have.
    silent_since: Option<u32>,
    consonants: Vec<&'static str>,
    vowels: Vec<&'static str>,
    laws: Vec<LawView>,
    words: usize,
    /// How it builds words: "suffixes", "prefixes", mixes, or "vowel
    /// patterns over consonant roots".
    word_building: String,
    /// Each relation's affix or pattern, as written.
    builders: Vec<Builder>,
    /// The smallest word sound change leaves: "two syllables".
    minimal_word: &'static str,
    stress: &'static str,
    geminates: bool,
    tones: usize,
    tonal: Option<umran_sim::tone::Tonal>,
    /// A few basic words, to know the language by.
    specimen: Vec<SpecimenWord>,
    /// Grammar expressed within one audible word, across eligible uses.
    how_synthetic: f32,
    /// Eligible uses whose plural, past, or object form differs from the base.
    contrast_retention: f32,
    grammar: GrammarView,
    pronouns: Vec<PronounView>,
    harmony: Option<HarmonyView>,
    /// The standing state whose standard it is, if any.
    standard_of: Option<usize>,
    /// How many of its meanings it says with words of its own.
    own_words: OwnWords,
    /// The given names in fashion, as said now.
    names: Vec<GivenView>,
    /// "single" or "double": one word, or two joined (Wulf-stan).
    name_style: NameStyle,
    /// The generation it was first written, or last respelled.
    written: Option<u32>,
    /// The religion whose sacred language it is, if it is one.
    sacred_of: Option<usize>,
    /// The state whose classical form it is, if it is one.
    classical_of: Option<usize>,
    /// The classical form its speakers write, or wrote before writing
    /// their own speech.
    high: Option<usize>,
    /// When its speakers began to write their own speech in place of
    /// `high`.
    vernacular: Option<u32>,
    /// 0–1: how much of the core vocabulary its speech still shares with
    /// `high`, while it is spoken.
    kept_from_high: Option<f32>,
    /// Named lands remembered by this language, ordered by region.
    known_lands: Vec<KnownLandView>,
}

#[derive(Serialize)]
struct PronounView {
    person: u8,
    number: &'static str,
    polite: bool,
    spelled: String,
    ipa: String,
    since: u32,
    origin: &'static str,
    source: Option<String>,
}

fn pronoun_view(variety: &Variety) -> Vec<PronounView> {
    umran_sim::pronouns::CELLS
        .into_iter()
        .filter_map(|(person, number, cell)| {
            let word = variety
                .lexicon
                .word_for(umran_sim::concepts::by_id(cell)?)?;
            let (origin, source) = match word.origin {
                Origin::Borrowed { from, source, .. } => (
                    "borrowed",
                    Some(format!("variety:{from}:word:{}", source.0)),
                ),
                Origin::Renewed { base, .. } => (
                    "renewed",
                    Some(variety.lexicon.get(base).first_sense.id.to_owned()),
                ),
                _ => ("founding", None),
            };
            Some(PronounView {
                person,
                number,
                polite: cell == "2sg" && variety.pronouns.polite,
                spelled: variety.written_word(word),
                ipa: word.form.ipa_stressed(variety.stress()),
                since: word.born,
                origin,
                source,
            })
        })
        .collect()
}

#[derive(Serialize)]
struct HarmonyView {
    feature: umran_sim::harmony::Feature,
    since: u32,
    lost: Option<u32>,
}

#[derive(Serialize)]
struct HarmonyAlternant {
    spelled: String,
    ipa: String,
}

#[derive(Serialize)]
struct KnownLandView {
    region: usize,
    spelled: String,
    ipa: String,
}

#[derive(Serialize)]
struct Builder {
    relation: &'static str,
    shape: String,
}

#[derive(Serialize)]
struct LawView {
    generation: u32,
    id: &'static str,
    label: String,
    /// The variety it spread from, if it came as a wave from a neighbour
    /// rather than arising here.
    from: Option<usize>,
    /// The authored action that applied this law, if any.
    decision: Option<usize>,
}

#[derive(Serialize)]
struct LawChoice {
    id: &'static str,
    label: &'static str,
    words: usize,
    specimen: Vec<SpecimenWord>,
    recent: bool,
}

#[derive(Serialize)]
struct ContactView {
    a: usize,
    b: usize,
    intensity: f32,
    kind: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StateView {
    id: usize,
    /// Its name, spelled in its rulers' language.
    name: String,
    meaning: String,
    ipa: String,
    /// How the name was spelled when coined, if sound change has altered it.
    once: Option<String>,
    /// Its first ruler, as his name was said then.
    founder: NameView,
    rulers: usize,
    /// Every people it has ruled, in the order they came under it.
    members: Vec<MemberView>,
    capital: usize,
    rose: u32,
    /// "conquest", "proclaimed", or the challenge it answered:
    /// "hard-times", "crowded", "neighbour", or "comfort".
    rise: &'static str,
    fell: Option<u32>,
    /// "rulers-ended", "capital-lost", "conquered", or "collapsed".
    fall: Option<&'static str>,
    /// Who conquered it, if it fell so.
    fallen_to: Option<usize>,
    /// When its court speech became its standard.
    standard: Option<u32>,
    /// 0–1: how closely its standard is guarded against foreign words.
    purism: f32,
    /// Its standard frozen as a classical form, once fixed.
    classical: Option<ClassicalView>,
    /// How many live in its city, the rulers its tribute feeds.
    city: f32,
    /// Every land it holds: its rulers' and its subjects'. Empty once it
    /// has fallen.
    lands: Vec<usize>,
}

#[derive(Serialize)]
struct ClassicalView {
    variety: usize,
    fixed: u32,
    /// "age" when grammarians fixed it, "fall" when its state fell.
    how: umran_sim::Fixing,
}

#[derive(Serialize)]
struct MemberView {
    community: usize,
    joined: u32,
    left: Option<u32>,
}

/// A given name in fashion.
#[derive(Serialize)]
struct GivenView {
    name: String,
    ipa: String,
    /// What its parts meant: "spear-friend".
    meaning: String,
    /// The sacred language it came from with a faith, if it did.
    from: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReligionView {
    id: usize,
    /// Its fixed name, in the speech of its founders or schismatics.
    name: String,
    meaning: String,
    ipa: String,
    founder: NameView,
    /// The people its founder came from, and their heart land then.
    people: usize,
    land: usize,
    founded: u32,
    /// "troubles", "quiet", or "proclaimed".
    how: &'static str,
    /// The variety holding its founder's speech, frozen.
    sacred: usize,
    converts: bool,
    translates: bool,
    scripture: bool,
    /// The living peoples that hold it.
    followers: Vec<usize>,
    /// Its meanings, and how the sacred language and each followers'
    /// language says them.
    words: Vec<RenderingRow>,
    shrine: ShrineView,
    parent: Option<usize>,
    split: Option<u32>,
    cause: Option<SchismCause>,
    named: Option<BranchNaming>,
    branches: Vec<usize>,
    shrines: Vec<ShrineView>,
    holy_land: Vec<HolyLand>,
    pilgrims: Vec<Pilgrimage>,
}

#[derive(Serialize)]
struct ShrineView {
    region: usize,
    kind: SacredKind,
    name: NameView,
}

#[derive(Serialize)]
struct CraftView {
    id: Craft,
    name: &'static str,
    /// The generation some people first held it.
    first: Option<u32>,
    /// The peoples that came upon it themselves.
    inventors: Vec<usize>,
    /// The living peoples that hold it.
    holders: Vec<usize>,
    /// Its meanings, and how each holders' language says them.
    words: Vec<RenderingRow>,
}

/// One meaning an idea brought, as several languages say it.
#[derive(Serialize)]
struct RenderingRow {
    concept: &'static str,
    gloss: &'static str,
    renderings: Vec<Rendering>,
}

#[derive(Serialize)]
struct Rendering {
    variety: usize,
    spelled: String,
    ipa: String,
    /// "borrowed", "kept", "stretched", "built", "coined", or "inherited".
    how: &'static str,
    /// The language it was borrowed from, the meaning it was stretched
    /// from, or the word it was built on.
    from: Option<String>,
}

/// How a language says its meanings: with words of its own, with loans,
/// or with one word stretched over several meanings.
#[derive(Serialize)]
struct OwnWords {
    /// Meanings it has a word for.
    meanings: usize,
    /// Said with a native word used for that meaning alone.
    own: usize,
    /// Said with a word borrowed from another people's language.
    loans: usize,
    /// Said with a word that is also the main word for another meaning.
    shared: usize,
}

fn state_views(world: &World) -> Vec<StateView> {
    world
        .states
        .iter()
        .enumerate()
        .map(|(id, s)| {
            let variety = &world.varieties[world.communities[s.rulers].variety];
            let name = variety.title(&s.name.form);
            let lands: Vec<usize> = if s.standing() {
                let mut lands: Vec<usize> = std::iter::once(s.rulers)
                    .chain(s.subjects())
                    .flat_map(|c| world.communities[c].lands.iter().copied())
                    .collect();
                lands.sort_unstable();
                lands.dedup();
                lands
            } else {
                Vec::new()
            };
            StateView {
                id,
                once: s
                    .name
                    .log
                    .first()
                    .and_then(|e| match &e.event {
                        Event::SoundLaw { before, .. } => Some(variety.title(before)),
                        _ => None,
                    })
                    .filter(|once| *once != name),
                name,
                meaning: s.name.meaning.clone(),
                ipa: s.name.form.ipa_stressed(
                    variety.stress_at(s.name.log.last().map_or(s.name.coined, |e| e.generation)),
                ),
                rulers: s.rulers,
                members: s
                    .members
                    .iter()
                    .map(|m| MemberView {
                        community: m.community,
                        joined: m.joined,
                        left: m.left,
                    })
                    .collect(),
                capital: s.capital,
                rose: s.rose,
                rise: match s.how {
                    Rise::Conquest => "conquest",
                    Rise::Proclaimed => "proclaimed",
                    Rise::Challenge(Challenge::HardTimes) => "hard-times",
                    Rise::Challenge(Challenge::Crowded) => "crowded",
                    Rise::Challenge(Challenge::Neighbour) => "neighbour",
                    Rise::Challenge(Challenge::Comfort) => "comfort",
                },
                fell: s.fell.map(|(g, _)| g),
                fall: s.fell.map(|(_, how)| match how {
                    Fall::RulersEnded => "rulers-ended",
                    Fall::CapitalLost => "capital-lost",
                    Fall::Conquered { .. } => "conquered",
                    Fall::Collapsed => "collapsed",
                }),
                founder: NameView::new(variety, &s.founder),
                fallen_to: match s.fell {
                    Some((_, Fall::Conquered { by })) => Some(by),
                    _ => None,
                },
                standard: s.standard,
                purism: s.purism,
                classical: s.classical.map(|k| ClassicalView {
                    variety: k.variety,
                    fixed: k.fixed,
                    how: k.how,
                }),
                city: world.city(id),
                lands,
            }
        })
        .collect()
}

/// How a craft is called in the workbench.
fn craft_name(craft: Craft) -> &'static str {
    match craft {
        Craft::Metalworking => "Metalworking",
        Craft::Riding => "Riding",
        Craft::Seafaring => "Seafaring",
        Craft::Writing => "Writing",
    }
}

fn craft_description(craft: Craft) -> &'static str {
    match craft {
        Craft::Metalworking => "Bronze and iron: stronger in war.",
        Craft::Riding => "Horses: herders range far and conquer.",
        Craft::Seafaring => "Ships: settle and trade across the sea.",
        Craft::Writing => "Spellings fixed; standards change slower.",
    }
}

/// A language's name for display: a faith's sacred language is the
/// founder's speech as it stood, and a classical form a standard as it
/// stood when fixed, so each is marked as such.
pub(crate) fn language_label(world: &World, variety: usize) -> String {
    let title = world.language_title(variety);
    if world.religions.iter().any(|r| r.sacred == variety) {
        format!("Sacred {title}")
    } else if world.classical_of(variety).is_some() {
        format!("Classical {title}")
    } else {
        title
    }
}

fn grammar_view(world: &World, variety: usize) -> GrammarView {
    let v = &world.varieties[variety];
    GrammarView {
        order: v.grammar.order,
        possessor: v.grammar.possessor,
        marking: if v.grammar.has_object_marking() {
            "case"
        } else {
            "order"
        },
        sample: grammar_sample(v, world.generation),
        classes: v
            .gender
            .classes
            .iter()
            .map(|class| NounClassView {
                id: class.id,
                basis: class.basis,
                marker: (!class.marker.segs.is_empty()).then(|| ClassMarkerView {
                    spelled: v.spell(&class.marker),
                    ipa: class.marker.ipa(),
                }),
                members: class.members,
                born: class.born,
                merged_into: class.merged_into,
            })
            .collect(),
        markers: v
            .grammar
            .markers
            .iter()
            .map(|marker| {
                let form = v.spell(&marker.form);
                let spelled = v.written.map_or_else(
                    || form.clone(),
                    |g| {
                        let g = marker.retired.map_or(g, |retired| g.min(retired));
                        v.spell(grammar_form_at(
                            &marker.form,
                            &marker.history,
                            g.max(marker.born),
                        ))
                    },
                );
                let said = (spelled != form).then(|| form.clone());
                GrammarMarkerView {
                    id: marker.id,
                    category: marker.category.id(),
                    kind: marker.kind.id(),
                    template: marker.template.as_ref().map(|t| t.notation()),
                    side: marker.side.id(),
                    form,
                    spelled,
                    said,
                    ipa: marker
                        .form
                        .ipa_stressed(marker.retired.map_or(v.stress(), |g| v.stress_at(g))),
                    share: v.grammar.marker_share(marker.id),
                    origin: grammar_origin_view(world, variety, marker),
                    born: marker.born,
                    retired: marker.retired,
                    productive: marker.productive,
                    alternants: v
                        .grammar
                        .harmony
                        .filter(|_| {
                            marker.kind == MarkerKind::Bound
                                && marker.retired.is_none()
                                && marker.productive
                        })
                        .map(|feature| {
                            feature
                                .alternants(&marker.form)
                                .iter()
                                .map(|form| HarmonyAlternant {
                                    spelled: v.spell(form),
                                    ipa: form.ipa(),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    history: grammar_history(
                        world,
                        variety,
                        &marker.history,
                        Some(&marker.form),
                        "marker",
                    ),
                }
            })
            .collect(),
        categories: Category::ALL
            .into_iter()
            .enumerate()
            .map(|(index, category)| {
                let summary = &v.grammar.summary.categories[index];
                GrammarCategoryView {
                    category: category.id(),
                    label: category.label(),
                    description: match category {
                        Category::Plural => "Plural marks more than one countable thing.",
                        Category::Past => "Past marks an event before the present.",
                        Category::Object => "The object marks the countable thing acted upon.",
                        Category::Future => "Future marks an event after the present.",
                        Category::Progressive => "Progressive marks an event in progress.",
                        Category::Genitive => "Genitive marks a countable possessor.",
                    },
                    eligible: summary.eligible,
                    how_synthetic: summary.how_synthetic,
                    contrast_retention: summary.contrast_retention,
                }
            })
            .collect(),
    }
}

fn grammar_sample(variety: &Variety, generation: u32) -> Option<GrammarSample> {
    let sentence = grammar_sentence(variety, generation, Category::Past)?;
    let future = grammar_sentence(variety, generation, Category::Future)?;
    let possession_order = match variety.grammar.possessor {
        PossessorOrder::Before => ["child", "fish"],
        PossessorOrder::After => ["fish", "child"],
    };
    let mut possession = GrammarRendering::default();
    for concept in possession_order {
        if concept == "fish" {
            sample_determiner(variety, concept, generation, &mut possession)?;
        }
        let category = (concept == "child").then_some(Category::Genitive);
        sample_word(variety, concept, category, generation, &mut possession)?;
    }
    Some(GrammarSample {
        sentence,
        possession,
        future,
    })
}

fn grammar_sentence(
    variety: &Variety,
    generation: u32,
    tense: Category,
) -> Option<GrammarRendering> {
    let words = [
        ("child", None),
        ("see", Some(tense)),
        ("dog", Some(Category::Object)),
    ];
    let order = match variety.grammar.order {
        WordOrder::SOV => [0, 2, 1],
        WordOrder::SVO => [0, 1, 2],
        WordOrder::VSO => [1, 0, 2],
    };
    let mut sentence = GrammarRendering::default();
    for index in order {
        let (concept, category) = words[index];
        if concept != "see" {
            sample_determiner(variety, concept, generation, &mut sentence)?;
        }
        sample_word(variety, concept, category, generation, &mut sentence)?;
    }
    Some(sentence)
}

fn sample_determiner(
    variety: &Variety,
    concept: &'static str,
    generation: u32,
    rendering: &mut GrammarRendering,
) -> Option<()> {
    let word = variety
        .lexicon
        .word_for(umran_sim::concepts::by_id(concept)?)?;
    if let Some(class) = variety.gender.agreement(word.id) {
        let form = class.agreement_at(generation);
        if !rendering.gloss.is_empty() {
            rendering.text.push(' ');
            rendering.ipa.push(' ');
        }
        rendering.text.push_str(&variety.spell(form));
        rendering.ipa.push_str(&form.ipa_stressed(variety.stress()));
        rendering.gloss.push(format!("DEM.CL{}", class.id));
    }
    Some(())
}

fn sample_word(
    variety: &Variety,
    concept: &'static str,
    category: Option<Category>,
    generation: u32,
    rendering: &mut GrammarRendering,
) -> Option<()> {
    let word = variety
        .lexicon
        .word_for(umran_sim::concepts::by_id(concept)?)?;
    let realization = category.and_then(|category| {
        word.paradigms
            .iter()
            .find(|p| p.category == category)?
            .realizations
            .iter()
            .filter(|r| r.born <= generation && r.retired.is_none() && r.share > 0.0)
            .max_by(|a, b| a.share.total_cmp(&b.share))
    });
    let append = |rendering: &mut GrammarRendering, form: &Form, gloss: String| {
        if !rendering.gloss.is_empty() {
            rendering.text.push(' ');
            rendering.ipa.push(' ');
        }
        rendering.text.push_str(&variety.spell(form));
        rendering.ipa.push_str(&form.ipa_stressed(variety.stress()));
        rendering.gloss.push(gloss);
    };
    if let Some(realization) = realization {
        let marker = variety.grammar.marker(realization.marker);
        let forms = variety
            .grammar
            .surface_at(&word.form, realization, generation);
        if forms.iter().any(|form| form.segs.is_empty()) {
            return None;
        }
        let label = match category? {
            Category::Past => "PAST",
            Category::Object => "OBJ",
            Category::Plural => "PL",
            Category::Future => "FUT",
            Category::Progressive => "PROG",
            Category::Genitive => "GEN",
        };
        for (index, form) in forms.iter().enumerate() {
            let gloss = match marker.kind {
                // A bound mark that sound change has worn to nothing is no
                // longer audible; the sample says so by glossing it bare.
                MarkerKind::Bound | MarkerKind::Pattern if form.segs == word.form.segs => {
                    concept.into()
                }
                MarkerKind::Bound | MarkerKind::Pattern => format!("{concept}-{label}"),
                MarkerKind::Particle
                    if (marker.side == Side::Prefix && index == 0)
                        || (marker.side == Side::Suffix && index == 1) =>
                {
                    label.into()
                }
                _ => concept.into(),
            };
            append(rendering, form, gloss);
        }
    } else {
        if word.form.segs.is_empty() {
            return None;
        }
        append(rendering, &word.form, concept.into());
    }
    Some(())
}

fn grammar_origin_view(world: &World, variety: usize, marker: &GrammarMarker) -> GrammarOriginView {
    let v = &world.varieties[variety];
    match &marker.origin {
        MarkerOrigin::Founding => GrammarOriginView::Founding,
        MarkerOrigin::Grammaticalized {
            source,
            concept,
            source_form,
        } => GrammarOriginView::Grammaticalized {
            source: source.0,
            concept: concept.id,
            gloss: concept.gloss,
            source_form: grammar_form_view(v, source_form, marker.born),
        },
        MarkerOrigin::Fused { particle } => GrammarOriginView::Fused {
            particle: *particle,
        },
        MarkerOrigin::Imported {
            from,
            marker: donor_marker,
            source,
        } => GrammarOriginView::Imported {
            from: *from,
            language: language_label(world, *from),
            marker: *donor_marker,
            source: grammar_form_view(&world.varieties[*from], source, marker.born),
        },
    }
}

fn paradigm_views(world: &World, variety: usize, word: &Lexeme) -> Vec<ParadigmView> {
    let v = &world.varieties[variety];
    word.paradigms
        .iter()
        .map(|paradigm| ParadigmView {
            category: paradigm.category.id(),
            label: paradigm.category.label(),
            realizations: paradigm
                .realizations
                .iter()
                .map(|realization| {
                    let marker = v.grammar.marker(realization.marker);
                    let current = match realization.retired {
                        Some(generation) => {
                            v.grammar
                                .surface_at(word.form_at(generation), realization, generation)
                        }
                        None => v.grammar.surface(&word.form, realization),
                    };
                    let form = spell_surface(v, &current);
                    let spelled = v.written.map_or_else(
                        || form.clone(),
                        |g| {
                            let generation = g.max(word.born).max(realization.born);
                            let generation = realization
                                .retired
                                .map_or(generation, |retired| generation.min(retired));
                            let at = v.grammar.surface_at(
                                word.form_at(generation),
                                realization,
                                generation,
                            );
                            spell_surface(v, &at)
                        },
                    );
                    let said = (spelled != form).then(|| form.clone());
                    RealizationView {
                        marker: marker.id,
                        kind: marker.kind.id(),
                        side: marker.side.id(),
                        form,
                        spelled,
                        said,
                        ipa: ipa_surface(
                            &current,
                            realization.retired.map_or(v.stress(), |g| v.stress_at(g)),
                        ),
                        share: realization.share,
                        born: realization.born,
                        retired: realization.retired,
                        history: grammar_history(
                            world,
                            variety,
                            &realization.history,
                            realization.form.as_ref(),
                            "form",
                        ),
                        marker_history: grammar_history(
                            world,
                            variety,
                            &marker.history,
                            Some(&marker.form),
                            "marker",
                        ),
                    }
                })
                .collect(),
        })
        .collect()
}

fn spell_surface(variety: &Variety, forms: &[Form]) -> String {
    let mut out = String::new();
    for (index, form) in forms.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(&variety.spell(form));
    }
    out
}

fn ipa_surface(forms: &[Form], stress: StressRule) -> String {
    let mut out = String::new();
    for (index, form) in forms.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(&form.ipa_stressed(stress));
    }
    out
}

pub(crate) fn grammar_form_view(
    variety: &Variety,
    form: &Form,
    generation: u32,
) -> GrammarFormView {
    GrammarFormView {
        form: variety.spell(form),
        ipa: form.ipa_stressed(variety.stress_at(generation)),
    }
}

/// A stored grammatical form after a generation, before later changes.
pub(crate) fn grammar_form_at<'a>(
    current: &'a Form,
    entries: &'a [GrammarEntry],
    generation: u32,
) -> &'a Form {
    entries
        .iter()
        .filter(|entry| entry.generation > generation)
        .find_map(|entry| match &entry.event {
            GrammarEvent::SoundLaw { before, .. } | GrammarEvent::Analogy { before } => {
                Some(before)
            }
            _ => None,
        })
        .unwrap_or(current)
}

fn grammar_history(
    world: &World,
    variety: usize,
    entries: &[GrammarEntry],
    current: Option<&Form>,
    subject: &str,
) -> Vec<HistoryLine> {
    let laws = catalog();
    let v = &world.varieties[variety];
    let mut out = Vec::with_capacity(entries.len());
    let mut after = current;
    for entry in entries.iter().rev() {
        let stress = v.stress_at(entry.generation);
        let text = match &entry.event {
            GrammarEvent::Created => match after {
                Some(form) => format!(
                    "Introduced this grammatical {subject} as /{}/",
                    form.ipa_stressed(stress)
                ),
                None => format!("Introduced this grammatical {subject}"),
            },
            GrammarEvent::SoundLaw { law, before } => {
                let initial = v.stress_at(entry.generation.saturating_sub(1));
                let mut prior = initial;
                // Several laws can move stress during one generation.
                // Each row needs the rules immediately before and after its law.
                let transition = v
                    .laws
                    .iter()
                    .filter(|(generation, _)| *generation == entry.generation)
                    .find_map(|(_, id)| {
                        let before = prior;
                        prior = laws
                            .iter()
                            .find(|law| law.id == *id)
                            .and_then(|law| law.stress)
                            .unwrap_or(prior);
                        (*id == *law).then_some((before, prior))
                    });
                let (before_stress, after_stress) = transition.unwrap_or((initial, stress));
                let before = before.ipa_stressed(before_stress);
                let label = laws
                    .iter()
                    .find(|l| l.id == *law)
                    .map_or_else(|| substrate_label(law), |law| law.label.to_string());
                match after {
                    Some(form) => format!(
                        "{label}: /{before}/ → /{}/",
                        form.ipa_stressed(after_stress)
                    ),
                    None => format!("{label}: changed /{before}/"),
                }
            }
            GrammarEvent::Analogy { before } => match after {
                Some(form) => format!(
                    "Reshaped to match the pattern used for new words: /{}/ → /{}/",
                    before.ipa_stressed(stress),
                    form.ipa_stressed(stress)
                ),
                None => format!(
                    "Reshaped /{}/ to match the pattern used for new words",
                    before.ipa_stressed(stress)
                ),
            },
            GrammarEvent::Imported { from, source } => {
                let language = world.language_title_at(*from, entry.generation);
                let source =
                    source.ipa_stressed(world.varieties[*from].stress_at(entry.generation));
                match after {
                    Some(form) => format!(
                        "Imported from {language}, heard as /{source}/ and adapted to /{}/",
                        form.ipa_stressed(stress)
                    ),
                    None => format!("Imported from {language}, heard as /{source}/"),
                }
            }
            GrammarEvent::Retired => format!("This grammatical {subject} fell out of use"),
        };
        out.push(HistoryLine {
            generation: entry.generation,
            text,
            cause: None,
        });
        match &entry.event {
            GrammarEvent::SoundLaw { before, .. } | GrammarEvent::Analogy { before } => {
                after = Some(before);
            }
            _ => {}
        }
    }
    out.reverse();
    out
}

/// A word as written and, when writing has fallen behind speech, as said.
fn written_and_said(variety: &Variety, word: &Lexeme) -> (String, Option<String>) {
    let written = variety.written_word(word);
    let said = variety.spell(&word.form);
    let differs = said != written;
    (written, differs.then_some(said))
}

/// How `variety` came by its word for `concept`: borrowed, stretched from
/// another meaning, built from a word it had, coined, or inherited.
fn rendering(world: &World, variety: usize, concept: &'static Concept) -> Option<Rendering> {
    let v = &world.varieties[variety];
    let word = v.lexicon.word_for(concept)?;
    let (how, from) = if word.first_sense.id != concept.id {
        ("stretched", Some(word.first_sense.gloss.to_string()))
    } else {
        let origin = origin_view(world, variety, word);
        let how = match origin.kind {
            "derived" => "built",
            kind => kind,
        };
        (how, origin.from)
    };
    let (spelled, _) = written_and_said(v, word);
    Some(Rendering {
        variety,
        spelled,
        ipa: word.form.ipa_stressed(v.stress()),
        how,
        from,
    })
}

/// For each meaning that waits for `need`, how each of `varieties` says it.
fn renderings(world: &World, need: Need, varieties: &[usize]) -> Vec<RenderingRow> {
    NEEDS
        .iter()
        .filter(|(_, n)| *n == need)
        .filter_map(|(id, _)| by_id(id))
        .map(|concept| RenderingRow {
            concept: concept.id,
            gloss: concept.gloss,
            renderings: varieties
                .iter()
                .filter_map(|&v| rendering(world, v, concept))
                .collect(),
        })
        .collect()
}

fn religion_views(world: &World) -> Vec<ReligionView> {
    let spoken = world.spoken();
    world
        .religions
        .iter()
        .enumerate()
        .map(|(id, r)| {
            let sacred = &world.varieties[r.sacred];
            let followers: Vec<usize> = world
                .living()
                .filter(|&c| world.communities[c].faith == Some(id))
                .collect();
            // The sacred language first, then each followers' language once.
            let mut varieties = vec![r.sacred];
            for &c in &followers {
                let v = world.communities[c].variety;
                if spoken[v] && !varieties.contains(&v) {
                    varieties.push(v);
                }
            }
            ReligionView {
                id,
                name: world.faith_name(id),
                meaning: r.name.meaning.clone(),
                ipa: NameView::new(&world.varieties[r.name_variety], &r.name).ipa,
                founder: NameView::new(sacred, &r.founder),
                people: r.people,
                land: r.land,
                founded: r.founded,
                how: match r.how {
                    Revelation::Troubles => "troubles",
                    Revelation::Quiet => "quiet",
                    Revelation::Proclaimed => "proclaimed",
                },
                sacred: r.sacred,
                converts: r.converts,
                translates: r.translates,
                scripture: r.scripture,
                followers,
                words: renderings(world, Need::Faith, &varieties),
                shrine: ShrineView {
                    region: r.shrine.region,
                    kind: r.shrine.kind,
                    name: NameView::new(&world.varieties[r.shrine.variety], &r.shrine.name),
                },
                parent: r.parent,
                split: r.split,
                cause: r.cause,
                named: r.named,
                branches: world
                    .religions
                    .iter()
                    .enumerate()
                    .filter_map(|(child, f)| (f.parent == Some(id)).then_some(child))
                    .collect(),
                shrines: r
                    .shrines()
                    .map(|s| ShrineView {
                        region: s.region,
                        kind: s.kind,
                        name: NameView::new(&world.varieties[s.variety], &s.name),
                    })
                    .collect(),
                holy_land: world.holy_lands(id),
                pilgrims: r.pilgrims.clone(),
            }
        })
        .collect()
}

fn craft_views(world: &World) -> Vec<CraftView> {
    let spoken = world.spoken();
    Craft::ALL
        .iter()
        .map(|&craft| {
            let learnt = world.events.iter().filter_map(|(g, e)| match *e {
                WorldEvent::Learnt {
                    community,
                    craft: c,
                    from,
                } if c == craft => Some((*g, community, from)),
                _ => None,
            });
            let first = learnt.clone().map(|(g, _, _)| g).next();
            let inventors = learnt
                .filter(|(_, _, from)| from.is_none())
                .map(|(_, c, _)| c)
                .collect();
            let holders: Vec<usize> = world
                .living()
                .filter(|&c| world.communities[c].crafts.contains(&craft))
                .collect();
            let mut varieties: Vec<usize> = Vec::new();
            for &c in &holders {
                let v = world.communities[c].variety;
                if spoken[v] && !varieties.contains(&v) {
                    varieties.push(v);
                }
            }
            CraftView {
                id: craft,
                name: craft_name(craft),
                first,
                inventors,
                holders,
                words: renderings(world, Need::Craft(craft), &varieties),
            }
        })
        .collect()
}

/// How `variety` says each meaning it has a word for. Words its speakers
/// kept from an older language when they shifted are their own.
fn own_words(world: &World, variety: usize) -> OwnWords {
    let lexicon = &world.varieties[variety].lexicon;
    let dominant: Vec<LexemeId> = lexicon.slots.iter().filter_map(|s| s.dominant()).collect();
    let mut uses: HashMap<LexemeId, usize> = HashMap::new();
    for &id in &dominant {
        *uses.entry(id).or_default() += 1;
    }
    let mut out = OwnWords {
        meanings: dominant.len(),
        own: 0,
        loans: 0,
        shared: 0,
    };
    for &id in &dominant {
        let word = lexicon.get(id);
        if matches!(word.origin, Origin::Borrowed { .. }) && !kept_through_shift(word) {
            out.loans += 1;
        } else if uses[&id] > 1 {
            out.shared += 1;
        } else {
            out.own += 1;
        }
    }
    out
}

/// Another language and the share of core words it shares with one.
#[derive(Serialize)]
struct KinView {
    other: usize,
    score: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OriginView {
    kind: &'static str,
    from: Option<String>,
    generation: u32,
    from_variety: Option<usize>,
    cause: Option<LoanCauseView>,
}

#[derive(Serialize)]
struct LexiconRow {
    concept: &'static str,
    gloss: &'static str,
    field: &'static str,
    rank: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<u32>,
    /// As written: as it sounded when the language was first written.
    spelled: String,
    /// As said now, spelled the same way, if writing has fallen behind.
    said: Option<String>,
    ipa: String,
    origin: OriginView,
    changes: usize,
    competitors: usize,
}

#[derive(Serialize)]
struct HistoryLine {
    generation: u32,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<LoanCauseView>,
}

#[derive(Serialize)]
struct GrammarView {
    order: WordOrder,
    possessor: PossessorOrder,
    marking: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample: Option<GrammarSample>,
    markers: Vec<GrammarMarkerView>,
    classes: Vec<NounClassView>,
    categories: Vec<GrammarCategoryView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NounClassView {
    id: u32,
    basis: umran_sim::gender::Basis,
    marker: Option<ClassMarkerView>,
    members: usize,
    born: u32,
    merged_into: Option<u32>,
}

#[derive(Serialize)]
struct ClassMarkerView {
    spelled: String,
    ipa: String,
}

#[derive(Serialize)]
struct GrammarSample {
    sentence: GrammarRendering,
    possession: GrammarRendering,
    future: GrammarRendering,
}

#[derive(Default, Serialize)]
struct GrammarRendering {
    text: String,
    ipa: String,
    gloss: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GrammarCategoryView {
    category: &'static str,
    label: &'static str,
    description: &'static str,
    eligible: usize,
    how_synthetic: f32,
    contrast_retention: f32,
}

#[derive(Serialize)]
struct GrammarMarkerView {
    id: u32,
    category: &'static str,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    template: Option<String>,
    side: &'static str,
    /// Current spoken form, rendered in the language's spelling.
    form: String,
    /// Spelling frozen when the marker was first written or last respelled.
    spelled: String,
    said: Option<String>,
    ipa: String,
    share: f32,
    origin: GrammarOriginView,
    born: u32,
    retired: Option<u32>,
    productive: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    alternants: Vec<HarmonyAlternant>,
    history: Vec<HistoryLine>,
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
enum GrammarOriginView {
    Founding,
    Grammaticalized {
        source: u32,
        concept: &'static str,
        gloss: &'static str,
        source_form: GrammarFormView,
    },
    Fused {
        particle: u32,
    },
    Imported {
        from: usize,
        language: String,
        marker: u32,
        source: GrammarFormView,
    },
}

#[derive(Clone, PartialEq, Serialize)]
pub(crate) struct GrammarFormView {
    form: String,
    ipa: String,
}

#[derive(Serialize)]
struct ParadigmView {
    category: &'static str,
    label: &'static str,
    realizations: Vec<RealizationView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RealizationView {
    marker: u32,
    kind: &'static str,
    side: &'static str,
    /// Current complete form, with spaces separating a particle from its word.
    form: String,
    /// Complete form frozen at writing or the realization's later birth.
    spelled: String,
    said: Option<String>,
    ipa: String,
    share: f32,
    born: u32,
    retired: Option<u32>,
    history: Vec<HistoryLine>,
    /// The shared marker's own history, especially for separate particles.
    marker_history: Vec<HistoryLine>,
}

#[derive(Serialize)]
struct VariantView {
    spelled: String,
    said: Option<String>,
    ipa: String,
    share: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    class: Option<u32>,
    origin: OriginView,
    senses: Vec<&'static str>,
    history: Vec<HistoryLine>,
    paradigms: Vec<ParadigmView>,
}

#[derive(Serialize)]
struct Cognate {
    variety: usize,
    name: String,
    spelled: String,
    ipa: String,
}

#[derive(Serialize)]
struct WordView {
    concept: &'static str,
    gloss: &'static str,
    field: &'static str,
    rank: Option<u8>,
    related: Vec<&'static str>,
    variants: Vec<VariantView>,
    cognates: Vec<Cognate>,
}

/// Measure the fixed canonical course, including its actual outlet. Drawing
/// coordinates cannot supply a distance on a sphere, especially at a chart seam.
fn river_view<'a>(map: &Map, id: usize, river: &'a River) -> Result<RiverView<'a>, String> {
    let last = *river
        .course
        .last()
        .ok_or_else(|| format!("River {id} has no course."))?;
    let join_at = match river.joins {
        Some(_) => Some(
            map.drainage[last]
                .ok_or_else(|| format!("River {id} has no downstream confluence."))?,
        ),
        None => None,
    };
    let endpoint = if let Some(region) = join_at {
        map.regions[region].position
    } else {
        map.shared_midpoint(last, river.mouth)
            .ok_or_else(|| format!("River {id} has no shared coastal boundary."))?
    };
    let course_radians: f64 = river
        .course
        .windows(2)
        .map(|pair| angular_distance(map.regions[pair[0]].position, map.regions[pair[1]].position))
        .sum();
    let length_km = f64::from(map.radius_km)
        * (course_radians + angular_distance(map.regions[last].position, endpoint));
    Ok(RiverView {
        id,
        course: &river.course,
        mouth: river.mouth,
        catchment: &river.catchment,
        joins: river.joins,
        join_at,
        length_km,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MapView<'a> {
    size: MapSize,
    geography: GeographyVersion,
    radius_km: f32,
    width: f32,
    height: f32,
    /// Equirectangular equatorial drawing scale, not a physical metric elsewhere.
    km_per_unit: f32,
    regions: Vec<RegionView>,
    landmasses: Vec<LandmassView>,
    rivers: Vec<RiverView<'a>>,
    climate_zones: Vec<ClimateZoneView<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegionView {
    id: usize,
    terrain: Terrain,
    area_km2: f32,
    elevation: f32,
    moisture: f32,
    warmth: f32,
    climate_zone: Option<usize>,
    /// Index into the map's landmasses; `None` for sea.
    landmass: Option<usize>,
    /// Longitude east and latitude north, in degrees.
    center: [f64; 2],
    /// Canonical clockwise, unclosed small-polygon ring in geographic degrees.
    boundary: Vec<[f64; 2]>,
    /// Derived equirectangular drawing coordinates, never physical distances.
    site: [f32; 2],
    coastal: bool,
    /// Land on an island rather than a continent.
    island: bool,
    /// Regions sharing a border with it.
    neighbours: Vec<usize>,
}

#[derive(Serialize)]
struct LandmassView {
    id: usize,
    kind: LandmassKind,
    regions: Vec<usize>,
    anchor: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RiverView<'a> {
    id: usize,
    course: &'a [usize],
    mouth: usize,
    catchment: &'a [usize],
    joins: Option<usize>,
    join_at: Option<usize>,
    length_km: f64,
}

#[derive(Serialize)]
struct ClimateZoneView<'a> {
    id: usize,
    regions: &'a [usize],
}

#[derive(Serialize)]
struct ClimateView {
    generation: u32,
    zones: Vec<ZoneClimateView>,
    regions: Vec<RegionClimateView>,
    rivers: Vec<RiverFlowView>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ZoneClimateView {
    id: usize,
    epoch: u32,
    remaining: u32,
    wetness: f32,
    warmth: f32,
    target_wetness: f32,
    target_warmth: f32,
    cause: ClimateCause,
    severity: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegionClimateView {
    id: usize,
    zone: Option<usize>,
    wetness: f32,
    warmth: f32,
    vegetation: Terrain,
    river_flow: f32,
    feeding: FeedingView,
    severe: bool,
}

#[derive(Serialize)]
struct FeedingView {
    foraging: f32,
    herding: f32,
    farming: f32,
}

#[derive(Serialize)]
struct RiverFlowView {
    id: usize,
    flow: f32,
    flowing: bool,
}

#[derive(Serialize)]
struct WordMapView {
    concept: &'static str,
    gloss: &'static str,
    words: Vec<MapWord>,
}

#[derive(Serialize)]
struct MapWord {
    community: usize,
    spelled: String,
    ipa: String,
    /// Words with the same group descend from one root.
    group: usize,
    origin: OriginView,
}

#[cfg(test)]
mod tests {
    use super::*;
    use umran_sim::CONCEPTS;
    use umran_sim::chronicle::SPHERICAL_GEOGRAPHY_REVISION;

    #[test]
    fn entity_ids_preserve_integers_and_reject_js_coercions() {
        for (value, expected) in [(0.0, 0), (42.0, 42), (u32::MAX as f64, u32::MAX as usize)] {
            assert_eq!(entity_id(value).unwrap(), expected);
        }
        for value in [
            -1.0,
            0.5,
            u32::MAX as f64 + 0.5,
            u32::MAX as f64 + 1.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            assert!(entity_id(value).is_err(), "invalid entity ID: {value}");
        }
    }

    const PEOPLE: &str = r#"{"kind":"people"}"#;
    const RIVER: &str = r#"{"kind":"place","place":"river"}"#;

    fn found(name: &str, preset: &str) -> String {
        let design = LanguageDesign::preset(preset, 1).unwrap();
        serde_json::json!({
            "kind": "found", "naming": {"kind": "people"}, "design": design, "seed": name.len(), "power": 0.5, "openness": 0.5
        })
        .to_string()
    }

    fn found_at(name: &str, preset: &str, region: usize) -> String {
        let mut action: serde_json::Value = serde_json::from_str(&found(name, preset)).unwrap();
        action["region"] = region.into();
        action.to_string()
    }

    #[test]
    fn authored_law_choices_match_application_without_mutating_the_reading() {
        let mut bench = Bench::new(5, "medium").unwrap();
        bench.act(&found("Hill", "familiar")).unwrap();
        let point = to_json(&bench.chronicle.end()).unwrap();
        let saved = bench.save().unwrap();
        let before = bench.chronicle.latest().varieties[0].clone();
        let choices: Vec<serde_json::Value> =
            serde_json::from_str(&bench.law_choices(&point, 0).unwrap()).unwrap();
        assert_eq!(bench.save().unwrap(), saved);
        assert!(bench.law_choices(&point, usize::MAX).is_err());
        assert!(
            bench
                .law_choices(r#"{"action":999,"offset":0}"#, 0)
                .is_err()
        );
        assert!(choices.iter().all(|c| c["id"] != "koine-levelling"));
        let catalog = catalog();
        let order =
            |c: &serde_json::Value| catalog.iter().position(|law| law.id == c["id"]).unwrap();
        assert!(choices.windows(2).all(|pair| {
            pair[0]["words"].as_u64() > pair[1]["words"].as_u64()
                || (pair[0]["words"] == pair[1]["words"] && order(&pair[0]) < order(&pair[1]))
        }));
        assert!(choices.iter().any(|c| c["id"] == "spirantization"));
        for choice in &choices {
            let mut applied = Bench::load(&saved).unwrap();
            applied
                .act(
                    &serde_json::json!({
                        "kind": "law", "variety": 0, "law": choice["id"]
                    })
                    .to_string(),
                )
                .unwrap();
            let world = applied.chronicle.latest();
            let after = &world.varieties[0];
            let changed = before
                .lexicon
                .living()
                .filter(|word| {
                    let now = after.lexicon.get(word.id);
                    word.form != now.form
                        || word.form.stressed_syllable(before.stress())
                            != now.form.stressed_syllable(after.stress())
                })
                .count();
            assert_eq!(choice["words"], changed, "{}", choice["id"]);
            let actual = serde_json::to_value(specimen(after, 0)).unwrap();
            for (preview, current) in choice["specimen"]
                .as_array()
                .unwrap()
                .iter()
                .zip(actual.as_array().unwrap())
            {
                for field in ["concept", "gloss", "spelled", "ipa", "stress"] {
                    assert_eq!(preview[field], current[field], "{} {field}", choice["id"]);
                }
                let original = before
                    .lexicon
                    .word_for(by_id(preview["concept"].as_str().unwrap()).unwrap())
                    .unwrap();
                let spelling = before.spell(&original.form);
                let ipa = original.form.ipa_stressed(before.stress());
                assert_eq!(
                    preview["was"],
                    if preview["spelled"] != spelling {
                        serde_json::json!(spelling)
                    } else {
                        serde_json::Value::Null
                    }
                );
                assert_eq!(
                    preview["wasIpa"],
                    if preview["ipa"] != ipa {
                        serde_json::json!(ipa)
                    } else {
                        serde_json::Value::Null
                    }
                );
            }
            assert_eq!(world.decisions.last().unwrap().action, 1);
            let overview: serde_json::Value =
                serde_json::from_str(&applied.overview(0).unwrap()).unwrap();
            assert_eq!(overview["varieties"][0]["laws"][0]["decision"], 1);
            let annal = overview["annals"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["kind"] == "law")
                .unwrap();
            assert_eq!(annal["decision"], 1);
            assert_eq!(annal["before"], serde_json::json!({"action":1,"offset":0}));
            let decisions: serde_json::Value =
                serde_json::from_str(&applied.decisions().unwrap()).unwrap();
            assert_eq!(decisions[1]["annals"][0], annal["id"]);
            assert_eq!(decisions[1]["people"], serde_json::json!([0]));
            let reloaded = Bench::load(&applied.save().unwrap()).unwrap();
            assert_eq!(
                reloaded.chronicle.latest().authored_laws,
                applied.chronicle.latest().authored_laws
            );
        }
    }

    #[test]
    fn authored_law_choices_allow_recent_stress_laws_and_resolve_exact_readings() {
        let mut bench = Bench::new(5, "medium").unwrap();
        let mut action: serde_json::Value =
            serde_json::from_str(&found("Hill", "familiar")).unwrap();
        action["design"]["stress"] = "final".into();
        bench.act(&action.to_string()).unwrap();
        let start = to_json(&bench.chronicle.end()).unwrap();
        let original = bench.law_choices(&start, 0).unwrap();
        for law in ["initial-stress", "penult-stress"] {
            bench
                .act(&serde_json::json!({"kind":"law","variety":0,"law":law}).to_string())
                .unwrap();
        }
        let point = to_json(&bench.chronicle.end()).unwrap();
        let choices: Vec<serde_json::Value> =
            serde_json::from_str(&bench.law_choices(&point, 0).unwrap()).unwrap();
        let initial = choices
            .iter()
            .find(|c| c["id"] == "initial-stress")
            .unwrap();
        assert_eq!(initial["recent"], true);
        bench
            .act(r#"{"kind":"law","variety":0,"law":"initial-stress"}"#)
            .unwrap();
        assert_eq!(bench.law_choices(&start, 0).unwrap(), original);
        assert_eq!(
            bench.chronicle.latest().authored_laws[&(0, 0, "initial-stress")],
            3
        );
        let overview: serde_json::Value =
            serde_json::from_str(&bench.overview(0).unwrap()).unwrap();
        let annal = overview["annals"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["kind"] == "law")
            .unwrap();
        assert_eq!(annal["decision"], 3);
        let before = serde_json::to_string(&serde_json::json!({"action":3,"offset":0})).unwrap();
        let earlier: serde_json::Value =
            serde_json::from_str(&bench.overview_at(&before).unwrap()).unwrap();
        assert_eq!(earlier["varieties"][0]["laws"][0]["decision"], 1);
    }

    #[test]
    fn authored_law_specimens_preserve_minimal_words_and_show_stress_only_changes() {
        let mut profile = umran_sim::SoundProfile::base();
        profile.stress = Some(StressRule::Final);
        let mut variety = Variety::found(7, &profile, Livelihood::Farming, Default::default());
        let water = variety
            .lexicon
            .word_for(by_id("water").unwrap())
            .unwrap()
            .id;
        variety.lexicon.get_mut(water).form = Form::from_ipa("katata").unwrap();
        let laws = catalog();
        let stress = laws.iter().find(|l| l.id == "initial-stress").unwrap();
        let rows = law_specimen(&variety, stress);
        let preview = rows.iter().find(|w| w.concept == "water").unwrap();
        assert_eq!(preview.ipa, "ˈkatata");
        assert_eq!(preview.was_ipa.as_deref(), Some("kataˈta"));
        assert_eq!(preview.was, None);
        assert_eq!(preview.stress, Some(0));
        assert_eq!(variety.stress(), StressRule::Final);

        let apocope = laws.iter().find(|l| l.id == "apocope").unwrap();
        variety.minimal = umran_sim::prosody::MinimalWord::TwoSyllables;
        variety.lexicon.get_mut(water).form = Form::from_ipa("kata").unwrap();
        let rows = law_specimen(&variety, apocope);
        let preview = rows.iter().find(|w| w.concept == "water").unwrap();
        assert_eq!(preview.spelled, "kata");
        assert_eq!(preview.was, None);
        assert_eq!(preview.was_ipa, None);
        variety.minimal = umran_sim::prosody::MinimalWord::Syllable;
        let rows = law_specimen(&variety, apocope);
        let preview = rows.iter().find(|w| w.concept == "water").unwrap();
        assert_eq!(preview.spelled, "kat");
        assert_eq!(preview.was.as_deref(), Some("kata"));
        variety.lexicon.get_mut(water).form = Form::from_ipa("ta").unwrap();
        let rows = law_specimen(&variety, apocope);
        let preview = rows.iter().find(|w| w.concept == "water").unwrap();
        assert_eq!(preview.spelled, "ta");
        assert_eq!(preview.was, None);
    }

    fn bench() -> Bench {
        let mut w = Bench::new(5, "medium").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        let home = w.chronicle.latest().communities[0].home();
        w.act(&found_at("Coast", "polynesian", home)).unwrap();
        w.act(r#"{"kind":"connect","a":0,"b":1,"intensity":0.6,"contact":"trade"}"#)
            .unwrap();
        w.act(r#"{"kind":"run","generations":12}"#).unwrap();
        settle(&mut w, 0.3);
        w.act(r#"{"kind":"run","generations":10}"#).unwrap();
        w
    }

    fn settle(w: &mut Bench, intensity: f32) {
        use umran_sim::settlement::{SettlementChoice, SettlementIntent};
        for intent in [SettlementIntent::Partition, SettlementIntent::Settlers] {
            let options = w
                .chronicle
                .latest()
                .settlement_options(0, intent, 0.5)
                .unwrap();
            if let Some(option) = options.into_iter().find(|o| o.reason.is_none()) {
                w.act(
                    &serde_json::to_string(&Action::Settle {
                        choice: SettlementChoice {
                            community: 0,
                            destination: option.region,
                            intent,
                            share: 0.5,
                            naming: None,
                            intensity,
                        },
                    })
                    .unwrap(),
                )
                .unwrap();
                return;
            }
        }
        panic!("this fixture needs somewhere to settle");
    }

    #[test]
    fn decisions_link_exact_actions_without_claiming_tick_annals() {
        let mut w = Bench::new(5, "medium").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        let home = w.chronicle.latest().communities[0].home();
        w.act(&found_at("Coast", "polynesian", home)).unwrap();
        w.act(r#"{"kind":"connect","a":0,"b":1,"intensity":0.6,"contact":"trade"}"#)
            .unwrap();
        let tick_start = w.chronicle.latest().events.len();
        w.act(r#"{"kind":"run","generations":40}"#).unwrap();
        let tick_end = w.chronicle.latest().events.len();
        let before_shift = w.chronicle.end();
        w.act(r#"{"kind":"shift","community":1,"toward":0}"#)
            .unwrap();
        let world = w.chronicle.latest();
        let entries = annals(world);
        let leaves: Vec<_> = entries
            .iter()
            .flat_map(|a| {
                if a.members.is_empty() {
                    std::slice::from_ref(a)
                } else {
                    a.members.as_slice()
                }
            })
            .collect();
        for a in &leaves {
            let expected =
                a.id.strip_prefix("world:")
                    .map(|p| p.parse::<usize>().unwrap())
                    .and_then(|p| world.decisions.iter().find(|d| d.events.contains(&p)))
                    .map(|d| d.action);
            assert_eq!(a.decision, expected, "{}", a.id);
            assert_eq!(
                a.before,
                expected.map(|action| HistoryPoint { action, offset: 0 })
            );
        }
        for position in tick_start..tick_end {
            let a = leaves
                .iter()
                .find(|a| a.id == annals::world_event_id(position))
                .unwrap();
            assert_eq!(a.decision, None);
        }
        assert_eq!(
            leaves
                .iter()
                .filter(|a| a.kind == "found")
                .map(|a| a.decision)
                .collect::<Vec<_>>(),
            vec![Some(0), Some(1)]
        );
        let decisions: serde_json::Value = serde_json::from_str(&w.decisions().unwrap()).unwrap();
        let decisions = decisions.as_array().unwrap();
        assert_eq!(
            decisions
                .iter()
                .map(|d| (
                    d["index"].as_u64().unwrap(),
                    d["generation"].as_u64().unwrap(),
                    d["kind"].as_str().unwrap()
                ))
                .collect::<Vec<_>>(),
            vec![
                (0, 0, "found"),
                (1, 0, "found"),
                (2, 0, "connect"),
                (4, 40, "shift")
            ]
        );
        for (view, decision) in decisions.iter().zip(&world.decisions) {
            let expected: Vec<_> = decision
                .events
                .clone()
                .map(annals::world_event_id)
                .collect();
            assert_eq!(view["annals"], serde_json::json!(expected));
        }
        assert_eq!(decisions[0]["people"], serde_json::json!([0]));
        assert_eq!(decisions[1]["variety"], 1);
        assert_eq!(decisions[2]["people"], serde_json::json!([0, 1]));
        assert_eq!(decisions[3]["variety"], world.communities[1].variety);
        let point = to_json(&before_shift).unwrap();
        let reading = w.read(0, &point).unwrap();
        let past: serde_json::Value = serde_json::from_str(&reading.decisions().unwrap()).unwrap();
        assert_eq!(past, serde_json::json!(&decisions[..3]));
        let overview: serde_json::Value =
            serde_json::from_str(&w.overview_at(&point).unwrap()).unwrap();
        assert!(
            overview["annals"]
                .as_array()
                .unwrap()
                .iter()
                .all(|a| a["decision"] != 4)
        );
        let live: serde_json::Value =
            serde_json::from_str(&w.overview(w.latest()).unwrap()).unwrap();
        assert_eq!(
            live["annals"].as_array().unwrap().last().unwrap()["before"],
            serde_json::json!({"action":4,"offset":0})
        );
    }

    #[test]
    fn empty_decisions_and_grouped_members_keep_their_own_authorship() {
        let mut w = Bench::new(5, "medium").unwrap();
        let mut founder: serde_json::Value =
            serde_json::from_str(&found("Hill", "familiar")).unwrap();
        founder["ethos"] = serde_json::json!({"martial":0.0});
        w.act(&founder.to_string()).unwrap();
        w.act(r#"{"kind":"temper","community":0,"axis":"martial","amount":0.01}"#)
            .unwrap();
        let home = w.chronicle.latest().communities[0].home();
        w.act(&found_at("Coast", "polynesian", home)).unwrap();
        w.act(r#"{"kind":"connect","a":0,"b":1,"intensity":0.6,"contact":"neighbours"}"#)
            .unwrap();
        w.act(r#"{"kind":"connect","a":0,"b":1,"intensity":0.7,"contact":"neighbours"}"#)
            .unwrap();
        let world = w.chronicle.latest();
        assert!(world.decisions[1].events.is_empty());
        let entries = annals(world);
        let group = entries.iter().find(|a| a.kind == "neighbours").unwrap();
        assert_eq!(group.decision, group.members[0].decision);
        assert_eq!(group.before, group.members[0].before);
        for member in &group.members {
            let position = member
                .id
                .strip_prefix("world:")
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let decision = world
                .decisions
                .iter()
                .find(|d| d.events.contains(&position))
                .unwrap();
            assert_eq!(member.decision, Some(decision.action));
            assert_eq!(member.before.unwrap().action, decision.action);
        }
        let views: serde_json::Value = serde_json::from_str(&w.decisions().unwrap()).unwrap();
        assert_eq!(views[1]["kind"], "temper");
        assert_eq!(views[1]["annals"], serde_json::json!([]));
        assert!(views.as_array().unwrap().iter().all(|v| {
            v["annals"].as_array().unwrap().iter().all(|id| {
                entries.iter().any(|a| {
                    a.id == id.as_str().unwrap()
                        || a.members.iter().any(|m| m.id == id.as_str().unwrap())
                })
            })
        }));
    }

    #[test]
    fn document_title_author_round_trip_without_changing_recipe() {
        let mut w = bench();
        let recipe = w.chronicle.recipe();
        w.set_title("Hūhupam");
        w.set_author("Sol");
        let saved = w.save().unwrap();
        let mut loaded = Bench::load(&saved).unwrap();
        assert_eq!(loaded.title(), Some("Hūhupam"));
        assert_eq!(loaded.author(), Some("Sol"));
        assert_eq!(loaded.chronicle.recipe(), recipe);
        assert_eq!(loaded.read(0, "").unwrap().title(), Some("Hūhupam"));
        loaded.set_title("");
        loaded.set_author("");
        let cleared = Bench::load(&loaded.save().unwrap()).unwrap();
        assert_eq!((cleared.title(), cleared.author()), (None, None));
        let legacy = Bench::load(&to_json(&recipe).unwrap()).unwrap();
        assert_eq!((legacy.title(), legacy.author()), (None, None));
        assert_eq!(legacy.chronicle.recipe(), recipe);
    }

    #[test]
    fn a_same_year_change_invalidates_a_settlement_preview_without_mutation() {
        let mut w = Bench::new(5, "medium").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        let point = serde_json::to_string(&w.chronicle.end()).unwrap();
        let preview: serde_json::Value =
            serde_json::from_str(&w.settlement(&point, 0, "settlers", 0.5, -1).unwrap()).unwrap();
        let destination = preview["options"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["reason"].is_null())
            .unwrap()["region"]
            .as_i64()
            .unwrap();
        let choice: serde_json::Value = serde_json::from_str(
            &w.settlement(&point, 0, "settlers", 0.5, destination as i32)
                .unwrap(),
        )
        .unwrap();
        let mut action = choice["plan"]["choice"].clone();
        action["kind"] = "settle".into();
        w.act(r#"{"kind":"craft","community":0,"craft":"writing"}"#)
            .unwrap();
        let before = w.save().unwrap();
        let view = w.overview(0).unwrap();
        assert!(
            w.act_at(
                &point,
                choice["mutation"].as_u64().unwrap() as u32,
                &action.to_string()
            )
            .is_err()
        );
        assert_eq!(w.save().unwrap(), before);
        assert_eq!(w.overview(0).unwrap(), view);
    }

    #[test]
    fn climate_scrubbing_restores_past_conditions_after_advancing() {
        let mut w = Bench::new(5, "medium").unwrap();
        let initial: serde_json::Value = serde_json::from_str(&w.climate(0).unwrap()).unwrap();
        w.act(r#"{"kind":"run","generations":12}"#).unwrap();
        let present: serde_json::Value = serde_json::from_str(&w.climate(12).unwrap()).unwrap();
        assert_eq!(present["generation"], 12);
        assert_ne!(present["zones"], initial["zones"]);
        let past: serde_json::Value = serde_json::from_str(&w.climate(0).unwrap()).unwrap();
        assert_eq!(past, initial);
        let clamped: serde_json::Value =
            serde_json::from_str(&w.climate(u32::MAX).unwrap()).unwrap();
        assert_eq!(clamped, present);
    }

    #[test]
    fn river_scrubbing_excludes_names_coined_after_the_requested_generation() {
        let mut w = Bench::new(5, "medium").unwrap();
        let home = w.chronicle.latest().map.rivers[0].course[0];
        let initial: serde_json::Value = serde_json::from_str(&w.river(0, 0).unwrap()).unwrap();
        w.act(r#"{"kind":"run","generations":1}"#).unwrap();
        w.act(&found_at("River", "familiar", home)).unwrap();
        let present: serde_json::Value = serde_json::from_str(&w.river(1, 0).unwrap()).unwrap();
        assert_eq!(present["names"][0]["since"], 1);
        let past: serde_json::Value = serde_json::from_str(&w.river(0, 0).unwrap()).unwrap();
        assert_eq!(past, initial);
        assert!(w.river(0, usize::MAX).is_err());
    }

    #[test]
    fn ethos_overview_scrubs_and_authored_nudges_report_thresholds() {
        let mut w = Bench::new(5, "medium").unwrap();
        let mut action: serde_json::Value =
            serde_json::from_str(&found("Hill", "familiar")).unwrap();
        action["ethos"] = serde_json::json!({"martial": 0.4});
        w.act(&action.to_string()).unwrap();
        let initial: serde_json::Value = serde_json::from_str(&w.overview(0).unwrap()).unwrap();
        w.act(r#"{"kind":"run","generations":1}"#).unwrap();
        w.act(r#"{"kind":"temper","community":0,"axis":"martial","amount":0.4}"#)
            .unwrap();
        let now: serde_json::Value = serde_json::from_str(&w.overview(1).unwrap()).unwrap();
        assert!(now["communities"][0]["ethos"]["martial"].as_f64().unwrap() > 0.75);
        let past: serde_json::Value = serde_json::from_str(&w.overview(0).unwrap()).unwrap();
        assert_eq!(
            past["communities"][0]["ethos"],
            initial["communities"][0]["ethos"]
        );
        let annals = now["annals"].as_array().unwrap();
        let temper = annals
            .iter()
            .find(|a| a["kind"] == "temper" && a["temper"]["cause"] == "fate")
            .unwrap();
        assert_eq!(temper["peoples"], serde_json::json!([0]));
        assert_eq!(
            temper["temper"],
            serde_json::json!({"axis":"martial","pole":"high","entered":true,"cause":"fate"})
        );
        assert!(
            annals
                .iter()
                .filter(|a| a["kind"] != "temper")
                .all(|a| a.get("temper") == Some(&serde_json::Value::Null))
        );
        assert!(
            w.act(r#"{"kind":"temper","community":0,"axis":"martial","amount":2}"#)
                .is_err()
        );
        let after: serde_json::Value = serde_json::from_str(&w.overview(1).unwrap()).unwrap();
        assert_eq!(
            after["communities"][0]["ethos"],
            now["communities"][0]["ethos"]
        );
    }

    #[test]
    fn landmasses_partition_land_and_identify_islands() {
        let w = Bench::new(5, "large").unwrap();
        let map: serde_json::Value = serde_json::from_str(&w.map().unwrap()).unwrap();
        let regions = map["regions"].as_array().unwrap();
        let landmasses = map["landmasses"].as_array().unwrap();
        for (id, landmass) in landmasses.iter().enumerate() {
            assert_eq!(landmass["id"], id);
            let members = landmass["regions"].as_array().unwrap();
            assert!(members.contains(&landmass["anchor"]));
            assert!(
                members
                    .windows(2)
                    .all(|pair| pair[0].as_u64() < pair[1].as_u64())
            );
            for member in members {
                let region = &regions[member.as_u64().unwrap() as usize];
                assert_eq!(region["landmass"], id);
                assert_ne!(region["terrain"], "sea");
                assert_eq!(region["island"], landmass["kind"] == "island");
            }
        }
        for region in regions {
            if region["terrain"] == "sea" {
                assert!(region["landmass"].is_null());
                assert_eq!(region["island"], false);
            } else {
                let landmass = &landmasses[region["landmass"].as_u64().unwrap() as usize];
                assert!(
                    landmass["regions"]
                        .as_array()
                        .unwrap()
                        .contains(&region["id"])
                );
            }
        }
    }

    #[test]
    fn spherical_map_exposes_physical_radius_and_consistent_source_chart() {
        for (size, count, radius) in [
            ("small", 642, 800.0),
            ("medium", 2562, 1600.0),
            ("large", 2562, 3200.0),
            ("vast", 10242, 6371.0),
        ] {
            let bench = Bench::new(5, size).unwrap();
            let map: serde_json::Value = serde_json::from_str(&bench.map().unwrap()).unwrap();
            let width = map["width"].as_f64().unwrap();
            let height = map["height"].as_f64().unwrap();
            let scale = map["kmPerUnit"].as_f64().unwrap();
            assert_eq!(map["radiusKm"], radius);
            assert!((width * scale / radius - 2.0 * std::f64::consts::PI).abs() < 1e-6);
            assert!((height * 2.0 - width).abs() < width * f64::from(f32::EPSILON));
            let regions = map["regions"].as_array().unwrap();
            assert_eq!(regions.len(), count);
            for region in regions {
                let lon = region["center"][0].as_f64().unwrap();
                let lat = region["center"][1].as_f64().unwrap();
                assert!((-180.0..=180.0).contains(&lon));
                assert!((-90.0..=90.0).contains(&lat));
                let x = region["site"][0].as_f64().unwrap();
                let y = region["site"][1].as_f64().unwrap();
                assert!((x - (lon + 180.0) / 360.0 * width).abs() < width * 1e-6);
                assert!((y - (90.0 - lat) / 180.0 * height).abs() < height * 1e-6);
                let ring = region["boundary"].as_array().unwrap();
                assert!(ring.len() >= 3);
                assert_ne!(ring.first(), ring.last(), "canonical rings are unclosed");
                for vertex in ring {
                    assert!((-180.0..=180.0).contains(&vertex[0].as_f64().unwrap()));
                    assert!((-90.0..=90.0).contains(&vertex[1].as_f64().unwrap()));
                }
            }
        }
    }

    #[test]
    fn river_lengths_follow_geodesic_courses_to_actual_outlets_and_confluences() {
        let mut map = Map::generate(5, MapSize::Medium);
        let radius = f64::from(map.radius_km);
        let original_positions = std::array::from_fn::<_, 4, _>(|id| map.regions[id].position);
        // Canonical equatorial centers at 179°, -179°, -178° and 0°. Their
        // three-degree route crosses the seam and must not visit the sea center.
        map.regions[0].position = [-0.9998476951563913, 0.01745240643728351, 0.0];
        map.regions[1].position = [-0.9998476951563913, -0.01745240643728351, 0.0];
        map.regions[2].position = [-0.9993908270190958, -0.03489949670250097, 0.0];
        map.regions[3].position = [1.0, 0.0, 0.0];
        map.drainage[1] = Some(2);
        let tributary = River {
            course: vec![0, 1],
            mouth: 3,
            catchment: vec![0, 1],
            joins: Some(0),
        };
        let view = river_view(&map, 1, &tributary).unwrap();
        assert_eq!(view.join_at, Some(2));
        assert!((view.length_km - radius * 3.0 * std::f64::consts::PI / 180.0).abs() < 1e-7);
        // Source chart positions are neither involved in this length nor changed
        // when an author switches projections.
        map.regions[0].site = [0.0, 0.0];
        map.regions[1].site = [1_000_000.0, 1_000_000.0];
        assert_eq!(
            river_view(&map, 1, &tributary).unwrap().length_km,
            view.length_km
        );
        map.radius_km *= 2.0;
        assert_eq!(
            river_view(&map, 1, &tributary).unwrap().length_km,
            view.length_km * 2.0
        );
        map.radius_km /= 2.0;
        for (region, position) in map.regions.iter_mut().zip(original_positions) {
            region.position = position;
        }
        let coast = map
            .regions
            .iter()
            .enumerate()
            .find_map(|(id, region)| {
                region
                    .terrain
                    .is_land()
                    .then(|| {
                        region
                            .neighbours
                            .iter()
                            .copied()
                            .find(|&other| !map.regions[other].terrain.is_land())
                            .map(|mouth| (id, mouth))
                    })
                    .flatten()
            })
            .unwrap();
        let river = River {
            course: vec![coast.0],
            mouth: coast.1,
            catchment: vec![coast.0],
            joins: None,
        };
        let outlet = map.shared_midpoint(coast.0, coast.1).unwrap();
        let length = river_view(&map, 0, &river).unwrap().length_km;
        assert!(
            (length / radius - angular_distance(map.regions[coast.0].position, outlet)).abs()
                < 1e-12
        );
        map.regions[coast.1].position = map.regions[coast.0].position;
        assert_eq!(river_view(&map, 0, &river).unwrap().length_km, length);
    }

    #[test]
    fn continent_names_and_memberships_belong_to_the_requested_generation() {
        // The fixture needs two continents; not every world has them.
        let (mut w, map, continents) = (0..64)
            .find_map(|seed| {
                let w = Bench::new(seed, "large").unwrap();
                let map: serde_json::Value = serde_json::from_str(&w.map().unwrap()).unwrap();
                let continents: Vec<serde_json::Value> = map["landmasses"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|landmass| landmass["kind"] == "continent")
                    .cloned()
                    .collect();
                (continents.len() >= 2).then_some((w, map, continents))
            })
            .expect("a large world with two continents");
        let mut action: serde_json::Value =
            serde_json::from_str(&found("Hill", "familiar")).unwrap();
        action["region"] = continents[0]["anchor"].clone();
        w.act(&action.to_string()).unwrap();
        let founded: serde_json::Value = serde_json::from_str(&w.overview(0).unwrap()).unwrap();
        let first = &founded["continents"][0];
        assert_eq!(first["landmass"], continents[0]["id"]);
        assert_eq!(first["peoples"], serde_json::json!([0]));
        assert_eq!(first["name"]["people"], 0);
        assert_eq!(first["name"]["variety"], 0);
        assert_eq!(first["name"]["since"], 0);
        assert_eq!(first["name"]["witness"], action["region"]);
        assert!(
            continents[0]["regions"]
                .as_array()
                .unwrap()
                .contains(&first["name"]["witness"])
        );
        assert!(founded["continents"][1]["name"].is_null());

        w.act(r#"{"kind":"run","generations":1}"#).unwrap();
        action["region"] = continents[1]["anchor"].clone();
        w.act(&action.to_string()).unwrap();
        w.act(r#"{"kind":"religion","community":1}"#).unwrap();
        let later: serde_json::Value = serde_json::from_str(&w.overview(1).unwrap()).unwrap();
        assert_eq!(later["continents"][0]["name"], first["name"]);
        assert_eq!(later["continents"][1]["name"]["people"], 1);
        assert_eq!(later["continents"][1]["name"]["since"], 1);
        assert!(
            later["continents"][1]["peoples"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(1))
        );
        let shrine = &later["religions"][0]["shrine"];
        let shrine_landmass =
            &map["regions"][shrine["region"].as_u64().unwrap() as usize]["landmass"];
        let sacred_continent = later["continents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|continent| continent["landmass"] == *shrine_landmass)
            .unwrap();
        assert_eq!(sacred_continent["religions"], serde_json::json!([0]));

        let early: serde_json::Value = serde_json::from_str(&w.overview(0).unwrap()).unwrap();
        assert_eq!(early["continents"], founded["continents"]);
    }

    #[test]
    fn known_lands_keep_the_languages_names_after_it_falls_silent() {
        let mut w = Bench::new(5, "small").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        w.act(&found("Coast", "polynesian")).unwrap();
        let before: serde_json::Value = serde_json::from_str(&w.overview(0).unwrap()).unwrap();
        let known = before["varieties"][0]["knownLands"].as_array().unwrap();
        let held = before["communities"][0]["lands"].as_array().unwrap();
        assert!(
            held.iter()
                .all(|region| known.iter().any(|land| land["region"] == *region))
        );
        assert!(
            known
                .windows(2)
                .all(|pair| pair[0]["region"].as_u64() < pair[1]["region"].as_u64())
        );
        let home = &before["communities"][0]["region"];
        let local = before["places"]
            .as_array()
            .unwrap()
            .iter()
            .find(|place| place["region"] == *home)
            .unwrap()["names"]
            .as_array()
            .unwrap()
            .last()
            .unwrap();
        let remembered = known.iter().find(|land| land["region"] == *home).unwrap();
        assert_eq!(remembered["spelled"], local["spelled"]);
        assert_eq!(remembered["ipa"], local["ipa"]);

        w.act(r#"{"kind":"shift","community":0,"toward":1}"#)
            .unwrap();
        let after: serde_json::Value = serde_json::from_str(&w.overview(0).unwrap()).unwrap();
        assert_eq!(after["varieties"][0]["spoken"], false);
        assert_eq!(
            after["varieties"][0]["knownLands"],
            before["varieties"][0]["knownLands"]
        );
    }

    #[test]
    fn shrines_retain_their_sacred_names_when_followers_change_speech() {
        let mut w = Bench::new(5, "small").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        w.act(r#"{"kind":"religion","community":0}"#).unwrap();
        let before: serde_json::Value = serde_json::from_str(&w.overview(0).unwrap()).unwrap();
        let shrine = &before["religions"][0]["shrine"];
        let sacred = before["religions"][0]["sacred"].as_u64().unwrap() as usize;
        let known = before["varieties"][sacred]["knownLands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|land| land["region"] == shrine["region"])
            .unwrap();
        assert_eq!(shrine["name"]["name"], known["spelled"]);
        assert_eq!(shrine["name"]["ipa"], known["ipa"]);
        w.act(&found("Coast", "polynesian")).unwrap();
        w.act(r#"{"kind":"shift","community":0,"toward":1}"#)
            .unwrap();
        w.act(r#"{"kind":"run","generations":30}"#).unwrap();
        let after: serde_json::Value = serde_json::from_str(&w.overview(30).unwrap()).unwrap();
        assert_eq!(after["religions"][0]["shrine"], *shrine);
    }

    #[test]
    fn the_chronicle_tells_sound_changes_through_words() {
        let mut w = bench();
        let overview: serde_json::Value =
            serde_json::from_str(&w.overview(w.latest()).unwrap()).unwrap();
        let annals = overview["annals"].as_array().unwrap();
        let laws: Vec<_> = annals.iter().filter(|a| a["kind"] == "law").collect();
        assert!(!laws.is_empty());
        for law in &laws {
            // One entry per language and year, with the laws as notes.
            assert!(!law["notes"].as_array().unwrap().is_empty());
        }
        assert!(
            laws.iter()
                .any(|a| a["text"].as_str().unwrap().contains('*')),
            "changes are shown through a word"
        );
        let years = |v: u64| {
            laws.iter()
                .filter(|a| a["variety"] == v)
                .map(|a| a["generation"].as_u64().unwrap())
                .collect::<Vec<_>>()
        };
        let mut unique = years(0);
        unique.dedup();
        assert_eq!(unique, years(0));
        // The same history is always told in the same words.
        let again: serde_json::Value = serde_json::from_str(
            &Bench::load(&w.save().unwrap())
                .unwrap()
                .overview(22)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(again["annals"], overview["annals"]);

        let captured: Vec<_> = laws
            .iter()
            .filter(|a| a["variety"] == 0)
            .map(|a| a["id"].clone())
            .collect();
        assert!(!captured.is_empty());
        w.act(r#"{"kind":"shift","community":0,"toward":1}"#)
            .unwrap();
        let shifted: serde_json::Value =
            serde_json::from_str(&w.overview(w.latest()).unwrap()).unwrap();
        for id in captured {
            let event = shifted["annals"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["id"] == id)
                .unwrap();
            assert!(
                event["peoples"]
                    .as_array()
                    .unwrap()
                    .contains(&serde_json::json!(0)),
                "a language shift cannot remove a people's earlier sound changes"
            );
        }
    }

    #[test]
    fn notebook_keeps_exact_readings_and_unavailable_references_without_changing_history() {
        use notebook::{Destination, NoteKind};
        let mut w = Bench::new(7, "small").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        let reading = ReadingRef {
            telling: 0,
            point: w.chronicle.end(),
        };
        let note = Note {
            id: "a-discovery".into(),
            title: "Before the arrival".into(),
            body: "Will these words survive?".into(),
            kind: NoteKind::Question,
            target: Some(Destination {
                reading,
                subject: Subject::Event {
                    id: "world:0".into(),
                },
            }),
            label: "The first founding".into(),
            generation: 0,
            revision: ENGINE_REVISION,
            archived: false,
        };
        let recipe = w.chronicle.recipe();
        let mutation = w.mutation;
        w.save_note(&to_json(&note).unwrap()).unwrap();
        assert_eq!(w.chronicle.recipe(), recipe);
        assert_eq!(
            w.mutation, mutation,
            "annotation edits do not invalidate a settlement preview"
        );
        w.act(&found("Coast", "polynesian")).unwrap();
        let target: Option<Destination> =
            serde_json::from_str(&w.resolve_note(&note.id).unwrap()).unwrap();
        let mut exact = w
            .read(0, &to_json(&target.unwrap().reading.point).unwrap())
            .unwrap();
        assert_eq!(
            exact.world(0).communities.len(),
            1,
            "the bookmark precedes the same-year second founding"
        );
        w.act(r#"{"kind":"shift","community":0,"toward":1}"#)
            .unwrap();
        let speech = w.chronicle.latest().communities[0].variety;
        w.act(r#"{"kind":"religion","community":0}"#).unwrap();
        let events = annals(w.chronicle.latest());
        let shift = events.iter().find(|a| a.kind == "shift").unwrap();
        assert!(shift.languages.contains(&0) && shift.languages.contains(&speech));
        let faith = events.iter().find(|a| a.kind == "faith").unwrap();
        assert!(faith.languages.contains(&speech));
        assert!(
            !faith.languages.contains(&0),
            "a later same-year event uses the new speech"
        );
        w.chronicle
            .act_at(reading.point, Action::Run { generations: 1 })
            .unwrap();
        let mut saved: Document = serde_json::from_str(&w.save().unwrap()).unwrap();
        saved.recipe.tellings[0].actions.push(Action::Shift {
            community: 999,
            toward: 0,
        });
        let mut recovered = Bench::load(&to_json(&saved).unwrap()).unwrap();
        assert!(recovered.resolve_note(&note.id).is_err());
        let mut edited = note.clone();
        edited.body.push_str(" The original reading needs repair.");
        recovered.save_note(&to_json(&edited).unwrap()).unwrap();
        let back = Bench::load(&recovered.save().unwrap()).unwrap();
        assert_eq!(back.notebook, vec![edited]);
        let mut old = note;
        old.revision -= 1;
        assert!(
            w.validate_note(&old)
                .unwrap_err()
                .contains("different engine revision")
        );
    }

    #[test]
    fn sound_change_entries_tell_the_specimen_words_they_reached() {
        use std::collections::BTreeMap;
        let mut w = bench();
        w.act(r#"{"kind":"run","generations":60}"#).unwrap();
        let overview: serde_json::Value =
            serde_json::from_str(&w.overview(w.latest()).unwrap()).unwrap();
        let words = |specimen: &serde_json::Value| -> BTreeMap<String, (String, Option<String>)> {
            specimen
                .as_array()
                .unwrap()
                .iter()
                .map(|s| {
                    (
                        s["concept"].as_str().unwrap().to_string(),
                        (
                            s["spelled"].as_str().unwrap().to_string(),
                            s["was"].as_str().map(str::to_string),
                        ),
                    )
                })
                .collect()
        };
        let entries: Vec<_> = overview["annals"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["kind"] == "law" && a["variety"] == 0)
            .map(|a| words(&a["specimen"]))
            .collect();
        assert!(entries.len() > 3);
        assert!(
            entries
                .iter()
                .flat_map(|e| e.values())
                .any(|(_, was)| was.is_some()),
            "some change reaches a specimen word"
        );
        // Each entry picks up where the language's previous one left off,
        // and the last leaves the words as the language says them now.
        let now = words(&overview["varieties"][0]["specimen"]);
        let mut left: BTreeMap<String, String> = BTreeMap::new();
        for entry in &entries {
            for (concept, (spelled, was)) in entry {
                if let Some(before) = left.get(concept) {
                    assert_eq!(was.as_ref().unwrap_or(spelled), before, "{concept}");
                }
                left.insert(concept.clone(), spelled.clone());
            }
        }
        for (concept, (spelled, _)) in &now {
            // A word taken up after the last change has no entry yet.
            if let Some(last) = left.get(concept) {
                assert_eq!(last, spelled, "{concept}");
            }
        }
    }

    #[test]
    fn the_chronicle_tells_contacts_beginning_and_ending() {
        let mut w = Bench::new(5, "medium").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        let home = w.chronicle.latest().communities[0].home();
        w.act(&found_at("Coast", "polynesian", home)).unwrap();
        w.act(r#"{"kind":"connect","a":0,"b":1,"intensity":0.6,"contact":"trade"}"#)
            .unwrap();
        w.act(r#"{"kind":"run","generations":80}"#).unwrap();
        let overview: serde_json::Value =
            serde_json::from_str(&w.overview(w.latest()).unwrap()).unwrap();
        let annals = overview["annals"].as_array().unwrap();
        let first = |kind: &str| {
            annals
                .iter()
                .find(|a| a["kind"] == kind)
                .map(|a| a["text"].as_str().unwrap().to_string())
        };
        let met = first("contact").expect("the trade is told");
        assert!(!met.contains('{'), "{met}");
        let parted = first("parted").expect("the trade ends within 2000 years");
        assert!(!parted.contains('{'), "{parted}");
    }

    #[test]
    fn comparison_resolves_each_telling_and_never_pairs_post_divergence_ids() {
        let mut w = Bench::new(5, "small").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        let before = w.chronicle.end();
        w.act(&found("Coast", "polynesian")).unwrap();
        let original = w.chronicle.active();
        let reading = serde_json::to_string(&ReadingRef {
            telling: original,
            point: before,
        })
        .unwrap();
        w.act_at(&reading, w.mutation, &found("Different", "iranian"))
            .unwrap();
        let alternative = w.chronicle.active();
        w.rename(alternative, "The inland account").unwrap();
        let saved = w.save().unwrap();
        let comparison: serde_json::Value =
            serde_json::from_str(&w.compare(original, alternative, 0).unwrap()).unwrap();
        assert_eq!(comparison["sharedPeoples"], serde_json::json!([0]));
        assert_eq!(
            comparison["left"]["communities"][1]["id"],
            comparison["right"]["communities"][1]["id"]
        );
        assert_ne!(
            comparison["left"]["communities"][1]["name"],
            comparison["right"]["communities"][1]["name"]
        );
        assert_eq!(w.save().unwrap(), saved, "comparison is read-only");
        assert!(
            w.compare(original, alternative, 1).is_err(),
            "comparison cannot invent future years"
        );
        let mut exact = w
            .read(original, &serde_json::to_string(&before).unwrap())
            .unwrap();
        let shown: serde_json::Value = serde_json::from_str(&exact.overview(0).unwrap()).unwrap();
        assert_eq!(shown["communities"].as_array().unwrap().len(), 1);
        assert!(
            exact.lexicon(0, 1).is_err(),
            "word views must share the exact reading"
        );
        w.restore(original).unwrap();
        assert_eq!(w.chronicle.tellings().len(), 2);
        w.restore(alternative).unwrap();
        assert_eq!(w.save().unwrap(), saved);
        assert_eq!(Bench::load(&saved).unwrap().save().unwrap(), saved);
        // An alternate action can stop replaying without disappearing from
        // the document, its metadata, or the downloadable recipe.
        let mut recipe = w.chronicle.recipe();
        recipe
            .tellings
            .iter_mut()
            .find(|t| t.id == original)
            .unwrap()
            .actions
            .push(Action::Shift {
                community: 99,
                toward: 0,
            });
        let damaged = Bench::load(&serde_json::to_string(&recipe).unwrap()).unwrap();
        assert_eq!(damaged.tellings().len(), 2);
        assert!(damaged.read(original, "").is_err());
        assert!(damaged.save().unwrap().contains("99"));
    }

    #[test]
    fn founding_sites_use_land_effort_skip_occupied_and_leave_history_untouched() {
        let mut bench = Bench::with_geography(5, "medium", "spherical-v1").unwrap();
        let map = &bench.chronicle.latest().map;
        let anchor = map
            .regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.terrain.is_land())
            .max_by_key(|&(r, _)| map.walking_row(r, 800.0).len())
            .unwrap()
            .0;
        let sites: Vec<usize> =
            serde_json::from_str(&bench.founding_sites(anchor, 12).unwrap()).unwrap();
        let world = bench.chronicle.latest();
        let mut expected: Vec<_> = world
            .map
            .walking_row(anchor, world.params.settle_apart)
            .iter()
            .copied()
            .collect();
        expected.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        assert_eq!(
            sites,
            expected
                .iter()
                .take(12)
                .map(|&(r, _)| r as usize)
                .collect::<Vec<_>>()
        );
        assert_eq!(sites[0], anchor);
        assert!(sites.len() > 2);
        assert_eq!(
            sites.iter().copied().collect::<HashSet<_>>().len(),
            sites.len()
        );
        assert!(sites.iter().all(|&r| {
            world.map.regions[r].terrain.is_land()
                && world.map.regions[r].landmass == world.map.regions[anchor].landmass
                && world.map.distance(anchor, r).is_finite()
        }));
        bench.act(&found_at("Source", "familiar", anchor)).unwrap();
        bench
            .act(&found_at("Neighbour", "indic", sites[1]))
            .unwrap();
        let saved = bench.save().unwrap();
        let view = bench.overview(0).unwrap();
        let available: Vec<usize> =
            serde_json::from_str(&bench.founding_sites(anchor, 12).unwrap()).unwrap();
        assert!(!available.contains(&anchor));
        assert!(!available.contains(&sites[1]));
        assert_eq!(available[0], sites[2]);
        assert_eq!(
            bench.founding_sites(anchor, 1).unwrap(),
            to_json(&available[..1]).unwrap()
        );
        let sea = bench
            .chronicle
            .latest()
            .map
            .regions
            .iter()
            .position(|r| !r.terrain.is_land())
            .unwrap();
        for (region, count) in [(sea, 3), (usize::MAX, 3), (anchor, 0), (anchor, 13)] {
            assert!(bench.founding_sites(region, count).is_err());
        }
        assert_eq!(bench.save().unwrap(), saved);
        assert_eq!(bench.overview(0).unwrap(), view);
        let mut unqueried = Bench::load(&saved).unwrap();
        bench.act(r#"{"kind":"run","generations":4}"#).unwrap();
        unqueried.act(r#"{"kind":"run","generations":4}"#).unwrap();
        assert_eq!(
            bench.chronicle.latest().communities,
            unqueried.chronicle.latest().communities
        );
        let mut queried: serde_json::Value =
            serde_json::from_str(&bench.overview(4).unwrap()).unwrap();
        let mut untouched: serde_json::Value =
            serde_json::from_str(&unqueried.overview(4).unwrap()).unwrap();
        queried.as_object_mut().unwrap().remove("mutation");
        untouched.as_object_mut().unwrap().remove("mutation");
        assert_eq!(queried, untouched);
        for variety in 0..bench.chronicle.latest().varieties.len() {
            assert_eq!(
                bench.lexicon(4, variety).unwrap(),
                unqueried.lexicon(4, variety).unwrap()
            );
        }
        assert_eq!(
            bench.chronicle.latest().events,
            unqueried.chronicle.latest().events
        );
        assert!(bench.founding_sites(anchor, 3).is_err());
    }

    #[test]
    fn founding_sites_return_short_or_empty_lists_without_water_jumps() {
        let mut bench = Bench::with_geography(5, "medium", "spherical-v1").unwrap();
        let map = &bench.chronicle.latest().map;
        let anchor = map
            .regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.terrain.is_land())
            .min_by_key(|&(r, _)| map.walking_row(r, 800.0).len())
            .unwrap()
            .0;
        let reachable: Vec<usize> =
            serde_json::from_str(&bench.founding_sites(anchor, 12).unwrap()).unwrap();
        assert!(reachable.len() < 12, "fixture has limited nearby land");
        for region in reachable {
            bench
                .act(&found_at("Occupied", "familiar", region))
                .unwrap();
        }
        assert_eq!(bench.founding_sites(anchor, 12).unwrap(), "[]");
    }

    #[test]
    fn related_founding_views_and_export_preserve_actual_family_links() {
        let mut bench = Bench::new(5, "medium").unwrap();
        let map = &bench.chronicle.latest().map;
        let anchor = map
            .regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.terrain.is_land())
            .max_by_key(|&(r, _)| map.walking_row(r, 800.0).len())
            .unwrap()
            .0;
        let sites: Vec<usize> =
            serde_json::from_str(&bench.founding_sites(anchor, 3).unwrap()).unwrap();
        bench.act(&found_at("Source", "familiar", anchor)).unwrap();
        let source = bench.chronicle.latest().communities[0].clone();
        bench
            .act(
                &serde_json::json!({
                    "kind": "found-related", "source": 0, "region": sites[1],
                    "naming": {"kind": "people"}, "livelihood": "farming",
                    "ethos": {"open": 0.4}
                })
                .to_string(),
            )
            .unwrap();
        let overview: serde_json::Value =
            serde_json::from_str(&bench.overview(0).unwrap()).unwrap();
        assert_eq!(
            overview["communities"][1]["parents"],
            serde_json::json!([0])
        );
        assert_eq!(
            overview["communities"][1]["size"],
            overview["communities"][0]["size"]
        );
        assert_eq!(overview["varieties"][1]["parent"], 0);
        assert_eq!(overview["varieties"][1]["forkedAt"], 0);
        assert_eq!(
            overview["varieties"][1]["family"],
            overview["varieties"][0]["family"]
        );
        assert_eq!(
            overview["varieties"][1]["specimen"],
            overview["varieties"][0]["specimen"]
        );
        assert_eq!(bench.chronicle.latest().communities[0], source);
        let decisions: serde_json::Value =
            serde_json::from_str(&bench.decisions().unwrap()).unwrap();
        assert_eq!(decisions[1]["kind"], "found-related");
        assert_eq!(decisions[1]["people"], serde_json::json!([0, 1]));
        assert_eq!(decisions[1]["variety"], 1);
        let saved = bench.save().unwrap();
        let recipe: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(recipe["geography"], "continental-v3");
        assert_eq!(recipe["tellings"][0]["actions"][1]["kind"], "found-related");
        let mut restored = Bench::load(&saved).unwrap();
        assert_eq!(restored.map().unwrap(), bench.map().unwrap());
        assert_eq!(restored.decisions().unwrap(), bench.decisions().unwrap());
        assert_eq!(
            restored.chronicle.latest().communities,
            bench.chronicle.latest().communities
        );
        for variety in 0..2 {
            assert_eq!(
                restored.lexicon(0, variety).unwrap(),
                bench.lexicon(0, variety).unwrap()
            );
        }
        assert_eq!(restored.save().unwrap(), saved);
    }

    #[test]
    fn revision_33_implicit_geography_survives_save_replay_and_branch() {
        let mut original = Bench::with_geography(5, "small", "spherical-v1").unwrap();
        let home = original.chronicle.latest().map.landmasses[0].anchor;
        original.act(&found_at("Old", "familiar", home)).unwrap();
        let first_people = original.chronicle.latest().communities.clone();
        let first_speech = original.lexicon(0, 0).unwrap();
        original.act(r#"{"kind":"run","generations":2}"#).unwrap();
        let old_map = original.map().unwrap();
        let mut legacy: serde_json::Value =
            serde_json::from_str(&original.save().unwrap()).unwrap();
        legacy["revision"] = SPHERICAL_GEOGRAPHY_REVISION.into();
        legacy.as_object_mut().unwrap().remove("geography");
        let mut loaded = Bench::load(&legacy.to_string()).unwrap();
        assert_eq!(loaded.map().unwrap(), old_map);
        assert_eq!(
            loaded.chronicle.latest().communities,
            original.chronicle.latest().communities
        );
        assert_eq!(
            loaded.lexicon(2, 0).unwrap(),
            original.lexicon(2, 0).unwrap()
        );
        let saved = loaded.save().unwrap();
        let exported: serde_json::Value = serde_json::from_str(&saved).unwrap();
        assert_eq!(exported["geography"], "spherical-v1");
        assert_eq!(Bench::load(&saved).unwrap().map().unwrap(), old_map);
        loaded.branch(0);
        assert_eq!(loaded.map().unwrap(), old_map);
        assert_eq!(loaded.chronicle.latest().communities, first_people);
        assert_eq!(loaded.lexicon(0, 0).unwrap(), first_speech);
        let fresh: serde_json::Value =
            serde_json::from_str(&Bench::new(5, "small").unwrap().map().unwrap()).unwrap();
        assert_eq!(fresh["geography"], "continental-v3");
        assert_ne!(
            fresh["regions"],
            serde_json::from_str::<serde_json::Value>(&old_map).unwrap()["regions"]
        );
    }

    #[test]
    fn founding_preview_is_pure_and_neighbours_form_real_contacts() {
        use umran_sim::world::Params;
        let mut w = Bench::new(5, "medium").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        let alone: serde_json::Value =
            serde_json::from_str(&w.founding_preview().unwrap()).unwrap();
        assert!(alone["peoples"][0]["nearestOther"].is_null());
        let home = w.chronicle.latest().communities[0].home();
        let adjacent = w.chronicle.latest().map.regions[home]
            .neighbours
            .iter()
            .copied()
            .find(|&r| w.chronicle.latest().map.regions[r].terrain.is_land())
            .unwrap();
        w.act(&found_at("Coast", "polynesian", adjacent)).unwrap();
        let distant = w
            .chronicle
            .latest()
            .map
            .regions
            .iter()
            .enumerate()
            .find(|&(r, land)| {
                land.terrain.is_land()
                    && w.chronicle.latest().map.closeness(home, r) == 0.0
                    && w.chronicle.latest().map.closeness(adjacent, r) == 0.0
            })
            .unwrap()
            .0;
        w.act(&found_at("Far", "indic", distant)).unwrap();
        w.act(&found_at("Same", "semitic", home)).unwrap();
        let save = w.save().unwrap();
        let view = w.overview(0).unwrap();
        let preview = w.founding_preview().unwrap();
        assert_eq!(w.founding_preview().unwrap(), preview);
        assert_eq!(w.save().unwrap(), save);
        assert_eq!(w.overview(0).unwrap(), view);
        let preview: serde_json::Value = serde_json::from_str(&preview).unwrap();
        assert_eq!(preview["peoples"][0]["nearestOther"], 3);
        assert_eq!(preview["peoples"][3]["nearestOther"], 0);
        let mut world = w.chronicle.latest().clone();
        world.params = Params::static_society();
        world.params.neighbour_rate = 1000.0;
        world.step();
        for pair in preview["pairs"].as_array().unwrap() {
            let a = pair["a"].as_u64().unwrap() as usize;
            let b = pair["b"].as_u64().unwrap() as usize;
            assert_eq!(
                pair["reach"] == "neighbours",
                world.contacts.iter().any(|c| c.a == a && c.b == b),
                "first neighbour encounters agree with the preview"
            );
        }
        let mut unqueried = Bench::load(&save).unwrap();
        w.act(r#"{"kind":"run","generations":4}"#).unwrap();
        unqueried.act(r#"{"kind":"run","generations":4}"#).unwrap();
        let mut queried: serde_json::Value = serde_json::from_str(&w.overview(4).unwrap()).unwrap();
        let mut untouched: serde_json::Value =
            serde_json::from_str(&unqueried.overview(4).unwrap()).unwrap();
        queried.as_object_mut().unwrap().remove("mutation");
        untouched.as_object_mut().unwrap().remove("mutation");
        assert_eq!(queried, untouched);
        assert!(w.founding_preview().is_err());
    }

    #[test]
    fn founding_preview_keeps_route_evidence_and_merchant_reach() {
        let mut w = Bench::new(5, "medium").unwrap();
        let world = w.chronicle.latest();
        let map = &world.map;
        let reach = world.params.trade_reach;
        let (coast, across) = map
            .regions
            .iter()
            .enumerate()
            .filter(|&(r, land)| {
                map.coastal(r)
                    && land
                        .landmass
                        .is_some_and(|mass| map.landmasses[mass].kind == LandmassKind::Continent)
            })
            .find_map(|(source, land)| {
                map.voyage_row(source, reach)
                    .iter()
                    .find(|&&(r, _)| map.regions[r as usize].landmass != land.landmass)
                    .map(|&(r, _)| (source, r as usize))
            })
            .unwrap();
        let walking = map
            .walking_row(coast, reach)
            .iter()
            .find(|&&(r, _)| map.closeness(coast, r as usize) == 0.0)
            .unwrap()
            .0 as usize;
        let far = map
            .regions
            .iter()
            .enumerate()
            .find(|&(r, land)| {
                land.terrain.is_land()
                    && land.landmass == map.regions[coast].landmass
                    && map.distance(coast, r) > reach
                    && map.voyage(coast, r) > reach
            })
            .unwrap()
            .0;
        let far_walk = map.distance(coast, far);
        let sea_effort = map.voyage(coast, across);
        for (i, region) in [coast, walking, across, far].into_iter().enumerate() {
            w.act(&found_at(&format!("Founder{i}"), "familiar", region))
                .unwrap();
        }
        let preview: serde_json::Value =
            serde_json::from_str(&w.founding_preview().unwrap()).unwrap();
        let pairs = preview["pairs"].as_array().unwrap();
        let walk = pairs.iter().find(|p| p["a"] == 0 && p["b"] == 1).unwrap();
        let sea = pairs.iter().find(|p| p["a"] == 0 && p["b"] == 2).unwrap();
        let apart = pairs.iter().find(|p| p["a"] == 0 && p["b"] == 3).unwrap();
        assert_eq!(walk["reach"], "walking");
        assert_eq!(sea["reach"], "sea");
        assert!(sea["walk"].is_null());
        assert_eq!(sea["voyage"].as_f64().unwrap() as f32, sea_effort);
        assert_eq!(sea["sameLandmass"], false);
        assert_eq!(apart["reach"], "apart");
        assert_eq!(apart["walk"].as_f64().unwrap() as f32, far_walk);
        assert_eq!(apart["sameLandmass"], true);
        assert_eq!(preview["peoples"][0]["coastal"], true);
    }

    #[test]
    fn founding_matches_its_preview() {
        let design = Bench::design("indic", 2).unwrap();
        let preview: serde_json::Value =
            serde_json::from_str(&Bench::preview(&design, 99, RIVER).unwrap()).unwrap();
        let mut w = Bench::new(1, "medium").unwrap();
        let action = format!(
            r#"{{"kind":"found","naming":{{"kind":"place","place":"river"}},"design":{design},"seed":99,"power":0.5,"openness":0.5}}"#
        );
        w.act(&action).unwrap();
        let word: serde_json::Value =
            serde_json::from_str(&w.word(0, 0, "water").unwrap()).unwrap();
        let shown = preview["words"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["gloss"] == "water")
            .unwrap();
        assert_eq!(word["variants"][0]["ipa"], shown["ipa"]);
        let overview: serde_json::Value = serde_json::from_str(&w.overview(0).unwrap()).unwrap();
        assert_eq!(
            overview["communities"][0]["name"],
            preview["people"]["name"]
        );
        assert_eq!(
            overview["communities"][0]["meaning"],
            "the people of the river"
        );
        assert_eq!(
            overview["varieties"][0]["name"],
            preview["language"]["name"]
        );
    }

    #[test]
    fn designs_preview_without_founding() {
        let design = Bench::design("semitic", 3).unwrap();
        let preview: serde_json::Value =
            serde_json::from_str(&Bench::preview(&design, 3, PEOPLE).unwrap()).unwrap();
        assert!(preview["words"].as_array().unwrap().len() > 20);
        assert!(!preview["families"].as_array().unwrap().is_empty());
        let drawn: serde_json::Value =
            serde_json::from_str(&Bench::frequency_design(4, 15, 5).unwrap()).unwrap();
        assert_eq!(drawn["sounds"].as_array().unwrap().len(), 20);
        assert!(Bench::preview("{}", 1, PEOPLE).is_err());
        assert!(Bench::preview(&design, 1, r#"{"kind":"place","place":"moon"}"#).is_err());
        let catalog: serde_json::Value = serde_json::from_str(&Bench::catalog().unwrap()).unwrap();
        assert!(catalog["sounds"].as_array().unwrap().len() > 70);
        assert!(catalog["presets"].as_array().unwrap().len() > 5);
    }

    #[test]
    fn views_serialize_at_any_generation() {
        let mut w = bench();
        let latest: serde_json::Value = serde_json::from_str(&w.overview(22).unwrap()).unwrap();
        assert_eq!(latest["generation"], 22);
        assert_eq!(latest["communities"].as_array().unwrap().len(), 3);
        let early: serde_json::Value = serde_json::from_str(&w.overview(5).unwrap()).unwrap();
        assert_eq!(early["communities"].as_array().unwrap().len(), 2);
        let rows: serde_json::Value = serde_json::from_str(&w.lexicon(22, 2).unwrap()).unwrap();
        let meanings = latest["varieties"][2]["ownWords"]["meanings"]
            .as_u64()
            .unwrap();
        assert_eq!(rows.as_array().unwrap().len() as u64, meanings);
        assert!(
            meanings < CONCEPTS.len() as u64,
            "meanings that wait for ideas have no word"
        );
        let word: serde_json::Value =
            serde_json::from_str(&w.word(22, 2, "water").unwrap()).unwrap();
        assert!(!word["variants"].as_array().unwrap().is_empty());
        assert!(
            !word["cognates"].as_array().unwrap().is_empty(),
            "Upland's water is cognate with Hill's"
        );
    }

    #[test]
    fn physically_refused_contacts_leave_the_book_and_world_unchanged() {
        let mut w = Bench::new(5, "medium").unwrap();
        let landmasses = &w.chronicle.latest().map.landmasses;
        let a = landmasses[0].anchor;
        let b = landmasses[1].anchor;
        w.act(&found_at("Hill", "familiar", a)).unwrap();
        w.act(&found_at("Coast", "polynesian", b)).unwrap();
        let saved = w.save().unwrap();
        let before = w.overview(0).unwrap();
        for contact in ["neighbours", "trade", "intermarriage", "religion", "rule"] {
            let action = serde_json::json!({
                "kind": "connect", "a": 0, "b": 1, "intensity": 0.8, "contact": contact
            });
            assert!(w.act(&action.to_string()).is_err(), "{contact}");
            assert_eq!(w.save().unwrap(), saved, "{contact} changed the recipe");
            assert_eq!(
                w.overview(0).unwrap(),
                before,
                "{contact} changed the world"
            );
        }
        // The refusal does not prevent a later physically valid action.
        settle(&mut w, 0.0);
        w.act(r#"{"kind":"connect","a":0,"b":2,"intensity":0.6,"contact":"trade"}"#)
            .unwrap();
        assert_eq!(
            w.chronicle.latest().contacts[0].kind,
            umran_sim::ContactKind::Trade
        );
    }

    #[test]
    fn vast_recipes_replay_and_branch_on_the_same_physical_map() {
        let view = |bench: &mut Bench, generation| {
            let mut view: serde_json::Value =
                serde_json::from_str(&bench.overview(generation).unwrap()).unwrap();
            // Preview tokens belong to this open session, not to the saved world.
            view.as_object_mut().unwrap().remove("mutation");
            view
        };
        let mut w = Bench::new(7, "vast").unwrap();
        let map = w.map().unwrap();
        let home = w.chronicle.latest().map.landmasses[0].anchor;
        w.act(&found_at("Hill", "familiar", home)).unwrap();
        w.act(r#"{"kind":"run","generations":2}"#).unwrap();
        let mut loaded = Bench::load(&w.save().unwrap()).unwrap();
        assert_eq!(loaded.map().unwrap(), map);
        assert_eq!(view(&mut loaded, 2), view(&mut w, 2));
        loaded.branch(1);
        loaded
            .act(r#"{"kind":"craft","community":0,"craft":"writing"}"#)
            .unwrap();
        loaded.act(r#"{"kind":"run","generations":2}"#).unwrap();
        let recipe: umran_sim::Recipe = serde_json::from_str(&loaded.save().unwrap()).unwrap();
        assert_eq!(recipe.map, MapSize::Vast);
        assert_eq!(loaded.latest(), 3);
        let mut replayed = Bench::load(&loaded.save().unwrap()).unwrap();
        assert_eq!(replayed.map().unwrap(), map);
        assert_eq!(view(&mut replayed, 3), view(&mut loaded, 3));
    }

    #[test]
    fn saves_round_trip_and_bad_input_is_rejected() {
        let w = bench();
        let mut loaded = Bench::load(&w.save().unwrap()).unwrap();
        assert_eq!(loaded.latest(), 22);
        assert!(
            loaded
                .overview(22)
                .unwrap()
                .contains(r#""savedRevision":null"#)
        );
        assert!(Bench::load("{}").is_err());
        let mut w = bench();
        assert!(
            w.act(r#"{"kind":"settle","community":999,"intent":"migration","destination":0,"share":1,"naming":null,"intensity":0}"#)
                .is_err()
        );
        assert!(w.act("not json").is_err());
    }

    #[test]
    fn flat_geography_recipes_are_rejected_without_reinterpreting_region_actions() {
        let mut bench = Bench::with_geography(5, "small", "spherical-v1").unwrap();
        let home = bench.chronicle.latest().map.landmasses[0].anchor;
        bench.act(&found_at("Hill", "familiar", home)).unwrap();
        bench.set_title("Kept original");
        let saved = bench.save().unwrap();
        let mut original: serde_json::Value = serde_json::from_str(&saved).unwrap();
        original.as_object_mut().unwrap().remove("geography");
        // This ID is valid on the new mesh, but did not identify this land on
        // the flat one. Successful action replay would still be misleading.
        for revision in [0, SPHERICAL_GEOGRAPHY_REVISION - 1] {
            original["revision"] = revision.into();
            assert!(Bench::load(&original.to_string()).is_err());
        }
        let current = Bench::load(&saved).unwrap();
        assert_eq!(current.chronicle.recipe(), bench.chronicle.recipe());
        assert_eq!(current.title(), bench.title());
        assert_eq!(current.map().unwrap(), bench.map().unwrap());
        // Later spherical revisions keep the ordinary mismatch warning rather
        // than incorrectly taking the pre-spherical recovery path.
        original["revision"] = (ENGINE_REVISION + 1).into();
        let mut different_revision = Bench::load(&original.to_string()).unwrap();
        let overview: serde_json::Value =
            serde_json::from_str(&different_revision.overview(0).unwrap()).unwrap();
        assert_eq!(overview["savedRevision"], ENGINE_REVISION + 1);
        assert_eq!(different_revision.map().unwrap(), current.map().unwrap());
    }
    #[test]
    fn specimens_show_pure_stress_movement_and_consonant_length() {
        use umran_sim::StressRule;
        let mut profile = umran_sim::SoundProfile::base();
        profile.stress = Some(StressRule::Final);
        let mut variety = Variety::found(7, &profile, Livelihood::Farming, Default::default());
        let word = variety
            .lexicon
            .slot(by_id("water").unwrap())
            .dominant()
            .unwrap();
        variety.lexicon.lexemes[word.0 as usize].form = Form::from_ipa("katːata").unwrap();
        variety.profile.stress = Some(StressRule::Initial);
        variety.stress_history.push((8, StressRule::Final));
        let rows = specimen(&variety, 8);
        let water = rows.iter().find(|w| w.concept == "water").unwrap();
        assert_eq!(water.spelled, "kattata");
        assert_eq!(water.ipa, "ˈkatːata");
        assert!(water.was.is_none());
        assert_eq!(water.was_ipa.as_deref(), Some("katːaˈta"));
        assert_eq!(water.stress, Some(0));
        let earlier = specimen(&variety, 7);
        let water = earlier.iter().find(|w| w.concept == "water").unwrap();
        assert_eq!(water.ipa, "katːaˈta");
        assert!(water.was_ipa.is_none());
    }

    #[test]
    fn grammatical_spelling_preserves_attestation_and_retired_particle_forms() {
        use umran_sim::grammar::{MarkerKind, Paradigm, Realization, Side};
        use umran_sim::lexicon::Entry;

        let form = |ipa: &str| Form::from_ipa(ipa).unwrap();
        let created = |generation| GrammarEntry {
            generation,
            event: GrammarEvent::Created,
        };
        let change = |generation, before: &str| GrammarEntry {
            generation,
            event: GrammarEvent::SoundLaw {
                law: "fixture-law",
                before: form(before),
            },
        };
        let mut bench = Bench::new(5, "medium").unwrap();
        bench.act(&found("Hill", "familiar")).unwrap();
        bench.act(&found("Lowland", "familiar")).unwrap();
        let mut world = bench.world(0).clone();
        world.generation = 10;
        world.varieties[1].profile.stress = Some(StressRule::Final);
        world.varieties[1].stress_history = vec![(8, StressRule::Initial)];
        let v = &mut world.varieties[0];
        v.written = Some(1);
        v.profile.stress = Some(StressRule::Initial);
        v.stress_history = vec![(8, StressRule::Final)];
        for word in &mut v.lexicon.lexemes {
            word.paradigms.clear();
        }
        v.grammar.markers = vec![
            GrammarMarker {
                id: 0,
                category: Category::Plural,
                kind: MarkerKind::Bound,
                side: Side::Suffix,
                form: form("i"),
                template: None,
                born: 0,
                origin: MarkerOrigin::Founding,
                productive: true,
                majority_generations: 0,
                retired: None,
                history: vec![created(0)],
            },
            GrammarMarker {
                id: 1,
                category: Category::Plural,
                kind: MarkerKind::Particle,
                side: Side::Suffix,
                form: form("meme"),
                template: None,
                born: 0,
                origin: MarkerOrigin::Imported {
                    from: 1,
                    marker: 9,
                    source: form("mimi"),
                },
                productive: false,
                majority_generations: 0,
                retired: None,
                history: vec![
                    GrammarEntry {
                        generation: 0,
                        event: GrammarEvent::Imported {
                            from: 1,
                            source: form("mimi"),
                        },
                    },
                    change(8, "mimi"),
                    change(9, "mimi"),
                ],
            },
        ];
        let stone = by_id("stone").unwrap();
        let id = v.lexicon.word_for(stone).unwrap().id;
        v.grammar.markers[0].origin = MarkerOrigin::Grammaticalized {
            source: id,
            concept: stone,
            source_form: form("kaka"),
        };
        let word = v.lexicon.get_mut(id);
        word.born = 0;
        word.form = form("tete");
        word.log = vec![
            Entry {
                generation: 6,
                event: Event::SoundLaw {
                    law: "fixture-law",
                    before: form("kaka"),
                },
            },
            Entry {
                generation: 9,
                event: Event::SoundLaw {
                    law: "fixture-law",
                    before: form("tata"),
                },
            },
        ];
        word.paradigms = vec![Paradigm {
            category: Category::Plural,
            realizations: vec![
                Realization {
                    marker: 0,
                    form: Some(form("tet")),
                    edge: 3,
                    share: 0.4,
                    born: 0,
                    retired: None,
                    history: vec![created(0), change(6, "kati")],
                },
                Realization {
                    marker: 1,
                    form: None,
                    edge: 0,
                    share: 0.3,
                    born: 0,
                    retired: None,
                    history: vec![GrammarEntry {
                        generation: 0,
                        event: GrammarEvent::Imported {
                            from: 1,
                            source: form("mimi"),
                        },
                    }],
                },
                Realization {
                    marker: 0,
                    form: Some(form("tei")),
                    edge: 2,
                    share: 0.3,
                    born: 7,
                    retired: None,
                    history: vec![created(7), change(9, "tai")],
                },
                Realization {
                    marker: 1,
                    form: None,
                    edge: 0,
                    share: 0.0,
                    born: 0,
                    retired: Some(7),
                    history: vec![
                        created(0),
                        GrammarEntry {
                            generation: 7,
                            event: GrammarEvent::Retired,
                        },
                    ],
                },
            ],
        }];
        let word = world.varieties[0].lexicon.get(id);
        let views = paradigm_views(&world, 0, word);
        let rows = &views[0].realizations;
        // An attached form keeps its own sound history, not base + today's marker.
        assert_eq!(rows[0].form, "tet");
        assert_eq!(rows[0].spelled, "kati");
        assert_eq!(rows[0].ipa, "tet");
        // A separate marker's shared history and the base's history both matter.
        assert_eq!(rows[1].form, "tete meme");
        assert_eq!(rows[1].spelled, "kaka mimi");
        assert_eq!(rows[1].ipa, "ˈtete ˈmeme");
        // A form first used after writing freezes at its own birth, not generation 1.
        assert_eq!(rows[2].form, "tei");
        assert_eq!(rows[2].spelled, "tai");
        // A retired particle pair stops changing even while the shared marker lives.
        assert_eq!(rows[3].form, "tata mimi");
        assert_eq!(rows[3].spelled, "kaka mimi");
        assert_eq!(rows[3].ipa, "taˈta miˈmi");
        assert!(rows[1].history[0].text.contains("/ˈmimi/"));
        assert!(rows[1].marker_history[1].text.contains("/miˈmi/"));
        assert!(rows[1].marker_history[1].text.contains("/ˈmimi/"));
        assert!(rows[1].marker_history[2].text.contains("/ˈmeme/"));
        let grammar = serde_json::to_value(grammar_view(&world, 0)).unwrap();
        assert_eq!(
            grammar["markers"][0]["origin"]["sourceForm"]["ipa"],
            "kaˈka"
        );
        assert_eq!(grammar["markers"][1]["origin"]["source"]["ipa"], "ˈmimi");
        // Later respelling cannot update a retired particle's host or marker.
        world.varieties[0].written = Some(10);
        let word = world.varieties[0].lexicon.get(id);
        let views = paradigm_views(&world, 0, word);
        assert_eq!(views[0].realizations[3].spelled, "tata mimi");
    }

    #[test]
    fn grammar_histories_preserve_each_stress_transition_within_one_generation() {
        use umran_sim::grammar::{MarkerKind, Paradigm, Realization, Side};

        let form = |ipa: &str| Form::from_ipa(ipa).unwrap();
        let created = || GrammarEntry {
            generation: 0,
            event: GrammarEvent::Created,
        };
        let mut bench = Bench::new(5, "medium").unwrap();
        bench.act(&found("Hill", "familiar")).unwrap();
        let mut world = bench.world(0).clone();
        world.generation = 5;
        let v = &mut world.varieties[0];
        v.profile.stress = Some(StressRule::Final);
        v.stress_history.clear();
        v.laws.clear();
        for word in &mut v.lexicon.lexemes {
            word.paradigms.clear();
        }
        v.grammar.markers = vec![
            GrammarMarker {
                id: 0,
                category: Category::Plural,
                kind: MarkerKind::Bound,
                side: Side::Suffix,
                form: form("ta"),
                template: None,
                born: 0,
                origin: MarkerOrigin::Founding,
                productive: true,
                majority_generations: 0,
                retired: None,
                history: vec![created()],
            },
            GrammarMarker {
                id: 1,
                category: Category::Plural,
                kind: MarkerKind::Particle,
                side: Side::Suffix,
                form: form("mamimi"),
                template: None,
                born: 0,
                origin: MarkerOrigin::Founding,
                productive: false,
                majority_generations: 0,
                retired: None,
                history: vec![created()],
            },
        ];
        let id = v.lexicon.word_for(by_id("person").unwrap()).unwrap().id;
        let word = v.lexicon.get_mut(id);
        word.form = form("kata");
        word.paradigms = vec![Paradigm {
            category: Category::Plural,
            realizations: vec![Realization {
                marker: 0,
                form: Some(form("katata")),
                edge: 4,
                share: 1.0,
                born: 0,
                retired: None,
                history: vec![created()],
            }],
        }];
        let laws = catalog();
        for (law_id, syllables) in [("initial-stress", (2, 0)), ("penult-stress", (0, 1))] {
            let law = laws.iter().find(|law| law.id == law_id).unwrap();
            let prior = v.stress();
            let before = v.lexicon.get(id).paradigms[0].realizations[0]
                .form
                .as_ref()
                .unwrap();
            let after = law.apply(before, v.minimal, prior);
            assert!(law.changes(before, &after, prior));
            assert_eq!(
                (
                    before.stressed_syllable(prior).unwrap(),
                    after.stressed_syllable(law.stress.unwrap()).unwrap(),
                ),
                syllables,
            );
            v.grammar
                .apply_law(&mut v.lexicon, law, v.minimal, prior, 5);
            v.laws.push((5, law.id));
            v.stress_history.push((5, prior));
            v.profile.stress = law.stress;
        }
        let word = world.varieties[0].lexicon.get(id);
        let paradigms = serde_json::to_value(paradigm_views(&world, 0, word)).unwrap();
        let history = &paradigms[0]["realizations"][0]["history"];
        assert!(
            history[1]["text"]
                .as_str()
                .unwrap()
                .ends_with("/kataˈta/ → /ˈkatata/")
        );
        assert!(
            history[2]["text"]
                .as_str()
                .unwrap()
                .ends_with("/ˈkatata/ → /kaˈtata/")
        );
        let grammar = serde_json::to_value(grammar_view(&world, 0)).unwrap();
        let history = &grammar["markers"][1]["history"];
        assert!(
            history[1]["text"]
                .as_str()
                .unwrap()
                .ends_with("/mamiˈmi/ → /ˈmamimi/")
        );
        assert!(
            history[2]["text"]
                .as_str()
                .unwrap()
                .ends_with("/ˈmamimi/ → /maˈmimi/")
        );
    }

    #[test]
    fn grammar_annals_keep_daughter_events_at_fork_without_retelling_inherited_events() {
        use umran_sim::grammar::{GrammarNotice, NoticeKind};

        let mut bench = Bench::new(5, "medium").unwrap();
        bench.act(&found("Hill", "familiar")).unwrap();
        let mut world = bench.world(0).clone();
        world.generation = 5;
        world.varieties[0].grammar.events = vec![GrammarNotice {
            generation: 5,
            category: Category::Plural,
            event: NoticeKind::ContrastLoss,
        }];
        let mut daughter = world.varieties[0].fork(0, 5);
        daughter.grammar.events.push(GrammarNotice {
            generation: 5,
            category: Category::Past,
            event: NoticeKind::ContrastLoss,
        });
        world.varieties.push(daughter);
        let entries = annals(&world);
        let grammar: Vec<_> = entries
            .iter()
            .filter_map(|entry| {
                entry.grammar.as_ref().map(|grammar| {
                    let data = serde_json::to_value(grammar).unwrap();
                    (
                        entry.variety.unwrap(),
                        data["category"].as_str().unwrap().to_string(),
                        data["event"].as_str().unwrap().to_string(),
                    )
                })
            })
            .collect();
        assert_eq!(
            grammar,
            vec![
                (0, "plural".to_string(), "contrast-loss".to_string()),
                (1, "past".to_string(), "contrast-loss".to_string()),
            ]
        );
    }
    #[test]
    fn possession_marks_the_possessor_with_bound_or_separate_genitive() {
        use umran_sim::grammar::{GrammarDesign, GrammarPrior};
        for choice in [
            GrammarChoice::Suffix,
            GrammarChoice::Prefix,
            GrammarChoice::Particle,
            GrammarChoice::None,
        ] {
            for order in [PossessorOrder::Before, PossessorOrder::After] {
                let mut profile = umran_sim::SoundProfile::base();
                profile.grammar = GrammarPrior::fixed(GrammarDesign {
                    genitive: Some(choice),
                    possessor: Some(order),
                    ..GrammarDesign::default()
                });
                let world = World::solo(11, &profile, umran_sim::Params::static_society());
                let variety = &world.varieties[0];
                let sample = grammar_sample(variety, 0).unwrap().possession;
                let child = variety
                    .lexicon
                    .word_for(umran_sim::concepts::by_id("child").unwrap())
                    .unwrap();
                let fish = variety
                    .lexicon
                    .word_for(umran_sim::concepts::by_id("fish").unwrap())
                    .unwrap();
                let realization = &child
                    .paradigms
                    .iter()
                    .find(|p| p.category == Category::Genitive)
                    .unwrap()
                    .realizations[0];
                let forms = variety.grammar.surface(&child.form, realization);
                let mut expected: Vec<_> = forms.iter().collect();
                if order == PossessorOrder::Before {
                    expected.push(&fish.form);
                } else {
                    expected.insert(0, &fish.form);
                }
                assert_eq!(
                    sample.ipa,
                    expected
                        .iter()
                        .map(|f| f.ipa_stressed(variety.stress()))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                assert_eq!(
                    sample.gloss.iter().filter(|g| g.contains("GEN")).count(),
                    usize::from(choice != GrammarChoice::None)
                );
                assert_eq!(
                    sample.gloss.iter().filter(|g| g.as_str() == "fish").count(),
                    1
                );
            }
        }
    }

    fn grammar_world(object: GrammarChoice, order: WordOrder) -> World {
        use umran_sim::grammar::{GrammarDesign, GrammarPrior};
        let mut profile = umran_sim::SoundProfile::by_id("germanic").unwrap();
        profile.grammar = GrammarPrior::fixed(GrammarDesign {
            plural: GrammarChoice::None,
            past: GrammarChoice::Suffix,
            object: Some(object),
            genitive: Some(GrammarChoice::None),
            order: Some(order),
            possessor: Some(PossessorOrder::Before),
            future: Some(GrammarChoice::Suffix),
            progressive: Some(GrammarChoice::None),
            classes: Some(umran_sim::gender::ClassChoice::None),
        });
        World::solo(7, &profile, umran_sim::Params::static_society())
    }

    #[test]
    fn grammar_sample_orders_subject_object_and_complete_past_form() {
        for (order, gloss) in [
            (WordOrder::SOV, ["child", "dog-OBJ", "see-PAST"]),
            (WordOrder::SVO, ["child", "see-PAST", "dog-OBJ"]),
            (WordOrder::VSO, ["see-PAST", "child", "dog-OBJ"]),
        ] {
            let world = grammar_world(GrammarChoice::Suffix, order);
            let variety = &world.varieties[0];
            let view = grammar_view(&world, 0);
            assert_eq!(view.marking, "case");
            let sample = view.sample.unwrap();
            assert_eq!(sample.sentence.gloss, gloss);
            let expected: Vec<_> = gloss
                .iter()
                .map(|gloss| {
                    let concept = gloss.split('-').next().unwrap();
                    let word = variety
                        .lexicon
                        .word_for(umran_sim::concepts::by_id(concept).unwrap())
                        .unwrap();
                    if concept == "child" {
                        word.form.clone()
                    } else {
                        let category = if concept == "see" {
                            Category::Past
                        } else {
                            Category::Object
                        };
                        word.paradigms
                            .iter()
                            .find(|p| p.category == category)
                            .unwrap()
                            .realizations[0]
                            .form
                            .clone()
                            .unwrap()
                    }
                })
                .collect();
            assert_eq!(sample.sentence.text, spell_surface(variety, &expected));
            assert_eq!(
                sample.sentence.ipa,
                ipa_surface(&expected, variety.stress())
            );
            assert_eq!(sample.possession.gloss, ["child", "fish"]);
        }
    }

    #[test]
    fn grammar_sample_keeps_particles_on_their_recorded_side_and_omits_absent_words() {
        let mut world = grammar_world(GrammarChoice::Particle, WordOrder::VSO);
        for side in [Side::Prefix, Side::Suffix] {
            let variety = &mut world.varieties[0];
            let object = variety
                .grammar
                .markers
                .iter_mut()
                .find(|m| m.category == Category::Object)
                .unwrap();
            object.side = side;
            let particle = object.form.clone();
            variety.grammar.possessor = PossessorOrder::After;
            let sample = grammar_view(&world, 0).sample.unwrap();
            let expected = match side {
                Side::Prefix => ["see-PAST", "child", "OBJ", "dog"],
                Side::Suffix => ["see-PAST", "child", "dog", "OBJ"],
            };
            assert_eq!(sample.sentence.gloss, expected);
            let position = expected.iter().position(|gloss| *gloss == "OBJ").unwrap();
            assert_eq!(
                sample.sentence.text.split_whitespace().nth(position),
                Some(world.varieties[0].spell(&particle).as_str()),
            );
            assert_eq!(sample.possession.gloss, ["fish", "child"]);
        }
        world.varieties[0]
            .lexicon
            .slot_mut(umran_sim::concepts::by_id("fish").unwrap())
            .variants
            .clear();
        let view = serde_json::to_value(grammar_view(&world, 0)).unwrap();
        assert!(view.get("sample").is_none());
    }

    #[test]
    fn grammar_object_apocope_loses_case_without_changing_inherited_order() {
        let mut world = grammar_world(GrammarChoice::Suffix, WordOrder::SOV);
        let variety = &mut world.varieties[0];
        variety.minimal = umran_sim::MinimalWord::Syllable;
        variety
            .grammar
            .markers
            .iter_mut()
            .find(|m| m.category == Category::Object)
            .unwrap()
            .form = Form::from_ipa("a").unwrap();
        for word in &mut variety.lexicon.lexemes {
            if word
                .paradigms
                .iter()
                .any(|p| p.category == Category::Object)
            {
                word.form = Form::from_ipa("kat").unwrap();
                word.paradigms.retain(|p| p.category != Category::Object);
            }
        }
        let stress = variety.stress();
        variety
            .grammar
            .sync(&mut variety.lexicon, &variety.morphology, stress, 0);
        assert_eq!(grammar_view(&world, 0).marking, "case");
        let variety = &mut world.varieties[0];
        let law = catalog()
            .into_iter()
            .find(|law| law.id == "apocope")
            .unwrap();
        variety
            .grammar
            .apply_law(&mut variety.lexicon, &law, variety.minimal, stress, 1);
        world.generation = 1;
        let view = grammar_view(&world, 0);
        assert_eq!(view.marking, "order");
        assert_eq!(view.order, WordOrder::SOV);
        assert_eq!(view.possessor, PossessorOrder::Before);
        let entries = annals(&world);
        let loss = entries
            .iter()
            .find(|entry| {
                entry.grammar.as_ref().is_some_and(|event| {
                    let event = serde_json::to_value(event).unwrap();
                    event["category"] == "object" && event["event"] == "contrast-loss"
                })
            })
            .expect("object contrast loss reaches the chronicle");
        println!("Object-loss annal: {}", loss.text);
        // The worn mark is no longer audible, so the sample shows a bare dog.
        let sample = view.sample.expect("sample survives the loss");
        assert!(
            !sample.sentence.gloss.iter().any(|g| g == "dog-OBJ"),
            "an eroded bound mark must not be glossed: {:?}",
            sample.sentence.gloss
        );
    }

    #[test]
    fn grammar_sample_without_object_marking_uses_the_bare_object() {
        let world = grammar_world(GrammarChoice::None, WordOrder::SOV);
        let view = grammar_view(&world, 0);
        assert_eq!(view.marking, "order");
        assert_eq!(
            view.sample.unwrap().sentence.gloss,
            ["child", "dog", "see-PAST"]
        );
    }
}

//! A frozen world: a save with the world its recipe resolved to written out
//! beside it. A recipe replays exactly only on the engine revision that wrote
//! it, so a later engine may name a people or spell a word differently. The
//! frozen record is data, not a replay: it reads the same under any engine.
//!
//! The file stays a save. Loading it replays the recipe and passes over the
//! record, so one file both reopens the world and keeps it as it was read.
use crate::annals::Annal;
use crate::extract::{NameQuarry, name_quarry};
use crate::notebook::Document;
use crate::phrasebook::{Phrase, phrases};
use crate::{
    Bench, CityView, CommunityView, ContactView, ContinentView, CraftView, LakeNamesView,
    LexiconRow, MoveView, PlaceView, ReadView, ReligionView, RiverNamesView, StateView,
    VarietyView, Workbench, WorldView, YEARS, fail, lake_names, lexicon_rows, river_names,
    world_view,
};
use serde::Serialize;
use umran_sim::{ENGINE_REVISION, HistoryPoint, TellingId};
use wasm_bindgen::prelude::*;

/// The shape of the record. Raise it when a reader of version 1 would
/// misread the record, not when a view gains a field.
const VERSION: u32 = 1;

#[derive(Serialize)]
struct FrozenFile<'a> {
    #[serde(flatten)]
    document: Document,
    frozen: Frozen<'a>,
}

/// The world as one reading resolved it. Its members are the views the
/// workbench reads, under the same names: `model.ts` describes them.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Frozen<'a> {
    version: u32,
    /// The engine revision that resolved this world. The recipe's own stamp
    /// can be older: a save reread and left unchanged keeps it.
    revision: u32,
    /// The telling read, and the exact point in it.
    telling: TellingId,
    point: HistoryPoint,
    generation: u32,
    years_per_generation: u32,
    communities: Vec<CommunityView>,
    /// Every language that has been spoken, each with its whole lexicon,
    /// the names it carries, and its phrasebook.
    varieties: Vec<FrozenVariety<'a>>,
    contacts: Vec<ContactView>,
    states: Vec<StateView>,
    cities: Vec<CityView>,
    religions: Vec<ReligionView>,
    continents: Vec<ContinentView>,
    crafts: Vec<CraftView>,
    /// Lands by region of the recipe's geography, which the recipe pins.
    places: Vec<PlaceView>,
    rivers: Vec<RiverNamesView>,
    lakes: Vec<LakeNamesView>,
    moves: Vec<MoveView>,
    annals: Vec<Annal>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrozenVariety<'a> {
    #[serde(flatten)]
    variety: VarietyView<'a>,
    lexicon: Vec<LexiconRow<'a>>,
    name_quarry: NameQuarry,
    phrasebook: Vec<Phrase>,
}

impl Bench {
    /// The save with the world as it stands at `generation` beside it. A
    /// fixed reading freezes its own point, and the file opens on its telling.
    pub fn frozen(&mut self, generation: u32) -> Result<String, String> {
        let document = self.document();
        let telling = self.chronicle.active();
        let fixed = self.fixed.as_ref().map(|(point, _)| *point);
        let generation = generation.min(self.latest());
        let point = fixed.unwrap_or_else(|| self.chronicle.point_at(generation));
        let world = self.world(generation);
        let WorldView {
            communities,
            varieties,
            contacts,
            states,
            cities,
            religions,
            continents,
            crafts,
            places,
            moves,
            annals,
        } = world_view(world);
        let varieties = varieties
            .into_iter()
            .enumerate()
            .map(|(id, variety)| {
                Ok(FrozenVariety {
                    variety,
                    lexicon: lexicon_rows(world, id)?,
                    name_quarry: name_quarry(world, id)?,
                    phrasebook: phrases(&world.varieties[id], world.generation),
                })
            })
            .collect::<Result<_, String>>()?;
        serde_json::to_string(&FrozenFile {
            document,
            frozen: Frozen {
                version: VERSION,
                revision: ENGINE_REVISION,
                telling,
                point,
                generation: world.generation,
                years_per_generation: YEARS,
                communities,
                varieties,
                contacts,
                states,
                cities,
                religions,
                continents,
                crafts,
                places,
                rivers: (0..world.river_names.len())
                    .map(|id| river_names(world, id))
                    .collect(),
                lakes: (0..world.lake_names.len())
                    .map(|id| lake_names(world, id))
                    .collect(),
                moves,
                annals,
            },
        })
        .map_err(|e| e.to_string())
    }
}

#[wasm_bindgen]
impl Workbench {
    pub fn frozen(&mut self, generation: u32) -> Result<String, JsValue> {
        self.bench.frozen(generation).map_err(fail)
    }
}

#[wasm_bindgen]
impl ReadView {
    pub fn frozen(&mut self, generation: u32) -> Result<String, JsValue> {
        self.bench.borrow_mut().frozen(generation).map_err(fail)
    }
}

#[cfg(test)]
mod tests;

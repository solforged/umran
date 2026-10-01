//! Browser facade over `langgen-sim`: the history lives here, and the
//! browser asks for presentation-ready views of any generation as JSON.

mod annals;

use annals::{Annal, annals};
use langgen_sim::compare::intelligibility;
use langgen_sim::concepts::related;
use langgen_sim::morphology::Slot;
use langgen_sim::names::PlaceOrigin;
use langgen_sim::phoneme::{Backness, Manner, Secondary};
use langgen_sim::{
    Action, CATALOG, Chronicle, ENGINE_REVISION, Event, FORMAT, Flavor, Form, Lexeme, MapSize,
    Origin, PhonemeId, Recipe, SetAside, Terrain, World, WorldEvent, catalog,
};
use langgen_sim::{LanguageDesign, MorphologyKind, Naming, Segment, Variety};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// The browser-facing history; `Bench` holds the logic so it can be tested
/// natively, where `JsValue` is unavailable.
#[wasm_bindgen]
pub struct Workbench {
    bench: Bench,
}

#[wasm_bindgen]
impl Workbench {
    /// A new history whose map `size` ("small", "medium", or "large") is
    /// drawn from `seed`.
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u32, size: &str) -> Result<Workbench, JsValue> {
        Ok(Workbench {
            bench: Bench::new(seed, size).map_err(fail)?,
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

    pub fn act(&mut self, action: &str) -> Result<(), JsValue> {
        self.bench.act(action).map_err(fail)
    }

    pub fn undo(&mut self) -> bool {
        self.bench.undo()
    }

    #[wasm_bindgen(js_name = runUntilEvent)]
    pub fn run_until_event(&mut self, limit: u32) -> u32 {
        self.bench.run_until_event(limit)
    }

    pub fn branch(&mut self, generation: u32) {
        self.bench.branch(generation)
    }

    /// Takes up a telling set aside, setting the present one aside.
    pub fn restore(&mut self, index: usize) -> Result<(), JsValue> {
        self.bench.restore(index).map_err(fail)
    }

    pub fn latest(&self) -> u32 {
        self.bench.latest()
    }

    pub fn overview(&mut self, generation: u32) -> Result<String, JsValue> {
        self.bench.overview(generation).map_err(fail)
    }

    pub fn lexicon(&mut self, generation: u32, variety: usize) -> Result<String, JsValue> {
        self.bench.lexicon(generation, variety).map_err(fail)
    }

    pub fn word(
        &mut self,
        generation: u32,
        variety: usize,
        concept: &str,
    ) -> Result<String, JsValue> {
        self.bench.word(generation, variety, concept).map_err(fail)
    }

    /// The land: regions with their outlines and terrain.
    pub fn map(&self) -> Result<String, JsValue> {
        self.bench.map().map_err(fail)
    }

    /// Every living people's word for `concept`, grouped by common root.
    #[wasm_bindgen(js_name = wordMap)]
    pub fn word_map(&mut self, generation: u32, concept: &str) -> Result<String, JsValue> {
        self.bench.word_map(generation, concept).map_err(fail)
    }
}

pub struct Bench {
    chronicle: Chronicle,
    /// The last world requested, so repeated views of one generation do not
    /// replay it again.
    cached: Option<World>,
    /// Engine revision a loaded recipe was saved with, if it differs.
    saved_revision: Option<u32>,
    /// What each telling set aside told, kept since telling it again means
    /// replaying it whole.
    told: Vec<(Vec<Action>, Vec<Annal>)>,
}

impl Bench {
    pub fn new(seed: u32, size: &str) -> Result<Bench, String> {
        let map: MapSize = serde_json::from_value(serde_json::Value::String(size.into()))
            .map_err(|_| format!("Unknown world size: {size}."))?;
        Ok(Bench {
            chronicle: Chronicle::new(u64::from(seed), map),
            cached: None,
            saved_revision: None,
            told: Vec::new(),
        })
    }

    /// Restores a saved recipe.
    pub fn load(json: &str) -> Result<Bench, String> {
        let recipe: Recipe =
            serde_json::from_str(json).map_err(|e| format!("Not a langgen save: {e}"))?;
        let chronicle = Chronicle::from_recipe(&recipe)?;
        Ok(Bench {
            chronicle,
            cached: None,
            saved_revision: (recipe.revision != ENGINE_REVISION).then_some(recipe.revision),
            told: Vec::new(),
        })
    }

    pub fn save(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.chronicle.recipe()).map_err(|e| e.to_string())
    }

    /// Sound profiles, flavors, and contact kinds to offer in forms.
    pub fn catalog() -> Result<String, String> {
        to_json(&CatalogView {
            revision: ENGINE_REVISION,
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
            name_places: langgen_sim::names::PLACES.to_vec(),
            name_epithets: langgen_sim::names::EPITHETS.to_vec(),
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
        let variety = Variety::found(u64::from(seed), &design.profile());
        let people = naming.coin(&variety, None, 0)?;
        let spelled = variety.title(&people.form);
        let language = langgen_sim::names::language_name(&variety, &people, &spelled, 0);
        let lexicon = &variety.lexicon;
        let row = |concept: &'static langgen_sim::Concept| {
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
                ipa: word.form.ipa(),
                from,
            })
        };
        let words = PREVIEW
            .iter()
            .filter_map(|id| langgen_sim::concepts::by_id(id))
            .filter_map(row)
            .collect();
        let families = langgen_sim::FAMILIES
            .iter()
            .filter_map(|(_, word, _)| langgen_sim::concepts::by_id(word))
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
                ipa: people.form.ipa(),
                meaning: people.meaning,
            },
            language: NameView {
                name: variety.title(&language.form),
                ipa: language.form.ipa(),
                meaning: language.meaning,
            },
            words,
            families,
            homophones,
            syllables,
        })
    }

    /// Applies an action (JSON, as `langgen_sim::Action`) at the latest
    /// generation. Rejected actions change nothing.
    pub fn act(&mut self, action: &str) -> Result<(), String> {
        let action: Action =
            serde_json::from_str(action).map_err(|e| format!("Malformed action: {e}"))?;
        self.chronicle.act(action)?;
        self.cached = None;
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        self.cached = None;
        self.chronicle.undo().is_some()
    }

    pub fn run_until_event(&mut self, limit: u32) -> u32 {
        self.cached = None;
        self.chronicle.run_until_event(limit)
    }

    /// Sets everything after `generation` aside as another telling.
    pub fn branch(&mut self, generation: u32) {
        self.cached = None;
        self.chronicle.branch_at(generation);
    }

    pub fn restore(&mut self, index: usize) -> Result<(), String> {
        self.cached = None;
        self.chronicle.restore(index)
    }

    /// Each telling set aside, with what it told that the present history
    /// does not, up to `generation` when viewing the past.
    fn tellings(&mut self, generation: u32) -> Vec<TellingView> {
        let current = self.chronicle.actions().to_vec();
        let present = annals(self.chronicle.latest());
        let until = if generation < self.latest() {
            generation
        } else {
            u32::MAX
        };
        let tellings = self.chronicle.tellings().to_vec();
        self.told
            .retain(|(actions, _)| tellings.iter().any(|t| &t.actions == actions));
        let mut out = Vec::new();
        for (index, telling) in tellings.iter().enumerate() {
            let told = match self.told.iter().find(|(a, _)| *a == telling.actions) {
                Some((_, told)) => told.clone(),
                None => {
                    let told = tell(self.chronicle.seed, self.chronicle.map, &telling.actions);
                    self.told.push((telling.actions.clone(), told.clone()));
                    told
                }
            };
            let from = divergence(&current, &telling.actions);
            let mut unmatched: Vec<&Annal> =
                present.iter().filter(|a| a.generation >= from).collect();
            let struck: Vec<Annal> = told
                .into_iter()
                .filter(|a| a.generation >= from && a.generation <= until)
                .filter(|a| match unmatched.iter().position(|p| *p == a) {
                    Some(i) => {
                        unmatched.swap_remove(i);
                        false
                    }
                    None => true,
                })
                .collect();
            if !struck.is_empty() {
                out.push(TellingView {
                    index,
                    why: telling.why,
                    from,
                    struck,
                });
            }
        }
        out
    }

    pub fn latest(&self) -> u32 {
        self.chronicle.latest().generation
    }

    /// Communities, varieties, contacts, and the timeline at `generation`.
    pub fn overview(&mut self, generation: u32) -> Result<String, String> {
        let latest = self.latest();
        let timeline = self.timeline();
        let seed = self.chronicle.seed;
        let saved_revision = self.saved_revision;
        let tellings = self.tellings(generation);
        let world = self.world(generation);
        let annals = annals(world);
        let spoken = world.spoken();
        let laws = catalog();
        let law_label = |id: &str| {
            laws.iter()
                .find(|l| l.id == id)
                .map_or_else(|| substrate_label(id), |l| l.label.to_string())
        };
        let view = Overview {
            seed,
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
                    name: world.community_name(id),
                    meaning: c.name.meaning.clone(),
                    ipa: c.name.form.ipa(),
                    coined: c.name.coined,
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
                            name: world.exonym(id, by),
                        })
                        // A neighbour that says the name as they do adds nothing.
                        .filter(|e| e.name != world.community_name(id))
                        .collect(),
                    variety: c.variety,
                    size: c.size,
                    prestige: c.prestige,
                    power: c.power,
                    openness: c.openness,
                    region: c.region,
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
                        name: world.language_title(id),
                        meaning: v.name.meaning.clone(),
                        parent: v.parent.map(|f| f.variety),
                        forked_at: v.parent.map(|f| f.generation),
                        family: world.family(id),
                        spoken: spoken[id],
                        profile: v.profile.name.clone(),
                        consonants: ipas(&consonants),
                        vowels: ipas(&vowels),
                        laws: v
                            .laws
                            .iter()
                            .map(|&(generation, law)| LawView {
                                generation,
                                id: law,
                                label: law_label(law),
                                from: v.waves.iter().find(|(w, _)| *w == law).map(|(_, f)| *f),
                            })
                            .collect(),
                        words: v.lexicon.living().count(),
                        word_building: word_building(v),
                        builders: builders(v),
                        minimal_word: v.minimal.label(),
                    }
                })
                .collect(),
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
            intelligibility: spoken_pairs(world),
            places: place_views(world),
            moves: move_views(world),
            annals,
            tellings,
        };
        to_json(&view)
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
                Some(LexiconRow {
                    concept: slot.concept.id,
                    gloss: slot.concept.gloss,
                    field: slot.concept.field.label(),
                    rank: slot.concept.stability,
                    spelled: v.spell(&word.form),
                    ipa: word.form.ipa(),
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
        let concept = langgen_sim::concepts::by_id(concept).ok_or("Unknown concept.")?;
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
                VariantView {
                    spelled: v.spell(&word.form),
                    ipa: word.form.ipa(),
                    share: var.weight,
                    origin: origin_view(world, variety, word),
                    senses: v.lexicon.senses(word.id).map(|c| c.gloss).collect(),
                    history: history(world, variety, word),
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
                    ipa: word.form.ipa(),
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
            width: map.width,
            height: map.height,
            regions: map
                .regions
                .iter()
                .enumerate()
                .map(|(id, r)| RegionView {
                    id,
                    terrain: r.terrain,
                    site: r.site,
                    outline: r.outline.clone(),
                    coastal: map.coastal(id),
                    island: map.island(id),
                    neighbours: r.neighbours.clone(),
                })
                .collect(),
        })
    }

    /// What each living people says for `concept` at `generation`, as a
    /// dialect atlas shows it: words descended from one root share a
    /// group, numbered in order of first appearance.
    pub fn word_map(&mut self, generation: u32, concept: &str) -> Result<String, String> {
        let concept = langgen_sim::concepts::by_id(concept).ok_or("Unknown concept.")?;
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
                    ipa: word.form.ipa(),
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
        let target = generation.min(self.latest());
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
                | Action::Connect { .. }
                | Action::Split { .. }
                | Action::Shift { .. } => continue,
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

/// The world's history up to now as annal entries: peoples appearing,
/// parting, meeting, and changing tongues, plus each language's sound laws.
/// The annals a telling set aside would have written, or none if it no
/// longer replays on this engine.
fn tell(seed: u64, map: MapSize, actions: &[Action]) -> Vec<Annal> {
    let recipe = Recipe {
        format: FORMAT.into(),
        revision: ENGINE_REVISION,
        seed,
        map,
        actions: actions.to_vec(),
        tellings: Vec::new(),
    };
    match Chronicle::from_recipe(&recipe) {
        Ok(chronicle) => annals(chronicle.latest()),
        Err(_) => Vec::new(),
    }
}

/// The generation from which two histories tell otherwise.
fn divergence(a: &[Action], b: &[Action]) -> u32 {
    let mut generation = 0;
    for (x, y) in a.iter().zip(b) {
        match (x, y) {
            (Action::Run { generations: m }, Action::Run { generations: n }) => {
                generation += m.min(n);
                if m != n {
                    break;
                }
            }
            _ if x != y => break,
            _ => {}
        }
    }
    generation
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

/// Affixes as "-ka" or "ma-"; patterns with C1 C2 C3 for root consonants.
/// Each relation's affix or pattern, then the renewing affix.
fn builders(v: &Variety) -> Vec<Builder> {
    let affix = |a: &langgen_sim::morphology::Affix| {
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

/// Every land that has been held, with all its names.
fn place_views(world: &World) -> Vec<PlaceView> {
    world
        .places
        .iter()
        .enumerate()
        .filter(|(_, names)| !names.is_empty())
        .map(|(region, names)| PlaceView {
            region,
            names: names
                .iter()
                .map(|p| {
                    let speech = &world.varieties[p.variety];
                    let spelled = speech.title(&p.name.form);
                    PlaceNameView {
                        since: p.since,
                        variety: p.variety,
                        language: world.language_title_at(p.variety, p.since),
                        ipa: p.name.form.ipa(),
                        meaning: p.name.meaning.clone(),
                        origin: match p.origin {
                            PlaceOrigin::Coined { .. } => "coined",
                            PlaceOrigin::Inherited => "inherited",
                            PlaceOrigin::Kept => "kept",
                            PlaceOrigin::Borrowed => "borrowed",
                        },
                        by: match p.origin {
                            PlaceOrigin::Coined { community } => Some(community),
                            _ => None,
                        },
                        once: Some(speech.title(p.name.form_at(p.since)))
                            .filter(|once| *once != spelled),
                        spelled,
                    }
                })
                .collect(),
        })
        .collect()
}

/// Every people's going to new land, in order.
fn move_views(world: &World) -> Vec<MoveView> {
    world
        .events
        .iter()
        .filter_map(|&(generation, ref event)| {
            let (community, from, to, kind) = match *event {
                WorldEvent::Migrated {
                    community,
                    from,
                    to,
                } => (community, from, to, "migration"),
                WorldEvent::Split {
                    daughter, from, to, ..
                } if from != to => (daughter, from, to, "split"),
                _ => return None,
            };
            Some(MoveView {
                generation,
                community,
                from,
                to,
                kind,
                overseas: world.map.overseas(from, to),
            })
        })
        .collect()
}

fn spoken_pairs(world: &World) -> Vec<Pair> {
    let spoken: Vec<usize> = (0..world.varieties.len())
        .filter(|&v| world.spoken()[v])
        .collect();
    let mut out = Vec::new();
    for (i, &a) in spoken.iter().enumerate() {
        for &b in &spoken[i + 1..] {
            out.push(Pair {
                a,
                b,
                score: intelligibility(&world.varieties[a].lexicon, &world.varieties[b].lexicon),
            });
        }
    }
    out
}

/// Whether a "borrowed" word was really kept from the speakers' old
/// language when their community shifted (a substrate word), judged in the
/// variety where the word entered.
fn kept_through_shift(world: &World, variety: usize, word: &Lexeme) -> bool {
    let Origin::Borrowed { from, .. } = word.origin else {
        return false;
    };
    let (owner, _) = world.root_of(variety, word.id);
    world.varieties[owner]
        .parent
        .is_some_and(|fork| fork.generation == word.born && fork.variety != from)
}

fn origin_view(world: &World, variety: usize, word: &Lexeme) -> OriginView {
    if kept_through_shift(world, variety, word) {
        let Origin::Borrowed { from, .. } = word.origin else {
            unreachable!()
        };
        return OriginView {
            kind: "kept",
            from: Some(world.language_title(from)),
            generation: word.born,
        };
    }
    match word.origin {
        Origin::Founding => OriginView {
            kind: "inherited",
            from: None,
            generation: word.born,
        },
        Origin::Expressive => OriginView {
            kind: "coined",
            from: None,
            generation: word.born,
        },
        Origin::Derived { base, relation } => OriginView {
            kind: "derived",
            from: Some(format!(
                "{} ({})",
                world.varieties[variety].lexicon.get(base).first_sense.gloss,
                relation.label()
            )),
            generation: word.born,
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
            }
        }
        Origin::Borrowed { from, .. } => OriginView {
            kind: "borrowed",
            from: Some(world.language_title(from)),
            generation: word.born,
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
            Origin::Borrowed { from, .. } if kept_through_shift(world, variety, word) => format!(
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
    let kept = kept_through_shift(world, variety, word);
    for (i, entry) in word.log.iter().enumerate() {
        let text = match &entry.event {
            // A kept word was never heard as foreign.
            Event::Borrowed { .. } if kept => continue,
            Event::Borrowed { source, .. } => format!(
                "Heard as /{}/, adapted to /{}/",
                source.ipa(),
                form_after(word, i).ipa()
            ),
            Event::SoundLaw { law, before } => {
                let label = laws
                    .iter()
                    .find(|l| l.id == *law)
                    .map_or_else(|| substrate_label(law), |l| l.label.to_string());
                format!(
                    "{label}: /{}/ → /{}/",
                    before.ipa(),
                    form_after(word, i).ipa()
                )
            }
            Event::Extended { to } => format!("Also came to mean '{}'", to.gloss),
            Event::Lost { sense } => format!("No longer used for '{}'", sense.gloss),
            Event::Obsolete => "Fell out of use".into(),
        };
        out.push(HistoryLine {
            generation: entry.generation,
            text,
        });
    }
    out
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

fn fail(message: String) -> JsValue {
    JsValue::from_str(&message)
}

fn to_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct Choice {
    id: String,
    name: String,
    description: String,
}

#[derive(Serialize)]
struct NameView {
    name: String,
    ipa: String,
    /// What it meant when coined.
    meaning: String,
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
    sounds: Vec<SoundView>,
    /// Chart columns and rows, in display order.
    places: Vec<&'static str>,
    manners: Vec<&'static str>,
    heights: Vec<&'static str>,
    presets: Vec<Choice>,
    contacts: Vec<Choice>,
    /// What a people can be named for, and the epithets it can take.
    name_places: Vec<&'static str>,
    name_epithets: Vec<&'static str>,
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
    let share = langgen_sim::typology::share(id);
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
    seed: u64,
    generation: u32,
    latest: u32,
    revision: u32,
    saved_revision: Option<u32>,
    timeline: Vec<Marker>,
    communities: Vec<CommunityView>,
    varieties: Vec<VarietyView>,
    contacts: Vec<ContactView>,
    intelligibility: Vec<Pair>,
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
struct TellingView {
    index: usize,
    why: SetAside,
    /// The generation from which it tells otherwise.
    from: u32,
    struck: Vec<Annal>,
}

#[derive(Serialize)]
struct PlaceView {
    region: usize,
    /// Its names, oldest first; the last is its name now.
    names: Vec<PlaceNameView>,
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
struct MoveView {
    generation: u32,
    community: usize,
    from: usize,
    to: usize,
    /// "migration" or "split".
    kind: &'static str,
    /// Whether they crossed the sea.
    overseas: bool,
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
    /// The map region it lives on.
    region: usize,
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
    family: usize,
    spoken: bool,
    profile: String,
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
}

#[derive(Serialize)]
struct ContactView {
    a: usize,
    b: usize,
    intensity: f32,
    kind: String,
}

#[derive(Serialize)]
struct Pair {
    a: usize,
    b: usize,
    score: f32,
}

#[derive(Serialize)]
struct OriginView {
    kind: &'static str,
    from: Option<String>,
    generation: u32,
}

#[derive(Serialize)]
struct LexiconRow {
    concept: &'static str,
    gloss: &'static str,
    field: &'static str,
    rank: Option<u8>,
    spelled: String,
    ipa: String,
    origin: OriginView,
    changes: usize,
    competitors: usize,
}

#[derive(Serialize)]
struct HistoryLine {
    generation: u32,
    text: String,
}

#[derive(Serialize)]
struct VariantView {
    spelled: String,
    ipa: String,
    share: f32,
    origin: OriginView,
    senses: Vec<&'static str>,
    history: Vec<HistoryLine>,
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

#[derive(Serialize)]
struct MapView {
    size: MapSize,
    width: f32,
    height: f32,
    regions: Vec<RegionView>,
}

#[derive(Serialize)]
struct RegionView {
    id: usize,
    terrain: Terrain,
    site: [f32; 2],
    outline: Vec<[f32; 2]>,
    coastal: bool,
    /// Land on a body of land of at most two regions.
    island: bool,
    /// Regions sharing a border with it.
    neighbours: Vec<usize>,
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
    use langgen_sim::CONCEPTS;

    const PEOPLE: &str = r#"{"kind":"people"}"#;
    const RIVER: &str = r#"{"kind":"place","place":"river"}"#;

    fn found(name: &str, preset: &str) -> String {
        let design = LanguageDesign::preset(preset, 1).unwrap();
        serde_json::json!({
            "kind": "found", "naming": {"kind": "people"}, "design": design, "seed": name.len(), "power": 0.5, "openness": 0.5
        })
        .to_string()
    }

    fn bench() -> Bench {
        let mut w = Bench::new(5, "medium").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        w.act(&found("Coast", "polynesian")).unwrap();
        w.act(r#"{"kind":"connect","a":0,"b":1,"intensity":0.6,"contact":"trade"}"#)
            .unwrap();
        w.act(r#"{"kind":"run","generations":12}"#).unwrap();
        w.act(r#"{"kind":"split","community":0,"intensity":0.3}"#)
            .unwrap();
        w.act(r#"{"kind":"run","generations":10}"#).unwrap();
        w
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
    }

    #[test]
    fn the_chronicle_tells_contacts_beginning_and_ending() {
        let mut w = Bench::new(5, "medium").unwrap();
        w.act(&found("Hill", "familiar")).unwrap();
        w.act(&found("Coast", "polynesian")).unwrap();
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
    fn set_aside_histories_stay_visible_struck_through() {
        let tellings = |w: &mut Bench| -> serde_json::Value {
            let overview: serde_json::Value =
                serde_json::from_str(&w.overview(w.latest()).unwrap()).unwrap();
            overview["tellings"].clone()
        };
        let mut w = bench();
        assert_eq!(tellings(&mut w), serde_json::json!([]));
        // Undo the last run and the split: both are struck, not lost.
        w.undo();
        w.undo();
        let struck = tellings(&mut w);
        assert_eq!(struck[0]["why"], "undone");
        assert_eq!(struck[0]["from"], 12);
        let kinds: Vec<&str> = struck[0]["struck"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["kind"].as_str().unwrap())
            .collect();
        assert!(kinds.contains(&"split"));
        // The struck entries come from the same telling, so restoring it
        // brings them back as the present.
        w.restore(0).unwrap();
        assert_eq!(w.latest(), 22);
        assert_eq!(tellings(&mut w), serde_json::json!([]));
        // Writing on from an earlier year is another telling.
        w.branch(5);
        w.act(r#"{"kind":"shift","community":0,"toward":1}"#)
            .unwrap();
        assert_eq!(tellings(&mut w)[0]["why"], "rewritten");
        assert_eq!(tellings(&mut w)[0]["from"], 5);
        let saved = Bench::load(&w.save().unwrap()).unwrap().save().unwrap();
        assert_eq!(saved, w.save().unwrap(), "tellings survive saving");
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
        assert_eq!(rows.as_array().unwrap().len(), CONCEPTS.len());
        let word: serde_json::Value =
            serde_json::from_str(&w.word(22, 2, "water").unwrap()).unwrap();
        assert!(!word["variants"].as_array().unwrap().is_empty());
        assert!(
            !word["cognates"].as_array().unwrap().is_empty(),
            "Upland's water is cognate with Hill's"
        );
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
            w.act(r#"{"kind":"split","community":9,"intensity":0}"#)
                .is_err()
        );
        assert!(w.act("not json").is_err());
    }
}

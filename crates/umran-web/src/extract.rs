//! Extraction reads: a meaning × language word table with root identity
//! kept, and one language's whole stock of names, each in one call, so
//! the workbench can show a quarry or export a table.
use crate::takeout::{Sheet, year};
use crate::{
    Bench, GivenView, NameView, OriginView, ReadView, Workbench, entity_id, fail, language_label,
    origin_view, silent_since, to_json,
};
use serde::Serialize;
use umran_sim::adapt::Adapter;
use umran_sim::rng::{key, stream};
use umran_sim::{LexemeId, Name, PlaceName, PlaceOrigin, Variety, World};
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
pub(crate) struct WordTable<'a> {
    generation: u32,
    /// In the order asked.
    varieties: Vec<TableVariety>,
    rows: Vec<TableRow<'a>>,
    /// Indexed by every cell's `root`, stable across the whole table.
    roots: Vec<TableRoot>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TableVariety {
    id: usize,
    name: String,
    /// When it fell silent, if it has: its words stand as they were then.
    silent_since: Option<u32>,
}

#[derive(Serialize)]
struct TableRow<'a> {
    concept: &'a str,
    gloss: &'a str,
    /// One per variety; `None` when the language has no word for the concept.
    cells: Vec<Option<TableCell>>,
}

#[derive(Serialize)]
struct TableCell {
    spelled: String,
    ipa: String,
    root: usize,
    origin: OriginView,
    /// A founding word given by the author.
    authored: bool,
    /// Other living words for the concept in this language.
    competitors: usize,
}

/// A root's first form: the founding or coined word it descends from, in
/// the language that minted it, as it was said when born.
#[derive(Serialize)]
struct TableRoot {
    variety: usize,
    spelled: String,
    ipa: String,
    born: u32,
}

pub(crate) fn word_table<'a>(
    world: &'a World,
    varieties: &[usize],
    concepts: &[&str],
) -> Result<WordTable<'a>, String> {
    let spoken = world.spoken();
    let varieties: Vec<TableVariety> = varieties
        .iter()
        .map(|&id| -> Result<TableVariety, String> {
            let spoken = *spoken
                .get(id)
                .ok_or_else(|| format!("Language {id} is not in this world."))?;
            Ok(TableVariety {
                id,
                name: language_label(world, id),
                silent_since: (!spoken).then(|| silent_since(world, id)).flatten(),
            })
        })
        .collect::<Result<_, _>>()?;
    let concepts = concepts
        .iter()
        .map(|id| {
            world
                .concepts
                .by_id(id)
                .ok_or_else(|| format!("Unknown concept: {id}."))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut roots: Vec<(usize, LexemeId)> = Vec::new();
    let rows = concepts
        .iter()
        .map(|&concept| TableRow {
            concept: &world.concepts[concept].id,
            gloss: &world.concepts[concept].gloss,
            cells: varieties
                .iter()
                .map(|variety| {
                    let v = &world.varieties[variety.id];
                    let slot = v.lexicon.slot(concept);
                    let id = slot.dominant()?;
                    let word = v.lexicon.get(id);
                    let root = world.root_of(variety.id, id);
                    let root = roots.iter().position(|r| *r == root).unwrap_or_else(|| {
                        roots.push(root);
                        roots.len() - 1
                    });
                    Some(TableCell {
                        spelled: v.spell(&word.form),
                        ipa: word.form.ipa_stressed(v.stress()),
                        root,
                        origin: origin_view(world, variety.id, word),
                        authored: word.authored,
                        competitors: slot.variants.len() - 1,
                    })
                })
                .collect(),
        })
        .collect();
    let roots = roots
        .into_iter()
        .map(|(variety, id)| {
            let v = &world.varieties[variety];
            let word = v.lexicon.get(id);
            let first = word.form_at(word.born);
            TableRoot {
                variety,
                spelled: v.spell(first),
                ipa: first.ipa_stressed(v.stress_at(word.born)),
                born: word.born,
            }
        })
        .collect();
    Ok(WordTable {
        generation: world.generation,
        varieties,
        rows,
        roots,
    })
}

#[derive(Serialize)]
pub(crate) struct NameQuarry {
    generation: u32,
    variety: usize,
    /// Every given name in fashion.
    given: Vec<GivenView>,
    /// The names this language uses for peoples: its own and its exonyms
    /// for the peoples its speakers deal with.
    peoples: Vec<PeopleName>,
    places: Vec<PlaceRow>,
    states: Vec<StateName>,
    faiths: Vec<FaithName>,
}

#[derive(Serialize)]
struct PeopleName {
    community: usize,
    name: String,
    ipa: String,
    meaning: String,
    since: u32,
}

#[derive(Serialize)]
struct PlaceRow {
    feature: &'static str,
    id: usize,
    name: String,
    ipa: String,
    meaning: String,
    since: u32,
    origin: &'static str,
}

#[derive(Serialize)]
struct StateName {
    state: usize,
    #[serde(flatten)]
    name: NameView,
}

#[derive(Serialize)]
struct FaithName {
    religion: usize,
    #[serde(flatten)]
    name: NameView,
}

fn origin_label(origin: PlaceOrigin) -> &'static str {
    match origin {
        PlaceOrigin::Coined { .. } => "coined",
        PlaceOrigin::Inherited => "inherited",
        PlaceOrigin::Kept => "kept",
        PlaceOrigin::Borrowed => "borrowed",
    }
}

pub(crate) fn name_quarry(world: &World, variety: usize) -> Result<NameQuarry, String> {
    let v = world
        .varieties
        .get(variety)
        .ok_or("No such language variety.")?;
    let speakers: Vec<usize> = world
        .living()
        .filter(|&c| world.communities[c].variety == variety)
        .collect();
    let mut peoples: Vec<PeopleName> = speakers
        .iter()
        .map(|&c| {
            let name = &world.communities[c].name;
            PeopleName {
                community: c,
                name: world.community_name(c),
                ipa: name.form.ipa_stressed(v.stress()),
                meaning: name.meaning.clone(),
                since: name.coined,
            }
        })
        .collect();
    // Exonyms as `World::exonym_heard` draws them, with the form kept for
    // its IPA; the ear is built once since it reads the whole lexicon.
    let mut ear: Option<Adapter> = None;
    for k in &world.contacts {
        for (by, other) in [(k.a, k.b), (k.b, k.a)] {
            if !speakers.contains(&by) || world.communities[other].variety == variety {
                continue;
            }
            let ear = ear.get_or_insert_with(|| world.ear(variety));
            let mut rng = stream(world.seed, &[key("exonym"), other as u64, by as u64]);
            let heard = ear.adapt(&world.communities[other].name.form, 0.0, &mut rng);
            peoples.push(PeopleName {
                community: other,
                name: v.title(&heard),
                ipa: heard.ipa_stressed(v.stress()),
                meaning: String::new(),
                since: k.since,
            });
        }
    }
    let mut places = place_rows(v, variety, "land", &world.places, &v.exonyms);
    places.extend(place_rows(
        v,
        variety,
        "river",
        &world.river_names,
        &v.river_exonyms,
    ));
    places.extend(place_rows(
        v,
        variety,
        "lake",
        &world.lake_names,
        &v.lake_exonyms,
    ));
    places.extend(
        world
            .continent_names
            .iter()
            .enumerate()
            .filter_map(|(landmass, name)| name.as_ref().map(|name| (landmass, name)))
            .filter(|(_, name)| name.variety == variety)
            .map(|(landmass, name)| PlaceRow {
                feature: "continent",
                id: landmass,
                name: name.spelled.clone(),
                ipa: name.ipa.clone(),
                meaning: name.meaning.clone(),
                since: name.since,
                origin: "coined",
            }),
    );
    places.extend(
        world
            .cities
            .iter()
            .enumerate()
            .filter(|(_, city)| world.standard_variety(city.state) == variety)
            .map(|(id, city)| {
                let name = world.city_name(id);
                // As `city_name` chooses: a heard memory first, then this
                // language's own local name, else a form fitted to its sounds.
                let origin = if v.exonyms.iter().any(|(r, _)| *r == city.region) {
                    "exonym"
                } else {
                    world.places[city.region]
                        .iter()
                        .rev()
                        .find(|p| p.variety == variety)
                        .map_or("exonym", |p| origin_label(p.origin))
                };
                PlaceRow {
                    feature: "city",
                    id,
                    name: v.title(&name.form),
                    ipa: name.form.ipa_stressed(v.stress()),
                    meaning: name.meaning.clone(),
                    since: city.since,
                    origin,
                }
            }),
    );
    Ok(NameQuarry {
        generation: world.generation,
        variety,
        given: v
            .given
            .iter()
            .map(|g| GivenView {
                name: v.title(&g.name.form),
                ipa: g.name.form.ipa_stressed(v.stress()),
                meaning: g.name.meaning.clone(),
                from: g.from,
            })
            .collect(),
        peoples,
        places,
        states: world
            .states
            .iter()
            .enumerate()
            .filter(|(_, s)| world.communities[s.rulers].variety == variety)
            .map(|(state, s)| StateName {
                state,
                name: NameView::new(v, &s.name),
            })
            .collect(),
        faiths: world
            .religions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.name_variety == variety)
            .map(|(religion, r)| FaithName {
                religion,
                name: NameView::new(v, &r.name),
            })
            .collect(),
    })
}

/// One row per place this language has a name for: its own local name
/// while its speakers hold the place, otherwise as `World::known_place`
/// chooses, a heard memory first, then its latest own local name.
fn place_rows(
    v: &Variety,
    variety: usize,
    feature: &'static str,
    records: &[Vec<PlaceName>],
    exonyms: &[(usize, Name)],
) -> Vec<PlaceRow> {
    let mut ids: Vec<usize> = exonyms
        .iter()
        .map(|(id, _)| *id)
        .chain(
            records
                .iter()
                .enumerate()
                .filter(|(_, names)| names.iter().any(|p| p.variety == variety))
                .map(|(id, _)| id),
        )
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| {
            let local = records[id].iter().rev().find(|p| p.variety == variety);
            let holds = records[id].last().is_some_and(|p| p.variety == variety);
            let heard = exonyms.iter().find(|(r, _)| *r == id).map(|(_, name)| name);
            let (name, since, origin) = match (local, heard) {
                (Some(place), _) if holds => (&place.name, place.since, origin_label(place.origin)),
                (_, Some(name)) => (name, name.coined, "exonym"),
                (Some(place), None) => (&place.name, place.since, origin_label(place.origin)),
                (None, None) => return None,
            };
            Some(PlaceRow {
                feature,
                id,
                name: v.title(&name.form),
                ipa: name.form.ipa_stressed(
                    v.stress_at(name.log.last().map_or(since, |e| e.generation.max(since))),
                ),
                meaning: name.meaning.clone(),
                since,
                origin,
            })
        })
        .collect()
}

/// A word table as CSV: one row per meaning. Each language has a column for
/// its word and one for its sounds, then one for the word's root number and
/// one for its origin. Words with the same root number are cognates. The
/// last column gives each root in the row its first form: the founding or
/// coined word it descends from, in the language that minted it.
pub(crate) fn word_table_csv(world: &World, table: &WordTable) -> String {
    let names: Vec<String> = table
        .varieties
        .iter()
        .map(|variety| match variety.silent_since {
            Some(since) => format!("{} (silent since year {})", variety.name, year(since)),
            None => variety.name.clone(),
        })
        .collect();
    let mut head = vec!["meaning".to_string()];
    for name in &names {
        head.push(name.clone());
        head.push(format!("{name} (IPA)"));
    }
    head.extend(names.iter().map(|name| format!("{name} root")));
    head.extend(names.iter().map(|name| format!("{name} origin")));
    head.push("first forms".into());
    let mut sheet = Sheet::default();
    sheet.row(&head);
    for row in &table.rows {
        let mut line = vec![row.gloss.to_string()];
        for cell in &row.cells {
            line.push(
                cell.as_ref()
                    .map_or_else(String::new, |w| w.spelled.clone()),
            );
            line.push(cell.as_ref().map_or_else(String::new, |w| w.ipa.clone()));
        }
        line.extend(row.cells.iter().map(|cell| {
            cell.as_ref()
                .map_or_else(String::new, |word| (word.root + 1).to_string())
        }));
        line.extend(row.cells.iter().map(|cell| {
            cell.as_ref().map_or_else(String::new, |word| {
                if word.authored {
                    "authored".into()
                } else {
                    word.origin.kind.to_string()
                }
            })
        }));
        let mut roots: Vec<usize> = row.cells.iter().flatten().map(|word| word.root).collect();
        roots.sort_unstable();
        roots.dedup();
        line.push(
            roots
                .into_iter()
                .map(|root| {
                    let first = &table.roots[root];
                    format!(
                        "{}: {} /{}/, {}, year {}",
                        root + 1,
                        first.spelled,
                        first.ipa,
                        language_label(world, first.variety),
                        year(first.born)
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        );
        sheet.row(&line);
    }
    sheet.text()
}

/// Every name a language uses as CSV: what kind of thing it names, the name,
/// its sounds, what its parts meant, and since when.
pub(crate) fn names_csv(quarry: &NameQuarry) -> String {
    let mut sheet = Sheet::default();
    sheet.row(["kind", "name", "ipa", "meaning", "since", "origin"]);
    for name in &quarry.given {
        let origin = if name.from.is_some() { "sacred" } else { "" };
        sheet.row([
            "given name",
            &name.name,
            &name.ipa,
            &name.meaning,
            "",
            origin,
        ]);
    }
    for name in &quarry.peoples {
        let since = year(name.since);
        sheet.row(["people", &name.name, &name.ipa, &name.meaning, &since, ""]);
    }
    for name in &quarry.places {
        let since = year(name.since);
        sheet.row([
            name.feature,
            &name.name,
            &name.ipa,
            &name.meaning,
            &since,
            name.origin,
        ]);
    }
    let named = quarry
        .states
        .iter()
        .map(|state| ("state", &state.name))
        .chain(quarry.faiths.iter().map(|faith| ("faith", &faith.name)));
    for (kind, name) in named {
        sheet.row([kind, &name.name, &name.ipa, &name.meaning, "", ""]);
    }
    sheet.text()
}

fn ids(json: &str, what: &str) -> Result<Vec<usize>, String> {
    let values: Vec<f64> =
        serde_json::from_str(json).map_err(|_| format!("Choose a list of {what}."))?;
    values.into_iter().map(entity_id).collect()
}

impl Bench {
    /// The dominant word for each of `concepts` in each of `varieties` at
    /// `generation`, with cognates sharing one root index across the table.
    pub fn word_table(
        &mut self,
        generation: u32,
        varieties: &str,
        concepts: &str,
    ) -> Result<String, String> {
        let varieties = ids(varieties, "languages")?;
        let concepts: Vec<String> =
            serde_json::from_str(concepts).map_err(|_| "Choose a list of concepts.".to_string())?;
        let concepts: Vec<&str> = concepts.iter().map(String::as_str).collect();
        to_json(&word_table(self.world(generation), &varieties, &concepts)?)
    }

    /// Every name `variety` carries at `generation`: given names, peoples,
    /// places, states, and faiths.
    pub fn name_quarry(&mut self, generation: u32, variety: usize) -> Result<String, String> {
        to_json(&name_quarry(self.world(generation), variety)?)
    }
}

#[wasm_bindgen]
impl Workbench {
    #[wasm_bindgen(js_name = wordTable)]
    pub fn word_table(
        &mut self,
        generation: u32,
        varieties: &str,
        concepts: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .word_table(generation, varieties, concepts)
            .map_err(fail)
    }

    #[wasm_bindgen(js_name = nameQuarry)]
    pub fn name_quarry(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .name_quarry(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
}

#[wasm_bindgen]
impl ReadView {
    #[wasm_bindgen(js_name = wordTable)]
    pub fn word_table(
        &mut self,
        generation: u32,
        varieties: &str,
        concepts: &str,
    ) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .word_table(generation, varieties, concepts)
            .map_err(fail)
    }

    #[wasm_bindgen(js_name = nameQuarry)]
    pub fn name_quarry(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .name_quarry(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
}

#[cfg(test)]
mod tests;

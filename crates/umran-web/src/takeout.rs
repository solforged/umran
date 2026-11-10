//! Files to take out of a world: one language's lexicon, names, sound laws,
//! and phrasebook, a sheet of chosen words across languages, and the family
//! tree. Each file is written here, in the facade, so the workbench and any
//! other caller hand out the same bytes. The workbench names what to take
//! and saves what comes back; it serializes nothing itself.
//!
//! Each writer sits beside the view it writes out (`extract.rs`,
//! `evidence.rs`, `phrasebook.rs`). This module holds the request, the CSV
//! rules, and the two files with no other home: the lexicon and the tree.
use crate::{
    Bench, LexiconForm, ReadView, VarietyView, Workbench, YEARS, evidence, extract, fail,
    language_label, lexicon_rows, phrasebook, to_json, world_view,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use umran_sim::World;
use wasm_bindgen::prelude::*;

/// What to take. Languages go by number, meanings by id.
#[derive(Deserialize)]
#[serde(tag = "what", rename_all = "kebab-case")]
enum Take {
    /// Every current word of one language, competing words included.
    Lexicon { variety: usize },
    /// Every name one language carries.
    Names { variety: usize },
    /// One language's sound laws in order, in the notation the law form reads.
    Laws { variety: usize },
    /// The same ledger as data, with each law's forms before and after.
    LawsData { variety: usize },
    /// One language's phrasebook as interlinear text.
    Phrasebook { variety: usize },
    /// Chosen meanings across chosen languages, with roots and first forms.
    Words {
        varieties: Vec<usize>,
        concepts: Vec<String>,
    },
    /// One language family as a Newick tree, or every family, one per line.
    Tree { family: Option<usize> },
}

/// A file ready to save.
#[derive(Serialize)]
struct Taken {
    /// What the file holds, in words: a caller makes its file name of this.
    name: String,
    extension: &'static str,
    /// The media type.
    media: &'static str,
    text: String,
}

const CSV: (&str, &str) = ("csv", "text/csv;charset=utf-8");
const TEXT: (&str, &str) = ("txt", "text/plain;charset=utf-8");
const JSON: (&str, &str) = ("json", "application/json");
const NEWICK: (&str, &str) = ("nwk", "text/plain;charset=utf-8");

/// A CSV file under RFC 4180's quoting: a cell holding a comma, a quote, or
/// a line break is quoted, and a quote inside it is doubled.
#[derive(Default)]
pub(crate) struct Sheet(String);

impl Sheet {
    pub(crate) fn row<S: AsRef<str>>(&mut self, cells: impl IntoIterator<Item = S>) {
        for (at, cell) in cells.into_iter().enumerate() {
            if at > 0 {
                self.0.push(',');
            }
            let cell = cell.as_ref();
            if cell.contains(['"', ',', '\n', '\r']) {
                self.0.push('"');
                self.0.push_str(&cell.replace('"', "\"\""));
                self.0.push('"');
            } else {
                self.0.push_str(cell);
            }
        }
        self.0.push('\n');
    }

    pub(crate) fn text(self) -> String {
        self.0
    }
}

pub(crate) fn year(generation: u32) -> String {
    (generation * YEARS).to_string()
}

/// One row per current word: the word each meaning goes by, then the words
/// competing with it. `id` is the meaning's own, the same in every language.
fn lexicon_csv(world: &World, variety: usize) -> Result<String, String> {
    let mut sheet = Sheet::default();
    sheet.row([
        "meaning", "word", "said", "ipa", "class", "field", "origin", "from", "since", "standing",
        "id",
    ]);
    for row in lexicon_rows(world, variety)? {
        let class = row.class.map(|class| class.to_string()).unwrap_or_default();
        let mut write = |form: &LexiconForm, class: &str, standing: &str| {
            sheet.row([
                row.gloss,
                &form.spelled,
                form.said.as_deref().unwrap_or_default(),
                &form.ipa,
                class,
                row.field,
                if form.origin.authored {
                    "authored"
                } else {
                    form.origin.kind
                },
                form.origin.from.as_deref().unwrap_or_default(),
                &year(form.origin.generation),
                standing,
                row.concept,
            ]);
        };
        write(&row.form, &class, "main");
        for form in &row.competing_forms {
            write(form, "", "competing");
        }
    }
    Ok(sheet.text())
}

/// A Newick label: quoted unless it is letters and digits alone.
fn label(name: &str) -> String {
    if !name.is_empty() && name.chars().all(char::is_alphanumeric) {
        name.into()
    } else {
        format!("'{}'", name.replace('\'', "''"))
    }
}

/// Language families as Newick trees, one to a line, with branch lengths in
/// years. A language lives on after a daughter parts from it, so each
/// parting is an unnamed fork with the daughter on one side and the parent
/// carrying on along the other. Every language is a tip: it ends at the
/// year it fell silent, or at this reading. Newick cannot draw a language
/// with two parents, so a koiné hangs from the largest of the languages it
/// was levelled from.
fn newick(varieties: &[VarietyView], now: u32, family: Option<usize>) -> Result<String, String> {
    let mut daughters: Vec<Vec<usize>> = vec![Vec::new(); varieties.len()];
    for v in varieties {
        if let Some(parent) = v.parent {
            daughters[parent].push(v.id);
        }
    }
    let parted = |id: usize| varieties[id].forked_at.unwrap_or(varieties[id].born);
    for list in &mut daughters {
        list.sort_by_key(|&id| (parted(id), id));
    }
    // Two languages of one name would be one tip to a tree reader.
    let mut namesakes: BTreeMap<&str, usize> = BTreeMap::new();
    for v in varieties {
        *namesakes.entry(&v.name).or_default() += 1;
    }
    let named = |id: usize| {
        let name = &varieties[id].name;
        if namesakes[name.as_str()] > 1 {
            label(&format!("{name} ({})", id + 1))
        } else {
            label(name)
        }
    };
    fn branch(
        id: usize,
        from: u32,
        rest: &[usize],
        ends: &dyn Fn(usize) -> u32,
        parted: &dyn Fn(usize) -> u32,
        named: &dyn Fn(usize) -> String,
        daughters: &[Vec<usize>],
    ) -> String {
        let Some(&first) = rest.first() else {
            let years = ends(id).saturating_sub(from) * YEARS;
            return format!("{}:{years}", named(id));
        };
        let at = parted(first).max(from);
        let together = rest
            .iter()
            .take_while(|&&other| parted(other) == parted(first))
            .count();
        let mut sides: Vec<String> = rest[..together]
            .iter()
            .map(|&daughter| {
                branch(
                    daughter,
                    at,
                    &daughters[daughter],
                    ends,
                    parted,
                    named,
                    daughters,
                )
            })
            .collect();
        sides.push(branch(
            id,
            at,
            &rest[together..],
            ends,
            parted,
            named,
            daughters,
        ));
        format!("({}):{}", sides.join(","), (at - from) * YEARS)
    }
    let ends = |id: usize| varieties[id].silent_since.unwrap_or(now);
    let roots: Vec<usize> = match family {
        Some(family) => {
            let root = varieties.get(family).ok_or("No such language family.")?;
            if root.parent.is_some() {
                return Err("A family is named by its first language.".into());
            }
            vec![family]
        }
        None => varieties
            .iter()
            .filter(|v| v.parent.is_none())
            .map(|v| v.id)
            .collect(),
    };
    Ok(roots
        .into_iter()
        .map(|root| {
            let tree = branch(
                root,
                varieties[root].born,
                &daughters[root],
                &ends,
                &parted,
                &named,
                &daughters,
            );
            // A family of one is still a tree to a reader that wants a fork.
            if daughters[root].is_empty() {
                format!("({tree});\n")
            } else {
                format!("{tree};\n")
            }
        })
        .collect())
}

impl Bench {
    /// The file `what` names (a JSON request), as the world stands at
    /// `generation`.
    pub fn take(&mut self, generation: u32, what: &str) -> Result<String, String> {
        let take: Take = serde_json::from_str(what)
            .map_err(|e| format!("That is not something to take out: {e}"))?;
        let world = self.world(generation);
        let at = format!("year {}", year(world.generation));
        let of = |variety: usize, what: &str| -> Result<String, String> {
            if variety >= world.varieties.len() {
                return Err("No such language variety.".into());
            }
            Ok(format!("{} {what} {at}", language_label(world, variety)))
        };
        let (name, (extension, media), text) = match take {
            Take::Lexicon { variety } => {
                (of(variety, "lexicon")?, CSV, lexicon_csv(world, variety)?)
            }
            Take::Names { variety } => (
                of(variety, "names")?,
                CSV,
                extract::names_csv(&extract::name_quarry(world, variety)?),
            ),
            Take::Laws { variety } => (
                of(variety, "sound laws")?,
                TEXT,
                evidence::laws_text(world, variety)?,
            ),
            Take::LawsData { variety } => (
                of(variety, "sound laws")?,
                JSON,
                evidence::laws_data(world, variety)?,
            ),
            Take::Phrasebook { variety } => (
                of(variety, "phrasebook")?,
                TEXT,
                phrasebook::interlinear(world, variety)?,
            ),
            Take::Words {
                varieties,
                concepts,
            } => {
                let concepts: Vec<&str> = concepts.iter().map(String::as_str).collect();
                (
                    format!("words {at}"),
                    CSV,
                    extract::word_table_csv(
                        world,
                        &extract::word_table(world, &varieties, &concepts)?,
                    ),
                )
            }
            Take::Tree { family } => {
                let varieties = world_view(world).varieties;
                let name = match family {
                    Some(family) if family < varieties.len() => {
                        format!("{} family {at}", varieties[family].name)
                    }
                    _ => format!("language families {at}"),
                };
                (name, NEWICK, newick(&varieties, world.generation, family)?)
            }
        };
        to_json(&Taken {
            name,
            extension,
            media,
            text,
        })
    }
}

#[wasm_bindgen]
impl Workbench {
    pub fn take(&mut self, generation: u32, what: &str) -> Result<String, JsValue> {
        self.bench.take(generation, what).map_err(fail)
    }
}

#[wasm_bindgen]
impl ReadView {
    pub fn take(&mut self, generation: u32, what: &str) -> Result<String, JsValue> {
        self.bench.borrow_mut().take(generation, what).map_err(fail)
    }
}

#[cfg(test)]
mod tests;

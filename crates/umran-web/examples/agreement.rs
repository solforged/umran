//! cargo run --release -p umran-web --example agreement [seeds] [subject] [fusion] [drop] [object] [object-fusion] [object-drop]
//!
//! Verb person agreement across isolated founders of the five presets, and
//! the workbench sample's languages with their "I saw you" renderings as
//! the facade shows them. Rates default to `Params::default()`.
#[path = "support/sample.rs"]
mod sample;
use serde_json::Value;
use umran_sim::grammar::{Agreement, AgreementKind, AgreementStage};
use umran_sim::{Params, SoundProfile, World};
use umran_web::Bench;

pub const PROFILES: [&str; 5] = ["familiar", "germanic", "semitic", "polynesian", "finnic"];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Counts {
    pub languages: usize,
    /// Stage at year 4000.
    pub clitic: usize,
    pub affix: usize,
    pub pro_drop: usize,
    /// Languages that ever cliticized, and losses observed over the run.
    pub ever: usize,
    pub losses: usize,
}

impl Counts {
    fn add(&mut self, other: Counts) {
        self.languages += other.languages;
        self.clitic += other.clitic;
        self.affix += other.affix;
        self.pro_drop += other.pro_drop;
        self.ever += other.ever;
        self.losses += other.losses;
    }
    pub fn marked(&self) -> usize {
        self.clitic + self.affix
    }
    fn print(&self, label: &str) {
        let share = |n: usize| 100.0 * n as f64 / self.languages.max(1) as f64;
        println!(
            "{label:12} languages={} clitic={} ({:.1}%) affix={} ({:.1}%) marked={:.1}% pro-drop={} ({:.1}%) ever={} losses={}",
            self.languages,
            self.clitic,
            share(self.clitic),
            self.affix,
            share(self.affix),
            share(self.marked()),
            self.pro_drop,
            share(self.pro_drop),
            self.ever,
            self.losses
        );
    }
}

/// An isolated society with pronoun renewal and the agreement cycle at
/// the given rates.
pub fn params(agreement: f32, fusion: f32, pro_drop: f32) -> Params {
    Params {
        pronoun_rate: Params::default().pronoun_rate,
        agreement_rate: agreement,
        agreement_fusion_rate: fusion,
        pro_drop_rate: pro_drop,
        object_agreement_rate: Params::default().object_agreement_rate,
        object_fusion_rate: Params::default().object_fusion_rate,
        object_drop_rate: Params::default().object_drop_rate,
        ..Params::static_society()
    }
}

fn count(agreement: &Agreement, out: &mut Counts) {
    out.languages += 1;
    match agreement.stage {
        AgreementStage::None => {}
        AgreementStage::Clitic => out.clitic += 1,
        AgreementStage::Affix => out.affix += 1,
    }
    out.pro_drop += usize::from(agreement.pro_drop);
    out.ever += usize::from(
        agreement
            .events
            .iter()
            .any(|e| e.kind == AgreementKind::Clitics),
    );
    out.losses += agreement
        .events
        .iter()
        .filter(|e| e.kind == AgreementKind::Loss)
        .count();
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Study {
    pub subject: Counts,
    pub object: Counts,
    /// Object outcomes restricted to languages that still mark subjects.
    pub both: Counts,
}

impl Study {
    fn add(&mut self, other: Self) {
        self.subject.add(other.subject);
        self.object.add(other.object);
        self.both.add(other.both);
    }
    fn record(&mut self, world: &World) {
        let grammar = &world.varieties[0].grammar;
        count(grammar.agreement(), &mut self.subject);
        count(grammar.object_agreement(), &mut self.object);
        if grammar.agreement().stage != AgreementStage::None {
            count(grammar.object_agreement(), &mut self.both);
        }
    }
    fn print(&self, label: &str) {
        self.subject.print(&format!("{label} subject"));
        self.object.print(&format!("{label} object"));
    }
}

/// Each profile has equal weight; every observation is an independent
/// founder over 4,000 years. Disjoint world seeds prevent shared draws.
pub fn study(seeds: u64, params: &Params) -> Study {
    println!(
        "{seeds} seeds × 160 generations per profile; agreement={} fusion={} pro-drop={}; object={} fusion={} drop={}; isolated society, pronoun renewal enabled",
        params.agreement_rate,
        params.agreement_fusion_rate,
        params.pro_drop_rate,
        params.object_agreement_rate,
        params.object_fusion_rate,
        params.object_drop_rate
    );
    let mut total = Study::default();
    let mut per_seed = vec![Study::default(); seeds as usize];
    for (p, profile) in PROFILES.into_iter().enumerate() {
        let mut row = Study::default();
        for seed in 0..seeds {
            let mut world = World::solo(
                seed * PROFILES.len() as u64 + p as u64,
                &SoundProfile::by_id(profile).unwrap(),
                params.clone(),
            );
            world.run(160);
            row.record(&world);
            per_seed[seed as usize].record(&world);
        }
        row.print(profile);
        total.add(row);
    }
    for (seed, counts) in per_seed.iter().enumerate() {
        counts.print(&format!("seed {seed}"));
    }
    total.print("pooled");
    println!(
        "pooled conditional: subject-marking={} both={} ({:.1}%) object-drop={} ({:.1}% of both)",
        total.both.languages,
        total.both.marked(),
        100.0 * total.both.marked() as f64 / total.both.languages.max(1) as f64,
        total.both.pro_drop,
        100.0 * total.both.pro_drop as f64 / total.both.marked().max(1) as f64
    );
    total
}

/// The workbench recipe's languages at its end, as the facade renders them.
/// Returns how many mark each role.
pub fn sample_report() -> (usize, usize) {
    let mut history = sample::sample();
    let mut bench = Bench::load(&serde_json::to_string(&history.recipe()).unwrap()).unwrap();
    let overview: Value = serde_json::from_str(&bench.overview(160).unwrap()).unwrap();
    let mut marked = 0;
    let mut object_marked = 0;
    println!("workbench sample (seed 21, medium, run 100 then 60) at year 4000:");
    for variety in overview["varieties"].as_array().unwrap() {
        let grammar = &variety["grammar"];
        let agreement = &grammar["agreement"];
        let object = &grammar["objectAgreement"];
        object_marked += usize::from(object["stage"] != "none");
        let stage = agreement["stage"].as_str().unwrap();
        if stage != "none" {
            marked += 1;
        }
        let pronoun = &grammar["sample"]["pronoun"]["first"];
        let rendering = if pronoun.is_null() {
            "(no pronoun sample)".to_owned()
        } else {
            format!(
                "{} /{}/ {}",
                pronoun["text"].as_str().unwrap_or(""),
                pronoun["ipa"].as_str().unwrap_or(""),
                pronoun["gloss"]
            )
        };
        println!(
            "  {} (variety {}, {}): subject={stage} pro-drop={} side={} history={} object={} object-drop={} object-history={} — I saw you: {rendering}",
            variety["name"].as_str().unwrap_or("?"),
            variety["id"],
            if variety["spoken"].as_bool().unwrap_or(false) {
                "spoken"
            } else {
                "silent"
            },
            agreement["proDrop"],
            agreement["side"].as_str().unwrap_or("?"),
            agreement["history"],
            object["stage"].as_str().unwrap(),
            object["proDrop"],
            object["history"]
        );
    }
    println!("  languages with agreement at the end: subject={marked} object={object_marked}");
    if let Some(variety) = overview["varieties"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| {
            v["grammar"]["agreement"]["stage"] != "none"
                && v["grammar"]["objectAgreement"]["stage"] != "none"
        })
        .max_by_key(|v| {
            v["grammar"]["agreement"]["history"]
                .as_array()
                .unwrap()
                .len()
                + v["grammar"]["objectAgreement"]["history"]
                    .as_array()
                    .unwrap()
                    .len()
        })
    {
        println!(
            "GrammarView.objectAgreement JSON: {}",
            variety["grammar"]["objectAgreement"]
        );
        if let Some(annal) = overview["annals"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["grammar"]["event"] == "agreement" && a["grammar"]["role"] == "object")
        {
            println!("object agreement annal JSON: {annal}");
        }
        let final_world = history.world_at(160);
        let id = variety["id"].as_u64().unwrap() as usize;
        let grammar = &final_world.varieties[id].grammar;
        let mut steps = std::collections::BTreeMap::<u32, Vec<String>>::new();
        for state in [grammar.agreement(), grammar.object_agreement()] {
            for event in &state.events {
                steps.entry(event.generation).or_default().push(format!(
                    "{} {}",
                    state.role.id(),
                    event.kind.id()
                ));
            }
        }
        for (generation, events) in steps {
            println!(
                "{} lineage: {} at generation {generation}",
                variety["name"],
                events.join(", ")
            );
            for at in [generation.saturating_sub(1), generation] {
                let mut ancestor = id;
                while let Some(fork) = final_world.varieties[ancestor].parent {
                    if fork.generation <= at {
                        break;
                    }
                    ancestor = fork.variety;
                }
                let snapshot: Value = serde_json::from_str(&bench.overview(at).unwrap()).unwrap();
                let v = snapshot["varieties"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|v| v["id"] == ancestor)
                    .unwrap();
                let sample = &v["grammar"]["sample"];
                for rendering in [
                    &sample["sentence"],
                    &sample["pronoun"]["first"],
                    &sample["pronoun"]["second"],
                ] {
                    println!(
                        "  g{at} {}: {} /{}/ {}",
                        rendering["translation"].as_str().unwrap_or("?"),
                        rendering["text"].as_str().unwrap_or("?"),
                        rendering["ipa"].as_str().unwrap_or("?"),
                        rendering["gloss"]
                    );
                }
            }
        }
    }
    (marked, object_marked)
}

#[cfg(not(test))]
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let arg = |i: usize, default: f32| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(default);
    let seeds = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(40);
    let defaults = Params::default();
    let mut params = params(
        arg(2, defaults.agreement_rate),
        arg(3, defaults.agreement_fusion_rate),
        arg(4, defaults.pro_drop_rate),
    );
    params.object_agreement_rate = arg(5, defaults.object_agreement_rate);
    params.object_fusion_rate = arg(6, defaults.object_fusion_rate);
    params.object_drop_rate = arg(7, defaults.object_drop_rate);
    study(seeds, &params);
    sample_report();
}

//! cargo run --release -p umran-web --example agreement [seeds] [agreement] [fusion] [pro-drop]
//!
//! Verb person agreement across isolated founders of the five presets, and
//! the workbench sample's languages with their "I saw you" renderings as
//! the facade shows them. Rates default to `Params::default()`.
#[path = "support/sample.rs"]
mod sample;
use serde_json::Value;
use umran_sim::grammar::{AgreementKind, AgreementStage};
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
        ..Params::static_society()
    }
}

fn count(world: &World, out: &mut Counts) {
    let agreement = world.varieties[0].grammar.agreement();
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

/// Each profile has equal weight; every observation is an independent
/// founder over 4,000 years. Disjoint world seeds prevent shared draws.
pub fn study(seeds: u64, params: &Params) -> Counts {
    println!(
        "{seeds} seeds × 160 generations per profile; agreement={} fusion={} pro-drop={}; isolated society, pronoun renewal enabled",
        params.agreement_rate, params.agreement_fusion_rate, params.pro_drop_rate
    );
    let mut total = Counts::default();
    for (p, profile) in PROFILES.into_iter().enumerate() {
        let mut row = Counts::default();
        for seed in 0..seeds {
            let mut world = World::solo(
                seed * PROFILES.len() as u64 + p as u64,
                &SoundProfile::by_id(profile).unwrap(),
                params.clone(),
            );
            world.run(160);
            count(&world, &mut row);
        }
        row.print(profile);
        total.add(row);
    }
    total.print("pooled");
    total
}

/// The workbench recipe's languages at its end, as the facade renders them.
/// Returns how many have agreement on the verb.
pub fn sample_report() -> usize {
    let history = sample::sample();
    let mut bench = Bench::load(&serde_json::to_string(&history.recipe()).unwrap()).unwrap();
    let overview: Value = serde_json::from_str(&bench.overview(160).unwrap()).unwrap();
    let mut marked = 0;
    println!("workbench sample (seed 21, medium, run 100 then 60) at year 4000:");
    for variety in overview["varieties"].as_array().unwrap() {
        let grammar = &variety["grammar"];
        let agreement = &grammar["agreement"];
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
            "  {} (variety {}, {}): stage={stage} pro-drop={} side={} history={} — I saw you: {rendering}",
            variety["name"].as_str().unwrap_or("?"),
            variety["id"],
            if variety["spoken"].as_bool().unwrap_or(false) {
                "spoken"
            } else {
                "silent"
            },
            agreement["proDrop"],
            agreement["side"].as_str().unwrap_or("?"),
            agreement["history"]
        );
    }
    println!("  languages with agreement at the end: {marked}");
    marked
}

#[cfg(not(test))]
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let arg = |i: usize, default: f32| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(default);
    let seeds = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(40);
    let defaults = Params::default();
    let params = params(
        arg(2, defaults.agreement_rate),
        arg(3, defaults.agreement_fusion_rate),
        arg(4, defaults.pro_drop_rate),
    );
    study(seeds, &params);
    sample_report();
}

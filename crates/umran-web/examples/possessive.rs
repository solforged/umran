//! cargo run --release -p umran-web --example possessive [seeds] [case] [fusion] [drop]
//! Default: 40 default-parameter worlds, 4,000 years, five rotating presets.
#[path = "support/sample.rs"]
mod sample;
use umran_sim::grammar::{AgreementKind, AgreementStage};
use umran_sim::{Params, SoundProfile, World};
use umran_web::Bench;

#[derive(Default)]
struct Counts {
    living: usize,
    ever_affix: usize,
    marked: usize,
    drop: usize,
}
impl Counts {
    fn record(&mut self, world: &World) {
        for (v, spoken) in world.spoken().into_iter().enumerate() {
            if !spoken {
                continue;
            }
            let state = world.varieties[v].grammar.possessor_agreement();
            self.living += 1;
            self.ever_affix += usize::from(
                state
                    .events
                    .iter()
                    .any(|e| e.kind == AgreementKind::Affixes),
            );
            self.marked += usize::from(state.stage != AgreementStage::None);
            self.drop += usize::from(state.pro_drop);
        }
    }
    fn print(&self, label: &str) {
        let share = |n| n as f64 / self.living.max(1) as f64;
        println!(
            "{label}: living={} ever-affix={} ({:.4}) clitic-or-affix={} ({:.4}) pro-drop={} ({:.4})",
            self.living,
            self.ever_affix,
            share(self.ever_affix),
            self.marked,
            share(self.marked),
            self.drop,
            share(self.drop)
        );
    }
    fn in_band(&self) -> bool {
        let share = |n| n as f64 / self.living.max(1) as f64;
        (0.25..=0.60).contains(&share(self.ever_affix))
            && (0.15..=0.50).contains(&share(self.marked))
    }
}
fn ribbon(world: &World, label: &str) {
    for (id, v) in world.varieties.iter().enumerate() {
        let state = v.grammar.possessor_agreement();
        let stages = state
            .events
            .iter()
            .map(|e| format!("{}:{}", e.generation * 25, e.kind.id()))
            .collect::<Vec<_>>()
            .join(" -> ");
        println!(
            "{label} language {id}: 0:none{}{}; final={} drop={} side={}",
            if stages.is_empty() { "" } else { " -> " },
            stages,
            state.stage.id(),
            state.pro_drop,
            state.side.id()
        );
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let seeds: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(40);
    let mut params = Params::default();
    for (i, rate) in [
        (2, &mut params.possessive_case_rate),
        (3, &mut params.possessive_fusion_rate),
        (4, &mut params.possessive_drop_rate),
    ] {
        if let Some(value) = args.get(i) {
            *rate = value.parse().expect("rate");
        }
    }
    println!(
        "{seeds} seeds × 4,000 years; Params::default() with possessive case={} fusion={} drop={}; living descendants pooled",
        params.possessive_case_rate, params.possessive_fusion_rate, params.possessive_drop_rate
    );
    let mut total = Counts::default();
    for seed in 0..seeds {
        let preset = ["familiar", "germanic", "semitic", "polynesian", "finnic"][seed as usize % 5];
        let mut world = World::solo(seed, &SoundProfile::by_id(preset).unwrap(), params.clone());
        world.run(160);
        let mut row = Counts::default();
        row.record(&world);
        row.print(&format!("seed {seed} {preset}"));
        total.record(&world);
        if seed == 0 {
            ribbon(&world, "seed 0");
        }
    }
    total.print("pooled default worlds");
    println!(
        "default-world band: {}",
        if total.in_band() { "PASS" } else { "OUTSIDE" }
    );
    let mut history = sample::sample();
    let world = history.world_at(160);
    let mut counts = Counts::default();
    counts.record(&world);
    counts.print("sample recipe seed 21 (reported separately)");
    ribbon(&world, "sample seed 21");
    let mut bench = Bench::load(&serde_json::to_string(&history.recipe()).unwrap()).unwrap();
    let variety = world
        .spoken()
        .into_iter()
        .enumerate()
        .filter(|(_, spoken)| *spoken)
        .max_by_key(|(id, _)| {
            let state = world.varieties[*id].grammar.possessor_agreement();
            (state.stage != AgreementStage::None, state.pro_drop)
        })
        .map(|(id, _)| id);
    if let Some(variety) = variety {
        println!("phrasebook at year 4000, sample variety {variety}:");
        let phrases: serde_json::Value =
            serde_json::from_str(&bench.phrasebook(160, variety).unwrap()).unwrap();
        for phrase in phrases.as_array().unwrap() {
            println!(
                "{}: {} /{}/ {} — {} notes={}",
                phrase["id"],
                phrase["text"],
                phrase["ipa"],
                phrase["gloss"],
                phrase["translation"],
                phrase["notes"]
            );
        }
    }
    assert!(
        seeds < 40 || total.in_band(),
        "possessor band outside contract"
    );
}

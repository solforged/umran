//! Pronoun stability, renewal, court address, and possession across profiles.
#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
use umran_sim::grammar::Category;
use umran_sim::pronouns::{CELLS, NoticeKind};
use umran_sim::{Origin, Params, SoundProfile, World};

pub const PROFILES: [&str; 5] = ["familiar", "germanic", "semitic", "polynesian", "finnic"];

#[derive(Default)]
pub struct Counts {
    pub cells: usize,
    pub renewed: usize,
    pub borrowed: usize,
    pub polite: usize,
    pub genitive: f32,
}

pub fn count(world: &World, v: usize, out: &mut Counts) {
    let variety = &world.varieties[v];
    for (_, _, cell) in CELLS {
        let word = variety
            .lexicon
            .word_for(umran_sim::concepts::by_id(cell).unwrap())
            .unwrap();
        assert!(word.form.vowel_count() > 0, "pronoun lost its last vowel");
        out.cells += 1;
        out.renewed += usize::from(matches!(word.origin, Origin::Renewed { .. }));
        out.borrowed += usize::from(matches!(word.origin, Origin::Borrowed { .. }));
    }
    out.polite += usize::from(variety.pronouns.polite);
    out.genitive +=
        variety.grammar.summary.categories[Category::Genitive.position()].contrast_retention;
}

pub fn study(profile: &str, seeds: u64) -> Counts {
    let mut counts = Counts::default();
    for seed in 0..seeds {
        let params = Params {
            pronoun_rate: Params::default().pronoun_rate,
            ..Params::static_society()
        };
        let mut world = World::solo(seed, &SoundProfile::by_id(profile).unwrap(), params);
        world.run(160);
        count(&world, 0, &mut counts);
    }
    counts
}

pub fn sample_report() -> Counts {
    let history = sample::sample();
    let world = history.latest();
    let mut counts = Counts::default();
    let mut languages = std::collections::BTreeSet::new();
    for c in world.living() {
        languages.insert(world.communities[c].variety);
    }
    for v in languages {
        count(world, v, &mut counts);
    }
    let (mut renewal, mut polite, mut borrowed) = (0, 0, 0);
    for (v, variety) in world.varieties.iter().enumerate() {
        for e in &variety.pronouns.events {
            if variety
                .parent
                .is_some_and(|p| world.varieties[p.variety].pronouns.events.contains(e))
            {
                continue;
            }
            match e.event {
                NoticeKind::Renewed { .. } => renewal += 1,
                NoticeKind::Polite { .. } => polite += 1,
                NoticeKind::Borrowed { .. } => borrowed += 1,
            }
            println!(
                "  year {} language {v} {}: {} -> {} {:?}",
                e.generation * 25,
                e.cell,
                e.before.ipa(),
                e.after.ipa(),
                e.event
            );
        }
    }
    println!(
        "sample at year {}: {} living cells, {renewal} renewals, {polite} court shifts, {borrowed} loans",
        world.generation * 25,
        counts.cells
    );
    counts
}

#[cfg(not(test))]
fn main() {
    let seeds = std::env::args()
        .nth(1)
        .and_then(|x| x.parse().ok())
        .unwrap_or(3);
    let start = std::time::Instant::now();
    println!("{seeds} seeds per profile, 4,000 years; fixed society with pronoun renewal enabled");
    for profile in PROFILES {
        let n = study(profile, seeds);
        println!(
            "{profile:12} cells={} renewed={:.1}% loans={} genitive-retained={:.1}%",
            n.cells,
            100.0 * n.renewed as f32 / n.cells as f32,
            n.borrowed,
            100.0 * n.genitive / seeds as f32
        );
    }
    sample_report();
    println!("elapsed {:.2}s", start.elapsed().as_secs_f32());
}

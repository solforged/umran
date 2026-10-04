//! Pronoun stability, renewal, court address, and possession across profiles.
#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
use umran_sim::grammar::Category;
use umran_sim::pronouns::NoticeKind;
use umran_sim::{Origin, Params, Rise, SoundProfile, World};

pub const PROFILES: [&str; 5] = ["familiar", "germanic", "semitic", "polynesian", "finnic"];

#[derive(Default)]
pub struct Counts {
    pub cells: usize,
    pub renewed: usize,
    pub borrowed: usize,
    pub polite: usize,
    pub generalised: usize,
    pub emerged: usize,
    pub systems: usize,
    pub inclusive_exclusive: usize,
    pub genitive: f32,
}

pub fn count(world: &World, v: usize, out: &mut Counts) {
    let variety = &world.varieties[v];
    out.systems += 1;
    out.inclusive_exclusive += usize::from(variety.pronouns.inclusive_exclusive);
    for (_, _, cell) in variety.pronouns.cells() {
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
    out.generalised += usize::from(variety.pronouns.generalised);
    out.emerged += usize::from(variety.pronouns.polite_since.is_some());
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

/// Matched courts isolate address change without stochastic state rise/fall.
pub fn court_study(profile: &str, seeds: u64) -> Counts {
    let mut counts = Counts::default();
    for seed in 0..seeds {
        let params = Params {
            pronoun_rate: Params::default().pronoun_rate,
            ..Params::static_society()
        };
        let mut world = World::solo(seed, &SoundProfile::by_id(profile).unwrap(), params);
        world.raise_state(0, None, Rise::Proclaimed);
        world.run(160);
        count(&world, 0, &mut counts);
        if seed == 0 && world.varieties[0].pronouns.polite {
            let v = &world.varieties[0];
            let form = |cell| {
                v.lexicon
                    .word_for(umran_sim::concepts::by_id(cell).unwrap())
                    .unwrap()
                    .form
                    .ipa()
            };
            println!(
                "{profile} court seed 0: familiar 2sg /{}/; polite 2sg /{}/; 2pl /{}/",
                form("2sg"),
                form("2sg-polite"),
                form("2pl")
            );
        }
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
    let (mut renewal, mut polite, mut borrowed, mut generalised) = (0, 0, 0, 0);
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
                NoticeKind::Generalised => generalised += 1,
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
        "sample at year {}: {} living cells, {renewal} renewals, {polite} court shifts, {generalised} generalisations, {borrowed} loans",
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
            "{profile:12} cells={} renewed={:.1}% loans={} genitive-retained={:.1}% inclusive/exclusive={}/{}",
            n.cells,
            100.0 * n.renewed as f32 / n.cells as f32,
            n.borrowed,
            100.0 * n.genitive / seeds as f32,
            n.inclusive_exclusive,
            n.systems,
        );
        let court = court_study(profile, seeds);
        println!(
            "{profile:12} courts={seeds} emerged={} coexisting={} generalised={} ({:.1}% of emerged)",
            court.emerged,
            court.polite,
            court.generalised,
            100.0 * court.generalised as f32 / court.emerged.max(1) as f32
        );
    }
    sample_report();
    println!("elapsed {:.2}s", start.elapsed().as_secs_f32());
}

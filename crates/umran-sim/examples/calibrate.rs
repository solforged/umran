//! Averages drift statistics over many seeds, for tuning `Params`.
//!
//! cargo run --release -p umran-sim --example calibrate -- [seeds] [generations] [preset]

use std::collections::HashMap;
use std::time::Instant;
use umran_sim::grammar::{Category, NoticeKind};
use umran_sim::{Craft, Lexicon, MapSize, Params, SoundProfile, World, WorldEvent};

fn kept(lexicon: &Lexicon, ranks: std::ops::RangeInclusive<u8>) -> f32 {
    let slots: Vec<_> = lexicon
        .slots
        .iter()
        .filter(|s| s.concept.stability.is_some_and(|r| ranks.contains(&r)))
        .collect();
    let kept = slots
        .iter()
        .filter(|s| lexicon.keeps_founding_word(s.concept))
        .count();
    kept as f32 / slots.len() as f32
}

fn geography(mut args: impl Iterator<Item = String>) {
    let seeds: usize = args.next().map_or(30, |s| s.parse().expect("seeds"));
    let generations: u32 = args.next().map_or(160, |s| s.parse().expect("generations"));
    let size = match args.next().as_deref().unwrap_or("medium") {
        "small" => MapSize::Small,
        "medium" => MapSize::Medium,
        "large" => MapSize::Large,
        "vast" => MapSize::Vast,
        _ => panic!("size: small, medium, large, vast"),
    };
    let founders: usize = args.next().map_or(6, |s| s.parse().expect("founders"));
    let profiles = SoundProfile::presets();
    let mut sums = [0.0f64; 9];
    let mut startup = Vec::new();
    let mut steps = Vec::new();
    for seed in 0..seeds {
        let start = Instant::now();
        let mut world = World::with_map(seed as u64, Params::default(), size);
        startup.push(start.elapsed().as_secs_f64() * 1000.0);
        for i in 0..founders {
            world.found(
                &profiles[i % profiles.len()],
                0.25 + (i % 6) as f32 * 0.12,
                0.5,
            );
        }
        for _ in 0..generations {
            let start = Instant::now();
            world.step();
            steps.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let count = |matches: fn(&WorldEvent) -> bool| {
            world.events.iter().filter(|(_, e)| matches(e)).count() as f64
        };
        let living: Vec<_> = world.living().collect();
        let row = [
            living.len() as f64,
            count(|e| matches!(e, WorldEvent::Spread { .. })),
            count(|e| matches!(e, WorldEvent::Migrated { .. })),
            world.states.iter().filter(|s| s.fell.is_none()).count() as f64,
            world.contacts.len() as f64,
            living
                .iter()
                .map(|&c| world.communities[c].size as f64)
                .sum(),
            count(|e| matches!(e, WorldEvent::Split { by_sea: true, .. })),
            count(|e| matches!(e, WorldEvent::Migrated { by_sea: true, .. })),
            living
                .iter()
                .map(|&c| world.communities[c].lands.len())
                .sum::<usize>() as f64,
        ];
        for (sum, value) in sums.iter_mut().zip(row) {
            *sum += value;
        }
        let first_ship = world.events.iter().find_map(|(g, e)| {
            matches!(
                e,
                WorldEvent::Learnt {
                    craft: Craft::Seafaring,
                    ..
                }
            )
            .then_some(*g)
        });
        println!("seed {seed}: {row:?}, first_seafaring={first_ship:?}");
    }
    startup.sort_by(f64::total_cmp);
    steps.sort_by(f64::total_cmp);
    println!(
        "means peoples spread migrations states contacts population sea_colonies sea_migrations holdings:"
    );
    println!("{:?}", sums.map(|x| x / seeds as f64));
    println!(
        "startup median {:.2} ms; step median {:.2} ms p95 {:.2} ms; mean {:.2} years/s",
        startup[startup.len() / 2],
        steps[steps.len() / 2],
        steps[steps.len() * 95 / 100],
        25_000.0 * steps.len() as f64 / steps.iter().sum::<f64>(),
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    if first.as_deref() == Some("geography") {
        geography(args);
        return;
    }
    let seeds: usize = first.map_or(200, |s| s.parse().expect("seeds"));
    let generations: u32 = args.next().map_or(40, |s| s.parse().expect("generations"));
    let only: Option<String> = args.next();

    let mut sums: HashMap<&str, f32> = HashMap::new();
    let mut founding_cases = 0;
    let mut case_losses = 0;
    let mut first_loss_years = 0_u64;
    let mut first_losses = 0;
    let mut object_rebuilds = 0;
    let profiles: Vec<SoundProfile> = match &only {
        Some(id) => vec![SoundProfile::by_id(id).expect("preset")],
        None => SoundProfile::presets(),
    };
    for n in 0..seeds {
        let profile = &profiles[n % profiles.len()];
        let mut sim = World::solo(n as u64, profile, Params::default());
        let founded_with_case = sim.varieties[0].grammar.has_object_marking();
        founding_cases += usize::from(founded_with_case);
        sim.run(generations);
        // Follow the same founder as the lexical measures, not its cloned daughters.
        let grammar = &sim.varieties[0].grammar;
        let first_loss = grammar.events.iter().find(|notice| {
            notice.category == Category::Object && matches!(notice.event, NoticeKind::ContrastLoss)
        });
        if let Some(loss) = first_loss {
            first_loss_years += u64::from(loss.generation) * 25;
            first_losses += 1;
            case_losses += usize::from(founded_with_case);
        }
        object_rebuilds += grammar
            .events
            .iter()
            .filter(|notice| {
                notice.category == Category::Object
                    && notice.generation > 0
                    && matches!(notice.event, NoticeKind::NewMarker { .. })
            })
            .count();
        let lexicon = &sim.varieties[0].lexicon;
        let culture: Vec<_> = lexicon
            .slots
            .iter()
            .filter(|s| s.concept.stability.is_none())
            .collect();
        let culture_kept = culture
            .iter()
            .filter(|s| lexicon.keeps_founding_word(s.concept))
            .count() as f32
            / culture.len() as f32;
        let mut forms: HashMap<&umran_sim::Form, usize> = HashMap::new();
        for slot in &lexicon.slots {
            if let Some(id) = slot.dominant() {
                *forms.entry(&lexicon.get(id).form).or_default() += 1;
            }
        }
        let distinct_ids: std::collections::HashSet<_> =
            lexicon.slots.iter().filter_map(|s| s.dominant()).collect();
        let mut add = |k, v: f32| *sums.entry(k).or_default() += v;
        add("case marking at year 0", u8::from(founded_with_case) as f32);
        add(
            "case marking at end",
            u8::from(grammar.has_object_marking()) as f32,
        );
        add("core retention", lexicon.core_retention());
        add("ranks 1-20 kept", kept(lexicon, 1..=20));
        add("ranks 81-100 kept", kept(lexicon, 81..=100));
        add("culture kept", culture_kept);
        add("sound laws", sim.varieties[0].laws.len() as f32);
        add(
            "contested concepts",
            lexicon
                .slots
                .iter()
                .filter(|s| s.variants.len() > 1)
                .count() as f32,
        );
        add(
            "polysemous dominant words",
            (lexicon.slots.len() - distinct_ids.len()) as f32,
        );
        add(
            "homophones among other words",
            forms.values().map(|&n| n.saturating_sub(1)).sum::<usize>() as f32
                - (lexicon.slots.len() - distinct_ids.len()) as f32,
        );
    }
    println!("{seeds} seeds × {generations} generations, profiles in rotation");
    let mut rows: Vec<_> = sums.into_iter().collect();
    rows.sort_by_key(|(k, _)| *k);
    for (k, v) in rows {
        println!("  {k:<30} {:.3}", v / seeds as f32);
    }
    println!(
        "  founding case languages lost   {case_losses}/{founding_cases} ({:.3})",
        case_losses as f32 / founding_cases.max(1) as f32
    );
    if first_losses > 0 {
        println!(
            "  mean first object loss year    {:.1} ({first_losses} languages)",
            first_loss_years as f64 / first_losses as f64
        );
    } else {
        println!("  mean first object loss year    none");
    }
    println!("  object rebuilds                {object_rebuilds}");
}

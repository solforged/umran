//! Regional rivers and climate calibration. Run with seeds, map size, generations.
use std::time::Instant;
use umran_sim::{MapSize, Params, SoundProfile, World, WorldEvent};

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u64 = args.next().map_or(30, |s| s.parse().expect("seeds"));
    let size = match args.next().as_deref().unwrap_or("medium") {
        "small" => MapSize::Small,
        "medium" => MapSize::Medium,
        "large" => MapSize::Large,
        "vast" => MapSize::Vast,
        _ => panic!("map size"),
    };
    let generations: u32 = args.next().map_or(160, |s| s.parse().expect("generations"));
    let founders = if size == MapSize::Vast { 60 } else { 6 };
    let profiles = SoundProfile::presets();
    let mut river_count = Vec::new();
    let mut lengths = Vec::new();
    let mut longest = Vec::new();
    let (mut first_states, mut first_river_states, mut events, mut moves) = (0, 0, 0, 0);
    let mut step_ms = Vec::new();
    for seed in 0..seeds {
        let mut world = World::with_map(seed, Params::default(), size);
        river_count.push(world.map.rivers.len());
        let courses: Vec<usize> = world.map.rivers.iter().map(|r| r.course.len()).collect();
        longest.push(courses.iter().copied().max().unwrap_or(0));
        lengths.extend(courses.iter().copied());
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
            step_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let climate = world
            .events
            .iter()
            .filter(|(_, e)| matches!(e, WorldEvent::Climate { .. }))
            .count();
        let migration = world
            .events
            .iter()
            .filter(|(_, e)| matches!(e, WorldEvent::Migrated { .. }))
            .count();
        events += climate;
        moves += migration;
        let first = world.states.first().map(|s| s.capital);
        if let Some(capital) = first {
            first_states += 1;
            first_river_states += usize::from(world.map.river_regions[capital].is_some());
        }
        println!(
            "seed={seed} rivers={} course_lengths={courses:?} zones={} first_state={first:?} climate_events={climate} migrations={migration} living={}",
            world.map.rivers.len(),
            world.map.climate_zones.len(),
            world.living().count()
        );
    }
    river_count.sort_unstable();
    lengths.sort_unstable();
    longest.sort_unstable();
    step_ms.sort_by(f64::total_cmp);
    let mean = |v: &[usize]| v.iter().sum::<usize>() as f64 / v.len().max(1) as f64;
    println!(
        "seeds={seeds} size={size:?} generations={generations} rivers_mean={:.3} rivers_range={:?} course_mean={:.3} course_range={:?} longest_median={} first_states={first_states} first_river_states={first_river_states} climate_events_per_1000_years={:.3} migrations_mean={:.3} step_median_ms={:.3} step_p95_ms={:.3} years_per_s={:.3}",
        mean(&river_count),
        (river_count.first(), river_count.last()),
        mean(&lengths),
        (lengths.first(), lengths.last()),
        longest[longest.len() / 2],
        events as f64 / seeds as f64 * 40.0 / f64::from(generations),
        moves as f64 / seeds as f64,
        step_ms[step_ms.len() / 2],
        step_ms[(step_ms.len() * 95 / 100).min(step_ms.len() - 1)],
        seeds as f64 * f64::from(generations) * 25_000.0 / step_ms.iter().sum::<f64>()
    );
}

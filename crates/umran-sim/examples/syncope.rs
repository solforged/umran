//! Cluster distributions for the same founding languages as the plausibility audit.
#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
#[path = "support/syncope.rs"]
mod syncope;

fn main() {
    let seeds = std::env::args()
        .nth(1)
        .map(|value| value.parse().expect("seed count"))
        .unwrap_or(40);
    let started = std::time::Instant::now();
    println!("syncope cohort: seeds={seeds} per profile, years=4000; living founder words");
    let reports = syncope::cohort(seeds);
    println!(
        "all profiles: clusters>=5={}",
        reports.iter().map(|(_, r)| r.large_words()).sum::<usize>()
    );
    let history = sample::sample();
    let world = history.latest();
    let mut report = syncope::Distribution::default();
    for (variety, spoken) in world.varieties.iter().zip(world.spoken()) {
        if spoken {
            report.add(variety);
        }
    }
    report.print("workbench sample (living varieties, not independent languages)");
    println!("elapsed={:.1}s", started.elapsed().as_secs_f32());
}

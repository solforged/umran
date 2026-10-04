//! cargo run --release -p umran-sim --example purism -- [seeds]
#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
#[path = "support/purism.rs"]
mod support;

fn main() {
    let seeds = std::env::args()
        .nth(1)
        .map_or(40, |n| n.parse::<u64>().expect("seeds"));
    let started = std::time::Instant::now();
    let mut summary = support::Summary::default();
    for seed in 0..seeds {
        let world = support::study(seed);
        let episodes: usize = world.varieties.iter().map(|v| v.purism.len()).sum();
        println!(
            "seed {seed}: profile {}, {} years, {episodes} reforms",
            world.varieties[0].profile.id,
            world.generation * 25
        );
        for (id, variety) in world.varieties.iter().enumerate() {
            for episode in &variety.purism {
                println!(
                    "  year {} high {id}: {:?}, world:{}, {} words",
                    episode.since * 25,
                    episode.cause.mechanism,
                    episode.cause.event,
                    episode.replaced.len()
                );
                for word in episode.replaced.iter().take(4) {
                    println!(
                        "    {}: {} -> {}",
                        word.concept, word.loan_form, word.native_form
                    );
                }
            }
        }
        summary.add(&world);
    }
    let sample = sample::sample();
    let mut sample_summary = support::Summary::default();
    sample_summary.add(sample.latest());
    println!(
        "BAND {seeds} seeds x 4,000 years (authored written standards, foreign contact, revival): {summary:?}"
    );
    println!("SAMPLE recipe (4,000 years, unmodified defaults): {sample_summary:?}");
    println!("Elapsed: {:.2}s", started.elapsed().as_secs_f32());
}

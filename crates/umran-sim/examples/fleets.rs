//! Coastal transport under default parameters across maps and sound profiles.
//! cargo run --release -p umran-sim --example fleets -- [seeds] [generations]
#[path = "support/fleets.rs"]
mod fleets;
#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u64 = args.next().map_or(4, |s| s.parse().expect("seeds"));
    let generations: u32 = args.next().map_or(160, |s| s.parse().expect("generations"));
    let start = std::time::Instant::now();
    let mut total = fleets::Counts::default();
    let mut maritime = fleets::Counts::default();
    let mut limited = 0;
    for seed in 0..seeds {
        let size = fleets::SIZES[seed as usize % fleets::SIZES.len()];
        let mut world = fleets::world(seed, size);
        world.run(generations);
        let mut counts = fleets::Counts::default();
        counts.add(&world);
        counts.report(&format!(
            "Seed {seed}, {size:?}, {} years",
            generations * 25
        ));
        total.add(&world);
        let (control, refused) = fleets::maritime(seed, size);
        maritime.add(&control);
        limited += refused;
    }
    total.report("All worlds");
    maritime.report("Authored maritime controls (year zero)");
    println!("Capacity-limited maritime journeys refused: {limited}");
    let mut sample_counts = fleets::Counts::default();
    sample_counts.add(sample::sample().latest());
    sample_counts.report("Full sample recipe");
    println!("Elapsed {:.2}s", start.elapsed().as_secs_f64());
}

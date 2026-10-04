//! Tonogenesis, register splitting, and loss over a founding-language cohort.
//! cargo run --release -p umran-sim --example tone -- [seeds]
#[path = "support/tone_band.rs"]
mod band;

fn main() {
    let seeds = std::env::args()
        .nth(1)
        .map_or(40, |s| s.parse().expect("seeds"));
    let stats = band::run(seeds);
    println!("tone band: {stats:?}");
}

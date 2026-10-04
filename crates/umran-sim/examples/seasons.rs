#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
#[path = "support/seasons_band.rs"]
mod support;

fn main() {
    println!("Seasonal weather: germanic, semitic, polynesian founders; 4000 years.");
    println!("Rates describe representative sampled years, not 25 separate annual ticks.");
    for seed in [0, 7, 21] {
        let world = support::history(seed);
        let mut report = support::Report::default();
        report.add(&world);
        report.print(&format!("seed {seed}"));
        for community in world.communities.iter().take(3) {
            let r = community.home();
            let profile = world.climate.seasons.profiles[r];
            println!(
                "  {}: {:?}, amplitude={:.2}, wet={:?}, floods={}",
                world.varieties[community.variety].title(&community.name.form),
                world.map.regions[r].terrain,
                profile.amplitude,
                profile.wet,
                profile.floods
            );
        }
    }
    let history = sample::sample();
    let mut report = support::Report::default();
    report.add(history.latest());
    report.print("workbench sample");
}

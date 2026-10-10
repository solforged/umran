//! cargo run --release -p umran-web --example names
#[path = "support/sample.rs"]
mod sample;

fn main() {
    let world = sample::sample().world_at(160);
    println!(
        "Sample year {}; revision {}",
        world.generation * 25,
        umran_sim::ENGINE_REVISION
    );
    for (i, people) in world.communities.iter().enumerate() {
        println!(
            "{i:3} {} /{}/ {:?}; parents {:?}",
            world.community_name(i),
            people.name.form.ipa(),
            people.name.meaning,
            people.parents
        );
    }
    let has_run = |name: &umran_sim::Name| {
        name.form
            .segs
            .windows(2)
            .any(|s| s[0].phone == s[1].phone && !umran_sim::CATALOG.get(s[0].phone).is_vowel())
    };
    let mut total = 0;
    let mut runs = 0;
    let mut current = 0;
    let mut current_runs = 0;
    for (region, places) in world.places.iter().enumerate() {
        for (i, place) in places.iter().enumerate() {
            total += 1;
            if has_run(&place.name) {
                runs += 1;
                println!(
                    "  place {region}: /{}/ {:?}; since {}; current={}",
                    place.name.form.ipa(),
                    place.name.meaning,
                    place.since,
                    i + 1 == places.len()
                );
            }
        }
        if let Some(place) = places.last() {
            current += 1;
            current_runs += usize::from(has_run(&place.name));
        }
    }
    println!(
        "Places with adjacent identical consonants: {runs}/{total} attestations; {current_runs}/{current} current names"
    );
    let stacked = world
        .living()
        .filter(|&c| {
            let people = &world.communities[c];
            people.name.has_epithet()
                && people
                    .parents
                    .iter()
                    .any(|&p| world.communities[p].name.has_epithet())
        })
        .count();
    println!("Living epithet-named daughters of epithet-named parents: {stacked}");
}

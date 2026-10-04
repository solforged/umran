//! Family-balanced noun-class sizes and inclusive/exclusive pronouns.
#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
use umran_sim::gender::ClassChoice;
use umran_sim::{Params, SoundProfile, World};

pub const PROFILES: [&str; 5] = ["familiar", "germanic", "semitic", "polynesian", "finnic"];
pub const WALS_GENDER: [f64; 5] = [
    145.0 / 257.0,
    50.0 / 257.0,
    26.0 / 257.0,
    12.0 / 257.0,
    24.0 / 257.0,
];

#[derive(Default)]
pub struct Counts {
    pub families: usize,
    pub founding: [usize; 5],
    pub final_sizes: [usize; 5],
    pub clusivity: usize,
    pub distinct_clusivity: usize,
}
fn bucket(classes: usize) -> usize {
    match classes {
        0 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        5.. => 4,
        _ => panic!("one class is not agreement"),
    }
}
fn count(world: &World, out: &mut Counts) {
    let variety = &world.varieties[0];
    out.families += 1;
    out.final_sizes[bucket(variety.gender.active_count())] += 1;
    if variety.pronouns.inclusive_exclusive {
        out.clusivity += 1;
        let form = |cell| {
            &variety
                .lexicon
                .word_for(umran_sim::concepts::by_id(cell).unwrap())
                .unwrap()
                .form
        };
        out.distinct_clusivity += usize::from(!umran_sim::grammar::same_sound(
            form("1pl-inclusive"),
            form("1pl-exclusive"),
            variety.stress(),
        ));
    }
}

/// Each profile has equal weight; every observation is an independent founder,
/// never a daughter. Disjoint world seeds prevent duplicated family draws.
pub fn study(seeds: u64) -> Counts {
    let mut total = Counts::default();
    for (p, profile) in PROFILES.into_iter().enumerate() {
        let mut row = Counts::default();
        for seed in 0..seeds {
            let params = Params {
                pronoun_rate: Params::default().pronoun_rate,
                class_emergence_rate: Params::default().class_emergence_rate,
                ..Params::static_society()
            };
            let mut world = World::solo(
                seed * PROFILES.len() as u64 + p as u64,
                &SoundProfile::by_id(profile).unwrap(),
                params,
            );
            row.founding[bucket(world.varieties[0].gender.active_count())] += 1;
            world.run(160);
            count(&world, &mut row);
        }
        println!(
            "{profile:12} families={} founding={:?} year4000={:?} inclusive/exclusive={} distinct={}",
            row.families, row.founding, row.final_sizes, row.clusivity, row.distinct_clusivity
        );
        total.families += row.families;
        total.clusivity += row.clusivity;
        total.distinct_clusivity += row.distinct_clusivity;
        for i in 0..5 {
            total.founding[i] += row.founding[i];
            total.final_sizes[i] += row.final_sizes[i];
        }
    }
    println!(
        "balanced: {} independent families, 4000 years; sizes [none,two,three,four,five+]",
        total.families
    );
    for (i, target) in WALS_GENDER.into_iter().enumerate() {
        println!(
            "gender bucket {i}: {}/{} = {:.1}%; WALS 30A {:.1}%",
            total.final_sizes[i],
            total.families,
            100.0 * total.final_sizes[i] as f64 / total.families as f64,
            target * 100.0
        );
    }
    println!(
        "inclusive/exclusive: {}/{} = {:.1}% (distinct {}); WALS 39A 63/200 = 31.5%",
        total.clusivity,
        total.families,
        100.0 * total.clusivity as f64 / total.families as f64,
        total.distinct_clusivity
    );
    total
}

pub fn sample_report() {
    let history = sample::sample();
    let world = history.latest();
    // One observation per founding family; descendants are not independent.
    for v in world
        .varieties
        .iter()
        .filter(|v| v.parent.is_none() && v.koine_of.is_empty())
    {
        println!(
            "sample year {}: classes={} inclusive/exclusive={}",
            world.generation * 25,
            v.gender.active_count(),
            v.pronouns.inclusive_exclusive
        );
    }
}

pub fn founding_report() -> [usize; 5] {
    let mut sizes = [0; 5];
    for seed in 0..1000 {
        let basis = ClassChoice::draw(seed);
        sizes[bucket(basis.draw_count(seed))] += 1;
    }
    println!("GENDER 1000 independent founding draws [none,two,three,four,five+] {sizes:?}");
    sizes
}

#[cfg(not(test))]
fn main() {
    let seeds = std::env::args()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(40);
    let started = std::time::Instant::now();
    founding_report();
    study(seeds);
    sample_report();
    println!("elapsed {:.2}s", started.elapsed().as_secs_f64());
}

//! Faith trees, pilgrim roads, and holy-land history over seeded worlds.
//! cargo run --release -p umran-sim --example faiths -- [seeds] [years] [first-seed] [seeded|natural|sample|sample-unseeded]
use umran_sim::schisms::SchismCause;
use umran_sim::{
    Action, Chronicle, ContactKind, LanguageDesign, MapSize, Naming, Params, Revelation,
    SoundProfile, World, WorldEvent,
};

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u64 = args.next().map_or(40, |s| s.parse().expect("seeds"));
    let years: u32 = args.next().map_or(4000, |s| s.parse().expect("years"));
    let first: u64 = args.next().map_or(0, |s| s.parse().expect("first seed"));
    let setup = args.next().unwrap_or_else(|| "seeded".into());
    assert!(seeds > 0 && years > 0 && years.is_multiple_of(25));
    assert!(["seeded", "natural", "sample", "sample-unseeded"].contains(&setup.as_str()));
    let presets = SoundProfile::presets();
    let mut band = Band::default();
    for seed in first..first + seeds {
        if setup.starts_with("sample") {
            match sample(seed, years, setup == "sample") {
                Ok(history) => band.record(history.latest()),
                Err(error) => println!("WORLD {seed}: sample recipe refused: {error}"),
            }
            continue;
        }
        let pick = |i: u64| &presets[((seed * 3 + i) as usize * 7) % presets.len()];
        let mut world = World::new(seed, Params::default());
        let hill = world.found(pick(0), 0.5, 0.4);
        let home = world.communities[hill].home();
        let coast = world.found_seeded(
            &Naming::People,
            pick(1),
            seed.wrapping_add(1),
            0.4,
            0.6,
            Some(home),
            None,
        );
        let empire = world.found_seeded(
            &Naming::People,
            pick(2),
            seed.wrapping_add(2),
            0.85,
            0.3,
            Some(home),
            None,
        );
        world.connect(hill, coast, 0.5, ContactKind::Trade).unwrap();
        world
            .connect(empire, coast, 0.8, ContactKind::Rule)
            .unwrap();
        world
            .connect(empire, hill, 0.3, ContactKind::Neighbours)
            .unwrap();
        if setup == "seeded" {
            let faith = world.found_religion(coast, Revelation::Proclaimed);
            world.convert(hill, faith, Some(coast));
            world.convert(empire, faith, Some(coast));
        }
        world.run(years / 25);
        band.record(&world);
    }
    band.counts.sort_unstable();
    let completed = band.counts.len();
    println!(
        "\nBAND {seeds} seeds x {years} years, default Params, setup {setup}; {completed} complete, {} refused",
        seeds as usize - completed
    );
    if completed == 0 {
        return;
    }
    let total: usize = band.counts.iter().sum();
    let median = (band.counts[(completed - 1) / 2] + band.counts[completed / 2]) as f32 / 2.0;
    println!(
        "schisms: total {total}, min {}, median {median:.1}, max {}; maximum descendants {}",
        band.counts[0],
        band.counts[completed - 1],
        band.max_descendants
    );
    println!(
        "succession: {}/{total} ({:.1}%); worlds with holy-land events: {}/{completed} ({:.1}%)",
        band.succession,
        100.0 * band.succession as f32 / total.max(1) as f32,
        band.holy_worlds,
        100.0 * band.holy_worlds as f32 / completed as f32
    );
    println!(
        "unseeded founding: {} faiths in {}/{completed} worlds; holy-war bonus: 2.0x, {} eligible conquest comparisons",
        band.natural_faiths, band.natural_worlds, band.holy_checks
    );
}

/// The exact sample.ts recipe and facade preset resolution, varying only
/// the world seed. The unseeded control omits its authored Religion action.
fn sample(seed: u64, years: u32, authored: bool) -> Result<Chronicle, String> {
    if years < 3250 {
        return Err("sample needs at least 3,250 years".into());
    }
    let mut history = Chronicle::new(seed, MapSize::Medium);
    for (preset, language_seed, naming, power) in [
        (
            "germanic",
            31,
            Naming::Place {
                place: "river".into(),
            },
            0.7,
        ),
        ("semitic", 52, Naming::People, 0.5),
        (
            "polynesian",
            73,
            Naming::Place {
                place: "sea".into(),
            },
            0.4,
        ),
    ] {
        history.act(Action::Found {
            naming,
            design: LanguageDesign::preset(preset, language_seed).expect("sample preset"),
            seed: language_seed,
            power,
            openness: 0.5,
            region: None,
            livelihood: None,
            ethos: None,
        })?;
    }
    history.act(Action::Run { generations: 100 })?;
    history.act(Action::Connect {
        a: 0,
        b: 2,
        intensity: 0.8,
        contact: ContactKind::Rule,
    })?;
    history.act(Action::Run { generations: 30 })?;
    if authored {
        history.act(Action::Religion { community: 2 })?;
    }
    history.act(Action::Shift {
        community: 2,
        toward: 0,
    })?;
    history.act(Action::Run {
        generations: years / 25 - 130,
    })?;
    Ok(history)
}

#[derive(Default)]
struct Band {
    counts: Vec<usize>,
    succession: usize,
    holy_worlds: usize,
    max_descendants: usize,
    holy_checks: u32,
    natural_faiths: usize,
    natural_worlds: usize,
}

impl Band {
    fn record(&mut self, world: &World) {
        let schisms = world
            .religions
            .iter()
            .filter(|r| r.parent.is_some())
            .count();
        self.succession += world
            .religions
            .iter()
            .filter(|r| r.cause == Some(SchismCause::Succession))
            .count();
        self.counts.push(schisms);
        self.holy_checks += world.holy_war_checks;
        let natural = world
            .religions
            .iter()
            .filter(|r| r.parent.is_none() && r.how != Revelation::Proclaimed)
            .count();
        self.natural_faiths += natural;
        self.natural_worlds += usize::from(natural > 0);
        let holy: Vec<_> = world
            .events
            .iter()
            .filter(|(_, e)| matches!(e, WorldEvent::HolyLand { .. }))
            .collect();
        self.holy_worlds += usize::from(!holy.is_empty());
        println!(
            "\nWORLD {} year {}: {} faiths ({natural} unseeded), {schisms} schisms, {} holy-land changes, {} holy-war comparisons",
            world.seed,
            world.generation * 25,
            world.religions.len(),
            holy.len(),
            world.holy_war_checks
        );
        for root in (0..world.religions.len()).filter(|&r| world.religions[r].parent.is_none()) {
            let descendants = (0..world.religions.len())
                .filter(|&r| r != root && world.faith_root(r) == root)
                .count();
            self.max_descendants = self.max_descendants.max(descendants);
            tree(world, root, 0);
        }
        for (generation, event) in holy {
            if let WorldEvent::HolyLand {
                religion,
                region,
                was_held_by,
                held_by,
                faithful,
            } = event
            {
                println!(
                    "  year {}: faith {religion}, shrine {region}: {was_held_by:?} -> {held_by:?}, faithful {faithful}",
                    generation * 25
                );
            }
        }
    }
}

fn tree(world: &World, id: usize, depth: usize) {
    let r = &world.religions[id];
    let indent = "  ".repeat(depth + 1);
    println!(
        "{indent}faith {id}: {} ({:?}); parent {:?}, cause {:?}, named {:?}, year {}, translates {}",
        world.faith_name(id),
        r.name.meaning,
        r.parent,
        r.cause,
        r.named,
        r.founded * 25,
        r.translates
    );
    for shrine in r.shrines() {
        println!(
            "{indent}  shrine {}: {} ({:?}, {:?})",
            shrine.region,
            world.varieties[shrine.variety].title(&shrine.name.form),
            shrine.name.meaning,
            shrine.kind
        );
    }
    for land in world.holy_lands(id) {
        println!(
            "{indent}  holy land {} held by {:?}, faithful {}",
            land.region, land.held_by, land.faithful
        );
    }
    for p in &r.pilgrims {
        println!(
            "{indent}  pilgrims people {}: {} -> {}, since year {}, path {:?}",
            p.people,
            p.from,
            p.to,
            p.since * 25,
            p.path
        );
    }
    for child in (0..world.religions.len()).filter(|&c| world.religions[c].parent == Some(id)) {
        tree(world, child, depth + 1);
    }
}

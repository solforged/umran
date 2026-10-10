use rand::RngCore;
use umran_sim::rng::{key, stream};
use umran_sim::{
    Form, GeographyVersion, Livelihood, MapSize, Name, Naming, Params, SoundProfile, World,
};

pub fn epithet(name: &Name) -> bool {
    name.meaning.strip_prefix("the ").is_some_and(|rest| {
        umran_sim::names::EPITHETS
            .iter()
            .any(|e| rest.strip_prefix(e).is_some_and(|s| s.starts_with(' ')))
    })
}

pub fn consonant_run(form: &Form) -> bool {
    form.segs
        .windows(2)
        .any(|s| s[0].phone == s[1].phone && !umran_sim::CATALOG.get(s[0].phone).is_vowel())
}

// Same walking-effort and gap selection as examples/settlement.rs, three-grouped.
fn grouped_sites(world: &World, anchor: usize) -> Vec<usize> {
    let mut nearby: Vec<_> = world
        .map
        .walking_row(anchor, world.params.settle_apart)
        .iter()
        .map(|&(r, effort)| (r as usize, effort))
        .filter(|&(r, effort)| {
            effort.is_finite() && Livelihood::ALL.iter().any(|&way| world.feeds(r, way) > 0.0)
        })
        .collect();
    nearby.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut steps = vec![u8::MAX; world.map.regions.len()];
    let mut sites = Vec::with_capacity(3);
    for gap in (0..=2).rev() {
        for &(r, _) in &nearby {
            if sites.len() == 3 {
                break;
            }
            if steps[r] > gap || (r == anchor && sites.is_empty()) {
                sites.push(r);
                steps[r] = 0;
                let mut frontier = vec![r];
                for step in 1..=2 {
                    let mut next = Vec::new();
                    for r in frontier {
                        for &n in &world.map.regions[r].neighbours {
                            if steps[n] > step {
                                steps[n] = step;
                                next.push(n);
                            }
                        }
                    }
                    frontier = next;
                }
            }
        }
    }
    assert_eq!(sites.len(), 3);
    sites
}

pub fn world(seed: u64) -> World {
    let mut world = World::with_geography(
        seed,
        Params::default(),
        MapSize::Medium,
        GeographyVersion::ContinentalV6,
    );
    let profiles = SoundProfile::presets();
    let mut probe = world.clone();
    let founder = probe.found(&profiles[0], 0.25, 0.5);
    let sites = grouped_sites(&world, probe.communities[founder].home());
    for (i, &site) in sites.iter().enumerate() {
        world.found_seeded(
            &Naming::People,
            &profiles[i],
            stream(seed, &[key("found"), i as u64]).next_u64(),
            0.25 + i as f32 * 0.12,
            0.5,
            Some(site),
            None,
            None,
        );
    }
    world
}

pub fn assert_forms(world: &World) {
    let check = |form: &Form| {
        assert!(
            !consonant_run(form),
            "seed {}, generation {}: /{}/",
            world.seed,
            world.generation,
            form.ipa()
        )
    };
    for c in world.living() {
        let c = &world.communities[c];
        check(&c.name.form);
        let v = &world.varieties[c.variety];
        check(&v.name.form);
        for word in v.lexicon.living() {
            check(&word.form);
            for realization in word.paradigms.iter().flat_map(|p| &p.realizations) {
                if realization.retired.is_none()
                    && let Some(form) = &realization.form
                {
                    check(form);
                }
            }
        }
        for marker in &v.grammar.markers {
            if marker.retired.is_none() {
                check(&marker.form);
            }
        }
        for (_, name) in v
            .exonyms
            .iter()
            .chain(&v.river_exonyms)
            .chain(&v.lake_exonyms)
        {
            check(&name.form);
        }
        for name in &v.given {
            check(&name.name.form);
        }
    }
    for place in world
        .places
        .iter()
        .chain(&world.river_names)
        .chain(&world.lake_names)
        .filter_map(|p| p.last())
    {
        check(&place.name.form);
    }
}

pub fn band(seeds: u64, generations: u32, verify: bool) {
    let (mut living, mut duplicate, mut epithets, mut stacked, mut daughters, mut occupied) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    for seed in 0..seeds {
        let mut world = world(seed);
        if verify {
            assert_forms(&world);
        }
        for _ in 0..generations {
            world.step();
            if verify {
                assert_forms(&world);
            }
        }
        let ids: Vec<_> = world.living().collect();
        let names: Vec<_> = ids.iter().map(|&c| world.community_name(c)).collect();
        let count = ids
            .iter()
            .filter(|&&c| {
                epithet(&world.communities[c].name)
                    && world.communities[c]
                        .parents
                        .iter()
                        .any(|&p| epithet(&world.communities[p].name))
            })
            .count();
        println!("seed {seed}: living={}, stacked={count}", ids.len());
        stacked += count;
        daughters += world
            .communities
            .iter()
            .filter(|c| !c.parents.is_empty())
            .count();
        living += ids.len();
        duplicate += names
            .iter()
            .enumerate()
            .filter(|(i, name)| {
                names
                    .iter()
                    .enumerate()
                    .any(|(j, other)| *i != j && *name == other)
            })
            .count();
        epithets += ids
            .iter()
            .filter(|&&c| epithet(&world.communities[c].name))
            .count();
        let mut lands = std::collections::BTreeSet::new();
        for &c in &ids {
            lands.extend(world.communities[c].lands.iter().copied());
        }
        occupied += lands.len();
    }
    println!(
        "{seeds} seeds x {generations} generations, three-grouped: mean living={:.3}; duplicate-name share={:.3}% ({duplicate}/{living}); epithet share={:.3}% ({epithets}/{living}); stacked={stacked}/{daughters} daughters ({:.3}%); mean occupied={:.3}",
        living as f64 / seeds as f64,
        duplicate as f64 * 100.0 / living as f64,
        epithets as f64 * 100.0 / living as f64,
        stacked as f64 * 100.0 / daughters.max(1) as f64,
        occupied as f64 / seeds as f64
    );
    if verify {
        assert!(
            stacked * 100 <= daughters,
            "stacked names exceed 1% of daughters"
        );
    }
}

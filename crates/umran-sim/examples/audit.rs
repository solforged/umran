//! Plausibility audit: worlds like the ones people build in the book,
//! three peoples in trade, rule, and neighbourhood, run for a long span.
//! Reports, for every spoken language, measures that real languages keep
//! within known bands, then sample words and names to read by eye.
//!
//! cargo run --release -p umran-sim --example audit -- [seeds] [generations]

use umran_sim::{CATALOG, ContactKind, Form, Naming, Origin, Params, SoundProfile, Variety, World};

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u64 = args.next().map_or(20, |s| s.parse().expect("seeds"));
    let last: u32 = args.next().map_or(255, |s| s.parse().expect("generations"));
    let presets = SoundProfile::presets();
    let n = presets.len() as u64;
    let mut rows: Vec<Row> = Vec::new();
    for seed in 0..seeds {
        let pick = |i: u64| &presets[((((seed % n) * 3 + i) * 7) % n) as usize];
        let mut world = World::new(
            seed,
            Params {
                ethos_enabled: !std::env::args().any(|a| a == "--neutral"),
                ..Params::default()
            },
        );
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
            None,
        );
        world.connect(hill, coast, 0.5, ContactKind::Trade).unwrap();
        world
            .connect(empire, coast, 0.8, ContactKind::Rule)
            .unwrap();
        world
            .connect(empire, hill, 0.3, ContactKind::Neighbours)
            .unwrap();
        world.run(last);
        let spoken = world.spoken();
        for (v, variety) in world
            .varieties
            .iter()
            .enumerate()
            .filter(|(v, _)| spoken[*v])
        {
            rows.push(measure(&world, v, variety));
        }
        if seed < 3 {
            sample(&world, seed);
        }
    }
    println!(
        "\n{} spoken languages over {seeds} worlds, {last} generations",
        rows.len()
    );
    let stat = |name: &str, f: &dyn Fn(&Row) -> f32| {
        let mut v: Vec<f32> = rows.iter().map(f).collect();
        v.sort_by(f32::total_cmp);
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        println!(
            "  {name:<34} mean {mean:>6.2}  min {:>6.2}  median {:>6.2}  90% {:>6.2}  max {:>6.2}",
            v[0],
            v[v.len() / 2],
            v[v.len() * 9 / 10],
            v[v.len() - 1]
        );
    };
    stat("consonants", &|r| r.consonants);
    stat("vowels", &|r| r.vowels);
    stat("sounds not in founding inventory", &|r| r.foreign);
    stat("syllables per word", &|r| r.syllables);
    stat("words of 4+ syllables (share)", &|r| r.long);
    stat("core loans (share)", &|r| r.core_loans);
    stat("all loans (share)", &|r| r.loans);
    stat("sound laws", &|r| r.laws);
    stat("longest consonant run", &|r| r.cluster);
    stat("words with 3+ consonants in a row", &|r| r.clusters);
    stat("words with 3+ vowels in a row", &|r| r.hiatus);
    stat("language name syllables", &|r| r.language_name);
    stat("people name syllables (mean)", &|r| r.people_name);
    stat("words with commonest consonant", &|r| r.top_consonant);
    stat("words repeating a consonant", &|r| r.repeats);
}

struct Row {
    consonants: f32,
    vowels: f32,
    foreign: f32,
    syllables: f32,
    long: f32,
    core_loans: f32,
    loans: f32,
    laws: f32,
    cluster: f32,
    clusters: f32,
    hiatus: f32,
    language_name: f32,
    people_name: f32,
    /// Share of words containing the language's commonest consonant.
    top_consonant: f32,
    /// Share of words repeating a consonant (fuf, kak).
    repeats: f32,
}

/// Longest runs of consonants and of vowels in a form.
fn runs(form: &Form) -> (usize, usize) {
    let (mut c, mut v, mut max_c, mut max_v) = (0, 0, 0, 0);
    for i in 0..form.segs.len() {
        if form.is_vowel(i) {
            v += 1;
            c = 0;
        } else {
            c += 1;
            v = 0;
        }
        max_c = max_c.max(c);
        max_v = max_v.max(v);
    }
    (max_c, max_v)
}

fn measure(world: &World, v: usize, variety: &Variety) -> Row {
    let lexicon = &variety.lexicon;
    let words: Vec<_> = lexicon
        .slots
        .iter()
        .filter_map(|s| s.dominant().map(|id| (s.concept, lexicon.get(id))))
        .collect();
    let n = words.len() as f32;
    let share = |f: &dyn Fn(&(&umran_sim::Concept, &umran_sim::Lexeme)) -> bool| {
        words.iter().filter(|w| f(w)).count() as f32
    };
    let core = share(&|(c, _)| c.stability.is_some());
    let loan = |l: &umran_sim::Lexeme| matches!(l.origin, Origin::Borrowed { .. });
    let (consonants, vowels) = variety.inventory();
    let founding: Vec<_> = variety
        .founding_inventory
        .consonants
        .iter()
        .chain(&variety.founding_inventory.vowels)
        .collect();
    let foreign = consonants
        .iter()
        .chain(&vowels)
        .filter(|p| !founding.contains(p))
        .count();
    let people: Vec<f32> = world
        .communities
        .iter()
        .filter(|c| c.variety == v)
        .map(|c| c.name.form.vowel_count() as f32)
        .collect();
    let has = |l: &umran_sim::Lexeme, p: &umran_sim::PhonemeId| l.form.phones().any(|q| q == *p);
    let top = consonants
        .iter()
        .map(|p| share(&|(_, l)| has(l, p)))
        .fold(0.0, f32::max);
    let repeats = |l: &umran_sim::Lexeme| {
        let cs: Vec<_> = l
            .form
            .phones()
            .filter(|p| !CATALOG.get(*p).is_vowel())
            .collect();
        cs.iter().enumerate().any(|(i, a)| cs[i + 1..].contains(a))
    };
    Row {
        top_consonant: top / n,
        repeats: share(&|(_, l)| repeats(l)) / n,
        consonants: consonants.len() as f32,
        vowels: vowels.len() as f32,
        foreign: foreign as f32,
        syllables: words
            .iter()
            .map(|(_, l)| l.form.vowel_count() as f32)
            .sum::<f32>()
            / n,
        long: share(&|(_, l)| l.form.vowel_count() >= 4) / n,
        core_loans: share(&|(c, l)| c.stability.is_some() && loan(l)) / core,
        loans: share(&|(_, l)| loan(l)) / n,
        laws: variety.laws.len() as f32,
        cluster: words
            .iter()
            .map(|(_, l)| runs(&l.form).0)
            .max()
            .unwrap_or(0) as f32,
        clusters: share(&|(_, l)| runs(&l.form).0 >= 3) / n,
        hiatus: share(&|(_, l)| runs(&l.form).1 >= 3) / n,
        language_name: variety.name.form.vowel_count() as f32,
        people_name: people.iter().sum::<f32>() / people.len().max(1) as f32,
    }
}

const SAMPLE: [&str; 12] = [
    "sun", "water", "fire", "stone", "mountain", "river", "eye", "hand", "mother", "king", "god",
    "bread",
];

fn sample(world: &World, seed: u64) {
    println!("\nWorld {seed}");
    let spoken = world.spoken();
    for (v, variety) in world
        .varieties
        .iter()
        .enumerate()
        .filter(|(v, _)| spoken[*v])
    {
        let people: Vec<String> = (0..world.communities.len())
            .filter(|&c| world.communities[c].variety == v)
            .map(|c| world.community_name(c))
            .collect();
        let (consonants, vowels) = variety.inventory();
        let ipa = |ids: &[umran_sim::PhonemeId]| {
            ids.iter()
                .map(|p| CATALOG.get(*p).ipa())
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!(
            "  {} ({}), spoken by {}; laws {}",
            world.language_title(v),
            variety.profile.name,
            people.join(", "),
            variety.laws.len()
        );
        println!("    sounds: {} | {}", ipa(&consonants), ipa(&vowels));
        if std::env::var("LAWS").is_ok() {
            let laws: Vec<String> = variety
                .laws
                .iter()
                .map(|(g, id)| format!("{g}:{id}"))
                .collect();
            println!("    laws: {}", laws.join(" "));
        }
        let words: Vec<String> = SAMPLE
            .iter()
            .filter_map(|id| {
                let concept = umran_sim::concepts::by_id(id)?;
                let word = variety.lexicon.word_for(concept)?;
                let mark = match word.origin {
                    Origin::Borrowed { .. } => "*",
                    Origin::Renewed { .. } => "+",
                    _ => "",
                };
                Some(format!("{id} {}{mark}", variety.spell(&word.form)))
            })
            .collect();
        println!("    {}", words.join(", "));
    }
}

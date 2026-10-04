use std::collections::BTreeMap;
use umran_sim::{CATALOG, Form, Params, SoundProfile, Variety, World};

pub fn max_cluster(form: &Form) -> usize {
    let (mut run, mut longest) = (0, 0);
    for seg in &form.segs {
        if CATALOG.get(seg.phone).is_vowel() {
            run = 0;
        } else {
            run += 1;
            longest = longest.max(run);
        }
    }
    longest
}

#[derive(Default, Debug)]
pub struct Distribution {
    pub words: BTreeMap<usize, usize>,
    pub languages: BTreeMap<usize, usize>,
}

impl Distribution {
    pub fn add(&mut self, variety: &Variety) {
        let mut longest = 0;
        for word in variety.lexicon.living() {
            let cluster = max_cluster(&word.form);
            *self.words.entry(cluster).or_default() += 1;
            longest = longest.max(cluster);
        }
        *self.languages.entry(longest).or_default() += 1;
    }

    pub fn large_words(&self) -> usize {
        self.words.range(5..).map(|(_, count)| count).sum()
    }

    pub fn print(&self, label: &str) {
        println!(
            "{label}: words={} clusters>=5={} word_max={:?} language_max={:?}",
            self.words.values().sum::<usize>(),
            self.large_words(),
            self.words,
            self.languages,
        );
    }
}

pub fn cohort(seeds: u64) -> Vec<(&'static str, Distribution)> {
    ["base", "germanic", "semitic", "finnic", "polynesian"]
        .into_iter()
        .map(|profile| {
            let mut distribution = Distribution::default();
            for seed in 0..seeds {
                let profile = if profile == "base" {
                    SoundProfile::base()
                } else {
                    SoundProfile::by_id(profile).unwrap()
                };
                let mut world = World::solo(seed, &profile, Params::default());
                world.run(160);
                distribution.add(&world.varieties[0]);
            }
            distribution.print(profile);
            (profile, distribution)
        })
        .collect()
}

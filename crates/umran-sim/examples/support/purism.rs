use umran_sim::schisms::SchismCause;
use umran_sim::{ContactKind, Craft, Naming, Params, Revelation, Rise, SoundProfile, World};

/// A controlled literate court with sustained foreign contact. Society is
/// static; ordinary sound/lexical change, classical fixing, and purism remain.
/// The written standard is authored at founding, not claimed as endogenous.
pub fn study(seed: u64) -> World {
    let profiles = SoundProfile::presets();
    let profile = &profiles[seed as usize % profiles.len()];
    let mut world = World::new(
        seed,
        Params {
            purism_rate: Params::default().purism_rate,
            loan_rate: 0.2,
            loan_share: 0.8,
            ..Params::static_society()
        },
    );
    let keepers = world.found(profile, 0.4, 0.8);
    let home = world.communities[keepers].home();
    let foreigners = world.found_seeded(
        &Naming::People,
        &profiles[(seed as usize + 5) % profiles.len()],
        seed + 400,
        0.9,
        0.4,
        Some(home),
        None,
        None,
    );
    world
        .connect(keepers, foreigners, 0.9, ContactKind::Trade)
        .unwrap();
    let state = world.raise_state(keepers, None, Rise::Proclaimed);
    world.states[state].purism = 0.0;
    world.learn(keepers, Craft::Writing, None);
    let religion = world.found_religion(keepers, Revelation::Proclaimed);
    world.religions[religion].scripture = true;
    world.religions[religion].translates = false;
    world.states[state].standard = Some(0);
    world.states[state].standard_speakers = Some(keepers);
    world.run(40);
    world.schism(keepers, SchismCause::Reform).unwrap();
    world.run(120);
    world
}

#[derive(Default, Debug)]
pub struct Summary {
    pub worlds: usize,
    pub episodes: usize,
    pub replacements: usize,
    pub revived: usize,
    pub coined: usize,
}

impl Summary {
    pub fn add(&mut self, world: &World) {
        let mut any = false;
        for variety in &world.varieties {
            for episode in &variety.purism {
                any = true;
                self.episodes += 1;
                self.replacements += episode.replaced.len();
                for word in &episode.replaced {
                    if variety.lexicon.get(word.native).born < episode.since {
                        self.revived += 1;
                    } else {
                        self.coined += 1;
                    }
                }
            }
        }
        self.worlds += usize::from(any);
    }
}

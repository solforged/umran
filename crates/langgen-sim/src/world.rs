use crate::adapt::Adapter;
use crate::concepts::{CONCEPTS, Concept, Field, related};
use crate::form::Form;
use crate::laws::{Law, catalog};
use crate::lexicon::{Entry, Event, LexemeId, Origin};
use crate::phoneme::PhonemeId;
use crate::phonotactics::Phonotactics;
use crate::profile::SoundProfile;
use crate::rng::{key, stream, weighted_index};
use crate::root::mint_one;
use crate::variety::Variety;
use rand::{Rng, RngCore};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Effective Leipzig–Jakarta rank for cultural vocabulary when it comes to
/// internal replacement.
const CULTURAL_RANK: f32 = 100.0;
/// Weight of no law happening when a sound change is due: about as likely
/// as one common law that neither pleases nor offends the culture.
const NO_CHANGE_WEIGHT: f32 = 1.0;
/// Most words competing for one concept at once.
const MAX_VARIANTS: usize = 3;
/// Floor on a loan's usage fitness, however low its donor's prestige.
const MIN_LOAN_FITNESS: f32 = 0.2;
/// Population of a newly founded community.
const INITIAL_SIZE: f32 = 1000.0;
/// Prestige gap below which no community abandons its language.
const SHIFT_MIN_GAP: f32 = 0.2;
/// Contact intensity between the halves of a community that just split.
const FISSION_CONTACT: f32 = 0.5;

/// Rates per generation. Defaults are calibrated so an isolated variety
/// keeps about 84% of its core list per 40 generations.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    /// Chance that a new sound law takes hold.
    pub sound_change_rate: f32,
    /// Chance that a mid-ranked core concept gains a new competing word.
    pub innovation_rate: f32,
    /// How many times more often the least stable core concept (rank 100)
    /// gains competitors than the most stable (rank 1).
    pub stability_spread: f32,
    /// Usage share a new native competitor starts with.
    pub newcomer_share: f32,
    /// Uses of each concept sampled per generation. Fewer speakers means
    /// faster random drift, as in small communities.
    pub speakers: u32,
    /// How strongly sound preferences steer which law happens.
    pub preference_pull: f32,
    /// Share of innovations that are new roots even when a related
    /// concept's word could extend instead.
    pub expressive_share: f32,
    /// Extra innovation hazard for a concept whose dominant word sounds
    /// like another word in use: 1.0 doubles it (homonymic clash).
    pub clash_pressure: f32,
    /// Usage disadvantage of a word that sounds like another word in use.
    pub clash_cost: f32,
    /// Chance per generation that a fully borrowable concept is borrowed
    /// across a contact of intensity 1 by a fully open community of equal
    /// prestige.
    pub loan_rate: f32,
    /// Usage share a loan starts with.
    pub loan_share: f32,
    /// How much a prestige gap multiplies borrowing toward the less
    /// prestigious side: the factor is exp(pull × gap).
    pub prestige_pull: f32,
    /// Usage advantage of a loan per unit of prestige its donor holds over
    /// the recipient.
    pub prestige_selection: f32,
    /// Chance a foreign sound survives in a loan, scaled by contact
    /// intensity and openness (a stand-in for bilingualism).
    pub bilingual_keep: f32,
    /// How strongly contact pulls a variety's sound changes toward its
    /// neighbours' established sounds (areal convergence).
    pub areal_pull: f32,
    /// Population growth per generation for a community far below
    /// `capacity`; growth slows logistically as it fills up.
    pub growth_rate: f32,
    /// Population one territory can support; communities that split share
    /// their founders' territory.
    pub capacity: f32,
    /// Above this population a community may split in two.
    pub split_size: f32,
    /// Chance per generation that a community above `split_size` splits.
    pub fission_rate: f32,
    /// How much relative size adds to prestige: prestige is power plus this
    /// times the log of size over the mean log size.
    pub size_prestige: f32,
    /// Chance per generation, per unit of prestige gap beyond
    /// `SHIFT_MIN_GAP`, that a community under rule or intermarriage
    /// abandons its language for its partner's.
    pub shift_rate: f32,
    /// Chance that a sound the new language has but the shifting community
    /// lacked merges into their nearest own sound (substrate accent).
    pub substrate_merge: f32,
    /// Scales the chance that a shifting community keeps its old word for
    /// a concept, by borrowability and how local the concept is.
    pub substrate_words: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            sound_change_rate: 0.3,
            innovation_rate: 0.003,
            stability_spread: 10.0,
            newcomer_share: 0.25,
            speakers: 12,
            preference_pull: 1.0,
            expressive_share: 0.2,
            clash_pressure: 6.0,
            clash_cost: 0.25,
            loan_rate: 0.05,
            loan_share: 0.3,
            prestige_pull: 2.0,
            prestige_selection: 0.3,
            bilingual_keep: 0.5,
            areal_pull: 3.0,
            growth_rate: 0.03,
            capacity: 8000.0,
            split_size: 4000.0,
            fission_rate: 0.1,
            size_prestige: 0.15,
            shift_rate: 0.02,
            substrate_merge: 0.6,
            substrate_words: 0.5,
        }
    }
}

impl Params {
    /// Defaults with no growth, splitting, or language shift, for studying
    /// one mechanism in a fixed society.
    pub fn static_society() -> Self {
        Self {
            growth_rate: 0.0,
            fission_rate: 0.0,
            shift_rate: 0.0,
            ..Self::default()
        }
    }
}

/// A group of people with a home variety and a few traits.
#[derive(Clone, Debug, PartialEq)]
pub struct Community {
    pub name: String,
    /// Index into `World::varieties`.
    pub variety: usize,
    /// Social standing relative to others, 0–1, recomputed each generation
    /// from `power` and relative size. Loans flow mostly downhill.
    pub prestige: f32,
    /// Authored standing apart from size: wealth, arms, sacred status.
    pub power: f32,
    /// Readiness to accept foreign words, 0–1.
    pub openness: f32,
    /// Population.
    pub size: f32,
    /// The land this community lives on: each founded people brings its
    /// own, and communities that split from it share it.
    pub territory: usize,
}

/// What brings two communities together, which shapes what they borrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContactKind {
    /// Plain proximity: every semantic field equally exposed.
    Neighbours,
    /// Goods, tools, food, and seafaring.
    Trade,
    /// Government, law, and war.
    Rule,
    /// Belief, ritual, and the vocabulary of thought.
    Religion,
    /// Households, kin, and food.
    Intermarriage,
}

impl ContactKind {
    /// Multiplier on borrowing in `field` for this kind of contact. Hand
    /// estimates of where each kind of contact concentrates.
    pub fn affinity(self, field: Field) -> f32 {
        use ContactKind::*;
        use Field::*;
        match (self, field) {
            (Neighbours, _) => 1.0,
            (Trade, Possession) => 3.0,
            (Trade, FoodDrink | ClothingGrooming | BasicActions) => 2.0,
            (Trade, Motion | Animals) => 1.5,
            (Trade, PhysicalWorld) => 1.2,
            (Trade, _) => 0.6,
            (Rule, Social | Law) => 3.0,
            (Rule, Warfare) => 2.5,
            (Rule, Possession) => 1.5,
            (Rule, _) => 0.7,
            (ContactKind::Religion, Field::Religion) => 4.0,
            (ContactKind::Religion, Speech | Emotions | Cognition) => 1.5,
            (ContactKind::Religion, _) => 0.5,
            (Intermarriage, Kinship) => 3.0,
            (Intermarriage, House | FoodDrink) => 2.0,
            (Intermarriage, ClothingGrooming) => 1.5,
            (Intermarriage, _) => 1.0,
        }
    }
}

/// Something that happened to communities rather than to words.
#[derive(Clone, Debug, PartialEq)]
pub enum WorldEvent {
    /// `daughter` split off from `community`, speaking a new variety.
    Split { community: usize, daughter: usize },
    /// `community` abandoned variety `from` for a daughter of `toward`'s.
    Shift {
        community: usize,
        from: usize,
        toward: usize,
        variety: usize,
    },
}

/// Ongoing contact between two communities, in both directions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub a: usize,
    pub b: usize,
    /// 0–1: how much the communities deal with each other.
    pub intensity: f32,
    pub kind: ContactKind,
}

/// Communities, their varieties, and the contacts between them, stepping
/// through generations of about 25 years.
#[derive(Clone, Debug)]
pub struct World {
    pub seed: u64,
    pub generation: u32,
    pub communities: Vec<Community>,
    pub varieties: Vec<Variety>,
    pub contacts: Vec<Contact>,
    pub params: Params,
    /// Splits and shifts, with the generation they happened in.
    pub events: Vec<(u32, WorldEvent)>,
    laws: Vec<Law>,
}

/// Per-variety random streams for one generation.
struct Step {
    seed: u64,
    generation: u32,
    variety: usize,
}

impl Step {
    fn rng(&self, labels: &[u64]) -> ChaCha8Rng {
        let mut keys = vec![
            key("step"),
            u64::from(self.generation),
            key("variety"),
            self.variety as u64,
        ];
        keys.extend_from_slice(labels);
        stream(self.seed, &keys)
    }
}

impl World {
    pub fn new(seed: u64, params: Params) -> Self {
        Self {
            seed,
            generation: 0,
            communities: Vec::new(),
            varieties: Vec::new(),
            contacts: Vec::new(),
            params,
            events: Vec::new(),
            laws: catalog(),
        }
    }

    /// A world with one community and its variety, for studying drift alone.
    pub fn solo(seed: u64, profile: &SoundProfile, params: Params) -> Self {
        let mut world = Self::new(seed, params);
        world.found("Solo", profile, 0.5, 0.5);
        world
    }

    /// Founds a community with a new variety of its own; returns its index.
    /// `power` is its standing apart from size, and its starting prestige.
    pub fn found(
        &mut self,
        name: &str,
        profile: &SoundProfile,
        power: f32,
        openness: f32,
    ) -> usize {
        let index = self.communities.len();
        let variety_seed = stream(self.seed, &[key("found"), index as u64]).next_u64();
        let mut variety = Variety::found(variety_seed, profile);
        variety.name = format!("{name} speech");
        self.varieties.push(variety);
        self.communities.push(Community {
            name: name.into(),
            variety: self.varieties.len() - 1,
            prestige: power.clamp(0.0, 1.0),
            power: power.clamp(0.0, 1.0),
            openness: openness.clamp(0.0, 1.0),
            size: INITIAL_SIZE,
            territory: self
                .communities
                .iter()
                .map(|c| c.territory + 1)
                .max()
                .unwrap_or(0),
        });
        index
    }

    /// Part of `community` moves off as a new community whose speech becomes
    /// a daughter variety; returns the new community's index. The two stay
    /// in contact at `intensity` (0 for none).
    pub fn split(&mut self, community: usize, name: &str, intensity: f32) -> usize {
        let parent = self.communities[community].variety;
        let mut daughter = self.varieties[parent].fork(parent, self.generation);
        daughter.name = format!("{name} speech");
        self.varieties.push(daughter);
        self.communities[community].size /= 2.0;
        let mut new = self.communities[community].clone();
        new.name = name.into();
        new.variety = self.varieties.len() - 1;
        self.communities.push(new);
        let index = self.communities.len() - 1;
        if intensity > 0.0 {
            self.connect(community, index, intensity, ContactKind::Neighbours);
        }
        self.events.push((
            self.generation,
            WorldEvent::Split {
                community,
                daughter: index,
            },
        ));
        index
    }

    /// The oldest variety and word id a word descends from by inheritance.
    /// Two words are cognate when their roots match; a loan is rooted in
    /// the variety that borrowed it, so it is never cognate with its source.
    pub fn root_of(&self, variety: usize, lexeme: LexemeId) -> (usize, LexemeId) {
        let mut at = variety;
        while let Some(fork) = self.varieties[at].parent {
            if lexeme.0 >= fork.inherited {
                break;
            }
            at = fork.variety;
        }
        (at, lexeme)
    }

    /// Whether the dominant words for `concept` in two varieties are
    /// cognate by descent.
    pub fn cognate(&self, a: usize, b: usize, concept: &Concept) -> bool {
        let word = |v: usize| self.varieties[v].lexicon.slot(concept).dominant();
        match (word(a), word(b)) {
            (Some(x), Some(y)) => self.root_of(a, x) == self.root_of(b, y),
            _ => false,
        }
    }

    pub fn connect(&mut self, a: usize, b: usize, intensity: f32, kind: ContactKind) {
        self.contacts.push(Contact {
            a,
            b,
            intensity: intensity.clamp(0.0, 1.0),
            kind,
        });
    }

    /// The home variety of `community`.
    pub fn variety_of(&self, community: usize) -> &Variety {
        &self.varieties[self.communities[community].variety]
    }

    pub fn run(&mut self, generations: u32) {
        for _ in 0..generations {
            self.step();
        }
    }

    pub fn step(&mut self) {
        self.generation += 1;
        let spoken = self.spoken();
        let areal = self.areal_targets();
        for (v, targets) in areal.iter().enumerate() {
            if spoken[v] {
                self.sound_change(v, targets);
            }
        }
        self.borrow();
        let prestige = self.variety_prestige();
        for v in (0..self.varieties.len()).filter(|&v| spoken[v]) {
            let clashes = self.varieties[v].lexicon.clashing();
            self.innovate(v, &clashes);
            self.drift(v, &clashes, &prestige);
            self.retire(v);
        }
        self.grow();
        self.split_large();
        self.shift_languages();
    }

    /// Which varieties some community still speaks. Unspoken varieties are
    /// extinct: they keep their record but no longer change.
    pub fn spoken(&self) -> Vec<bool> {
        let mut out = vec![false; self.varieties.len()];
        for c in &self.communities {
            out[c.variety] = true;
        }
        out
    }

    fn community_rng(&self, community: usize, label: &str) -> ChaCha8Rng {
        stream(
            self.seed,
            &[
                key("step"),
                u64::from(self.generation),
                key("community"),
                community as u64,
                key(label),
            ],
        )
    }

    /// Logistic growth against each territory's capacity, with a little
    /// noise; then prestige from power plus relative size.
    fn grow(&mut self) {
        let mut occupied: HashMap<usize, f32> = HashMap::new();
        for c in &self.communities {
            *occupied.entry(c.territory).or_default() += c.size;
        }
        for c in 0..self.communities.len() {
            let mut rng = self.community_rng(c, "grow");
            let noise = rng.gen_range(-0.05..0.05);
            let community = &mut self.communities[c];
            let room = 1.0 - occupied[&community.territory] / self.params.capacity;
            community.size *= (self.params.growth_rate * room + noise).exp();
        }
        let n = self.communities.len() as f32;
        let mean = self.communities.iter().map(|c| c.size.ln()).sum::<f32>() / n;
        for community in &mut self.communities {
            let relative = self.params.size_prestige * (community.size.ln() - mean);
            community.prestige = (community.power + relative).clamp(0.0, 1.0);
        }
    }

    /// Large communities sometimes split; the new half speaks a daughter
    /// variety and stays in moderate contact with the old.
    fn split_large(&mut self) {
        for c in 0..self.communities.len() {
            if self.communities[c].size <= self.params.split_size {
                continue;
            }
            let mut rng = self.community_rng(c, "fission");
            if rng.r#gen::<f32>() < self.params.fission_rate {
                let name = self.daughter_name(c);
                self.split(c, &name, FISSION_CONTACT);
            }
        }
    }

    /// "Hill 2", "Hill 3", ...: the founding name plus a count of the
    /// communities already bearing it.
    fn daughter_name(&self, community: usize) -> String {
        let base = self.communities[community]
            .name
            .split(' ')
            .next()
            .unwrap_or_default()
            .to_string();
        let taken = self
            .communities
            .iter()
            .filter(|c| c.name.split(' ').next() == Some(base.as_str()))
            .count();
        format!("{base} {}", taken + 1)
    }

    /// The variety a lineage starts from: its ultimate ancestor.
    pub fn family(&self, variety: usize) -> usize {
        let mut at = variety;
        while let Some(fork) = self.varieties[at].parent {
            at = fork.variety;
        }
        at
    }

    /// Under rule or intermarriage, a community far below its partner in
    /// prestige may abandon its language for the partner's, unless it
    /// already speaks a language of the same family.
    fn shift_languages(&mut self) {
        for c in 0..self.communities.len() {
            let mut options: Vec<(usize, f32)> = Vec::new();
            for contact in &self.contacts {
                let other = match (contact.a == c, contact.b == c) {
                    (true, _) => contact.b,
                    (_, true) => contact.a,
                    _ => continue,
                };
                let factor = match contact.kind {
                    ContactKind::Rule => 2.0,
                    ContactKind::Intermarriage => 1.5,
                    _ => continue,
                };
                let (me, them) = (&self.communities[c], &self.communities[other]);
                let gap = them.prestige - me.prestige - SHIFT_MIN_GAP;
                if self.family(me.variety) != self.family(them.variety) && gap > 0.0 {
                    options.push((
                        other,
                        self.params.shift_rate * contact.intensity * gap * factor,
                    ));
                }
            }
            let total: f32 = options.iter().map(|(_, h)| h).sum();
            let mut rng = self.community_rng(c, "shift");
            if total > 0.0 && rng.r#gen::<f32>() < total {
                let target = options[weighted_index(&mut rng, options.iter().map(|(_, h)| *h))].0;
                self.shift(c, target);
            }
        }
    }

    /// `community` adopts `target`'s language, imperfectly: it speaks a new
    /// daughter of that variety, keeps its own sound preferences, merges
    /// sounds it never had into its nearest own ones (each with probability
    /// `substrate_merge`), and keeps some old words, mostly about the local
    /// world. Returns the new variety's index. The old variety goes extinct
    /// if no one else speaks it.
    pub fn shift(&mut self, community: usize, target: usize) -> usize {
        let generation = self.generation;
        let old = self.communities[community].variety;
        let source = self.communities[target].variety;
        let old_sounds = self.varieties[old].established();
        let mut new = self.varieties[source].fork(source, generation);
        new.profile = self.varieties[old].profile.clone();
        new.name = format!(
            "{} {}",
            self.communities[community].name, self.varieties[source].name
        );
        let mut rng = self.community_rng(community, "substrate");

        let mut foreign: Vec<PhonemeId> = new
            .established()
            .into_iter()
            .filter(|p| !old_sounds.contains(p))
            .collect();
        foreign.sort_by_key(|p| p.0);
        for sound in foreign {
            let nearest = old_sounds
                .iter()
                .copied()
                .filter(|o| crate::adapt::distance(sound, *o).is_finite())
                .min_by(|a, b| {
                    crate::adapt::distance(sound, *a)
                        .total_cmp(&crate::adapt::distance(sound, *b))
                        .then(a.0.cmp(&b.0))
                });
            let Some(nearest) = nearest else { continue };
            if rng.r#gen::<f32>() >= self.params.substrate_merge {
                continue;
            }
            let merge = crate::change::SoundChange {
                id: "substrate".into(),
                target: crate::change::Matcher::Phone(sound),
                result: crate::change::Rewrite::Phone(nearest),
                left: crate::change::Env::Any,
                right: crate::change::Env::Any,
            };
            for lexeme in new
                .lexicon
                .lexemes
                .iter_mut()
                .filter(|l| l.obsolete.is_none())
            {
                let after = merge.apply(&lexeme.form);
                if after != lexeme.form {
                    let before = std::mem::replace(&mut lexeme.form, after);
                    lexeme.log.push(Entry {
                        generation,
                        event: Event::SoundLaw {
                            law: "substrate",
                            before,
                        },
                    });
                }
            }
            new.laws.push((generation, "substrate"));
        }

        let new_index = self.varieties.len();
        for (i, concept) in CONCEPTS.iter().enumerate() {
            let local = match concept.field {
                Field::PhysicalWorld | Field::Animals | Field::Agriculture | Field::FoodDrink => {
                    2.0
                }
                _ => 1.0,
            };
            let keep = (self.params.substrate_words * concept.borrowability() * local).min(0.9);
            if rng.r#gen::<f32>() >= keep || new.lexicon.slots[i].variants.len() >= MAX_VARIANTS {
                continue;
            }
            let Some(word) = self.varieties[old].lexicon.slots[i].dominant() else {
                continue;
            };
            let form = self.varieties[old].lexicon.get(word).form.clone();
            let origin = Origin::Borrowed {
                from: old,
                source: word,
            };
            let id = new.lexicon.coin(form.clone(), origin, concept, generation);
            new.lexicon.get_mut(id).log.push(Entry {
                generation,
                event: Event::Borrowed {
                    from: old,
                    source: form,
                },
            });
            new.lexicon.slots[i].introduce(id, self.params.loan_share);
        }
        self.varieties.push(new);
        self.communities[community].variety = new_index;
        self.events.push((
            generation,
            WorldEvent::Shift {
                community,
                from: old,
                toward: target,
                variety: new_index,
            },
        ));
        new_index
    }

    fn at(&self, variety: usize) -> Step {
        Step {
            seed: self.seed,
            generation: self.generation,
            variety,
        }
    }

    /// For each variety, the established sounds of the varieties it is in
    /// contact with, weighted by intensity and by the partner's share of the
    /// pair's prestige (1 for equals, toward 2 for a dominant partner).
    fn areal_targets(&self) -> Vec<Vec<(HashSet<PhonemeId>, f32)>> {
        let established: Vec<HashSet<PhonemeId>> =
            self.varieties.iter().map(Variety::established).collect();
        let mut out = vec![Vec::new(); self.varieties.len()];
        for contact in &self.contacts {
            for (me, other) in [(contact.a, contact.b), (contact.b, contact.a)] {
                let (m, o) = (&self.communities[me], &self.communities[other]);
                if m.variety == o.variety {
                    continue;
                }
                let total = (m.prestige + o.prestige).max(f32::EPSILON);
                let weight = contact.intensity * 2.0 * o.prestige / total;
                out[m.variety].push((established[o.variety].clone(), weight));
            }
        }
        out
    }

    /// Prestige of each variety: the highest among communities at home in it.
    fn variety_prestige(&self) -> Vec<f32> {
        let mut out = vec![0.0_f32; self.varieties.len()];
        for c in &self.communities {
            out[c.variety] = out[c.variety].max(c.prestige);
        }
        out
    }

    /// Per-generation chance that `concept` gains a native competitor.
    pub fn innovation_hazard(&self, concept: &Concept) -> f32 {
        let rank = concept.stability.map_or(CULTURAL_RANK, f32::from);
        self.params.innovation_rate * self.params.stability_spread.powf((rank - 50.5) / 99.0)
    }

    /// Maybe applies one new sound law to every living word. Laws that
    /// would change nothing are skipped; laws that move sounds toward the
    /// culture's preferences, or toward the sounds of its contacts, are
    /// likelier.
    fn sound_change(&mut self, v: usize, areal: &[(HashSet<PhonemeId>, f32)]) {
        let mut rng = self.at(v).rng(&[key("sound")]);
        if rng.r#gen::<f32>() >= self.params.sound_change_rate {
            return;
        }
        let variety = &self.varieties[v];
        let applied: HashSet<&str> = variety.laws.iter().map(|(_, id)| *id).collect();
        let prior = &variety.profile.inventory;
        let candidates: Vec<(&Law, f32)> = self
            .laws
            .iter()
            .filter(|law| !applied.contains(law.id))
            .filter_map(|law| {
                let a = law.assess(variety.lexicon.living().map(|l| &l.form), prior)?;
                let areal: f32 = areal.iter().map(|(target, w)| w * a.toward(target)).sum();
                let bias = (self.params.preference_pull * a.pull.clamp(-5.0, 3.0)
                    + self.params.areal_pull * areal)
                    .exp();
                Some((law, law.commonness * bias))
            })
            .collect();
        // "Nothing happens" competes like a common, neutral law, so a
        // culture is not forced into a change it strongly disprefers.
        let weights = candidates.iter().map(|(_, w)| *w).chain([NO_CHANGE_WEIGHT]);
        let Some((law, _)) = candidates.get(weighted_index(&mut rng, weights)) else {
            return;
        };
        let law = (*law).clone();

        let generation = self.generation;
        let variety = &mut self.varieties[v];
        for lexeme in &mut variety.lexicon.lexemes {
            if lexeme.obsolete.is_some() {
                continue;
            }
            let after = law.apply(&lexeme.form);
            if after != lexeme.form {
                let before = std::mem::replace(&mut lexeme.form, after);
                lexeme.log.push(Entry {
                    generation,
                    event: Event::SoundLaw {
                        law: law.id,
                        before,
                    },
                });
            }
        }
        variety.laws.push((generation, law.id));
    }

    /// Each contact carries words both ways, mostly from the more
    /// prestigious side. A concept's chance of being borrowed scales with
    /// intensity, the recipient's openness, the prestige gap, the contact's
    /// affinity for its field, and its own borrowability. The donor's
    /// current word is adapted to the recipient's sounds and enters as a
    /// competitor; all loans are decided against this generation's state.
    fn borrow(&mut self) {
        struct Loan {
            recipient: usize,
            concept: usize,
            from: usize,
            source: LexemeId,
            source_form: Form,
            form: Form,
        }
        let mut loans = Vec::new();
        let mut adapters: Vec<Option<Adapter>> = vec![None; self.varieties.len()];
        for contact in &self.contacts {
            for (donor, recipient) in [(contact.a, contact.b), (contact.b, contact.a)] {
                let (d, r) = (&self.communities[donor], &self.communities[recipient]);
                if d.variety == r.variety {
                    continue;
                }
                let base = self.params.loan_rate
                    * contact.intensity
                    * r.openness
                    * (self.params.prestige_pull * (d.prestige - r.prestige)).exp();
                let keep_foreign = self.params.bilingual_keep * contact.intensity * r.openness;
                let donor_lexicon = &self.varieties[d.variety].lexicon;
                let recipient_lexicon = &self.varieties[r.variety].lexicon;
                for (i, concept) in CONCEPTS.iter().enumerate() {
                    let hazard =
                        base * contact.kind.affinity(concept.field) * concept.borrowability();
                    let mut rng =
                        self.at(r.variety)
                            .rng(&[key("borrow"), d.variety as u64, key(concept.id)]);
                    if rng.r#gen::<f32>() >= hazard {
                        continue;
                    }
                    let Some(source) = donor_lexicon.slots[i].dominant() else {
                        continue;
                    };
                    let already = recipient_lexicon.slots[i].variants.iter().any(|v| {
                        recipient_lexicon.get(v.lexeme).origin
                            == Origin::Borrowed {
                                from: d.variety,
                                source,
                            }
                    });
                    if already {
                        continue;
                    }
                    let adapter = adapters[r.variety].get_or_insert_with(|| {
                        Adapter::new(
                            recipient_lexicon.living().map(|l| &l.form),
                            &self.varieties[r.variety].profile.inventory,
                        )
                    });
                    let source_form = donor_lexicon.get(source).form.clone();
                    let form = adapter.adapt(&source_form, keep_foreign, &mut rng);
                    loans.push(Loan {
                        recipient: r.variety,
                        concept: i,
                        from: d.variety,
                        source,
                        source_form,
                        form,
                    });
                }
            }
        }
        let generation = self.generation;
        for loan in loans {
            let lexicon = &mut self.varieties[loan.recipient].lexicon;
            if lexicon.slots[loan.concept].variants.len() >= MAX_VARIANTS {
                continue;
            }
            let concept = &CONCEPTS[loan.concept];
            let origin = Origin::Borrowed {
                from: loan.from,
                source: loan.source,
            };
            // A donor word already borrowed for another sense is the same
            // loanword gaining a meaning, not a second borrowing.
            let existing = lexicon.living().find(|l| l.origin == origin).map(|l| l.id);
            let id = match existing {
                Some(id) => {
                    lexicon.get_mut(id).log.push(Entry {
                        generation,
                        event: Event::Extended { to: concept },
                    });
                    id
                }
                None => {
                    let id = lexicon.coin(loan.form, origin, concept, generation);
                    lexicon.get_mut(id).log.push(Entry {
                        generation,
                        event: Event::Borrowed {
                            from: loan.from,
                            source: loan.source_form,
                        },
                    });
                    id
                }
            };
            lexicon.slots[loan.concept].introduce(id, self.params.loan_share);
        }
    }

    /// Concepts gain native competitors: usually a word for a related
    /// concept extends to cover it (sun > day, see > know), otherwise a new
    /// root is coined from the language's current sounds, avoiding forms
    /// its semantic field already uses.
    fn innovate(&mut self, v: usize, clashes: &HashSet<LexemeId>) {
        let generation = self.generation;
        let step = self.at(v);
        let hazards: Vec<f32> = CONCEPTS.iter().map(|c| self.innovation_hazard(c)).collect();
        let params = &self.params;
        let lexicon = &mut self.varieties[v].lexicon;
        let observed = Phonotactics::observe(lexicon.living().map(|l| &l.form));
        for (i, concept) in CONCEPTS.iter().enumerate() {
            let mut rng = step.rng(&[key("innovate"), key(concept.id)]);
            let clash = lexicon.slots[i]
                .dominant()
                .is_some_and(|id| clashes.contains(&id));
            let pressure = if clash {
                1.0 + params.clash_pressure
            } else {
                1.0
            };
            if rng.r#gen::<f32>() >= hazards[i] * pressure {
                continue;
            }
            if lexicon.slots[i].variants.len() >= MAX_VARIANTS {
                continue;
            }
            let mut donors: Vec<LexemeId> = related(concept)
                .filter_map(|other| lexicon.slot(other).dominant())
                .filter(|id| !lexicon.slots[i].has(*id))
                .collect();
            donors.sort();
            donors.dedup();

            let newcomer = if !donors.is_empty() && rng.r#gen::<f32>() >= params.expressive_share {
                let id = donors[rng.gen_range(0..donors.len())];
                lexicon.get_mut(id).log.push(Entry {
                    generation,
                    event: Event::Extended { to: concept },
                });
                id
            } else {
                let used: HashSet<Form> = lexicon.living().map(|l| l.form.clone()).collect();
                let field: HashSet<Form> = lexicon
                    .slots
                    .iter()
                    .filter(|s| s.concept.field == concept.field)
                    .flat_map(|s| s.variants.iter())
                    .map(|v| lexicon.get(v.lexeme).form.clone())
                    .collect();
                let form = mint_one(&mut rng, &observed, concept, &used, &field);
                lexicon.coin(form, Origin::Expressive, concept, generation)
            };
            lexicon.slots[i].introduce(newcomer, params.newcomer_share);
        }
    }

    /// Usage shares drift by resampling each contested concept's uses.
    /// Words that sound like other words in use pay a small cost; loans
    /// gain or lose by their donor's prestige relative to the recipient's.
    /// A word that draws no uses loses that concept.
    fn drift(&mut self, v: usize, clashes: &HashSet<LexemeId>, prestige: &[f32]) {
        let generation = self.generation;
        let step = self.at(v);
        let params = &self.params;
        let n = params.speakers.max(1);
        let own = prestige[v];
        let lexicon = &mut self.varieties[v].lexicon;
        let fitness: Vec<f32> = lexicon
            .lexemes
            .iter()
            .map(|l| {
                let clash = if clashes.contains(&l.id) {
                    1.0 - params.clash_cost
                } else {
                    1.0
                };
                let loan = match l.origin {
                    Origin::Borrowed { from, .. } => (1.0
                        + params.prestige_selection * (prestige[from] - own))
                        .max(MIN_LOAN_FITNESS),
                    _ => 1.0,
                };
                clash * loan
            })
            .collect();
        for (i, concept) in CONCEPTS.iter().enumerate() {
            if lexicon.slots[i].variants.len() < 2 {
                continue;
            }
            let mut rng = step.rng(&[key("drift"), key(concept.id)]);
            let slot = &mut lexicon.slots[i];
            let mut counts = vec![0u32; slot.variants.len()];
            for _ in 0..n {
                let weights = slot
                    .variants
                    .iter()
                    .map(|var| var.weight * fitness[var.lexeme.0 as usize]);
                counts[weighted_index(&mut rng, weights)] += 1;
            }
            for (variant, count) in slot.variants.iter_mut().zip(counts) {
                variant.weight = count as f32 / n as f32;
            }
            let lost: Vec<LexemeId> = slot
                .variants
                .iter()
                .filter(|var| var.weight == 0.0)
                .map(|var| var.lexeme)
                .collect();
            slot.variants.retain(|var| var.weight > 0.0);
            for id in lost {
                lexicon.get_mut(id).log.push(Entry {
                    generation,
                    event: Event::Lost { sense: concept },
                });
            }
        }
    }

    /// Words no longer used for any concept become obsolete. They keep
    /// their history but stop undergoing sound change.
    fn retire(&mut self, v: usize) {
        let generation = self.generation;
        let lexicon = &mut self.varieties[v].lexicon;
        let used: HashSet<LexemeId> = lexicon
            .slots
            .iter()
            .flat_map(|s| s.variants.iter().map(|var| var.lexeme))
            .collect();
        for lexeme in &mut lexicon.lexemes {
            if lexeme.obsolete.is_none() && !used.contains(&lexeme.id) {
                lexeme.obsolete = Some(generation);
                lexeme.log.push(Entry {
                    generation,
                    event: Event::Obsolete,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_are_reproducible() {
        let profile = SoundProfile::by_id("elvish").unwrap();
        let mut a = World::solo(7, &profile, Params::default());
        let mut b = World::solo(7, &profile, Params::default());
        a.run(40);
        b.run(40);
        assert_eq!(a.varieties[0].lexicon, b.varieties[0].lexicon);
        assert_eq!(a.varieties[0].laws, b.varieties[0].laws);
    }

    #[test]
    fn slots_stay_normalized_and_words_keep_a_vowel() {
        let profile = SoundProfile::by_id("kuo-toa").unwrap();
        let mut sim = World::solo(3, &profile, Params::default());
        sim.run(80);
        for slot in &sim.varieties[0].lexicon.slots {
            let total: f32 = slot.variants.iter().map(|v| v.weight).sum();
            assert!(
                (total - 1.0).abs() < 1e-4,
                "{} sums to {total}",
                slot.concept.id
            );
        }
        for lexeme in sim.varieties[0].lexicon.living() {
            assert!(
                lexeme.form.vowel_count() > 0,
                "{} lost its vowels",
                lexeme.form.ipa()
            );
        }
    }

    /// Glottochronology's classic estimate is roughly 80–90% retention of
    /// the 100-item core list per millennium (about 40 generations), with
    /// the most stable meanings kept far more often than the least.
    #[test]
    fn core_retention_per_millennium_matches_glottochronology() {
        let seeds = 60;
        let (mut total, mut top, mut bottom) = (0.0, 0.0, 0.0);
        for (n, profile) in SoundProfile::examples()
            .iter()
            .cycle()
            .take(seeds)
            .enumerate()
        {
            let mut sim = World::solo(n as u64, profile, Params::default());
            sim.run(40);
            let lexicon = &sim.varieties[0].lexicon;
            total += lexicon.core_retention();
            let kept = |lo: u8, hi: u8| {
                lexicon
                    .slots
                    .iter()
                    .filter(|s| s.concept.stability.is_some_and(|r| (lo..=hi).contains(&r)))
                    .filter(|s| lexicon.keeps_founding_word(s.concept))
                    .count() as f32
                    / 20.0
            };
            top += kept(1, 20);
            bottom += kept(81, 100);
        }
        let (mean, top, bottom) = (
            total / seeds as f32,
            top / seeds as f32,
            bottom / seeds as f32,
        );
        assert!((0.80..=0.90).contains(&mean), "mean retention {mean:.3}");
        assert!(
            top > bottom + 0.1,
            "ranks 1–20 kept {top:.2}, ranks 81–100 {bottom:.2}"
        );
    }

    /// Sound preferences steer which laws a language undergoes: a culture
    /// that likes fricatives spirantizes far more often, and one that
    /// dislikes labiodental fricatives rarely turns w into v.
    #[test]
    fn preferences_steer_sound_change() {
        let rate = |profile: &SoundProfile, law: &str| {
            let seeds = 120;
            let hits = (0..seeds)
                .filter(|&seed| {
                    let mut sim = World::solo(seed, profile, Params::default());
                    sim.run(40);
                    sim.varieties[0].laws.iter().any(|(_, id)| *id == law)
                })
                .count();
            hits as f32 / seeds as f32
        };
        let neutral = SoundProfile::by_id("neutral").unwrap();
        let illithid = SoundProfile::by_id("illithid").unwrap();
        let fishy = neutral.flavored(&crate::flavor::Flavor::by_id("fish-mouthed").unwrap());
        let (spir_neutral, spir_illithid) = (
            rate(&neutral, "spirantization"),
            rate(&illithid, "spirantization"),
        );
        assert!(
            spir_illithid > 3.0 * spir_neutral + 0.2,
            "{spir_illithid} vs {spir_neutral}"
        );
        let (w_neutral, w_fishy) = (rate(&neutral, "w-fortition"), rate(&fishy, "w-fortition"));
        assert!(w_fishy < 0.5 * w_neutral, "{w_fishy} vs {w_neutral}");
    }

    /// A prestigious donor and an open recipient in contact for 40
    /// generations.
    fn pair(seed: u64, kind: ContactKind) -> World {
        let mut world = World::new(seed, Params::static_society());
        let d = world.found("Donor", &SoundProfile::by_id("illithid").unwrap(), 0.8, 0.4);
        let r = world.found(
            "Recipient",
            &SoundProfile::by_id("kuo-toa").unwrap(),
            0.3,
            0.7,
        );
        world.connect(d, r, 0.8, kind);
        world.run(40);
        world
    }

    fn is_loan(world: &World, community: usize, concept: &str) -> bool {
        let lexicon = &world.variety_of(community).lexicon;
        let concept = crate::concepts::by_id(concept).unwrap();
        lexicon
            .word_for(concept)
            .is_some_and(|l| matches!(l.origin, Origin::Borrowed { .. }))
    }

    /// Mean share of each field's concepts that the recipient now
    /// expresses with a loan.
    fn field_shares(worlds: &[World]) -> Vec<(Field, f32)> {
        let mut out: Vec<(Field, f32)> = Vec::new();
        for &(field, _) in crate::wold::BORROWED_SCORE {
            let ids: Vec<&str> = CONCEPTS
                .iter()
                .filter(|c| c.field == field)
                .map(|c| c.id)
                .collect();
            let loans: usize = worlds
                .iter()
                .map(|w| ids.iter().filter(|id| is_loan(w, 1, id)).count())
                .sum();
            out.push((field, loans as f32 / (ids.len() * worlds.len()) as f32));
        }
        out
    }

    /// The milestone check: with no field favoured by the kind of contact,
    /// loan shares by field rank roughly as WOLD's do. Borrowability is set
    /// per concept, so this ranking has to emerge from what each field
    /// contains.
    #[test]
    fn loan_shares_by_field_rank_like_wold() {
        let worlds: Vec<World> = (0..40).map(|s| pair(s, ContactKind::Neighbours)).collect();
        let shares = field_shares(&worlds);
        let pairs: Vec<(f32, f32)> = shares
            .iter()
            .zip(crate::wold::BORROWED_SCORE)
            .map(|((_, sim), (_, wold))| (*sim, *wold))
            .collect();
        let rho = crate::wold::spearman(&pairs);
        assert!(rho >= 0.5, "Spearman {rho:.2} against WOLD: {shares:?}");
        let overall: f32 = shares.iter().map(|(_, s)| s).sum::<f32>() / shares.len() as f32;
        assert!(
            (0.1..=0.45).contains(&overall),
            "overall field share {overall:.2}"
        );
    }

    #[test]
    fn core_vocabulary_stays_mostly_native() {
        let worlds: Vec<World> = (0..40).map(|s| pair(s, ContactKind::Neighbours)).collect();
        let pronouns = worlds
            .iter()
            .flat_map(|w| ["1sg", "2sg", "3sg"].map(|p| is_loan(w, 1, p)))
            .filter(|&loan| loan)
            .count() as f32
            / (3 * worlds.len()) as f32;
        assert!(pronouns < 0.05, "pronouns borrowed {pronouns:.3}");
        let shares = field_shares(&worlds);
        let share = |f: Field| shares.iter().find(|(x, _)| *x == f).unwrap().1;
        assert!(
            share(Field::Body) < 0.5 * share(Field::Religion),
            "{shares:?}"
        );
    }

    #[test]
    fn loans_flow_toward_the_less_prestigious_side() {
        let (mut down, mut up) = (0, 0);
        for seed in 0..20 {
            let world = pair(seed, ContactKind::Neighbours);
            let count = |c: usize| CONCEPTS.iter().filter(|k| is_loan(&world, c, k.id)).count();
            down += count(1);
            up += count(0);
        }
        assert!(
            down > 5 * up.max(1),
            "recipient took {down}, donor took {up}"
        );
    }

    #[test]
    fn contact_kinds_concentrate_borrowing() {
        let share = |kind: ContactKind, field: Field| {
            let worlds: Vec<World> = (0..20).map(|s| pair(s, kind)).collect();
            field_shares(&worlds)
                .into_iter()
                .find(|(f, _)| *f == field)
                .unwrap()
                .1
        };
        assert!(
            share(ContactKind::Religion, Field::Religion)
                > share(ContactKind::Neighbours, Field::Religion) + 0.1
        );
        assert!(
            share(ContactKind::Trade, Field::Possession)
                > share(ContactKind::Trade, Field::Body) + 0.2
        );
    }

    #[test]
    fn loans_record_their_source_and_later_history() {
        let world = pair(5, ContactKind::Neighbours);
        let recipient = world.variety_of(1);
        let donor = world.communities[0].variety;
        let loans: Vec<_> = recipient
            .lexicon
            .lexemes
            .iter()
            .filter(|l| matches!(l.origin, Origin::Borrowed { .. }))
            .collect();
        assert!(!loans.is_empty());
        for loan in loans {
            let Origin::Borrowed { from, source } = loan.origin else {
                unreachable!()
            };
            assert_eq!(from, donor);
            assert!(world.varieties[from].lexicon.lexemes.len() > source.0 as usize);
            assert!(
                matches!(loan.log.first().map(|e| &e.event), Some(Event::Borrowed { from: f, .. }) if *f == donor),
                "a loan's history starts with its borrowing"
            );
            // Sound laws can only follow the borrowing, never precede it.
            for entry in &loan.log {
                assert!(entry.generation >= loan.born);
            }
        }
        // One donor word is borrowed once, however many senses it brings.
        let mut sources: Vec<_> = recipient
            .lexicon
            .living()
            .filter_map(|l| match l.origin {
                Origin::Borrowed { from, source } => Some((from, source)),
                _ => None,
            })
            .collect();
        let n = sources.len();
        sources.sort();
        sources.dedup();
        assert_eq!(sources.len(), n);
    }

    /// The milestone check: given only two daughters' word lists, the
    /// comparative method recovers the regular correspondences that true
    /// cognates show and flags most loans from an unrelated language.
    #[test]
    fn comparative_method_recovers_correspondences_and_flags_loans() {
        use crate::compare::{Settings, compare, regular_correspondences};
        let profiles = SoundProfile::examples();
        let concepts: Vec<&'static Concept> = CONCEPTS.iter().collect();
        let (mut kept, mut cognates, mut loans_flagged, mut loans) = (0, 0, 0, 0);
        let (mut recovered, mut true_total, mut correct, mut found_total) = (0, 0, 0, 0);
        for seed in 0..30u64 {
            let mut world = World::new(seed, Params::static_society());
            let west = world.found("West", &profiles[seed as usize % 4], 0.5, 0.5);
            let outsiders = world.found("Outsiders", &profiles[(seed as usize + 2) % 4], 0.9, 0.3);
            world.run(5);
            let east = world.split(west, "East", 0.0);
            world.connect(outsiders, east, 0.8, ContactKind::Rule);
            world.run(40);
            let (a, b) = (
                world.communities[west].variety,
                world.communities[east].variety,
            );
            let out = world.communities[outsiders].variety;
            let result = compare(
                &world.varieties[a].lexicon,
                &world.varieties[b].lexicon,
                &concepts,
            );
            let truly: Vec<bool> = result
                .rows
                .iter()
                .map(|r| world.cognate(a, b, r.concept))
                .collect();
            let changed =
                |list: Vec<(crate::compare::Pair, u32)>| -> HashSet<crate::compare::Pair> {
                    list.into_iter()
                        .map(|(p, _)| p)
                        .filter(|p| p.0 != p.1)
                        .collect()
                };
            let truth = changed(regular_correspondences(
                result
                    .rows
                    .iter()
                    .zip(&truly)
                    .filter(|(_, t)| **t)
                    .map(|(r, _)| r.alignment.as_slice()),
                &Settings::default(),
            ));
            let found = changed(result.regular.clone());
            recovered += truth.intersection(&found).count();
            true_total += truth.len();
            correct += found.intersection(&truth).count();
            found_total += found.len();
            for (row, &cognate) in result.rows.iter().zip(&truly) {
                if cognate {
                    cognates += 1;
                    kept += usize::from(row.cognate);
                }
                let loan = world.varieties[b]
                    .lexicon
                    .word_for(row.concept)
                    .is_some_and(
                        |l| matches!(l.origin, Origin::Borrowed { from, .. } if from == out),
                    );
                if loan {
                    loans += 1;
                    loans_flagged += usize::from(!row.cognate);
                }
            }
        }
        let rate = |x: usize, n: usize| x as f32 / n.max(1) as f32;
        assert!(
            rate(recovered, true_total) >= 0.8,
            "recall {recovered}/{true_total}"
        );
        assert!(
            rate(correct, found_total) >= 0.8,
            "precision {correct}/{found_total}"
        );
        assert!(
            rate(loans_flagged, loans) >= 0.85,
            "loans flagged {loans_flagged}/{loans}"
        );
        assert!(
            rate(kept, cognates) >= 0.85,
            "cognates kept {kept}/{cognates}"
        );
    }

    #[test]
    fn daughters_share_inherited_words_until_they_diverge() {
        let mut world = World::solo(
            9,
            &SoundProfile::by_id("neutral").unwrap(),
            Params::default(),
        );
        world.run(3);
        let east = world.split(0, "East", 0.0);
        let (a, b) = (
            world.communities[0].variety,
            world.communities[east].variety,
        );
        assert!(
            CONCEPTS.iter().all(|c| world.cognate(a, b, c)),
            "identical at the split"
        );
        assert_eq!(world.varieties[b].parent.map(|f| f.variety), Some(a));
        world.run(40);
        let shared = CONCEPTS.iter().filter(|c| world.cognate(a, b, c)).count();
        assert!(
            shared < CONCEPTS.len(),
            "independent change after the split"
        );
        assert!(
            shared > CONCEPTS.len() / 2,
            "still mostly cognate after a millennium"
        );
        assert_ne!(world.varieties[a].laws, world.varieties[b].laws);
    }

    /// Long, intense contact without borrowing makes two unrelated sound
    /// systems more alike than the same contact without areal pull.
    #[test]
    fn contact_pulls_sound_systems_together() {
        let overlap = |areal_pull: f32| {
            let pairs = [
                ("neutral", "elvish"),
                ("illithid", "kuo-toa"),
                ("neutral", "illithid"),
            ];
            let mut total = 0.0;
            for seed in 0..30u64 {
                let (x, y) = pairs[seed as usize % 3];
                let params = Params {
                    loan_rate: 0.0,
                    areal_pull,
                    ..Params::static_society()
                };
                let mut world = World::new(seed, params);
                let a = world.found("A", &SoundProfile::by_id(x).unwrap(), 0.5, 0.5);
                let b = world.found("B", &SoundProfile::by_id(y).unwrap(), 0.5, 0.5);
                world.connect(a, b, 1.0, ContactKind::Neighbours);
                world.run(80);
                let (sa, sb) = (
                    world.variety_of(a).established(),
                    world.variety_of(b).established(),
                );
                total += sa.intersection(&sb).count() as f32 / sa.union(&sb).count() as f32;
            }
            total / 30.0
        };
        let (without, with) = (overlap(0.0), overlap(Params::default().areal_pull));
        assert!(
            with > without + 0.03,
            "overlap {with:.3} with areal pull, {without:.3} without"
        );
    }

    #[test]
    fn communities_grow_split_and_rank_by_size() {
        let mut world = World::solo(
            4,
            &SoundProfile::by_id("neutral").unwrap(),
            Params::default(),
        );
        world.run(200);
        assert!(
            world.communities.len() > 1,
            "a growing community eventually splits"
        );
        for (i, c) in world.communities.iter().enumerate().skip(1) {
            let fork = world.varieties[c.variety]
                .parent
                .expect("split communities speak daughters");
            assert!(fork.generation > 0, "community {i}");
        }
        // With equal power, the larger community has at least the prestige.
        let mut by_size = world.communities.clone();
        by_size.sort_by(|a, b| a.size.total_cmp(&b.size));
        assert!(by_size.last().unwrap().prestige >= by_size[0].prestige);
    }

    fn conquest(seed: u64, params: Params) -> (World, usize, usize) {
        let mut world = World::new(seed, params);
        let rulers = world.found(
            "Rulers",
            &SoundProfile::by_id("illithid").unwrap(),
            0.9,
            0.3,
        );
        let subjects = world.found(
            "Subjects",
            &SoundProfile::by_id("kuo-toa").unwrap(),
            0.2,
            0.6,
        );
        world.connect(rulers, subjects, 1.0, ContactKind::Rule);
        (world, rulers, subjects)
    }

    #[test]
    fn shifting_communities_keep_an_accent_and_some_old_words() {
        let (mut world, rulers, subjects) = conquest(3, Params::static_society());
        world.run(10);
        let old = world.communities[subjects].variety;
        let ruling = world.communities[rulers].variety;
        let new = world.shift(subjects, rulers);
        let shifted = &world.varieties[new];
        assert_eq!(shifted.parent.map(|f| f.variety), Some(ruling));
        assert_eq!(
            shifted.profile, world.varieties[old].profile,
            "speakers keep their preferences"
        );
        let substrate = shifted
            .lexicon
            .living()
            .filter(|l| matches!(l.origin, Origin::Borrowed { from, .. } if from == old))
            .count();
        assert!(substrate > 0, "some old words survive");
        assert!(
            shifted.laws.iter().any(|(_, id)| *id == "substrate"),
            "an accent merges sounds"
        );

        let laws_at_shift = world.varieties[old].laws.len();
        assert!(!world.spoken()[old]);
        world.run(40);
        assert_eq!(
            world.varieties[old].laws.len(),
            laws_at_shift,
            "extinct varieties stop changing"
        );
    }

    #[test]
    fn subjugated_communities_tend_to_shift_on_their_own() {
        let shifted = (0..20u64)
            .filter(|&seed| {
                let (mut world, _, subjects) = conquest(seed, Params::default());
                world.run(60);
                // The subjects' own variety never gains a parent except by
                // their shifting to another language.
                let v = world.communities[subjects].variety;
                world.varieties[v].parent.is_some()
            })
            .count();
        assert!(shifted >= 10, "{shifted} of 20 shifted");
    }

    #[test]
    fn intelligibility_falls_as_daughters_diverge() {
        use crate::compare::intelligibility;
        let mut world = World::new(2, Params::static_society());
        let west = world.found("West", &SoundProfile::by_id("neutral").unwrap(), 0.5, 0.5);
        let other = world.found("Other", &SoundProfile::by_id("illithid").unwrap(), 0.5, 0.5);
        let east = world.split(west, "East", 0.0);
        let score = |w: &World, a: usize, b: usize| {
            intelligibility(&w.variety_of(a).lexicon, &w.variety_of(b).lexicon)
        };
        assert_eq!(score(&world, west, east), 1.0);
        world.run(40);
        let sisters = score(&world, west, east);
        let strangers = score(&world, west, other);
        assert!(
            sisters < 1.0 && sisters > strangers + 0.1,
            "{sisters} vs {strangers}"
        );
    }

    #[test]
    fn intelligibility_is_symmetric_and_near_zero_for_strangers() {
        use crate::compare::intelligibility;
        let mut world = World::new(6, Params::static_society());
        let a = world.found("A", &SoundProfile::by_id("neutral").unwrap(), 0.5, 0.5);
        let b = world.found("B", &SoundProfile::by_id("elvish").unwrap(), 0.5, 0.5);
        world.run(20);
        let (la, lb) = (&world.variety_of(a).lexicon, &world.variety_of(b).lexicon);
        assert!((intelligibility(la, lb) - intelligibility(lb, la)).abs() < 1e-6);
        assert!(intelligibility(la, lb) < 0.2);
    }

    #[test]
    fn shifted_communities_do_not_shift_again_within_the_family() {
        for seed in 0..10u64 {
            let (mut world, _, subjects) = conquest(seed, Params::default());
            world.run(120);
            let shifts = world
                .events
                .iter()
                .filter(|(_, e)| matches!(e, WorldEvent::Shift { community, .. } if *community == subjects))
                .count();
            assert!(shifts <= 1, "seed {seed}: {shifts} shifts");
        }
    }

    #[test]
    fn splitting_shares_land_instead_of_multiplying_it() {
        let mut world = World::solo(
            8,
            &SoundProfile::by_id("neutral").unwrap(),
            Params::default(),
        );
        world.run(300);
        let total: f32 = world.communities.iter().map(|c| c.size).sum();
        assert!(total < world.params.capacity * 1.2, "population {total}");
        assert!(
            world.communities.len() <= 8,
            "{} communities",
            world.communities.len()
        );
    }
}

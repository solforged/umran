use crate::adapt::Adapter;
use crate::concepts::{CONCEPTS, Concept, Field, related};
use crate::form::Form;
use crate::geography::{Map, MapSize};
use crate::laws::{Law, catalog};
use crate::lexicon::{Entry, Event, LexemeId, Lexicon, Origin};
use crate::morphology::Morphology;
use crate::names::{Landscape, Name, Naming, PlaceName, PlaceOrigin, place_name};
use crate::phoneme::PhonemeId;
use crate::phonotactics::Phonotactics;
use crate::profile::SoundProfile;
use crate::rng::{key, stream, weighted_index};
use crate::root::mint_one;
use crate::variety::Variety;
use rand::{Rng, RngCore};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

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
/// Prestige gap below which no people conquers another.
const CONQUEST_MIN_GAP: f32 = 0.3;
/// How much longer rule lasts per unit of prestige gap between ruler and
/// ruled: an empire far stronger than its subjects holds them longer.
const RULE_HOLD: f32 = 3.0;
/// Intensity of rule after a conquest, at least.
const CONQUEST_INTENSITY: f32 = 0.6;
/// Travel effort beyond which a founding people no longer prefers land
/// further from others: far enough to be its own.
const SETTLE_APART: f32 = 8.0;
/// Furthest a migrating people travels, in travel effort: about five
/// open plains, or a long way across the steppe.
const MIGRATION_REACH: f32 = 6.0;
/// How much likelier a people is to leave land it shares with a stronger
/// people.
const PUSHED: f32 = 2.0;
/// Share of a weaker people's land a stronger newcomer counts as free:
/// the locals make room, or are made to.
const YIELD: f32 = 0.5;
/// Furthest a coastal people sends a colony along or across the sea when
/// the land beside it is full, in travel effort: one or two sea regions.
const COLONY_REACH: f32 = 8.0;
/// How many times larger than the land's namers a people must grow before
/// its own word for the land takes over, so names do not flip back and
/// forth between peoples of about the same size.
const PLACE_HOLD: f32 = 2.0;
/// Chance that newcomers keep a land's old name, fitted to their sounds,
/// rather than coin their own: likelier when they had dealings with the
/// namers, as most river names in England are Celtic.
const PLACE_KEEP_KNOWN: f32 = 0.85;
const PLACE_KEEP_UNKNOWN: f32 = 0.4;
/// Generations a sound change keeps spreading after it takes hold in a
/// variety, its pull fading over them: a wave runs for a few centuries,
/// then the change is simply part of the language.
const WAVE_SPAN: u32 = 10;
/// Generations apart at which two varieties take up each other's sound
/// changes half as readily as twin dialects do.
const KIN_SPAN: f32 = 20.0;
/// How readily unrelated languages take up each other's sound changes,
/// relative to twin dialects: areal changes do cross families, as the
/// uvular r crossed western Europe, but seldom.
const KIN_STRANGERS: f32 = 0.15;

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
    /// Extra innovation hazard for a concept whose dominant word has worn
    /// below its language's minimal word.
    pub worn_pressure: f32,
    /// Usage disadvantage of a word below its language's minimal word.
    pub worn_cost: f32,
    /// Share of innovations for a clashing or worn word that rebuild it
    /// from its own material rather than replace it.
    pub renewal_share: f32,
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
    /// Population an open plain region can support; other land supports
    /// its terrain's share of this, and peoples on one region share it.
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
    /// Scales how soon contacts end of themselves; 1 gives each kind its
    /// typical lifespan, 0 makes contacts last for ever.
    pub contact_turnover: f32,
    /// Chance per generation that peoples sharing land and out of contact
    /// come to deal with each other as neighbours again.
    pub neighbour_rate: f32,
    /// Chance per generation that a people opens trade with another it
    /// has no dealings with.
    pub trade_rate: f32,
    /// Chance per generation, per unit of prestige gap beyond
    /// `CONQUEST_MIN_GAP`, that a people comes to rule one it deals with.
    pub conquest_rate: f32,
    /// Chance per generation that a people leaves its land for better land
    /// within reach, scaled by how crowded home is, how mobile its terrain
    /// makes it, and whether a stronger people shares it.
    pub migration_rate: f32,
    /// Scales the chance per generation that a sound change spreads from a
    /// variety to one it is in contact with, by the contact's kind and
    /// intensity, how close their land is, how near their kinship, and
    /// which is the more prestigious.
    pub wave_rate: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            sound_change_rate: 0.3,
            innovation_rate: 0.0055,
            stability_spread: 10.0,
            newcomer_share: 0.25,
            speakers: 12,
            preference_pull: 1.0,
            expressive_share: 0.2,
            clash_pressure: 6.0,
            clash_cost: 0.25,
            worn_pressure: 6.0,
            worn_cost: 0.15,
            renewal_share: 0.5,
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
            contact_turnover: 1.0,
            neighbour_rate: 0.1,
            trade_rate: 0.01,
            conquest_rate: 0.05,
            migration_rate: 0.02,
            wave_rate: 1.0,
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
            contact_turnover: 0.0,
            neighbour_rate: 0.0,
            trade_rate: 0.0,
            conquest_rate: 0.0,
            migration_rate: 0.0,
            wave_rate: 0.0,
            ..Self::default()
        }
    }
}

/// A group of people with a home variety and a few traits.
#[derive(Clone, Debug, PartialEq)]
pub struct Community {
    /// What its people call themselves, in their current language.
    pub name: Name,
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
    /// The region of the map this community lives on. Founded peoples
    /// settle open land; a community that splits off takes the roomiest
    /// land beside its parent's when that has more room, else stays.
    pub region: usize,
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

    /// Generations this kind of contact typically lasts before it ends of
    /// itself: trade routes fail and empires fall within centuries, while
    /// neighbours and shared gods last far longer.
    pub fn lifespan(self) -> f32 {
        match self {
            ContactKind::Trade => 12.0,
            ContactKind::Rule => 16.0,
            ContactKind::Intermarriage => 20.0,
            ContactKind::Religion => 30.0,
            ContactKind::Neighbours => 40.0,
        }
    }

    /// How readily a sound change passes along this kind of contact: it
    /// spreads through daily talk between people living side by side or
    /// marrying, less through rulers' speech, least through traders and
    /// priests.
    pub fn carries_sounds(self) -> f32 {
        match self {
            ContactKind::Neighbours | ContactKind::Intermarriage => 1.0,
            ContactKind::Rule => 0.7,
            ContactKind::Trade | ContactKind::Religion => 0.3,
        }
    }
}

/// Something that happened to communities rather than to words.
#[derive(Clone, Debug, PartialEq)]
pub enum WorldEvent {
    /// `community` was founded with a new language.
    Found { community: usize },
    /// `daughter` split off from `community`, speaking a new variety, and
    /// settled region `to`; `from` is the parent's land.
    Split {
        community: usize,
        daughter: usize,
        from: usize,
        to: usize,
    },
    /// `community` abandoned variety `from` for a daughter of `toward`'s.
    Shift {
        community: usize,
        from: usize,
        toward: usize,
        variety: usize,
    },
    /// `a` and `b` came into contact, by an author's hand or the world's.
    Met {
        a: usize,
        b: usize,
        kind: ContactKind,
    },
    /// The contact between `a` and `b` ended of itself. For rule, `a` is
    /// the side that ruled.
    Parted {
        a: usize,
        b: usize,
        kind: ContactKind,
    },
    /// `ruler` came to rule `ruled`, with whom it already had dealings.
    Conquered { ruler: usize, ruled: usize },
    /// The whole of `community` left region `from` for region `to`.
    Migrated {
        community: usize,
        from: usize,
        to: usize,
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
    /// The generation it began.
    pub since: u32,
}

/// Communities, their varieties, and the contacts between them, stepping
/// through generations of about 25 years.
#[derive(Clone, Debug)]
pub struct World {
    pub seed: u64,
    pub generation: u32,
    /// The land, drawn from the seed. Shared, since it never changes.
    pub map: Arc<Map>,
    pub communities: Vec<Community>,
    pub varieties: Vec<Variety>,
    pub contacts: Vec<Contact>,
    pub params: Params,
    /// Things that happened to communities, with the generation they
    /// happened in.
    pub events: Vec<(u32, WorldEvent)>,
    /// What each region is called, by every language that has held it,
    /// oldest first; the last is its name now. Empty for land no one has
    /// held, and for the sea.
    pub places: Vec<Vec<PlaceName>>,
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
    /// A world on a map of the default size.
    pub fn new(seed: u64, params: Params) -> Self {
        Self::with_map(seed, params, MapSize::default())
    }

    /// A world on a map of `size`, drawn from `seed`.
    pub fn with_map(seed: u64, params: Params, size: MapSize) -> Self {
        let map = Map::generate(seed, size);
        Self {
            seed,
            generation: 0,
            places: vec![Vec::new(); map.regions.len()],
            map: Arc::new(map),
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
        world.found(profile, 0.5, 0.5);
        world
    }

    /// Founds a community with a new variety of its own; returns its index.
    /// `power` is its standing apart from size, and its starting prestige.
    /// The variety's seed derives from the world seed and the index.
    pub fn found(&mut self, profile: &SoundProfile, power: f32, openness: f32) -> usize {
        let index = self.communities.len();
        let variety_seed = stream(self.seed, &[key("found"), index as u64]).next_u64();
        self.found_seeded(
            &Naming::People,
            profile,
            variety_seed,
            power,
            openness,
            None,
        )
    }

    /// Founds a community whose language comes from `variety_seed`, so a
    /// design previewed with that seed founds exactly the words shown.
    /// The people names itself as `naming` says, in its new language's
    /// words, and names the language after itself. It settles `region`
    /// if given, which must be land, or else land of the world's choosing.
    pub fn found_seeded(
        &mut self,
        naming: &Naming,
        profile: &SoundProfile,
        variety_seed: u64,
        power: f32,
        openness: f32,
        region: Option<usize>,
    ) -> usize {
        let index = self.communities.len();
        let mut variety = Variety::found(variety_seed, profile);
        let name = self.coin(&variety, naming, None);
        variety.name = self.language_name(&variety, &name);
        self.varieties.push(variety);
        let region = region.unwrap_or_else(|| self.homeland(index));
        self.communities.push(Community {
            name,
            variety: self.varieties.len() - 1,
            prestige: power.clamp(0.0, 1.0),
            power: power.clamp(0.0, 1.0),
            openness: openness.clamp(0.0, 1.0),
            size: INITIAL_SIZE,
            region,
        });
        self.events
            .push((self.generation, WorldEvent::Found { community: index }));
        self.hold_places();
        index
    }

    /// Where newly founded community `community` settles: unpeopled land,
    /// likelier the more it feeds and the further it lies from other
    /// peoples, or the roomiest land when none is unpeopled.
    fn homeland(&self, community: usize) -> usize {
        let peopled: HashSet<usize> = self.communities.iter().map(|c| c.region).collect();
        let open: Vec<usize> = (0..self.map.regions.len())
            .filter(|&r| self.map.regions[r].terrain.is_land() && !peopled.contains(&r))
            .collect();
        if open.is_empty() {
            let all: Vec<usize> = (0..self.map.regions.len()).collect();
            return self.roomiest(&all).expect("every map has land");
        }
        let weight = |r: usize| {
            let fertility = self.map.regions[r].terrain.fertility();
            let apart = peopled
                .iter()
                .map(|&o| self.map.distance(r, o))
                .fold(SETTLE_APART, f32::min);
            fertility * fertility * apart
        };
        let mut rng = stream(self.seed, &[key("homeland"), community as u64]);
        open[weighted_index(&mut rng, open.iter().map(|&r| weight(r)))]
    }

    /// How many each region can support.
    pub fn capacity(&self, region: usize) -> f32 {
        self.params.capacity * self.map.regions[region].terrain.fertility()
    }

    /// Population living on each region.
    fn occupation(&self) -> HashMap<usize, f32> {
        let mut out: HashMap<usize, f32> = HashMap::new();
        for c in &self.communities {
            *out.entry(c.region).or_default() += c.size;
        }
        out
    }

    /// The land among `regions` with the most room left, lowest index
    /// first on a tie; `None` if all of them are sea.
    fn roomiest(&self, regions: &[usize]) -> Option<usize> {
        let occupied = self.occupation();
        let room = |r: usize| self.capacity(r) - occupied.get(&r).copied().unwrap_or(0.0);
        regions
            .iter()
            .copied()
            .filter(|&r| self.map.regions[r].terrain.is_land())
            .fold(None, |best: Option<usize>, r| match best {
                Some(b) if room(b) >= room(r) => Some(b),
                _ => Some(r),
            })
    }

    /// Part of `community` moves off as a new community whose speech becomes
    /// a daughter variety; returns the new community's index. The two stay
    /// in contact at `intensity` (0 for none).
    /// The new community names itself as `naming` says, or chooses a name
    /// itself if `None`; its speech is named after it.
    pub fn split(&mut self, community: usize, naming: Option<&Naming>, intensity: f32) -> usize {
        let parent = self.communities[community].variety;
        let mut daughter = self.varieties[parent].fork(parent, self.generation);
        let naming = match naming {
            Some(n) => n.clone(),
            None => self.daughter_naming(community),
        };
        let base = &self.communities[community].name;
        let taken = |name: &Name| {
            self.communities
                .iter()
                .any(|c| c.name.form.segs == name.form.segs)
        };
        let mut name = self.coin(&daughter, &naming, Some((base, &self.varieties[parent])));
        // A clipped name can come out as an existing one ("the new Tinea"
        // clipped back to "Tinea"); a place name tells them apart instead.
        if taken(&name) {
            let other = crate::names::PLACES
                .iter()
                .map(|p| Naming::Place { place: (*p).into() })
                .map(|n| self.coin(&daughter, &n, None))
                .find(|n| !taken(n));
            name = other.unwrap_or(name);
        }
        daughter.name = self.language_name(&daughter, &name);
        self.varieties.push(daughter);
        // The leavers take the roomiest land beside home if it has more
        // room than home, judged before they go. When the land beside is
        // full, a coastal people sends them along or over the sea instead,
        // to the coast with the most room for the voyage, as Greek cities
        // sent out colonies.
        let home = self.communities[community].region;
        let occupied = self.occupation();
        let room = |r: usize| self.capacity(r) - occupied.get(&r).copied().unwrap_or(0.0);
        let colony = || {
            let map = &self.map;
            (0..map.regions.len())
                .filter(|&r| map.coastal(home) && map.coastal(r) && r != home)
                .filter(|&r| !map.regions[home].neighbours.contains(&r))
                .filter(|&r| map.distance(home, r) <= COLONY_REACH && room(r) > room(home))
                .map(|r| (r, room(r) / (1.0 + map.distance(home, r))))
                .fold(None, |best: Option<(usize, f32)>, (r, score)| match best {
                    Some((_, s)) if s >= score => best,
                    _ => Some((r, score)),
                })
                .map(|(r, _)| r)
        };
        let region = match self.roomiest(&self.map.regions[home].neighbours) {
            Some(beside) if room(beside) > room(home) => beside,
            _ => colony().unwrap_or(home),
        };
        self.communities[community].size /= 2.0;
        let mut new = self.communities[community].clone();
        new.name = name;
        new.variety = self.varieties.len() - 1;
        new.region = region;
        self.communities.push(new);
        let index = self.communities.len() - 1;
        if intensity > 0.0 {
            self.link(community, index, intensity, ContactKind::Neighbours);
        }
        self.events.push((
            self.generation,
            WorldEvent::Split {
                community,
                daughter: index,
                from: home,
                to: region,
            },
        ));
        self.hold_places();
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

    /// Brings `a` and `b` into contact, as an event of the world's history.
    pub fn connect(&mut self, a: usize, b: usize, intensity: f32, kind: ContactKind) {
        self.link(a, b, intensity, kind);
        self.events
            .push((self.generation, WorldEvent::Met { a, b, kind }));
    }

    /// A contact with no event of its own, as between the halves of a
    /// split, which the split's event already tells.
    fn link(&mut self, a: usize, b: usize, intensity: f32, kind: ContactKind) {
        self.contacts.push(Contact {
            a,
            b,
            intensity: intensity.clamp(0.0, 1.0),
            kind,
            since: self.generation,
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
        self.spread_waves(&spoken);
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
        self.migrate();
        self.shift_languages();
        self.end_contacts();
        self.make_contacts();
        self.hold_places();
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

    /// Logistic growth against each region's capacity, with a little
    /// noise; then prestige from power plus relative size.
    fn grow(&mut self) {
        let occupied = self.occupation();
        for c in 0..self.communities.len() {
            let mut rng = self.community_rng(c, "grow");
            let noise = rng.gen_range(-0.05..0.05);
            let region = self.communities[c].region;
            let room = 1.0 - occupied[&region] / self.capacity(region);
            self.communities[c].size *= (self.params.growth_rate * room + noise).exp();
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
                self.split(c, None, FISSION_CONTACT);
            }
        }
    }

    /// Peoples sometimes leave their land for better land within reach:
    /// likelier the more crowded home is, the more mobile its terrain makes
    /// them, and when a stronger people shares it. A stronger people counts
    /// part of a weaker people's land as free for the taking, so invaders
    /// seek out good land others hold. The newcomers come to deal with
    /// those already there as neighbours.
    fn migrate(&mut self) {
        for c in 0..self.communities.len() {
            let home = self.communities[c].region;
            let (size, prestige) = (self.communities[c].size, self.communities[c].prestige);
            let others = |r: usize| {
                self.communities
                    .iter()
                    .enumerate()
                    .filter(move |&(o, k)| o != c && k.region == r)
                    .map(|(_, k)| k)
            };
            let crowding = (size + others(home).map(|k| k.size).sum::<f32>()) / self.capacity(home);
            let pushed = if others(home).any(|k| k.prestige > prestige && k.size > size) {
                PUSHED
            } else {
                1.0
            };
            let mobility = self.map.regions[home].terrain.mobility();
            let hazard = self.params.migration_rate * mobility * crowding * pushed;
            let mut rng = self.community_rng(c, "migrate");
            if rng.r#gen::<f32>() >= hazard {
                continue;
            }
            let free = |r: usize| {
                let held: f32 = others(r)
                    .map(|k| {
                        if k.prestige >= prestige {
                            k.size
                        } else {
                            k.size * YIELD
                        }
                    })
                    .sum();
                self.capacity(r) - held
            };
            let stay = self.capacity(home) - others(home).map(|k| k.size).sum::<f32>();
            let options: Vec<(usize, f32)> = (0..self.map.regions.len())
                .filter(|&r| r != home && self.map.regions[r].terrain.is_land())
                .filter(|&r| self.map.distance(home, r) <= MIGRATION_REACH)
                .filter(|&r| free(r) > stay && free(r) >= size / 2.0)
                .map(|r| {
                    let d = self.map.distance(home, r);
                    (r, (free(r) - stay) / ((1.0 + d) * (1.0 + d)))
                })
                .collect();
            if options.is_empty() {
                continue;
            }
            let to = options[weighted_index(&mut rng, options.iter().map(|(_, w)| *w))].0;
            self.communities[c].region = to;
            self.events.push((
                self.generation,
                WorldEvent::Migrated {
                    community: c,
                    from: home,
                    to,
                },
            ));
            let locals: Vec<usize> = (0..self.communities.len())
                .filter(|&o| o != c && self.communities[o].region == to)
                .filter(|&o| {
                    !self
                        .contacts
                        .iter()
                        .any(|k| (k.a, k.b) == (c, o) || (k.a, k.b) == (o, c))
                })
                .collect();
            for o in locals {
                let intensity = rng.gen_range(0.3..0.7);
                self.connect(c, o, intensity, ContactKind::Neighbours);
            }
        }
    }

    /// Each land is called what the people holding it calls it: the
    /// largest people living there, once it outnumbers the land's namers
    /// `PLACE_HOLD` times over. A people whose language descends from the
    /// namers' inherits the name; others mostly keep it, fitted to their
    /// own sounds, or coin their own. Land no one lives on keeps its last
    /// name unchanged.
    fn hold_places(&mut self) {
        let generation = self.generation;
        for r in 0..self.map.regions.len() {
            let here = || {
                self.communities
                    .iter()
                    .enumerate()
                    .filter(move |(_, k)| k.region == r)
            };
            let largest = here().fold(None, |best: Option<(usize, f32)>, (i, k)| match best {
                Some((_, s)) if s >= k.size => best,
                _ => Some((i, k.size)),
            });
            let Some((holder, size)) = largest else {
                continue;
            };
            let variety = self.communities[holder].variety;
            let mut rng = stream(
                self.seed,
                &[
                    key("place"),
                    r as u64,
                    u64::from(generation),
                    variety as u64,
                ],
            );
            let before = self.places[r].last();
            let origin = match before {
                None => None,
                Some(p) if p.variety == variety => continue,
                Some(p) => {
                    let namers: f32 = here()
                        .filter(|(_, k)| k.variety == p.variety)
                        .map(|(_, k)| k.size)
                        .sum();
                    if size < namers * PLACE_HOLD {
                        continue;
                    }
                    if self.descends(variety, p.variety) {
                        Some(PlaceOrigin::Inherited)
                    } else {
                        let knew = namers > 0.0
                            || self.contacts.iter().any(|k| {
                                let other = match (k.a == holder, k.b == holder) {
                                    (true, _) => k.b,
                                    (_, true) => k.a,
                                    _ => return false,
                                };
                                self.communities[other].variety == p.variety
                            });
                        let keep = if knew {
                            PLACE_KEEP_KNOWN
                        } else {
                            PLACE_KEEP_UNKNOWN
                        };
                        (rng.r#gen::<f32>() < keep).then_some(PlaceOrigin::Borrowed)
                    }
                }
            };
            let speech = &self.varieties[variety];
            let name = match (origin, before) {
                (Some(PlaceOrigin::Inherited), Some(p)) => p.name.clone(),
                (Some(PlaceOrigin::Borrowed), Some(p)) => {
                    let adapter = Adapter::new(
                        speech.lexicon.living().map(|l| &l.form),
                        &speech.profile.inventory,
                    );
                    Name {
                        form: adapter.adapt(&p.name.form, 0.0, &mut rng),
                        meaning: p.name.meaning.clone(),
                        coined: generation,
                        log: Vec::new(),
                    }
                }
                _ => {
                    let land = Landscape {
                        terrain: self.map.regions[r].terrain,
                        coastal: self.map.coastal(r),
                        island: self.map.island(r),
                    };
                    let people = &self.communities[holder].name;
                    let spelled = speech.title(&people.form);
                    let coined = place_name(speech, land, (people, &spelled), &mut rng, generation);
                    let Some(name) = coined else { continue };
                    name
                }
            };
            let origin = origin.unwrap_or(PlaceOrigin::Coined { community: holder });
            self.places[r].push(PlaceName {
                variety,
                since: generation,
                name,
                origin,
            });
        }
    }

    /// Whether `variety` descends from `ancestor` through splits and shifts.
    pub fn descends(&self, variety: usize, ancestor: usize) -> bool {
        let mut at = variety;
        while let Some(fork) = self.varieties[at].parent {
            if fork.variety == ancestor {
                return true;
            }
            at = fork.variety;
        }
        false
    }

    /// What region `region` was called at `generation`, spelled in the
    /// language that held it then; `None` if no one had named it yet.
    pub fn place_at(&self, region: usize, generation: u32) -> Option<String> {
        let place = self.places[region]
            .iter()
            .rev()
            .find(|p| p.since <= generation)?;
        Some(self.varieties[place.variety].title(place.name.form_at(generation)))
    }

    /// What a group leaving `community` calls itself: an epithet on the
    /// old name or a place, avoiding meanings other communities have.
    fn daughter_naming(&self, community: usize) -> Naming {
        let parent = &self.communities[community].name;
        let spelled = self.community_name(community);
        let taken: HashSet<String> = self
            .communities
            .iter()
            .map(|c| c.name.meaning.clone())
            .collect();
        let options: Vec<(Naming, f32)> = Naming::for_daughter(parent)
            .into_iter()
            .filter(|(n, _)| {
                let meaning = match n {
                    Naming::Epithet { epithet } => format!("the {epithet} {spelled}"),
                    Naming::Place { place } => format!("the people of the {place}"),
                    _ => return true,
                };
                !taken.contains(&meaning)
            })
            .collect();
        if options.is_empty() {
            return Naming::Epithet {
                epithet: "new".into(),
            };
        }
        let mut rng = self.community_rng(community, "naming");
        options[weighted_index(&mut rng, options.iter().map(|(_, w)| *w))]
            .0
            .clone()
    }

    /// A people's name in `variety`'s words, falling back to "the people"
    /// if `naming` cannot be built. `base` is the name an epithet
    /// qualifies, with the variety that spells it.
    fn coin(&self, variety: &Variety, naming: &Naming, base: Option<(&Name, &Variety)>) -> Name {
        let spelled = base.map(|(name, v)| (name, v.title(&name.form)));
        naming
            .coin(
                variety,
                spelled.as_ref().map(|(n, s)| (*n, s.as_str())),
                self.generation,
            )
            .or_else(|_| Naming::People.coin(variety, None, self.generation))
            .unwrap_or_default()
    }

    fn language_name(&self, variety: &Variety, people: &Name) -> Name {
        crate::names::language_name(
            variety,
            people,
            &variety.title(&people.form),
            self.generation,
        )
    }

    /// What `community` calls itself, spelled in its language.
    pub fn community_name(&self, community: usize) -> String {
        self.variety_of(community)
            .title(&self.communities[community].name.form)
    }

    /// A community's name as it was said at `generation`, spelled the way
    /// its speech is spelled now.
    pub fn community_name_at(&self, community: usize, generation: u32) -> String {
        self.variety_of(community)
            .title(self.communities[community].name.form_at(generation))
    }

    /// What `by`'s people call `community`: its own name as they hear it,
    /// fitted to their sounds the way a loan is. Computed from the current
    /// names, so it is how they would say it now rather than a name with a
    /// history of its own.
    pub fn exonym(&self, community: usize, by: usize) -> String {
        let listener = self.variety_of(by);
        let adapter = Adapter::new(
            listener.lexicon.living().map(|l| &l.form),
            &listener.profile.inventory,
        );
        let mut rng = stream(self.seed, &[key("exonym"), community as u64, by as u64]);
        let heard = adapter.adapt(&self.communities[community].name.form, 0.0, &mut rng);
        listener.title(&heard)
    }

    /// What a variety's speakers call it, spelled.
    pub fn language_title(&self, variety: usize) -> String {
        let v = &self.varieties[variety];
        v.title(&v.name.form)
    }

    /// What a variety's speakers called it at `generation`, spelled.
    pub fn language_title_at(&self, variety: usize, generation: u32) -> String {
        let v = &self.varieties[variety];
        v.title(v.name.form_at(generation))
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

    /// Contacts end of themselves, each kind after its typical lifespan
    /// on average: routes fail, empires fall, distant neighbours drift
    /// apart, though peoples sharing land stay neighbours. Rule
    /// lasts longer the further its ruler stands above the ruled. Rule and
    /// intermarriage leave the peoples neighbours, at half the intensity.
    fn end_contacts(&mut self) {
        let generation = self.generation;
        let mut ended = Vec::new();
        for (i, contact) in self.contacts.iter().enumerate() {
            let mut rng = stream(
                self.seed,
                &[
                    key("step"),
                    u64::from(generation),
                    key("contact end"),
                    contact.a as u64,
                    contact.b as u64,
                    key(&format!("{:?}", contact.kind)),
                ],
            );
            let gap =
                (self.communities[contact.a].prestige - self.communities[contact.b].prestige).abs();
            let (ca, cb) = (&self.communities[contact.a], &self.communities[contact.b]);
            let hold = match contact.kind {
                // Peoples on the same land stay neighbours.
                ContactKind::Neighbours if ca.region == cb.region => continue,
                ContactKind::Rule => 1.0 + RULE_HOLD * gap,
                _ => 1.0,
            };
            // A contact lasts a third of its lifespan before it can end, so
            // routes and alliances are not made and lost within a lifetime.
            let lifespan = contact.kind.lifespan() * hold;
            let settled = generation - contact.since >= (lifespan / 3.0) as u32;
            let hazard = self.params.contact_turnover * 1.5 / lifespan;
            if settled && rng.r#gen::<f32>() < hazard {
                ended.push(i);
            }
        }
        for &i in ended.iter().rev() {
            let Contact {
                a,
                b,
                kind,
                intensity,
                ..
            } = self.contacts.remove(i);
            // Peoples that ruled or married one another still live near
            // each other afterwards.
            if matches!(kind, ContactKind::Rule | ContactKind::Intermarriage) {
                self.link(a, b, intensity / 2.0, ContactKind::Neighbours);
            }
            let ruler_first = self.communities[a].prestige >= self.communities[b].prestige;
            let (a, b) = if kind == ContactKind::Rule && !ruler_first {
                (b, a)
            } else {
                (a, b)
            };
            self.events
                .push((generation, WorldEvent::Parted { a, b, kind }));
        }
    }

    /// The world makes contacts of its own: peoples on the same or
    /// bordering land deal with each other again, peoples open trade,
    /// nearer partners likelier, and a people far above one it deals with
    /// may come to rule it.
    fn make_contacts(&mut self) {
        let in_touch = |contacts: &[Contact], a: usize, b: usize| {
            contacts
                .iter()
                .any(|k| (k.a, k.b) == (a, b) || (k.a, k.b) == (b, a))
        };
        for c in 0..self.communities.len() {
            let mut rng = self.community_rng(c, "contact");
            let strangers: Vec<usize> = (0..self.communities.len())
                .filter(|&o| o != c && !in_touch(&self.contacts, c, o))
                .collect();
            let here = self.communities[c].region;
            for &o in strangers.iter().filter(|&&o| o > c) {
                let near = self.map.closeness(here, self.communities[o].region);
                if near > 0.0 && rng.r#gen::<f32>() < self.params.neighbour_rate * near {
                    let intensity = rng.gen_range(0.3..0.7) * near;
                    self.connect(c, o, intensity, ContactKind::Neighbours);
                }
            }
            let strangers: Vec<usize> = strangers
                .into_iter()
                .filter(|&o| !in_touch(&self.contacts, c, o))
                .collect();
            if !strangers.is_empty() && rng.r#gen::<f32>() < self.params.trade_rate {
                let reach = |o: usize| {
                    let d = self.map.distance(here, self.communities[o].region);
                    1.0 / ((1.0 + d) * (1.0 + d))
                };
                let o = strangers[weighted_index(&mut rng, strangers.iter().map(|&o| reach(o)))];
                let intensity = rng.gen_range(0.2..0.6);
                self.connect(c, o, intensity, ContactKind::Trade);
            }
            let me = self.communities[c].prestige;
            let ruled: Vec<usize> = self
                .contacts
                .iter()
                .enumerate()
                .filter(|(_, k)| k.kind != ContactKind::Rule && (k.a == c || k.b == c))
                .filter_map(|(i, k)| {
                    let other = if k.a == c { k.b } else { k.a };
                    let gap = me - self.communities[other].prestige - CONQUEST_MIN_GAP;
                    let rules_already = self
                        .contacts
                        .iter()
                        .any(|r| r.kind == ContactKind::Rule && (r.a == other || r.b == other));
                    let hazard = self.params.conquest_rate * gap;
                    (gap > 0.0 && !rules_already && rng.r#gen::<f32>() < hazard).then_some(i)
                })
                .collect();
            if let Some(&i) = ruled.first() {
                let contact = &mut self.contacts[i];
                let ruled = if contact.a == c { contact.b } else { contact.a };
                contact.kind = ContactKind::Rule;
                contact.intensity = contact.intensity.max(CONQUEST_INTENSITY);
                contact.since = self.generation;
                self.events
                    .push((self.generation, WorldEvent::Conquered { ruler: c, ruled }));
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
        // The people keeps its own name and names its new speech after
        // itself, the new language's way: Bulgars gave Slavic speech theirs.
        new.name = self.language_name(&new, &self.communities[community].name);
        self.varieties.push(new);
        self.communities[community].variety = new_index;
        // So does its name for the land it holds.
        let region = self.communities[community].region;
        if let Some(p) = self.places[region].last().filter(|p| p.variety == old) {
            let kept = PlaceName {
                variety: new_index,
                since: generation,
                name: p.name.clone(),
                origin: PlaceOrigin::Kept,
            };
            self.places[region].push(kept);
        }
        self.events.push((
            generation,
            WorldEvent::Shift {
                community,
                from: old,
                toward: target,
                variety: new_index,
            },
        ));
        self.hold_places();
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
                let a = law.assess(
                    variety.lexicon.living().map(|l| &l.form),
                    prior,
                    variety.minimal,
                )?;
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
        self.apply_law(v, &law);
    }

    /// Applies `law` to every living word of variety `v`, and to the names
    /// of its language, its peoples, and the lands they hold.
    fn apply_law(&mut self, v: usize, law: &Law) {
        let generation = self.generation;
        let variety = &mut self.varieties[v];
        let minimal = variety.minimal;
        for lexeme in &mut variety.lexicon.lexemes {
            if lexeme.obsolete.is_some() {
                continue;
            }
            let after = law.apply(&lexeme.form, minimal);
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
        // Names are words too.
        let after = law.apply(&variety.name.form, minimal);
        variety.name.change(after, law.id, generation);
        for community in self.communities.iter_mut().filter(|c| c.variety == v) {
            let after = law.apply(&community.name.form, minimal);
            community.name.change(after, law.id, generation);
        }
        // And so are the names of the lands its speakers hold.
        for community in self.communities.iter().filter(|c| c.variety == v) {
            if let Some(p) = self.places[community.region]
                .last_mut()
                .filter(|p| p.variety == v)
            {
                let after = law.apply(&p.name.form, minimal);
                p.name.change(after, law.id, generation);
            }
        }
    }

    /// Sound changes spread like waves. A law that took hold in a variety
    /// in the last `WAVE_SPAN` generations may pass to a variety in
    /// contact with it that has not had it: likelier through close
    /// dealings between near land, between close kin (dialects that parted
    /// lately), from the more prestigious side, while the change is fresh,
    /// and when the receiving speakers like what it does. At most one law
    /// arrives in a variety per generation, all decided against the state
    /// before any arrives, so the order of varieties does not matter.
    fn spread_waves(&mut self, spoken: &[bool]) {
        if self.params.wave_rate <= 0.0 {
            return;
        }
        let generation = self.generation;
        let mut arrivals: Vec<(usize, Law, usize)> = Vec::new();
        for v in (0..self.varieties.len()).filter(|&v| spoken[v]) {
            let applied: HashSet<&str> = self.varieties[v].laws.iter().map(|(_, id)| *id).collect();
            // For each law on offer: its total pull, and the variety
            // pulling hardest, which is where it is said to come from.
            let mut offers: BTreeMap<&'static str, (f32, usize, f32)> = BTreeMap::new();
            for contact in &self.contacts {
                for (me, other) in [(contact.a, contact.b), (contact.b, contact.a)] {
                    let (m, o) = (&self.communities[me], &self.communities[other]);
                    if m.variety != v || o.variety == v {
                        continue;
                    }
                    let near = 0.2 + 0.8 * self.map.closeness(m.region, o.region);
                    let prestige = (1.0 + 2.0 * (o.prestige - m.prestige)).clamp(0.25, 3.0);
                    let pull = self.params.wave_rate
                        * contact.kind.carries_sounds()
                        * contact.intensity
                        * near
                        * prestige
                        * self.kinship(v, o.variety);
                    for &(g, id) in &self.varieties[o.variety].laws {
                        let age = generation.saturating_sub(g);
                        if age >= WAVE_SPAN || applied.contains(id) {
                            continue;
                        }
                        let h = pull * (1.0 - age as f32 / WAVE_SPAN as f32);
                        let offer = offers.entry(id).or_insert((0.0, o.variety, 0.0));
                        offer.0 += h;
                        if h > offer.2 {
                            (offer.1, offer.2) = (o.variety, h);
                        }
                    }
                }
            }
            let variety = &self.varieties[v];
            let offered: Vec<(&Law, usize, f32)> = offers
                .into_iter()
                .filter_map(|(id, (h, from, _))| {
                    let law = self.laws.iter().find(|l| l.id == id)?;
                    let a = law.assess(
                        variety.lexicon.living().map(|l| &l.form),
                        &variety.profile.inventory,
                        variety.minimal,
                    )?;
                    let taste = (self.params.preference_pull * a.pull.clamp(-5.0, 3.0))
                        .exp()
                        .min(3.0);
                    Some((law, from, h * taste))
                })
                .collect();
            if offered.is_empty() {
                continue;
            }
            let total: f32 = offered.iter().map(|o| o.2).sum();
            let mut rng = self.at(v).rng(&[key("wave")]);
            if rng.r#gen::<f32>() < total {
                let (law, from, _) = offered[weighted_index(&mut rng, offered.iter().map(|o| o.2))];
                arrivals.push((v, law.clone(), from));
            }
        }
        for (v, law, from) in arrivals {
            self.apply_law(v, &law);
            self.varieties[v].waves.push((law.id, from));
        }
    }

    /// How readily varieties `a` and `b` take up each other's sound
    /// changes: 1 for dialects that have just parted, half that once they
    /// have been apart `KIN_SPAN` generations, falling on toward
    /// `KIN_STRANGERS`, which is all unrelated languages get.
    fn kinship(&self, a: usize, b: usize) -> f32 {
        // Each variety's line back to its root, with the generation each
        // step of it left its parent.
        let line = |mut at: usize| {
            let mut out = vec![(at, None)];
            while let Some(fork) = self.varieties[at].parent {
                out.last_mut().expect("never empty").1 = Some(fork.generation);
                at = fork.variety;
                out.push((at, None));
            }
            out
        };
        let (la, lb) = (line(a), line(b));
        let Some((i, j)) = la
            .iter()
            .enumerate()
            .find_map(|(i, (x, _))| Some((i, lb.iter().position(|(y, _)| y == x)?)))
        else {
            return KIN_STRANGERS;
        };
        // The lines parted when the first of them left their common
        // ancestor.
        let parted = [
            i.checked_sub(1).map(|k| la[k].1),
            j.checked_sub(1).map(|k| lb[k].1),
        ]
        .into_iter()
        .flatten()
        .flatten()
        .min();
        let Some(parted) = parted else {
            return 1.0;
        };
        let apart = self.generation.saturating_sub(parted) as f32;
        (1.0 / (1.0 + apart / KIN_SPAN)).max(KIN_STRANGERS)
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
    /// its semantic field already uses. A word that sounds like another or
    /// has worn below the minimal word draws competitors more often, and
    /// some of them are the word itself renewed.
    fn innovate(&mut self, v: usize, clashes: &HashSet<LexemeId>) {
        let generation = self.generation;
        let step = self.at(v);
        let hazards: Vec<f32> = CONCEPTS.iter().map(|c| self.innovation_hazard(c)).collect();
        let params = &self.params;
        let spelling = self.varieties[v].profile.spelling.clone();
        let morphology = self.varieties[v].morphology.clone();
        let minimal = self.varieties[v].minimal;
        let lexicon = &mut self.varieties[v].lexicon;
        let observed = Phonotactics::observe(lexicon.living().map(|l| &l.form));
        for (i, concept) in CONCEPTS.iter().enumerate() {
            let mut rng = step.rng(&[key("innovate"), key(concept.id)]);
            let dominant = lexicon.slots[i].dominant();
            let clash = dominant.is_some_and(|id| clashes.contains(&id));
            let worn = dominant.is_some_and(|id| minimal.worn(&lexicon.get(id).form));
            let pressure = 1.0
                + f32::from(u8::from(clash)) * params.clash_pressure
                + f32::from(u8::from(worn)) * params.worn_pressure;
            if rng.r#gen::<f32>() >= hazards[i] * pressure {
                continue;
            }
            if lexicon.slots[i].variants.len() >= MAX_VARIANTS {
                continue;
            }
            let renewed = dominant
                .filter(|_| clash || worn)
                .and_then(|base| renewal(lexicon, &morphology, &step, concept, base, params));
            if let Some((form, origin)) = renewed {
                let id = lexicon.coin(form, origin, concept, generation);
                lexicon.slots[i].introduce(id, params.newcomer_share);
                continue;
            }
            let mut donors: Vec<LexemeId> = related(concept)
                .filter_map(|other| lexicon.slot(other).dominant())
                .filter(|id| !lexicon.slots[i].has(*id))
                .collect();
            donors.sort();
            donors.dedup();

            let newcomer = if !donors.is_empty() && rng.r#gen::<f32>() >= params.expressive_share {
                let id = donors[crate::rng::index(&mut rng, donors.len())];
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
                let form = mint_one(
                    &mut rng,
                    &observed,
                    &spelling,
                    Some(&morphology),
                    concept,
                    &used,
                    &field,
                );
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
        let minimal = self.varieties[v].minimal;
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
                let worn = if minimal.worn(&l.form) {
                    1.0 - params.worn_cost
                } else {
                    1.0
                };
                let loan = match l.origin {
                    Origin::Borrowed { from, .. } => (1.0
                        + params.prestige_selection * (prestige[from] - own))
                        .max(MIN_LOAN_FITNESS),
                    _ => 1.0,
                };
                clash * worn * loan
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

/// A worn or clashing word rebuilt from its own material, drawn from the
/// renewal stream: compounded with a related concept's word when there is
/// one and the draw falls that way, otherwise given the renewing affix.
/// `None` when the result would sound like a word already in use, or
/// when the draw keeps the usual ways of innovating.
fn renewal(
    lexicon: &Lexicon,
    morphology: &Morphology,
    step: &Step,
    concept: &'static Concept,
    base: LexemeId,
    params: &Params,
) -> Option<(Form, Origin)> {
    let mut rng = step.rng(&[key("renew"), key(concept.id)]);
    if rng.r#gen::<f32>() >= params.renewal_share {
        return None;
    }
    let mut partners: Vec<LexemeId> = related(concept)
        .filter_map(|other| lexicon.slot(other).dominant())
        .filter(|&id| id != base)
        .collect();
    partners.sort();
    partners.dedup();
    let old = &lexicon.get(base).form;
    let (form, with) = if !partners.is_empty() && rng.r#gen::<bool>() {
        let with = partners[crate::rng::index(&mut rng, partners.len())];
        (
            morphology.compound(&lexicon.get(with).form, old),
            Some(with),
        )
    } else {
        (morphology.renew(old), None)
    };
    let taken = lexicon.living().any(|l| l.form == form);
    (!taken).then_some((form, Origin::Renewed { base, with }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_are_reproducible() {
        let profile = SoundProfile::by_id("germanic").unwrap();
        let mut a = World::solo(7, &profile, Params::default());
        let mut b = World::solo(7, &profile, Params::default());
        a.run(40);
        b.run(40);
        assert_eq!(a.varieties[0].lexicon, b.varieties[0].lexicon);
        assert_eq!(a.varieties[0].laws, b.varieties[0].laws);
    }

    #[test]
    fn slots_stay_normalized_and_words_keep_a_vowel() {
        let profile = SoundProfile::by_id("polynesian").unwrap();
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
        // Calibrated on the base profile; inventories much larger or
        // smaller than its make fewer or more homophones, and so fewer or
        // more clash-driven replacements.
        for (n, profile) in std::iter::repeat_n(SoundProfile::base(), seeds).enumerate() {
            let mut sim = World::solo(n as u64, &profile, Params::default());
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
        let neutral = SoundProfile::base();
        let illithid = SoundProfile::by_id("iranian").unwrap();
        let fishy = neutral.flavored(&crate::flavor::Flavor::by_id("nahuatl").unwrap());
        let (spir_neutral, spir_illithid) = (
            rate(&neutral, "spirantization"),
            rate(&illithid, "spirantization"),
        );
        // A strong cultural effect: several times as likely, and by a wide
        // margin, even though spirantization's rare outputs (θ, x) count
        // against every culture.
        assert!(
            spir_illithid > 3.0 * spir_neutral && spir_illithid > spir_neutral + 0.2,
            "{spir_illithid} vs {spir_neutral}"
        );
        let (w_neutral, w_fishy) = (rate(&neutral, "w-fortition"), rate(&fishy, "w-fortition"));
        assert!(w_fishy < 0.5 * w_neutral, "{w_fishy} vs {w_neutral}");
    }

    /// A prestigious donor and an open recipient in contact for 40
    /// generations.
    fn pair(seed: u64, kind: ContactKind) -> World {
        let mut world = World::new(seed, Params::static_society());
        let d = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.8, 0.4);
        let r = world.found(&SoundProfile::by_id("polynesian").unwrap(), 0.3, 0.7);
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
        // Pronoun borrowing is rare but real (English "they" is Norse); about
        // 5% under this intense, lopsided contact over 400 seeds.
        assert!(pronouns < 0.08, "pronouns borrowed {pronouns:.3}");
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
        let profiles = SoundProfile::presets();
        let concepts: Vec<&'static Concept> = CONCEPTS.iter().collect();
        let (mut kept, mut cognates, mut loans_flagged, mut loans) = (0, 0, 0, 0);
        let (mut recovered, mut true_total, mut correct, mut found_total) = (0, 0, 0, 0);
        for seed in 0..30u64 {
            let mut world = World::new(seed, Params::static_society());
            let west = world.found(&profiles[seed as usize % 4], 0.5, 0.5);
            let outsiders = world.found(&profiles[(seed as usize + 2) % 4], 0.9, 0.3);
            world.run(5);
            let east = world.split(west, None, 0.0);
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
        let mut world = World::solo(9, &SoundProfile::base(), Params::default());
        world.run(3);
        let east = world.split(0, None, 0.0);
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
                ("familiar", "germanic"),
                ("iranian", "polynesian"),
                ("familiar", "iranian"),
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
                let a = world.found(&SoundProfile::by_id(x).unwrap(), 0.5, 0.5);
                let b = world.found(&SoundProfile::by_id(y).unwrap(), 0.5, 0.5);
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
        let mut world = World::solo(4, &SoundProfile::base(), Params::default());
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
        // With equal power, the larger community has at least the prestige,
        // among those that did not split since prestige was last reckoned.
        let split_now = |i: usize| {
            world.events.iter().any(|(g, e)| {
                *g == world.generation
                    && matches!(*e, WorldEvent::Split { community, daughter, .. }
                        if community == i || daughter == i)
            })
        };
        let mut by_size: Vec<&Community> = (0..world.communities.len())
            .filter(|&i| !split_now(i))
            .map(|i| &world.communities[i])
            .collect();
        by_size.sort_by(|a, b| a.size.total_cmp(&b.size));
        assert!(by_size.last().unwrap().prestige >= by_size[0].prestige);
    }

    fn conquest(seed: u64, params: Params) -> (World, usize, usize) {
        let mut world = World::new(seed, params);
        let rulers = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.9, 0.3);
        let subjects = world.found(&SoundProfile::by_id("polynesian").unwrap(), 0.2, 0.6);
        world.connect(rulers, subjects, 1.0, ContactKind::Rule);
        (world, rulers, subjects)
    }

    #[test]
    fn shifting_communities_keep_an_accent_and_some_old_words() {
        let (mut world, rulers, subjects) = conquest(3, Params::static_society());
        world.run(10);
        let old = world.communities[subjects].variety;
        let ruling = world.communities[rulers].variety;
        let land = world.communities[subjects].region;
        let land_name = world.places[land].last().unwrap().name.form.clone();
        let new = world.shift(subjects, rulers);
        let kept = world.places[land].last().unwrap();
        assert_eq!((kept.variety, kept.origin), (new, PlaceOrigin::Kept));
        assert_eq!(kept.name.form, land_name, "they keep their land's name");
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
        // Rule ends in time, so many subjects outlast their rulers, as the
        // Britons, Greeks, and Persians did; a sizable share still shift.
        assert!((5..=15).contains(&shifted), "{shifted} of 20 shifted");
    }

    #[test]
    fn intelligibility_falls_as_daughters_diverge() {
        use crate::compare::intelligibility;
        let mut world = World::new(2, Params::static_society());
        let west = world.found(&SoundProfile::base(), 0.5, 0.5);
        let other = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.5, 0.5);
        let east = world.split(west, None, 0.0);
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
        let a = world.found(&SoundProfile::base(), 0.5, 0.5);
        let b = world.found(&SoundProfile::by_id("germanic").unwrap(), 0.5, 0.5);
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
    fn peoples_name_themselves_and_their_speech() {
        let mut world = World::new(3, Params::static_society());
        let a = world.found(&SoundProfile::base(), 0.5, 0.5);
        let b = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.9, 0.5);
        assert_eq!(world.communities[a].name.meaning, "the people");
        let people = |w: &World, c: usize| {
            let concept = crate::concepts::by_id("people").unwrap();
            w.variety_of(c)
                .lexicon
                .word_for(concept)
                .unwrap()
                .form
                .clone()
        };
        assert_eq!(world.communities[a].name.form, people(&world, a));
        assert!(
            world.varieties[0]
                .name
                .meaning
                .contains(&world.community_name(a))
        );

        // A daughter takes a new name, by itself or as told.
        let d = world.split(a, None, 0.3);
        assert_ne!(world.communities[d].name, world.communities[a].name);
        let river = Naming::Place {
            place: "river".into(),
        };
        let e = world.split(a, Some(&river), 0.3);
        assert_eq!(world.communities[e].name.meaning, "the people of the river");

        // Neighbours say it with their own sounds.
        assert!(!world.exonym(a, b).is_empty());

        // A people that shifts keeps its name, and names its new speech
        // after itself.
        let before = world.communities[a].name.clone();
        let v = world.shift(a, b);
        assert_eq!(world.communities[a].name, before);
        assert!(
            world.varieties[v]
                .name
                .meaning
                .contains(&world.community_name(a))
        );
    }

    /// Mean syllables of the dominant words, and the share of distinct
    /// dominant words that sound like another.
    fn shape(world: &World) -> (f32, f32) {
        let lexicon = &world.varieties[0].lexicon;
        let mut words: Vec<LexemeId> = lexicon
            .slots
            .iter()
            .filter_map(crate::lexicon::Slot::dominant)
            .collect();
        words.sort();
        words.dedup();
        let form = |id: LexemeId| &lexicon.get(id).form;
        let syllables = words
            .iter()
            .map(|&id| form(id).vowel_count() as f32)
            .sum::<f32>();
        let homophones = words
            .iter()
            .filter(|&&a| words.iter().any(|&b| a != b && form(a) == form(b)))
            .count();
        let n = words.len() as f32;
        (syllables / n, homophones as f32 / n)
    }

    #[test]
    fn worn_words_are_renewed_and_word_shapes_persist() {
        let mean = |preset: &str| {
            let (mut length, mut homophony) = (0.0, 0.0);
            for seed in 0..12 {
                let profile = SoundProfile::by_id(preset).unwrap();
                let mut world = World::solo(seed, &profile, Params::static_society());
                world.run(160);
                let (l, h) = shape(&world);
                length += l / 12.0;
                homophony += h / 12.0;
            }
            (length, homophony)
        };
        let (long, long_homophony) = mean("polynesian");
        let (short, short_homophony) = mean("pie-like");
        // After four thousand years, languages of long words keep them and
        // languages of short roots stay short, rather than all converging.
        assert!(long > 1.75, "polynesian words wore down to {long:.2}");
        assert!(short < 1.5, "pie-like words grew to {short:.2}");
        assert!(long - short > 0.4);
        assert!(
            long_homophony < 0.05 && short_homophony < 0.05,
            "homophony {long_homophony:.3}, {short_homophony:.3}"
        );
    }

    /// Contacts come and go: a trade route between peoples on different
    /// lands ends within centuries and leaves an event, rule fades into
    /// neighbourhood, and a static society keeps its contacts for ever.
    #[test]
    fn contacts_end_and_the_world_makes_new_ones() {
        let (mut ended, mut freed, mut met) = (0, 0, 0);
        for seed in 0..20 {
            let mut world = World::new(seed, Params::default());
            let a = world.found(&SoundProfile::by_id("germanic").unwrap(), 0.5, 0.5);
            let b = world.found(&SoundProfile::by_id("finnic").unwrap(), 0.5, 0.5);
            let c = world.found(&SoundProfile::by_id("semitic").unwrap(), 0.5, 0.5);
            world.connect(a, b, 0.5, ContactKind::Trade);
            world.connect(a, c, 0.5, ContactKind::Rule);
            let made = world.events.len();
            world.run(40);
            let events = &world.events[made..];
            ended += usize::from(events.iter().any(|(_, e)| {
                matches!(
                    e,
                    WorldEvent::Parted {
                        kind: ContactKind::Trade,
                        ..
                    }
                )
            }));
            freed += usize::from(events.iter().any(|(_, e)| {
                matches!(
                    e,
                    WorldEvent::Parted {
                        kind: ContactKind::Rule,
                        ..
                    }
                )
            }));
            met += usize::from(
                events
                    .iter()
                    .any(|(_, e)| matches!(e, WorldEvent::Met { .. })),
            );
            for (_, e) in events {
                if let WorldEvent::Parted {
                    a,
                    b,
                    kind: ContactKind::Rule,
                } = *e
                {
                    let pair = |x: usize, y: usize| (x, y) == (a, b) || (y, x) == (a, b);
                    let neighbours = world
                        .contacts
                        .iter()
                        .any(|k| k.kind == ContactKind::Neighbours && pair(k.a, k.b));
                    let since = world.events.iter().any(|(_, later)| {
                        matches!(*later, WorldEvent::Parted { a: x, b: y, kind: ContactKind::Neighbours } if pair(x, y))
                    });
                    assert!(neighbours || since, "rule left no neighbourhood");
                }
            }
        }
        assert!(ended >= 15, "trade ended in {ended} of 20");
        assert!(freed >= 8, "rule ended in {freed} of 20");
        assert!(met >= 5, "the world made contacts in {met} of 20");

        let mut world = World::new(1, Params::static_society());
        let a = world.found(&SoundProfile::by_id("germanic").unwrap(), 0.5, 0.5);
        let b = world.found(&SoundProfile::by_id("finnic").unwrap(), 0.5, 0.5);
        world.connect(a, b, 0.5, ContactKind::Trade);
        world.run(100);
        assert_eq!(world.contacts.len(), 1);
    }

    /// Peoples split again and again over a long history, yet their names
    /// stay short enough to say: epithets do not stack and long names are
    /// clipped.
    #[test]
    fn names_stay_short_over_many_splits() {
        for seed in 0..4 {
            let mut world = World::new(seed, Params::default());
            let a = world.found(&SoundProfile::by_id("finnic").unwrap(), 0.5, 0.5);
            let b = world.found(&SoundProfile::by_id("semitic").unwrap(), 0.8, 0.3);
            world.connect(a, b, 0.6, ContactKind::Rule);
            world.run(150);
            assert!(world.communities.len() > 2, "no splits to test");
            for c in &world.communities {
                assert!(c.name.form.vowel_count() <= 3, "{}", c.name.form.ipa());
            }
            for (v, spoken) in world.spoken().into_iter().enumerate() {
                let name = &world.varieties[v].name.form;
                assert!(!spoken || name.vowel_count() <= 4, "{}", name.ipa());
            }
        }
    }

    #[test]
    fn names_undergo_sound_change() {
        let mut world = World::new(5, Params::static_society());
        world.found(&SoundProfile::base(), 0.5, 0.5);
        let concept = crate::concepts::by_id("people").unwrap();
        world.run(80);
        let name = &world.communities[0].name;
        let word = world.variety_of(0).lexicon.slot(concept);
        // While "people" keeps its founding word, unrenewed, the name moves
        // with it.
        let lexicon = &world.variety_of(0).lexicon;
        let now = lexicon.get(word.dominant().unwrap());
        if now.born == 0 && lexicon.keeps_founding_word(concept) {
            assert_eq!(name.form, now.form);
        }
        assert!(!name.log.is_empty() || world.variety_of(0).laws.is_empty());
    }

    #[test]
    fn split_offs_settle_bordering_land_or_coastal_colonies() {
        for seed in [8, 9, 10] {
            let mut world = World::solo(seed, &SoundProfile::base(), Params::default());
            world.run(300);
            for (region, people) in world.occupation() {
                assert!(
                    people < world.capacity(region) * 1.2,
                    "seed {seed}: region {region} holds {people}"
                );
            }
            let mut moved = false;
            for (_, event) in &world.events {
                if let WorldEvent::Split { from, to, .. } = *event {
                    let map = &world.map;
                    let colony = map.coastal(from)
                        && map.coastal(to)
                        && map.distance(from, to) <= COLONY_REACH;
                    assert!(from == to || map.regions[from].neighbours.contains(&to) || colony);
                    moved |= from != to;
                }
            }
            assert!(moved, "seed {seed}: no people ever left home");
        }
    }

    #[test]
    fn place_names_change_with_their_holders_speech_and_freeze_when_left() {
        let mut world = World::new(5, Params::static_society());
        world.found(&SoundProfile::base(), 0.5, 0.5);
        let home = world.communities[0].region;
        let first = world.places[home][0].clone();
        assert_eq!(first.origin, PlaceOrigin::Coined { community: 0 });
        assert_eq!(first.variety, world.communities[0].variety);
        world.run(80);
        let named = &world.places[home][0].name;
        assert!(!named.log.is_empty() || world.variety_of(0).laws.is_empty());

        let elsewhere = (0..world.map.regions.len())
            .find(|&r| r != home && world.map.regions[r].terrain.is_land())
            .unwrap();
        world.communities[0].region = elsewhere;
        let left = world.places[home][0].name.form.clone();
        world.run(80);
        assert_eq!(world.places[home][0].name.form, left);
        assert_eq!(world.places[elsewhere].len(), 1, "they name their new land");
    }

    #[test]
    fn newcomers_mostly_keep_the_name_of_land_they_take_over() {
        let mut kept = 0;
        for seed in 0..40 {
            let mut world = World::new(seed, Params::static_society());
            let natives = world.found(&SoundProfile::base(), 0.2, 0.5);
            let comers = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.9, 0.5);
            world.connect(natives, comers, 0.5, ContactKind::Trade);
            let land = world.communities[natives].region;
            world.communities[comers].region = land;
            world.communities[comers].size = world.communities[natives].size * PLACE_HOLD * 1.5;
            world.step();
            let names = &world.places[land];
            assert_eq!(
                names.len(),
                2,
                "seed {seed}: the larger people's name takes over"
            );
            assert_eq!(names[1].variety, world.communities[comers].variety);
            match names[1].origin {
                PlaceOrigin::Borrowed => {
                    kept += 1;
                    assert_eq!(names[1].name.meaning, names[0].name.meaning);
                }
                PlaceOrigin::Coined { community } => assert_eq!(community, comers),
                other => panic!("seed {seed}: {other:?}"),
            }
        }
        assert!((28..=39).contains(&kept), "{kept} of 40 kept the old name");
    }

    #[test]
    fn peoples_migrate_onto_land_within_reach() {
        let mut moved = 0;
        for seed in 0..6 {
            let mut world = World::new(seed, Params::default());
            for preset in ["familiar", "polynesian", "iranian", "finnic"] {
                world.found(&SoundProfile::by_id(preset).unwrap(), 0.5, 0.5);
            }
            world.run(120);
            for (_, event) in &world.events {
                if let WorldEvent::Migrated { from, to, .. } = *event {
                    moved += 1;
                    assert!(from != to && world.map.regions[to].terrain.is_land());
                    assert!(world.map.distance(from, to) <= MIGRATION_REACH);
                }
            }
        }
        assert!(moved >= 3, "{moved} migrations in 6 books");
    }

    #[test]
    fn sound_changes_spread_to_close_kin_far_more_than_to_strangers() {
        let (mut kin, mut strangers) = (0, 0);
        for seed in 0..30 {
            let params = Params {
                wave_rate: 1.0,
                ..Params::static_society()
            };
            let mut world = World::new(seed, params);
            // One taste and one land for all, so only kinship differs.
            let home = world.found(&SoundProfile::base(), 0.5, 0.5);
            let stranger = world.found(&SoundProfile::base(), 0.5, 0.5);
            let dialect = world.split(home, None, 0.5);
            world.connect(home, stranger, 0.5, ContactKind::Neighbours);
            let land = world.communities[home].region;
            world.communities[stranger].region = land;
            world.communities[dialect].region = land;
            world.run(60);
            let source = world.communities[home].variety;
            for (who, count) in [(dialect, &mut kin), (stranger, &mut strangers)] {
                let v = world.variety_of(who);
                for &(id, from) in &v.waves {
                    assert!(
                        v.laws.iter().any(|(_, l)| *l == id),
                        "a wave is a law undergone"
                    );
                    *count += usize::from(from == source);
                }
            }
        }
        assert!(
            kin >= 10 && kin >= 2 * strangers,
            "{kin} changes reached the dialect, {strangers} the strangers"
        );
    }
}

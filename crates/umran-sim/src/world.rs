use crate::adapt::Adapter;
use crate::concepts::{CONCEPTS, Concept, Field, related};
use crate::form::Form;
use crate::geography::{Map, MapSize, Terrain};
use crate::ideas::{Craft, Religion, SACRED_INTENSITY, SACRED_PRESTIGE, living_related};
use crate::laws::{Law, catalog};
use crate::lexicon::{Entry, Event, LexemeId, Lexicon, Origin};
use crate::livelihood::Livelihood;
use crate::morphology::Morphology;
use crate::names::{Landscape, Name, Naming, PlaceName, PlaceOrigin, place_name};
use crate::phoneme::PhonemeId;
use crate::phonotactics::Phonotactics;
use crate::polity::{
    LEVEL_FLOOR, LEVELLING, PURIST_COST, PURIST_PRESSURE, STANDARD_PRESTIGE, STANDARD_SHIFT, State,
};
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
/// Usage share a word stretched to a meaning its speakers' way of life
/// links it with takes at once: twice an ordinary newcomer's.
const APT_SHARE: f32 = 0.5;
/// Floor on a loan's usage fitness, however low its donor's prestige.
const MIN_LOAN_FITNESS: f32 = 0.2;
/// Population of a newly founded community.
const INITIAL_SIZE: f32 = 1000.0;
/// Prestige gap below which no community abandons its language.
const SHIFT_MIN_GAP: f32 = 0.2;
/// Contact intensity between the halves of a community that just split.
const FISSION_CONTACT: f32 = 0.5;
/// Weight of a people that moves off naming itself for its new land,
/// against epithets and places weighing 1 together.
const LAND_NAMING: f32 = 0.4;
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
/// Population below which a people can no longer go on as a people: it
/// dies out, or merges into a people sharing its land.
const MIN_PEOPLE: f32 = 100.0;
/// Share of what its lands feed it a people must be using before it
/// spreads into the land beside them.
const SPREAD_FULL: f32 = 0.6;
/// Share of what a land would feed them that must be free before a
/// people settles it beside their own.
const SPREAD_ROOM: f32 = 0.3;
/// How many times a stronger people must outnumber one on a land before
/// it crowds it off, and the chance per generation that it does.
const CROWDED_OUT: f32 = 4.0;
const DISPLACE_CHANCE: f32 = 0.3;
/// How much more a way of life must feed on a people's own lands before
/// they take it up.
const ADOPT_GAIN: f32 = 1.5;
/// How much rarer foragers begin farming of their own accord than learn
/// it from farmers they deal with.
const FARMING_FOUND: f32 = 0.02;
/// Size, as a share of a kindred people sharing its heart land, below
/// which a people may merge into it.
const MERGE_SHARE: f32 = 0.15;

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
    /// Population growth per generation for a farming community far below
    /// its land's capacity; growth slows logistically as it fills up, and
    /// other ways of living grow more slowly (`Livelihood::growth`).
    pub growth_rate: f32,
    /// Population an open plain region can support when farmed; other
    /// land and other ways of living support their share of this
    /// (`Livelihood::feeds`), and everyone on a land shares it.
    pub capacity: f32,
    /// Population at which a farming people starts to come apart; other
    /// ways of living hold together at their share of it
    /// (`Livelihood::cohesion`).
    pub cohesion_size: f32,
    /// Travel effort from its heartland at which a people starts to come
    /// apart, for farmers; more mobile peoples hold together further.
    pub cohesion_reach: f32,
    /// Chance per generation, per unit of strain beyond holding together,
    /// that a people splits.
    pub fission_rate: f32,
    /// Chance per generation that a people with no room left takes land
    /// beside its own, scaled by how mobile its way of life makes it.
    pub spread_rate: f32,
    /// Chance per generation that bad times (famine, plague, drought)
    /// strike a peopled land.
    pub hardship_rate: f32,
    /// Chance per generation that a people takes up a better way of
    /// living it knows of, from its own past or a people it deals with,
    /// scaled by how closely it deals with them.
    pub adoption_rate: f32,
    /// Chance per generation that a people far smaller than another on its
    /// heartland, speaking a language of the same family, merges into it.
    pub merge_rate: f32,
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
    /// Chance per generation that a large farming people under no state,
    /// facing a challenge, organizes itself into one; a tenth of this in
    /// comfort.
    pub state_rate: f32,
    /// Chance per generation that a state collapses of itself; bad times
    /// on its rulers' lands make it likelier.
    pub collapse_rate: f32,
    /// Chance per generation that a people leaves its land for better land
    /// within reach, scaled by how crowded home is, how mobile its terrain
    /// makes it, and whether a stronger people shares it.
    pub migration_rate: f32,
    /// Scales the chance per generation that a sound change spreads from a
    /// variety to one it is in contact with, by the contact's kind and
    /// intensity, how close their land is, how near their kinship, and
    /// which is the more prestigious.
    pub wave_rate: f32,
    /// Scales the chance per generation that a people comes upon a craft
    /// its land and life invite (`ideas::Craft`).
    pub craft_rate: f32,
    /// Scales the chance per generation that a craft passes along a
    /// contact, by its intensity and kind.
    pub idea_rate: f32,
    /// Chance per generation that a faith is founded among the subjects
    /// of an old state in a time of troubles; a tenth of it in quiet times.
    pub religion_rate: f32,
    /// Scales the chance per generation that a people takes up the faith
    /// of one it deals with, by the contact's intensity and kind.
    pub conversion_rate: f32,
    /// Chance per generation that a language takes up a new given name.
    pub name_turnover: f32,
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
            growth_rate: 0.07,
            capacity: 40000.0,
            cohesion_size: 100000.0,
            cohesion_reach: 3.0,
            fission_rate: 0.1,
            spread_rate: 0.3,
            hardship_rate: 0.004,
            adoption_rate: 0.1,
            merge_rate: 0.05,
            size_prestige: 0.15,
            shift_rate: 0.02,
            substrate_merge: 0.6,
            substrate_words: 0.5,
            contact_turnover: 1.0,
            neighbour_rate: 0.1,
            trade_rate: 0.01,
            conquest_rate: 0.05,
            state_rate: 0.02,
            collapse_rate: 0.02,
            migration_rate: 0.02,
            wave_rate: 1.0,
            craft_rate: 0.001,
            idea_rate: 0.02,
            religion_rate: 0.02,
            conversion_rate: 0.05,
            name_turnover: 0.1,
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
            spread_rate: 0.0,
            hardship_rate: 0.0,
            adoption_rate: 0.0,
            merge_rate: 0.0,
            shift_rate: 0.0,
            contact_turnover: 0.0,
            neighbour_rate: 0.0,
            trade_rate: 0.0,
            conquest_rate: 0.0,
            state_rate: 0.0,
            collapse_rate: 0.0,
            migration_rate: 0.0,
            wave_rate: 0.0,
            craft_rate: 0.0,
            idea_rate: 0.0,
            religion_rate: 0.0,
            conversion_rate: 0.0,
            name_turnover: 0.0,
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
    /// The lands this people holds, its heartland first. Its people are
    /// spread over them by how many each feeds them.
    pub lands: Vec<usize>,
    /// How it gets its food.
    pub livelihood: Livelihood,
    /// The generation it came to an end, dying out or merging into
    /// another people; its record stays, but it no longer lives anywhere.
    pub ended: Option<u32>,
    /// The founded religion it holds, if any, else its own folk religion.
    pub faith: Option<usize>,
    /// The crafts it holds, in `Craft` order.
    pub crafts: Vec<Craft>,
}

impl Community {
    /// The land at the heart of its lands, where it was founded or last
    /// settled.
    pub fn home(&self) -> usize {
        self.lands[0]
    }

    pub fn living(&self) -> bool {
        self.ended.is_none()
    }
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
    /// `community` took land `to` beside its own, keeping its other lands.
    Spread { community: usize, to: usize },
    /// `community` was crowded off land `region`, one of several it held,
    /// by `by`, the largest people living there.
    Displaced {
        community: usize,
        region: usize,
        by: usize,
    },
    /// Bad times struck land `region`, killing `share` of everyone on it.
    HardTimes {
        region: usize,
        kind: Hardship,
        share: f32,
    },
    /// `community` took up a new way of living, learnt from `from`, or
    /// found by itself or remembered from its past when `None`.
    Adopted {
        community: usize,
        livelihood: Livelihood,
        from: Option<usize>,
    },
    /// `community` came to an end: dwindled away, or merged `into`
    /// another people living on its land.
    Ended {
        community: usize,
        into: Option<usize>,
    },
    /// State `state` arose, by conquest, in answer to a challenge, or by
    /// an author's hand (`State::how`).
    Rose { state: usize },
    /// State `state` fell (`State::fell` says how).
    Fell { state: usize },
    /// State `state` took its court speech as its standard.
    Standard { state: usize },
    /// `community` took up `craft`, taught by `from` or by itself.
    Learnt {
        community: usize,
        craft: Craft,
        from: Option<usize>,
    },
    /// Religion `religion` was founded (`Religion::how` says how).
    Revealed { religion: usize },
    /// `community` took up `religion`, taught by `from` or by an author.
    Converted {
        community: usize,
        religion: usize,
        from: Option<usize>,
    },
    /// In variety `variety`, spoken by `community`, `word` came to mean
    /// `to` instead of `from` with a change of faith: the old gods became
    /// demons.
    Pejorated {
        community: usize,
        variety: usize,
        word: LexemeId,
        from: &'static Concept,
        to: &'static Concept,
    },
    /// The state whose standard variety `variety` is spelled it anew, as
    /// it now sounds.
    Respelled { variety: usize },
}

/// What kind of bad times strike a land.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Hardship {
    Famine,
    Plague,
    /// Only on dry land: steppe and desert.
    Drought,
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
    /// Every state that has stood, in the order they arose.
    pub states: Vec<State>,
    /// Every religion founded, in order.
    pub religions: Vec<Religion>,
    laws: Vec<Law>,
}

/// Per-variety random streams for one generation.
pub(crate) struct Step {
    seed: u64,
    generation: u32,
    variety: usize,
}

impl Step {
    pub(crate) fn rng(&self, labels: &[u64]) -> ChaCha8Rng {
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
            states: Vec::new(),
            religions: Vec::new(),
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
            None,
        )
    }

    /// Founds a community whose language comes from `variety_seed`, so a
    /// design previewed with that seed founds exactly the words shown.
    /// The people names itself as `naming` says, in its new language's
    /// words, and names the language after itself. It settles `region`
    /// if given, which must be land, or else land of the world's choosing,
    /// and lives as `livelihood` says, or else as its land suits.
    #[allow(clippy::too_many_arguments)]
    pub fn found_seeded(
        &mut self,
        naming: &Naming,
        profile: &SoundProfile,
        variety_seed: u64,
        power: f32,
        openness: f32,
        region: Option<usize>,
        livelihood: Option<Livelihood>,
    ) -> usize {
        let index = self.communities.len();
        let region = region.unwrap_or_else(|| self.homeland(index));
        let livelihood =
            livelihood.unwrap_or_else(|| Livelihood::of_land(self.map.regions[region].terrain));
        let mut variety = Variety::found(variety_seed, profile, livelihood);
        let name = self.coin(&variety, naming, None);
        variety.name = self.language_name(&variety, &name);
        self.varieties.push(variety);
        self.communities.push(Community {
            name,
            variety: self.varieties.len() - 1,
            prestige: power.clamp(0.0, 1.0),
            power: power.clamp(0.0, 1.0),
            openness: openness.clamp(0.0, 1.0),
            size: INITIAL_SIZE,
            lands: vec![region],
            livelihood,
            ended: None,
            faith: None,
            crafts: Vec::new(),
        });
        self.events
            .push((self.generation, WorldEvent::Found { community: index }));
        self.hold_places();
        index
    }

    /// Peoples that have not come to an end, by index.
    pub fn living(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.communities.len()).filter(|&c| self.communities[c].living())
    }

    /// Where newly founded community `community` settles: unpeopled land,
    /// likelier the more it feeds and the further it lies from other
    /// peoples, or the roomiest land when none is unpeopled.
    fn homeland(&self, community: usize) -> usize {
        let peopled: HashSet<usize> = self
            .living()
            .flat_map(|c| self.communities[c].lands.iter().copied())
            .collect();
        let open: Vec<usize> = (0..self.map.regions.len())
            .filter(|&r| self.map.regions[r].terrain.is_land() && !peopled.contains(&r))
            .collect();
        if open.is_empty() {
            let all: Vec<usize> = (0..self.map.regions.len()).collect();
            return self
                .roomiest(&all, Livelihood::Farming)
                .expect("every map has land");
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

    /// How many `region` feeds a people living by `livelihood`.
    pub fn feeds(&self, region: usize, livelihood: Livelihood) -> f32 {
        self.params.capacity * livelihood.feeds(self.map.regions[region].terrain)
    }

    /// How many of `community` live on each of its lands: its people are
    /// spread over them by how many each feeds them.
    pub fn presence(&self, community: usize) -> Vec<(usize, f32)> {
        let c = &self.communities[community];
        if !c.living() {
            return Vec::new();
        }
        let fed: Vec<f32> = c
            .lands
            .iter()
            .map(|&r| self.feeds(r, c.livelihood))
            .collect();
        let total: f32 = fed.iter().sum();
        c.lands
            .iter()
            .zip(fed)
            .map(|(&r, f)| {
                let share = if total > 0.0 {
                    f / total
                } else {
                    1.0 / c.lands.len() as f32
                };
                (r, c.size * share)
            })
            .collect()
    }

    /// Population living on each region.
    fn occupation(&self) -> HashMap<usize, f32> {
        let mut out: HashMap<usize, f32> = HashMap::new();
        for c in self.living() {
            for (r, n) in self.presence(c) {
                *out.entry(r).or_default() += n;
            }
        }
        out
    }

    /// The land among `regions` with the most room left for a people living
    /// by `livelihood`, lowest index first on a tie; `None` if all of them
    /// are sea.
    fn roomiest(&self, regions: &[usize], livelihood: Livelihood) -> Option<usize> {
        let occupied = self.occupation();
        let room = |r: usize| self.feeds(r, livelihood) - occupied.get(&r).copied().unwrap_or(0.0);
        regions
            .iter()
            .copied()
            .filter(|&r| self.map.regions[r].terrain.is_land())
            .fold(None, |best: Option<usize>, r| match best {
                Some(b) if room(b) >= room(r) => Some(b),
                _ => Some(r),
            })
    }

    /// How close the nearest of two peoples' lands are, as `Map::closeness`:
    /// 1 when they share land.
    pub fn nearness(&self, a: usize, b: usize) -> f32 {
        let (la, lb) = (&self.communities[a].lands, &self.communities[b].lands);
        la.iter()
            .flat_map(|&x| lb.iter().map(move |&y| (x, y)))
            .map(|(x, y)| self.map.closeness(x, y))
            .fold(0.0, f32::max)
    }

    /// Travel effort between the nearest of two peoples' lands.
    pub fn apart(&self, a: usize, b: usize) -> f32 {
        let (la, lb) = (&self.communities[a].lands, &self.communities[b].lands);
        la.iter()
            .flat_map(|&x| lb.iter().map(move |&y| (x, y)))
            .map(|(x, y)| self.map.distance(x, y))
            .fold(f32::INFINITY, f32::min)
    }

    /// Whether two peoples hold any land in common.
    pub fn share_land(&self, a: usize, b: usize) -> bool {
        let lb = &self.communities[b].lands;
        self.communities[a].lands.iter().any(|r| lb.contains(r))
    }

    /// Part of `community` moves off as a new community whose speech becomes
    /// a daughter variety; returns the new community's index. The two stay
    /// in contact at `intensity` (0 for none). A people holding many lands
    /// comes apart along them: the leavers keep the far lands, nearer the
    /// furthest of them than the heart, and their share of the people.
    /// A people on one land sends half its number to new land.
    /// The new community names itself as `naming` says, or chooses a name
    /// itself if `None`; its speech is named after it.
    pub fn split(&mut self, community: usize, naming: Option<&Naming>, intensity: f32) -> usize {
        let parent = self.communities[community].variety;
        let mut daughter = self.varieties[parent].fork(parent, self.generation);
        let home = self.communities[community].home();
        let (region, leaving, share) = self.leavers(community);
        // The land they settle, as they say it, which they may be named for.
        let land = self.heard_place(region, parent, &daughter);
        let naming = match naming {
            Some(n) => n.clone(),
            None => {
                let spelled = land.as_ref().map(|l| daughter.title(&l.form));
                self.daughter_naming(community, spelled.as_deref(), region != home)
            }
        };
        let parent_name = &self.communities[community].name;
        let base = |n: &Naming| match n {
            Naming::Land => land.as_ref().map(|l| (l, &daughter)),
            _ => Some((parent_name, &self.varieties[parent])),
        };
        let taken = |name: &Name| {
            self.communities
                .iter()
                .any(|c| c.name.form.segs == name.form.segs)
        };
        let mut name = self.coin(&daughter, &naming, base(&naming));
        // A clipped name can come out as an existing one ("the new Tinea"
        // clipped back to "Tinea"). The land they settle, a place, or an
        // epithet on the old name said short ("the far Goths") tells them
        // apart instead; when all of those are taken, whichever of them
        // needs the fewest syllables more, and only if even every whole
        // name is another's, the chosen one qualified again ("the new
        // Muktho").
        if taken(&name) {
            let short_parent = Name {
                form: crate::names::clipped(
                    parent_name.form.clone(),
                    crate::names::MAX_EPITHET_BASE,
                ),
                ..parent_name.clone()
            };
            let others = land
                .iter()
                .map(|_| Naming::Land)
                .chain(
                    crate::names::PLACES
                        .iter()
                        .map(|p| Naming::Place { place: (*p).into() }),
                )
                .filter(|n| *n != naming)
                .map(|n| {
                    let b = base(&n);
                    (n, b)
                })
                .chain(crate::names::EPITHETS.iter().map(|e| {
                    let epithet = Naming::Epithet {
                        epithet: (*e).into(),
                    };
                    (epithet, Some((&short_parent, &self.varieties[parent])))
                }));
            let wholes: Vec<Name> = std::iter::once((naming.clone(), base(&naming)))
                .chain(others)
                .filter_map(|(n, b)| {
                    let spelled = b.map(|(name, v)| (name, v.title(&name.form)));
                    n.coin_whole(
                        &daughter,
                        spelled.as_ref().map(|(name, s)| (*name, s.as_str())),
                        self.generation,
                    )
                    .ok()
                })
                .collect();
            let longest = wholes.iter().map(|w| w.form.vowel_count()).max();
            let free = (crate::names::MAX_PEOPLE_NAME..=longest.unwrap_or(0)).find_map(|max| {
                wholes
                    .iter()
                    .filter(|w| w.form.vowel_count() >= max)
                    .map(|w| Name {
                        form: crate::names::clipped(w.form.clone(), max),
                        ..w.clone()
                    })
                    .find(|n| !taken(n))
            });
            name = match free {
                Some(free) => free,
                None => {
                    let mut whole = wholes.into_iter().next().unwrap_or(name);
                    let new = Naming::Epithet {
                        epithet: "new".into(),
                    };
                    while taken(&whole) {
                        let spelled = daughter.title(&whole.form);
                        match new.coin_whole(&daughter, Some((&whole, &spelled)), self.generation) {
                            Ok(longer) => whole = longer,
                            Err(_) => break,
                        }
                    }
                    whole
                }
            };
        }
        daughter.name = self.fresh_language_name(&daughter, &name);
        self.varieties.push(daughter);
        let presence = self.presence(community);
        // A people on one land keeps it, whether or not the leavers stay.
        let lands = &self.communities[community].lands;
        let kept: Vec<usize> = if share.is_some() {
            lands.clone()
        } else {
            lands
                .iter()
                .copied()
                .filter(|r| !leaving.contains(r))
                .collect()
        };
        let size = self.communities[community].size;
        let gone = match share {
            Some(share) => size * share,
            None => presence
                .iter()
                .filter(|(r, _)| leaving.contains(r))
                .map(|(_, n)| n)
                .sum(),
        };
        self.communities[community].size = size - gone;
        self.communities[community].lands = kept;
        let mut new = self.communities[community].clone();
        new.name = name;
        new.variety = self.varieties.len() - 1;
        new.size = gone;
        new.lands = leaving;
        self.communities.push(new);
        let index = self.communities.len() - 1;
        if intensity > 0.0 {
            self.link(community, index, intensity, ContactKind::Neighbours);
        }
        self.inherit_state(community, index);
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
    /// Rule goes to the more prestigious side (`a` on a tie), whose state
    /// the other joins.
    pub fn connect(&mut self, a: usize, b: usize, intensity: f32, kind: ContactKind) {
        self.events
            .push((self.generation, WorldEvent::Met { a, b, kind }));
        if kind == ContactKind::Rule {
            let (rulers, ruled) = if self.communities[a].prestige >= self.communities[b].prestige {
                (a, b)
            } else {
                (b, a)
            };
            self.subject(rulers, ruled, intensity);
        } else {
            self.link(a, b, intensity, kind);
        }
    }

    /// A contact with no event of its own, as between the halves of a
    /// split, which the split's event already tells. Two peoples have at
    /// most one contact: a new one between them replaces the old.
    pub(crate) fn link(&mut self, a: usize, b: usize, intensity: f32, kind: ContactKind) {
        self.contacts
            .retain(|k| (k.a, k.b) != (a, b) && (k.a, k.b) != (b, a));
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
            self.renew_names(v);
        }
        self.grow();
        self.spread();
        self.displace();
        self.split_large();
        self.migrate();
        self.adopt();
        self.merge();
        self.shift_languages();
        self.end_contacts();
        self.make_contacts();
        self.hold_states();
        self.rise_states();
        self.standardize();
        self.spread_crafts();
        self.found_religions();
        self.spread_faiths();
        self.learn_words();
        self.reform_spelling();
        self.hold_places();
        self.hear_places();
    }

    /// Which varieties some community still speaks. Unspoken varieties are
    /// extinct: they keep their record but no longer change.
    pub fn spoken(&self) -> Vec<bool> {
        let mut out = vec![false; self.varieties.len()];
        for c in self.living() {
            out[self.communities[c].variety] = true;
        }
        out
    }

    pub(crate) fn community_rng(&self, community: usize, label: &str) -> ChaCha8Rng {
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

    /// Each people's lands: how many they feed it, with what ruling a
    /// state adds, and how many live on them, of every people.
    fn fed_and_crowd(&self, community: usize, occupied: &HashMap<usize, f32>) -> (f32, f32) {
        let k = &self.communities[community];
        let fed = k
            .lands
            .iter()
            .map(|&r| self.feeds(r, k.livelihood))
            .sum::<f32>()
            + self.tribute(community);
        let crowd = k
            .lands
            .iter()
            .map(|r| occupied.get(r).copied().unwrap_or(0.0))
            .sum();
        (fed, crowd)
    }

    /// Logistic growth against what each people's lands feed it, turning
    /// to decline when everyone living there is more than they feed it,
    /// with a little noise. Then bad times strike some lands; a people
    /// left too few to go on ends; and prestige follows from power plus
    /// relative size.
    fn grow(&mut self) {
        let occupied = self.occupation();
        let living: Vec<usize> = self.living().collect();
        for &c in &living {
            let mut rng = self.community_rng(c, "grow");
            let noise = rng.gen_range(-0.05..0.05);
            let (fed, crowd) = self.fed_and_crowd(c, &occupied);
            let room = if fed > 0.0 {
                (1.0 - crowd / fed).max(-1.0)
            } else {
                -1.0
            };
            let rate = self.params.growth_rate * self.communities[c].livelihood.growth();
            self.communities[c].size *= (rate * room + noise).exp();
        }
        self.hard_times(occupied.keys().copied().collect());
        for &c in &living {
            if self.communities[c].size < MIN_PEOPLE {
                self.end(c, None);
            }
        }
        let living: Vec<usize> = self.living().collect();
        if living.is_empty() {
            return;
        }
        let n = living.len() as f32;
        let mean = living
            .iter()
            .map(|&c| self.communities[c].size.ln())
            .sum::<f32>()
            / n;
        for c in living {
            let might = self.might(c);
            let community = &mut self.communities[c];
            let relative = self.params.size_prestige * (community.size.ln() - mean);
            community.prestige = (community.power + relative + might).clamp(0.0, 1.0);
        }
    }

    /// Famine, plague, or drought strikes some of the `peopled` lands,
    /// killing a share of everyone living there. A people living on that
    /// land alone loses that share of itself; one spread over many lands
    /// loses only what lived there, so small peoples suffer worst.
    fn hard_times(&mut self, mut peopled: Vec<usize>) {
        peopled.sort_unstable();
        let generation = self.generation;
        for r in peopled {
            let mut rng = stream(
                self.seed,
                &[
                    key("step"),
                    u64::from(generation),
                    key("hard times"),
                    r as u64,
                ],
            );
            if rng.r#gen::<f32>() >= self.params.hardship_rate {
                continue;
            }
            let dry = matches!(
                self.map.regions[r].terrain,
                Terrain::Steppe | Terrain::Desert
            );
            let kinds: &[Hardship] = if dry {
                &[Hardship::Famine, Hardship::Plague, Hardship::Drought]
            } else {
                &[Hardship::Famine, Hardship::Plague]
            };
            let kind = kinds[crate::rng::index(&mut rng, kinds.len())];
            let share = rng.gen_range(0.2..0.5);
            for c in self.living().collect::<Vec<_>>() {
                let lost: f32 = self
                    .presence(c)
                    .into_iter()
                    .filter(|&(land, _)| land == r)
                    .map(|(_, n)| n * share)
                    .sum();
                self.communities[c].size -= lost;
            }
            self.events.push((
                generation,
                WorldEvent::HardTimes {
                    region: r,
                    kind,
                    share,
                },
            ));
        }
    }

    /// `community` comes to an end: merged `into` another people, which
    /// takes in its number, or dwindled away. It keeps its record and the
    /// lands it last held, but its dealings end without a word.
    fn end(&mut self, community: usize, into: Option<usize>) {
        if let Some(host) = into {
            self.communities[host].size += self.communities[community].size;
        }
        self.communities[community].ended = Some(self.generation);
        self.contacts
            .retain(|k| k.a != community && k.b != community);
        self.events
            .push((self.generation, WorldEvent::Ended { community, into }));
    }

    /// A people using most of what its lands feed it sometimes takes land
    /// beside them where it would have more room, counting part of a
    /// weaker people's land as free, as migrants do; likelier the more
    /// mobile its way of life. It comes to deal with those already there
    /// as neighbours.
    fn spread(&mut self) {
        let mut occupied = self.occupation();
        for c in self.living().collect::<Vec<_>>() {
            let (fed, crowd) = self.fed_and_crowd(c, &occupied);
            if fed <= 0.0 || crowd / fed < SPREAD_FULL {
                continue;
            }
            let k = &self.communities[c];
            let livelihood = k.livelihood;
            let mut rng = self.community_rng(c, "spread");
            if rng.r#gen::<f32>() >= self.params.spread_rate * self.mobility(c) {
                continue;
            }
            // Settling beside their own, they take only land with room
            // left; invaders, who come in force, take land others hold.
            let room = |r: usize| {
                let fed = self.feeds(r, livelihood);
                fed - occupied.get(&r).copied().unwrap_or(0.0)
            };
            let heart = k.home();
            let mut beside: Vec<usize> = k
                .lands
                .iter()
                .flat_map(|&r| self.map.regions[r].neighbours.iter().copied())
                .filter(|r| self.map.regions[*r].terrain.is_land() && !k.lands.contains(r))
                .collect();
            beside.sort_unstable();
            beside.dedup();
            let options: Vec<(usize, f32)> = beside
                .into_iter()
                .map(|r| (r, room(r)))
                .filter(|&(r, f)| f > SPREAD_ROOM * self.feeds(r, livelihood))
                .map(|(r, f)| (r, f / (1.0 + self.map.distance(heart, r))))
                .collect();
            if options.is_empty() {
                continue;
            }
            let to = options[weighted_index(&mut rng, options.iter().map(|(_, w)| *w))].0;
            self.communities[c].lands.push(to);
            self.events
                .push((self.generation, WorldEvent::Spread { community: c, to }));
            self.meet_locals(c, to, &mut rng);
            occupied = self.occupation();
        }
    }

    /// How much room each land has for `community`: what it feeds them,
    /// less everyone else living there, of whom a weaker people counts
    /// only in part, since the locals make room, or are made to.
    fn free_land(&self, community: usize) -> impl Fn(usize) -> f32 + '_ {
        let me = &self.communities[community];
        let mut held: HashMap<usize, f32> = HashMap::new();
        for o in self.living().filter(|&o| o != community) {
            let weight = if self.communities[o].prestige >= me.prestige {
                1.0
            } else {
                YIELD
            };
            for (r, n) in self.presence(o) {
                *held.entry(r).or_default() += n * weight;
            }
        }
        let livelihood = me.livelihood;
        move |r| self.feeds(r, livelihood) - held.get(&r).copied().unwrap_or(0.0)
    }

    /// `community`, newly come to land `to`, deals with those already there
    /// as neighbours.
    fn meet_locals(&mut self, community: usize, to: usize, rng: &mut ChaCha8Rng) {
        let locals: Vec<usize> = self
            .living()
            .filter(|&o| o != community && self.communities[o].lands.contains(&to))
            .filter(|&o| {
                !self
                    .contacts
                    .iter()
                    .any(|k| (k.a, k.b) == (community, o) || (k.a, k.b) == (o, community))
            })
            .collect();
        for o in locals {
            let intensity = rng.gen_range(0.3..0.7);
            self.connect(community, o, intensity, ContactKind::Neighbours);
        }
    }

    /// Who lives on each land, and how many of them.
    fn dwellers(&self) -> HashMap<usize, Vec<(usize, f32)>> {
        let mut out: HashMap<usize, Vec<(usize, f32)>> = HashMap::new();
        for c in self.living() {
            for (r, n) in self.presence(c) {
                out.entry(r).or_default().push((c, n));
            }
        }
        out
    }

    /// A people on several lands gives one up when a stronger people
    /// living there outnumbers it many times over. Losing its heart, it
    /// gathers around the land where most of it lives.
    fn displace(&mut self) {
        let dwellers = self.dwellers();
        for c in self.living().collect::<Vec<_>>() {
            if self.communities[c].lands.len() < 2 {
                continue;
            }
            let mut rng = self.community_rng(c, "displace");
            let prestige = self.communities[c].prestige;
            let mine = self.presence(c);
            let lost = mine.iter().find_map(|&(r, n)| {
                dwellers
                    .get(&r)?
                    .iter()
                    .filter(|&&(o, m)| {
                        o != c && self.communities[o].prestige > prestige && m > n * CROWDED_OUT
                    })
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|&(by, _)| (r, by))
            });
            let Some((region, by)) = lost else { continue };
            if rng.r#gen::<f32>() >= DISPLACE_CHANCE {
                continue;
            }
            let lands = &mut self.communities[c].lands;
            lands.retain(|&r| r != region);
            if mine[0].0 == region {
                let best = mine
                    .iter()
                    .filter(|(r, _)| *r != region)
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|&(r, _)| r)
                    .expect("it keeps a land");
                lands.retain(|&r| r != best);
                lands.insert(0, best);
            }
            self.events.push((
                self.generation,
                WorldEvent::Displaced {
                    community: c,
                    region,
                    by,
                },
            ));
        }
    }

    /// A people too large for its way of life to hold together, or spread
    /// too far from its heart, sometimes comes apart along its lands; the
    /// leavers speak a daughter variety and stay in moderate contact.
    fn split_large(&mut self) {
        for c in self.living().collect::<Vec<_>>() {
            let k = &self.communities[c];
            if k.lands.len() < 2 {
                continue;
            }
            let heart = k.home();
            let too_large = k.size / (self.params.cohesion_size * k.livelihood.cohesion()) - 1.0;
            let reach = k
                .lands
                .iter()
                .map(|&r| self.map.distance(heart, r))
                .fold(0.0, f32::max);
            let too_far = reach / (self.params.cohesion_reach * self.mobility(c)) - 1.0;
            let strain = too_large.max(0.0) + too_far.max(0.0);
            if strain <= 0.0 {
                continue;
            }
            let mut rng = self.community_rng(c, "fission");
            if rng.r#gen::<f32>() < self.params.fission_rate * strain {
                self.split(c, None, FISSION_CONTACT);
            }
        }
    }

    /// Peoples on a single land sometimes leave it for better land within
    /// reach: likelier the more crowded home is, the more mobile its
    /// terrain and way of life make them, and when a stronger people
    /// shares it. A stronger people counts part of a weaker people's land
    /// as free for the taking, so invaders seek out good land others hold.
    /// The newcomers come to deal with those already there as neighbours.
    /// Peoples on many lands spread and part instead.
    fn migrate(&mut self) {
        let dwellers = self.dwellers();
        for c in self.living().collect::<Vec<_>>() {
            if self.communities[c].lands.len() != 1 {
                continue;
            }
            let home = self.communities[c].home();
            let (size, prestige, livelihood) = {
                let k = &self.communities[c];
                (k.size, k.prestige, k.livelihood)
            };
            // Everyone else living at home, and whether a stronger, larger
            // people is among them.
            let mut here = 0.0;
            let mut stronger = false;
            for &(o, n) in dwellers.get(&home).into_iter().flatten() {
                if o == c || !self.communities[o].living() {
                    continue;
                }
                let k = &self.communities[o];
                here += n;
                stronger |= k.prestige > prestige && k.size > size;
            }
            let fed = self.feeds(home, livelihood).max(1.0);
            let crowding = (size + here) / fed;
            let pushed = if stronger { PUSHED } else { 1.0 };
            let mobility = self.map.regions[home].terrain.mobility() * self.mobility(c);
            let hazard = self.params.migration_rate * mobility * crowding * pushed;
            let mut rng = self.community_rng(c, "migrate");
            if rng.r#gen::<f32>() >= hazard {
                continue;
            }
            let free = self.free_land(c);
            let stay = fed - here;
            // Only seafarers cross the sea.
            let sails = self.sails(c);
            let options: Vec<(usize, f32)> = (0..self.map.regions.len())
                .filter(|&r| r != home && self.map.regions[r].terrain.is_land())
                .filter(|&r| self.map.distance(home, r) <= MIGRATION_REACH)
                .filter(|&r| sails || !self.map.overseas(home, r))
                .filter(|&r| free(r) > stay && free(r) >= size / 2.0)
                .map(|r| {
                    let d = self.map.distance(home, r);
                    (r, (free(r) - stay) / ((1.0 + d) * (1.0 + d)))
                })
                .collect();
            if options.is_empty() {
                continue;
            }
            drop(free);
            let to = options[weighted_index(&mut rng, options.iter().map(|(_, w)| *w))].0;
            self.communities[c].lands = vec![to];
            self.events.push((
                self.generation,
                WorldEvent::Migrated {
                    community: c,
                    from: home,
                    to,
                },
            ));
            self.meet_locals(c, to, &mut rng);
        }
    }

    /// Peoples take up a way of life that feeds them far better on their
    /// own lands once they know of it: from a people they deal with,
    /// likelier the closer the dealings; herding from their own farming,
    /// since farmers keep animals; and, rarely, farming of their own accord
    /// on open plain, as it began in a few places in the world.
    fn adopt(&mut self) {
        for c in self.living().collect::<Vec<_>>() {
            let k = &self.communities[c];
            let own = k.livelihood;
            let fed = |l: Livelihood| -> f32 { k.lands.iter().map(|&r| self.feeds(r, l)).sum() };
            let mut options: Vec<(Livelihood, Option<usize>, f32)> = Vec::new();
            for contact in &self.contacts {
                let other = match (contact.a == c, contact.b == c) {
                    (true, _) => contact.b,
                    (_, true) => contact.a,
                    _ => continue,
                };
                let theirs = self.communities[other].livelihood;
                if theirs != own {
                    options.push((
                        theirs,
                        Some(other),
                        self.params.adoption_rate * contact.intensity,
                    ));
                }
            }
            match own {
                Livelihood::Farming => {
                    options.push((Livelihood::Herding, None, self.params.adoption_rate / 2.0))
                }
                Livelihood::Foraging if self.map.regions[k.home()].terrain == Terrain::Plains => {
                    options.push((
                        Livelihood::Farming,
                        None,
                        self.params.adoption_rate * FARMING_FOUND,
                    ))
                }
                _ => {}
            }
            options.retain(|&(l, _, _)| fed(l) >= ADOPT_GAIN * fed(own));
            let total: f32 = options.iter().map(|o| o.2).sum();
            let mut rng = self.community_rng(c, "adopt");
            if options.is_empty() || rng.r#gen::<f32>() >= total {
                continue;
            }
            let (livelihood, from, _) =
                options[weighted_index(&mut rng, options.iter().map(|o| o.2))];
            self.communities[c].livelihood = livelihood;
            self.events.push((
                self.generation,
                WorldEvent::Adopted {
                    community: c,
                    livelihood,
                    from,
                },
            ));
        }
    }

    /// A people far smaller than another living on its heart land, and
    /// speaking a language of the same family, sometimes merges into it,
    /// as dialect speakers are drawn into a larger people of their kin.
    /// Peoples of another family first take up its language.
    fn merge(&mut self) {
        for c in self.living().collect::<Vec<_>>() {
            if !self.communities[c].living() {
                continue;
            }
            let k = &self.communities[c];
            let (heart, size, family) = (k.home(), k.size, self.family(k.variety));
            let host = self
                .living()
                .filter(|&o| o != c && self.communities[o].lands.contains(&heart))
                .filter(|&o| self.family(self.communities[o].variety) == family)
                .filter(|&o| size < MERGE_SHARE * self.communities[o].size)
                .max_by(|&a, &b| {
                    self.communities[a]
                        .size
                        .total_cmp(&self.communities[b].size)
                });
            let Some(host) = host else { continue };
            let mut rng = self.community_rng(c, "merge");
            if rng.r#gen::<f32>() < self.params.merge_rate {
                self.end(c, Some(host));
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
        // Who lives on each land, and how many of them.
        let mut dwellers: HashMap<usize, Vec<(usize, f32)>> = HashMap::new();
        for c in self.living() {
            for (r, n) in self.presence(c) {
                dwellers.entry(r).or_default().push((c, n));
            }
        }
        for r in 0..self.map.regions.len() {
            let here = || dwellers.get(&r).into_iter().flatten().copied();
            let largest = here().fold(None, |best: Option<(usize, f32)>, (i, n)| match best {
                Some((_, s)) if s >= n => best,
                _ => Some((i, n)),
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
                        .filter(|&(k, _)| self.communities[k].variety == p.variety)
                        .map(|(_, n)| n)
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

    /// Peoples name the lands they live on or beside that speakers of
    /// another language hold, as they hear the holders say them: the
    /// holders' own name if their language descends from the namers',
    /// otherwise fitted to their sounds. Each language hears a land once;
    /// the name is then its own word and changes with it.
    fn hear_places(&mut self) {
        let generation = self.generation;
        let mut heard: Vec<(usize, usize, Name)> = Vec::new();
        let mut ears: HashMap<usize, Adapter> = HashMap::new();
        for c in self.living().collect::<Vec<_>>() {
            let v = self.communities[c].variety;
            let mut near: Vec<usize> = self.communities[c]
                .lands
                .iter()
                .flat_map(|&r| {
                    std::iter::once(r).chain(self.map.regions[r].neighbours.iter().copied())
                })
                .collect();
            near.sort_unstable();
            near.dedup();
            for r in near {
                let Some(place) = self.places[r].last() else {
                    continue;
                };
                let known = |(x, _): &(usize, Name)| *x == r;
                if place.variety == v
                    || self.varieties[v].exonyms.iter().any(known)
                    || heard.iter().any(|&(hv, hr, _)| (hv, hr) == (v, r))
                {
                    continue;
                }
                let form = if self.descends(v, place.variety) {
                    place.name.form.clone()
                } else {
                    let mut rng = stream(self.seed, &[key("place exonym"), r as u64, v as u64]);
                    let ear = ears.entry(v).or_insert_with(|| self.ear(v));
                    ear.adapt(&place.name.form, 0.0, &mut rng)
                };
                let name = Name {
                    form,
                    meaning: place.name.meaning.clone(),
                    coined: generation,
                    log: Vec::new(),
                };
                heard.push((v, r, name));
            }
        }
        for (v, r, name) in heard {
            self.varieties[v].exonyms.push((r, name));
        }
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

    /// Where the leavers of a split of `community` go: their new heart, the
    /// lands they take, its heart first, and, for a people on one land,
    /// the share of it that leaves. A people on many lands parts along
    /// them: the furthest from its heart becomes the leavers', with every
    /// land nearer it than the heart.
    fn leavers(&self, community: usize) -> (usize, Vec<usize>, Option<f32>) {
        let c = &self.communities[community];
        let heart = c.home();
        let far = c.lands[1..].iter().copied().max_by(|&a, &b| {
            self.map
                .distance(heart, a)
                .total_cmp(&self.map.distance(heart, b))
        });
        match far {
            Some(far) => {
                let mut leaving: Vec<usize> = c
                    .lands
                    .iter()
                    .copied()
                    .filter(|&r| self.map.distance(r, far) < self.map.distance(r, heart))
                    .collect();
                leaving.sort_by_key(|&r| r != far);
                (far, leaving, None)
            }
            None => {
                let region = self.leavers_land(heart, c.livelihood, self.sails(community));
                (region, vec![region], Some(0.5))
            }
        }
    }

    /// Where a people leaving `home` goes: the roomiest land beside home if
    /// it has more room than home, judged before they go. When the land
    /// beside is full, a seafaring coastal people sends them along or over
    /// the sea instead, to the coast with the most room for the voyage, as
    /// Greek cities sent out colonies.
    fn leavers_land(&self, home: usize, livelihood: Livelihood, sails: bool) -> usize {
        let occupied = self.occupation();
        let room = |r: usize| self.feeds(r, livelihood) - occupied.get(&r).copied().unwrap_or(0.0);
        let colony = || {
            let map = &self.map;
            (0..map.regions.len())
                .filter(|&r| sails && map.coastal(home) && map.coastal(r) && r != home)
                .filter(|&r| !map.regions[home].neighbours.contains(&r))
                .filter(|&r| map.distance(home, r) <= COLONY_REACH && room(r) > room(home))
                .map(|r| (r, room(r) / (1.0 + map.distance(home, r))))
                .fold(None, |best: Option<(usize, f32)>, (r, score)| match best {
                    Some((_, s)) if s >= score => best,
                    _ => Some((r, score)),
                })
                .map(|(r, _)| r)
        };
        match self.roomiest(&self.map.regions[home].neighbours, livelihood) {
            Some(beside) if room(beside) > room(home) => beside,
            _ => colony().unwrap_or(home),
        }
    }

    /// What `region` is called, as speakers of `variety`, a daughter of
    /// `parent`, would say it: as its namers say it if `parent` descends
    /// from them, otherwise fitted to `variety`'s sounds. `None` if no one
    /// has named it yet.
    fn heard_place(&self, region: usize, parent: usize, variety: &Variety) -> Option<Name> {
        let place = self.places[region].last()?;
        if place.variety == parent || self.descends(parent, place.variety) {
            return Some(place.name.clone());
        }
        let mut rng = stream(
            self.seed,
            &[
                key("heard-place"),
                region as u64,
                u64::from(self.generation),
            ],
        );
        let ear = Adapter::new(
            variety.lexicon.living().map(|l| &l.form),
            &variety.profile.inventory,
        );
        Some(Name {
            form: ear.adapt(&place.name.form, 0.0, &mut rng),
            meaning: place.name.meaning.clone(),
            coined: self.generation,
            log: Vec::new(),
        })
    }

    /// What a group leaving `community` calls itself: an epithet on the
    /// old name, a place, or, when `moving`, its new land (`land`, as it
    /// says it), avoiding meanings other communities have. When all of
    /// those are had, it is named for its land even if staying; failing
    /// that, for a place another people in the world is also named for,
    /// as many peoples are "of the river"; and only failing that too, "the
    /// new" old name.
    fn daughter_naming(&self, community: usize, land: Option<&str>, moving: bool) -> Naming {
        let parent = &self.communities[community].name;
        let spelled = self.community_name(community);
        let taken: HashSet<String> = self
            .communities
            .iter()
            .map(|c| c.name.meaning.clone())
            .collect();
        let land = land.filter(|l| !taken.contains(&format!("the people of {l}")));
        let all = Naming::for_daughter(parent);
        let mut options: Vec<(Naming, f32)> = all
            .iter()
            .filter(|(n, _)| {
                let meaning = match n {
                    Naming::Epithet { epithet } => format!("the {epithet} {spelled}"),
                    Naming::Place { place } => format!("the people of the {place}"),
                    _ => return true,
                };
                !taken.contains(&meaning)
            })
            .cloned()
            .collect();
        if moving && land.is_some() {
            options.push((Naming::Land, LAND_NAMING));
        }
        if options.is_empty() {
            if land.is_some() {
                return Naming::Land;
            }
            options = all
                .into_iter()
                .filter(|(n, _)| matches!(n, Naming::Place { .. }))
                .collect();
        }
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
    /// qualifies or the land a people is named for, with the variety that
    /// spells it.
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

    /// A new language's name, as `language_name`, but formed another way
    /// when that name is already a spoken language's.
    fn fresh_language_name(&self, variety: &Variety, people: &Name) -> Name {
        let spoken = self.spoken();
        let taken = |name: &Name| {
            (0..self.varieties.len())
                .any(|v| spoken[v] && self.varieties[v].name.form.segs == name.form.segs)
        };
        let mut ways = crate::names::language_names(
            variety,
            people,
            &variety.title(&people.form),
            self.generation,
        );
        let free = ways.iter().position(|n| !taken(n)).unwrap_or(0);
        ways.swap_remove(free)
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
        self.exonym_heard(&self.ear(self.communities[by].variety), community, by)
    }

    /// How speakers of `variety` fit foreign sounds to their own: build it
    /// once to hear many names, as `exonym_heard` does.
    pub fn ear(&self, variety: usize) -> Adapter {
        let listener = &self.varieties[variety];
        Adapter::new(
            listener.lexicon.living().map(|l| &l.form),
            &listener.profile.inventory,
        )
    }

    /// `exonym`, with the ear of `by`'s language already built.
    pub fn exonym_heard(&self, ear: &Adapter, community: usize, by: usize) -> String {
        let mut rng = stream(self.seed, &[key("exonym"), community as u64, by as u64]);
        let heard = ear.adapt(&self.communities[community].name.form, 0.0, &mut rng);
        self.variety_of(by).title(&heard)
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
    /// already speaks a language of the same family; so may a small people
    /// living among a far larger one, as its neighbours.
    fn shift_languages(&mut self) {
        for c in self.living().collect::<Vec<_>>() {
            let mut options: Vec<(usize, f32)> = Vec::new();
            for contact in &self.contacts {
                let other = match (contact.a == c, contact.b == c) {
                    (true, _) => contact.b,
                    (_, true) => contact.a,
                    _ => continue,
                };
                let swamped = self.communities[c].size < MERGE_SHARE * self.communities[other].size
                    && self.share_land(c, other);
                let factor = match contact.kind {
                    ContactKind::Rule if self.under_standard(c, other) => 2.0 * STANDARD_SHIFT,
                    ContactKind::Rule => 2.0,
                    ContactKind::Intermarriage => 1.5,
                    ContactKind::Neighbours if swamped => 1.0,
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
        let (struck, _) = self.recent_challenges();
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
            let hold = match contact.kind {
                // Peoples on the same land stay neighbours.
                ContactKind::Neighbours if self.share_land(contact.a, contact.b) => continue,
                ContactKind::Rule => {
                    (1.0 + RULE_HOLD * gap) * self.rule_hold(contact.a, contact.b, &struck)
                }
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
            let ruler_first = self.rules_over(a, b)
                || (!self.rules_over(b, a)
                    && self.communities[a].prestige >= self.communities[b].prestige);
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
        for c in self.living().collect::<Vec<_>>() {
            let mut rng = self.community_rng(c, "contact");
            let strangers: Vec<usize> = self
                .living()
                .filter(|&o| o != c && !in_touch(&self.contacts, c, o))
                .collect();
            for &o in strangers.iter().filter(|&&o| o > c) {
                let near = self.nearness(c, o);
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
                    let d = self.apart(c, o);
                    1.0 / ((1.0 + d) * (1.0 + d))
                };
                let o = strangers[weighted_index(&mut rng, strangers.iter().map(|&o| reach(o)))];
                let intensity = rng.gen_range(0.2..0.6);
                self.connect(c, o, intensity, ContactKind::Trade);
            }
            // A people under another's rule makes no conquests of its own.
            if self.ruled_by(c).is_some() {
                continue;
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
                    let hazard = self.params.conquest_rate * gap;
                    (gap > 0.0 && self.ruled_by(other).is_none() && rng.r#gen::<f32>() < hazard)
                        .then_some(i)
                })
                .collect();
            if let Some(&i) = ruled.first() {
                let contact = self.contacts[i];
                let ruled = if contact.a == c { contact.b } else { contact.a };
                self.events
                    .push((self.generation, WorldEvent::Conquered { ruler: c, ruled }));
                self.subject(c, ruled, contact.intensity.max(CONQUEST_INTENSITY));
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
        new.name = self.fresh_language_name(&new, &self.communities[community].name);
        self.varieties.push(new);
        self.communities[community].variety = new_index;
        // So does its name for the lands it holds.
        for region in self.communities[community].lands.clone() {
            if let Some(p) = self.places[region].last().filter(|p| p.variety == old) {
                let kept = PlaceName {
                    variety: new_index,
                    since: generation,
                    name: p.name.clone(),
                    origin: PlaceOrigin::Kept,
                };
                self.places[region].push(kept);
            }
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

    pub(crate) fn at(&self, variety: usize) -> Step {
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

    /// Prestige of each variety: the highest among communities at home in
    /// it, and more for a standing state's standard, whose words carry
    /// the court's standing wherever they go. A faith's sacred language
    /// keeps its standing however few speak it.
    fn variety_prestige(&self) -> Vec<f32> {
        let mut out = vec![0.0_f32; self.varieties.len()];
        for c in &self.communities {
            out[c.variety] = out[c.variety].max(c.prestige);
        }
        for (v, s) in self.standards().into_iter().enumerate() {
            if s.is_some() {
                out[v] += STANDARD_PRESTIGE;
            }
        }
        for r in &self.religions {
            out[r.sacred] = out[r.sacred].max(SACRED_PRESTIGE);
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
        if rng.r#gen::<f32>() >= self.params.sound_change_rate * self.pace(v) {
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
    /// of its language, its peoples, the given names in fashion, and the
    /// lands they hold. Names of the dead (founders of states and faiths)
    /// are kept as they were said.
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
        for community in self
            .communities
            .iter_mut()
            .filter(|c| c.variety == v && c.living())
        {
            let after = law.apply(&community.name.form, minimal);
            community.name.change(after, law.id, generation);
        }
        // And so are the given names in fashion.
        for given in &mut self.varieties[v].given {
            let after = law.apply(&given.name.form, minimal);
            given.name.change(after, law.id, generation);
        }
        // The name of a state its speakers rule changes with their speech.
        for s in 0..self.states.len() {
            let state = &self.states[s];
            if state.standing() && self.communities[state.rulers].variety == v {
                let after = law.apply(&state.name.form, minimal);
                self.states[s].name.change(after, law.id, generation);
            }
        }
        // And so are the names of the lands its speakers hold, each once.
        let mut held: Vec<usize> = self
            .communities
            .iter()
            .filter(|c| c.variety == v && c.living())
            .flat_map(|c| c.lands.iter().copied())
            .collect();
        held.sort_unstable();
        held.dedup();
        for region in held {
            if let Some(p) = self.places[region].last_mut().filter(|p| p.variety == v) {
                let after = law.apply(&p.name.form, minimal);
                p.name.change(after, law.id, generation);
            }
        }
        // And its names for lands others hold.
        for (_, name) in &mut self.varieties[v].exonyms {
            let after = law.apply(&name.form, minimal);
            name.change(after, law.id, generation);
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
            // A standard is held in place against its neighbours' changes
            // as against its own.
            let pace = self.pace(v);
            // For each law on offer: its total pull, and the variety
            // pulling hardest, which is where it is said to come from.
            let mut offers: BTreeMap<&'static str, (f32, usize, f32)> = BTreeMap::new();
            for contact in &self.contacts {
                for (me, other) in [(contact.a, contact.b), (contact.b, contact.a)] {
                    let (m, o) = (&self.communities[me], &self.communities[other]);
                    if m.variety != v || o.variety == v {
                        continue;
                    }
                    let near = 0.2 + 0.8 * self.nearness(me, other);
                    let prestige = (1.0 + 2.0 * (o.prestige - m.prestige)).clamp(0.25, 3.0);
                    let pull = self.params.wave_rate
                        * pace
                        * contact.kind.carries_sounds()
                        * contact.intensity
                        * near
                        * prestige
                        * self.kinship(v, o.variety)
                        * if self.under_standard(me, other) {
                            LEVELLING
                        } else {
                            1.0
                        };
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
    /// prestigious side, and the faithful take words from their faith's
    /// sacred language, the more if they read. A concept's chance of being
    /// borrowed scales with intensity, the recipient's openness, the
    /// prestige gap, the contact's affinity for its field, and its own
    /// borrowability. The donor's current word is adapted to the
    /// recipient's sounds and enters as a competitor; all loans are
    /// decided against this generation's state. No one borrows a word for
    /// a meaning its people has no notion of yet.
    fn borrow(&mut self) {
        struct Loan {
            recipient: usize,
            concept: usize,
            from: usize,
            source: LexemeId,
            source_form: Form,
            form: Form,
        }
        /// Words flowing from a donor variety, with the donor's standing,
        /// to a people, at an intensity, along a kind of contact, as
        /// levelling or not.
        struct Channel {
            donor: usize,
            prestige: f32,
            recipient: usize,
            intensity: f32,
            kind: ContactKind,
            levelled: bool,
        }
        let mut channels: Vec<Channel> = Vec::new();
        for contact in &self.contacts {
            for (donor, recipient) in [(contact.a, contact.b), (contact.b, contact.a)] {
                let (d, r) = (&self.communities[donor], &self.communities[recipient]);
                if d.variety == r.variety {
                    continue;
                }
                // A standard's words reach its subjects' kindred speech far
                // more readily, even basic words (dialect levelling).
                channels.push(Channel {
                    donor: d.variety,
                    prestige: d.prestige,
                    recipient,
                    intensity: contact.intensity,
                    kind: contact.kind,
                    levelled: self.under_standard(recipient, donor)
                        && self.family(d.variety) == self.family(r.variety),
                });
            }
        }
        for c in self.living() {
            let Some(faith) = self.communities[c].faith else {
                continue;
            };
            let reads = self.communities[c].crafts.contains(&Craft::Writing);
            channels.push(Channel {
                donor: self.religions[faith].sacred,
                prestige: SACRED_PRESTIGE,
                recipient: c,
                intensity: SACRED_INTENSITY * if reads { 2.0 } else { 1.0 },
                kind: ContactKind::Religion,
                levelled: false,
            });
        }
        let mut loans = Vec::new();
        let mut adapters: Vec<Option<Adapter>> = vec![None; self.varieties.len()];
        for channel in &channels {
            let r = &self.communities[channel.recipient];
            let levelled = channel.levelled;
            // A purist standard keeps foreign words out.
            let base = self.params.loan_rate
                * channel.intensity
                * r.openness
                * (self.params.prestige_pull * (channel.prestige - r.prestige)).exp()
                * if levelled { LEVELLING } else { 1.0 }
                * (1.0 - self.purism(r.variety));
            let keep_foreign = self.params.bilingual_keep * channel.intensity * r.openness;
            let donor_lexicon = &self.varieties[channel.donor].lexicon;
            let recipient_lexicon = &self.varieties[r.variety].lexicon;
            for (i, concept) in CONCEPTS.iter().enumerate() {
                let borrowability = if levelled {
                    concept.borrowability().max(LEVEL_FLOOR)
                } else {
                    concept.borrowability()
                };
                let hazard = base * channel.kind.affinity(concept.field) * borrowability;
                let mut rng =
                    self.at(r.variety)
                        .rng(&[key("borrow"), channel.donor as u64, key(concept.id)]);
                if rng.r#gen::<f32>() >= hazard {
                    continue;
                }
                if recipient_lexicon.slots[i].variants.is_empty() {
                    continue;
                }
                let Some(source) = donor_lexicon.slots[i].dominant() else {
                    continue;
                };
                // A word the recipient already says alike is nothing new:
                // no levelling, and no learned loan of its own old word.
                if recipient_lexicon.slots[i].dominant().is_some_and(|own| {
                    recipient_lexicon.get(own).form == donor_lexicon.get(source).form
                }) {
                    continue;
                }
                let already = recipient_lexicon.slots[i].variants.iter().any(|v| {
                    recipient_lexicon.get(v.lexeme).origin
                        == Origin::Borrowed {
                            from: channel.donor,
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
                    from: channel.donor,
                    source,
                    source_form,
                    form,
                });
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
    /// concept extends to cover it (sun > day, see > know, and for herders
    /// cattle > wealth), otherwise a new root is coined from the language's
    /// current sounds, avoiding forms its semantic field already uses. A
    /// word that sounds like another or has worn below the minimal word
    /// draws competitors more often, and some of them are the word itself
    /// renewed. Meanings the language has no word for yet wait for their
    /// idea (`ideas::NEEDS`).
    fn innovate(&mut self, v: usize, clashes: &HashSet<LexemeId>) {
        let generation = self.generation;
        let step = self.at(v);
        let hazards: Vec<f32> = CONCEPTS.iter().map(|c| self.innovation_hazard(c)).collect();
        // A standard takes up new words slowly, and a purist one replaces
        // its loans with words of its own.
        let pace = self.pace(v);
        let purism = self.purism(v);
        let livelihood = self
            .speakers(v)
            .map_or(Livelihood::Farming, |c| self.communities[c].livelihood);
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
            let loan = dominant
                .is_some_and(|id| matches!(lexicon.get(id).origin, Origin::Borrowed { .. }));
            let pressure = 1.0
                + f32::from(u8::from(clash)) * params.clash_pressure
                + f32::from(u8::from(worn)) * params.worn_pressure
                + f32::from(u8::from(loan)) * PURIST_PRESSURE * purism;
            if rng.r#gen::<f32>() >= hazards[i] * pressure * pace {
                continue;
            }
            if lexicon.slots[i].variants.is_empty()
                || lexicon.slots[i].variants.len() >= MAX_VARIANTS
            {
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
                .chain(living_related(concept, livelihood))
                .filter_map(|other| lexicon.slot(other).dominant())
                .filter(|id| !lexicon.slots[i].has(*id))
                .collect();
            donors.sort();
            donors.dedup();
            // A word its way of life makes apt (cattle for wealth, among
            // herders) takes more of the uses from the start.
            let apt: Vec<LexemeId> = living_related(concept, livelihood)
                .filter_map(|other| lexicon.slot(other).dominant())
                .collect();

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
            let share = if apt.contains(&newcomer) {
                APT_SHARE
            } else {
                params.newcomer_share
            };
            lexicon.slots[i].introduce(newcomer, share);
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
        let purism = self.purism(v);
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
                    Origin::Borrowed { from, .. } => {
                        (1.0 + params.prestige_selection * (prestige[from] - own))
                            .max(MIN_LOAN_FITNESS)
                            * (1.0 - PURIST_COST * purism)
                    }
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
            // A meaning is either unknown, with no words yet, or shared out
            // among its words.
            let total: f32 = slot.variants.iter().map(|v| v.weight).sum();
            assert!(
                slot.variants.is_empty() || (total - 1.0).abs() < 1e-4,
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
            // Meanings every language has: those waiting for an idea have
            // no word to borrow in a world without ideas.
            let ids: Vec<&str> = CONCEPTS
                .iter()
                .filter(|c| c.field == field && crate::ideas::need(c).is_none())
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
        let known: Vec<&Concept> = CONCEPTS
            .iter()
            .filter(|c| world.varieties[a].lexicon.word_for(c).is_some())
            .collect();
        assert!(
            known.iter().all(|c| world.cognate(a, b, c)),
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
        let land = world.communities[subjects].home();
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
    fn peoples_that_split_off_never_take_a_name_in_use() {
        for seed in 0..6 {
            for profile in ["familiar", "polynesian", "finnic"] {
                let mut world = World::new(seed, Params::static_society());
                world.found(&SoundProfile::by_id(profile).unwrap(), 0.5, 0.5);
                for _ in 0..24 {
                    world.split(0, None, 0.0);
                }
                let names: HashSet<_> = world
                    .communities
                    .iter()
                    .map(|c| &c.name.form.segs)
                    .collect();
                let all: Vec<_> = (0..world.communities.len())
                    .map(|c| {
                        (
                            world.community_name(c),
                            world.communities[c].name.meaning.clone(),
                        )
                    })
                    .collect();
                assert_eq!(
                    names.len(),
                    world.communities.len(),
                    "seed {seed}, {profile}: {all:?}"
                );
            }
        }
    }

    #[test]
    fn names_stay_short_when_every_name_is_had() {
        // Each split from the newest people, all in one generation, so the
        // daughters speak alike and soon run out of places and epithets.
        for seed in 0..4 {
            for profile in ["familiar", "polynesian", "finnic"] {
                let mut world = World::new(seed, Params::static_society());
                world.found(&SoundProfile::by_id(profile).unwrap(), 0.5, 0.5);
                for _ in 0..24 {
                    world.split(world.communities.len() - 1, None, 0.0);
                }
                let names: HashSet<_> = world
                    .communities
                    .iter()
                    .map(|c| &c.name.form.segs)
                    .collect();
                assert_eq!(
                    names.len(),
                    world.communities.len(),
                    "seed {seed}, {profile}"
                );
                for c in &world.communities {
                    let syllables = c.name.form.vowel_count();
                    assert!(
                        syllables <= crate::names::MAX_PEOPLE_NAME + 1,
                        "seed {seed}, {profile}: {} is {syllables} syllables, “{}”",
                        c.name.form.ipa(),
                        c.name.meaning
                    );
                }
            }
        }
    }

    #[test]
    fn two_peoples_keep_one_contact_however_often_they_meet() {
        let mut world = World::new(3, Params::static_society());
        let a = world.found(&SoundProfile::base(), 0.5, 0.5);
        let b = world.found(&SoundProfile::base(), 0.5, 0.5);
        world.connect(a, b, 0.4, ContactKind::Trade);
        world.connect(b, a, 0.7, ContactKind::Religion);
        assert_eq!(world.contacts.len(), 1);
        assert_eq!(world.contacts[0].kind, ContactKind::Religion);
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
    /// clipped, a syllable longer only where the short name is taken.
    #[test]
    fn names_stay_short_over_many_splits() {
        for seed in 0..4 {
            let mut world = World::new(seed, Params::default());
            let a = world.found(&SoundProfile::by_id("finnic").unwrap(), 0.5, 0.5);
            let b = world.found(&SoundProfile::by_id("semitic").unwrap(), 0.8, 0.3);
            world.connect(a, b, 0.6, ContactKind::Rule);
            // Peoples spread over their land before they come apart.
            world.run(300);
            assert!(world.communities.len() > 2, "no splits to test");
            for c in &world.communities {
                assert!(c.name.form.vowel_count() <= 4, "{}", c.name.form.ipa());
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
    fn peoples_spread_but_never_outgrow_what_their_lands_feed_them() {
        for seed in [8, 9, 10] {
            let mut world = World::new(seed, Params::default());
            let design = SoundProfile::base();
            world.found_seeded(
                &Naming::People,
                &design,
                seed,
                0.5,
                0.5,
                None,
                Some(Livelihood::Farming),
            );
            world.run(300);
            let mut spread = false;
            for c in world.living() {
                let k = &world.communities[c];
                let fed: f32 = k.lands.iter().map(|&r| world.feeds(r, k.livelihood)).sum();
                assert!(
                    k.size < fed * 1.3,
                    "seed {seed}: {} on lands feeding {fed}",
                    k.size
                );
                spread |= k.lands.len() > 1;
            }
            assert!(
                spread,
                "seed {seed}: no people ever held more than one land"
            );
        }
    }

    #[test]
    fn a_people_on_many_lands_parts_along_them() {
        let mut world = World::new(4, Params::static_society());
        let c = world.found(&SoundProfile::base(), 0.5, 0.5);
        let heart = world.communities[c].home();
        // Its heart, a land beside it, and the land furthest from it.
        let beside = world.map.regions[heart]
            .neighbours
            .iter()
            .copied()
            .find(|&r| world.map.regions[r].terrain.is_land())
            .unwrap();
        let far = (0..world.map.regions.len())
            .filter(|&r| world.map.regions[r].terrain.is_land())
            .max_by(|&a, &b| {
                world
                    .map
                    .distance(heart, a)
                    .total_cmp(&world.map.distance(heart, b))
            })
            .unwrap();
        world.communities[c].lands = vec![heart, beside, far];
        world.communities[c].size = 9000.0;
        let daughter = world.split(c, None, 0.5);
        assert_eq!(world.communities[c].lands, vec![heart, beside]);
        assert_eq!(world.communities[daughter].lands, vec![far]);
        let total = world.communities[c].size + world.communities[daughter].size;
        assert!((total - 9000.0).abs() < 1.0, "no one is lost: {total}");
    }

    #[test]
    fn place_names_change_with_their_holders_speech_and_freeze_when_left() {
        let mut world = World::new(5, Params::static_society());
        world.found(&SoundProfile::base(), 0.5, 0.5);
        let home = world.communities[0].home();
        let first = world.places[home][0].clone();
        assert_eq!(first.origin, PlaceOrigin::Coined { community: 0 });
        assert_eq!(first.variety, world.communities[0].variety);
        world.run(80);
        let named = &world.places[home][0].name;
        assert!(!named.log.is_empty() || world.variety_of(0).laws.is_empty());

        let elsewhere = (0..world.map.regions.len())
            .find(|&r| r != home && world.map.regions[r].terrain.is_land())
            .unwrap();
        world.communities[0].lands = vec![elsewhere];
        let left = world.places[home][0].name.form.clone();
        world.run(80);
        assert_eq!(world.places[home][0].name.form, left);
        assert_eq!(world.places[elsewhere].len(), 1, "they name their new land");
    }

    #[test]
    fn neighbours_name_a_land_once_and_change_it_their_own_way() {
        let mut diverged = 0;
        for seed in 0..8 {
            let mut world = World::new(seed, Params::static_society());
            let holders = world.found(&SoundProfile::base(), 0.5, 0.5);
            let neighbours = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.5, 0.5);
            let home = world.communities[holders].home();
            let beside = world.map.regions[home]
                .neighbours
                .iter()
                .copied()
                .find(|&r| world.map.regions[r].terrain.is_land())
                .unwrap();
            world.communities[neighbours].lands = vec![beside];
            world.step();
            let v = world.communities[neighbours].variety;
            let heard = |w: &World| {
                let names: Vec<_> = w.varieties[v]
                    .exonyms
                    .iter()
                    .filter(|(r, _)| *r == home)
                    .collect();
                assert_eq!(names.len(), 1, "seed {seed}: heard once");
                names[0].1.clone()
            };
            assert_eq!(heard(&world).meaning, world.places[home][0].name.meaning);
            world.run(160);
            let name = heard(&world);
            let theirs = &world.varieties[v].laws;
            for entry in &name.log {
                let Event::SoundLaw { law, .. } = entry.event else {
                    panic!("seed {seed}: only sound laws change it");
                };
                assert!(
                    theirs.iter().any(|&(_, l)| l == law),
                    "seed {seed}: {law} is not the neighbours' law"
                );
            }
            diverged += usize::from(!name.log.is_empty());
        }
        assert!(diverged >= 4, "only {diverged} of 8 changed it at all");
    }

    #[test]
    fn newcomers_mostly_keep_the_name_of_land_they_take_over() {
        let mut kept = 0;
        for seed in 0..40 {
            let mut world = World::new(seed, Params::static_society());
            let natives = world.found(&SoundProfile::base(), 0.2, 0.5);
            let comers = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.9, 0.5);
            world.connect(natives, comers, 0.5, ContactKind::Trade);
            let land = world.communities[natives].home();
            world.communities[comers].lands = vec![land];
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
            world.run(240);
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
    fn foragers_learn_farming_from_farmers_they_deal_with() {
        let params = Params {
            adoption_rate: 0.1,
            ..Params::static_society()
        };
        let (mut learned, mut alone) = (0, 0);
        for seed in 0..20 {
            for teacher in [Livelihood::Farming, Livelihood::Foraging] {
                let mut world = World::new(seed, params.clone());
                let profile = SoundProfile::base();
                let plain = (0..world.map.regions.len())
                    .find(|&r| world.map.regions[r].terrain == Terrain::Plains)
                    .unwrap();
                let found = |w: &mut World, l| {
                    w.found_seeded(
                        &Naming::People,
                        &profile,
                        seed,
                        0.5,
                        0.5,
                        Some(plain),
                        Some(l),
                    )
                };
                let foragers = found(&mut world, Livelihood::Foraging);
                let other = found(&mut world, teacher);
                world.connect(foragers, other, 0.8, ContactKind::Neighbours);
                world.run(20);
                let farming = world.communities[foragers].livelihood == Livelihood::Farming;
                match teacher {
                    Livelihood::Farming => learned += usize::from(farming),
                    _ => alone += usize::from(farming),
                }
            }
        }
        assert!(learned >= 14, "{learned} of 20 learned from farmers");
        assert!(alone <= 3, "{alone} of 20 began farming beside foragers");
    }

    #[test]
    fn a_people_too_few_to_go_on_ends_and_deals_no_more() {
        let mut world = World::new(2, Params::static_society());
        let a = world.found(&SoundProfile::base(), 0.5, 0.5);
        let b = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.5, 0.5);
        world.connect(a, b, 0.5, ContactKind::Trade);
        world.communities[b].size = MIN_PEOPLE / 2.0;
        world.step();
        assert_eq!(world.communities[b].ended, Some(world.generation));
        assert!(world.contacts.is_empty(), "its dealings end with it");
        assert!(
            !world.spoken()[world.communities[b].variety],
            "no one speaks its language"
        );
        assert!(world.events.iter().any(|(_, e)| *e
            == WorldEvent::Ended {
                community: b,
                into: None
            }));
        world.run(5);
        assert!(world.communities[b].ended.is_some(), "it stays ended");
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
            let land = world.communities[home].home();
            world.communities[stranger].lands = vec![land];
            world.communities[dialect].lands = vec![land];
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

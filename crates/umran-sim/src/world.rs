use crate::adapt::Adapter;
use crate::concepts::{CONCEPTS, Concept, Field, related};
use crate::diglossia::{CLASSICAL_PRESTIGE, Vernacular};
use crate::ethos::{Axis, Effect, Ethos, FoundingEthos, Pole, TemperCause};
use crate::form::Form;
use crate::geography::{CACHE_REACH_KM, LandmassKind, Map, MapSize, REFERENCE_TRAVEL_KM, Terrain};
use crate::grammar::{Category, ImportedPair, MarkerKind, MarkerOrigin, Side};
use crate::ideas::{Craft, Religion, SACRED_INTENSITY, SACRED_PRESTIGE, living_related};
use crate::laws::{Law, catalog};
use crate::lexicon::{Entry, Event, LexemeId, Lexicon, Origin};
use crate::livelihood::Livelihood;
use crate::morphology::Morphology;
use crate::names::{
    ContinentName, Landscape, Name, Naming, PlaceName, PlaceOrigin, continent_name, place_name,
};
use crate::phoneme::PhonemeId;
use crate::phonotactics::Phonotactics;
use crate::polity::{
    Fall, LEVEL_FLOOR, LEVELLING, PURIST_COST, PURIST_PRESSURE, STANDARD_PRESTIGE, STANDARD_SHIFT,
    State,
};
use crate::profile::SoundProfile;
use crate::provenance::LoanCause;
use crate::rng::{index, key, stream, weighted_index};
use crate::root::mint_one;
use crate::schisms::SchismCause;
use crate::variety::Variety;
use rand::{Rng, RngCore};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
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
/// How much likelier a people is to leave land it shares with a stronger
/// people.
const PUSHED: f32 = 2.0;
/// Share of a weaker people's land a stronger newcomer counts as free:
/// the locals make room, or are made to.
const YIELD: f32 = 0.5;
/// How many times larger than the land's namers a people must grow before
/// its own word for the land takes over, so names do not flip back and
/// forth between peoples of about the same size.
pub(crate) const PLACE_HOLD: f32 = 2.0;
/// Chance that newcomers keep a land's old name, fitted to their sounds,
/// rather than coin their own: likelier when they had dealings with the
/// namers, as most river names in England are Celtic.
pub(crate) const PLACE_KEEP_KNOWN: f32 = 0.85;
pub(crate) const PLACE_KEEP_UNKNOWN: f32 = 0.4;
/// Generations a sound change keeps spreading after it takes hold in a
/// variety, its pull fading over them: a wave runs for a few centuries,
/// then the change is simply part of the language.
const WAVE_SPAN: u32 = 10;
/// Generations after a law last took hold in a variety before it may take
/// hold there again. Kinds of change recur: Germanic consonants shifted
/// under Grimm's law and again, in High German, some fifteen centuries
/// later, and vowels lengthen and shorten by turns.
const LAW_RECURRENCE: u32 = 60;
/// Generations apart at which two varieties take up each other's sound
/// changes half as readily as twin dialects do.
const KIN_SPAN: f32 = 20.0;
/// How readily unrelated languages take up each other's sound changes,
/// relative to twin dialects: areal changes do cross families, as the
/// uvular r crossed western Europe, but seldom.
const KIN_STRANGERS: f32 = 0.15;
/// Population below which a people can no longer go on as a people: it
/// dies out, or merges into a people sharing its land.
pub(crate) const MIN_PEOPLE: f32 = 100.0;
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
pub(crate) const ADOPT_GAIN: f32 = 1.5;
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
    /// Population fed by a reference-area plain under farming. Actual
    /// polygon area and livelihood set its share, shared by all residents.
    pub capacity: f32,
    /// Founding separation saturation, in effort-km.
    pub settle_apart: f32,
    /// Population at which a farming people starts to come apart; other
    /// ways of living hold together at their share of it
    /// (`Livelihood::cohesion`).
    pub cohesion_size: f32,
    /// Effort-km from its heartland at which a farming people starts to
    /// come apart; more mobile peoples hold together further.
    pub cohesion_reach: f32,
    /// Chance per generation, per unit of strain beyond holding together,
    /// that a people splits.
    pub fission_rate: f32,
    /// Maximum coast-to-coast colony journey, in effort-km.
    pub colony_reach: f32,
    /// Chance per generation that a people with no room left takes land
    /// beside its own, scaled by how mobile its way of life makes it.
    pub spread_rate: f32,
    /// Chance per generation that famine or plague strikes a peopled land.
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
    /// Maximum symmetric merchant journey, in effort-km.
    pub trade_reach: f32,
    /// Chance per generation, per unit of prestige gap beyond
    /// `CONQUEST_MIN_GAP`, that a people comes to rule one it deals with.
    pub conquest_rate: f32,
    /// Maximum directed expedition or symmetric intermarriage effort-km.
    pub conquest_reach: f32,
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
    /// Maximum whole-people journey, in effort-km.
    pub migration_reach: f32,
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
    /// Chance per generation that an eligible faith divides.
    pub schism_rate: f32,
    /// Chance per faithful people to open a shrine contact each generation.
    pub pilgrimage_rate: f32,
    /// Maximum directed journey to the shrine, in effort-km.
    pub pilgrimage_reach: f32,
    /// Chance per generation that a language takes up a new given name.
    pub name_turnover: f32,
    /// Fraction of the gap to a city's migrant makeup filled per generation.
    /// Zero disables cities, including in an authored state.
    pub city_rate: f32,
    /// False pins founding ethos to zero and disables all shifts, for baseline replay.
    pub ethos_enabled: bool,
    /// False freezes ethos, including authored nudges and inheritance drift.
    pub ethos_shifts: bool,
    /// False freezes seeded regional climate histories at their baseline.
    pub climate_enabled: bool,
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
            settle_apart: 800.0,
            cohesion_size: 100000.0,
            cohesion_reach: 300.0,
            fission_rate: 0.1,
            colony_reach: 1200.0,
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
            trade_reach: 1800.0,
            conquest_rate: 0.05,
            conquest_reach: 800.0,
            state_rate: 0.02,
            collapse_rate: 0.02,
            migration_rate: 0.02,
            migration_reach: 600.0,
            wave_rate: 1.0,
            craft_rate: 0.001,
            idea_rate: 0.02,
            religion_rate: 0.02,
            conversion_rate: 0.05,
            schism_rate: 0.035,
            pilgrimage_rate: 0.02,
            pilgrimage_reach: 1200.0,
            name_turnover: 0.1,
            city_rate: 0.25,
            ethos_enabled: true,
            ethos_shifts: true,
            climate_enabled: true,
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
            schism_rate: 0.0,
            pilgrimage_rate: 0.0,
            name_turnover: 0.0,
            city_rate: 0.0,
            ethos_shifts: false,
            climate_enabled: false,
            ..Self::default()
        }
    }
}

/// A physical journey, measured in equivalent plain kilometres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Journey {
    pub effort: f32,
    pub by_sea: bool,
}

pub(crate) fn row_distance(row: &[(u32, f32)], region: usize) -> f32 {
    row.binary_search_by_key(&(region as u32), |&(r, _)| r)
        .map_or(f32::INFINITY, |i| row[i].1)
}

fn pair(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

pub(crate) fn route_row(
    map: &Map,
    source: usize,
    reach: f32,
    by_sea: bool,
) -> Cow<'_, [(u32, f32)]> {
    if reach <= CACHE_REACH_KM {
        Cow::Borrowed(if by_sea {
            map.voyage_cached(source)
        } else {
            map.walking_cached(source)
        })
    } else if by_sea {
        map.voyage_row(source, reach)
    } else {
        map.walking_row(source, reach)
    }
}

struct JourneyScratch {
    best: Vec<Journey>,
    touched: Vec<usize>,
    rows: HashMap<(usize, bool), Vec<(u32, f32)>>,
}

impl JourneyScratch {
    fn new(regions: usize) -> Self {
        Self {
            best: vec![
                Journey {
                    effort: f32::INFINITY,
                    by_sea: false
                };
                regions
            ],
            touched: Vec::new(),
            rows: HashMap::new(),
        }
    }

    fn iter(&self) -> impl Iterator<Item = (usize, Journey)> + '_ {
        self.touched.iter().map(|&r| (r, self.best[r]))
    }
}

pub(crate) struct Spatial {
    pub occupied: Vec<f32>,
    pub dwellers: Vec<Vec<(usize, f32)>>,
    held: Vec<Vec<usize>>,
}

impl Spatial {
    fn replace(&mut self, world: &World, community: usize, old: &[(usize, f32)]) {
        for &(r, n) in old {
            self.occupied[r] -= n;
            self.dwellers[r].retain(|&(c, _)| c != community);
            self.held[r].retain(|&c| c != community);
        }
        for (r, n) in world.presence_iter(community) {
            self.occupied[r] += n;
            let row = &mut self.dwellers[r];
            let index = row
                .binary_search_by_key(&community, |&(c, _)| c)
                .unwrap_err();
            row.insert(index, (community, n));
        }
        for &r in &world.communities[community].lands {
            let row = &mut self.held[r];
            let index = row.binary_search(&community).unwrap_err();
            row.insert(index, community);
        }
    }

    fn add(&mut self, world: &World, community: usize) {
        for (r, n) in world.presence_iter(community) {
            self.occupied[r] += n;
            self.dwellers[r].push((community, n));
        }
        for &r in &world.communities[community].lands {
            self.held[r].push(community);
        }
    }
}

pub(crate) struct ContactIndex {
    pairs: HashSet<(usize, usize)>,
    adjacent: Vec<Vec<Contact>>,
}

impl ContactIndex {
    pub(crate) fn contains(&self, a: usize, b: usize) -> bool {
        self.pairs.contains(&pair(a, b))
    }

    pub(crate) fn insert(&mut self, contact: Contact) {
        if !self.pairs.insert(pair(contact.a, contact.b)) {
            for c in [contact.a, contact.b] {
                self.adjacent[c].retain(|k| pair(k.a, k.b) != pair(contact.a, contact.b));
            }
        }
        for c in [contact.a, contact.b] {
            self.adjacent[c].push(contact);
        }
    }

    pub(crate) fn partners(
        &self,
        community: usize,
    ) -> impl Iterator<Item = (usize, f32, ContactKind)> + '_ {
        self.adjacent[community].iter().map(move |k| {
            (
                if k.a == community { k.b } else { k.a },
                k.intensity,
                k.kind,
            )
        })
    }
}

/// A group of people with a home variety and a few traits.
#[derive(Clone, Debug, PartialEq)]
pub struct Community {
    /// Direct community ancestry, independent of later language shifts.
    pub parents: Vec<usize>,
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
    pub ethos: Ethos,
    /// Recorded poles in `Axis::ALL` order: high 1, low -1, neither 0.
    pub(crate) temper_marks: [i8; 6],
    /// End-of-generation snapshots, at most one per changed generation.
    pub(crate) ethos_history: Vec<(u32, Ethos)>,
    pub(crate) ethos_challenged: u32,
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
    /// The complete census and route evidence for an authored choice.
    Settlement(Box<crate::settlement::SettlementRecord>),
    /// `community` was founded with a new language.
    Found { community: usize },
    /// `daughter` split off from `community`, speaking a new variety, and
    /// settled region `to`; `from` is the parent's land.
    Split {
        community: usize,
        daughter: usize,
        from: usize,
        to: usize,
        by_sea: bool,
        travelled: bool,
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
        by_sea: bool,
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
    /// A regional climate shift changes harvests, not a recorded death toll.
    Climate {
        zone: usize,
        cause: crate::climate::ClimateCause,
        change: crate::climate::ClimateChange,
        severity: u8,
        wetness: f32,
        warmth: f32,
        lands: Vec<usize>,
        peoples: Vec<usize>,
    },
    /// A fixed river course becomes usable or dries below its flow threshold.
    RiverFlow {
        river: usize,
        flowing: bool,
        cause: crate::climate::ClimateCause,
        lands: Vec<usize>,
        peoples: Vec<usize>,
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
    /// State `state` selected a standard language.
    Standard { state: usize },
    /// A state's capital passed the great-city threshold.
    City { city: usize },
    /// A city's mixed speech became a people and a variety of its own.
    Koine {
        city: usize,
        community: usize,
        variety: usize,
    },
    /// `community` took up `craft`, taught by `from` or by itself.
    Learnt {
        community: usize,
        craft: Craft,
        from: Option<usize>,
    },
    /// Religion `religion` was founded (`Religion::how` says how).
    Revealed { religion: usize },
    /// A branch broke from `parent` among `community`.
    Schism {
        religion: usize,
        parent: usize,
        community: usize,
        cause: SchismCause,
    },
    /// First pilgrims of this faith from `landmass`, once per landmass.
    Pilgrimage {
        religion: usize,
        community: usize,
        landmass: usize,
        from: usize,
        to: usize,
    },
    /// A shrine passed between faithful and other holders.
    HolyLand {
        religion: usize,
        region: usize,
        was_held_by: Option<usize>,
        held_by: Option<usize>,
        faithful: bool,
    },
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
    /// State `state`'s standard was fixed as a classical form
    /// (`State::classical` says how).
    Fixed { state: usize },
    /// Speakers of `variety` began to write their own speech in place of
    /// a classical form.
    Vernacular { variety: usize, by: Vernacular },
    /// An axis entered or left a notable pole, with its cause.
    Temper {
        community: usize,
        axis: Axis,
        pole: Pole,
        entered: bool,
        cause: TemperCause,
    },
}

/// What kind of bad times strike a land.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Hardship {
    Famine,
    Plague,
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
    /// Recorded beginning, or the pilgrimage that created this contact.
    pub cause: Option<crate::Cause>,
}

/// Communities, their varieties, and the contacts between them, stepping
/// through generations of about 25 years.
#[derive(Clone, Debug)]
pub struct World {
    pub seed: u64,
    pub generation: u32,
    /// The fixed geography and route rows for currently usable river valleys.
    pub map: Arc<Map>,
    pub climate: crate::climate::Climate,
    pub communities: Vec<Community>,
    pub varieties: Vec<Variety>,
    pub contacts: Vec<Contact>,
    pub params: Params,
    /// Things that happened to communities, with the generation they
    /// happened in.
    pub events: Vec<(u32, WorldEvent)>,
    /// Optional causes by response event index, within this telling.
    pub causes: std::collections::BTreeMap<usize, crate::Cause>,
    pub(crate) triggers: crate::causes::Triggers,
    /// What each region is called, by every language that has held it,
    /// oldest first; the last is its name now. Empty for land no one has
    /// held, and for the sea.
    pub places: Vec<Vec<PlaceName>>,
    /// Historical hydronyms, indexed by stable river id, not by land.
    /// The last record is the current local name; other languages keep
    /// their own forms in `Variety::river_exonyms`.
    pub river_names: Vec<Vec<PlaceName>>,
    /// Fixed chart headings, indexed by landmass; islands remain unnamed.
    pub continent_names: Vec<Option<ContinentName>>,
    /// Every state that has stood, in the order they arose.
    pub states: Vec<State>,
    pub cities: Vec<crate::cities::City>,
    /// Every religion founded, in order.
    pub religions: Vec<Religion>,
    /// Eligible conquest comparisons whose hazard gained holy-land pressure.
    pub holy_war_checks: u32,
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
        let mut map = Map::generate(seed, size);
        let climate = crate::climate::Climate::new(seed, &map);
        map.set_valley_flows(&climate.flows);
        Self {
            seed,
            generation: 0,
            places: vec![Vec::new(); map.regions.len()],
            river_names: vec![Vec::new(); map.rivers.len()],
            continent_names: vec![None; map.landmasses.len()],
            map: Arc::new(map),
            climate,
            communities: Vec::new(),
            varieties: Vec::new(),
            contacts: Vec::new(),
            params,
            events: Vec::new(),
            causes: Default::default(),
            triggers: Default::default(),
            states: Vec::new(),
            cities: Vec::new(),
            religions: Vec::new(),
            holy_war_checks: 0,
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
        ethos: Option<&FoundingEthos>,
    ) -> usize {
        let index = self.communities.len();
        let region = region.unwrap_or_else(|| self.homeland(index));
        let livelihood = livelihood.unwrap_or_else(|| self.default_livelihood(region));
        let ethos = self.founding_ethos(index, region, livelihood, ethos);
        let mut variety = Variety::found(variety_seed, profile, livelihood, ethos);
        let name = self.coin(&variety, naming, None);
        variety.name = self.language_name(&variety, &name);
        self.varieties.push(variety);
        self.communities.push(Community {
            parents: Vec::new(),
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
            ethos,
            temper_marks: ethos.temper_marks(),
            ethos_history: vec![(self.generation, ethos)],
            ethos_challenged: self.generation,
        });
        self.events
            .push((self.generation, WorldEvent::Found { community: index }));
        self.refresh_places();
        index
    }

    /// Peoples that have not come to an end, by index.
    pub fn living(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.communities.len()).filter(|&c| self.communities[c].living())
    }

    /// Where newly founded community `community` settles: unpeopled land on
    /// a continent, likelier the more it feeds and the further it lies from
    /// other peoples; unpeopled island land once the continents are full;
    /// or the roomiest land when none is unpeopled. Islands are left for
    /// seafarers to find, rather than stranding a founding people on one.
    fn homeland(&self, community: usize) -> usize {
        let peopled: HashSet<usize> = self
            .living()
            .flat_map(|c| self.communities[c].lands.iter().copied())
            .collect();
        let unpeopled = |r: &usize| self.map.regions[*r].terrain.is_land() && !peopled.contains(r);
        let mut open: Vec<usize> = (0..self.map.regions.len())
            .filter(|r| unpeopled(r) && !self.map.island(*r))
            .collect();
        if open.is_empty() {
            open = (0..self.map.regions.len()).filter(unpeopled).collect();
        }
        if open.is_empty() {
            let all: Vec<usize> = (0..self.map.regions.len()).collect();
            return self
                .roomiest(&all, Livelihood::Farming)
                .expect("every map has land");
        }
        let mut nearest = vec![self.params.settle_apart; self.map.regions.len()];
        for &source in &peopled {
            for &(r, effort) in route_row(&self.map, source, self.params.settle_apart, false).iter()
            {
                nearest[r as usize] = nearest[r as usize].min(effort);
            }
        }
        let weight = |r: usize| {
            let fertility = self.feeds(r, self.default_livelihood(r)) / self.params.capacity;
            let apart = nearest[r];
            fertility * fertility * apart
        };
        let mut rng = stream(self.seed, &[key("homeland"), community as u64]);
        open[weighted_index(&mut rng, open.iter().map(|&r| weight(r)))]
    }

    /// How many `region` feeds a people living by `livelihood`.
    pub fn feeds(&self, region: usize, livelihood: Livelihood) -> f32 {
        self.params.capacity * self.climate.regions[region].feeding[livelihood as usize]
    }

    /// Current food can invite valley farming or grazing on a drying plain.
    fn default_livelihood(&self, region: usize) -> Livelihood {
        let baseline = Livelihood::of_land(self.map.regions[region].terrain);
        if self.map.river_regions[region].is_some()
            && self.feeds(region, Livelihood::Farming) > self.feeds(region, baseline) * 1.5
        {
            Livelihood::Farming
        } else if baseline == Livelihood::Farming
            && self.feeds(region, Livelihood::Herding) > self.feeds(region, Livelihood::Farming)
        {
            Livelihood::Herding
        } else {
            baseline
        }
    }

    /// How many of `community` live on each of its lands: its people are
    /// spread over them by how many each feeds them.
    pub fn presence(&self, community: usize) -> Vec<(usize, f32)> {
        self.presence_iter(community).collect()
    }

    pub(crate) fn presence_iter(
        &self,
        community: usize,
    ) -> impl Iterator<Item = (usize, f32)> + '_ {
        let c = &self.communities[community];
        let residence = self.city_residence(community);
        let urban = residence.map_or(0.0, |(_, n)| n);
        let total: f32 = c.lands.iter().map(|&r| self.feeds(r, c.livelihood)).sum();
        let extra = residence.filter(|(r, _)| !c.lands.contains(r));
        c.lands
            .iter()
            .copied()
            .map(move |r| {
                let share = if total > 0.0 {
                    self.feeds(r, c.livelihood) / total
                } else {
                    1.0 / c.lands.len() as f32
                };
                let local = residence
                    .filter(|&(site, _)| site == r)
                    .map_or(0.0, |(_, n)| n);
                (r, (c.size - urban) * share + local)
            })
            .chain(extra)
            .filter(move |_| c.living())
    }

    /// Population living on each region.
    fn occupation(&self) -> Vec<f32> {
        let mut out = vec![0.0; self.map.regions.len()];
        for c in self.living() {
            for (r, n) in self.presence_iter(c) {
                out[r] += n;
            }
        }
        out
    }

    pub(crate) fn spatial(&self) -> Spatial {
        let n = self.map.regions.len();
        let mut view = Spatial {
            occupied: vec![0.0; n],
            dwellers: vec![Vec::new(); n],
            held: vec![Vec::new(); n],
        };
        for c in self.living() {
            for (r, n) in self.presence_iter(c) {
                view.occupied[r] += n;
                view.dwellers[r].push((c, n));
            }
            for &r in &self.communities[c].lands {
                view.held[r].push(c);
            }
        }
        view
    }

    pub(crate) fn contact_index(&self) -> ContactIndex {
        let mut view = ContactIndex {
            pairs: HashSet::with_capacity(self.contacts.len()),
            adjacent: vec![Vec::new(); self.communities.len()],
        };
        for &contact in &self.contacts {
            view.insert(contact);
        }
        view
    }

    /// The land among `regions` with the most room left for a people living
    /// by `livelihood`, lowest index first on a tie; `None` if all of them
    /// are sea.
    fn roomiest(&self, regions: &[usize], livelihood: Livelihood) -> Option<usize> {
        let occupied = self.occupation();
        let room = |r: usize| self.feeds(r, livelihood) - occupied[r];
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
        let (small, large) = if la.len() <= lb.len() {
            (la, lb)
        } else {
            (lb, la)
        };
        self.near_held(small, |r| large.contains(&r))
    }

    fn nearness_indexed(&self, a: usize, b: usize, spatial: &Spatial) -> f32 {
        let (small, other) = if self.communities[a].lands.len() <= self.communities[b].lands.len() {
            (&self.communities[a].lands, b)
        } else {
            (&self.communities[b].lands, a)
        };
        self.near_held(small, |r| spatial.held[r].binary_search(&other).is_ok())
    }

    fn near_held(&self, lands: &[usize], contains: impl Fn(usize) -> bool) -> f32 {
        let mut near: f32 = 0.0;
        for &r in lands {
            if contains(r) {
                return self.map.closeness(r, r);
            }
            for &n in &self.map.regions[r].neighbours {
                if contains(n) {
                    near = near.max(self.map.closeness(r, n));
                }
            }
        }
        near
    }

    /// Exact directed access to a destination, using only the traveller's ships.
    pub fn journey_to(&self, community: usize, region: usize) -> Option<Journey> {
        let mut best = Journey {
            effort: f32::INFINITY,
            by_sea: false,
        };
        for &source in &self.communities[community].lands {
            let walk = self.map.distance(source, region);
            if walk < best.effort || (walk == best.effort && best.by_sea) {
                best = Journey {
                    effort: walk,
                    by_sea: false,
                };
            }
            if self.sails(community) {
                let sea = self.map.voyage(source, region);
                if sea < best.effort {
                    best = Journey {
                        effort: sea,
                        by_sea: true,
                    };
                }
            }
        }
        best.effort.is_finite().then_some(best)
    }

    pub(crate) fn journey_to_within(
        &self,
        community: usize,
        region: usize,
        reach: f32,
    ) -> Option<Journey> {
        let mut best = Journey {
            effort: f32::INFINITY,
            by_sea: false,
        };
        for &source in &self.communities[community].lands {
            for by_sea in [false, true] {
                if by_sea && !self.sails(community) {
                    continue;
                }
                let row = route_row(&self.map, source, reach, by_sea);
                let effort = row_distance(&row, region);
                if effort <= reach
                    && (effort < best.effort || (effort == best.effort && best.by_sea && !by_sea))
                {
                    best = Journey { effort, by_sea };
                }
            }
        }
        best.effort.is_finite().then_some(best)
    }

    /// Exact access from one people to another. Only the departing side supplies ships.
    pub fn journey_between(&self, a: usize, b: usize) -> Option<Journey> {
        self.journey_between_within(a, b, f32::INFINITY)
    }

    fn journey_between_within(&self, a: usize, b: usize, reach: f32) -> Option<Journey> {
        let mut best = Journey {
            effort: f32::INFINITY,
            by_sea: false,
        };
        for &source in &self.communities[a].lands {
            let row = route_row(&self.map, source, reach, false);
            for &destination in &self.communities[b].lands {
                let effort = row_distance(&row, destination);
                if effort <= reach
                    && (effort < best.effort || (effort == best.effort && best.by_sea))
                {
                    best = Journey {
                        effort,
                        by_sea: false,
                    };
                }
            }
            if self.sails(a) {
                let row = route_row(&self.map, source, reach, true);
                for &destination in &self.communities[b].lands {
                    let effort = row_distance(&row, destination);
                    if effort <= reach && effort < best.effort {
                        best = Journey {
                            effort,
                            by_sea: true,
                        };
                    }
                }
            }
        }
        best.effort.is_finite().then_some(best)
    }

    /// Merchant or passenger access can be supplied by either side.
    fn apart_within(&self, a: usize, b: usize, reach: f32) -> f32 {
        [
            self.journey_between_within(a, b, reach),
            self.journey_between_within(b, a, reach),
        ]
        .into_iter()
        .flatten()
        .map(|j| j.effort)
        .fold(f32::INFINITY, f32::min)
    }

    /// Exact symmetric journey effort between the peoples' nearest holdings.
    pub fn apart(&self, a: usize, b: usize) -> f32 {
        self.apart_within(a, b, f32::INFINITY)
    }

    /// Directed reach belongs to the actual ruler, not the subjects.
    pub(crate) fn can_rule(&self, ruler: usize, subject: usize) -> bool {
        self.journey_between_within(ruler, subject, self.params.conquest_reach)
            .is_some()
    }

    /// Reachable destinations in region order, keeping the chosen travel mode.
    #[cfg(test)]
    fn journeys(&self, community: usize, reach: f32) -> Vec<(usize, Journey)> {
        let mut scratch = JourneyScratch::new(self.map.regions.len());
        self.fill_journeys(community, reach, &mut scratch);
        scratch.iter().collect()
    }

    fn fill_journeys(&self, community: usize, reach: f32, scratch: &mut JourneyScratch) {
        for r in scratch.touched.drain(..) {
            scratch.best[r] = Journey {
                effort: f32::INFINITY,
                by_sea: false,
            };
        }
        for &source in &self.communities[community].lands {
            for by_sea in [false, true] {
                if by_sea && !self.sails(community) {
                    continue;
                }
                let row = if reach > CACHE_REACH_KM {
                    Cow::Borrowed(
                        scratch
                            .rows
                            .entry((source, by_sea))
                            .or_insert_with(|| {
                                route_row(&self.map, source, reach, by_sea).into_owned()
                            })
                            .as_slice(),
                    )
                } else {
                    route_row(&self.map, source, reach, by_sea)
                };
                for &(r, effort) in row.iter().filter(|(_, d)| *d <= reach) {
                    let r = r as usize;
                    let old = &mut scratch.best[r];
                    if effort < old.effort || (effort == old.effort && old.by_sea && !by_sea) {
                        if !old.effort.is_finite() {
                            scratch.touched.push(r);
                        }
                        *old = Journey { effort, by_sea };
                    }
                }
            }
        }
        scratch.touched.sort_unstable();
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
        self.split_with_spatial(community, naming, intensity, None)
    }

    fn split_with_spatial(
        &mut self,
        community: usize,
        naming: Option<&Naming>,
        intensity: f32,
        spatial: Option<&mut Spatial>,
    ) -> usize {
        let (region, lands, share, by_sea) = self.leavers(community);
        self.divide(
            community,
            naming,
            intensity,
            crate::settlement::Division {
                region,
                lands,
                share,
                by_sea,
                record: true,
            },
            spatial,
        )
    }

    pub(crate) fn divide(
        &mut self,
        community: usize,
        naming: Option<&Naming>,
        intensity: f32,
        division: crate::settlement::Division,
        spatial: Option<&mut Spatial>,
    ) -> usize {
        let crate::settlement::Division {
            region,
            lands: leaving,
            share,
            by_sea,
            record,
        } = division;
        let affected = self.communities[community].lands.clone();
        if let Some(view) = spatial.as_deref() {
            let contacts = self.contact_index();
            self.hold_places_indexed(Some(&affected), &view.dwellers, &contacts);
        } else {
            self.refresh_places();
        }
        let parent = self.communities[community].variety;
        let mut daughter = self.varieties[parent].fork(parent, self.generation);
        self.inherit_places(parent, &mut daughter);
        let home = self.communities[community].home();
        // Leavers may name themselves for the actual river of their new
        // homeland, using its name as they know it, not the word "river".
        let river = (region != home
            && naming.is_none()
            && stream(
                self.seed,
                &[
                    key("river people"),
                    community as u64,
                    u64::from(self.generation),
                ],
            )
            .r#gen::<f32>()
                < 0.35)
            .then(|| self.nearby_river_name(parent, region))
            .flatten();
        let land = river
            .cloned()
            .or_else(|| self.heard_place(region, parent, &daughter));
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
            // Short names first, then a syllable longer where every short
            // one is taken; never longer than that.
            let longest = wholes
                .iter()
                .map(|w| w.form.vowel_count())
                .max()
                .unwrap_or(0);
            let free = (crate::names::MAX_PEOPLE_NAME
                ..=longest.min(crate::names::MAX_PEOPLE_NAME + 1))
                .find_map(|max| {
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
        let parent = &self.communities[community];
        let new = Community {
            parents: vec![community],
            name,
            variety: self.varieties.len() - 1,
            size: gone,
            lands: leaving,
            prestige: parent.prestige,
            power: parent.power,
            openness: parent.openness,
            livelihood: parent.livelihood,
            ended: None,
            faith: parent.faith,
            crafts: parent.crafts.clone(),
            ethos: parent.ethos,
            temper_marks: parent.ethos.temper_marks(),
            ethos_history: Vec::new(),
            ethos_challenged: parent.ethos_challenged,
        };
        self.communities.push(new);
        let index = self.communities.len() - 1;
        let exposure = self.climate.exposure.get(community).copied().flatten();
        self.climate.exposure.resize(self.communities.len(), None);
        self.climate.exposure[index] = exposure;
        self.divide_city_residents(community, index, size, share.is_none());
        if intensity > 0.0 {
            if !by_sea && self.nearness(community, index) > 0.0 {
                self.link(community, index, intensity, ContactKind::Neighbours);
            } else if self
                .apart_within(community, index, self.params.trade_reach)
                .is_finite()
            {
                self.link(community, index, intensity, ContactKind::Trade);
            }
        }
        self.inherit_state(community, index);
        if record {
            self.events.push((
                self.generation,
                WorldEvent::Split {
                    community,
                    daughter: index,
                    from: home,
                    to: region,
                    by_sea,
                    travelled: share.is_some() && region != home,
                },
            ));
        }
        self.inherit_ethos(index, share.is_some() && region != home);
        self.reconcile_contacts();
        if let Some(view) = spatial {
            view.replace(self, community, &presence);
            view.add(self, index);
            let mut affected = affected;
            affected.extend_from_slice(&self.communities[index].lands);
            affected.sort_unstable();
            affected.dedup();
            let contacts = self.contact_index();
            self.hold_places_indexed(Some(&affected), &view.dwellers, &contacts);
        } else {
            self.refresh_places();
        }
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

    /// Authoring overrides chance, never physical access.
    pub(crate) fn contact_eligible(&self, a: usize, b: usize, kind: ContactKind) -> bool {
        if a == b
            || !self.communities.get(a).is_some_and(Community::living)
            || !self.communities.get(b).is_some_and(Community::living)
        {
            return false;
        }
        match kind {
            ContactKind::Neighbours => self.nearness(a, b) > 0.0,
            ContactKind::Trade => self.apart_within(a, b, self.params.trade_reach).is_finite(),
            ContactKind::Intermarriage => self
                .apart_within(a, b, self.params.conquest_reach)
                .is_finite(),
            ContactKind::Religion => self
                .apart_within(a, b, self.params.pilgrimage_reach)
                .is_finite(),
            ContactKind::Rule => {
                let (ruler, subject) =
                    if self.communities[a].prestige >= self.communities[b].prestige {
                        (a, b)
                    } else {
                        (b, a)
                    };
                self.can_rule(ruler, subject)
            }
        }
    }

    /// Brings `a` and `b` into contact, as an event of the world's history.
    /// Rule goes to the more prestigious side (`a` on a tie), whose state
    /// the other joins.
    pub fn connect(
        &mut self,
        a: usize,
        b: usize,
        intensity: f32,
        kind: ContactKind,
    ) -> Result<(), String> {
        if !self.contact_eligible(a, b, kind) {
            return Err(format!(
                "these peoples cannot sustain {kind:?} contact within physical reach"
            ));
        }
        let changes_rule = kind == ContactKind::Rule
            || self
                .contacts
                .iter()
                .any(|k| k.kind == ContactKind::Rule && pair(k.a, k.b) == pair(a, b));
        let event = self.events.len();
        self.events
            .push((self.generation, WorldEvent::Met { a, b, kind }));
        if kind == ContactKind::Rule {
            let (rulers, ruled) = if self.communities[a].prestige >= self.communities[b].prestige {
                (a, b)
            } else {
                (b, a)
            };
            self.subject(rulers, ruled, intensity, event);
        } else {
            self.link(a, b, intensity, kind);
            self.contacts.last_mut().unwrap().cause = Some(crate::Cause {
                event,
                mechanism: crate::Mechanism::Contact,
            });
        }
        if changes_rule {
            self.reconcile_contacts();
        }
        Ok(())
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
            cause: None,
        });
    }

    fn rule_relation(&self, contact: &Contact) -> Option<(usize, usize, usize)> {
        for (ruler, subject) in [(contact.a, contact.b), (contact.b, contact.a)] {
            if let Some(state) = self.ruled_by(subject)
                && self.states[state].rulers == ruler
            {
                return Some((state, ruler, subject));
            }
        }
        None
    }

    fn part_contact(&mut self, contact: Contact) {
        let Some(index) = self
            .contacts
            .iter()
            .position(|k| k.kind == contact.kind && pair(k.a, k.b) == pair(contact.a, contact.b))
        else {
            return;
        };
        let relation = (contact.kind == ContactKind::Rule)
            .then(|| self.rule_relation(&contact))
            .flatten();
        let (a, b) = if let Some((state, ruler, subject)) = relation {
            self.leave(state, subject);
            (ruler, subject)
        } else {
            self.contacts.remove(index);
            if matches!(contact.kind, ContactKind::Rule | ContactKind::Intermarriage)
                && self.contact_eligible(contact.a, contact.b, ContactKind::Neighbours)
            {
                self.link(
                    contact.a,
                    contact.b,
                    contact.intensity / 2.0,
                    ContactKind::Neighbours,
                );
            }
            (contact.a, contact.b)
        };
        self.events.push((
            self.generation,
            WorldEvent::Parted {
                a,
                b,
                kind: contact.kind,
            },
        ));
        if contact.kind == ContactKind::Rule {
            self.freed_ethos(b);
        }
    }

    /// Territorial changes end inaccessible relations, even without turnover.
    pub(crate) fn reconcile_contacts(&mut self) {
        let invalid: Vec<_> = self
            .contacts
            .iter()
            .copied()
            .filter(|k| {
                if k.kind == ContactKind::Rule {
                    !self.communities[k.a].living()
                        || !self.communities[k.b].living()
                        || self
                            .rule_relation(k)
                            .is_none_or(|(_, ruler, subject)| !self.can_rule(ruler, subject))
                } else {
                    !self.contact_eligible(k.a, k.b, k.kind)
                }
            })
            .collect();
        for contact in invalid {
            self.part_contact(contact);
        }
        self.reconcile_memberships();
        for state in 0..self.states.len() {
            if !self.states[state].standing() {
                continue;
            }
            let ruler = self.states[state].rulers;
            if !self.communities[ruler].living() {
                self.fall(state, Fall::RulersEnded);
            } else if !self.communities[ruler]
                .lands
                .contains(&self.states[state].capital)
            {
                self.fall(state, Fall::CapitalLost);
            }
        }
    }

    fn reconcile_memberships(&mut self) {
        let rules: HashSet<_> = self
            .contacts
            .iter()
            .filter(|k| k.kind == ContactKind::Rule)
            .map(|k| pair(k.a, k.b))
            .collect();
        let mut orphaned = Vec::new();
        for (state, s) in self.states.iter().enumerate().filter(|(_, s)| s.standing()) {
            for community in s.subjects() {
                if !self.communities[community].living()
                    || !rules.contains(&pair(s.rulers, community))
                {
                    orphaned.push((state, community));
                }
            }
        }
        for (state, community) in orphaned {
            self.leave(state, community);
        }
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
        self.reconcile_contacts();
        self.preserve_places();
        self.generation += 1;
        if self.advance_climate() {
            self.reconcile_contacts();
        }
        let spoken = self.spoken();
        let areal = self.areal_targets();
        for (v, targets) in areal.iter().enumerate() {
            if spoken[v] {
                self.sound_change(v, targets);
            }
        }
        self.spread_waves(&spoken);
        self.borrow();
        self.grammar_contact(&spoken);
        let prestige = self.variety_prestige();
        for v in (0..self.varieties.len()).filter(|&v| spoken[v]) {
            let clashes = self.varieties[v].lexicon.clashing();
            self.innovate(v, &clashes);
            self.drift(v, &clashes, &prestige);
            self.retire(v);
            let variety = &mut self.varieties[v];
            let stress = variety.stress();
            variety.grammar.evolve(
                self.seed,
                v,
                self.generation,
                &mut variety.lexicon,
                &variety.morphology,
                stress,
                self.params.speakers,
            );
            self.renew_names(v);
        }
        self.grow();
        self.spread();
        self.displace();
        self.split_large();
        self.migrate();
        self.reconcile_contacts();
        self.adopt();
        self.merge();
        self.reconcile_contacts();
        self.shift_languages();
        self.end_contacts();
        self.make_contacts();
        self.hold_states();
        self.reconcile_contacts();
        self.rise_states();
        self.grow_cities();
        self.standardize();
        self.spread_crafts();
        self.found_religions();
        self.spread_faiths();
        self.divide_faiths();
        self.send_pilgrims();
        self.observe_holy_lands();
        self.learn_words();
        self.reform_spelling();
        self.fix_classics();
        self.hold_places();
        self.hear_places();
        self.name_continents();
        self.temper_generation();
        for variety in &mut self.varieties {
            variety.sync_grammar(self.generation);
        }
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
    fn fed_and_crowd(&self, community: usize, occupied: &[f32]) -> (f32, f32) {
        let k = &self.communities[community];
        let fed = k
            .lands
            .iter()
            .map(|&r| self.feeds(r, k.livelihood))
            .sum::<f32>()
            + self.tribute(community);
        let crowd = k.lands.iter().map(|&r| occupied[r]).sum();
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
            if crowd >= fed {
                self.communities[c].ethos_challenged = self.generation;
            }
            let room = if fed > 0.0 {
                (1.0 - crowd / fed).max(-1.0)
            } else {
                -1.0
            };
            let rate = self.params.growth_rate * self.communities[c].livelihood.growth();
            self.communities[c].size *= crate::math::exp(rate * room + noise);
        }
        self.hard_times(&occupied);
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
            .map(|&c| crate::math::ln(self.communities[c].size))
            .sum::<f32>()
            / n;
        for c in living {
            let might = self.might(c);
            let community = &mut self.communities[c];
            let relative = self.params.size_prestige * (crate::math::ln(community.size) - mean);
            community.prestige = (community.power + relative + might).clamp(0.0, 1.0);
        }
    }

    /// Famine or plague strikes some of the occupied lands,
    /// killing a share of everyone living there. A people living on that
    /// land alone loses that share of itself; one spread over many lands
    /// loses only what lived there, so small peoples suffer worst.
    fn hard_times(&mut self, occupied: &[f32]) {
        let peopled: Vec<_> = occupied
            .iter()
            .enumerate()
            .filter(|(_, n)| **n > 0.0)
            .map(|(r, _)| r)
            .collect();
        let generation = self.generation;
        let dwellers = self.spatial().dwellers;
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
            let kinds = [Hardship::Famine, Hardship::Plague];
            let kind = kinds[crate::rng::index(&mut rng, kinds.len())];
            let share = rng.gen_range(0.2..0.5);
            for &(c, _) in &dwellers[r] {
                let lost = self
                    .presence_iter(c)
                    .find(|&(land, _)| land == r)
                    .unwrap()
                    .1
                    * share;
                if lost > 0.0 {
                    let (fed, crowd) = self.fed_and_crowd(c, occupied);
                    self.hardship_ethos(c, crowd >= fed * 0.9);
                }
                self.communities[c].size -= lost;
            }
            self.record_event(WorldEvent::HardTimes {
                region: r,
                kind,
                share,
            });
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
        let mut spatial = self.spatial();
        let mut contacts = self.contact_index();
        let mut marks = vec![0usize; self.map.regions.len()];
        let mut before = Vec::new();
        for c in self.living().collect::<Vec<_>>() {
            let (fed, crowd) = self.fed_and_crowd(c, &spatial.occupied);
            if fed <= 0.0 || crowd / fed < SPREAD_FULL {
                continue;
            }
            let k = &self.communities[c];
            let livelihood = k.livelihood;
            let mut rng = self.community_rng(c, "spread");
            if rng.r#gen::<f32>()
                >= self.params.spread_rate
                    * self.mobility(c)
                    * self.communities[c].ethos.factor(Effect::Spread)
            {
                continue;
            }
            // Settling beside their own, they take only land with room
            // left; invaders, who come in force, take land others hold.
            let room = |r: usize| {
                let fed = self.feeds(r, livelihood);
                fed - spatial.occupied[r]
            };
            let heart = k.home();
            let distances = self.map.walking_row(heart, f32::INFINITY);
            let mark = c + 1;
            for &r in &k.lands {
                marks[r] = mark;
            }
            let mut beside = Vec::new();
            for &r in &k.lands {
                for &n in &self.map.regions[r].neighbours {
                    if marks[n] != mark && self.map.regions[n].terrain.is_land() {
                        marks[n] = mark;
                        beside.push(n);
                    }
                }
            }
            beside.sort_unstable();
            let options: Vec<(usize, f32)> = beside
                .into_iter()
                .map(|r| (r, room(r)))
                .filter(|&(r, f)| f > SPREAD_ROOM * self.feeds(r, livelihood))
                .map(|(r, f)| {
                    (
                        r,
                        f / (1.0 + row_distance(&distances, r) / REFERENCE_TRAVEL_KM),
                    )
                })
                .collect();
            if options.is_empty() {
                continue;
            }
            let to = options[weighted_index(&mut rng, options.iter().map(|(_, w)| *w))].0;
            before.clear();
            before.extend(self.presence_iter(c));
            self.communities[c].lands.push(to);
            self.events
                .push((self.generation, WorldEvent::Spread { community: c, to }));
            self.meet_locals(c, to, &mut rng, &spatial, &mut contacts);
            spatial.replace(self, c, &before);
        }
    }

    /// How much room each land has for `community`: what it feeds them,
    /// less everyone else living there, of whom a weaker people counts
    /// only in part, since the locals make room, or are made to.
    pub(crate) fn free_room(&self, community: usize, region: usize, spatial: &Spatial) -> f32 {
        let me = &self.communities[community];
        let held: f32 = spatial.dwellers[region]
            .iter()
            .filter(|&&(c, _)| c != community)
            .map(|&(c, n)| {
                n * if self.communities[c].prestige >= me.prestige {
                    1.0
                } else {
                    YIELD
                }
            })
            .sum();
        self.feeds(region, me.livelihood) - held
    }

    /// `community`, newly come to land `to`, deals with those already there
    /// as neighbours.
    fn meet_locals(
        &mut self,
        community: usize,
        to: usize,
        rng: &mut ChaCha8Rng,
        spatial: &Spatial,
        contacts: &mut ContactIndex,
    ) {
        for &(other, _) in &spatial.dwellers[to] {
            if other == community
                || contacts.contains(community, other)
                || self.nearness(community, other) <= 0.0
            {
                continue;
            }
            let intensity = rng.gen_range(0.3..0.7);
            self.connect(community, other, intensity, ContactKind::Neighbours)
                .expect("arrivals share land with the locals");
            contacts.insert(*self.contacts.last().unwrap());
        }
    }

    /// Who lives on each land, and how many of them.
    pub(crate) fn dwellers(&self) -> Vec<Vec<(usize, f32)>> {
        let mut dwellers = vec![Vec::new(); self.map.regions.len()];
        for c in self.living() {
            for (r, n) in self.presence_iter(c) {
                dwellers[r].push((c, n));
            }
        }
        dwellers
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
                    .get(r)?
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
            let cause = self.neighbour_cause(c, by);
            self.record_response(
                WorldEvent::Displaced {
                    community: c,
                    region,
                    by,
                },
                cause,
            );
            self.communities[c].ethos_challenged = self.generation;
        }
    }

    /// A people too large for its way of life to hold together, or spread
    /// too far from its heart, sometimes comes apart along its lands; the
    /// leavers speak a daughter variety and stay in moderate contact.
    fn split_large(&mut self) {
        let mut spatial = self.spatial();
        for c in self.living().collect::<Vec<_>>() {
            let k = &self.communities[c];
            if k.lands.len() < 2 {
                continue;
            }
            let heart = k.home();
            let distances = self.map.walking_row(heart, f32::INFINITY);
            let too_large = k.size / (self.params.cohesion_size * k.livelihood.cohesion()) - 1.0;
            let reach = k
                .lands
                .iter()
                .map(|&r| row_distance(&distances, r))
                .fold(0.0, f32::max);
            let too_far = reach
                / (self.params.cohesion_reach
                    * self.mobility(c)
                    * k.ethos.factor(Effect::Cohesion))
                - 1.0;
            let strain = too_large.max(0.0) + too_far.max(0.0);
            if strain <= 0.0 {
                continue;
            }
            let mut rng = self.community_rng(c, "fission");
            if rng.r#gen::<f32>() < self.params.fission_rate * strain {
                self.split_with_spatial(c, None, FISSION_CONTACT, Some(&mut spatial));
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
        let mut spatial = self.spatial();
        let dwellers = spatial.dwellers.clone();
        let mut contacts = self.contact_index();
        let mut journeys = JourneyScratch::new(self.map.regions.len());
        let mut before = Vec::new();
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
            let mut stronger_cause = None;
            for &(o, n) in &dwellers[home] {
                if o == c || !self.communities[o].living() {
                    continue;
                }
                let k = &self.communities[o];
                here += n;
                stronger |= k.prestige > prestige && k.size > size;
                if k.prestige > prestige && k.size > size && stronger_cause.is_none() {
                    stronger_cause = self.neighbour_cause(c, o);
                }
            }
            let fed = self.feeds(home, livelihood).max(1.0);
            let crowding = (size + here) / fed;
            let pushed = if stronger { PUSHED } else { 1.0 };
            let mobility = self.map.regions[home].terrain.mobility() * self.mobility(c);
            let hazard = self.params.migration_rate
                * mobility
                * crowding
                * pushed
                * self.communities[c].ethos.factor(Effect::Migration);
            let mut rng = self.community_rng(c, "migrate");
            if rng.r#gen::<f32>() >= hazard {
                continue;
            }
            let stay = fed - here;
            self.fill_journeys(c, self.params.migration_reach, &mut journeys);
            let options: Vec<(usize, Journey, f32)> = journeys
                .iter()
                .filter(|&(r, _)| r != home)
                .filter_map(|(r, journey)| {
                    let room = self.free_room(c, r, &spatial);
                    if room <= stay || room < size / 2.0 {
                        return None;
                    }
                    let d = journey.effort / REFERENCE_TRAVEL_KM;
                    Some((r, journey, (room - stay) / ((1.0 + d) * (1.0 + d))))
                })
                .collect();
            if options.is_empty() {
                continue;
            }
            let (to, journey, _) = options[weighted_index(&mut rng, options.iter().map(|o| o.2))];
            before.clear();
            before.extend(self.presence_iter(c));
            self.communities[c].lands = vec![to];
            let cause = stronger_cause.or_else(|| self.feeding_cause(home, livelihood));
            self.record_response(
                WorldEvent::Migrated {
                    community: c,
                    from: home,
                    to,
                    by_sea: journey.by_sea,
                },
                cause,
            );
            self.meet_locals(c, to, &mut rng, &spatial, &mut contacts);
            spatial.replace(self, c, &before);
        }
    }

    /// Peoples take up a way of life that feeds them far better on their
    /// own lands once they know of it: from a people they deal with,
    /// likelier the closer the dealings; herding from their own farming,
    /// since farmers keep animals; and, rarely, farming of their own accord
    /// on suitable plains or river valleys.
    fn adopt(&mut self) {
        let contacts = self.contact_index();
        for c in self.living().collect::<Vec<_>>() {
            let k = &self.communities[c];
            let own = k.livelihood;
            let fed = |l: Livelihood| -> f32 { k.lands.iter().map(|&r| self.feeds(r, l)).sum() };
            let mut options: Vec<(Livelihood, Option<usize>, f32)> = Vec::new();
            for (other, intensity, _) in contacts.partners(c) {
                let theirs = self.communities[other].livelihood;
                if theirs != own {
                    options.push((theirs, Some(other), self.params.adoption_rate * intensity));
                }
            }
            match own {
                Livelihood::Farming => {
                    options.push((Livelihood::Herding, None, self.params.adoption_rate / 2.0))
                }
                Livelihood::Foraging | Livelihood::Herding
                    if self.map.regions[k.home()].terrain == Terrain::Plains
                        || (self.map.river_regions[k.home()].is_some()
                            && self.feeds(k.home(), Livelihood::Farming)
                                >= ADOPT_GAIN * self.feeds(k.home(), own)) =>
                {
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
            let cause = self.adoption_cause(c, own, livelihood);
            self.communities[c].livelihood = livelihood;
            self.record_response(
                WorldEvent::Adopted {
                    community: c,
                    livelihood,
                    from,
                },
                cause,
            );
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

    /// Named lands remembered by this language, whether held, surveyed, or heard.
    pub fn known_lands(&self, variety: usize) -> Vec<usize> {
        let mut lands: Vec<usize> = self.varieties[variety]
            .exonyms
            .iter()
            .map(|(region, _)| *region)
            .chain(
                self.places
                    .iter()
                    .enumerate()
                    .filter(|(_, names)| names.iter().any(|p| p.variety == variety))
                    .map(|(region, _)| region),
            )
            .collect();
        lands.sort_unstable();
        lands.dedup();
        lands
    }

    /// This language's first-heard memory, or its latest own local name.
    /// Never substitutes the present holder's unrelated words.
    pub fn known_place(&self, variety: usize, region: usize) -> Option<&Name> {
        self.varieties[variety]
            .exonyms
            .iter()
            .find(|(r, _)| *r == region)
            .map(|(_, name)| name)
            .or_else(|| {
                self.places[region]
                    .iter()
                    .rev()
                    .find(|p| p.variety == variety)
                    .map(|p| &p.name)
            })
    }

    /// Snapshots the parent's local land and river names into a fork's
    /// evolving memories. Already cloned exonyms win local conflicts.
    pub fn inherit_places(&self, parent: usize, daughter: &mut Variety) {
        for (region, names) in self.places.iter().enumerate() {
            if !daughter.exonyms.iter().any(|(r, _)| *r == region)
                && let Some(place) = names.iter().rev().find(|p| p.variety == parent)
            {
                daughter.exonyms.push((region, place.name.clone()));
            }
        }
        self.inherit_river_names(parent, daughter);
    }

    /// Keeps the last local name before its language loses a land or its
    /// naming record is superseded. The historical local record stays frozen.
    fn remember_place(&mut self, region: usize) {
        if let Some(place) = self.places[region].last()
            && !self.varieties[place.variety]
                .exonyms
                .iter()
                .any(|(r, _)| *r == region)
        {
            self.varieties[place.variety]
                .exonyms
                .push((region, place.name.clone()));
        }
    }

    fn preserve_places(&mut self) {
        let dwellers = self.dwellers();
        self.preserve_river_names_indexed(&dwellers);
        self.preserve_places_indexed(None, &dwellers);
    }

    fn preserve_places_indexed(
        &mut self,
        affected: Option<&[usize]>,
        dwellers: &[Vec<(usize, f32)>],
    ) {
        let count = affected.map_or(dwellers.len(), <[usize]>::len);
        for index in 0..count {
            let region = affected.map_or(index, |regions| regions[index]);
            if let Some(place) = self.places[region].last()
                && !dwellers[region]
                    .iter()
                    .any(|&(c, _)| self.communities[c].variety == place.variety)
            {
                self.remember_place(region);
            }
        }
    }

    /// Local discovery at API boundaries never spends an indirect report.
    pub(crate) fn refresh_places(&mut self) {
        self.hold_places();
        self.hear_places_phase(false);
        self.name_continents();
    }

    fn name_continents(&mut self) {
        for landmass in 0..self.map.landmasses.len() {
            if self.map.landmasses[landmass].kind != LandmassKind::Continent
                || self.continent_names[landmass].is_some()
            {
                continue;
            }
            let witness = self.living().find_map(|people| {
                let community = &self.communities[people];
                let variety = community.variety;
                let held = community
                    .lands
                    .iter()
                    .copied()
                    .filter(|&r| self.map.regions[r].landmass == Some(landmass))
                    .filter(|&r| self.known_place(variety, r).is_some())
                    .min();
                let known = || {
                    self.map.landmasses[landmass]
                        .regions
                        .iter()
                        .copied()
                        .filter_map(|r| self.known_place(variety, r).map(|name| (r, name.coined)))
                        .min_by_key(|&(r, since)| (since, r))
                        .map(|(r, _)| r)
                };
                held.or_else(known).map(|region| (people, variety, region))
            });
            let Some((people, variety, region)) = witness else {
                continue;
            };
            let speech = &self.varieties[variety];
            let mut rng = stream(
                self.seed,
                &[key("continent name"), landmass as u64, variety as u64],
            );
            let foreign =
                self.map.regions[self.communities[people].home()].landmass != Some(landmass);
            let Some(name) = continent_name(
                speech,
                self.known_place(variety, region).unwrap(),
                &self.communities[people].name,
                foreign,
                &mut rng,
                self.generation,
            ) else {
                continue;
            };
            self.continent_names[landmass] = Some(ContinentName {
                spelled: speech.title(&name.form),
                ipa: name.form.ipa_stressed(speech.stress()),
                meaning: name.meaning,
                variety,
                people,
                witness: region,
                since: self.generation,
            });
        }
    }

    /// Each land is called what the people holding it calls it: the
    /// largest people living there, once it outnumbers the land's namers
    /// `PLACE_HOLD` times over. A people whose language descends from the
    /// namers' inherits the name; others mostly keep it, fitted to their
    /// own sounds, or coin their own. Land no one lives on keeps its last
    /// name unchanged.
    fn hold_places(&mut self) {
        let dwellers = self.dwellers();
        let contacts = self.contact_index();
        self.hold_places_indexed(None, &dwellers, &contacts);
    }

    fn hold_places_indexed(
        &mut self,
        affected: Option<&[usize]>,
        dwellers: &[Vec<(usize, f32)>],
        contacts: &ContactIndex,
    ) {
        self.preserve_places_indexed(affected, dwellers);
        self.refresh_river_names_indexed(dwellers);
        let generation = self.generation;
        let count = affected.map_or(dwellers.len(), <[usize]>::len);
        for index in 0..count {
            let r = affected.map_or(index, |regions| regions[index]);
            let here = || dwellers[r].iter().copied();
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
                            || contacts
                                .partners(holder)
                                .any(|(other, _, _)| self.communities[other].variety == p.variety);
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
                (Some(PlaceOrigin::Inherited), Some(p)) => {
                    self.known_place(variety, r).unwrap_or(&p.name).clone()
                }
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
                    let coined = place_name(
                        speech,
                        land,
                        (people, &spelled),
                        self.nearby_river_name(variety, r),
                        &mut rng,
                        generation,
                    );
                    let Some(name) = coined else { continue };
                    name
                }
            };
            let origin = origin.unwrap_or(PlaceOrigin::Coined { community: holder });
            self.remember_place(r);
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

    /// Once-per-generation hearing: local observations and partner lands,
    /// plus one indirect phase-start report per contact in each direction.
    fn hear_places(&mut self) {
        self.hear_places_phase(true);
    }

    pub(crate) fn place_heard(
        &self,
        receiver: usize,
        source: usize,
        name: &Name,
        rng: &mut impl Rng,
        ears: &mut HashMap<usize, Adapter>,
    ) -> Name {
        let form = if receiver == source || self.descends(receiver, source) {
            name.form.clone()
        } else {
            ears.entry(receiver)
                .or_insert_with(|| self.ear(receiver))
                .adapt(&name.form, 0.0, rng)
        };
        Name {
            form,
            meaning: name.meaning.clone(),
            coined: self.generation,
            log: Vec::new(),
        }
    }

    fn hear_places_phase(&mut self, reports: bool) {
        let mut heard: BTreeMap<(usize, usize), Name> = BTreeMap::new();
        let mut ears = HashMap::new();
        let mut known = HashSet::new();
        let mut knowledge = vec![Vec::new(); self.varieties.len()];
        for (v, variety) in self.varieties.iter().enumerate() {
            known.extend(variety.exonyms.iter().map(|(r, _)| (v, *r)));
        }
        for (r, names) in self.places.iter().enumerate() {
            known.extend(names.iter().map(|p| (p.variety, r)));
        }
        for &(v, r) in &known {
            knowledge[v].push(r);
        }
        for row in &mut knowledge {
            row.sort_unstable();
        }
        for c in self.living() {
            let v = self.communities[c].variety;
            let mut near: Vec<usize> = self.communities[c]
                .lands
                .iter()
                .flat_map(|&r| {
                    std::iter::once(r).chain(self.map.regions[r].neighbours.iter().copied())
                })
                .filter(|&r| self.map.regions[r].terrain.is_land())
                .collect();
            near.sort_unstable();
            near.dedup();
            for r in near {
                if known.contains(&(v, r)) || heard.contains_key(&(v, r)) {
                    continue;
                }
                let name = if let Some(place) = self.places[r].last() {
                    let mut rng = stream(self.seed, &[key("place exonym"), r as u64, v as u64]);
                    self.place_heard(v, place.variety, &place.name, &mut rng, &mut ears)
                } else {
                    let mut rng = stream(self.seed, &[key("place survey"), r as u64, v as u64]);
                    let speech = &self.varieties[v];
                    let people = &self.communities[c].name;
                    let Some(name) = place_name(
                        speech,
                        Landscape {
                            terrain: self.map.regions[r].terrain,
                            coastal: self.map.coastal(r),
                            island: self.map.island(r),
                        },
                        (people, &speech.title(&people.form)),
                        self.nearby_river_name(v, r),
                        &mut rng,
                        self.generation,
                    ) else {
                        continue;
                    };
                    name
                };
                heard.insert((v, r), name);
            }
        }
        let mut contacts: Vec<(usize, usize)> = self
            .contacts
            .iter()
            .filter(|k| self.communities[k.a].living() && self.communities[k.b].living())
            .map(|k| (k.a.min(k.b), k.a.max(k.b)))
            .collect();
        contacts.sort_unstable();
        contacts.dedup();
        // Held-land hearing takes precedence over indirect reports, regardless
        // of which contact would otherwise have been visited first.
        for &(a, b) in &contacts {
            for (speaker, listener) in [(a, b), (b, a)] {
                let (source, receiver) = (
                    self.communities[speaker].variety,
                    self.communities[listener].variety,
                );
                for &r in &self.communities[speaker].lands {
                    if known.contains(&(receiver, r)) || heard.contains_key(&(receiver, r)) {
                        continue;
                    }
                    let Some(name) = self.known_place(source, r) else {
                        continue;
                    };
                    let mut rng =
                        stream(self.seed, &[key("place exonym"), r as u64, receiver as u64]);
                    heard.insert(
                        (receiver, r),
                        self.place_heard(receiver, source, name, &mut rng, &mut ears),
                    );
                }
            }
        }
        if reports {
            for &(a, b) in &contacts {
                for (direction, (speaker, listener)) in [(a, b), (b, a)].into_iter().enumerate() {
                    let (source, receiver) = (
                        self.communities[speaker].variety,
                        self.communities[listener].variety,
                    );
                    let candidates: Vec<usize> = knowledge[source]
                        .iter()
                        .copied()
                        .filter(|&r| {
                            !known.contains(&(receiver, r)) && !heard.contains_key(&(receiver, r))
                        })
                        .collect();
                    if candidates.is_empty() {
                        continue;
                    }
                    let mut rng = stream(
                        self.seed,
                        &[
                            key("place report"),
                            u64::from(self.generation),
                            a as u64,
                            b as u64,
                            direction as u64,
                        ],
                    );
                    let r = candidates[index(&mut rng, candidates.len())];
                    let name = self.known_place(source, r).unwrap();
                    heard.insert(
                        (receiver, r),
                        self.place_heard(receiver, source, name, &mut rng, &mut ears),
                    );
                }
            }
        }
        for ((v, r), name) in heard {
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
    fn leavers(&self, community: usize) -> (usize, Vec<usize>, Option<f32>, bool) {
        let c = &self.communities[community];
        let heart = c.home();
        let heart_row = self.map.walking_row(heart, f32::INFINITY);
        let far = c.lands[1..]
            .iter()
            .copied()
            .max_by(|&a, &b| row_distance(&heart_row, a).total_cmp(&row_distance(&heart_row, b)));
        match far {
            Some(far) => {
                let far_row = self.map.walking_row(far, f32::INFINITY);
                let mut leaving: Vec<usize> = c
                    .lands
                    .iter()
                    .copied()
                    .filter(|&r| row_distance(&far_row, r) < row_distance(&heart_row, r))
                    .collect();
                leaving.sort_by_key(|&r| r != far);
                (far, leaving, None, false)
            }
            None => {
                let (region, by_sea) = self.leavers_land(
                    heart,
                    c.livelihood,
                    self.sails(community),
                    c.ethos.factor(Effect::Colony),
                );
                (region, vec![region], Some(0.5), by_sea)
            }
        }
    }

    /// Where a people leaving `home` goes: the roomiest land beside home if
    /// it has more room than home, judged before they go. When the land
    /// beside is full, a seafaring coastal people sends them along or over
    /// the sea instead, to the coast with the most room for the voyage, as
    /// Greek cities sent out colonies.
    fn leavers_land(
        &self,
        home: usize,
        livelihood: Livelihood,
        sails: bool,
        seaward: f32,
    ) -> (usize, bool) {
        let occupied = self.occupation();
        let room = |r: usize| self.feeds(r, livelihood) - occupied[r];
        let attraction = |r| {
            if self.map.overseas(home, r) {
                seaward
            } else {
                1.0
            }
        };
        if let Some(beside) = self.roomiest(&self.map.regions[home].neighbours, livelihood)
            && room(beside) > room(home)
        {
            return (beside, false);
        }
        if !sails || !self.map.coastal(home) {
            return (home, false);
        }
        let row = route_row(&self.map, home, self.params.colony_reach, true);
        let colony = row
            .iter()
            .copied()
            .filter(|&(r, d)| {
                d <= self.params.colony_reach
                    && !self.map.regions[home].neighbours.contains(&(r as usize))
                    && room(r as usize) * attraction(r as usize) > room(home)
            })
            .map(|(r, d)| {
                (
                    r as usize,
                    room(r as usize) / (1.0 + d / REFERENCE_TRAVEL_KM) * attraction(r as usize),
                )
            })
            .fold(None, |best: Option<(usize, f32)>, (r, score)| match best {
                Some((_, s)) if s >= score => best,
                _ => Some((r, score)),
            });
        colony.map_or((home, false), |(r, _)| (r, true))
    }

    /// What `region` is called, as speakers of `variety`, a daughter of
    /// `parent`, would say it: as its namers say it if `parent` descends
    /// from them, otherwise fitted to `variety`'s sounds. `None` if no one
    /// has named it yet.
    pub(crate) fn heard_place(
        &self,
        region: usize,
        parent: usize,
        variety: &Variety,
    ) -> Option<Name> {
        if let Some((_, name)) = variety.exonyms.iter().find(|(r, _)| *r == region) {
            return Some(name.clone());
        }
        if let Some(name) = self.known_place(parent, region) {
            return Some(name.clone());
        }
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
    pub(crate) fn coin(
        &self,
        variety: &Variety,
        naming: &Naming,
        base: Option<(&Name, &Variety)>,
    ) -> Name {
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
    pub(crate) fn fresh_language_name(&self, variety: &Variety, people: &Name) -> Name {
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
                    _ if self.under_standard(c, other) => 2.0 * STANDARD_SHIFT,
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
        for contact in &self.contacts {
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
                ended.push(*contact);
            }
        }
        for contact in ended.into_iter().rev() {
            self.part_contact(contact);
        }
    }

    /// The world makes contacts of its own: peoples on the same or
    /// bordering land deal with each other again, peoples open trade,
    /// nearer partners likelier, and a people far above one it deals with
    /// may come to rule it.
    fn make_contacts(&mut self) {
        let spatial = self.spatial();
        let mut contacts = self.contact_index();
        let mut journeys = JourneyScratch::new(self.map.regions.len());
        let mut neighbour_marks = vec![0usize; self.communities.len()];
        let mut neighbours = Vec::new();
        let mut effort = vec![f32::INFINITY; self.communities.len()];
        let mut partners = Vec::new();
        for c in self.living().collect::<Vec<_>>() {
            let mut rng = self.community_rng(c, "contact");
            neighbours.clear();
            for &r in &self.communities[c].lands {
                for land in std::iter::once(r).chain(self.map.regions[r].neighbours.iter().copied())
                {
                    if !self.map.regions[land].terrain.is_land() {
                        continue;
                    }
                    for &other in &spatial.held[land] {
                        if other > c
                            && neighbour_marks[other] != c + 1
                            && !contacts.contains(c, other)
                        {
                            neighbour_marks[other] = c + 1;
                            neighbours.push(other);
                        }
                    }
                }
            }
            neighbours.sort_unstable();
            for &other in &neighbours {
                let near = self.nearness_indexed(c, other, &spatial);
                if near > 0.0 && rng.r#gen::<f32>() < self.params.neighbour_rate * near {
                    let intensity = rng.gen_range(0.3..0.7) * near;
                    self.connect(c, other, intensity, ContactKind::Neighbours)
                        .expect("neighbours have a land border");
                    contacts.insert(*self.contacts.last().unwrap());
                }
            }
            for other in partners.drain(..) {
                effort[other] = f32::INFINITY;
            }
            self.fill_journeys(c, self.params.trade_reach, &mut journeys);
            let mut offer = |region: usize, distance: f32, incoming: bool| {
                for &other in &spatial.held[region] {
                    if other == c || contacts.contains(c, other) || (incoming && !self.sails(other))
                    {
                        continue;
                    }
                    if !effort[other].is_finite() {
                        partners.push(other);
                    }
                    effort[other] = effort[other].min(distance);
                }
            };
            for (region, journey) in journeys.iter() {
                offer(region, journey.effort, false);
            }
            // The partner may carry the initiating people's merchants.
            for &source in &self.communities[c].lands {
                let row = if self.params.trade_reach > CACHE_REACH_KM {
                    Cow::Borrowed(
                        journeys
                            .rows
                            .entry((source, true))
                            .or_insert_with(|| {
                                route_row(&self.map, source, self.params.trade_reach, true)
                                    .into_owned()
                            })
                            .as_slice(),
                    )
                } else {
                    route_row(&self.map, source, self.params.trade_reach, true)
                };
                for &(r, distance) in row.iter().filter(|(_, d)| *d <= self.params.trade_reach) {
                    offer(r as usize, distance, true);
                }
            }
            partners.sort_unstable();
            if !partners.is_empty()
                && rng.r#gen::<f32>()
                    < self.params.trade_rate * self.communities[c].ethos.factor(Effect::Contact)
            {
                let other = partners[weighted_index(
                    &mut rng,
                    partners.iter().map(|&o| {
                        let d = effort[o] / REFERENCE_TRAVEL_KM;
                        1.0 / ((1.0 + d) * (1.0 + d))
                    }),
                )];
                let intensity = rng.gen_range(0.2..0.6);
                self.connect(c, other, intensity, ContactKind::Trade)
                    .expect("trade candidates are physically eligible");
                contacts.insert(*self.contacts.last().unwrap());
            }
            if self.ruled_by(c).is_some() {
                continue;
            }
            let me = self.communities[c].prestige;
            let mut conquered = None;
            for &contact in &contacts.adjacent[c] {
                if contact.kind == ContactKind::Rule {
                    continue;
                }
                let other = if contact.a == c { contact.b } else { contact.a };
                let gap = me - self.communities[other].prestige - CONQUEST_MIN_GAP;
                let eligible =
                    gap > 0.0 && self.ruled_by(other).is_none() && self.can_rule(c, other);
                let holy = eligible && self.holy_war_target(c, other);
                if holy {
                    self.holy_war_checks += 1;
                }
                let factor = if holy { 2.0 } else { 1.0 };
                let hazard = (self.params.conquest_rate
                    * gap
                    * factor
                    * self.communities[c].ethos.factor(Effect::Conquest))
                .min(1.0);
                if eligible && rng.r#gen::<f32>() < hazard && conquered.is_none() {
                    conquered = Some(contact);
                }
            }
            if let Some(contact) = conquered {
                let ruled = if contact.a == c { contact.b } else { contact.a };
                let cause = self.holy_war_cause(c, ruled);
                let event = self.record_response(WorldEvent::Conquered { ruler: c, ruled }, cause);
                self.subject(c, ruled, contact.intensity.max(CONQUEST_INTENSITY), event);
                contacts = self.contact_index();
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
        self.refresh_places();
        let generation = self.generation;
        let old = self.communities[community].variety;
        let source = self.communities[target].variety;
        let old_sounds = self.varieties[old].established();
        let mut new = self.varieties[source].fork(source, generation);
        self.inherit_places(source, &mut new);
        new.profile = self.varieties[old].profile.clone();
        new.profile.stress = self.varieties[source].profile.stress;
        let stress = new.stress();
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
                let after = merge.apply(&lexeme.form, stress);
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
            let law = Law {
                id: "substrate",
                label: "Substrate sounds merge",
                rules: vec![merge],
                commonness: 0.0,
                stress: None,
            };
            new.grammar
                .apply_law(&mut new.lexicon, &law, new.minimal, stress, generation);
            for (_, name) in &mut new.river_exonyms {
                name.change(&law, new.minimal, stress, generation);
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
            let cause = LoanCause::Shift {
                community,
                from_variety: old,
            };
            let origin = Origin::Borrowed {
                from: old,
                source: word,
                cause,
            };
            let id = new.lexicon.coin(form.clone(), origin, concept, generation);
            new.lexicon.get_mut(id).log.push(Entry {
                generation,
                event: Event::Borrowed {
                    from: old,
                    source: form,
                    cause,
                },
            });
            new.lexicon.slots[i].introduce(id, self.params.loan_share);
        }
        let mut memory_rng = stream(
            self.seed,
            &[
                key("place memory shift"),
                u64::from(generation),
                community as u64,
                new_index as u64,
            ],
        );
        let mut ear = None;
        for region in self.known_lands(old) {
            if new.exonyms.iter().any(|(r, _)| *r == region) {
                continue;
            }
            let name = self.known_place(old, region).unwrap();
            let adapter = ear.get_or_insert_with(|| {
                Adapter::new(
                    new.lexicon.living().map(|l| &l.form),
                    &new.profile.inventory,
                )
            });
            new.exonyms.push((
                region,
                Name {
                    form: adapter.adapt(&name.form, 0.0, &mut memory_rng),
                    meaning: name.meaning.clone(),
                    coined: generation,
                    log: Vec::new(),
                },
            ));
        }
        new.sync_grammar(generation);
        new.grammar.simplify(
            &mut new.lexicon,
            stress,
            self.seed ^ community as u64,
            generation,
        );
        self.shift_river_names(community, old, &mut new);
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
                self.remember_place(region);
                self.places[region].push(kept);
            }
        }
        self.keep_river_names(community, old);
        self.events.push((
            generation,
            WorldEvent::Shift {
                community,
                from: old,
                toward: target,
                variety: new_index,
            },
        ));
        self.refresh_places();
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
    /// and a state's classical form keep their standing however few speak
    /// them.
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
        for s in &self.states {
            if let Some(k) = s.classical {
                out[k.variety] = out[k.variety].max(CLASSICAL_PRESTIGE);
            }
        }
        out
    }

    /// Per-generation chance that `concept` gains a native competitor.
    pub fn innovation_hazard(&self, concept: &Concept) -> f32 {
        let rank = concept.stability.map_or(CULTURAL_RANK, f32::from);
        self.params.innovation_rate
            * crate::math::pow(self.params.stability_spread, (rank - 50.5) / 99.0)
    }

    /// Maybe applies one new sound law to every living word. Laws that
    /// would change nothing are skipped, as are laws that took hold here
    /// lately (`LAW_RECURRENCE`); laws that move sounds toward the
    /// culture's preferences, or toward the sounds of its contacts, are
    /// likelier.
    fn sound_change(&mut self, v: usize, areal: &[(HashSet<PhonemeId>, f32)]) {
        let mut rng = self.at(v).rng(&[key("sound")]);
        if rng.r#gen::<f32>() >= self.params.sound_change_rate * self.pace(v) {
            return;
        }
        let variety = &self.varieties[v];
        let recent = self.recent_laws(v);
        let prior = &variety.profile.inventory;
        let candidates: Vec<(&Law, f32)> = self
            .laws
            .iter()
            .filter(|law| !recent.contains(law.id))
            .filter_map(|law| {
                let a = law.assess_weighted(
                    variety.grammar.forms(&variety.lexicon),
                    prior,
                    variety.minimal,
                    variety.stress(),
                )?;
                let areal: f32 = areal.iter().map(|(target, w)| w * a.toward(target)).sum();
                let bias = crate::math::exp(
                    self.params.preference_pull * a.pull.clamp(-5.0, 3.0)
                        + self.params.areal_pull * areal,
                );
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
    /// lands and rivers they hold. Names of the dead (founders of states
    /// and faiths) are kept as they were said.
    pub(crate) fn apply_law(&mut self, v: usize, law: &Law) {
        let generation = self.generation;
        let variety = &mut self.varieties[v];
        let minimal = variety.minimal;
        let stress = variety.stress();
        for lexeme in &mut variety.lexicon.lexemes {
            if lexeme.obsolete.is_some() {
                continue;
            }
            let after = law.apply(&lexeme.form, minimal, stress);
            if law.changes(&lexeme.form, &after, stress) {
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
        variety
            .grammar
            .apply_law(&mut variety.lexicon, law, minimal, stress, generation);
        variety.laws.push((generation, law.id));
        if let Some(next) = law.stress {
            variety.stress_history.push((generation, stress));
            variety.profile.stress = Some(next);
        }
        // Names are words too.
        variety.name.change(law, minimal, stress, generation);
        for community in self
            .communities
            .iter_mut()
            .filter(|c| c.variety == v && c.living())
        {
            community.name.change(law, minimal, stress, generation);
        }
        // And so are the given names in fashion.
        for given in &mut self.varieties[v].given {
            given.name.change(law, minimal, stress, generation);
        }
        // The name of a state its speakers rule changes with their speech.
        for s in 0..self.states.len() {
            let state = &self.states[s];
            if state.standing() && self.communities[state.rulers].variety == v {
                self.states[s].name.change(law, minimal, stress, generation);
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
        self.change_river_names(v, law, minimal, stress, &held);
        for region in held {
            if let Some(p) = self.places[region].last_mut().filter(|p| p.variety == v) {
                p.name.change(law, minimal, stress, generation);
            }
        }
        // And its names for lands others hold.
        for (_, name) in &mut self.varieties[v].exonyms {
            name.change(law, minimal, stress, generation);
        }
    }

    /// Sound changes spread like waves. A law that took hold in a variety
    /// in the last `WAVE_SPAN` generations may pass to a variety in
    /// contact with it that has not had it recently: likelier through close
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
            let recent = self.recent_laws(v);
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
                        * self.city_wave_weight(me, other)
                        * prestige
                        * self.kinship(v, o.variety)
                        * if self.under_standard(me, other) {
                            LEVELLING
                        } else {
                            1.0
                        };
                    for &(g, id) in &self.varieties[o.variety].laws {
                        let age = generation.saturating_sub(g);
                        if age >= WAVE_SPAN || recent.contains(id) {
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
                    let a = law.assess_weighted(
                        variety.grammar.forms(&variety.lexicon),
                        &variety.profile.inventory,
                        variety.minimal,
                        variety.stress(),
                    )?;
                    let taste =
                        crate::math::exp(self.params.preference_pull * a.pull.clamp(-5.0, 3.0))
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
            self.varieties[v].waves.push((generation, law.id, from));
        }
    }

    /// Laws that took hold in variety `v` within the last
    /// `LAW_RECURRENCE` generations, which may not take hold again yet.
    fn recent_laws(&self, v: usize) -> HashSet<&'static str> {
        let generation = self.generation;
        self.varieties[v]
            .laws
            .iter()
            .filter(|&&(g, _)| generation.saturating_sub(g) < LAW_RECURRENCE)
            .map(|&(_, id)| id)
            .collect()
    }

    /// How readily varieties `a` and `b` take up each other's sound
    /// changes: 1 for dialects that have just parted, half that once they
    /// have been apart `KIN_SPAN` generations, falling on toward
    /// `KIN_STRANGERS`, which is all unrelated languages get.
    pub(crate) fn kinship(&self, a: usize, b: usize) -> f32 {
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
    /// prestigious side, the faithful take words from their faith's
    /// sacred language, and writers from the classical form they write,
    /// the more if they read. A concept's chance of being
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
            cause: LoanCause,
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
            cause: LoanCause,
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
                    cause: self.contact_loan_cause(contact, donor, recipient),
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
                cause: LoanCause::Faith {
                    religion: faith,
                    teacher: None,
                    recipient: c,
                },
            });
        }
        for (c, high, intensity, standing) in self.classical_sources() {
            channels.push(Channel {
                donor: high,
                prestige: standing,
                recipient: c,
                intensity,
                kind: ContactKind::Rule,
                levelled: false,
                cause: LoanCause::Classical {
                    classical: high,
                    recipient: c,
                },
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
                * crate::math::exp(self.params.prestige_pull * (channel.prestige - r.prestige))
                * if levelled { LEVELLING } else { 1.0 }
                * (1.0 - self.purism(r.variety))
                * r.ethos.factor(Effect::Borrowing);
            let keep_foreign = self.params.bilingual_keep
                * channel.intensity
                * r.openness
                * r.ethos.factor(Effect::Borrowing);
            let donor_lexicon = &self.varieties[channel.donor].lexicon;
            let recipient_lexicon = &self.varieties[r.variety].lexicon;
            for (i, concept) in CONCEPTS.iter().enumerate() {
                // Each concept has its own stream. Reject impossible loans
                // before constructing it; accepted loans keep the same draws.
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
                    recipient_lexicon
                        .get(v.lexeme)
                        .origin
                        .is_loan_from(channel.donor, source)
                });
                if already {
                    continue;
                }
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
                    cause: channel.cause,
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
                cause: loan.cause,
            };
            // A donor word already borrowed for another sense is the same
            // loanword gaining a meaning, not a second borrowing.
            let existing = lexicon
                .living()
                .find(|l| l.origin.is_loan_from(loan.from, loan.source))
                .map(|l| l.id);
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
                            cause: loan.cause,
                        },
                    });
                    id
                }
            };
            lexicon.slots[loan.concept].introduce(id, self.params.loan_share);
        }
    }

    /// Native marking is normal. Strong bilingual contact occasionally carries pairs.
    fn grammar_contact(&mut self, spoken: &[bool]) {
        let generation = self.generation;
        let mut strong: BTreeMap<(usize, usize), f32> = BTreeMap::new();
        for contact in &self.contacts {
            if !matches!(
                contact.kind,
                ContactKind::Neighbours | ContactKind::Intermarriage | ContactKind::Rule
            ) || contact.intensity < 0.75
            {
                continue;
            }
            for (recipient, donor) in [(contact.a, contact.b), (contact.b, contact.a)] {
                let (r, d) = (&self.communities[recipient], &self.communities[donor]);
                if r.variety != d.variety && r.openness >= 0.5 {
                    let entry = strong.entry((r.variety, d.variety)).or_default();
                    *entry = entry.max(contact.intensity * r.openness);
                }
            }
        }
        for v in (0..self.varieties.len()).filter(|&v| spoken[v]) {
            self.varieties[v].sync_grammar(generation);
            self.varieties[v]
                .grammar
                .contact_generations
                .retain(|from, _| strong.contains_key(&(v, *from)));
        }
        for ((recipient, donor), intensity) in strong {
            let duration = self.varieties[recipient]
                .grammar
                .contact_generations
                .entry(donor)
                .or_default();
            *duration += 1;
            let sustained = *duration >= 8;
            let new_loans: Vec<_> = self.varieties[recipient]
                .lexicon
                .living()
                .filter(|l| {
                    l.born == generation
                        && matches!(l.origin, Origin::Borrowed { from, .. } if from == donor)
                })
                .map(|l| l.id)
                .collect();
            if !new_loans.is_empty() {
                let own = &self.varieties[recipient];
                let adapter = Adapter::new(
                    own.lexicon.living().map(|l| &l.form),
                    &own.profile.inventory,
                );
                let mut pairs = Vec::new();
                for id in new_loans {
                    let Origin::Borrowed { source, .. } = own.lexicon.get(id).origin else {
                        continue;
                    };
                    let donor_word = self.varieties[donor].lexicon.get(source);
                    for category in Category::ALL {
                        if !own
                            .lexicon
                            .get(id)
                            .paradigms
                            .iter()
                            .any(|p| p.category == category)
                        {
                            continue;
                        }
                        let mut rng = self.at(recipient).rng(&[
                            key("grammar imported pair"),
                            donor as u64,
                            key(category.id()),
                            id.0 as u64,
                        ]);
                        if rng.r#gen::<f32>() >= 0.04 * intensity {
                            continue;
                        }
                        let Some(paradigm) =
                            donor_word.paradigms.iter().find(|p| p.category == category)
                        else {
                            continue;
                        };
                        let Some(realization) = paradigm
                            .realizations
                            .iter()
                            .filter(|r| {
                                r.retired.is_none()
                                    && self.varieties[donor].grammar.marker(r.marker).kind
                                        != MarkerKind::None
                            })
                            .max_by(|a, b| a.share.total_cmp(&b.share))
                        else {
                            continue;
                        };
                        let marker = self.varieties[donor].grammar.marker(realization.marker);
                        let (form, edge, marker_form, source_form) = match marker.kind {
                            MarkerKind::Bound => {
                                let Some(source_form) = realization.form.as_ref() else {
                                    continue;
                                };
                                let form = adapter.adapt(
                                    source_form,
                                    self.params.bilingual_keep * intensity,
                                    &mut rng,
                                );
                                let mut input = 0;
                                let mut output = 0;
                                let mut edge = 0;
                                for (left, right) in crate::compare::align(source_form, &form) {
                                    if input >= realization.edge {
                                        edge = output;
                                        break;
                                    }
                                    input += usize::from(left.is_some());
                                    output += usize::from(right.is_some());
                                    edge = output;
                                }
                                let edge = edge.min(form.segs.len());
                                let edge_form = Form {
                                    segs: match marker.side {
                                        Side::Prefix => form.segs[..edge].to_vec(),
                                        Side::Suffix => form.segs[edge..].to_vec(),
                                    },
                                    boundaries: Vec::new(),
                                    stress: None,
                                };
                                (Some(form), edge, edge_form, source_form.clone())
                            }
                            MarkerKind::Particle => (
                                None,
                                0,
                                adapter.adapt(
                                    &marker.form,
                                    self.params.bilingual_keep * intensity,
                                    &mut rng,
                                ),
                                marker.form.clone(),
                            ),
                            MarkerKind::None => unreachable!(),
                        };
                        pairs.push((
                            id,
                            ImportedPair {
                                category,
                                source_marker: marker.id,
                                kind: marker.kind,
                                side: marker.side,
                                marker_form,
                                source_marker_form: marker.form.clone(),
                                form,
                                edge,
                                source_form,
                            },
                        ));
                    }
                }
                let own = &mut self.varieties[recipient];
                let stress = own.stress();
                for (id, pair) in pairs {
                    own.grammar
                        .import_pair(&mut own.lexicon, id, donor, pair, stress, generation);
                }
            }
            if !sustained {
                continue;
            }
            let own = &self.varieties[recipient];
            let candidates: Vec<_> = own
                .grammar
                .markers
                .iter()
                .filter(|m| {
                    !m.productive
                        && m.kind == MarkerKind::Bound
                        && m.retired.is_none()
                        && matches!(m.origin, MarkerOrigin::Imported { from, .. } if from == donor)
                })
                .filter(|m| {
                    own.lexicon
                        .living()
                        .filter(|l| {
                            l.paradigms.iter().any(|p| {
                                p.realizations
                                    .iter()
                                    .any(|r| r.retired.is_none() && r.marker == m.id)
                            })
                        })
                        .count()
                        >= 3
                })
                .map(|m| m.id)
                .collect();
            for marker in candidates {
                let mut rng = self.at(recipient).rng(&[
                    key("grammar marker transfer"),
                    donor as u64,
                    marker as u64,
                ]);
                if rng.r#gen::<f32>() < 0.0002 * intensity {
                    let own = &mut self.varieties[recipient];
                    let stress = own.stress();
                    own.grammar.transfer_marker(
                        marker,
                        donor,
                        &mut own.lexicon,
                        &own.morphology,
                        stress,
                        generation,
                    );
                }
            }
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

    fn water_pair() -> (World, usize, usize) {
        let mut world = World::new(7, Params::static_society());
        let coast: Vec<_> = (0..world.map.regions.len())
            .filter(|&r| world.map.coastal(r))
            .collect();
        let (from, to) = coast
            .iter()
            .find_map(|&a| {
                coast.iter().find_map(|&b| {
                    (world.map.overseas(a, b)
                        && world.map.voyage(a, b) <= 1800.0
                        && world.map.regions[a]
                            .neighbours
                            .iter()
                            .any(|&r| world.map.regions[r].terrain.is_land()))
                    .then_some((a, b))
                })
            })
            .expect("nearby separate shores");
        let a = world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            7,
            0.8,
            0.5,
            Some(from),
            Some(Livelihood::Farming),
            None,
        );
        let b = world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            8,
            0.2,
            0.5,
            Some(to),
            Some(Livelihood::Farming),
            None,
        );
        (world, a, b)
    }

    #[test]
    fn journeys_require_own_ships_and_respect_inclusive_reach() {
        let (mut world, a, b) = water_pair();
        let to = world.communities[b].home();
        assert!(world.journey_to(a, to).is_none());
        assert!(world.apart(a, b).is_infinite());
        world.learn(b, Craft::Seafaring, None);
        assert!(world.journey_between(a, b).is_none());
        let voyage = world.journey_between(b, a).unwrap();
        assert!(voyage.by_sea);
        assert_eq!(world.apart(a, b), voyage.effort);
        let from = world.communities[a].home();
        assert!(
            world
                .journeys(b, voyage.effort)
                .iter()
                .any(|(r, j)| *r == from && j.by_sea)
        );
        assert!(
            !world
                .journeys(b, voyage.effort - 0.01)
                .iter()
                .any(|(r, _)| *r == from)
        );
        world.learn(a, Craft::Seafaring, None);
        let reverse = world.journey_to(a, to).unwrap();
        assert!(reverse.by_sea);
        assert!((reverse.effort - voyage.effort).abs() < 0.001);
        assert!(!world.journey_to(a, from).unwrap().by_sea);
    }

    #[test]
    fn authored_contacts_refuse_unreachable_pairs_without_mutation() {
        let (mut world, a, b) = water_pair();
        let before = world.clone();
        for kind in [
            ContactKind::Neighbours,
            ContactKind::Trade,
            ContactKind::Rule,
            ContactKind::Intermarriage,
            ContactKind::Religion,
        ] {
            assert!(world.connect(a, b, 0.5, kind).is_err());
            assert_eq!(world.events, before.events);
            assert_eq!(world.contacts, before.contacts);
            assert_eq!(world.communities, before.communities);
            assert_eq!(world.states, before.states);
        }
        world.learn(b, Craft::Seafaring, None);
        let effort = world.apart(a, b);
        world.params.trade_reach = effort;
        world.connect(a, b, 0.5, ContactKind::Trade).unwrap();
        let events = world.events.clone();
        assert!(world.connect(a, b, 0.8, ContactKind::Rule).is_err());
        assert_eq!(world.events, events);
        assert!(world.states.is_empty());
        assert_eq!(world.contacts[0].kind, ContactKind::Trade);
        world.params.pilgrimage_reach = effort;
        world.connect(a, b, 0.5, ContactKind::Religion).unwrap();
        world.params.conquest_reach = effort;
        world
            .connect(a, b, 0.5, ContactKind::Intermarriage)
            .unwrap();
        world.params.trade_reach = effort - 0.01;
        assert!(world.connect(a, b, 0.5, ContactKind::Trade).is_err());
        assert!(world.connect(a, b, 0.5, ContactKind::Neighbours).is_err());
        world.learn(a, Craft::Seafaring, None);
        world.connect(a, b, 0.8, ContactKind::Rule).unwrap();
        assert!(world.rules_over(a, b));
    }

    #[test]
    fn movement_ends_actual_rulers_unreachable_rule_once_without_turnover() {
        let (mut world, ruler, subject) = water_pair();
        let remote = world.communities[subject].home();
        world.communities[subject].lands = world.communities[ruler].lands.clone();
        world
            .connect(ruler, subject, 0.8, ContactKind::Rule)
            .unwrap();
        let state = world.rules(ruler).unwrap();
        world.communities[subject].lands = vec![remote];
        world.learn(subject, Craft::Seafaring, None);
        world.communities[subject].prestige = 1.0;
        world.communities[ruler].prestige = 0.1;
        world.params.conquest_reach = 1800.0;
        assert!(world.can_rule(subject, ruler));
        assert!(!world.can_rule(ruler, subject));
        world.reconcile_contacts();
        assert_eq!(world.ruled_by(subject), None);
        assert!(world.states[state].subjects().next().is_none());
        assert!(world.contacts.is_empty());
        world.reconcile_contacts();
        let parted: Vec<_> = world
            .events
            .iter()
            .filter(|(_, e)| {
                matches!(e,
            WorldEvent::Parted { a, b, kind: ContactKind::Rule } if *a == ruler && *b == subject)
            })
            .collect();
        assert_eq!(parted.len(), 1);
        assert!(world.states[state].standing());
    }

    #[test]
    fn pilgrimage_reaches_the_site_not_the_holders_other_lands() {
        let (mut world, pilgrim, holder) = water_pair();
        let shrine = world.communities[holder].home();
        let home = world.communities[pilgrim].home();
        let faith = world.found_religion(pilgrim, crate::ideas::Revelation::Proclaimed);
        world.religions[faith].shrine.region = shrine;
        world.communities[holder].lands.push(home);
        world.params.pilgrimage_rate = 1.0;
        world.learn(holder, Craft::Seafaring, None);
        assert!(world.journey_between(pilgrim, holder).is_some());
        world.send_pilgrims();
        assert!(world.contacts.is_empty());
        world.learn(pilgrim, Craft::Seafaring, None);
        let effort = world.map.voyage(home, shrine);
        world.params.pilgrimage_reach = effort - 0.01;
        world.send_pilgrims();
        assert!(world.contacts.is_empty());
        world.params.pilgrimage_reach = effort;
        world.send_pilgrims();
        assert_eq!(world.contacts[0].kind, ContactKind::Religion);
        assert_eq!(world.contacts[0].intensity, 0.3);
        world
            .connect(pilgrim, holder, 0.7, ContactKind::Trade)
            .unwrap();
        let contacts = world.contacts.clone();
        let events = world.events.clone();
        world.send_pilgrims();
        assert_eq!(world.contacts, contacts);
        assert_eq!(world.events, events);
    }

    fn crowd_walkable_lands(world: &mut World, community: usize) {
        let home = world.communities[community].home();
        let blocker = world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            9,
            1.0,
            0.5,
            Some(home),
            Some(Livelihood::Farming),
            None,
        );
        world.communities[blocker].lands = world
            .map
            .walking_row(home, f32::INFINITY)
            .iter()
            .map(|&(r, _)| r as usize)
            .collect();
        world.communities[blocker].size = 100_000_000.0;
    }

    #[test]
    fn maritime_moves_record_mode_and_never_borrow_another_peoples_ships() {
        let (mut world, a, b) = water_pair();
        world.params.migration_rate = 1000.0;
        world.params.migration_reach = 1800.0;
        world.params.colony_reach = 1800.0;
        world.params.trade_reach = 1800.0;
        let home = world.communities[a].home();
        crowd_walkable_lands(&mut world, a);
        world.learn(b, Craft::Seafaring, None);
        assert_eq!(
            world.leavers_land(home, Livelihood::Farming, false, 1.0),
            (home, false)
        );
        world.migrate();
        assert_eq!(world.communities[a].home(), home);
        world.learn(a, Craft::Seafaring, None);
        let daughter = world.split(a, None, 0.5);
        let coast = world.communities[daughter].home();
        assert!(world.map.voyage(home, coast) <= world.params.colony_reach);
        assert!(world.events.iter().any(|(_, e)| matches!(e,
            WorldEvent::Split { daughter: d, by_sea: true, .. } if *d == daughter)));
        assert!(
            world
                .contacts
                .iter()
                .any(|k| k.a == a && k.b == daughter && k.kind == ContactKind::Trade)
        );
        world.migrate();
        assert!(world.events.iter().any(|(_, e)| matches!(e,
            WorldEvent::Migrated { community, by_sea: true, .. } if *community == a)));
        for (_, event) in &world.events {
            if let WorldEvent::Migrated {
                from, to, by_sea, ..
            } = *event
            {
                let effort = if by_sea {
                    world.map.voyage(from, to)
                } else {
                    world.map.distance(from, to)
                };
                assert!(effort <= world.params.migration_reach);
            }
        }
    }

    #[test]
    fn urban_residence_does_not_grant_a_neighbouring_holding_or_port() {
        let (mut world, ruler, subject) = water_pair();
        world.learn(ruler, Craft::Seafaring, None);
        world.params.conquest_reach = 1800.0;
        world.params.city_rate = 1.0;
        for c in [ruler, subject] {
            world.communities[c].size = 300_000.0;
        }
        let remote = world.communities[subject].home();
        let capital = world.communities[ruler].home();
        world.communities[subject].lands = vec![capital];
        world
            .connect(ruler, subject, 0.6, ContactKind::Rule)
            .unwrap();
        world.grow_cities();
        world.communities[subject].lands = vec![remote];
        assert!(
            world
                .presence(subject)
                .iter()
                .any(|&(r, n)| r == capital && n > 0.0)
        );
        let counted: f32 = world.presence(subject).iter().map(|(_, n)| n).sum();
        assert!((counted - world.communities[subject].size).abs() < 0.1);
        let rule = world.contacts[0];
        world.part_contact(rule);
        let spatial = world.spatial();
        assert_eq!(world.nearness_indexed(ruler, subject, &spatial), 0.0);
        assert!(world.journey_to(subject, capital).is_none());
        world.params.neighbour_rate = 1000.0;
        world.make_contacts();
        assert!(world.contacts.is_empty());
    }

    #[test]
    fn regional_food_and_presence_follow_polygon_area() {
        let mut world = World::new(7, Params::static_society());
        let lands: Vec<_> = world
            .map
            .regions
            .iter()
            .enumerate()
            .filter(|(id, r)| {
                r.terrain == Terrain::Plains && world.map.river_regions[*id].is_none()
            })
            .map(|(r, _)| r)
            .take(2)
            .collect();
        assert_eq!(lands.len(), 2);
        let a = world.found(&SoundProfile::base(), 0.5, 0.5);
        world.communities[a].lands = lands.clone();
        let fed_a = world.feeds(lands[0], Livelihood::Farming);
        let fed_b = world.feeds(lands[1], Livelihood::Farming);
        let area_ratio =
            world.map.regions[lands[0]].area_km2 / world.map.regions[lands[1]].area_km2;
        assert!((fed_a / fed_b - area_ratio).abs() < 1e-5);
        let presence = world.presence(a);
        assert!(
            (presence.iter().map(|(_, n)| n).sum::<f32>() - world.communities[a].size).abs() < 0.01
        );
        assert!((presence[0].1 / presence[1].1 - area_ratio).abs() < 1e-5);
        for (r, region) in world.map.regions.iter().enumerate() {
            if region.terrain == Terrain::Sea {
                assert_eq!(world.feeds(r, Livelihood::Farming), 0.0);
            }
        }
    }
    #[test]
    fn sequential_spread_redistributes_all_old_land_presence() {
        let mut world = World::new(7, Params::static_society());
        world.params.spread_rate = 1000.0;
        let (home, destination) = world
            .map
            .regions
            .iter()
            .enumerate()
            .find_map(|(r, region)| {
                if region.terrain != Terrain::Plains
                    || region
                        .neighbours
                        .iter()
                        .filter(|&&n| world.map.regions[n].terrain.is_land())
                        .count()
                        < 2
                {
                    return None;
                }
                region
                    .neighbours
                    .iter()
                    .copied()
                    .find(|&n| {
                        world.map.regions[n].terrain == Terrain::Plains
                            && world.feeds(n, Livelihood::Farming)
                                > world.feeds(r, Livelihood::Farming) / 2.0
                    })
                    .map(|n| (r, n))
            })
            .unwrap();
        for (seed, share) in [(1, 0.3), (2, 0.3), (3, 0.1)] {
            let c = world.found_seeded(
                &Naming::People,
                &SoundProfile::base(),
                seed,
                0.5,
                0.5,
                Some(home),
                Some(Livelihood::Farming),
                None,
            );
            world.communities[c].size = world.feeds(home, Livelihood::Farming) * share;
        }
        let blocked: Vec<_> = world.map.regions[home]
            .neighbours
            .iter()
            .copied()
            .filter(|&r| r != destination && world.map.regions[r].terrain.is_land())
            .collect();
        let blocker = world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            4,
            1.0,
            0.5,
            Some(blocked[0]),
            Some(Livelihood::Farming),
            None,
        );
        world.communities[blocker].lands = blocked;
        world.communities[blocker].size = 100_000_000.0;
        let population: f32 = world.living().map(|c| world.communities[c].size).sum();
        world.spread();
        assert!(world.communities[0].lands.contains(&destination));
        assert_eq!(world.communities[1].lands, vec![home]);
        assert_eq!(world.communities[2].lands, vec![home]);
        let after: f32 = world.living().map(|c| world.communities[c].size).sum();
        assert_eq!(population, after);
        for c in world.living() {
            assert!(
                (world.presence(c).iter().map(|(_, n)| n).sum::<f32>() - world.communities[c].size)
                    .abs()
                    < world.communities[c].size * 1e-6
            );
        }
    }

    #[test]
    fn later_migrants_see_the_room_consumed_by_earlier_migrants() {
        let (mut world, first, second) = water_pair();
        let home = world.communities[first].home();
        world.communities[second].lands = vec![home];
        let destination = world
            .map
            .walking_row(home, 600.0)
            .iter()
            .find(|&&(r, _)| {
                r as usize != home && world.map.regions[r as usize].terrain == Terrain::Plains
            })
            .unwrap()
            .0 as usize;
        let capacity = world.feeds(destination, Livelihood::Farming);
        for c in [first, second] {
            world.communities[c].size = capacity * 0.9;
        }
        crowd_walkable_lands(&mut world, first);
        world
            .communities
            .last_mut()
            .unwrap()
            .lands
            .retain(|&r| r != destination);
        world.params.migration_rate = 1000.0;
        world.params.migration_reach = 600.0;
        let population = world.communities[first].size + world.communities[second].size;
        world.migrate();
        assert_eq!(world.communities[first].lands, vec![destination]);
        assert_eq!(world.communities[second].lands, vec![home]);
        assert_eq!(
            world.communities[first].size + world.communities[second].size,
            population
        );
    }

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

    #[test]
    fn a_sound_law_recurs_in_a_lineage_only_after_its_quiet_span() {
        let params = Params {
            sound_change_rate: 1.0,
            preference_pull: 0.0,
            areal_pull: 0.0,
            ..Params::static_society()
        };
        let mut world = World::solo(0, &SoundProfile::base(), params);
        let mut law = catalog()
            .into_iter()
            .find(|law| law.id == "w-fortition")
            .unwrap();
        law.commonness = 1_000_000.0;
        world.laws = vec![law];
        let word = world.varieties[0].lexicon.living().next().unwrap().id;
        let before = Form::from_ipa("wawa").unwrap();
        let after = Form::from_ipa("vava").unwrap();
        world.varieties[0].lexicon.get_mut(word).form = before.clone();
        world.sound_change(0, &[]);
        assert_eq!(world.varieties[0].lexicon.get(word).form, after);

        let daughter = world.split(0, None, 0.0);
        let v = world.communities[daughter].variety;
        // Later words can reintroduce w, but the inherited law is still recent.
        world.varieties[v].lexicon.get_mut(word).form = before.clone();
        for generation in 1..LAW_RECURRENCE {
            world.generation = generation;
            world.sound_change(v, &[]);
            assert_eq!(world.varieties[v].lexicon.get(word).form, before);
            assert_eq!(world.varieties[v].laws, vec![(0, "w-fortition")]);
        }
        // A fresh wave obeys the same quiet span as a local change.
        let mut wave = world.clone();
        wave.params.wave_rate = 1_000_000.0;
        let source = wave.found(&SoundProfile::base(), 0.9, 0.5);
        let from = wave.communities[source].variety;
        wave.communities[source].lands = wave.communities[daughter].lands.clone();
        wave.connect(daughter, source, 1.0, ContactKind::Neighbours)
            .unwrap();
        let law = wave.laws[0].clone();
        wave.apply_law(from, &law);
        wave.spread_waves(&wave.spoken());
        assert_eq!(wave.varieties[v].lexicon.get(word).form, before);
        assert_eq!(wave.varieties[v].laws, vec![(0, "w-fortition")]);
        wave.generation = LAW_RECURRENCE;
        wave.spread_waves(&wave.spoken());
        assert_eq!(wave.varieties[v].lexicon.get(word).form, after);
        assert_eq!(
            wave.varieties[v].laws,
            vec![(0, "w-fortition"), (LAW_RECURRENCE, "w-fortition")]
        );
        assert_eq!(
            wave.varieties[v].waves,
            vec![(LAW_RECURRENCE, "w-fortition", from)]
        );

        world.generation = LAW_RECURRENCE;
        world.sound_change(v, &[]);
        assert_eq!(world.varieties[v].lexicon.get(word).form, after);
        assert_eq!(
            world.varieties[v].laws,
            vec![(0, "w-fortition"), (LAW_RECURRENCE, "w-fortition")]
        );
    }

    /// A prestigious donor and an open recipient in contact for 40
    /// generations.
    fn pair(seed: u64, kind: ContactKind) -> World {
        let mut world = World::new(seed, Params::static_society());
        let d = world.found(&SoundProfile::by_id("iranian").unwrap(), 0.8, 0.4);
        let r = world.found(&SoundProfile::by_id("polynesian").unwrap(), 0.3, 0.7);
        world.communities[r].lands = world.communities[d].lands.clone();
        world.connect(d, r, 0.8, kind).unwrap();
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
            let Origin::Borrowed { from, source, .. } = loan.origin else {
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
                Origin::Borrowed { from, source, .. } => Some((from, source)),
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
        use crate::compare::{Settings, compare_varieties, regular_correspondences};
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
            world.communities[outsiders].lands = world.communities[east].lands.clone();
            world
                .connect(outsiders, east, 0.8, ContactKind::Rule)
                .unwrap();
            world.run(40);
            let (a, b) = (
                world.communities[west].variety,
                world.communities[east].variety,
            );
            let out = world.communities[outsiders].variety;
            let result = compare_varieties(&world.varieties[a], &world.varieties[b], &concepts);
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
                world.communities[b].lands = world.communities[a].lands.clone();
                world.connect(a, b, 1.0, ContactKind::Neighbours).unwrap();
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
        let home = world.communities[rulers].home();
        let nearby = world.map.regions[home]
            .neighbours
            .iter()
            .copied()
            .find(|&r| world.map.regions[r].terrain.is_land())
            .unwrap();
        world.communities[subjects].lands = vec![nearby];
        world
            .connect(rulers, subjects, 1.0, ContactKind::Rule)
            .unwrap();
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
        world.communities[b].lands = world.communities[a].lands.clone();
        world.connect(a, b, 0.4, ContactKind::Trade).unwrap();
        world.connect(b, a, 0.7, ContactKind::Religion).unwrap();
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
                // Keep vocabulary tiers fixed, rather than drawing a different
                // livelihood from each seed's river and climate suitability.
                let mut world = World::new(seed, Params::static_society());
                let variety_seed = stream(seed, &[key("found"), 0]).next_u64();
                world.found_seeded(
                    &Naming::People,
                    &profile,
                    variety_seed,
                    0.5,
                    0.5,
                    None,
                    Some(Livelihood::Foraging),
                    None,
                );
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
            world.communities[b].lands = world.communities[a].lands.clone();
            world.communities[c].lands = world.communities[a].lands.clone();
            world.connect(a, b, 0.5, ContactKind::Trade).unwrap();
            world.connect(a, c, 0.5, ContactKind::Rule).unwrap();
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
        }
        assert!(ended >= 15, "trade ended in {ended} of 20");
        assert!(freed >= 8, "rule ended in {freed} of 20");
        assert!(met >= 5, "the world made contacts in {met} of 20");

        let mut world = World::new(1, Params::static_society());
        let a = world.found(&SoundProfile::by_id("germanic").unwrap(), 0.5, 0.5);
        let b = world.found(&SoundProfile::by_id("finnic").unwrap(), 0.5, 0.5);
        world.communities[b].lands = world.communities[a].lands.clone();
        world.connect(a, b, 0.5, ContactKind::Trade).unwrap();
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
            world.communities[b].lands = world.communities[a].lands.clone();
            world.connect(a, b, 0.6, ContactKind::Rule).unwrap();
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
            let mut world = World::new(
                seed,
                Params {
                    growth_rate: Params::default().growth_rate,
                    spread_rate: Params::default().spread_rate,
                    ..Params::static_society()
                },
            );
            let design = SoundProfile::base();
            world.found_seeded(
                &Naming::People,
                &design,
                seed,
                0.5,
                0.5,
                None,
                Some(Livelihood::Farming),
                None,
            );
            world.run(300);
            let mut spread = false;
            for c in world.living() {
                let k = &world.communities[c];
                // Rulers and townsfolk can also eat stored food and tribute.
                let fed: f32 = k
                    .lands
                    .iter()
                    .map(|&r| world.feeds(r, k.livelihood))
                    .sum::<f32>()
                    + world.tribute(c);
                assert!(
                    k.size < fed * 1.3,
                    "seed {seed}, community {c}: {} on lands feeding {fed}; tribute {}, urban {:?}",
                    k.size,
                    world.tribute(c),
                    world.city_residence(c)
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
        let home = world
            .map
            .landmasses
            .iter()
            .find(|m| m.kind == LandmassKind::Continent)
            .unwrap()
            .anchor;
        let c = world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            4,
            0.5,
            0.5,
            Some(home),
            Some(Livelihood::Farming),
            None,
        );
        let heart = world.communities[c].home();
        // Its heart, a land beside it, and the land furthest from it.
        let beside = world.map.regions[heart]
            .neighbours
            .iter()
            .copied()
            .find(|&r| world.map.regions[r].terrain.is_land())
            .unwrap();
        let row = world.map.walking_row(heart, f32::INFINITY);
        let far = row
            .iter()
            .filter(|&&(r, _)| r as usize != heart)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0 as usize;
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
            world.communities[comers].lands = world.communities[natives].lands.clone();
            world
                .connect(natives, comers, 0.5, ContactKind::Trade)
                .unwrap();
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
                if let WorldEvent::Migrated {
                    from, to, by_sea, ..
                } = *event
                {
                    moved += 1;
                    assert!(from != to && world.map.regions[to].terrain.is_land());
                    let effort = if by_sea {
                        world.map.voyage(from, to)
                    } else {
                        world.map.distance(from, to)
                    };
                    assert!(effort <= world.params.migration_reach);
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
                        None,
                    )
                };
                let foragers = found(&mut world, Livelihood::Foraging);
                let other = found(&mut world, teacher);
                world
                    .connect(foragers, other, 0.8, ContactKind::Neighbours)
                    .unwrap();
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
        world.communities[b].lands = world.communities[a].lands.clone();
        world.connect(a, b, 0.5, ContactKind::Trade).unwrap();
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
            let land = world.communities[home].home();
            world.communities[stranger].lands = vec![land];
            world.communities[dialect].lands = vec![land];
            world
                .connect(home, stranger, 0.5, ContactKind::Neighbours)
                .unwrap();
            world.run(60);
            let source = world.communities[home].variety;
            for (who, count) in [(dialect, &mut kin), (stranger, &mut strangers)] {
                let v = world.variety_of(who);
                for &(g, id, from) in &v.waves {
                    assert!(v.laws.contains(&(g, id)), "a wave is a law undergone");
                    *count += usize::from(from == source);
                }
            }
        }
        assert!(
            kin >= 10 && kin >= 2 * strangers,
            "{kin} changes reached the dialect, {strangers} the strangers"
        );
    }

    fn cultural_world(seed: u64) -> World {
        World::with_map(seed, Params::static_society(), MapSize::Large)
    }

    fn cultural_found(world: &mut World, seed: u64, region: usize) -> usize {
        world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            seed,
            0.5,
            0.5,
            Some(region),
            Some(Livelihood::Farming),
            None,
        )
    }

    fn cultural_memory(world: &mut World, variety: usize, region: usize, meaning: &str) {
        let speech = &world.varieties[variety];
        let mut name = Naming::Place {
            place: "river".into(),
        }
        .coin(speech, None, world.generation)
        .unwrap();
        name.meaning = meaning.into();
        world.varieties[variety].exonyms.push((region, name));
    }

    #[test]
    fn cultural_owned_and_surveyed_names_survive_departure_and_evolve() {
        let mut evolved = 0;
        for seed in 0..8 {
            let mut world = cultural_world(seed);
            let home = world
                .map
                .landmasses
                .iter()
                .find(|m| m.kind == LandmassKind::Continent)
                .unwrap()
                .anchor;
            let c = cultural_found(&mut world, seed, home);
            let v = world.communities[c].variety;
            let surveyed = world.map.regions[home]
                .neighbours
                .iter()
                .copied()
                .find(|&r| world.map.regions[r].terrain.is_land())
                .unwrap();
            assert!(world.places[surveyed].is_empty());
            let survey = world.known_place(v, surveyed).unwrap().clone();
            let local = world.known_place(v, home).unwrap().clone();
            let elsewhere = (0..world.map.regions.len())
                .find(|&r| {
                    world.map.regions[r].terrain.is_land() && !world.known_lands(v).contains(&r)
                })
                .unwrap();
            world.communities[c].lands = vec![elsewhere];
            world.refresh_places();
            assert_eq!(world.known_place(v, home), Some(&local));
            assert_eq!(world.known_place(v, surveyed), Some(&survey));
            world.run(100);
            assert_eq!(
                world.places[home][0].name, local,
                "the abandoned local record freezes"
            );
            let remembered = world.known_place(v, home).unwrap();
            assert_eq!(remembered.meaning, local.meaning);
            for entry in &remembered.log {
                if entry.generation > local.log.last().map_or(0, |e| e.generation) {
                    let Event::SoundLaw { law, .. } = entry.event else {
                        panic!("only sound laws");
                    };
                    assert!(
                        world.varieties[v]
                            .laws
                            .iter()
                            .any(|&(g, l)| g == entry.generation && l == law)
                    );
                }
            }
            evolved += usize::from(remembered.form != local.form);
            let lands = world.known_lands(v);
            assert!(lands.windows(2).all(|pair| pair[0] < pair[1]));
            assert_eq!(
                world.varieties[v]
                    .exonyms
                    .iter()
                    .filter(|(r, _)| *r == home)
                    .count(),
                1
            );
            assert!(world.places[surveyed].is_empty());
        }
        assert!(
            evolved >= 4,
            "{evolved} of 8 abandoned names evolved in memory"
        );
    }

    #[test]
    fn cultural_contacts_teach_partner_lands_without_ownership_or_contact_retention() {
        let mut world = cultural_world(9);
        let mainland = world
            .map
            .landmasses
            .iter()
            .find(|m| m.kind == LandmassKind::Continent)
            .unwrap();
        let home = mainland.anchor;
        let remote = world
            .map
            .walking_row(home, world.params.trade_reach)
            .iter()
            .filter(|&&(r, _)| world.map.closeness(home, r as usize) == 0.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap()
            .0 as usize;
        let a = cultural_found(&mut world, 9, home);
        let b = cultural_found(&mut world, 10, remote);
        let (av, bv) = (world.communities[a].variety, world.communities[b].variety);
        let (ah, bh) = (world.communities[a].home(), world.communities[b].home());
        assert!(world.known_place(av, bh).is_none());
        assert!(world.known_place(bv, ah).is_none());
        world.connect(a, b, 0.5, ContactKind::Trade).unwrap();
        world.refresh_places();
        let remembered = world.known_place(av, bh).unwrap().clone();
        assert!(world.known_place(bv, ah).is_some());
        assert_eq!(world.communities[a].lands, vec![ah]);
        world.contacts.clear();
        world.refresh_places();
        assert_eq!(world.known_place(av, bh), Some(&remembered));
    }

    #[test]
    fn cultural_reports_are_batched_one_per_direction_and_first_hearing_wins() {
        let mut world = cultural_world(11);
        let home = world.map.landmasses[0].anchor;
        let a = cultural_found(&mut world, 11, home);
        let b = cultural_found(&mut world, 12, home);
        let c = cultural_found(&mut world, 13, home);
        let (av, bv, cv) = (
            world.communities[a].variety,
            world.communities[b].variety,
            world.communities[c].variety,
        );
        let remote: Vec<_> = (0..world.map.regions.len())
            .filter(|&r| {
                world.map.regions[r].terrain.is_land()
                    && [av, bv, cv]
                        .iter()
                        .all(|&v| world.known_place(v, r).is_none())
            })
            .take(6)
            .collect();
        assert_eq!(remote.len(), 6);
        for &r in &remote[..3] {
            cultural_memory(&mut world, av, r, "A's remembered shore");
        }
        for &r in &remote[3..] {
            cultural_memory(&mut world, bv, r, "B's remembered shore");
        }
        // The chart's present local record is not what these speakers remember.
        let outsider = world.varieties.len();
        world.varieties.push(Variety::found(
            17,
            &SoundProfile::base(),
            Livelihood::Farming,
            Default::default(),
        ));
        let mut current = Naming::People
            .coin(&world.varieties[outsider], None, 0)
            .unwrap();
        current.meaning = "the present holder's unrelated name".into();
        for &r in &remote {
            world.places[r].push(PlaceName {
                variety: outsider,
                since: 0,
                name: current.clone(),
                origin: PlaceOrigin::Borrowed,
            });
        }
        world.connect(b, c, 0.5, ContactKind::Trade).unwrap();
        world.connect(a, b, 0.5, ContactKind::Trade).unwrap();
        for _ in 0..3 {
            world.refresh_places();
        }
        assert!(
            remote.iter().all(|&r| world.known_place(cv, r).is_none()),
            "local refresh spends no report"
        );
        world.hear_places();
        assert_eq!(
            remote[..3]
                .iter()
                .filter(|&&r| world.known_place(bv, r).is_some())
                .count(),
            1
        );
        assert_eq!(
            remote[3..]
                .iter()
                .filter(|&&r| world.known_place(av, r).is_some())
                .count(),
            1
        );
        assert_eq!(
            remote[3..]
                .iter()
                .filter(|&&r| world.known_place(cv, r).is_some())
                .count(),
            1
        );
        assert!(
            remote[..3]
                .iter()
                .all(|&r| world.known_place(cv, r).is_none()),
            "no same-phase cascade"
        );
        let heard = remote[..3]
            .iter()
            .copied()
            .find(|&r| world.known_place(bv, r).is_some())
            .unwrap();
        let first = world.known_place(bv, heard).unwrap().clone();
        assert_eq!(
            first.meaning, "A's remembered shore",
            "reports use the reporter's memory"
        );
        world.varieties[av]
            .exonyms
            .iter_mut()
            .find(|(r, _)| *r == heard)
            .unwrap()
            .1
            .meaning = "later political name".into();
        world.generation += 1;
        world.hear_places();
        assert_eq!(world.known_place(bv, heard), Some(&first));
        assert_eq!(
            world.varieties[bv]
                .exonyms
                .iter()
                .filter(|(r, _)| *r == heard)
                .count(),
            1
        );
    }

    #[test]
    fn cultural_forks_snapshot_own_names_without_later_parental_discoveries() {
        let mut world = cultural_world(12);
        let home = world.map.landmasses[0].anchor;
        let c = cultural_found(&mut world, 12, home);
        let parent = world.communities[c].variety;
        let local = world.known_place(parent, home).unwrap().clone();
        let d = world.split(c, None, 0.0);
        let daughter = world.communities[d].variety;
        assert_eq!(world.known_place(daughter, home), Some(&local));
        let remote = (0..world.map.regions.len())
            .find(|&r| {
                world.map.regions[r].terrain.is_land()
                    && world.known_place(parent, r).is_none()
                    && world.known_place(daughter, r).is_none()
            })
            .unwrap();
        cultural_memory(&mut world, parent, remote, "a discovery after separation");
        assert!(world.known_place(daughter, remote).is_none());
        assert!(world.known_place(parent, remote).is_some());
    }

    #[test]
    fn cultural_shift_unites_memories_and_target_memories_win_conflicts() {
        let mut world = cultural_world(14);
        let home = world.map.landmasses[0].anchor;
        let a = cultural_found(&mut world, 14, home);
        let b = cultural_found(&mut world, 15, home);
        let (old, target) = (world.communities[a].variety, world.communities[b].variety);
        let remote: Vec<_> = (0..world.map.regions.len())
            .filter(|&r| {
                world.map.regions[r].terrain.is_land()
                    && world.known_place(old, r).is_none()
                    && world.known_place(target, r).is_none()
            })
            .take(3)
            .collect();
        cultural_memory(&mut world, old, remote[0], "old speakers' shore");
        cultural_memory(&mut world, target, remote[1], "target speakers' shore");
        cultural_memory(&mut world, old, remote[2], "old conflict");
        cultural_memory(&mut world, target, remote[2], "target conflict");
        let old_known = world.known_lands(old);
        let target_known = world.known_lands(target);
        let shifted = world.shift(a, b);
        for r in old_known.into_iter().chain(target_known) {
            assert!(world.known_place(shifted, r).is_some());
        }
        assert_eq!(
            world.known_place(shifted, remote[0]).unwrap().meaning,
            "old speakers' shore"
        );
        assert_eq!(
            world.known_place(shifted, remote[2]).unwrap().meaning,
            "target conflict"
        );
        let known = world.known_lands(shifted);
        assert_eq!(world.varieties[shifted].exonyms.len(), known.len());
    }

    #[test]
    fn cultural_continent_attestations_have_true_witnesses_and_stay_fixed() {
        let mut world = cultural_world(15);
        assert!(world.continent_names.iter().all(Option::is_none));
        let before = world.clone();
        let landmass = world
            .map
            .landmasses
            .iter()
            .position(|m| m.kind == LandmassKind::Continent)
            .unwrap();
        let home = world.map.landmasses[landmass].anchor;
        let c = cultural_found(&mut world, 15, home);
        let name = world.continent_names[landmass].clone().unwrap();
        assert_eq!(
            (name.people, name.variety, name.witness, name.since),
            (c, world.communities[c].variety, home, 0)
        );
        assert!(world.known_place(name.variety, name.witness).is_some());
        assert_eq!(world.map.regions[name.witness].landmass, Some(landmass));
        assert!(
            before.continent_names.iter().all(Option::is_none),
            "historical snapshot excludes later attestations"
        );
        world.run(80);
        let conqueror = cultural_found(&mut world, 16, home);
        world.communities[conqueror].size = world.communities[c].size * 3.0;
        world.refresh_places();
        assert_eq!(world.continent_names[landmass], Some(name));
        for (id, mass) in world.map.landmasses.iter().enumerate() {
            if mass.kind == LandmassKind::Island {
                assert!(world.continent_names[id].is_none());
            }
        }
    }

    #[test]
    fn cultural_continent_naming_prefers_first_knower_and_earliest_foreign_witness() {
        let mut world = cultural_world(42);
        let continents: Vec<_> = world
            .map
            .landmasses
            .iter()
            .enumerate()
            .filter(|(_, m)| m.kind == LandmassKind::Continent)
            .map(|(id, m)| (id, m.anchor))
            .collect();
        let a = cultural_found(&mut world, 42, continents[0].1);
        let b = cultural_found(&mut world, 43, continents[0].1);
        let (av, bv) = (world.communities[a].variety, world.communities[b].variety);
        let target = continents[1].0;
        assert!(world.continent_names[target].is_none());
        let later = world.map.landmasses[target].regions[0];
        let earlier = world.map.landmasses[target].regions[1];
        cultural_memory(&mut world, av, later, "heard later");
        cultural_memory(&mut world, av, earlier, "heard earlier");
        world.varieties[av]
            .exonyms
            .iter_mut()
            .find(|(r, _)| *r == later)
            .unwrap()
            .1
            .coined = 15;
        world.varieties[av]
            .exonyms
            .iter_mut()
            .find(|(r, _)| *r == earlier)
            .unwrap()
            .1
            .coined = 2;
        cultural_memory(
            &mut world,
            bv,
            later,
            "another people's still earlier report",
        );
        world.generation = 20;
        world.refresh_places();
        let name = world.continent_names[target].as_ref().unwrap();
        assert_eq!(
            (name.people, name.variety, name.witness, name.since),
            (a, av, earlier, 20)
        );
    }
    #[test]
    fn stress_shifts_record_words_and_names_without_changing_segments() {
        use crate::StressRule;
        let mut profile = SoundProfile::base();
        profile.stress = Some(StressRule::Final);
        let mut world = World::solo(7, &profile, Params::static_society());
        let v = world.communities[0].variety;
        let word = world.varieties[v].lexicon.slots[0].dominant().unwrap();
        let form = Form::from_ipa("katata").unwrap();
        world.varieties[v].lexicon.lexemes[word.0 as usize].form = form.clone();
        world.varieties[v].name.form = form.clone();
        world.communities[0].name.form = form.clone();
        world.generation = 8;
        let daughter = world.split(0, None, 0.0);
        let receiver = world.communities[daughter].variety;
        world
            .connect(0, daughter, 1.0, ContactKind::Neighbours)
            .unwrap();
        let law = catalog()
            .into_iter()
            .find(|l| l.id == "initial-stress")
            .unwrap();
        world.apply_law(v, &law);
        let speech = &world.varieties[v];
        assert_eq!(speech.stress(), StressRule::Initial);
        assert_eq!(speech.stress_at(7), StressRule::Final);
        assert_eq!(speech.stress_at(8), StressRule::Initial);
        assert_eq!(speech.lexicon.get(word).form, form);
        assert_eq!(speech.name.form, form);
        assert_eq!(world.communities[0].name.form, form);
        assert_eq!(speech.laws.last(), Some(&(8, "initial-stress")));
        assert!(matches!(
            speech.lexicon.get(word).log.last().unwrap().event,
            Event::SoundLaw {
                law: "initial-stress",
                ..
            }
        ));
        assert!(matches!(
            speech.name.log.last().unwrap().event,
            Event::SoundLaw {
                law: "initial-stress",
                ..
            }
        ));
        assert!(matches!(
            world.communities[0].name.log.last().unwrap().event,
            Event::SoundLaw {
                law: "initial-stress",
                ..
            }
        ));
        assert_eq!(speech.fork(v, 8).stress_history, speech.stress_history);
        world.params.wave_rate = 1_000_000.0;
        world.spread_waves(&world.spoken());
        let received = &world.varieties[receiver];
        assert_eq!(received.stress(), StressRule::Initial);
        assert_eq!(received.lexicon.get(word).form, form);
        assert!(received.waves.contains(&(8, "initial-stress", v)));
    }

    #[test]
    fn initial_stress_suffixes_lose_plural_contrast_more_often() {
        use crate::{GrammarChoice, GrammarDesign, GrammarPrior, MinimalWord, StressRule};
        let mut losses = [0.0_f32; 3];
        for seed in 0..30 {
            for (i, (stress, choice)) in [
                (StressRule::Initial, GrammarChoice::Suffix),
                (StressRule::Final, GrammarChoice::Suffix),
                (StressRule::Initial, GrammarChoice::Prefix),
            ]
            .into_iter()
            .enumerate()
            {
                let mut profile = SoundProfile::base();
                profile.stress = Some(stress);
                profile.grammar = GrammarPrior::fixed(GrammarDesign {
                    plural: choice,
                    past: GrammarChoice::None,
                });
                let mut world = World::solo(
                    seed,
                    &profile,
                    Params {
                        sound_change_rate: 1.0,
                        innovation_rate: 0.0,
                        preference_pull: 0.0,
                        name_turnover: 0.0,
                        ..Params::static_society()
                    },
                );
                world
                    .laws
                    .retain(|law| matches!(law.id, "unstressed-reduction" | "unstressed-apocope"));
                world.varieties[0].minimal = MinimalWord::Syllable;
                world.run(80);
                losses[i] +=
                    1.0 - world.varieties[0].grammar.summary.categories[0].contrast_retention;
            }
        }
        for loss in &mut losses {
            *loss /= 30.0;
        }
        assert!(
            losses[0] > losses[1] + 0.08,
            "initial/final/prefix: {losses:?}"
        );
        assert!(
            losses[0] > losses[2] + 0.08,
            "initial/final/prefix: {losses:?}"
        );
        assert!((0.08..=0.85).contains(&losses[0]), "{losses:?}");
    }
}

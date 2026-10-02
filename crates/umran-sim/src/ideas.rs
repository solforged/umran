//! Ideas: crafts and founded religions, which peoples hold and pass on
//! along their contacts, as they pass on words. Each idea brings meanings
//! a language has no word for until its speakers hold the idea: no one
//! says "iron" before metalworking. When the idea arrives the language
//! finds a word, in the ways real languages do: it borrows the word of
//! those who taught it (English priest and bishop came with the Church),
//! stretches a word it has (English write first meant "to scratch"),
//! builds one from its own parts (German Gewissen copies Latin
//! con-scientia), or coins one. Purist standards build; open peoples
//! borrow.
//!
//! A founded religion begins with a founder among the subjects of an old
//! state in a time of troubles, which is Toynbee's "universal church", or
//! by an author's hand. Its founder's speech, frozen as it stood, becomes
//! its sacred language, which keeps lending words to the faithful long
//! after, as Latin lent English fragile beside the inherited frail.
//! Conversion changes old words too: converts to a faith that keeps its
//! sacred language may turn their old gods into demons and their old
//! priests into sorcerers, as Greek daimōn became the Christian demon.

use crate::adapt::Adapter;
use crate::concepts::{CONCEPTS, Concept, Relation, by_id, related};
use crate::diglossia::Vernacular;
use crate::ethos::{Axis, Effect, TemperCause};
use crate::form::Form;
use crate::geography::{LandmassKind, Terrain};
use crate::lexicon::{Entry, Event, LexemeId, Origin};
use crate::livelihood::Livelihood;
use crate::names::{GivenName, MAX_PEOPLE_NAME, Name, clipped, given_name};
use crate::phonotactics::Phonotactics;
use crate::rng::{index, key, stream, weighted_index};
use crate::root::mint_one;
use crate::schisms::{BranchNaming, HolyLand, Pilgrimage, SchismCause};
use crate::world::{ContactKind, World, WorldEvent};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A skill a people holds and can teach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Craft {
    /// Bronze and iron: it begins among peoples with hills or mountains
    /// and makes them stronger in war.
    Metalworking,
    /// It begins among herders of the steppe and carries them far, as it
    /// carried the steppe peoples again and again.
    Riding,
    /// Without it no people crosses the sea.
    Seafaring,
    /// It begins only at the court of a great city, as in Sumer, Egypt,
    /// China, and Mesoamerica; everyone else learns it from someone.
    Writing,
}

impl Craft {
    pub const ALL: [Craft; 4] = [
        Craft::Metalworking,
        Craft::Riding,
        Craft::Seafaring,
        Craft::Writing,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Craft::Metalworking => "metalworking",
            Craft::Riding => "riding",
            Craft::Seafaring => "seafaring",
            Craft::Writing => "writing",
        }
    }

    /// How readily it passes along a contact of `kind`: crafts travel
    /// with traders and rulers, and writing with priests too.
    fn carried(self, kind: ContactKind) -> f32 {
        match (self, kind) {
            (_, ContactKind::Trade) => 2.0,
            (_, ContactKind::Rule) => 1.5,
            (Craft::Writing, ContactKind::Religion) => 3.0,
            (_, ContactKind::Religion) => 0.5,
            (_, ContactKind::Neighbours | ContactKind::Intermarriage) => 1.0,
        }
    }

    /// How much likelier than `craft_rate` a people that may invent it
    /// is to. Coasts invite boats early; ores and horses take longer
    /// to master; writing follows the first great cities within a few
    /// centuries, as in Sumer.
    fn invention(self) -> f32 {
        match self {
            Craft::Seafaring => 6.0,
            Craft::Metalworking => 4.0,
            Craft::Riding => 8.0,
            Craft::Writing => 30.0,
        }
    }
}

/// What a people must hold for its language to have a word for a meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Need {
    Craft(Craft),
    Livelihood(Livelihood),
    /// Any founded religion.
    Faith,
}

/// Meanings that wait for an idea. All others every language has.
pub static NEEDS: &[(&str, Need)] = &[
    ("iron", Need::Craft(Craft::Metalworking)),
    ("bronze", Need::Craft(Craft::Metalworking)),
    ("smith", Need::Craft(Craft::Metalworking)),
    ("ride", Need::Craft(Craft::Riding)),
    ("saddle", Need::Craft(Craft::Riding)),
    ("sail", Need::Craft(Craft::Seafaring)),
    ("ship", Need::Craft(Craft::Seafaring)),
    ("write", Need::Craft(Craft::Writing)),
    ("book", Need::Craft(Craft::Writing)),
    ("letter", Need::Craft(Craft::Writing)),
    ("till", Need::Livelihood(Livelihood::Farming)),
    ("herd", Need::Livelihood(Livelihood::Herding)),
    ("holy", Need::Faith),
    ("sin", Need::Faith),
    ("temple", Need::Faith),
    ("prophet", Need::Faith),
    ("heaven", Need::Faith),
    ("demon", Need::Faith),
    ("sorcerer", Need::Faith),
];

/// What a people must hold to have a word for `concept`, if anything.
pub fn need(concept: &Concept) -> Option<Need> {
    NEEDS
        .iter()
        .find(|(id, _)| *id == concept.id)
        .map(|(_, n)| *n)
}

/// How a way of life shows in words: meanings a people living that way
/// stretches the first word to cover. Herders count wealth in cattle
/// (Latin pecunia from pecus, "cattle"; English fee from Germanic fehu,
/// "cattle, property") and call a priest a shepherd (Latin pastor).
/// Farmers call offspring seed (Hebrew zera, Greek sperma) and worship
/// tilling (Latin colere, "to till, to tend, to worship", whence cult).
pub static LIVING_RELATED: &[(Livelihood, &str, &str)] = &[
    (Livelihood::Herding, "cattle", "wealth"),
    (Livelihood::Herding, "herd", "priest"),
    (Livelihood::Farming, "seed", "child"),
    (Livelihood::Farming, "till", "pray"),
];

/// Concepts whose word a people living by `livelihood` may stretch to
/// cover `concept`.
pub fn living_related(
    concept: &Concept,
    livelihood: Livelihood,
) -> impl Iterator<Item = &'static Concept> + '_ {
    LIVING_RELATED
        .iter()
        .filter(move |(l, _, to)| *l == livelihood && *to == concept.id)
        .filter_map(|(_, from, _)| by_id(from))
}

/// Words a language may build for an idea's meanings from words it has.
static BUILDS: &[(&str, &str, Relation)] = &[
    ("iron", "smith", Relation::Agent),
    ("ride", "saddle", Relation::Instrument),
    ("write", "letter", Relation::Result),
    ("write", "book", Relation::Result),
    ("field", "till", Relation::Action),
    ("cattle", "herd", Relation::Collective),
    ("pray", "temple", Relation::Place),
    ("say", "prophet", Relation::Agent),
    ("god", "heaven", Relation::Place),
    ("god", "holy", Relation::Abstract),
];

/// Prestige from metalworking, and from riding for herders: arms that
/// win wars.
pub(crate) const MIGHT: f32 = 0.1;
/// How much farther riders range and hold together.
pub(crate) const RIDING_MOBILITY: f32 = 1.5;
/// City a state's capital needs before its rulers may invent writing.
const WRITING_CITY: f32 = 10000.0;
/// How much more slowly a written standard changes.
pub(crate) const WRITTEN_PACE: f32 = 0.8;
/// Generations a state must have stood, and the size its subjects must
/// have, before a faith may be founded among them.
const FAITH_STATE_AGE: u32 = 8;
const FAITH_SIZE: f32 = 5000.0;
/// Pressure counted when no troubles struck lately.
const FAITH_COMFORT: f32 = 0.1;
/// Chances that a founded faith seeks converts, and that it translates
/// rather than keeping its sacred language.
const CONVERTING: f32 = 0.7;
const TRANSLATING: f32 = 0.35;
/// Chance that a founder's reform turns his people's old gods into
/// demons and calls the one god "lord", as Zoroaster's made the daevas
/// devils among the Iranians while the Indians kept deva for "god".
const REFORM: f32 = 0.4;
/// Chance that converts to a faith keeping its sacred language turn
/// their old god into a demon, and again their old priest into a
/// sorcerer (Persian magus, a priest, gave magic).
const PEJORATION: f32 = 0.6;
/// Usage share a word replacing a pejorated one takes at once.
const REPLACING_SHARE: f32 = 0.7;
/// How much less readily a faith seeking no converts spreads, and a
/// people of one founded faith takes another.
const CLOSED: f32 = 0.05;
const RIVAL: f32 = 0.2;
/// Standing of a sacred language's words, whoever speaks it now.
pub(crate) const SACRED_PRESTIGE: f32 = 0.8;
/// How closely the faithful deal with their sacred language, doubled for
/// a people that reads.
pub(crate) const SACRED_INTENSITY: f32 = 0.2;
/// Names converts take from their faith's sacred language.
const SACRED_NAMES: usize = 2;
/// Chance that converts to a faith written down learn writing with it.
const SCRIPTURE: f32 = 0.5;
/// Weights of the ways to a word for a new meaning, besides borrowing:
/// stretching an old word, building one, and coining one. Borrowing from
/// a sacred language the faith keeps weighs this much more.
const STRETCH: f32 = 0.6;
const BUILD: f32 = 0.6;
const COIN: f32 = 0.3;
const SACRED_BORROW: f32 = 4.0;
/// Laws since a standard was last spelled before its state may reform
/// its spelling, and the chance per generation that it does.
const RESPELL_LAWS: usize = 6;
const RESPELL: f32 = 0.05;

/// Why a known land was revered as a religion's sacred place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SacredKind {
    Home,
    Mountain,
    Island,
    FarShore,
}

/// A site and its name, frozen in the faith's sacred language.
#[derive(Clone, Debug, PartialEq)]
pub struct SacredPlace {
    pub region: usize,
    pub kind: SacredKind,
    pub name: Name,
    /// Speech in which this fixed name was coined.
    pub variety: usize,
}

/// A founded religion.
#[derive(Clone, Debug, PartialEq)]
pub struct Religion {
    /// Coined in its founder's language: "the followers of Zarat", or a
    /// word such as "the law" or "the way".
    pub name: Name,
    /// Speech in which the faith's fixed name was coined.
    pub name_variety: usize,
    /// As said when he lived.
    pub founder: Name,
    /// The people the founder came from, and its heart land then.
    pub people: usize,
    pub land: usize,
    pub founded: u32,
    pub how: Revelation,
    /// The variety holding the founder's speech as it stood, frozen.
    pub sacred: usize,
    /// Its revered site, distinct from founder-home provenance in `land`.
    pub shrine: SacredPlace,
    /// Additional holy places inherited or revered by a branch.
    pub extra_shrines: Vec<SacredPlace>,
    pub parent: Option<usize>,
    pub split: Option<u32>,
    pub cause: Option<SchismCause>,
    pub named: Option<BranchNaming>,
    pub pilgrims: Vec<Pilgrimage>,
    /// Landmasses whose first pilgrims have already been recorded.
    pub(crate) pilgrim_landmasses: Vec<usize>,
    /// Last observed allegiance at each shrine, for transition events.
    pub(crate) holy_land: Vec<HolyLand>,
    /// Whether it seeks converts.
    pub converts: bool,
    /// Whether converts render its words in their own speech, rather than
    /// taking them from its sacred language.
    pub translates: bool,
    /// Whether its founder's people wrote, so that its teaching is written.
    pub scripture: bool,
}

impl Religion {
    pub fn shrines(&self) -> impl Iterator<Item = &SacredPlace> {
        std::iter::once(&self.shrine).chain(self.extra_shrines.iter())
    }
}

/// How a religion was founded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Revelation {
    /// Among the subjects of an old state in a time of troubles.
    Troubles,
    /// Among them in quiet times.
    Quiet,
    /// By an author's hand.
    Proclaimed,
}

impl World {
    /// Whether `community` holds what `need` asks.
    pub fn holds(&self, community: usize, need: Need) -> bool {
        let k = &self.communities[community];
        match need {
            Need::Craft(craft) => k.crafts.contains(&craft),
            Need::Livelihood(l) => k.livelihood == l,
            Need::Faith => k.faith.is_some(),
        }
    }

    /// The peoples `community` deals with, how closely, and how.
    pub(crate) fn partners(
        &self,
        community: usize,
    ) -> impl Iterator<Item = (usize, f32, ContactKind)> + '_ {
        self.contacts.iter().filter_map(move |k| {
            let other = match (k.a == community, k.b == community) {
                (true, _) => k.b,
                (_, true) => k.a,
                _ => return None,
            };
            Some((other, k.intensity, k.kind))
        })
    }

    /// Prestige a people's crafts add.
    pub(crate) fn might(&self, community: usize) -> f32 {
        let k = &self.communities[community];
        let metal = k.crafts.contains(&Craft::Metalworking);
        let riders = k.crafts.contains(&Craft::Riding) && k.livelihood == Livelihood::Herding;
        MIGHT * (f32::from(u8::from(metal)) + f32::from(u8::from(riders)))
    }

    /// How readily a people takes to the road, by its way of life and,
    /// for herders, riding.
    pub(crate) fn mobility(&self, community: usize) -> f32 {
        let k = &self.communities[community];
        let riding = k.crafts.contains(&Craft::Riding) && k.livelihood == Livelihood::Herding;
        k.livelihood.mobility() * if riding { RIDING_MOBILITY } else { 1.0 }
    }

    /// Whether a people can cross the sea.
    pub(crate) fn sails(&self, community: usize) -> bool {
        self.communities[community]
            .crafts
            .contains(&Craft::Seafaring)
    }

    /// The largest living people speaking `variety`.
    pub(crate) fn speakers(&self, variety: usize) -> Option<usize> {
        self.living()
            .filter(|&c| self.communities[c].variety == variety)
            .max_by(|&a, &b| {
                self.communities[a]
                    .size
                    .total_cmp(&self.communities[b].size)
                    .then(b.cmp(&a))
            })
    }

    /// Peoples learn crafts from those they deal with, likelier the closer
    /// the dealings and the better that kind of dealings carries the
    /// craft, or come upon one themselves where their land and life invite
    /// it. All decided against this generation's state.
    pub(crate) fn spread_crafts(&mut self) {
        if self.params.craft_rate <= 0.0 && self.params.idea_rate <= 0.0 {
            return;
        }
        let mut learnt = Vec::new();
        let contacts = self.contact_index();
        for c in self.living().collect::<Vec<_>>() {
            let mut rng = self.community_rng(c, "craft");
            for craft in Craft::ALL {
                if self.communities[c].crafts.contains(&craft) {
                    continue;
                }
                let teachers: Vec<(usize, f32)> = contacts
                    .partners(c)
                    .filter(|&(o, _, _)| self.communities[o].crafts.contains(&craft))
                    .map(|(o, intensity, kind)| {
                        (o, self.params.idea_rate * intensity * craft.carried(kind))
                    })
                    .collect();
                let invent = self.params.craft_rate
                    * craft.invention()
                    * self.invites(c, craft)
                    * if craft == Craft::Seafaring {
                        self.communities[c].ethos.factor(Effect::Seafaring)
                    } else {
                        1.0
                    };
                let taught: f32 = teachers.iter().map(|t| t.1).sum();
                let draw = rng.r#gen::<f32>();
                if draw >= invent + taught {
                    continue;
                }
                let from = (draw >= invent)
                    .then(|| teachers[weighted_index(&mut rng, teachers.iter().map(|t| t.1))].0);
                learnt.push((c, craft, from));
            }
        }
        for (c, craft, from) in learnt {
            self.learn(c, craft, from);
        }
    }

    /// 1 when `community`'s land and life invite it to come upon `craft`
    /// itself, else 0. Ore and wild horses count within reach: on a land
    /// it holds or one bordering it, as the first smiths were farmers
    /// below the ore-bearing hills.
    fn invites(&self, community: usize, craft: Craft) -> f32 {
        let k = &self.communities[community];
        let has = |t: Terrain| {
            k.lands.iter().any(|&r| {
                let region = &self.map.regions[r];
                region.terrain == t
                    || region
                        .neighbours
                        .iter()
                        .any(|&n| self.map.regions[n].terrain == t)
            })
        };
        let yes = match craft {
            Craft::Metalworking => {
                k.livelihood != Livelihood::Foraging
                    && (has(Terrain::Hills) || has(Terrain::Mountains))
            }
            Craft::Riding => k.livelihood == Livelihood::Herding && has(Terrain::Steppe),
            Craft::Seafaring => k.lands.iter().any(|&r| self.map.coastal(r)),
            Craft::Writing => self
                .rules(community)
                .is_some_and(|s| self.city(s) >= WRITING_CITY),
        };
        f32::from(u8::from(yes))
    }

    /// `community` takes up `craft`, taught by `from` or by itself.
    pub fn learn(&mut self, community: usize, craft: Craft, from: Option<usize>) {
        let k = &mut self.communities[community];
        if k.crafts.contains(&craft) {
            return;
        }
        k.crafts.push(craft);
        k.crafts.sort();
        let v = k.variety;
        if craft == Craft::Writing {
            self.begin_writing(v);
        }
        self.events.push((
            self.generation,
            WorldEvent::Learnt {
                community,
                craft,
                from,
            },
        ));
        if craft == Craft::Seafaring {
            self.nudge_ethos(community, Axis::Seaward, 0.12, TemperCause::Seafaring);
        }
    }

    /// Every people that holds an idea has words for its meanings. Where
    /// its language has none yet, it finds one (`find_word`). A language
    /// is written from when a people speaking it first writes, unless they
    /// write a classical form instead.
    pub(crate) fn learn_words(&mut self) {
        for c in self.living().collect::<Vec<_>>() {
            let v = self.communities[c].variety;
            if self.holds(c, Need::Craft(Craft::Writing)) {
                self.begin_writing(v);
            }
            for concept in CONCEPTS {
                let Some(need) = need(concept) else { continue };
                if self.holds(c, need)
                    && self.varieties[v].lexicon.slot(concept).variants.is_empty()
                {
                    self.find_word(c, concept);
                }
            }
        }
    }

    /// A word for `concept`, new to `community`'s language: borrowed from
    /// those who taught the idea (from a faith's sacred language, if the
    /// faith keeps it), stretched from a word for a related meaning or one
    /// its way of life links, built from a word it has, or coined. Open
    /// peoples borrow; purist standards and translating faiths build.
    fn find_word(&mut self, community: usize, concept: &'static Concept) {
        let k = &self.communities[community];
        let v = k.variety;
        let mut rng = self.at(v).rng(&[key("learn"), key(concept.id)]);
        let purism = self.purism(v);
        let faith = match need(concept) {
            Some(Need::Faith) => k.faith.map(|r| &self.religions[r]),
            _ => None,
        };
        let lexicon = &self.varieties[v].lexicon;
        let sacred = faith.filter(|r| !r.translates && r.sacred != v);
        let teacher = match sacred {
            Some(r) => Some(r.sacred),
            None => need(concept).and_then(|need| {
                self.partners(community)
                    .filter(|&(o, _, _)| {
                        let ov = self.communities[o].variety;
                        ov != v
                            && self.holds(o, need)
                            && self.varieties[ov]
                                .lexicon
                                .slot(concept)
                                .dominant()
                                .is_some()
                    })
                    .max_by(|a, b| {
                        let weight = |&(o, i, _): &(usize, f32, ContactKind)| {
                            i * (0.1 + self.communities[o].prestige)
                        };
                        weight(a).total_cmp(&weight(b)).then(b.0.cmp(&a.0))
                    })
                    .map(|(o, _, _)| self.communities[o].variety)
            }),
        };
        let mut donors: Vec<LexemeId> = related(concept)
            .chain(living_related(concept, k.livelihood))
            .filter_map(|other| lexicon.slot(other).dominant())
            .collect();
        donors.sort();
        donors.dedup();
        let morphology = &self.varieties[v].morphology;
        let build = BUILDS
            .iter()
            .filter(|(_, word, _)| *word == concept.id)
            .find_map(|&(base, _, relation)| {
                let base = lexicon.slot(by_id(base)?).dominant()?;
                let form = morphology.derive(&lexicon.get(base).form, None, relation)?;
                let free = !lexicon.living().any(|l| l.form == form);
                free.then_some((form, base, relation))
            });
        let borrow = k.openness
            * (1.0 - purism)
            * if sacred.is_some() { SACRED_BORROW } else { 1.0 }
            * k.ethos.factor(Effect::Borrowing);
        let translating = faith.is_some_and(|r| r.translates);
        let weights = [
            if teacher.is_some() { borrow } else { 0.0 },
            if donors.is_empty() { 0.0 } else { STRETCH },
            match build {
                Some(_) => BUILD + 2.0 * purism + f32::from(u8::from(translating)),
                None => 0.0,
            },
            COIN,
        ];
        let generation = self.generation;
        let id = match weighted_index(&mut rng, weights.into_iter()) {
            0 => match self.loan(
                v,
                teacher.expect("weighted only with a teacher"),
                concept,
                &mut rng,
            ) {
                Some(id) => id,
                None => return,
            },
            1 => {
                let id = donors[index(&mut rng, donors.len())];
                self.varieties[v].lexicon.get_mut(id).log.push(Entry {
                    generation,
                    event: Event::Extended { to: concept },
                });
                id
            }
            2 => {
                let (form, base, relation) = build.expect("weighted only with a build");
                self.varieties[v].lexicon.coin(
                    form,
                    Origin::Derived { base, relation },
                    concept,
                    generation,
                )
            }
            _ => {
                let variety = &self.varieties[v];
                let lexicon = &variety.lexicon;
                let observed = Phonotactics::observe(lexicon.living().map(|l| &l.form));
                let used: HashSet<Form> = lexicon.living().map(|l| l.form.clone()).collect();
                let field: HashSet<Form> = lexicon
                    .slots
                    .iter()
                    .filter(|s| s.concept.field == concept.field)
                    .flat_map(|s| s.variants.iter())
                    .map(|var| lexicon.get(var.lexeme).form.clone())
                    .collect();
                let form = mint_one(
                    &mut rng,
                    &observed,
                    &variety.profile.spelling,
                    Some(&variety.morphology),
                    concept,
                    &used,
                    &field,
                );
                self.varieties[v]
                    .lexicon
                    .coin(form, Origin::Expressive, concept, generation)
            }
        };
        self.varieties[v]
            .lexicon
            .slot_mut(concept)
            .introduce(id, 1.0);
    }

    /// `from`'s word for `concept`, taken into variety `v` and fitted to its
    /// sounds; the same loanword again if `v` already took that word for
    /// another meaning. `None` if `from` has no word for it. The caller
    /// gives it its place among `concept`'s words.
    fn loan(
        &mut self,
        v: usize,
        from: usize,
        concept: &'static Concept,
        rng: &mut impl Rng,
    ) -> Option<LexemeId> {
        let generation = self.generation;
        let source = self.varieties[from].lexicon.slot(concept).dominant()?;
        let origin = Origin::Borrowed { from, source };
        let lexicon = &mut self.varieties[v].lexicon;
        let existing = lexicon.living().find(|l| l.origin == origin).map(|l| l.id);
        if let Some(id) = existing {
            lexicon.get_mut(id).log.push(Entry {
                generation,
                event: Event::Extended { to: concept },
            });
            return Some(id);
        }
        let source_form = self.varieties[from].lexicon.get(source).form.clone();
        let form = self
            .ear_of(v)
            .adapt(&source_form, self.params.bilingual_keep / 2.0, rng);
        let lexicon = &mut self.varieties[v].lexicon;
        let id = lexicon.coin(form, origin, concept, generation);
        lexicon.get_mut(id).log.push(Entry {
            generation,
            event: Event::Borrowed {
                from,
                source: source_form,
            },
        });
        Some(id)
    }

    /// How speakers of `v` hear foreign words.
    fn ear_of(&self, v: usize) -> Adapter {
        let variety = &self.varieties[v];
        Adapter::new(
            variety.lexicon.living().map(|l| &l.form),
            &variety.profile.inventory,
        )
    }

    /// A new given name in `community`'s language, the way it names its
    /// children, not already in its stock.
    fn coin_given(&self, community: usize, rng: &mut impl Rng) -> Option<Name> {
        let k = &self.communities[community];
        let variety = &self.varieties[k.variety];
        let devout = k.faith.is_some();
        (0..8).find_map(|_| {
            let name = given_name(variety, k.livelihood, devout, k.ethos, rng, self.generation)?;
            let taken = variety.given.iter().any(|g| g.name.form == name.form);
            (!taken).then_some(name)
        })
    }

    /// Puts `name` among the given names of variety `v`, in place of one
    /// gone out of fashion once the stock is full.
    fn give_name(&mut self, v: usize, given: GivenName, rng: &mut impl Rng) {
        let stock = &mut self.varieties[v].given;
        if stock.iter().any(|g| g.name.form == given.name.form) {
            return;
        }
        if stock.len() < crate::names::GIVEN_STOCK {
            stock.push(given);
        } else {
            let i = index(rng, stock.len());
            stock[i] = given;
        }
    }

    /// Names go in and out of fashion: now and then a language takes up a
    /// new one of its own making.
    pub(crate) fn renew_names(&mut self, v: usize) {
        let mut rng = self.at(v).rng(&[key("given")]);
        if rng.r#gen::<f32>() >= self.params.name_turnover {
            return;
        }
        let Some(c) = self.speakers(v) else { return };
        if let Some(name) = self.coin_given(c, &mut rng) {
            self.give_name(v, GivenName { name, from: None }, &mut rng);
        }
    }

    /// Faiths are founded among the subjects of states that have stood a
    /// while, mostly in a time of troubles: bad times on their lands or
    /// their rulers', or being crowded off land.
    pub(crate) fn found_religions(&mut self) {
        if self.params.religion_rate <= 0.0 {
            return;
        }
        let (struck, crowded) = self.recent_challenges();
        for c in self.living().collect::<Vec<_>>() {
            let k = &self.communities[c];
            if k.faith.is_some() || k.size < FAITH_SIZE {
                continue;
            }
            let Some(s) = self.ruled_by(c) else { continue };
            let state = &self.states[s];
            if self.generation < state.rose + FAITH_STATE_AGE {
                continue;
            }
            let rulers = &self.communities[state.rulers];
            let troubled = crowded.contains(&c)
                || k.lands
                    .iter()
                    .chain(rulers.lands.iter())
                    .any(|r| struck.contains(r));
            let pressure = if troubled { 1.0 } else { FAITH_COMFORT };
            let mut rng = self.community_rng(c, "faith");
            if rng.r#gen::<f32>()
                < self.params.religion_rate * pressure * k.ethos.factor(Effect::Faith)
            {
                let how = if troubled {
                    Revelation::Troubles
                } else {
                    Revelation::Quiet
                };
                self.found_religion(c, how);
            }
        }
    }

    /// A faith founded among `community` by a founder of theirs, `how`;
    /// returns its index. The founder's people take it up and find words
    /// for it in their own speech, perhaps turning their old gods into
    /// demons by his reform; then their speech, frozen as it stands,
    /// becomes the faith's sacred language.
    pub fn found_religion(&mut self, community: usize, how: Revelation) -> usize {
        self.refresh_places();
        let index = self.religions.len();
        let generation = self.generation;
        let mut rng = stream(self.seed, &[key("religion"), index as u64]);
        let converts = rng.r#gen::<f32>() < CONVERTING;
        let translates = rng.r#gen::<f32>() < TRANSLATING;
        let reform = rng.r#gen::<f32>() < REFORM;
        let v = self.communities[community].variety;
        let founder = self
            .coin_given(community, &mut rng)
            .unwrap_or_else(|| Name {
                coined: generation,
                ..self.communities[community].name.clone()
            });
        let old_faith = self.communities[community].faith;
        let mut shrine_rng = stream(self.seed, &[key("sacred place"), index as u64]);
        let shrine = self.sacred_place(community, false, &mut shrine_rng);
        self.religions.push(Religion {
            name: Name::default(),
            name_variety: v,
            founder: founder.clone(),
            people: community,
            land: self.communities[community].home(),
            founded: generation,
            how,
            sacred: v,
            shrine,
            extra_shrines: Vec::new(),
            parent: None,
            split: None,
            cause: None,
            named: None,
            pilgrims: Vec::new(),
            pilgrim_landmasses: Vec::new(),
            holy_land: Vec::new(),
            converts,
            translates,
            scripture: self.holds(community, Need::Craft(Craft::Writing)),
        });
        self.communities[community].faith = Some(index);
        self.give_name(
            v,
            GivenName {
                name: founder.clone(),
                from: None,
            },
            &mut rng,
        );
        if reform && old_faith.is_none() {
            let lord =
                by_id("chief").and_then(|chief| self.varieties[v].lexicon.slot(chief).dominant());
            if let Some(lord) = lord {
                self.pejorate(community, "god", "demon", lord);
            }
        }
        for concept in CONCEPTS {
            if need(concept) == Some(Need::Faith)
                && self.varieties[v].lexicon.slot(concept).variants.is_empty()
            {
                self.find_word(community, concept);
            }
        }
        let mut sacred = self.varieties[v].fork(v, generation);
        self.inherit_places(v, &mut sacred);
        sacred.name = self.varieties[v].name.clone();
        self.varieties.push(sacred);
        self.religions[index].sacred = self.varieties.len() - 1;
        self.religions[index].name_variety = self.religions[index].sacred;
        self.religions[index].shrine.variety = self.religions[index].sacred;
        self.religions[index].name = self.coin_faith_name(community, &founder, &mut rng);
        self.events
            .push((generation, WorldEvent::Revealed { religion: index }));
        self.nudge_ethos(community, Axis::Pious, 0.12, TemperCause::Faith);
        // He teaches in his own speech, as the Buddha did in a vernacular
        // rather than Sanskrit; written down, it is written in its own right.
        if self.religions[index].scripture {
            self.write_vernacular(v, Vernacular::Scripture { religion: index });
        }
        self.religions[index].holy_land = self.holy_lands(index);
        index
    }

    pub(crate) fn sacred_place(
        &self,
        community: usize,
        local: bool,
        rng: &mut impl Rng,
    ) -> SacredPlace {
        let people = &self.communities[community];
        let home = people.home();
        let knowledge: Vec<_> = self
            .known_lands(people.variety)
            .into_iter()
            .filter(|r| !local || people.lands.contains(r))
            .collect();
        let categories = [
            (SacredKind::Home, vec![home], 3.0),
            (
                SacredKind::Mountain,
                knowledge
                    .iter()
                    .copied()
                    .filter(|&r| self.map.regions[r].terrain == Terrain::Mountains)
                    .collect(),
                3.0,
            ),
            (
                SacredKind::Island,
                knowledge
                    .iter()
                    .copied()
                    .filter(|&r| self.map.island(r))
                    .collect(),
                2.0,
            ),
            (
                SacredKind::FarShore,
                knowledge
                    .iter()
                    .copied()
                    .filter(|&r| {
                        self.map.coastal(r)
                            && self.map.regions[r].landmass != self.map.regions[home].landmass
                            && self.map.regions[r].landmass.is_some_and(|id| {
                                self.map.landmasses[id].kind == LandmassKind::Continent
                            })
                    })
                    .collect(),
                2.0,
            ),
        ];
        let eligible: Vec<_> = categories
            .iter()
            .filter(|(_, regions, _)| !regions.is_empty())
            .collect();
        let (kind, regions, _) = eligible[weighted_index(rng, eligible.iter().map(|c| c.2))];
        let region = regions[index(rng, regions.len())];
        SacredPlace {
            region,
            kind: *kind,
            variety: people.variety,
            name: self
                .known_place(people.variety, region)
                .expect("a founder's held and known lands have names")
                .clone(),
        }
    }

    /// A faith's name in its founder's speech: his followers' ("the
    /// followers of Zarat", as Christians and Buddhists are named), or a
    /// word for what it teaches ("the law", "the way", as Dharma and Dao).
    fn coin_faith_name(&self, community: usize, founder: &Name, rng: &mut impl Rng) -> Name {
        let variety = &self.varieties[self.communities[community].variety];
        let teachings: Vec<(&str, &Form)> = [
            ("law", "the law"),
            ("path", "the way"),
            ("word", "the word"),
            ("light", "the light"),
        ]
        .iter()
        .filter_map(|&(id, meaning)| {
            let word = variety.lexicon.word_for(by_id(id)?)?;
            Some((meaning, &word.form))
        })
        .collect();
        let (form, meaning) = if teachings.is_empty() || rng.r#gen::<f32>() < 0.5 {
            (
                variety.morphology.belonging(&founder.form),
                format!("the followers of {}", variety.title(&founder.form)),
            )
        } else {
            let (meaning, form) = teachings[index(rng, teachings.len())];
            (form.clone(), meaning.to_string())
        };
        Name {
            form: clipped(form, MAX_PEOPLE_NAME + 2),
            meaning,
            coined: self.generation,
            log: Vec::new(),
        }
    }

    /// In `community`'s speech, the word for `from` comes to mean `to`,
    /// and gives way in its old meaning to `new`: the old gods become
    /// demons, and the one god is called by another word.
    fn pejorate(&mut self, community: usize, from: &str, to: &str, new: LexemeId) {
        let (Some(from), Some(to)) = (by_id(from), by_id(to)) else {
            return;
        };
        let generation = self.generation;
        let v = self.communities[community].variety;
        let lexicon = &mut self.varieties[v].lexicon;
        let Some(old) = lexicon.slot(from).dominant() else {
            return;
        };
        if old == new || lexicon.slot(to).has(old) {
            return;
        }
        let share = if lexicon.slot(to).variants.is_empty() {
            1.0
        } else {
            REPLACING_SHARE
        };
        lexicon.slot_mut(to).introduce(old, share);
        lexicon.get_mut(old).log.push(Entry {
            generation,
            event: Event::Extended { to },
        });
        if !lexicon.slot(from).has(new) {
            if lexicon.get(new).first_sense.id != from.id {
                lexicon.get_mut(new).log.push(Entry {
                    generation,
                    event: Event::Extended { to: from },
                });
            }
            lexicon.slot_mut(from).introduce(new, REPLACING_SHARE);
        }
        self.events.push((
            generation,
            WorldEvent::Pejorated {
                community,
                variety: v,
                word: old,
                from,
                to,
            },
        ));
    }

    /// Faiths pass along contacts: subjects take their rulers' faith more
    /// readily than rulers their subjects', and priests and traders carry
    /// a faith that seeks converts. A people already of another founded
    /// faith seldom changes. All decided against this generation's state.
    pub(crate) fn spread_faiths(&mut self) {
        if self.params.conversion_rate <= 0.0 || self.religions.is_empty() {
            return;
        }
        let mut converted = Vec::new();
        let contacts = self.contact_index();
        for c in self.living().collect::<Vec<_>>() {
            let k = &self.communities[c];
            let teachers: Vec<(usize, usize, f32)> = contacts
                .partners(c)
                .filter_map(|(o, intensity, kind)| {
                    let other = &self.communities[o];
                    let r = other.faith?;
                    if k.faith == Some(r) {
                        return None;
                    }
                    let carried = match kind {
                        ContactKind::Rule if self.rules_over(o, c) => 1.5,
                        ContactKind::Rule => 0.3,
                        ContactKind::Religion => 3.0,
                        ContactKind::Trade | ContactKind::Intermarriage => 1.0,
                        ContactKind::Neighbours => 0.5,
                    };
                    let open = if self.religions[r].converts {
                        1.0
                    } else {
                        CLOSED
                    };
                    let rival = match k.faith {
                        Some(f) if self.faith_root(f) == self.faith_root(r) => RIVAL * 2.0,
                        Some(_) => RIVAL,
                        None => 1.0,
                    };
                    let pull =
                        crate::math::exp(self.params.prestige_pull * (other.prestige - k.prestige));
                    let w = self.params.conversion_rate
                        * intensity
                        * carried
                        * open
                        * rival
                        * pull
                        * k.ethos.factor(Effect::Conversion);
                    Some((o, r, w))
                })
                .collect();
            let total: f32 = teachers.iter().map(|t| t.2).sum();
            let mut rng = self.community_rng(c, "convert");
            if teachers.is_empty() || rng.r#gen::<f32>() >= total {
                continue;
            }
            let (o, r, _) = teachers[weighted_index(&mut rng, teachers.iter().map(|t| t.2))];
            converted.push((c, r, o));
        }
        for (c, r, o) in converted {
            self.convert(c, r, Some(o));
        }
    }

    /// `community` takes up faith `religion`, taught by `from`. Converts to
    /// a faith that keeps its sacred language may turn their old god into
    /// a demon and their old priest into a sorcerer, taking the sacred
    /// words in their place. They take names from the faith, and may
    /// learn to write with its scripture; a scripture translated into
    /// their speech has them write their own speech.
    pub fn convert(&mut self, community: usize, religion: usize, from: Option<usize>) {
        let mut rng = self.community_rng(community, "conversion");
        self.convert_with_rng(community, religion, from, &mut rng);
    }

    pub(crate) fn convert_with_rng(
        &mut self,
        community: usize,
        religion: usize,
        from: Option<usize>,
        mut rng: &mut impl Rng,
    ) {
        let old = self.communities[community].faith;
        if old == Some(religion) {
            return;
        }
        self.communities[community].faith = Some(religion);
        self.events.push((
            self.generation,
            WorldEvent::Converted {
                community,
                religion,
                from,
            },
        ));
        self.nudge_ethos(community, Axis::Pious, 0.08, TemperCause::Faith);
        let r = &self.religions[religion];
        let (sacred, translates, scripture) = (r.sacred, r.translates, r.scripture);
        let v = self.communities[community].variety;
        if !translates && old.is_none() {
            for (from_meaning, to_meaning) in [("god", "demon"), ("priest", "sorcerer")] {
                if rng.r#gen::<f32>() >= PEJORATION {
                    continue;
                }
                let Some(concept) = by_id(from_meaning) else {
                    continue;
                };
                if let Some(new) = self.loan(v, sacred, concept, &mut rng) {
                    self.pejorate(community, from_meaning, to_meaning, new);
                }
            }
        }
        let mut names: Vec<Name> = vec![self.religions[religion].founder.clone()];
        let stock = &self.varieties[sacred].given;
        if !stock.is_empty() {
            names.push(stock[index(&mut rng, stock.len())].name.clone());
        }
        let ear = self.ear_of(v);
        for name in names.into_iter().take(SACRED_NAMES) {
            let form = ear.adapt(&name.form, self.params.bilingual_keep / 2.0, &mut rng);
            let given = GivenName {
                name: Name {
                    form,
                    coined: self.generation,
                    log: Vec::new(),
                    ..name
                },
                from: Some(sacred),
            };
            self.give_name(v, given, &mut rng);
        }
        if scripture && rng.r#gen::<f32>() < SCRIPTURE {
            self.learn(community, Craft::Writing, from);
        }
        if scripture && translates {
            self.write_vernacular(v, Vernacular::Scripture { religion });
        }
        let r = &self.religions[religion];
        for shrine in r.shrines() {
            if self.known_place(v, shrine.region).is_none() {
                let mut hearing = if r.parent.is_none() {
                    stream(
                        self.seed,
                        &[key("sacred place hearing"), religion as u64, v as u64],
                    )
                } else {
                    stream(
                        self.seed,
                        &[
                            key("schism"),
                            key("hearing"),
                            religion as u64,
                            v as u64,
                            shrine.region as u64,
                        ],
                    )
                };
                let name = Name {
                    form: self.ear(v).adapt(&shrine.name.form, 0.0, &mut hearing),
                    meaning: shrine.name.meaning.clone(),
                    coined: self.generation,
                    log: Vec::new(),
                };
                self.varieties[v].exonyms.push((shrine.region, name));
            }
        }
        self.refresh_places();
    }

    /// A standing state whose written standard has drifted far from its
    /// spelling may reform the spelling to fit speech again.
    pub(crate) fn reform_spelling(&mut self) {
        for (v, s) in self.standards().into_iter().enumerate() {
            let (Some(s), Some(written)) = (s, self.varieties[v].written) else {
                continue;
            };
            let drifted = self.varieties[v]
                .laws
                .iter()
                .filter(|(g, _)| *g > written)
                .count();
            if drifted < RESPELL_LAWS {
                continue;
            }
            let mut rng = stream(
                self.seed,
                &[
                    key("step"),
                    u64::from(self.generation),
                    key("respell"),
                    s as u64,
                ],
            );
            if rng.r#gen::<f32>() < RESPELL {
                self.varieties[v].written = Some(self.generation);
                self.events
                    .push((self.generation, WorldEvent::Respelled { variety: v }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::Naming;
    use crate::profile::SoundProfile;
    use crate::world::Params;

    fn word(world: &World, community: usize, id: &str) -> Option<LexemeId> {
        world
            .variety_of(community)
            .lexicon
            .slot(by_id(id).unwrap())
            .dominant()
    }

    /// A world of one people living by `livelihood`.
    fn people(seed: u64, livelihood: Livelihood, params: Params) -> World {
        let mut world = World::new(seed, params);
        world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            seed,
            0.5,
            0.5,
            None,
            Some(livelihood),
            None,
        );
        world
    }

    #[test]
    fn meanings_wait_for_their_ideas() {
        let farmers = people(1, Livelihood::Farming, Params::static_society());
        let herders = people(1, Livelihood::Herding, Params::static_society());
        for id in ["iron", "ship", "write", "holy", "ride"] {
            assert!(word(&farmers, 0, id).is_none(), "{id}");
        }
        assert!(word(&farmers, 0, "till").is_some() && word(&farmers, 0, "herd").is_none());
        assert!(word(&herders, 0, "herd").is_some() && word(&herders, 0, "till").is_none());
        let mut world = farmers;
        world.learn(0, Craft::Metalworking, None);
        world.learn_words();
        for id in ["iron", "bronze", "smith"] {
            assert!(word(&world, 0, id).is_some(), "{id}");
        }
    }

    #[test]
    fn open_peoples_borrow_a_crafts_words_and_closed_ones_make_their_own() {
        let loans = |openness: f32| {
            let mut n = 0;
            for seed in 0..30 {
                let mut world = World::new(seed, Params::static_society());
                let smiths = world.found_seeded(
                    &Naming::People,
                    &SoundProfile::base(),
                    seed,
                    0.8,
                    0.5,
                    None,
                    Some(Livelihood::Farming),
                    None,
                );
                let learners = world.found_seeded(
                    &Naming::People,
                    &SoundProfile::by_id("polynesian").unwrap(),
                    seed + 100,
                    0.3,
                    openness,
                    None,
                    Some(Livelihood::Farming),
                    None,
                );
                world.communities[learners].lands = world.communities[smiths].lands.clone();
                world
                    .connect(smiths, learners, 0.6, ContactKind::Trade)
                    .unwrap();
                world.learn(smiths, Craft::Metalworking, None);
                world.learn_words();
                world.learn(learners, Craft::Metalworking, Some(smiths));
                world.learn_words();
                let lexicon = &world.variety_of(learners).lexicon;
                n += ["iron", "bronze", "smith"]
                    .iter()
                    .filter(|id| {
                        let id = lexicon.slot(by_id(id).unwrap()).dominant().unwrap();
                        matches!(lexicon.get(id).origin, Origin::Borrowed { .. })
                    })
                    .count();
            }
            n
        };
        let (open, closed) = (loans(0.9), loans(0.1));
        assert!(
            open > 2 * closed.max(1) && open > 40,
            "open {open}, closed {closed} of 90"
        );
    }

    /// Deva and daēva: one branch's reform turns its word for the gods into
    /// its word for demons, while its sister keeps the same word for gods.
    #[test]
    fn a_reform_leaves_cognates_meaning_god_and_demon() {
        let mut reformed = 0;
        for seed in 0..30 {
            let mut world = World::solo(seed, &SoundProfile::base(), Params::static_society());
            let east = world.split(0, None, 0.5);
            world.found_religion(east, Revelation::Proclaimed);
            let pejorated = world.events.iter().find_map(|(_, e)| match e {
                WorldEvent::Pejorated { variety, word, .. } => Some((*variety, *word)),
                _ => None,
            });
            let Some((variety, demon)) = pejorated else {
                continue;
            };
            reformed += 1;
            let west = world.communities[0].variety;
            let god = word(&world, 0, "god").unwrap();
            assert_eq!(world.root_of(variety, demon), world.root_of(west, god));
            let lexicon = &world.varieties[variety].lexicon;
            assert!(lexicon.slot(by_id("demon").unwrap()).has(demon));
            assert_ne!(lexicon.slot(by_id("god").unwrap()).dominant(), Some(demon));
        }
        assert!((5..=25).contains(&reformed), "{reformed} reforms in 30");
    }

    /// The faithful borrow from their sacred language even words their own
    /// speech inherited, so the old word comes back beside its worn
    /// descendant, as Latin fragile came back beside French-born frail.
    #[test]
    fn a_sacred_language_lends_learned_doublets() {
        let (mut learned, mut doublets) = (0, 0);
        for seed in 0..6 {
            let mut world = World::solo(seed, &SoundProfile::base(), Params::static_society());
            let r = world.found_religion(0, Revelation::Proclaimed);
            world.learn(0, Craft::Writing, None);
            world.run(200);
            let sacred = world.religions[r].sacred;
            let v = world.communities[0].variety;
            let lexicon = &world.varieties[v].lexicon;
            for l in lexicon.living() {
                let Origin::Borrowed { from, source } = l.origin else {
                    continue;
                };
                if from != sacred {
                    continue;
                }
                learned += 1;
                let root = world.root_of(sacred, source);
                doublets += usize::from(
                    lexicon
                        .living()
                        .any(|n| n.form != l.form && world.root_of(v, n.id) == root),
                );
            }
        }
        assert!(
            learned > 0 && doublets > 0,
            "{learned} learned loans, {doublets} doublets"
        );
    }

    /// Herders stretch their word for cattle to wealth (whichever word now
    /// means cattle, often an old word for horse); farmers have no such
    /// link, so their word for wealth is never a stretched one.
    #[test]
    fn herders_count_wealth_in_cattle() {
        let stretched = |livelihood| {
            (0..20u64)
                .filter(|&seed| {
                    let mut world = people(seed, livelihood, Params::static_society());
                    world.run(300);
                    let lexicon = &world.variety_of(0).lexicon;
                    lexicon
                        .slot(by_id("wealth").unwrap())
                        .variants
                        .iter()
                        .any(|v| {
                            lexicon.get(v.lexeme).log.iter().any(
                                |e| matches!(e.event, Event::Extended { to } if to.id == "wealth"),
                            )
                        })
                })
                .count()
        };
        let (herders, farmers) = (
            stretched(Livelihood::Herding),
            stretched(Livelihood::Farming),
        );
        assert!(
            herders >= 6 && farmers == 0,
            "herders {herders}, farmers {farmers} of 20"
        );
    }

    #[test]
    fn herders_name_children_for_horses_and_cattle() {
        let share = |livelihood| {
            let (mut beasts, mut all) = (0, 0);
            for seed in 0..40 {
                let world = people(seed, livelihood, Params::static_society());
                for g in &world.variety_of(0).given {
                    all += 1;
                    beasts += usize::from(
                        g.name
                            .meaning
                            .split('-')
                            .any(|e| e == "horse" || e == "cattle"),
                    );
                }
            }
            beasts as f32 / all as f32
        };
        let (herders, farmers) = (share(Livelihood::Herding), share(Livelihood::Farming));
        assert!(
            herders > 2.0 * farmers,
            "herders {herders:.2}, farmers {farmers:.2}"
        );
    }

    #[test]
    fn writing_keeps_old_spellings_as_speech_moves_on() {
        let mut world = World::solo(5, &SoundProfile::base(), Params::static_society());
        world.learn(0, Craft::Writing, None);
        let v = world.variety_of(0);
        let then: Vec<(LexemeId, String)> = v
            .lexicon
            .living()
            .map(|l| (l.id, v.spell(&l.form)))
            .collect();
        world.run(150);
        let v = world.variety_of(0);
        let mut lagging = 0;
        for (id, spelled) in then {
            let l = v.lexicon.get(id);
            if l.obsolete.is_some() {
                continue;
            }
            assert_eq!(v.written_word(l), spelled);
            lagging += usize::from(spelled != v.spell(&l.form));
        }
        assert!(lagging > 0, "no word is written otherwise than it sounds");
    }

    #[test]
    fn only_seafarers_cross_the_sea() {
        for seed in 0..4 {
            let params = Params {
                craft_rate: 0.0,
                ..Params::default()
            };
            let mut world = World::new(seed, params);
            for _ in 0..3 {
                world.found(&SoundProfile::base(), 0.5, 0.5);
            }
            world.run(200);
            for (_, e) in &world.events {
                if let WorldEvent::Migrated { from, to, .. } | WorldEvent::Split { from, to, .. } =
                    e
                {
                    assert!(!world.map.overseas(*from, *to), "{e:?}");
                }
            }
        }
    }

    #[test]
    fn needs_and_builds_name_known_concepts() {
        for (id, _) in NEEDS {
            assert!(by_id(id).is_some(), "{id}");
        }
        for (base, word, _) in BUILDS {
            assert!(
                by_id(base).is_some() && by_id(word).is_some(),
                "{base}/{word}"
            );
        }
        for (_, from, to) in LIVING_RELATED {
            assert!(by_id(from).is_some() && by_id(to).is_some(), "{from}/{to}");
        }
    }

    fn cultural_faith_world(seed: u64) -> (World, usize) {
        let mut world = World::with_map(
            seed,
            Params::static_society(),
            crate::geography::MapSize::Large,
        );
        let home = world
            .map
            .landmasses
            .iter()
            .find(|m| m.kind == LandmassKind::Continent)
            .unwrap()
            .anchor;
        let founder = world.found_seeded(
            &Naming::People,
            &SoundProfile::base(),
            seed,
            0.5,
            0.5,
            Some(home),
            Some(Livelihood::Farming),
            None,
        );
        (world, founder)
    }

    fn cultural_teach_chart(world: &mut World, community: usize) {
        let variety = world.communities[community].variety;
        let home = world.communities[community].home();
        let name = world.known_place(variety, home).unwrap().clone();
        for region in 0..world.map.regions.len() {
            if world.map.regions[region].terrain.is_land()
                && world.known_place(variety, region).is_none()
            {
                let mut remembered = name.clone();
                remembered.meaning = format!("the remembered shore {region}");
                world.varieties[variety].exonyms.push((region, remembered));
            }
        }
    }

    #[test]
    fn cultural_shrines_use_founder_knowledge_and_available_category_weights() {
        let mut kinds = [0; 4];
        for seed in 0..128 {
            let (mut world, founder) = cultural_faith_world(seed);
            cultural_teach_chart(&mut world, founder);
            let home = world.communities[founder].home();
            let variety = world.communities[founder].variety;
            let known = world.known_lands(variety);
            assert!(known.iter().any(|&r| world.map.island(r)));
            assert!(
                known
                    .iter()
                    .any(|&r| world.map.regions[r].terrain == Terrain::Mountains)
            );
            assert!(known.iter().any(|&r| world.map.coastal(r)
                && world.map.regions[r].landmass != world.map.regions[home].landmass
                && world.map.landmasses[world.map.regions[r].landmass.unwrap()].kind
                    == LandmassKind::Continent));
            let religion = world.found_religion(founder, Revelation::Proclaimed);
            let shrine = &world.religions[religion].shrine;
            assert!(known.contains(&shrine.region));
            assert_eq!(
                world.known_place(variety, shrine.region),
                Some(&shrine.name)
            );
            assert_eq!(
                world.known_place(world.religions[religion].sacred, shrine.region),
                Some(&shrine.name)
            );
            let bucket = match shrine.kind {
                SacredKind::Home => {
                    assert_eq!(shrine.region, home);
                    0
                }
                SacredKind::Mountain => {
                    assert_eq!(world.map.regions[shrine.region].terrain, Terrain::Mountains);
                    1
                }
                SacredKind::Island => {
                    assert!(world.map.island(shrine.region));
                    2
                }
                SacredKind::FarShore => {
                    assert!(world.map.coastal(shrine.region));
                    assert_ne!(
                        world.map.regions[shrine.region].landmass,
                        world.map.regions[home].landmass
                    );
                    assert_eq!(
                        world.map.landmasses[world.map.regions[shrine.region].landmass.unwrap()]
                            .kind,
                        LandmassKind::Continent
                    );
                    3
                }
            };
            kinds[bucket] += 1;
        }
        for (i, count) in kinds.into_iter().enumerate() {
            let band = if i < 2 { 25..=52 } else { 12..=39 };
            assert!(band.contains(&count), "category {i}: {count} of 128");
        }
    }

    #[test]
    fn cultural_sacred_place_draws_do_not_disturb_doctrine_or_religion_names() {
        for seed in 0..12 {
            let (mut local, founder) = cultural_faith_world(seed);
            let mut informed = local.clone();
            cultural_teach_chart(&mut informed, founder);
            let a = local.found_religion(founder, Revelation::Proclaimed);
            let b = informed.found_religion(founder, Revelation::Proclaimed);
            let (a, b) = (&local.religions[a], &informed.religions[b]);
            assert_eq!(
                (a.converts, a.translates, a.scripture),
                (b.converts, b.translates, b.scripture)
            );
            assert_eq!(
                (a.founder.clone(), a.name.clone()),
                (b.founder.clone(), b.name.clone())
            );
            assert_eq!(
                local.varieties[a.sacred].lexicon,
                informed.varieties[b.sacred].lexicon
            );
        }
    }

    #[test]
    fn cultural_conversion_teaches_a_frozen_shrine_without_granting_land() {
        let (mut world, founder) = cultural_faith_world(30);
        let religion = world.found_religion(founder, Revelation::Proclaimed);
        let shrine = world.religions[religion].shrine.clone();
        let sacred = world.religions[religion].sacred;
        let home = (0..world.map.regions.len())
            .find(|&r| {
                world.map.regions[r].terrain.is_land()
                    && r != shrine.region
                    && !world.map.regions[r].neighbours.contains(&shrine.region)
            })
            .unwrap();
        let convert = world.found_seeded(
            &Naming::People,
            &SoundProfile::by_id("iranian").unwrap(),
            31,
            0.5,
            0.5,
            Some(home),
            Some(Livelihood::Farming),
            None,
        );
        let variety = world.communities[convert].variety;
        assert!(world.known_place(variety, shrine.region).is_none());
        world.convert(convert, religion, Some(founder));
        assert_eq!(
            world.known_place(variety, shrine.region).unwrap().meaning,
            shrine.name.meaning
        );
        assert_eq!(world.communities[convert].lands, vec![home]);
        assert!(
            !world.places[shrine.region]
                .iter()
                .any(|p| p.variety == variety)
        );
        world.convert(convert, religion, Some(founder));
        assert_eq!(
            world.varieties[variety]
                .exonyms
                .iter()
                .filter(|(r, _)| *r == shrine.region)
                .count(),
            1
        );
        let frozen_known: Vec<_> = world
            .known_lands(sacred)
            .into_iter()
            .map(|r| (r, world.known_place(sacred, r).unwrap().clone()))
            .collect();
        world.run(80);
        assert_eq!(world.religions[religion].shrine, shrine);
        for (r, name) in frozen_known {
            assert_eq!(world.known_place(sacred, r), Some(&name));
        }
    }

    #[test]
    fn cultural_sacred_forks_snapshot_local_names_not_later_discoveries() {
        let (mut world, founder) = cultural_faith_world(32);
        let variety = world.communities[founder].variety;
        let before = world.known_lands(variety);
        let religion = world.found_religion(founder, Revelation::Proclaimed);
        let sacred = world.religions[religion].sacred;
        assert_eq!(world.known_lands(sacred), before);
        let unknown = (0..world.map.regions.len())
            .find(|&r| {
                world.map.regions[r].terrain.is_land() && world.known_place(variety, r).is_none()
            })
            .unwrap();
        let remembered = world
            .known_place(variety, world.communities[founder].home())
            .unwrap()
            .clone();
        world.varieties[variety].exonyms.push((unknown, remembered));
        assert!(world.known_place(sacred, unknown).is_none());
    }
}

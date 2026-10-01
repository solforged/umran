use serde::Serialize;

/// Semantic fields, following the World Loanword Database's chapters
/// (Haspelmath and Tadmor, 2009). Assignments below are approximate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum Field {
    PhysicalWorld,
    Kinship,
    Animals,
    Body,
    FoodDrink,
    ClothingGrooming,
    House,
    Agriculture,
    BasicActions,
    Motion,
    Possession,
    Spatial,
    Quantity,
    Time,
    SensePerception,
    Emotions,
    Cognition,
    Speech,
    Social,
    Warfare,
    Law,
    Religion,
    Function,
}

impl Field {
    /// The WOLD chapter name.
    pub fn label(self) -> &'static str {
        use Field::*;
        match self {
            PhysicalWorld => "The physical world",
            Kinship => "Kinship",
            Animals => "Animals",
            Body => "The body",
            FoodDrink => "Food and drink",
            ClothingGrooming => "Clothing and grooming",
            House => "The house",
            Agriculture => "Agriculture and vegetation",
            BasicActions => "Basic actions and technology",
            Motion => "Motion",
            Possession => "Possession",
            Spatial => "Spatial relations",
            Quantity => "Quantity",
            Time => "Time",
            SensePerception => "Sense perception",
            Emotions => "Emotions and values",
            Cognition => "Cognition",
            Speech => "Speech and language",
            Social => "Social and political relations",
            Warfare => "Warfare and hunting",
            Law => "Law",
            Religion => "Religion and belief",
            Function => "Function words",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Class {
    Entity,
    Event,
    Property,
    /// Pronouns, deictics, interrogatives, negation, adpositions, adverbs.
    Function,
}

/// A weak cross-linguistic sound-meaning association. Blasi et al. (2016,
/// PNAS) report these across thousands of languages; cited from memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Iconic {
    /// "nose": /n/ and other nasals.
    Nasal,
    /// "tongue": /l/.
    Lateral,
    /// "small": /i/.
    CloseFront,
    /// "red": /r/.
    Rhotic,
    /// "sand": /s/.
    Sibilant,
    /// "breast": /m/.
    LabialNasal,
    /// "mother": the nursery pattern of nasal plus open vowel, often
    /// doubled (mama, nana), as Jakobson described for babbling.
    NurseryMother,
    /// "father": the nursery pattern of lip or tongue-tip stop plus open
    /// vowel, often doubled (papa, baba, tata, dada).
    NurseryFather,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Concept {
    pub id: &'static str,
    pub gloss: &'static str,
    pub field: Field,
    pub class: Class,
    /// Rank on the Leipzig–Jakarta list (Tadmor 2009): 1 is the meaning
    /// least often borrowed. `None` for cultural vocabulary.
    pub stability: Option<u8>,
    /// How culture-bound a non-core concept is; `None` for the core list.
    pub tier: Option<Tier>,
    pub iconic: Option<Iconic>,
    /// Meanings that languages often express with reduplicated or
    /// repeated-consonant words: small things, insects and birds, sounds
    /// and cries, baby talk. Other meanings avoid those shapes.
    pub expressive: bool,
}

/// How culture-bound a concept outside the core list is. Judged per
/// concept by what kind of thing it names, never per semantic field, so
/// field-level borrowing patterns have to emerge rather than be assumed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Tier {
    /// Universal experience: sun, mother, heart, to sing.
    Basic,
    /// Ordinary material and social life: bread, boat, to buy, village.
    Everyday,
    /// Institutions, trade goods, and specialist technology: market,
    /// priest, iron, law.
    Specialized,
}

impl Concept {
    /// How much longer than a language's typical root this concept's word
    /// tends to be: frequent, basic meanings get short words and specialist
    /// ones long (Zipf's law of abbreviation). Core rank stands in for
    /// frequency.
    pub fn length_bias(&self) -> f32 {
        match (self.stability, self.tier) {
            (Some(rank), _) => 0.6 + 0.6 * f32::from(rank - 1) / 99.0,
            (None, Some(Tier::Basic)) => 1.2,
            (None, Some(Tier::Everyday)) => 1.6,
            (None, Some(Tier::Specialized)) | (None, None) => 2.0,
        }
    }

    /// Relative ease of borrowing, from 0.02 for the most stable core
    /// meaning to 1.0 for specialized culture. Core meanings rise
    /// geometrically with Leipzig–Jakarta rank to 0.15 at rank 100; those
    /// ranks were derived from WOLD borrowing data at the meaning level.
    pub fn borrowability(&self) -> f32 {
        match (self.stability, self.tier) {
            (Some(rank), _) => 0.02 * 7.5_f32.powf(f32::from(rank - 1) / 99.0),
            (None, Some(Tier::Basic)) => 0.25,
            (None, Some(Tier::Everyday)) => 0.6,
            (None, Some(Tier::Specialized)) | (None, None) => 1.0,
        }
    }
}

const fn core(
    rank: u8,
    id: &'static str,
    gloss: &'static str,
    field: Field,
    class: Class,
) -> Concept {
    Concept {
        id,
        gloss,
        field,
        class,
        stability: Some(rank),
        tier: None,
        iconic: None,
        expressive: false,
    }
}

const fn culture(
    tier: Tier,
    id: &'static str,
    gloss: &'static str,
    field: Field,
    class: Class,
) -> Concept {
    Concept {
        id,
        gloss,
        field,
        class,
        stability: None,
        tier: Some(tier),
        iconic: None,
        expressive: false,
    }
}

const fn iconic(concept: Concept, iconic: Iconic) -> Concept {
    Concept {
        iconic: Some(iconic),
        ..concept
    }
}

const fn expressive(concept: Concept) -> Concept {
    Concept {
        expressive: true,
        ..concept
    }
}

use Class::{Entity as E, Event as V, Function as F, Property as P};
use Field::*;
use Tier::*;

pub static CONCEPTS: &[Concept] = &[
    // Leipzig–Jakarta list, rank order.
    core(1, "fire", "fire", PhysicalWorld, E),
    iconic(core(2, "nose", "nose", Body, E), Iconic::Nasal),
    core(3, "go", "to go", Motion, V),
    core(4, "water", "water", PhysicalWorld, E),
    core(5, "mouth", "mouth", Body, E),
    iconic(core(6, "tongue", "tongue", Body, E), Iconic::Lateral),
    core(7, "blood", "blood", Body, E),
    core(8, "bone", "bone", Body, E),
    core(9, "2sg", "you (singular)", Function, F),
    core(10, "root", "root", Agriculture, E),
    core(11, "come", "to come", Motion, V),
    iconic(core(12, "breast", "breast", Body, E), Iconic::LabialNasal),
    core(13, "rain", "rain", PhysicalWorld, E),
    core(14, "1sg", "I, me", Function, F),
    core(15, "name", "name", Speech, E),
    expressive(core(16, "louse", "louse", Animals, E)),
    core(17, "wing", "wing", Animals, E),
    core(18, "flesh", "flesh, meat", Body, E),
    core(19, "hand", "arm, hand", Body, E),
    expressive(core(20, "fly", "fly (insect)", Animals, E)),
    core(21, "night", "night", Time, E),
    core(22, "ear", "ear", Body, E),
    core(23, "neck", "neck", Body, E),
    core(24, "far", "far", Spatial, P),
    core(25, "do", "to do, make", BasicActions, V),
    core(26, "house", "house", House, E),
    core(27, "stone", "stone, rock", PhysicalWorld, E),
    core(28, "bitter", "bitter", SensePerception, P),
    core(29, "say", "to say", Speech, V),
    core(30, "tooth", "tooth", Body, E),
    core(31, "hair", "hair", Body, E),
    core(32, "big", "big", Spatial, P),
    core(33, "one", "one", Quantity, P),
    core(34, "who", "who?", Function, F),
    core(35, "3sg", "he, she, it", Function, F),
    core(36, "hit", "to hit, beat", BasicActions, V),
    core(37, "foot", "leg, foot", Body, E),
    core(38, "horn", "horn", Animals, E),
    core(39, "this", "this", Function, F),
    core(40, "fish", "fish", Animals, E),
    core(41, "yesterday", "yesterday", Time, F),
    core(42, "drink", "to drink", FoodDrink, V),
    core(43, "black", "black", SensePerception, P),
    core(44, "navel", "navel", Body, E),
    core(45, "stand", "to stand", Motion, V),
    core(46, "bite", "to bite", FoodDrink, V),
    core(47, "back", "back", Body, E),
    core(48, "wind", "wind", PhysicalWorld, E),
    core(49, "smoke", "smoke", PhysicalWorld, E),
    core(50, "what", "what?", Function, F),
    core(51, "child", "child (kin term)", Kinship, E),
    core(52, "egg", "egg", Animals, E),
    core(53, "give", "to give", Possession, V),
    core(54, "new", "new", Time, P),
    core(55, "burn", "to burn (intransitive)", PhysicalWorld, V),
    core(56, "not", "not", Function, F),
    core(57, "good", "good", Emotions, P),
    core(58, "know", "to know", Cognition, V),
    core(59, "knee", "knee", Body, E),
    iconic(core(60, "sand", "sand", PhysicalWorld, E), Iconic::Sibilant),
    expressive(core(61, "laugh", "to laugh", Emotions, V)),
    core(62, "hear", "to hear", SensePerception, V),
    core(63, "soil", "soil", PhysicalWorld, E),
    core(64, "leaf", "leaf", Agriculture, E),
    iconic(core(65, "red", "red", SensePerception, P), Iconic::Rhotic),
    core(66, "liver", "liver", Body, E),
    core(67, "hide", "to hide", BasicActions, V),
    core(68, "skin", "skin, hide", Body, E),
    core(69, "suck", "to suck", FoodDrink, V),
    core(70, "carry", "to carry", Motion, V),
    core(71, "ant", "ant", Animals, E),
    core(72, "heavy", "heavy", SensePerception, P),
    core(73, "take", "to take", Possession, V),
    core(74, "old", "old", Time, P),
    core(75, "eat", "to eat", FoodDrink, V),
    core(76, "thigh", "thigh", Body, E),
    core(77, "thick", "thick", Spatial, P),
    core(78, "long", "long", Spatial, P),
    expressive(core(79, "blow", "to blow", PhysicalWorld, V)),
    core(80, "wood", "wood", Agriculture, E),
    core(81, "run", "to run", Motion, V),
    core(82, "fall", "to fall", Motion, V),
    core(83, "eye", "eye", Body, E),
    core(84, "ash", "ash", PhysicalWorld, E),
    core(85, "tail", "tail", Animals, E),
    core(86, "dog", "dog", Animals, E),
    expressive(core(87, "cry", "to cry, weep", Emotions, V)),
    core(88, "tie", "to tie", BasicActions, V),
    core(89, "see", "to see", SensePerception, V),
    core(90, "sweet", "sweet", SensePerception, P),
    core(91, "rope", "rope", BasicActions, E),
    core(92, "shadow", "shade, shadow", PhysicalWorld, E),
    expressive(core(93, "bird", "bird", Animals, E)),
    core(94, "salt", "salt", FoodDrink, E),
    expressive(iconic(
        core(95, "small", "small", Spatial, P),
        Iconic::CloseFront,
    )),
    core(96, "wide", "wide", Spatial, P),
    core(97, "star", "star", PhysicalWorld, E),
    core(98, "in", "in", Function, F),
    core(99, "hard", "hard", SensePerception, P),
    core(100, "grind", "to crush, grind", BasicActions, V),
    // Cultural vocabulary: more exposed to contact and to challenges.
    culture(Basic, "sun", "sun", PhysicalWorld, E),
    culture(Basic, "moon", "moon", PhysicalWorld, E),
    culture(Basic, "sky", "sky", PhysicalWorld, E),
    culture(Basic, "sea", "sea", PhysicalWorld, E),
    culture(Basic, "river", "river", PhysicalWorld, E),
    culture(Basic, "mountain", "mountain", PhysicalWorld, E),
    culture(Basic, "hill", "hill", PhysicalWorld, E),
    culture(Basic, "island", "island", PhysicalWorld, E),
    culture(Basic, "cloud", "cloud", PhysicalWorld, E),
    culture(Basic, "light", "light", PhysicalWorld, E),
    culture(Basic, "dark", "dark", PhysicalWorld, P),
    culture(Specialized, "gold", "gold", PhysicalWorld, E),
    culture(Specialized, "iron", "iron", PhysicalWorld, E),
    culture(Basic, "day", "day", Time, E),
    culture(Basic, "year", "year", Time, E),
    expressive(iconic(
        culture(Basic, "mother", "mother", Kinship, E),
        Iconic::NurseryMother,
    )),
    expressive(iconic(
        culture(Basic, "father", "father", Kinship, E),
        Iconic::NurseryFather,
    )),
    culture(Basic, "person", "person", Social, E),
    culture(Basic, "people", "people, folk", Social, E),
    culture(Specialized, "chief", "chief, king", Social, E),
    culture(Everyday, "friend", "friend", Social, E),
    culture(Everyday, "stranger", "stranger, guest", Social, E),
    culture(Everyday, "village", "village", Social, E),
    culture(Basic, "heart", "heart", Body, E),
    culture(Basic, "life", "life", Body, E),
    culture(Basic, "die", "to die", Body, V),
    culture(Basic, "tree", "tree", Agriculture, E),
    culture(Basic, "seed", "seed", Agriculture, E),
    culture(Everyday, "field", "field", Agriculture, E),
    culture(Everyday, "grain", "grain", Agriculture, E),
    culture(Everyday, "cattle", "cattle", Animals, E),
    culture(Everyday, "horse", "horse", Animals, E),
    culture(Basic, "food", "food", FoodDrink, E),
    culture(Everyday, "bread", "bread", FoodDrink, E),
    culture(Specialized, "beer", "beer", FoodDrink, E),
    culture(Everyday, "cook", "to cook", FoodDrink, V),
    culture(Everyday, "cloth", "cloth", ClothingGrooming, E),
    culture(Everyday, "weave", "to weave", ClothingGrooming, V),
    culture(Everyday, "needle", "needle", ClothingGrooming, E),
    culture(Everyday, "wall", "wall", House, E),
    culture(Everyday, "door", "door", House, E),
    culture(Everyday, "knife", "knife", BasicActions, E),
    culture(Everyday, "pot", "pot", BasicActions, E),
    culture(Basic, "path", "path, road", Motion, E),
    culture(Everyday, "boat", "boat", Motion, E),
    culture(Everyday, "sail", "sail", Motion, E),
    culture(Basic, "swim", "to swim", Motion, V),
    culture(Everyday, "war", "war", Warfare, E),
    culture(Everyday, "spear", "spear", Warfare, E),
    culture(Everyday, "bow", "bow", Warfare, E),
    culture(Everyday, "shield", "shield", Warfare, E),
    culture(Everyday, "fight", "to fight", Warfare, V),
    culture(Everyday, "hunt", "to hunt", Warfare, V),
    culture(Everyday, "fishing", "to fish", Warfare, V),
    culture(Everyday, "net", "net", Warfare, E),
    culture(Everyday, "buy", "to buy", Possession, V),
    culture(Everyday, "sell", "to sell", Possession, V),
    culture(Specialized, "market", "market", Possession, E),
    culture(Specialized, "law", "law, custom", Law, E),
    culture(Everyday, "oath", "oath", Law, E),
    culture(Everyday, "judge", "to judge", Law, V),
    culture(Specialized, "god", "god", Religion, E),
    culture(Everyday, "spirit", "spirit", Religion, E),
    culture(Specialized, "priest", "priest", Religion, E),
    culture(Specialized, "sacrifice", "to sacrifice", Religion, V),
    culture(Everyday, "pray", "to pray", Religion, V),
    culture(Basic, "voice", "voice", Speech, E),
    culture(Basic, "word", "word", Speech, E),
    culture(Basic, "sing", "to sing", Speech, V),
    culture(Basic, "love", "to love", Emotions, V),
    culture(Basic, "fear", "to fear", Emotions, V),
    culture(Basic, "think", "to think", Cognition, V),
];

pub fn by_id(id: &str) -> Option<&'static Concept> {
    CONCEPTS.iter().find(|c| c.id == id)
}

/// Pairs of concepts where one word plausibly comes to cover the other:
/// attested polysemies and semantic shifts such as sun/day, see/know,
/// this/he, sky/god, and shadow/spirit. Hand-curated after cross-linguistic
/// colexification patterns (CLICS) from memory, not checked against the
/// database. Links are symmetric.
pub static RELATED: &[(&str, &str)] = &[
    // The physical world and time.
    ("sun", "day"),
    ("light", "day"),
    ("fire", "light"),
    ("dark", "night"),
    ("black", "dark"),
    ("night", "yesterday"),
    ("river", "water"),
    ("sea", "water"),
    ("smoke", "cloud"),
    ("soil", "sand"),
    ("soil", "field"),
    ("hill", "mountain"),
    ("stone", "mountain"),
    ("wall", "stone"),
    // Religion and the unseen.
    ("sky", "god"),
    ("spirit", "wind"),
    ("shadow", "spirit"),
    ("life", "spirit"),
    ("chief", "god"),
    // People and places.
    ("person", "people"),
    ("person", "3sg"),
    ("this", "3sg"),
    ("who", "what"),
    ("house", "village"),
    ("door", "mouth"),
    // The body.
    ("tooth", "horn"),
    ("skin", "cloth"),
    ("flesh", "food"),
    ("liver", "heart"),
    ("red", "blood"),
    // Plants and animals.
    ("tree", "wood"),
    ("seed", "grain"),
    ("seed", "egg"),
    ("fly", "bird"),
    ("ant", "louse"),
    ("cattle", "horse"),
    ("bread", "food"),
    // Things.
    ("spear", "knife"),
    ("bow", "spear"),
    ("shield", "wall"),
    ("net", "rope"),
    ("sail", "cloth"),
    // Actions.
    ("see", "know"),
    ("hear", "know"),
    ("think", "know"),
    ("eat", "bite"),
    ("drink", "suck"),
    ("go", "run"),
    ("go", "come"),
    ("carry", "take"),
    ("take", "buy"),
    ("give", "sell"),
    ("hit", "fight"),
    ("fight", "hunt"),
    ("hunt", "fishing"),
    ("burn", "cook"),
    ("fall", "die"),
    ("cry", "sing"),
    ("say", "pray"),
    ("tie", "weave"),
    // Properties and speech.
    ("big", "long"),
    ("far", "long"),
    ("thick", "wide"),
    ("heavy", "hard"),
    ("good", "sweet"),
    ("word", "name"),
    ("word", "voice"),
    ("law", "word"),
];

/// How a derived word's meaning relates to its base's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum Relation {
    /// A group of the base: person > people.
    Collective,
    /// One who does the base: pray > priest.
    Agent,
    /// Where the base happens: buy > market.
    Place,
    /// What the base is done with: hunt > spear.
    Instrument,
    /// What the base produces: weave > cloth.
    Result,
    /// To act with or on the base: fish > to fish.
    Action,
    /// The base as a state or affair: fight > war.
    Abstract,
}

impl Relation {
    pub const ALL: [Relation; 7] = [
        Relation::Collective,
        Relation::Agent,
        Relation::Place,
        Relation::Instrument,
        Relation::Result,
        Relation::Action,
        Relation::Abstract,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Relation::Collective => "collective",
            Relation::Agent => "agent",
            Relation::Place => "place",
            Relation::Instrument => "instrument",
            Relation::Result => "result",
            Relation::Action => "action",
            Relation::Abstract => "abstract",
        }
    }
}

/// Word families: where a language may build the second concept's word from
/// the first's. Listed so that every base comes before what derives from
/// it. Hand-picked from common derivational patterns; each language uses
/// each link only some of the time.
pub static FAMILIES: &[(&str, &str, Relation)] = &[
    ("person", "people", Relation::Collective),
    ("fish", "fishing", Relation::Action),
    ("fishing", "net", Relation::Instrument),
    ("fire", "cook", Relation::Action),
    ("eat", "food", Relation::Result),
    ("grain", "bread", Relation::Result),
    ("say", "word", Relation::Result),
    ("fight", "war", Relation::Abstract),
    ("fight", "shield", Relation::Instrument),
    ("hunt", "spear", Relation::Instrument),
    ("weave", "cloth", Relation::Result),
    ("buy", "market", Relation::Place),
    ("judge", "law", Relation::Result),
    ("pray", "priest", Relation::Agent),
    ("seed", "field", Relation::Place),
];

/// Concepts a word for `concept` could plausibly extend to, or come from.
pub fn related(concept: &Concept) -> impl Iterator<Item = &'static Concept> + '_ {
    RELATED.iter().filter_map(move |&(a, b)| {
        let other = if a == concept.id {
            b
        } else if b == concept.id {
            a
        } else {
            return None;
        };
        by_id(other)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique() {
        let ids: HashSet<_> = CONCEPTS.iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), CONCEPTS.len());
    }

    #[test]
    fn related_pairs_name_known_concepts() {
        for (a, b) in RELATED {
            assert!(by_id(a).is_some() && by_id(b).is_some(), "{a}/{b}");
            assert_ne!(a, b);
        }
        let sun: Vec<_> = related(by_id("sun").unwrap()).map(|c| c.id).collect();
        assert_eq!(sun, ["day"]);
    }

    #[test]
    fn families_name_known_concepts_with_bases_first() {
        let mut derived: Vec<&str> = Vec::new();
        for (base, word, _) in FAMILIES {
            assert!(
                by_id(base).is_some() && by_id(word).is_some(),
                "{base}/{word}"
            );
            assert!(!derived.contains(word), "{word} is derived twice");
            derived.push(word);
        }
        for (i, (base, _, _)) in FAMILIES.iter().enumerate() {
            assert!(
                !FAMILIES[i..].iter().any(|(_, w, _)| w == base),
                "{base} must be derived before it is used as a base"
            );
        }
    }

    #[test]
    fn core_list_is_ranks_one_to_one_hundred() {
        let ranks: Vec<u8> = CONCEPTS.iter().filter_map(|c| c.stability).collect();
        assert_eq!(ranks, (1..=100).collect::<Vec<_>>());
    }
}

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
    pub iconic: Option<Iconic>,
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
        iconic: None,
    }
}

const fn culture(id: &'static str, gloss: &'static str, field: Field, class: Class) -> Concept {
    Concept {
        id,
        gloss,
        field,
        class,
        stability: None,
        iconic: None,
    }
}

const fn iconic(concept: Concept, iconic: Iconic) -> Concept {
    Concept {
        iconic: Some(iconic),
        ..concept
    }
}

use Class::{Entity as E, Event as V, Function as F, Property as P};
use Field::*;

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
    core(16, "louse", "louse", Animals, E),
    core(17, "wing", "wing", Animals, E),
    core(18, "flesh", "flesh, meat", Body, E),
    core(19, "hand", "arm, hand", Body, E),
    core(20, "fly", "fly (insect)", Animals, E),
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
    core(61, "laugh", "to laugh", Emotions, V),
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
    core(79, "blow", "to blow", PhysicalWorld, V),
    core(80, "wood", "wood", Agriculture, E),
    core(81, "run", "to run", Motion, V),
    core(82, "fall", "to fall", Motion, V),
    core(83, "eye", "eye", Body, E),
    core(84, "ash", "ash", PhysicalWorld, E),
    core(85, "tail", "tail", Animals, E),
    core(86, "dog", "dog", Animals, E),
    core(87, "cry", "to cry, weep", Emotions, V),
    core(88, "tie", "to tie", BasicActions, V),
    core(89, "see", "to see", SensePerception, V),
    core(90, "sweet", "sweet", SensePerception, P),
    core(91, "rope", "rope", BasicActions, E),
    core(92, "shadow", "shade, shadow", PhysicalWorld, E),
    core(93, "bird", "bird", Animals, E),
    core(94, "salt", "salt", FoodDrink, E),
    iconic(core(95, "small", "small", Spatial, P), Iconic::CloseFront),
    core(96, "wide", "wide", Spatial, P),
    core(97, "star", "star", PhysicalWorld, E),
    core(98, "in", "in", Function, F),
    core(99, "hard", "hard", SensePerception, P),
    core(100, "grind", "to crush, grind", BasicActions, V),
    // Cultural vocabulary: more exposed to contact and to challenges.
    culture("sun", "sun", PhysicalWorld, E),
    culture("moon", "moon", PhysicalWorld, E),
    culture("sky", "sky", PhysicalWorld, E),
    culture("sea", "sea", PhysicalWorld, E),
    culture("river", "river", PhysicalWorld, E),
    culture("mountain", "mountain", PhysicalWorld, E),
    culture("hill", "hill", PhysicalWorld, E),
    culture("island", "island", PhysicalWorld, E),
    culture("cloud", "cloud", PhysicalWorld, E),
    culture("light", "light", PhysicalWorld, E),
    culture("dark", "dark", PhysicalWorld, P),
    culture("gold", "gold", PhysicalWorld, E),
    culture("iron", "iron", PhysicalWorld, E),
    culture("day", "day", Time, E),
    culture("year", "year", Time, E),
    culture("mother", "mother", Kinship, E),
    culture("father", "father", Kinship, E),
    culture("person", "person", Social, E),
    culture("people", "people, folk", Social, E),
    culture("chief", "chief, king", Social, E),
    culture("friend", "friend", Social, E),
    culture("stranger", "stranger, guest", Social, E),
    culture("village", "village", Social, E),
    culture("heart", "heart", Body, E),
    culture("life", "life", Body, E),
    culture("die", "to die", Body, V),
    culture("tree", "tree", Agriculture, E),
    culture("seed", "seed", Agriculture, E),
    culture("field", "field", Agriculture, E),
    culture("grain", "grain", Agriculture, E),
    culture("cattle", "cattle", Animals, E),
    culture("horse", "horse", Animals, E),
    culture("food", "food", FoodDrink, E),
    culture("bread", "bread", FoodDrink, E),
    culture("beer", "beer", FoodDrink, E),
    culture("cook", "to cook", FoodDrink, V),
    culture("cloth", "cloth", ClothingGrooming, E),
    culture("weave", "to weave", ClothingGrooming, V),
    culture("needle", "needle", ClothingGrooming, E),
    culture("wall", "wall", House, E),
    culture("door", "door", House, E),
    culture("knife", "knife", BasicActions, E),
    culture("pot", "pot", BasicActions, E),
    culture("path", "path, road", Motion, E),
    culture("boat", "boat", Motion, E),
    culture("sail", "sail", Motion, E),
    culture("swim", "to swim", Motion, V),
    culture("war", "war", Warfare, E),
    culture("spear", "spear", Warfare, E),
    culture("bow", "bow", Warfare, E),
    culture("shield", "shield", Warfare, E),
    culture("fight", "to fight", Warfare, V),
    culture("hunt", "to hunt", Warfare, V),
    culture("fishing", "to fish", Warfare, V),
    culture("net", "net", Warfare, E),
    culture("buy", "to buy", Possession, V),
    culture("sell", "to sell", Possession, V),
    culture("market", "market", Possession, E),
    culture("law", "law, custom", Law, E),
    culture("oath", "oath", Law, E),
    culture("judge", "to judge", Law, V),
    culture("god", "god", Religion, E),
    culture("spirit", "spirit", Religion, E),
    culture("priest", "priest", Religion, E),
    culture("sacrifice", "to sacrifice", Religion, V),
    culture("pray", "to pray", Religion, V),
    culture("voice", "voice", Speech, E),
    culture("word", "word", Speech, E),
    culture("sing", "to sing", Speech, V),
    culture("love", "to love", Emotions, V),
    culture("fear", "to fear", Emotions, V),
    culture("think", "to think", Cognition, V),
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
    fn core_list_is_ranks_one_to_one_hundred() {
        let ranks: Vec<u8> = CONCEPTS.iter().filter_map(|c| c.stability).collect();
        assert_eq!(ranks, (1..=100).collect::<Vec<_>>());
    }
}

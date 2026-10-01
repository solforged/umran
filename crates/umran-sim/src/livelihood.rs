//! How a people gets its food, which decides how many a land feeds them,
//! how fast they grow, how readily they move, and how large they can grow
//! before they come apart. Farming feeds many times more people on good
//! land than foraging, which is why farmers spread so far: the Bantu
//! languages across half of Africa, the Austronesian across the Pacific.

use crate::geography::Terrain;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Livelihood {
    /// Hunting, fishing, and gathering: few people, small groups.
    Foraging,
    /// Herds of animals: middling numbers, always on the move.
    Herding,
    /// Fields and crops: many people, rooted to their land.
    #[default]
    Farming,
}

impl Livelihood {
    pub const ALL: [Livelihood; 3] = [
        Livelihood::Foraging,
        Livelihood::Herding,
        Livelihood::Farming,
    ];

    /// How many a land of `terrain` feeds a people living this way, as a
    /// share of what open plain feeds farmers.
    pub fn feeds(self, terrain: Terrain) -> f32 {
        use Terrain::*;
        match (self, terrain) {
            (_, Sea) => 0.0,
            (Livelihood::Farming, Plains) => 1.0,
            (Livelihood::Farming, Forest | Hills) => 0.4,
            (Livelihood::Farming, Steppe) => 0.12,
            (Livelihood::Farming, Mountains) => 0.08,
            (Livelihood::Farming, Desert) => 0.03,
            (Livelihood::Herding, Steppe) => 0.3,
            (Livelihood::Herding, Plains) => 0.25,
            (Livelihood::Herding, Hills) => 0.2,
            (Livelihood::Herding, Forest | Mountains | Desert) => 0.08,
            (Livelihood::Foraging, Forest) => 0.04,
            (Livelihood::Foraging, Plains) => 0.03,
            (Livelihood::Foraging, Hills | Steppe) => 0.025,
            (Livelihood::Foraging, Mountains) => 0.012,
            (Livelihood::Foraging, Desert) => 0.006,
        }
    }

    /// Growth per generation when there is room, as a share of farmers'.
    pub fn growth(self) -> f32 {
        match self {
            Livelihood::Farming => 1.0,
            Livelihood::Herding => 0.7,
            Livelihood::Foraging => 0.4,
        }
    }

    /// How readily they take to the road, beyond what their land makes
    /// them: herders follow their herds, farmers stay by their fields.
    pub fn mobility(self) -> f32 {
        match self {
            Livelihood::Herding => 2.0,
            Livelihood::Foraging => 1.5,
            Livelihood::Farming => 1.0,
        }
    }

    /// How large a people living this way grows before it starts to come
    /// apart, as a share of what holds farmers together. Bands of
    /// foragers seldom hold together beyond a few thousand; herders can
    /// gather in confederacies, farmers in chiefdoms of tens of thousands.
    pub fn cohesion(self) -> f32 {
        match self {
            Livelihood::Farming => 1.0,
            Livelihood::Herding => 0.5,
            Livelihood::Foraging => 0.06,
        }
    }

    /// How a people first settling `terrain` lives: farmers on open
    /// plain, herders on steppe and desert, foragers in forest, hills,
    /// and mountains.
    pub fn of_land(terrain: Terrain) -> Self {
        match terrain {
            Terrain::Plains => Livelihood::Farming,
            Terrain::Steppe | Terrain::Desert => Livelihood::Herding,
            _ => Livelihood::Foraging,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Livelihood::Foraging => "foragers",
            Livelihood::Herding => "herders",
            Livelihood::Farming => "farmers",
        }
    }
}

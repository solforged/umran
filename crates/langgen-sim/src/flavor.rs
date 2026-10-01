use crate::inventory::{UNLISTED_PRIMARY, UNLISTED_SECONDARY};
use crate::phoneme::{Backness, Height, Manner, Place};
use crate::profile::SoundProfile;
use serde::{Deserialize, Serialize};

/// A phonetic vibe layered onto a profile, such as "slightly more plausible
/// for fish mouths". The engine never reads the brief: a person or Claude
/// writes the adjustments from it, and the simulation sees only numbers.
///
/// Because the same preference score drives inventory sampling and the odds
/// of each sound law, a flavor shapes both the starting sounds and the
/// direction they drift.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Flavor {
    pub id: String,
    pub name: String,
    /// The plain-language brief the adjustments were written from.
    pub brief: String,
    pub manner: Vec<(Manner, f32)>,
    pub place: Vec<(Place, f32)>,
    pub voiced: f32,
    pub height: Vec<(Height, f32)>,
    pub backness: Vec<(Backness, f32)>,
    pub rounded: f32,
    /// Adjustments to single segments by IPA.
    pub segments: Vec<(String, f32)>,
    pub require: Vec<String>,
    pub forbid: Vec<String>,
    pub consonant_count: Option<(u8, u8)>,
    pub vowel_count: Option<(u8, u8)>,
    pub final_coda: Option<f32>,
    pub disyllabic_roots: Option<f32>,
    pub preferred_onsets: Vec<String>,
    pub preferred_codas: Vec<String>,
}

impl SoundProfile {
    /// This profile with `flavor`'s adjustments added. Flavors stack.
    pub fn flavored(&self, flavor: &Flavor) -> SoundProfile {
        let mut out = self.clone();
        let inv = &mut out.inventory;
        nudge(&mut inv.manner, &flavor.manner, UNLISTED_PRIMARY);
        nudge(&mut inv.place, &flavor.place, UNLISTED_SECONDARY);
        nudge(&mut inv.height, &flavor.height, UNLISTED_PRIMARY);
        nudge(&mut inv.backness, &flavor.backness, UNLISTED_SECONDARY);
        nudge(&mut inv.extra, &flavor.segments, 0.0);
        inv.voiced += flavor.voiced;
        inv.rounded += flavor.rounded;
        inv.required.retain(|s| !flavor.forbid.contains(s));
        inv.forbidden.retain(|s| !flavor.require.contains(s));
        union(&mut inv.required, &flavor.require);
        union(&mut inv.forbidden, &flavor.forbid);
        inv.consonant_count = flavor.consonant_count.unwrap_or(inv.consonant_count);
        inv.vowel_count = flavor.vowel_count.unwrap_or(inv.vowel_count);

        let tac = &mut out.phonotactics;
        tac.final_coda = flavor.final_coda.unwrap_or(tac.final_coda);
        tac.disyllabic_roots = flavor.disyllabic_roots.unwrap_or(tac.disyllabic_roots);
        union(&mut tac.preferred_onsets, &flavor.preferred_onsets);
        union(&mut tac.preferred_codas, &flavor.preferred_codas);

        out.id = format!("{}+{}", out.id, flavor.id);
        out.name = format!("{}, {}", out.name, flavor.name);
        out
    }
}

/// Adds each delta to the matching entry, or lists it at `unlisted + delta`
/// so an adjustment never jumps a feature past its implicit penalty.
fn nudge<K: PartialEq + Clone>(dest: &mut Vec<(K, f32)>, deltas: &[(K, f32)], unlisted: f32) {
    for (key, delta) in deltas {
        match dest.iter_mut().find(|(k, _)| k == key) {
            Some((_, w)) => *w += delta,
            None => dest.push((key.clone(), unlisted + delta)),
        }
    }
}

fn union(dest: &mut Vec<String>, add: &[String]) {
    for s in add {
        if !dest.contains(s) {
            dest.push(s.clone());
        }
    }
}

impl Flavor {
    /// Example flavors written by Claude from Sol's briefs. Adjust freely.
    pub fn examples() -> Vec<Flavor> {
        vec![fish_mouthed(), pie_like()]
    }

    pub fn by_id(id: &str) -> Option<Flavor> {
        Self::examples().into_iter().find(|f| f.id == id)
    }
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn weighted(list: &[(&str, f32)]) -> Vec<(String, f32)> {
    list.iter().map(|(s, w)| (s.to_string(), *w)).collect()
}

/// Lips do the work; no tongue-tip hissing, rounded back vowels.
fn fish_mouthed() -> Flavor {
    use Backness::*;
    use Manner::*;
    use Place::*;
    Flavor {
        id: "fish-mouthed".into(),
        name: "Fish-mouthed".into(),
        brief: "Make it sound slightly more plausible for fish mouths.".into(),
        manner: vec![(Fricative, -1.5), (Affricate, -2.0), (Nasal, 0.4)],
        place: vec![
            (Bilabial, 1.5),
            (Dental, -2.0),
            (Labiodental, -1.5),
            (Postalveolar, -1.5),
            (Palatal, -1.0),
        ],
        backness: vec![(Back, 1.0), (Front, -0.5)],
        rounded: 1.0,
        segments: weighted(&[("b", 1.0), ("p", 1.0), ("m", 0.5), ("o", 0.8), ("u", 0.8)]),
        ..Flavor::default()
    }
}

/// Stops in voiceless and voiced series, a lone sibilant /s/, laryngeal-like
/// /h x/, an e/o-heavy vowel system, and closed CeC roots.
fn pie_like() -> Flavor {
    use Manner::*;
    use Place::*;
    Flavor {
        id: "pie-like".into(),
        name: "PIE-like".into(),
        brief: "Give it more of a Proto-Indo-European vibe.".into(),
        manner: vec![(Stop, 1.2), (Fricative, -1.0)],
        place: vec![(Velar, 1.0)],
        segments: weighted(&[
            ("s", 2.5),
            ("h", 2.0),
            ("x", 1.5),
            ("e", 2.0),
            ("o", 1.5),
            ("a", -1.5),
        ]),
        require: strings(&["s", "e", "o", "k", "g", "w", "r", "n", "m"]),
        forbid: strings(&["f", "v", "θ", "ð", "ʃ", "ʒ", "z", "ts", "dz", "tʃ", "dʒ"]),
        final_coda: Some(0.8),
        disyllabic_roots: Some(0.0),
        preferred_onsets: strings(&["kw", "gw", "st", "sk", "tr", "pr"]),
        preferred_codas: strings(&["r", "n", "m", "s", "t", "k"]),
        ..Flavor::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::Inventory;
    use crate::phoneme::CATALOG;
    use crate::rng::stream;

    fn rate(profile: &SoundProfile, ipa: &str) -> f64 {
        let id = CATALOG.id_by_ipa(ipa).unwrap();
        let hits = (0..300)
            .filter(|&seed| {
                Inventory::sample(&profile.inventory, &mut stream(seed, &[])).contains(id)
            })
            .count();
        hits as f64 / 300.0
    }

    #[test]
    fn flavors_shift_inventories_without_replacing_them() {
        let neutral = SoundProfile::by_id("neutral").unwrap();
        let fishy = neutral.flavored(&Flavor::by_id("fish-mouthed").unwrap());
        assert!(rate(&fishy, "b") > rate(&neutral, "b"));
        assert!(rate(&fishy, "f") < rate(&neutral, "f"));
        assert_eq!(fishy.id, "neutral+fish-mouthed");

        let pie = neutral.flavored(&Flavor::by_id("pie-like").unwrap());
        assert_eq!(rate(&pie, "s"), 1.0);
        assert_eq!(rate(&pie, "f"), 0.0);
        assert_eq!(pie.phonotactics.disyllabic_roots, 0.0);
    }

    #[test]
    fn unlisted_features_start_from_their_penalty() {
        let mut list = vec![(Manner::Stop, 1.0)];
        nudge(
            &mut list,
            &[(Manner::Stop, 0.5), (Manner::Nasal, 0.4)],
            -0.9,
        );
        assert_eq!(list[0], (Manner::Stop, 1.5));
        assert_eq!(list[1].0, Manner::Nasal);
        assert!((list[1].1 - -0.5).abs() < 1e-6);
    }
}

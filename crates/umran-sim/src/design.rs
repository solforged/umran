//! A language as a person designs it before founding: the exact sounds
//! (each used or favoured), a few knobs for word shape and word building,
//! grammar choices, optional stress, and spelling. Presets and drawing by world frequency
//! produce designs; a design then resolves to the `SoundProfile` the engine runs on.

use crate::grammar::{GrammarDesign, GrammarPrior};
use crate::inventory::Inventory;
use crate::phoneme::{CATALOG, PhonemeId};
use crate::profile::{MorphologyKind, MorphologyPrior, PhonotacticPrior, SoundProfile, Spelling};
use crate::prosody::StressRule;
use crate::rng::{key, stream};
use serde::{Deserialize, Serialize};

/// Preference added to a favoured sound: used more in words, and sound
/// changes lean toward it.
const FAVOURED: f32 = 2.0;
/// Preference for sounds the design leaves out: sound changes and loans
/// rarely bring them in, but can.
const ABSENT: f32 = -2.5;
/// Extra weight in a preset that marks a sound as one of its signatures.
const SIGNATURE: f32 = 1.5;
const MIN_CONSONANTS: usize = 3;
const MIN_VOWELS: usize = 2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sound {
    pub ipa: String,
    #[serde(default)]
    pub favoured: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageDesign {
    pub sounds: Vec<Sound>,
    /// 0–1: how often roots have more than one syllable.
    pub word_length: f32,
    /// 0–1: how often words end in a consonant.
    pub final_consonants: f32,
    /// Whether consonants may meet inside a word (kasta) or every inner
    /// syllable stays open (kasata).
    pub inner_clusters: bool,
    /// 0–1: tolerance for a repeated consonant in a root (kika).
    pub repetition: f32,
    /// 0–1: how often a root's vowels are long.
    pub long_vowels: f32,
    /// 0–1: how often an internal consonant is long; off by default.
    #[serde(default)]
    pub geminates: f32,
    #[serde(default)]
    pub stress: Option<StressRule>,
    pub building: MorphologyKind,
    /// 0–1: share of affixes that are suffixes rather than prefixes.
    pub suffixing: f32,
    /// 0–1: how often related meanings are built from one another.
    pub derivation: f32,
    /// Resolved, editable founding choices for plural and past marking.
    #[serde(default)]
    pub grammar: GrammarDesign,
    pub spelling: Spelling,
}

impl LanguageDesign {
    /// A language with the given numbers of consonants and vowels, chosen
    /// by how common sounds are worldwide.
    pub fn by_frequency(seed: u64, consonants: u8, vowels: u8) -> Self {
        let mut profile = SoundProfile::base();
        profile.inventory.consonant_count = (consonants, consonants);
        profile.inventory.vowel_count = (vowels, vowels);
        Self::from_profile(&profile, seed)
    }

    /// A preset resolved into an editable design, by flavor id ("indic",
    /// "semitic", ...).
    pub fn preset(id: &str, seed: u64) -> Option<Self> {
        SoundProfile::by_id(id).map(|p| Self::from_profile(&p, seed))
    }

    /// Samples a profile's inventory, resolves grammar, and copies
    /// its knobs. Sounds the profile strongly boosts or requires beyond the
    /// base are marked favoured.
    pub fn from_profile(profile: &SoundProfile, seed: u64) -> Self {
        let inventory = Inventory::sample(&profile.inventory, &mut stream(seed, &[key("design")]));
        let base = SoundProfile::base();
        let favoured = |ipa: &str| {
            let extra = profile
                .inventory
                .extra
                .iter()
                .find(|(s, _)| s == ipa)
                .map_or(0.0, |(_, w)| *w);
            let signature = profile.inventory.required.iter().any(|r| r == ipa)
                && !base.inventory.required.iter().any(|r| r == ipa);
            extra >= SIGNATURE || signature
        };
        let sounds = inventory
            .consonants
            .iter()
            .chain(&inventory.vowels)
            .map(|id| {
                let ipa = CATALOG.get(*id).ipa();
                Sound {
                    ipa: ipa.into(),
                    favoured: favoured(ipa),
                }
            })
            .collect();
        let tac = &profile.phonotactics;
        Self {
            sounds,
            word_length: tac.disyllabic_roots,
            final_consonants: tac.final_coda,
            inner_clusters: !tac.open_medial,
            repetition: tac.identical_consonants,
            long_vowels: tac.long_vowels,
            geminates: tac.geminates,
            stress: profile.stress,
            building: profile.morphology.kind,
            suffixing: profile.morphology.suffixing,
            derivation: profile.morphology.derivation,
            grammar: profile.grammar.draw(seed, &profile.morphology),
            spelling: profile.spelling.clone(),
        }
    }

    /// Why this design cannot found a language, if it cannot.
    pub fn validate(&self) -> Result<(), String> {
        let mut seen: Vec<PhonemeId> = Vec::new();
        for sound in &self.sounds {
            let id = CATALOG
                .id_by_ipa(&sound.ipa)
                .ok_or_else(|| format!("unknown sound '{}'", sound.ipa))?;
            if seen.contains(&id) {
                return Err(format!("'{}' is listed twice", sound.ipa));
            }
            seen.push(id);
        }
        let vowels = seen
            .iter()
            .filter(|id| CATALOG.get(**id).is_vowel())
            .count();
        if seen.len() - vowels < MIN_CONSONANTS {
            return Err(format!("choose at least {MIN_CONSONANTS} consonants"));
        }
        if vowels < MIN_VOWELS {
            return Err(format!("choose at least {MIN_VOWELS} vowels"));
        }
        let knobs = [
            self.word_length,
            self.final_consonants,
            self.repetition,
            self.long_vowels,
            self.geminates,
            self.suffixing,
            self.derivation,
        ];
        if knobs.iter().any(|k| !(0.0..=1.0).contains(k)) {
            return Err("knobs run from 0 to 1".into());
        }
        Ok(())
    }

    /// The profile the engine runs on: exactly these sounds, favoured ones
    /// preferred, absent ones discouraged, and the knobs as set.
    pub fn profile(&self) -> SoundProfile {
        let mut profile = SoundProfile::base();
        profile.id = "designed".into();
        profile.name = "Designed".into();
        profile.description = String::new();
        let inv = &mut profile.inventory;
        inv.exact = true;
        inv.required = self.sounds.iter().map(|s| s.ipa.clone()).collect();
        inv.forbidden = Vec::new();
        inv.extra = CATALOG
            .segments
            .iter()
            .map(|seg| {
                let ipa = seg.ipa();
                let weight = match self.sounds.iter().find(|s| s.ipa == ipa) {
                    Some(s) if s.favoured => FAVOURED,
                    Some(_) => 0.0,
                    None => ABSENT,
                };
                (ipa.to_string(), weight)
            })
            .collect();
        profile.phonotactics = PhonotacticPrior {
            max_onset: 2,
            max_coda: 1,
            final_coda: self.final_consonants,
            open_medial: !self.inner_clusters,
            preferred_onsets: Vec::new(),
            preferred_codas: Vec::new(),
            disyllabic_roots: self.word_length,
            identical_consonants: self.repetition,
            long_vowels: self.long_vowels,
            geminates: self.geminates,
        };
        profile.morphology = MorphologyPrior {
            kind: self.building,
            suffixing: self.suffixing,
            derivation: self.derivation,
        };
        profile.spelling = self.spelling.clone();
        profile.stress = self.stress;
        profile.grammar = GrammarPrior::fixed(self.grammar);
        profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concepts;
    use crate::grammar::{Category, GrammarChoice, MarkerKind, Side};
    use crate::variety::Variety;

    fn assert_founding_grammar(variety: &Variety, expected: GrammarDesign) {
        for category in Category::ALL {
            let marker = variety
                .grammar
                .markers
                .iter()
                .find(|marker| marker.category == category)
                .expect("each founding category has a marker");
            let choice = match (marker.kind, marker.side) {
                (MarkerKind::Bound, Side::Suffix) => GrammarChoice::Suffix,
                (MarkerKind::Bound, Side::Prefix) => GrammarChoice::Prefix,
                (MarkerKind::Particle, _) => GrammarChoice::Particle,
                (MarkerKind::None, _) => GrammarChoice::None,
            };
            assert_eq!(choice, expected.choice(category), "{}", category.id());
        }
    }

    #[test]
    fn designs_found_exactly_their_sounds() {
        let design = LanguageDesign::by_frequency(7, 12, 5);
        assert!(design.validate().is_ok());
        let variety = Variety::found(
            7,
            &design.profile(),
            crate::Livelihood::Farming,
            Default::default(),
        );
        let (c, v) = variety.inventory();
        let mut used: Vec<&str> = c
            .iter()
            .chain(&v)
            .map(|id| CATALOG.get(*id).ipa())
            .collect();
        let mut chosen: Vec<&str> = design.sounds.iter().map(|s| s.ipa.as_str()).collect();
        used.sort();
        chosen.sort();
        // Every word uses only chosen sounds; rare chosen sounds may be unused.
        assert!(
            used.iter().all(|u| chosen.contains(u)),
            "{used:?} vs {chosen:?}"
        );
        assert!(used.len() + 3 >= chosen.len());
    }

    #[test]
    fn presets_resolve_with_their_signatures_favoured() {
        let indic = LanguageDesign::preset("indic", 3).unwrap();
        let favoured: Vec<&str> = indic
            .sounds
            .iter()
            .filter(|s| s.favoured)
            .map(|s| s.ipa.as_str())
            .collect();
        assert!(
            favoured.contains(&"bʱ") && favoured.contains(&"ʈ"),
            "{favoured:?}"
        );
        assert!(
            LanguageDesign::preset("semitic", 3).unwrap().building == MorphologyKind::RootPattern
        );
        assert!(LanguageDesign::preset("no-such", 3).is_none());
    }

    #[test]
    fn bad_designs_are_explained() {
        let mut design = LanguageDesign::by_frequency(1, 10, 5);
        design.sounds.push(Sound {
            ipa: "nope".into(),
            favoured: false,
        });
        assert!(design.validate().unwrap_err().contains("nope"));
        let mut design = LanguageDesign::by_frequency(1, 10, 5);
        design
            .sounds
            .retain(|s| !CATALOG.get(CATALOG.id_by_ipa(&s.ipa).unwrap()).is_vowel());
        assert!(design.validate().unwrap_err().contains("vowels"));
        let mut design = LanguageDesign::by_frequency(1, 10, 5);
        design.word_length = 1.5;
        assert!(design.validate().is_err());
    }

    #[test]
    fn designs_round_trip_as_json() {
        let design = LanguageDesign::preset("nahuatl", 9).unwrap();
        let json = serde_json::to_string(&design).unwrap();
        assert_eq!(
            serde_json::from_str::<LanguageDesign>(&json).unwrap(),
            design
        );
    }

    #[test]
    fn old_design_without_grammar_founds_with_suffixes_at_every_seed() {
        let mut design = LanguageDesign::preset("nahuatl", 9).unwrap();
        design.grammar = GrammarDesign {
            plural: GrammarChoice::Particle,
            past: GrammarChoice::Prefix,
        };
        let mut json = serde_json::to_value(&design).unwrap();
        json.as_object_mut().unwrap().remove("grammar");
        let loaded: LanguageDesign = serde_json::from_value(json).unwrap();
        let suffixes = GrammarDesign {
            plural: GrammarChoice::Suffix,
            past: GrammarChoice::Suffix,
        };
        assert_eq!(
            loaded,
            LanguageDesign {
                grammar: suffixes,
                ..design
            }
        );
        for seed in [0, 7, u64::MAX] {
            let variety = Variety::found(
                seed,
                &loaded.profile(),
                crate::Livelihood::Farming,
                crate::Ethos::default(),
            );
            assert_founding_grammar(&variety, suffixes);
        }
    }

    #[test]
    fn preset_and_resolved_design_found_with_the_same_grammar() {
        for profile in SoundProfile::presets() {
            for seed in 0..4 {
                let design = LanguageDesign::from_profile(&profile, seed);
                let preset = Variety::found(
                    seed,
                    &profile,
                    crate::Livelihood::Farming,
                    crate::Ethos::default(),
                );
                let resolved = Variety::found(
                    seed,
                    &design.profile(),
                    crate::Livelihood::Farming,
                    crate::Ethos::default(),
                );
                assert_founding_grammar(&preset, design.grammar);
                assert_founding_grammar(&resolved, design.grammar);
            }
        }
    }

    #[test]
    fn saved_grammar_edits_control_founding_without_inflecting_mass_or_collective_senses() {
        let mut design = LanguageDesign::preset("germanic", 7).unwrap();
        let choices = [
            GrammarChoice::Suffix,
            GrammarChoice::Prefix,
            GrammarChoice::Particle,
            GrammarChoice::None,
        ];
        for (i, plural) in choices.into_iter().enumerate() {
            design.grammar = GrammarDesign {
                plural,
                past: choices[(i + 1) % choices.len()],
            };
            let json = serde_json::to_string(&design).unwrap();
            let loaded: LanguageDesign = serde_json::from_str(&json).unwrap();
            let variety = Variety::found(
                23,
                &loaded.profile(),
                crate::Livelihood::Farming,
                crate::Ethos::default(),
            );
            assert_founding_grammar(&variety, design.grammar);
            for id in ["water", "people", "blood", "sand", "cattle", "food"] {
                let word = variety
                    .lexicon
                    .word_for(concepts::by_id(id).unwrap())
                    .expect("ordinary founding meaning");
                assert!(word.paradigms.is_empty(), "{id} must not inflect");
            }
            for (id, category) in [("fish", Category::Plural), ("go", Category::Past)] {
                let word = variety
                    .lexicon
                    .word_for(concepts::by_id(id).unwrap())
                    .unwrap();
                assert!(
                    word.paradigms.iter().any(|paradigm| {
                        paradigm.category == category
                            && paradigm
                                .realizations
                                .iter()
                                .any(|form| form.retired.is_none())
                    }),
                    "{id} must carry {}",
                    category.id()
                );
            }
        }
    }

    #[test]
    fn explicit_design_stress_controls_founding_without_changing_segments() {
        let mut design = LanguageDesign::preset("germanic", 7).unwrap();
        let initial = {
            design.stress = Some(StressRule::Initial);
            Variety::found(
                7,
                &design.profile(),
                crate::Livelihood::Farming,
                crate::Ethos::default(),
            )
        };
        design.stress = Some(StressRule::Final);
        let final_stress = Variety::found(
            7,
            &design.profile(),
            crate::Livelihood::Farming,
            crate::Ethos::default(),
        );
        assert_eq!(initial.stress(), StressRule::Initial);
        assert_eq!(final_stress.stress(), StressRule::Final);
        let mut moved = false;
        for (a, b) in initial.lexicon.living().zip(final_stress.lexicon.living()) {
            assert_eq!(a.first_sense.id, b.first_sense.id);
            assert_eq!(a.form.segs, b.form.segs);
            assert_eq!(a.form.stress, None);
            assert_eq!(b.form.stress, None);
            moved |= a.form.stressed_syllable(initial.stress())
                != b.form.stressed_syllable(final_stress.stress());
        }
        assert!(moved);
    }
}

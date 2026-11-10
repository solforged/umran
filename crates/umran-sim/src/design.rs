//! A language as a person designs it before founding: the exact sounds
//! (each used or favoured), a few knobs for word shape and word building,
//! grammar choices, optional stress, and spelling. Presets and drawing by world frequency
//! produce designs; a design then resolves to the `SoundProfile` the engine runs on.

use crate::concepts::{Concept, ConceptId, ConceptTable};
use crate::form::Form;
use crate::grammar::{Category, GrammarChoice, GrammarDesign, GrammarPrior, Side};
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

/// A root the author gives the language at founding: a catalog meaning
/// and its form in the catalog's own IPA, with `ː` for length. The engine
/// keeps the form verbatim as the year-0 word for that meaning.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub concept: String,
    pub ipa: String,
}

/// An affix the author gives the language at founding: the category it
/// marks, the side of the word it sits on, and its form in the catalog's
/// IPA without the hyphen. Giving one settles that the category is marked
/// by an affix on that side. Unlike a root it may be a lone consonant, as
/// the plural *-s*.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Affix {
    pub category: Category,
    pub side: Side,
    pub ipa: String,
}

/// Why an affix is refused. Like a root, it is taken whole or not at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AffixError {
    /// The catalog cannot read the form: the fragment it stopped at, or
    /// `None` for an empty form.
    BadIpa(Option<String>),
    /// A sound the design leaves out, by IPA.
    OutsideInventory(String),
    /// Another affix already marks this category.
    Duplicate,
}

impl Affix {
    /// The form as the author would write it: `-s`, `na-`.
    pub fn written(&self) -> String {
        match self.side {
            Side::Suffix => format!("-{}", self.ipa),
            Side::Prefix => format!("{}-", self.ipa),
        }
    }

    /// Parse an authored affix without repairing it.
    pub fn form(&self) -> Result<Form, AffixError> {
        let raw = self.ipa.trim();
        if raw.is_empty() {
            return Err(AffixError::BadIpa(None));
        }
        if let Some(tone) = raw.chars().find(|c| matches!(c, '˥' | '˧' | '˩')) {
            return Err(AffixError::BadIpa(Some(tone.to_string())));
        }
        Form::from_ipa(raw).ok_or_else(|| AffixError::BadIpa(Some(unread(raw))))
    }
}

impl AffixError {
    pub fn describe(&self, affix: &Affix) -> String {
        let (written, category) = (affix.written(), affix.category.id());
        match self {
            Self::BadIpa(Some(at)) => {
                format!("The {category} affix {written} cannot be read at “{at}”.")
            }
            Self::BadIpa(None) => format!("The {category} affix is empty."),
            Self::OutsideInventory(ipa) => format!(
                "The {category} affix {written} uses {ipa}, a sound these speakers do not have."
            ),
            Self::Duplicate => format!("The {category} is given two affixes."),
        }
    }
}

/// Why a root is refused. A root is taken whole or not at all: the engine
/// never repairs, trims, or drops an author's form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootError {
    /// No meaning a founding language has a word for carries this id.
    UnknownConcept,
    /// The catalog cannot read the form: the fragment it stopped at, or
    /// `None` for an empty form.
    BadIpa(Option<String>),
    /// The form parses but has no vowel, so it could never be a word.
    NoVowel,
    /// A sound the design leaves out, by IPA.
    OutsideInventory(String),
    /// Another root already gives this meaning its word.
    Duplicate,
}

impl RootError {
    pub fn describe(&self, root: &Root) -> String {
        match self {
            Self::UnknownConcept => format!("The catalog has no meaning “{}”.", root.concept),
            Self::BadIpa(Some(at)) if matches!(at.as_str(), "ˈ" | "ˌ") => format!(
                "The root /{}/ for “{}” marks its stress. Stress follows the language's rule, so leave the mark out.",
                root.ipa, root.concept
            ),
            Self::BadIpa(Some(at)) => format!(
                "The root /{}/ for “{}” cannot be read at “{at}”.",
                root.ipa, root.concept
            ),
            Self::BadIpa(None) => format!("The root for “{}” is empty.", root.concept),
            Self::NoVowel => format!(
                "The root /{}/ for “{}” has no vowel.",
                root.ipa, root.concept
            ),
            Self::OutsideInventory(ipa) => format!(
                "The root /{}/ for “{}” uses {ipa}, a sound these speakers do not have.",
                root.ipa, root.concept
            ),
            Self::Duplicate => format!("“{}” is given two roots.", root.concept),
        }
    }
}

/// Whether a founding language gets a word for `concept` at year 0: every
/// meaning except the pronoun cells a language may lack and the meanings
/// that wait for a craft, a way of life, or a faith (`ideas::NEEDS`).
pub fn foundable(concept: &Concept) -> bool {
    !crate::pronouns::is_optional_cell(&concept.id) && crate::ideas::need(concept).is_none()
}

impl Root {
    /// The meaning and form this root founds, before the design's own
    /// sounds are checked.
    pub fn resolve(&self, concepts: &ConceptTable) -> Result<(ConceptId, Form), RootError> {
        let concept = concepts
            .by_id(&self.concept)
            .ok_or(RootError::UnknownConcept)?;
        Ok((concept, self.form()?))
    }

    /// Parse an authored form without repairing it.
    pub fn form(&self) -> Result<Form, RootError> {
        let raw = self.ipa.trim();
        if raw.is_empty() {
            return Err(RootError::BadIpa(None));
        }
        if let Some(tone) = raw.chars().find(|c| matches!(c, '˥' | '˧' | '˩')) {
            // Tone arises in a history; no founding root carries it.
            return Err(RootError::BadIpa(Some(tone.to_string())));
        }
        let form = Form::from_ipa(raw).ok_or_else(|| RootError::BadIpa(Some(unread(raw))))?;
        if form.vowel_count() == 0 {
            return Err(RootError::NoVowel);
        }
        Ok(form)
    }
}

/// The character where the catalog stops reading `raw`: the first one past
/// the longest prefix it can read.
fn unread(raw: &str) -> String {
    let text: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '.')
        .collect();
    let readable = text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .rev()
        .find(|&end| Form::from_ipa(&text[..end]).is_some())
        .unwrap_or(0);
    text[readable..]
        .chars()
        .next()
        .map_or_else(String::new, |c| c.to_string())
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
    /// Consonant clusters that may begin a syllable (`st`, `tr`, `kw`),
    /// each two or three of the design's consonants. Drawn words use them
    /// as often as `onset_clusters` says, and loans may keep them. Absent
    /// in older designs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clusters: Vec<String>,
    /// 0–1: how often a drawn syllable begins with one of `clusters`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub onset_clusters: f32,
    #[serde(default)]
    pub stress: Option<StressRule>,
    pub building: MorphologyKind,
    /// 0–1: share of affixes that are suffixes rather than prefixes.
    pub suffixing: f32,
    /// 0–1: how often related meanings are built from one another.
    pub derivation: f32,
    /// Editable founding choices for marking, word order, and possession.
    #[serde(default)]
    pub grammar: GrammarDesign,
    pub spelling: Spelling,
    /// The author's own words for some meanings, kept verbatim at year 0;
    /// every other word is drawn. Absent in older designs.
    #[serde(default)]
    pub roots: Vec<Root>,
    /// The author's own affixes for some grammatical categories, kept
    /// verbatim at year 0; every other marker is drawn. Each one settles
    /// its category's marking over `grammar`. Absent in older designs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affixes: Vec<Affix>,
}

fn is_zero(value: &f32) -> bool {
    *value == 0.0
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
            // A preset's preferred clusters are not carried over: a design
            // has clusters only where its author lists them.
            clusters: Vec::new(),
            onset_clusters: 0.0,
            stress: profile.stress,
            building: profile.morphology.kind,
            suffixing: profile.morphology.suffixing,
            derivation: profile.morphology.derivation,
            grammar: profile.grammar.draw(seed, &profile.morphology),
            spelling: profile.spelling.clone(),
            roots: profile.roots.clone(),
            affixes: profile.affixes.clone(),
        }
    }

    /// Why this design cannot found a language, if it cannot.
    pub fn validate(&self, concepts: &ConceptTable) -> Result<(), String> {
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
            self.onset_clusters,
            self.suffixing,
            self.derivation,
        ];
        if knobs.iter().any(|k| !(0.0..=1.0).contains(k)) {
            return Err("knobs run from 0 to 1".into());
        }
        let mut supplied: Vec<ConceptId> = Vec::new();
        for root in &self.roots {
            let (concept, _) = self
                .check_root(root, concepts)
                .map_err(|e| e.describe(root))?;
            if supplied.contains(&concept) {
                return Err(RootError::Duplicate.describe(root));
            }
            supplied.push(concept);
        }
        for (at, cluster) in self.clusters.iter().enumerate() {
            let ids = self.check_cluster(cluster)?;
            if self.clusters[..at]
                .iter()
                .any(|other| CATALOG.parse_ipa(other).as_ref() == Some(&ids))
            {
                return Err(format!("the cluster '{cluster}' is listed twice"));
            }
        }
        let mut marked: Vec<Category> = Vec::new();
        for affix in &self.affixes {
            self.check_affix(affix).map_err(|e| e.describe(affix))?;
            if marked.contains(&affix.category) {
                return Err(AffixError::Duplicate.describe(affix));
            }
            marked.push(affix.category);
        }
        Ok(())
    }

    /// The form `affix` founds in this design, or why it is refused: it
    /// must read in the catalog and use only the design's sounds.
    pub fn check_affix(&self, affix: &Affix) -> Result<Form, AffixError> {
        let form = affix.form()?;
        if let Some(seg) = form.segs.iter().find(|seg| {
            !self
                .sounds
                .iter()
                .any(|s| s.ipa == CATALOG.get(seg.phone).ipa())
        }) {
            return Err(AffixError::OutsideInventory(
                CATALOG.get(seg.phone).ipa().to_string(),
            ));
        }
        Ok(form)
    }

    /// The sounds of an onset cluster, or why it is refused: two or three
    /// consonants, all among the design's sounds.
    pub fn check_cluster(&self, cluster: &str) -> Result<Vec<PhonemeId>, String> {
        let ids = CATALOG
            .parse_ipa(cluster)
            .ok_or_else(|| format!("the cluster '{cluster}' cannot be read"))?;
        if !(2..=3).contains(&ids.len()) || ids.iter().any(|id| CATALOG.get(*id).is_vowel()) {
            return Err(format!(
                "'{cluster}' is not a cluster of two or three consonants"
            ));
        }
        if let Some(id) = ids
            .iter()
            .find(|id| !self.sounds.iter().any(|s| s.ipa == CATALOG.get(**id).ipa()))
        {
            return Err(format!(
                "the cluster '{cluster}' uses {}, a sound these speakers do not have",
                CATALOG.get(*id).ipa()
            ));
        }
        Ok(ids)
    }

    /// The grammar choices with each given affix's category settled as an
    /// affix on its side.
    pub fn settled_grammar(&self) -> GrammarDesign {
        let mut grammar = self.grammar;
        for affix in &self.affixes {
            grammar.set(
                affix.category,
                match affix.side {
                    Side::Suffix => GrammarChoice::Suffix,
                    Side::Prefix => GrammarChoice::Prefix,
                },
            );
        }
        grammar
    }

    /// The meaning and form `root` founds in this design, or why it is
    /// refused: its form must read in the catalog and use only the design's
    /// sounds, so later coinages and the card's sound list agree with it.
    /// Shapes the design's knobs would never draw are accepted: the
    /// author's language outranks the knobs.
    pub fn check_root(
        &self,
        root: &Root,
        concepts: &ConceptTable,
    ) -> Result<(ConceptId, Form), RootError> {
        let (concept, form) = root.resolve(concepts)?;
        if !foundable(&concepts[concept]) {
            return Err(RootError::UnknownConcept);
        }
        if let Some(seg) = form.segs.iter().find(|seg| {
            !self
                .sounds
                .iter()
                .any(|s| s.ipa == CATALOG.get(seg.phone).ipa())
        }) {
            return Err(RootError::OutsideInventory(
                CATALOG.get(seg.phone).ipa().to_string(),
            ));
        }
        Ok((concept, form))
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
            max_onset: self
                .clusters
                .iter()
                .filter_map(|cluster| CATALOG.parse_ipa(cluster))
                .map(|ids| ids.len() as u8)
                .fold(2, u8::max),
            max_coda: 1,
            final_coda: self.final_consonants,
            open_medial: !self.inner_clusters,
            preferred_onsets: self.clusters.clone(),
            preferred_codas: Vec::new(),
            disyllabic_roots: self.word_length,
            identical_consonants: self.repetition,
            long_vowels: self.long_vowels,
            geminates: self.geminates,
            onset_clusters: self.onset_clusters,
        };
        profile.morphology = MorphologyPrior {
            kind: self.building,
            suffixing: self.suffixing,
            derivation: self.derivation,
        };
        profile.spelling = self.spelling.clone();
        profile.roots = self.roots.clone();
        profile.affixes = self.affixes.clone();
        profile.stress = self.stress;
        profile.grammar = GrammarPrior::fixed(self.settled_grammar());
        profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concepts;
    use crate::grammar::{Category, GrammarChoice, MarkerKind, Side};
    use crate::lexicon::Origin;
    use crate::variety::Variety;

    fn assert_founding_grammar(variety: &Variety, expected: GrammarDesign) {
        assert_eq!(Some(variety.grammar.order), expected.order);
        assert_eq!(Some(variety.grammar.possessor), expected.possessor);
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
                (MarkerKind::Pattern, _) => unreachable!("patterns supplement the chosen affix"),
            };
            assert_eq!(choice, expected.choice(category), "{}", category.id());
        }
    }

    #[test]
    fn designs_found_exactly_their_sounds() {
        let design = LanguageDesign::by_frequency(7, 12, 5);
        assert!(design.validate(&ConceptTable::default()).is_ok());
        let variety = Variety::found(
            &crate::ConceptTable::default(),
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
        assert!(
            design
                .validate(&ConceptTable::default())
                .unwrap_err()
                .contains("nope")
        );
        let mut design = LanguageDesign::by_frequency(1, 10, 5);
        design
            .sounds
            .retain(|s| !CATALOG.get(CATALOG.id_by_ipa(&s.ipa).unwrap()).is_vowel());
        assert!(
            design
                .validate(&ConceptTable::default())
                .unwrap_err()
                .contains("vowels")
        );
        let mut design = LanguageDesign::by_frequency(1, 10, 5);
        design.word_length = 1.5;
        assert!(design.validate(&ConceptTable::default()).is_err());
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
            ..GrammarDesign::default()
        };
        let mut json = serde_json::to_value(&design).unwrap();
        json.as_object_mut().unwrap().remove("grammar");
        let loaded: LanguageDesign = serde_json::from_value(json).unwrap();
        let suffixes = GrammarDesign::default();
        assert_eq!(
            loaded,
            LanguageDesign {
                grammar: suffixes,
                ..design
            }
        );
        for seed in [0, 7, u64::MAX] {
            let variety = Variety::found(
                &crate::ConceptTable::default(),
                seed,
                &loaded.profile(),
                crate::Livelihood::Farming,
                crate::Ethos::default(),
            );
            assert_founding_grammar(
                &variety,
                loaded
                    .profile()
                    .grammar
                    .draw(seed, &loaded.profile().morphology),
            );
        }
    }

    #[test]
    fn older_design_draws_missing_categories_and_orders_at_founding() {
        let design = LanguageDesign::preset("germanic", 9).unwrap();
        let mut json = serde_json::to_value(&design).unwrap();
        let grammar = json["grammar"].as_object_mut().unwrap();
        for field in ["object", "order", "possessor", "future", "progressive"] {
            grammar.remove(field);
        }
        let loaded: LanguageDesign = serde_json::from_value(json).unwrap();
        let profile = loaded.profile();
        assert_eq!(profile.grammar.object, None);
        assert_eq!(profile.grammar.order, None);
        assert_eq!(profile.grammar.possessor, None);
        assert_eq!(profile.grammar.future, None);
        assert_eq!(profile.grammar.progressive, None);
        for seed in [0, 7, u64::MAX] {
            let variety = Variety::found(
                &crate::ConceptTable::default(),
                seed,
                &profile,
                crate::Livelihood::Farming,
                crate::Ethos::default(),
            );
            assert_founding_grammar(&variety, profile.grammar.draw(seed, &profile.morphology));
        }
    }

    #[test]
    fn preset_and_resolved_design_found_with_the_same_grammar() {
        for profile in SoundProfile::presets() {
            for seed in 0..4 {
                let design = LanguageDesign::from_profile(&profile, seed);
                let preset = Variety::found(
                    &crate::ConceptTable::default(),
                    seed,
                    &profile,
                    crate::Livelihood::Farming,
                    crate::Ethos::default(),
                );
                let resolved = Variety::found(
                    &crate::ConceptTable::default(),
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
                object: Some(choices[(i + 2) % choices.len()]),
                future: Some(choices[(i + 3) % choices.len()]),
                progressive: Some(choices[i]),
                ..design.grammar
            };
            let json = serde_json::to_string(&design).unwrap();
            let loaded: LanguageDesign = serde_json::from_str(&json).unwrap();
            let variety = Variety::found(
                &crate::ConceptTable::default(),
                23,
                &loaded.profile(),
                crate::Livelihood::Farming,
                crate::Ethos::default(),
            );
            assert_founding_grammar(&variety, design.grammar);
            for id in ["fire", "water", "people", "blood", "sand", "cattle", "food"] {
                let word = variety
                    .lexicon
                    .word_for(concepts::by_id(id).unwrap())
                    .expect("ordinary founding meaning");
                assert!(word.paradigms.is_empty(), "{id} must not inflect");
            }
            for (id, category) in [
                ("fish", Category::Plural),
                ("go", Category::Past),
                ("dog", Category::Object),
                ("go", Category::Future),
                ("stand", Category::Progressive),
            ] {
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
                &crate::ConceptTable::default(),
                7,
                &design.profile(),
                crate::Livelihood::Farming,
                crate::Ethos::default(),
            )
        };
        design.stress = Some(StressRule::Final);
        let final_stress = Variety::found(
            &crate::ConceptTable::default(),
            7,
            &design.profile(),
            crate::Livelihood::Farming,
            crate::Ethos::default(),
        );
        assert_eq!(initial.stress(), StressRule::Initial);
        assert_eq!(final_stress.stress(), StressRule::Final);
        let mut moved = false;
        for (a, b) in initial.lexicon.living().zip(final_stress.lexicon.living()) {
            assert_eq!(a.first_sense, b.first_sense);
            assert_eq!(a.form.segs, b.form.segs);
            assert_eq!(a.form.stress, None);
            assert_eq!(b.form.stress, None);
            moved |= a.form.stressed_syllable(initial.stress())
                != b.form.stressed_syllable(final_stress.stress());
        }
        assert!(moved);
    }

    /// The familiar preset with every sound the roots need, and the roots.
    fn with_roots(seed: u64, roots: &[(&str, &str)]) -> LanguageDesign {
        let mut design = LanguageDesign::preset("familiar", seed).unwrap();
        for (concept, ipa) in roots {
            for seg in Form::from_ipa(ipa).unwrap().segs {
                let ipa = CATALOG.get(seg.phone).ipa();
                if !design.sounds.iter().any(|s| s.ipa == ipa) {
                    design.sounds.push(Sound {
                        ipa: ipa.into(),
                        favoured: false,
                    });
                }
            }
            design.roots.push(Root {
                concept: (*concept).into(),
                ipa: (*ipa).into(),
            });
        }
        design
    }

    fn found(design: &LanguageDesign, seed: u64) -> Variety {
        Variety::found(
            &crate::ConceptTable::default(),
            seed,
            &design.profile(),
            crate::Livelihood::Farming,
            crate::Ethos::default(),
        )
    }

    #[test]
    fn roots_are_refused_whole_with_a_reason() {
        // The sounds the roots below use, without any root yet.
        let mut design = with_roots(
            3,
            &[
                ("water", "nara"),
                ("stone", "mata"),
                ("chief", "kraspantark"),
            ],
        );
        design.roots.clear();
        let root = |concept: &str, ipa: &str| Root {
            concept: concept.into(),
            ipa: ipa.into(),
        };
        let refused = |concept: &str, ipa: &str| {
            design
                .check_root(&root(concept, ipa), &crate::ConceptTable::default())
                .unwrap_err()
        };
        assert_eq!(refused("unicorn", "nara"), RootError::UnknownConcept);
        // Pronoun cells a language may lack, and meanings that wait for a
        // craft, a way of life, or a faith, have no founding word.
        assert_eq!(refused("2sg-polite", "nara"), RootError::UnknownConcept);
        assert_eq!(refused("iron", "nara"), RootError::UnknownConcept);
        assert_eq!(refused("till", "nara"), RootError::UnknownConcept);
        assert_eq!(refused("water", ""), RootError::BadIpa(None));
        assert_eq!(
            refused("water", "naña"),
            RootError::BadIpa(Some("ñ".into()))
        );
        assert_eq!(
            refused("water", "ˈnara"),
            RootError::BadIpa(Some("ˈ".into()))
        );
        assert_eq!(
            refused("water", "na˥ra"),
            RootError::BadIpa(Some("˥".into()))
        );
        assert_eq!(refused("water", "kr"), RootError::NoVowel);
        assert!(!design.sounds.iter().any(|s| s.ipa == "q"));
        assert_eq!(
            refused("water", "qara"),
            RootError::OutsideInventory("q".into())
        );
        let (concept, form) = design
            .check_root(&root("water", "na.ra"), &crate::ConceptTable::default())
            .unwrap();
        assert_eq!(crate::CONCEPTS[concept.0 as usize].id, "water");
        assert_eq!(form, Form::from_ipa("nara").unwrap());
        assert_eq!(form.stress, None);
        assert!(form.boundaries.is_empty());

        // `validate` names the offending root and refuses the whole design.
        design.roots = vec![root("water", "nara"), root("stone", "qara")];
        assert!(
            design
                .validate(&ConceptTable::default())
                .unwrap_err()
                .contains("stone")
        );
        design.roots.pop();
        design.roots.push(root("water", "mata"));
        assert!(
            design
                .validate(&ConceptTable::default())
                .unwrap_err()
                .contains("water")
        );
        design.roots.pop();
        assert_eq!(design.validate(&ConceptTable::default()), Ok(()));
        // Shapes the knobs never draw are the author's business.
        design.word_length = 0.0;
        design.final_consonants = 0.0;
        design.inner_clusters = false;
        design.roots.push(root("chief", "kraspantark"));
        assert_eq!(design.validate(&ConceptTable::default()), Ok(()));
    }

    #[test]
    fn supplied_roots_leave_every_other_word_alone() {
        let given = [("water", "nara"), ("stone", "gora"), ("chief", "kaːr")];
        for seed in [1, 7, 1319325112] {
            let mut same_sounds = with_roots(seed, &given);
            same_sounds.roots.clear();
            let plain = found(&same_sounds, seed);
            let rooted = found(&with_roots(seed, &given), seed);
            assert_eq!(plain.lexicon.lexemes.len(), rooted.lexicon.lexemes.len());
            for (a, b) in plain.lexicon.lexemes.iter().zip(&rooted.lexicon.lexemes) {
                assert_eq!(a.first_sense, b.first_sense);
                match given
                    .iter()
                    .find(|(id, _)| *id == crate::CONCEPTS[a.first_sense.0 as usize].id)
                {
                    Some((_, ipa)) => {
                        assert_eq!(b.form, Form::from_ipa(ipa).unwrap());
                        assert!(b.authored);
                        assert_eq!(b.origin, Origin::Founding);
                    }
                    None => {
                        assert_eq!(
                            a.form, b.form,
                            "seed {seed}: {}",
                            plain.lexicon.concepts[a.first_sense].id
                        );
                        assert!(!b.authored);
                    }
                }
            }
            // The author's word is the word for its meaning, and the sample
            // list has no duplicate within the field.
            let water = rooted
                .lexicon
                .word_for(concepts::by_id("water").unwrap())
                .unwrap();
            assert!(water.authored);
            assert_eq!(rooted.spell(&water.form), "nara");
        }
    }

    /// A root the author gave is an ordinary word afterwards: sound law
    /// changes it, and nothing else does.
    #[test]
    fn an_authored_root_changes_only_by_recorded_laws() {
        use crate::lexicon::Event;
        use crate::world::{Params, World};
        let supplied = Form::from_ipa("nara").unwrap();
        let mut changed = 0;
        for seed in 0..6 {
            let design = with_roots(seed, &[("water", "nara")]);
            let mut world = World::new(seed, Params::static_society());
            world.found_seeded(
                &crate::Naming::People,
                &design.profile(),
                seed,
                0.5,
                0.5,
                None,
                None,
                None,
            );
            world.run(40);
            let variety = &world.varieties[0];
            let word = variety
                .lexicon
                .lexemes
                .iter()
                .find(|l| l.authored)
                .expect("the author's root is in the lexicon");
            assert_eq!(world.concepts[word.first_sense].id, "water");
            assert_eq!(*word.form_at(0), supplied, "seed {seed}");
            let mut current = supplied.clone();
            for entry in &word.log {
                let Event::SoundLaw { law, before } = &entry.event else {
                    continue;
                };
                assert!(
                    variety.laws.contains(&(entry.generation, law.clone())),
                    "seed {seed}: {law} at {} is not a recorded law",
                    entry.generation
                );
                assert_eq!(
                    *before, current,
                    "seed {seed}: {law} follows the last change"
                );
                let rule = world
                    .law_catalog()
                    .iter()
                    .find(|l| l.id == *law)
                    .expect("a catalog law");
                // The rule a law applied under: the one before any shift at
                // or after its generation.
                let stress = variety
                    .stress_history
                    .iter()
                    .find(|(g, _)| *g >= entry.generation)
                    .map_or(variety.stress(), |(_, before)| *before);
                current = rule.apply(before, variety.minimal, stress);
                changed += 1;
            }
            assert_eq!(
                word.form, current,
                "seed {seed}: the form is the laws' result"
            );
        }
        assert!(changed > 0, "forty generations change the word somewhere");
    }

    #[test]
    fn an_authored_pronoun_cell_is_kept() {
        for seed in [2, 5] {
            let plain = found(&with_roots(seed, &[]), seed);
            let rooted = found(&with_roots(seed, &[("1sg", "ŋo")]), seed);
            let cell = |v: &Variety, id: &str| {
                v.lexicon
                    .word_for(concepts::by_id(id).unwrap())
                    .unwrap()
                    .form
                    .clone()
            };
            assert_eq!(cell(&rooted, "1sg"), Form::from_ipa("ŋo").unwrap());
            assert_ne!(cell(&plain, "1sg"), cell(&rooted, "1sg"));
            for id in ["2sg", "3sg", "2pl", "3pl"] {
                assert_eq!(cell(&plain, id), cell(&rooted, id), "seed {seed}: {id}");
            }
        }
    }

    fn affix(category: Category, side: Side, ipa: &str) -> Affix {
        Affix {
            category,
            side,
            ipa: ipa.into(),
        }
    }

    #[test]
    fn affixes_found_verbatim_and_move_no_other_draw() {
        for seed in [3, 11, 29] {
            // The roots add the affixes' sounds to the design.
            let plain = with_roots(seed, &[("water", "nasa")]);
            let mut given = plain.clone();
            given.affixes = vec![
                affix(Category::Plural, Side::Suffix, "s"),
                affix(Category::Future, Side::Prefix, "na"),
            ];
            assert_eq!(given.validate(&crate::ConceptTable::default()), Ok(()));
            let (drawn, founded) = (found(&plain, seed), found(&given, seed));
            let marker = |variety: &Variety, category: Category| {
                variety
                    .grammar
                    .markers
                    .iter()
                    .find(|marker| marker.category == category)
                    .unwrap()
                    .clone()
            };
            let plural = marker(&founded, Category::Plural);
            assert_eq!(plural.kind, MarkerKind::Bound);
            assert_eq!(plural.side, Side::Suffix);
            assert_eq!(plural.form, Form::from_ipa("s").unwrap());
            assert_eq!(plural.origin, crate::grammar::MarkerOrigin::Authored);
            let future = marker(&founded, Category::Future);
            assert_eq!(
                (future.kind, future.side),
                (MarkerKind::Bound, Side::Prefix)
            );
            assert_eq!(future.form, Form::from_ipa("na").unwrap());
            // Words inflect with them: a plural is its word and the -s.
            let (word, plurals) = founded
                .lexicon
                .lexemes
                .iter()
                .find_map(|word| {
                    let paradigm = word
                        .paradigms
                        .iter()
                        .find(|paradigm| paradigm.category == Category::Plural)?;
                    Some((word, paradigm.realizations.first()?.form.as_ref()?))
                })
                .expect("some noun has a plural");
            assert_eq!(
                plurals.ipa(),
                format!("{}s", word.form.ipa()),
                "seed {seed}"
            );
            // The categories left alone are marked as the seed drew them,
            // and no word changes.
            for category in [Category::Past, Category::Object, Category::Genitive] {
                let (a, b) = (marker(&drawn, category), marker(&founded, category));
                assert_eq!((a.kind, a.side, a.form), (b.kind, b.side, b.form));
            }
            for (a, b) in drawn.lexicon.lexemes.iter().zip(&founded.lexicon.lexemes) {
                assert_eq!(a.form, b.form, "seed {seed}");
            }
        }
    }

    #[test]
    fn affixes_are_refused_whole_with_a_reason() {
        let mut design = with_roots(3, &[("water", "nasa")]);
        assert!(!design.sounds.iter().any(|s| s.ipa == "q"));
        let refused = |design: &LanguageDesign, affix: Affix| design.check_affix(&affix);
        assert_eq!(
            refused(&design, affix(Category::Plural, Side::Suffix, "")),
            Err(AffixError::BadIpa(None))
        );
        assert_eq!(
            refused(&design, affix(Category::Plural, Side::Suffix, "sñ")),
            Err(AffixError::BadIpa(Some("ñ".into())))
        );
        assert_eq!(
            refused(&design, affix(Category::Plural, Side::Suffix, "a˥")),
            Err(AffixError::BadIpa(Some("˥".into())))
        );
        assert_eq!(
            refused(&design, affix(Category::Plural, Side::Suffix, "qa")),
            Err(AffixError::OutsideInventory("q".into()))
        );
        design.affixes = vec![
            affix(Category::Object, Side::Suffix, "n"),
            affix(Category::Object, Side::Prefix, "a"),
        ];
        assert_eq!(
            design.validate(&crate::ConceptTable::default()),
            Err("The object is given two affixes.".into())
        );
        design.affixes.truncate(1);
        assert_eq!(design.validate(&crate::ConceptTable::default()), Ok(()));
        // Giving one settles the category as an affix on its side, over
        // whatever the grammar choices said.
        design.grammar.object = Some(GrammarChoice::None);
        assert_eq!(design.settled_grammar().object, Some(GrammarChoice::Suffix));
        assert_eq!(design.affixes[0].written(), "-n");
    }

    /// A design with s, t, r, and k among its sounds and no roots.
    fn clustering(seed: u64) -> LanguageDesign {
        let mut design = with_roots(seed, &[("water", "strika")]);
        design.roots.clear();
        design
    }

    #[test]
    fn listed_clusters_open_drawn_words_and_none_are_drawn_unasked() {
        for seed in [3, 11, 29] {
            let plain = clustering(seed);
            let mut listed = plain.clone();
            listed.clusters = vec!["st".into(), "tr".into(), "str".into()];
            assert_eq!(listed.validate(&crate::ConceptTable::default()), Ok(()));
            // Listed but never drawn: loans may keep them, and every word
            // is the one the seed drew before.
            let (before, after) = (found(&plain, seed), found(&listed, seed));
            for (a, b) in before.lexicon.lexemes.iter().zip(&after.lexicon.lexemes) {
                assert_eq!(a.form, b.form, "seed {seed}");
            }
            for inner in [true, false] {
                let mut drawn = listed.clone();
                drawn.onset_clusters = 0.5;
                drawn.inner_clusters = inner;
                let variety = found(&drawn, seed);
                let allowed: Vec<Vec<PhonemeId>> = drawn
                    .clusters
                    .iter()
                    .map(|cluster| CATALOG.parse_ipa(cluster).unwrap())
                    .collect();
                let (mut opening, mut inside) = (0, 0);
                for word in &variety.lexicon.lexemes {
                    if word.origin != Origin::Founding {
                        continue;
                    }
                    for (at, syllable) in word.form.syllables().iter().enumerate() {
                        if syllable.onset.len() < 2 {
                            continue;
                        }
                        let onset: Vec<PhonemeId> = word.form.segs[syllable.onset.clone()]
                            .iter()
                            .map(|seg| seg.phone)
                            .collect();
                        // A coda before it may have joined the syllable's
                        // start; what was drawn is still one of the listed.
                        assert!(
                            allowed.iter().any(
                                |cluster| onset.ends_with(cluster) || cluster.ends_with(&onset)
                            ),
                            "seed {seed}: /{}/",
                            word.form.ipa()
                        );
                        if at == 0 {
                            opening += 1;
                        } else {
                            inside += 1;
                        }
                    }
                }
                assert!(
                    opening > 20,
                    "seed {seed}: {opening} words open on a cluster"
                );
                // Where inner syllables stay open, a cluster only opens a word.
                assert_eq!(inside > 0, inner, "seed {seed}: {inside} inside");
            }
        }
    }

    #[test]
    fn clusters_are_refused_with_a_reason() {
        let mut design = clustering(3);
        assert!(!design.sounds.iter().any(|s| s.ipa == "q"));
        let refused = |design: &mut LanguageDesign, cluster: &str| {
            design.clusters = vec![cluster.into()];
            design
                .validate(&crate::ConceptTable::default())
                .unwrap_err()
        };
        assert_eq!(
            refused(&mut design, "sa"),
            "'sa' is not a cluster of two or three consonants"
        );
        assert_eq!(
            refused(&mut design, "s"),
            "'s' is not a cluster of two or three consonants"
        );
        assert_eq!(
            refused(&mut design, "strk"),
            "'strk' is not a cluster of two or three consonants"
        );
        assert_eq!(
            refused(&mut design, "sq"),
            "the cluster 'sq' uses q, a sound these speakers do not have"
        );
        assert_eq!(
            refused(&mut design, "s?"),
            "the cluster 's?' cannot be read"
        );
        design.clusters = vec!["st".into(), "s t".into()];
        assert_eq!(
            design.validate(&crate::ConceptTable::default()),
            Err("the cluster 's t' is listed twice".into())
        );
        design.clusters = vec!["st".into()];
        design.onset_clusters = 1.5;
        assert_eq!(
            design.validate(&crate::ConceptTable::default()),
            Err("knobs run from 0 to 1".into())
        );
    }

    #[test]
    fn a_design_without_affixes_reads_and_writes_as_before() {
        let design = LanguageDesign::preset("familiar", 3).unwrap();
        let written = serde_json::to_value(&design).unwrap();
        assert!(written.get("affixes").is_none());
        assert!(written.get("clusters").is_none());
        assert!(written.get("onsetClusters").is_none());
        let read: LanguageDesign = serde_json::from_value(written).unwrap();
        assert_eq!(read, design);
        let mut given = design.clone();
        given.affixes = vec![affix(Category::Past, Side::Suffix, "ta")];
        let written = serde_json::to_value(&given).unwrap();
        assert_eq!(
            written["affixes"],
            serde_json::json!([{"category": "past", "side": "suffix", "ipa": "ta"}])
        );
        assert_eq!(
            serde_json::from_value::<LanguageDesign>(written).unwrap(),
            given
        );
    }
}

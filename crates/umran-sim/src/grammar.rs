//! Grammatical contrasts and their changing spoken realizations.
//! Inflection changes a word for grammar. Particles remain separate words.

use crate::concepts::{Concept, by_id};
use crate::form::{Form, Seg};
use crate::laws::Law;
use crate::lexicon::{LexemeId, Lexicon};
use crate::morphology::Morphology;
use crate::phonotactics::Phonotactics;
use crate::profile::{MorphologyKind, MorphologyPrior, SoundProfile};
use crate::prosody::{MinimalWord, StressRule};
use crate::rng::{index, key, stream, weighted_index};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Plural,
    Past,
    Object,
    Future,
    Progressive,
    Genitive,
}
impl Category {
    pub const ALL: [Self; 6] = [
        Self::Plural,
        Self::Past,
        Self::Object,
        Self::Future,
        Self::Progressive,
        Self::Genitive,
    ];
    pub fn id(self) -> &'static str {
        match self {
            Self::Plural => "plural",
            Self::Past => "past",
            Self::Object => "object",
            Self::Future => "future",
            Self::Progressive => "progressive",
            Self::Genitive => "genitive",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Plural => "plural",
            Self::Past => "past",
            Self::Object => "the object",
            Self::Future => "future",
            Self::Progressive => "progressive",
            Self::Genitive => "the possessor",
        }
    }
    pub fn position(self) -> usize {
        match self {
            Self::Plural => 0,
            Self::Past => 1,
            Self::Object => 2,
            Self::Future => 3,
            Self::Progressive => 4,
            Self::Genitive => 5,
        }
    }
    fn sources(self) -> &'static [&'static str] {
        match self {
            Self::Plural => &["many", "all", "people"],
            Self::Past => &["finish", "have"],
            Self::Object => &["take", "give", "hand"],
            Self::Future => &["go", "come", "have"],
            Self::Progressive => &["stand"],
            Self::Genitive => &["have", "hand"],
        }
    }
    fn evolves(self, tense_aspect: bool) -> bool {
        tense_aspect || !matches!(self, Self::Future | Self::Progressive)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GrammarChoice {
    Suffix,
    Prefix,
    Particle,
    None,
}
impl GrammarChoice {
    pub fn id(self) -> &'static str {
        match self {
            Self::Suffix => "suffix",
            Self::Prefix => "prefix",
            Self::Particle => "particle",
            Self::None => "none",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordOrder {
    #[default]
    SOV,
    SVO,
    VSO,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PossessorOrder {
    #[default]
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarDesign {
    pub plural: GrammarChoice,
    pub past: GrammarChoice,
    /// Omitted settings are drawn when the founding seed is known.
    #[serde(default)]
    pub object: Option<GrammarChoice>,
    #[serde(default)]
    pub order: Option<WordOrder>,
    #[serde(default)]
    pub possessor: Option<PossessorOrder>,
    #[serde(default)]
    pub future: Option<GrammarChoice>,
    #[serde(default)]
    pub progressive: Option<GrammarChoice>,
    #[serde(default)]
    pub genitive: Option<GrammarChoice>,
}
impl Default for GrammarDesign {
    fn default() -> Self {
        Self {
            plural: GrammarChoice::Suffix,
            past: GrammarChoice::Suffix,
            object: None,
            order: None,
            possessor: None,
            future: None,
            progressive: None,
            genitive: None,
        }
    }
}
impl GrammarDesign {
    pub fn choice(self, category: Category) -> GrammarChoice {
        match category {
            Category::Plural => self.plural,
            Category::Past => self.past,
            Category::Object => self.object.expect("resolved founding object choice"),
            Category::Future => self.future.expect("resolved founding future choice"),
            Category::Progressive => self
                .progressive
                .expect("resolved founding progressive choice"),
            Category::Genitive => self.genitive.expect("resolved founding genitive choice"),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GrammarPrior {
    pub plural: Option<GrammarChoice>,
    pub past: Option<GrammarChoice>,
    #[serde(default)]
    pub object: Option<GrammarChoice>,
    #[serde(default)]
    pub order: Option<WordOrder>,
    #[serde(default)]
    pub possessor: Option<PossessorOrder>,
    #[serde(default)]
    pub future: Option<GrammarChoice>,
    #[serde(default)]
    pub progressive: Option<GrammarChoice>,
    #[serde(default)]
    pub genitive: Option<GrammarChoice>,
}
impl GrammarPrior {
    pub fn fixed(design: GrammarDesign) -> Self {
        Self {
            plural: Some(design.plural),
            past: Some(design.past),
            object: design.object,
            order: design.order,
            possessor: design.possessor,
            future: design.future,
            progressive: design.progressive,
            genitive: design.genitive,
        }
    }
    pub fn draw(self, seed: u64, morphology: &MorphologyPrior) -> GrammarDesign {
        let mut choices = [GrammarChoice::None; Category::ALL.len()];
        for category in Category::ALL {
            let resolved = match category {
                Category::Plural => self.plural,
                Category::Past => self.past,
                Category::Object => self.object,
                Category::Future => self.future,
                Category::Progressive => self.progressive,
                Category::Genitive => self.genitive,
            };
            choices[category.position()] = resolved.unwrap_or_else(|| {
                let mut rng = stream(seed, &[key("grammar founding"), key(category.id())]);
                let (bound, particle, none) = match (category, morphology.kind) {
                    (Category::Object, _) => (0.45, 0.15, 0.4),
                    (Category::Future, _) => (0.3, 0.3, 0.4),
                    (Category::Progressive, _) => (0.25, 0.35, 0.4),
                    (Category::Genitive, _) => (0.45, 0.35, 0.20),
                    (_, MorphologyKind::Concatenative) => (0.6, 0.3, 0.1),
                    (_, MorphologyKind::RootPattern) => (0.5, 0.4, 0.1),
                };
                match weighted_index(
                    &mut rng,
                    [
                        bound * morphology.suffixing,
                        bound * (1.0 - morphology.suffixing),
                        particle,
                        none,
                    ]
                    .into_iter(),
                ) {
                    0 => GrammarChoice::Suffix,
                    1 => GrammarChoice::Prefix,
                    2 => GrammarChoice::Particle,
                    _ => GrammarChoice::None,
                }
            });
        }
        let order = self.order.unwrap_or_else(|| {
            let suffixing = morphology.suffixing;
            let weights = [
                0.15 + 0.40 * suffixing,
                0.50 - 0.15 * suffixing,
                0.35 - 0.25 * suffixing,
            ];
            let mut rng = stream(seed, &[key("grammar word order")]);
            [WordOrder::SOV, WordOrder::SVO, WordOrder::VSO]
                [weighted_index(&mut rng, weights.into_iter())]
        });
        let possessor = self.possessor.unwrap_or_else(|| {
            let before = match order {
                WordOrder::SOV => 0.8,
                WordOrder::SVO => 0.5,
                WordOrder::VSO => 0.15,
            };
            let mut rng = stream(seed, &[key("grammar possessor order")]);
            if rng.r#gen::<f32>() < before {
                PossessorOrder::Before
            } else {
                PossessorOrder::After
            }
        });
        GrammarDesign {
            plural: choices[0],
            past: choices[1],
            object: Some(choices[2]),
            order: Some(order),
            possessor: Some(possessor),
            future: Some(choices[Category::Future.position()]),
            progressive: Some(choices[Category::Progressive.position()]),
            genitive: Some(choices[Category::Genitive.position()]),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerKind {
    Bound,
    Particle,
    None,
}
impl MarkerKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Bound => "bound",
            Self::Particle => "particle",
            Self::None => "none",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Prefix,
    Suffix,
}
impl Side {
    pub fn id(self) -> &'static str {
        match self {
            Self::Prefix => "prefix",
            Self::Suffix => "suffix",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum MarkerOrigin {
    Founding,
    Grammaticalized {
        source: LexemeId,
        concept: &'static Concept,
        source_form: Form,
    },
    Fused {
        particle: u32,
    },
    Imported {
        from: usize,
        marker: u32,
        source: Form,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub id: u32,
    pub category: Category,
    pub kind: MarkerKind,
    pub side: Side,
    pub form: Form,
    pub born: u32,
    pub origin: MarkerOrigin,
    pub productive: bool,
    pub majority_generations: u32,
    pub retired: Option<u32>,
    pub history: Vec<GrammarEntry>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GrammarEntry {
    pub generation: u32,
    pub event: GrammarEvent,
}
#[derive(Clone, Debug, PartialEq)]
pub enum GrammarEvent {
    Created,
    SoundLaw { law: &'static str, before: Form },
    Analogy { before: Form },
    Imported { from: usize, source: Form },
    Retired,
}
/// One donor's current marked form, adapted at acquisition.
pub(crate) struct ImportedPair {
    pub category: Category,
    pub source_marker: u32,
    pub kind: MarkerKind,
    pub side: Side,
    pub marker_form: Form,
    pub source_marker_form: Form,
    pub form: Option<Form>,
    pub edge: usize,
    pub source_form: Form,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Realization {
    pub marker: u32,
    pub form: Option<Form>,
    pub edge: usize,
    pub share: f32,
    pub born: u32,
    pub retired: Option<u32>,
    pub history: Vec<GrammarEntry>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Paradigm {
    pub category: Category,
    pub realizations: Vec<Realization>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GrammarNotice {
    pub generation: u32,
    pub category: Category,
    pub event: NoticeKind,
}
#[derive(Clone, Debug, PartialEq)]
pub enum NoticeKind {
    NewMarker {
        marker: u32,
    },
    Fusion {
        particle: u32,
        marker: u32,
    },
    ContrastLoss,
    Analogy {
        lexeme: LexemeId,
        before: Form,
        after: Form,
    },
    ImportedPair {
        lexeme: LexemeId,
        from: usize,
    },
    MarkerTransfer {
        marker: u32,
        from: usize,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CategorySummary {
    pub eligible: usize,
    pub contrast_retention: f32,
    pub how_synthetic: f32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GrammarSummary {
    pub how_synthetic: f32,
    pub contrast_retention: f32,
    pub categories: [CategorySummary; Category::ALL.len()],
    pub marker_shares: Vec<f32>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Grammar {
    pub order: WordOrder,
    pub possessor: PossessorOrder,
    pub markers: Vec<Marker>,
    pub events: Vec<GrammarNotice>,
    pub summary: GrammarSummary,
    /// Consecutive generations of strong bilingual contact, by donor variety.
    pub contact_generations: BTreeMap<usize, u32>,
    usage: Vec<[f32; Category::ALL.len()]>,
}

fn historical<'a>(form: &'a Form, history: &'a [GrammarEntry], generation: u32) -> &'a Form {
    history
        .iter()
        .filter(|e| e.generation > generation)
        .find_map(|e| match &e.event {
            GrammarEvent::SoundLaw { before, .. } | GrammarEvent::Analogy { before } => {
                Some(before)
            }
            _ => None,
        })
        .unwrap_or(form)
}
pub fn same_sound(a: &Form, b: &Form, stress: StressRule) -> bool {
    a.segs == b.segs && a.stressed_syllable(stress) == b.stressed_syllable(stress)
}

impl Grammar {
    pub fn marker(&self, id: u32) -> &Marker {
        &self.markers[id as usize]
    }
    pub fn marker_share(&self, id: u32) -> f32 {
        self.summary
            .marker_shares
            .get(id as usize)
            .copied()
            .unwrap_or(0.0)
    }
    /// Case marking includes adpositions, provided a productive contrast survives.
    pub fn has_object_marking(&self) -> bool {
        self.summary.categories[Category::Object.position()].contrast_retention > 0.5
            && self.markers.iter().any(|marker| {
                marker.category == Category::Object
                    && marker.kind != MarkerKind::None
                    && marker.productive
                    && marker.retired.is_none()
            })
    }
    pub fn surface(&self, base: &Form, realization: &Realization) -> Vec<Form> {
        self.surface_at(base, realization, u32::MAX)
    }
    pub fn surface_at(&self, base: &Form, realization: &Realization, generation: u32) -> Vec<Form> {
        let generation = generation.min(realization.retired.unwrap_or(u32::MAX));
        if let Some(form) = &realization.form {
            return vec![historical(form, &realization.history, generation).clone()];
        }
        let marker = self.marker(realization.marker);
        if marker.kind != MarkerKind::Particle {
            return vec![base.clone()];
        }
        let particle = historical(&marker.form, &marker.history, generation).clone();
        match marker.side {
            Side::Prefix => vec![particle, base.clone()],
            Side::Suffix => vec![base.clone(), particle],
        }
    }
    pub fn audible(&self, base: &Form, realization: &Realization, stress: StressRule) -> bool {
        if let Some(form) = &realization.form {
            !same_sound(base, form, stress)
        } else {
            let marker = self.marker(realization.marker);
            marker.kind == MarkerKind::Particle && !marker.form.segs.is_empty()
        }
    }
    pub fn found(
        seed: u64,
        profile: &SoundProfile,
        tactics: &Phonotactics,
        morphology: &Morphology,
        stress: StressRule,
        lexicon: &mut Lexicon,
    ) -> Self {
        let design = profile.grammar.draw(seed, &profile.morphology);
        let mut grammar = Self {
            order: design.order.expect("resolved founding order"),
            possessor: design.possessor.expect("resolved founding possessor"),
            ..Self::default()
        };
        for category in Category::ALL {
            let choice = design.choice(category);
            let mut rng = stream(seed, &[key("grammar marker founding"), key(category.id())]);
            let side = match choice {
                GrammarChoice::Prefix => Side::Prefix,
                GrammarChoice::Suffix => Side::Suffix,
                _ => {
                    if rng.r#gen::<f32>() < profile.morphology.suffixing {
                        Side::Suffix
                    } else {
                        Side::Prefix
                    }
                }
            };
            let (kind, form, origin) = match choice {
                GrammarChoice::None => (MarkerKind::None, Form::default(), MarkerOrigin::Founding),
                GrammarChoice::Particle => {
                    let candidates: Vec<_> = category
                        .sources()
                        .iter()
                        .filter_map(|id| {
                            let concept = by_id(id)?;
                            Some((concept, lexicon.word_for(concept)?))
                        })
                        .collect();
                    let (concept, source) = candidates[index(&mut rng, candidates.len())];
                    (
                        MarkerKind::Particle,
                        source.form.clone(),
                        MarkerOrigin::Grammaticalized {
                            source: source.id,
                            concept,
                            source_form: source.form.clone(),
                        },
                    )
                }
                _ => {
                    let vowel = tactics.nuclei
                        [weighted_index(&mut rng, tactics.nuclei.iter().map(|(_, w)| *w))]
                    .0;
                    // Vowel endings are short enough to erode. A consonant is optional.
                    let mut phones = vec![vowel];
                    if rng.r#gen::<f32>() < 0.35 {
                        let onsets: Vec<_> = tactics
                            .onsets
                            .iter()
                            .filter(|(phones, _)| phones.len() == 1)
                            .collect();
                        let consonant =
                            onsets[weighted_index(&mut rng, onsets.iter().map(|(_, w)| *w))].0[0];
                        if side == Side::Suffix && rng.r#gen::<bool>() {
                            phones.push(consonant);
                        } else {
                            phones.insert(0, consonant);
                        }
                    }
                    (
                        MarkerKind::Bound,
                        Form::from_phones(phones),
                        MarkerOrigin::Founding,
                    )
                }
            };
            grammar.add_marker(category, kind, side, form, origin, true, 0);
        }
        grammar.sync(lexicon, morphology, stress, 0);
        grammar
    }
    #[allow(clippy::too_many_arguments)]
    fn add_marker(
        &mut self,
        category: Category,
        kind: MarkerKind,
        side: Side,
        mut form: Form,
        origin: MarkerOrigin,
        productive: bool,
        generation: u32,
    ) -> u32 {
        let id = self.markers.len() as u32;
        if kind == MarkerKind::Bound {
            form.stress = None;
        }
        self.markers.push(Marker {
            id,
            category,
            kind,
            side,
            form,
            born: generation,
            origin,
            productive,
            majority_generations: 0,
            retired: None,
            history: vec![GrammarEntry {
                generation,
                event: GrammarEvent::Created,
            }],
        });
        id
    }
    fn notice(&mut self, generation: u32, category: Category, event: NoticeKind) {
        self.events.push(GrammarNotice {
            generation,
            category,
            event,
        });
    }
    fn uses(&mut self, lexicon: &Lexicon) {
        self.usage
            .resize(lexicon.lexemes.len(), [0.0; Category::ALL.len()]);
        self.usage.fill([0.0; Category::ALL.len()]);
        for slot in &lexicon.slots {
            for category in Category::ALL {
                if slot.concept.categories.allows(category) {
                    for variant in &slot.variants {
                        self.usage[variant.lexeme.0 as usize][category.position()] +=
                            variant.weight;
                    }
                }
            }
        }
    }
    fn productive(&self, category: Category) -> Option<u32> {
        self.markers
            .iter()
            .filter(|m| m.category == category && m.productive && m.retired.is_none())
            .max_by(|a, b| {
                self.marker_share(a.id)
                    .total_cmp(&self.marker_share(b.id))
                    .then(b.id.cmp(&a.id))
            })
            .map(|m| m.id)
    }
    fn materialize(
        &self,
        marker: u32,
        base: &Form,
        morphology: &Morphology,
        stress: StressRule,
        generation: u32,
        share: f32,
    ) -> Realization {
        let m = self.marker(marker);
        let (form, edge) = if m.kind == MarkerKind::Bound {
            let mut joined = if m.form.segs.is_empty() {
                base.clone()
            } else {
                match m.side {
                    Side::Prefix => morphology.join(&m.form, base),
                    Side::Suffix => morphology.join(base, &m.form),
                }
            };
            let edge = if m.form.segs.is_empty() {
                match m.side {
                    Side::Prefix => 0,
                    Side::Suffix => joined.segs.len(),
                }
            } else {
                joined.boundaries[0]
            };
            if stress == StressRule::Free && joined.stress.is_none() {
                // New unaccented stems have initial lexical stress. Prefixes
                // carry that syllable across their vowels and any lost hiatus.
                let vowels = joined.vowel_count();
                joined.stress = (vowels > 0).then(|| match m.side {
                    Side::Prefix => vowels.saturating_sub(base.vowel_count()),
                    Side::Suffix => 0,
                });
            }
            (Some(joined), edge)
        } else {
            (None, 0)
        };
        Realization {
            marker,
            form,
            edge,
            share,
            born: generation,
            retired: None,
            history: vec![GrammarEntry {
                generation,
                event: GrammarEvent::Created,
            }],
        }
    }
    /// Only missing categories are materialized. Existing forms are never rebuilt.
    pub fn sync(
        &mut self,
        lexicon: &mut Lexicon,
        morphology: &Morphology,
        stress: StressRule,
        generation: u32,
    ) {
        self.uses(lexicon);
        for word in &mut lexicon.lexemes {
            for paradigm in &mut word.paradigms {
                if word.obsolete.is_some()
                    || self.usage[word.id.0 as usize][paradigm.category.position()] == 0.0
                {
                    for r in &mut paradigm.realizations {
                        if r.retired.is_none() {
                            r.retired = Some(generation);
                            r.share = 0.0;
                            r.history.push(GrammarEntry {
                                generation,
                                event: GrammarEvent::Retired,
                            });
                        }
                    }
                }
            }
            if word.obsolete.is_some() {
                continue;
            }
            for category in Category::ALL {
                if self.usage[word.id.0 as usize][category.position()] == 0.0 {
                    continue;
                }
                let exists = word.paradigms.iter().any(|p| {
                    p.category == category && p.realizations.iter().any(|r| r.retired.is_none())
                });
                if !exists && let Some(marker) = self.productive(category) {
                    let realization =
                        self.materialize(marker, &word.form, morphology, stress, generation, 1.0);
                    if let Some(p) = word.paradigms.iter_mut().find(|p| p.category == category) {
                        p.realizations.push(realization);
                    } else {
                        word.paradigms.push(Paradigm {
                            category,
                            realizations: vec![realization],
                        });
                    }
                }
            }
        }
        self.refresh(lexicon, stress, generation);
    }
    pub fn refresh(&mut self, lexicon: &Lexicon, stress: StressRule, generation: u32) {
        self.uses(lexicon);
        let old = self.summary.categories;
        let mut summary = GrammarSummary {
            marker_shares: vec![0.0; self.markers.len()],
            ..GrammarSummary::default()
        };
        let mut totals = [0.0; Category::ALL.len()];
        for word in lexicon.living() {
            for p in &word.paradigms {
                let i = p.category.position();
                let usage = self.usage[word.id.0 as usize][i];
                if usage == 0.0 {
                    continue;
                }
                summary.categories[i].eligible += 1;
                totals[i] += usage;
                for r in p.realizations.iter().filter(|r| r.retired.is_none()) {
                    let weight = usage * r.share;
                    summary.marker_shares[r.marker as usize] += weight;
                    if self.audible(&word.form, r, stress) {
                        summary.categories[i].contrast_retention += weight;
                        if r.form.is_some() {
                            summary.categories[i].how_synthetic += weight;
                        }
                    }
                }
            }
        }
        let total: f32 = totals.iter().sum();
        summary.how_synthetic = summary
            .categories
            .iter()
            .map(|c| c.how_synthetic)
            .sum::<f32>()
            / total.max(f32::EPSILON);
        summary.contrast_retention = summary
            .categories
            .iter()
            .map(|c| c.contrast_retention)
            .sum::<f32>()
            / total.max(f32::EPSILON);
        for category in Category::ALL {
            let i = category.position();
            let c = &mut summary.categories[i];
            c.contrast_retention /= totals[i].max(f32::EPSILON);
            c.how_synthetic /= totals[i].max(f32::EPSILON);
            if c.eligible >= 5 && old[i].contrast_retention > 0.5 && c.contrast_retention <= 0.5 {
                self.notice(generation, category, NoticeKind::ContrastLoss);
            }
        }
        for (m, share) in self.markers.iter().zip(&mut summary.marker_shares) {
            *share /= totals[m.category.position()].max(f32::EPSILON);
        }
        self.summary = summary;
    }
    /// Productive material follows surviving edges, including empty ones.
    fn update_edges(&mut self, lexicon: &Lexicon, generation: u32, law: &'static str) {
        let mut edges: Vec<HashMap<Vec<Seg>, f32>> = vec![HashMap::new(); self.markers.len()];
        for word in lexicon.living() {
            for p in &word.paradigms {
                for r in p.realizations.iter().filter(|r| r.retired.is_none()) {
                    if let Some(form) = &r.form {
                        let m = self.marker(r.marker);
                        let edge = r.edge.min(form.segs.len());
                        let segs = match m.side {
                            Side::Prefix => &form.segs[..edge],
                            Side::Suffix => &form.segs[edge..],
                        };
                        *edges[r.marker as usize].entry(segs.to_vec()).or_default() +=
                            r.share * self.usage[word.id.0 as usize][p.category.position()];
                    }
                }
            }
        }
        for (marker, edges) in self.markers.iter_mut().zip(edges) {
            if marker.kind != MarkerKind::Bound {
                continue;
            }
            if let Some((segs, _)) = edges.into_iter().max_by(|(a, wa), (b, wb)| {
                wa.total_cmp(wb).then_with(|| {
                    b.iter()
                        .map(|s| (s.phone.0, s.long))
                        .cmp(a.iter().map(|s| (s.phone.0, s.long)))
                })
            }) {
                let after = Form {
                    segs,
                    boundaries: Vec::new(),
                    stress: None,
                };
                if marker.form.segs != after.segs {
                    let before = std::mem::replace(&mut marker.form, after);
                    marker.history.push(GrammarEntry {
                        generation,
                        event: GrammarEvent::SoundLaw { law, before },
                    });
                }
            }
        }
    }
    pub fn apply_law(
        &mut self,
        lexicon: &mut Lexicon,
        law: &Law,
        minimal: MinimalWord,
        stress: StressRule,
        generation: u32,
    ) {
        for marker in &mut self.markers {
            if marker.kind == MarkerKind::Particle && marker.retired.is_none() {
                let after = law.apply(&marker.form, minimal, stress);
                if law.changes(&marker.form, &after, stress) {
                    let before = std::mem::replace(&mut marker.form, after);
                    marker.history.push(GrammarEntry {
                        generation,
                        event: GrammarEvent::SoundLaw {
                            law: law.id,
                            before,
                        },
                    });
                }
            }
        }
        for word in &mut lexicon.lexemes {
            if word.obsolete.is_some() {
                continue;
            }
            for p in &mut word.paradigms {
                for r in p.realizations.iter_mut().filter(|r| r.retired.is_none()) {
                    let Some(form) = &mut r.form else {
                        continue;
                    };
                    let before = form.clone();
                    for rule in &law.rules {
                        let (next, edge) = rule.apply_with_edge(form, stress, r.edge);
                        if !minimal.blocks(form, &next) {
                            r.edge = edge;
                            *form = next;
                        }
                    }
                    if law.changes(&before, form, stress) {
                        r.history.push(GrammarEntry {
                            generation,
                            event: GrammarEvent::SoundLaw {
                                law: law.id,
                                before,
                            },
                        });
                    }
                }
            }
        }
        self.uses(lexicon);
        self.update_edges(lexicon, generation, law.id);
        self.refresh(lexicon, law.stress.unwrap_or(stress), generation);
    }
    /// Shared particles occur once. Attached alternatives share one category use.
    pub fn forms<'a>(&'a self, lexicon: &'a Lexicon) -> impl Iterator<Item = (&'a Form, f32)> {
        lexicon
            .living()
            .flat_map(|l| {
                std::iter::once((&l.form, 1.0)).chain(
                    l.paradigms
                        .iter()
                        .flat_map(|p| p.realizations.iter())
                        .filter(|r| r.retired.is_none())
                        .filter_map(|r| r.form.as_ref().map(|f| (f, r.share))),
                )
            })
            .chain(
                self.markers
                    .iter()
                    .filter(|m| {
                        m.kind == MarkerKind::Particle
                            && m.retired.is_none()
                            && self.marker_share(m.id) > 0.0
                    })
                    .map(|m| (&m.form, self.marker_share(m.id))),
            )
    }
    fn introduce(paradigm: &mut Paradigm, mut realization: Realization) {
        if paradigm
            .realizations
            .iter()
            .filter(|r| r.retired.is_none())
            .count()
            >= 3
        {
            let least = paradigm
                .realizations
                .iter()
                .enumerate()
                .filter(|(_, r)| r.retired.is_none())
                .min_by(|(_, a), (_, b)| a.share.total_cmp(&b.share))
                .map(|(i, _)| i)
                .expect("active forms");
            let retired = &mut paradigm.realizations[least];
            retired.retired = Some(realization.born);
            retired.share = 0.0;
            retired.history.push(GrammarEntry {
                generation: realization.born,
                event: GrammarEvent::Retired,
            });
            let total: f32 = paradigm
                .realizations
                .iter()
                .filter(|r| r.retired.is_none())
                .map(|r| r.share)
                .sum();
            for r in paradigm
                .realizations
                .iter_mut()
                .filter(|r| r.retired.is_none())
            {
                r.share /= total.max(f32::EPSILON);
            }
        }
        let active = paradigm.realizations.iter().any(|r| r.retired.is_none());
        if !active {
            realization.share = 1.0;
        }
        for r in paradigm
            .realizations
            .iter_mut()
            .filter(|r| r.retired.is_none())
        {
            r.share *= 1.0 - realization.share;
        }
        paradigm.realizations.push(realization);
    }
    fn introduce_marker(
        &self,
        marker: u32,
        lexicon: &mut Lexicon,
        morphology: &Morphology,
        stress: StressRule,
        generation: u32,
        share: f32,
    ) {
        let category = self.marker(marker).category;
        for word in &mut lexicon.lexemes {
            if word.obsolete.is_some() {
                continue;
            }
            if let Some(p) = word.paradigms.iter_mut().find(|p| {
                p.category == category && p.realizations.iter().any(|r| r.retired.is_none())
            }) {
                let r = self.materialize(marker, &word.form, morphology, stress, generation, share);
                Self::introduce(p, r);
            }
        }
    }
    pub fn fuse(
        &mut self,
        particle: u32,
        lexicon: &mut Lexicon,
        morphology: &Morphology,
        stress: StressRule,
        generation: u32,
    ) -> u32 {
        let old = self.marker(particle).clone();
        let marker = self.add_marker(
            old.category,
            MarkerKind::Bound,
            old.side,
            old.form,
            MarkerOrigin::Fused { particle },
            old.productive,
            generation,
        );
        // Only words that actually use the particle receive its fused competitor.
        for word in &mut lexicon.lexemes {
            if word.obsolete.is_some() {
                continue;
            }
            for p in &mut word.paradigms {
                if p.realizations
                    .iter()
                    .any(|r| r.retired.is_none() && r.marker == particle)
                {
                    let r =
                        self.materialize(marker, &word.form, morphology, stress, generation, 0.18);
                    Self::introduce(p, r);
                }
            }
        }
        self.notice(
            generation,
            old.category,
            NoticeKind::Fusion { particle, marker },
        );
        self.refresh(lexicon, stress, generation);
        marker
    }
    #[allow(clippy::too_many_arguments)]
    pub fn evolve(
        &mut self,
        seed: u64,
        variety: usize,
        generation: u32,
        lexicon: &mut Lexicon,
        morphology: &Morphology,
        stress: StressRule,
        speakers: u32,
        tense_aspect: bool,
    ) {
        self.sync(lexicon, morphology, stress, generation);
        for category in Category::ALL {
            if !category.evolves(tense_aspect) {
                continue;
            }
            let i = category.position();
            let state = self.summary.categories[i];
            let loss = if state.eligible >= 5
                && self
                    .markers
                    .iter()
                    .any(|m| m.category == category && m.kind != MarkerKind::None)
            {
                1.0 - state.contrast_retention
            } else {
                0.0
            };
            let mut rng = stream(
                seed,
                &[
                    key("grammar rebuilding"),
                    variety as u64,
                    generation as u64,
                    key(category.id()),
                ],
            );
            if rng.r#gen::<f32>() < 0.0007 + 0.008 * loss {
                let sources: Vec<_> = category
                    .sources()
                    .iter()
                    .filter_map(|id| {
                        let concept = by_id(id)?;
                        Some((concept, lexicon.word_for(concept)?.id))
                    })
                    .collect();
                if !sources.is_empty() {
                    let (concept, source) = sources[index(&mut rng, sources.len())];
                    let source_form = lexicon.get(source).form.clone();
                    let side = self
                        .productive(category)
                        .map(|m| self.marker(m).side)
                        .unwrap_or(Side::Suffix);
                    let marker = self.add_marker(
                        category,
                        MarkerKind::Particle,
                        side,
                        source_form.clone(),
                        MarkerOrigin::Grammaticalized {
                            source,
                            concept,
                            source_form,
                        },
                        true,
                        generation,
                    );
                    self.introduce_marker(marker, lexicon, morphology, stress, generation, 0.12);
                    self.notice(generation, category, NoticeKind::NewMarker { marker });
                }
            }
        }
        for word in &mut lexicon.lexemes {
            if word.obsolete.is_some() {
                continue;
            }
            for p in &mut word.paradigms {
                if !p.category.evolves(tense_aspect) {
                    continue;
                }
                let mut active = [0usize; 3];
                let mut active_count = 0;
                for (i, r) in p.realizations.iter().enumerate() {
                    if r.retired.is_none() {
                        active[active_count] = i;
                        active_count += 1;
                    }
                }
                if active_count < 2 {
                    continue;
                }
                let mut rng = stream(
                    seed,
                    &[
                        key("grammar competition"),
                        variety as u64,
                        generation as u64,
                        key(p.category.id()),
                        word.id.0 as u64,
                    ],
                );
                let mut weights = [0.0_f32; 3];
                for (weight, &i) in weights.iter_mut().zip(&active[..active_count]) {
                    *weight = p.realizations[i].share
                        * if self.audible(&word.form, &p.realizations[i], stress) {
                            1.06
                        } else {
                            1.0
                        };
                }
                let n = speakers.clamp(16, 64);
                let mut counts = [0u32; 3];
                for _ in 0..n {
                    counts[weighted_index(&mut rng, weights[..active_count].iter().copied())] += 1;
                }
                for (&i, count) in active[..active_count].iter().zip(counts) {
                    let r = &mut p.realizations[i];
                    r.share = count as f32 / n as f32;
                    if count == 0 {
                        r.retired = Some(generation);
                        r.history.push(GrammarEntry {
                            generation,
                            event: GrammarEvent::Retired,
                        });
                    }
                }
            }
        }
        self.refresh(lexicon, stress, generation);
        let mut fusions = Vec::new();
        for i in 0..self.markers.len() {
            let already_fused = self
                .markers
                .iter()
                .any(|m| matches!(m.origin,MarkerOrigin::Fused {particle} if particle==i as u32));
            let marker = &mut self.markers[i];
            if !marker.category.evolves(tense_aspect)
                || marker.kind != MarkerKind::Particle
                || marker.retired.is_some()
                || !marker.productive
            {
                continue;
            }
            let share = self.summary.marker_shares[marker.id as usize];
            marker.majority_generations = if share > 0.5 {
                marker.majority_generations + 1
            } else {
                0
            };
            if marker.majority_generations >= 8 && !already_fused {
                let mut rng = stream(
                    seed,
                    &[
                        key("grammar fusion"),
                        variety as u64,
                        generation as u64,
                        key(marker.category.id()),
                        marker.id as u64,
                    ],
                );
                if rng.r#gen::<f32>() < 0.005 {
                    fusions.push(marker.id);
                }
            }
        }
        for particle in fusions {
            self.fuse(particle, lexicon, morphology, stress, generation);
        }
        for category in Category::ALL {
            if !category.evolves(tense_aspect) {
                continue;
            }
            let mut rng = stream(
                seed,
                &[
                    key("grammar analogy"),
                    variety as u64,
                    generation as u64,
                    key(category.id()),
                ],
            );
            if rng.r#gen::<f32>() >= 0.002 {
                continue;
            }
            let Some(marker) = self.productive(category) else {
                continue;
            };
            if self.marker(marker).kind != MarkerKind::Bound {
                continue;
            }
            let candidates: Vec<_> = lexicon
                .living()
                .filter_map(|word| {
                    let p = word.paradigms.iter().find(|p| p.category == category)?;
                    let r = p
                        .realizations
                        .iter()
                        .filter(|r| r.retired.is_none() && r.form.is_some())
                        .max_by(|a, b| a.share.total_cmp(&b.share))?;
                    let regular = self
                        .materialize(marker, &word.form, morphology, stress, generation, r.share);
                    (!same_sound(r.form.as_ref()?, regular.form.as_ref()?, stress))
                        .then_some(word.id)
                })
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let id = candidates[index(&mut rng, candidates.len())];
            let word = lexicon.get_mut(id);
            let p = word
                .paradigms
                .iter_mut()
                .find(|p| p.category == category)
                .expect("candidate category");
            let r = p
                .realizations
                .iter_mut()
                .filter(|r| r.retired.is_none() && r.form.is_some())
                .max_by(|a, b| a.share.total_cmp(&b.share))
                .expect("candidate form");
            let regular =
                self.materialize(marker, &word.form, morphology, stress, generation, r.share);
            let before = r
                .form
                .replace(regular.form.expect("bound form"))
                .expect("old bound form");
            r.marker = marker;
            r.edge = regular.edge;
            r.history.push(GrammarEntry {
                generation,
                event: GrammarEvent::Analogy {
                    before: before.clone(),
                },
            });
            self.notice(
                generation,
                category,
                NoticeKind::Analogy {
                    lexeme: id,
                    before,
                    after: r.form.as_ref().expect("bound form").clone(),
                },
            );
        }
        self.refresh(lexicon, stress, generation);
        for marker in &mut self.markers {
            if marker.retired.is_none()
                && self.summary.marker_shares[marker.id as usize] == 0.0
                && self.summary.categories[marker.category.position()].eligible > 0
            {
                marker.retired = Some(generation);
                marker.productive = false;
                marker.history.push(GrammarEntry {
                    generation,
                    event: GrammarEvent::Retired,
                });
            }
        }
    }
    pub(crate) fn import_pair(
        &mut self,
        lexicon: &mut Lexicon,
        lexeme: LexemeId,
        from: usize,
        pair: ImportedPair,
        stress: StressRule,
        generation: u32,
    ) {
        let category = pair.category;
        let marker = self
            .markers
            .iter()
            .find(|m| {
                m.category == category
                    && m.retired.is_none()
                    && matches!(m.origin, MarkerOrigin::Imported {from:f,marker,..}
                    if f == from && marker == pair.source_marker)
            })
            .map(|m| m.id)
            .unwrap_or_else(|| {
                self.add_marker(
                    category,
                    pair.kind,
                    pair.side,
                    pair.marker_form,
                    MarkerOrigin::Imported {
                        from,
                        marker: pair.source_marker,
                        source: pair.source_marker_form,
                    },
                    false,
                    generation,
                )
            });
        let word = lexicon.get_mut(lexeme);
        let Some(p) = word.paradigms.iter_mut().find(|p| p.category == category) else {
            return;
        };
        if p.realizations
            .iter()
            .any(|r| r.retired.is_none() && r.marker == marker)
        {
            return;
        }
        let r = Realization {
            marker,
            form: pair.form,
            edge: pair.edge,
            share: 0.2,
            born: generation,
            retired: None,
            history: vec![GrammarEntry {
                generation,
                event: GrammarEvent::Imported {
                    from,
                    source: pair.source_form,
                },
            }],
        };
        Self::introduce(p, r);
        self.notice(
            generation,
            category,
            NoticeKind::ImportedPair { lexeme, from },
        );
        self.refresh(lexicon, stress, generation);
    }
    pub fn transfer_marker(
        &mut self,
        marker: u32,
        from: usize,
        lexicon: &mut Lexicon,
        morphology: &Morphology,
        stress: StressRule,
        generation: u32,
    ) {
        self.markers[marker as usize].productive = true;
        self.introduce_marker(marker, lexicon, morphology, stress, generation, 0.08);
        self.notice(
            generation,
            self.marker(marker).category,
            NoticeKind::MarkerTransfer { marker, from },
        );
        self.refresh(lexicon, stress, generation);
    }
    pub fn simplify(
        &mut self,
        lexicon: &mut Lexicon,
        stress: StressRule,
        seed: u64,
        generation: u32,
    ) {
        for category in Category::ALL {
            let mut rng = stream(
                seed,
                &[key("grammar shift"), generation as u64, key(category.id())],
            );
            if rng.r#gen::<f32>() >= 0.25 {
                continue;
            }
            let Some(marker) = self.productive(category) else {
                continue;
            };
            for word in &mut lexicon.lexemes {
                if let Some(p) = word.paradigms.iter_mut().find(|p| p.category == category) {
                    let has = p
                        .realizations
                        .iter()
                        .any(|r| r.retired.is_none() && r.marker == marker);
                    if !has {
                        continue;
                    }
                    let total: f32 = p
                        .realizations
                        .iter()
                        .filter(|r| r.retired.is_none())
                        .map(|r| r.share * if r.marker == marker { 1.2 } else { 1.0 })
                        .sum();
                    for r in p.realizations.iter_mut().filter(|r| r.retired.is_none()) {
                        r.share = r.share * if r.marker == marker { 1.2 } else { 1.0 } / total;
                    }
                }
            }
        }
        self.refresh(lexicon, stress, generation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Livelihood, Variety};

    #[test]
    fn particle_fusion_keeps_its_current_sounds_and_separate_competitor() {
        let mut profile = SoundProfile::base();
        profile.grammar = GrammarPrior::fixed(GrammarDesign {
            plural: GrammarChoice::Particle,
            past: GrammarChoice::None,
            ..GrammarDesign::default()
        });
        profile.stress = Some(StressRule::Initial);
        profile.phonotactics.open_medial = false;
        let mut variety = Variety::found(9, &profile, Livelihood::Farming, crate::Ethos::default());
        variety.minimal = MinimalWord::Syllable;
        let particle = variety
            .grammar
            .markers
            .iter()
            .find(|m| m.category == Category::Plural)
            .unwrap()
            .id;
        variety.grammar.markers[particle as usize].form = Form::from_ipa("mika").unwrap();
        let apocope = crate::catalog()
            .into_iter()
            .find(|law| law.id == "apocope")
            .unwrap();
        let stress = variety.stress();
        variety
            .grammar
            .apply_law(&mut variety.lexicon, &apocope, variety.minimal, stress, 7);
        assert_eq!(variety.grammar.marker(particle).form.ipa(), "mik");
        let fused = variety.grammar.fuse(
            particle,
            &mut variety.lexicon,
            &variety.morphology,
            stress,
            8,
        );
        let word = variety.lexicon.word_for(by_id("person").unwrap()).unwrap();
        let p = word
            .paradigms
            .iter()
            .find(|p| p.category == Category::Plural)
            .unwrap();
        let separate = p
            .realizations
            .iter()
            .find(|r| r.marker == particle && r.retired.is_none())
            .unwrap();
        let bound = p
            .realizations
            .iter()
            .find(|r| r.marker == fused && r.retired.is_none())
            .unwrap();
        assert_eq!(variety.grammar.surface(&word.form, separate).len(), 2);
        assert_eq!(variety.grammar.surface(&word.form, bound).len(), 1);
        assert!(
            bound
                .form
                .as_ref()
                .unwrap()
                .segs
                .ends_with(&Form::from_ipa("mik").unwrap().segs)
        );
        assert!((separate.share + bound.share - 1.0).abs() < 1e-6);
        assert!(
            matches!(variety.grammar.events.last().unwrap().event,NoticeKind::Fusion {particle:p,marker:m} if p==particle && m==fused)
        );
    }

    #[test]
    fn bound_forms_carry_free_lexical_stress_across_prefixes_and_hiatus() {
        let variety = Variety::found(
            9,
            &SoundProfile::base(),
            Livelihood::Farming,
            crate::Ethos::default(),
        );
        let mut grammar = Grammar::default();
        let marker = grammar.add_marker(
            Category::Plural,
            MarkerKind::Bound,
            Side::Prefix,
            Form::from_ipa("mi").unwrap(),
            MarkerOrigin::Founding,
            true,
            0,
        );
        let mut base = Form::from_ipa("akapa").unwrap();
        base.stress = Some(1);
        let marked =
            grammar.materialize(marker, &base, &variety.morphology, StressRule::Free, 0, 1.0);
        let form = marked.form.unwrap();
        assert_eq!(form.ipa(), "mikapa");
        assert_eq!(marked.edge, 2);
        assert_eq!(form.stress, Some(1));
        assert_eq!(form.ipa_stressed(StressRule::Free), "miˈkapa");

        let unaccented = Form::from_ipa("kata").unwrap();
        let marked = grammar.materialize(
            marker,
            &unaccented,
            &variety.morphology,
            StressRule::Free,
            0,
            1.0,
        );
        assert_eq!(marked.form.unwrap().stress, Some(1));
        let predictable = grammar.materialize(
            marker,
            &unaccented,
            &variety.morphology,
            StressRule::Penult,
            0,
            1.0,
        );
        assert_eq!(predictable.form.unwrap().stress, None);

        grammar.markers[marker as usize].side = Side::Suffix;
        grammar.markers[marker as usize].form = Form::default();
        base.boundaries = vec![2];
        let zero =
            grammar.materialize(marker, &base, &variety.morphology, StressRule::Free, 0, 1.0);
        assert_eq!(zero.edge, base.segs.len());
        assert_eq!(zero.form.unwrap(), base);
    }

    #[test]
    fn stress_only_laws_record_grammar_history_and_refresh_derived_contrast() {
        let mut lexicon = Lexicon::found(std::iter::empty());
        let concept = by_id("person").unwrap();
        let mut base = Form::from_ipa("kata").unwrap();
        base.stress = Some(0);
        let mut marked = base.clone();
        marked.stress = Some(1);
        assert!(!same_sound(&base, &marked, StressRule::Free));
        assert!(same_sound(&base, &marked, StressRule::Initial));
        assert!(same_sound(
            &Form::from_ipa("kata").unwrap(),
            &base,
            StressRule::Free,
        ));
        let word = lexicon.coin(base, crate::lexicon::Origin::Founding, concept, 0);
        lexicon.slot_mut(concept).introduce(word, 1.0);
        let mut grammar = Grammar::default();
        let bound = grammar.add_marker(
            Category::Plural,
            MarkerKind::Bound,
            Side::Suffix,
            Form::default(),
            MarkerOrigin::Founding,
            true,
            0,
        );
        let mut particle_form = Form::from_ipa("mika").unwrap();
        particle_form.stress = Some(1);
        let particle = grammar.add_marker(
            Category::Plural,
            MarkerKind::Particle,
            Side::Suffix,
            particle_form.clone(),
            MarkerOrigin::Founding,
            false,
            0,
        );
        lexicon.get_mut(word).paradigms.push(Paradigm {
            category: Category::Plural,
            realizations: vec![Realization {
                marker: bound,
                edge: marked.segs.len(),
                form: Some(marked.clone()),
                share: 1.0,
                born: 0,
                retired: None,
                history: vec![GrammarEntry {
                    generation: 0,
                    event: GrammarEvent::Created,
                }],
            }],
        });
        grammar.refresh(&lexicon, StressRule::Free, 0);
        assert_eq!(grammar.summary.contrast_retention, 1.0);
        let law = crate::catalog()
            .into_iter()
            .find(|law| law.id == "initial-stress")
            .unwrap();
        grammar.apply_law(
            &mut lexicon,
            &law,
            MinimalWord::Syllable,
            StressRule::Free,
            1,
        );
        assert_eq!(grammar.summary.contrast_retention, 0.0);
        assert_eq!(grammar.summary.how_synthetic, 0.0);
        let realization = &lexicon.get(word).paradigms[0].realizations[0];
        assert_eq!(realization.form.as_ref().unwrap(), &marked);
        assert_eq!(realization.edge, marked.segs.len());
        assert!(matches!(
            &realization.history.last().unwrap().event,
            GrammarEvent::SoundLaw { law: "initial-stress", before } if before == &marked
        ));
        assert_eq!(grammar.marker(particle).form, particle_form);
        assert!(matches!(
            &grammar.marker(particle).history.last().unwrap().event,
            GrammarEvent::SoundLaw { law: "initial-stress", before } if before == &particle_form
        ));
    }

    #[test]
    fn imported_particles_are_shared_and_fusion_cannot_make_them_productive() {
        use crate::change::{Env, Matcher, Rewrite, SoundChange};

        let variety = Variety::found(
            9,
            &SoundProfile::base(),
            Livelihood::Farming,
            crate::Ethos::default(),
        );
        let mut grammar = Grammar::default();
        let native = grammar.add_marker(
            Category::Plural,
            MarkerKind::Bound,
            Side::Suffix,
            Form::from_ipa("u").unwrap(),
            MarkerOrigin::Founding,
            true,
            0,
        );
        let mut lexicon = Lexicon::found(std::iter::empty());
        let words: Vec<_> = [("person", "kata"), ("stone", "pata")]
            .into_iter()
            .map(|(id, ipa)| {
                let concept = by_id(id).unwrap();
                let word = lexicon.coin(
                    Form::from_ipa(ipa).unwrap(),
                    crate::lexicon::Origin::Founding,
                    concept,
                    0,
                );
                lexicon.slot_mut(concept).introduce(word, 1.0);
                word
            })
            .collect();
        grammar.sync(&mut lexicon, &variety.morphology, StressRule::Initial, 0);
        for &word in &words {
            grammar.import_pair(
                &mut lexicon,
                word,
                2,
                ImportedPair {
                    category: Category::Plural,
                    source_marker: 7,
                    kind: MarkerKind::Particle,
                    side: Side::Suffix,
                    marker_form: Form::from_ipa("mikant").unwrap(),
                    source_marker_form: Form::from_ipa("miːkant").unwrap(),
                    form: None,
                    edge: 0,
                    source_form: Form::from_ipa("miːkant").unwrap(),
                },
                StressRule::Initial,
                1,
            );
        }
        let particle = lexicon.get(words[0]).paradigms[0]
            .realizations
            .iter()
            .find(|r| r.marker != native)
            .unwrap()
            .marker;
        assert!(matches!(
            &grammar.marker(particle).origin,
            MarkerOrigin::Imported { source, .. } if source.ipa() == "miːkant"
        ));
        for &word in &words {
            let realization = lexicon.get(word).paradigms[0]
                .realizations
                .iter()
                .find(|r| r.marker == particle)
                .unwrap();
            assert_eq!(realization.form, None);
            assert!(grammar.audible(&lexicon.get(word).form, realization, StressRule::Initial));
        }
        let law = Law {
            id: "test-final-consonant-loss",
            label: "A final consonant disappears",
            rules: vec![SoundChange {
                id: "test-final-consonant-loss".into(),
                target: Matcher::AnyConsonant,
                result: Rewrite::Delete,
                left: Env::Any,
                right: Env::WordEdge,
            }],
            commonness: 1.0,
            stress: None,
        };
        grammar.apply_law(
            &mut lexicon,
            &law,
            MinimalWord::Syllable,
            StressRule::Initial,
            2,
        );
        assert_eq!(grammar.marker(particle).form.ipa(), "mikan");
        let historical_surface = grammar.surface_at(
            &lexicon.get(words[0]).form,
            &lexicon.get(words[0]).paradigms[0].realizations[1],
            1,
        );
        assert_eq!(historical_surface[1].ipa(), "mikant");
        let fused = grammar.fuse(
            particle,
            &mut lexicon,
            &variety.morphology,
            StressRule::Initial,
            3,
        );
        assert!(!grammar.marker(particle).productive);
        assert!(!grammar.marker(fused).productive);
        assert_eq!(grammar.productive(Category::Plural), Some(native));
    }

    #[test]
    fn daughters_keep_founding_orders_through_grammar_evolution() {
        for order in [WordOrder::SOV, WordOrder::SVO, WordOrder::VSO] {
            for possessor in [PossessorOrder::Before, PossessorOrder::After] {
                let mut profile = SoundProfile::base();
                profile.grammar.order = Some(order);
                profile.grammar.possessor = Some(possessor);
                let parent =
                    Variety::found(42, &profile, Livelihood::Farming, crate::Ethos::default());
                let mut daughter = parent.fork(0, 5);
                let stress = daughter.stress();
                for generation in 6..40 {
                    daughter.grammar.evolve(
                        42,
                        1,
                        generation,
                        &mut daughter.lexicon,
                        &daughter.morphology,
                        stress,
                        1000,
                        true,
                    );
                }
                assert_eq!(
                    (daughter.grammar.order, daughter.grammar.possessor),
                    (order, possessor)
                );
                assert_eq!(
                    (parent.grammar.order, parent.grammar.possessor),
                    (order, possessor)
                );
            }
        }
    }
}

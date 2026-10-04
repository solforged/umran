//! Word-level vowel agreement, its emergence from assimilation, and its loss.
use crate::grammar::{GrammarEntry, GrammarEvent, MarkerKind, Side};
use crate::rng::{key, stream};
use crate::{CATALOG, Cause, Entry, Event, Form, LexemeId, Origin, PhonemeId, Variety, World};
use rand::Rng;
use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Feature {
    Backness,
    Rounding,
    Atr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    /// The first paired stem vowel controls a word; affixes follow the stem.
    Word,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Harmony {
    pub feature: Feature,
    pub domain: Domain,
    pub since: u32,
    pub lost: Option<u32>,
    /// Post-gain loans admitted without regularizing their stem vowels.
    pub disharmonic_loans: BTreeSet<LexemeId>,
    loan_seed: u64,
    checked_words: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    Assimilation { generation: u32, law: &'static str },
    ContrastMerger { law: &'static str },
    LexicalAttrition,
    Contact { donor: usize, cause: Option<Cause> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Notice {
    pub generation: u32,
    pub feature: Feature,
    pub gained: bool,
    pub trigger: Trigger,
}

// Only attested catalog pairs participate. Unpaired vowels are transparent.
// ATR is represented by these tense/lax pairs, not inferred from height alone.
static BACKNESS: LazyLock<Vec<(PhonemeId, PhonemeId)>> =
    LazyLock::new(|| pairs(&[("y", "u"), ("ø", "o"), ("a", "ɑ")]));
static ROUNDING: LazyLock<Vec<(PhonemeId, PhonemeId)>> =
    LazyLock::new(|| pairs(&[("i", "y"), ("e", "ø")]));
static ATR: LazyLock<Vec<(PhonemeId, PhonemeId)>> =
    LazyLock::new(|| pairs(&[("ɪ", "i"), ("ʊ", "u"), ("ɛ", "e"), ("ɔ", "o")]));

fn pairs(symbols: &[(&str, &str)]) -> Vec<(PhonemeId, PhonemeId)> {
    symbols
        .iter()
        .map(|(a, b)| (CATALOG.id_by_ipa(a).unwrap(), CATALOG.id_by_ipa(b).unwrap()))
        .collect()
}

impl Feature {
    pub const ALL: [Self; 3] = [Self::Backness, Self::Rounding, Self::Atr];
    pub fn id(self) -> &'static str {
        match self {
            Self::Backness => "backness",
            Self::Rounding => "rounding",
            Self::Atr => "atr",
        }
    }
    pub fn law(self) -> &'static str {
        match self {
            Self::Backness => "harmony-backness",
            Self::Rounding => "harmony-rounding",
            Self::Atr => "harmony-atr",
        }
    }
    pub fn precursor(self) -> &'static str {
        match self {
            Self::Backness => "umlaut",
            Self::Rounding => "rounding-assimilation",
            Self::Atr => "atr-assimilation",
        }
    }
    fn pairs(self) -> &'static [(PhonemeId, PhonemeId)] {
        match self {
            Self::Backness => &BACKNESS,
            Self::Rounding => &ROUNDING,
            Self::Atr => &ATR,
        }
    }
    fn value(self, phone: PhonemeId) -> Option<bool> {
        self.pairs().iter().find_map(|&(a, b)| {
            if phone == a {
                Some(false)
            } else if phone == b {
                Some(true)
            } else {
                None
            }
        })
    }
    fn agreeing(self, phone: PhonemeId, value: bool) -> PhonemeId {
        self.pairs()
            .iter()
            .find_map(|&(a, b)| (phone == a || phone == b).then_some(if value { b } else { a }))
            .unwrap_or(phone)
    }
    fn controller(self, form: &Form) -> Option<bool> {
        form.phones().find_map(|p| self.value(p))
    }
    pub fn disharmonic(self, form: &Form) -> bool {
        self.controller(form).is_some_and(|value| {
            form.phones()
                .any(|p| self.value(p).is_some_and(|v| v != value))
        })
    }
    /// Rewrites only phone identity: length, tone, stress and boundaries survive.
    /// Returns the old form only when changed; unchanged words allocate nothing.
    fn agree_range(
        self,
        form: &mut Form,
        range: std::ops::Range<usize>,
        value: bool,
    ) -> Option<Form> {
        let first = range
            .clone()
            .find(|&i| self.agreeing(form.segs[i].phone, value) != form.segs[i].phone)?;
        let before = form.clone();
        for seg in &mut form.segs[first..range.end] {
            seg.phone = self.agreeing(seg.phone, value);
        }
        Some(before)
    }
    pub fn apply(self, form: &mut Form) -> Option<Form> {
        let value = self.controller(form)?;
        self.agree_range(form, 0..form.segs.len(), value)
    }
    /// New realizations need no historical copy of their pre-agreement form.
    pub fn realize_bound(self, form: &mut Form, edge: usize, side: Side, base: &Form) {
        let edge = edge.min(form.segs.len());
        let (range, value) = match side {
            Side::Prefix => (0..edge, self.controller(base)),
            Side::Suffix => (
                edge..form.segs.len(),
                base.segs.iter().rev().find_map(|s| self.value(s.phone)),
            ),
        };
        if let Some(value) = value {
            for seg in &mut form.segs[range] {
                seg.phone = self.agreeing(seg.phone, value);
            }
        }
    }
    /// Attached markers agree even on exceptional loans without repairing the
    /// loan stem. Suffixes follow its last paired vowel, prefixes its first.
    pub fn agree_bound(
        self,
        form: &mut Form,
        edge: usize,
        side: Side,
        base: &Form,
    ) -> Option<Form> {
        let edge = edge.min(form.segs.len());
        let (range, value) = match side {
            Side::Prefix => (0..edge, self.controller(base)),
            Side::Suffix => (
                edge..form.segs.len(),
                base.segs.iter().rev().find_map(|s| self.value(s.phone)),
            ),
        };
        self.agree_range(form, range, value?)
    }
    /// Marginal vowels still contrast: the borrowing threshold is not a
    /// phonemicity test, nor does a small lexicon need an accidental minimal pair.
    pub fn has_contrast(self, variety: &Variety) -> bool {
        let mut present = 0;
        for (form, share) in variety.grammar.forms(&variety.lexicon) {
            if share > 0.0 {
                present |= self.phone_mask(form.phones());
            }
        }
        self.mask_has_pair(present)
    }
    fn phone_mask(self, phones: impl Iterator<Item = PhonemeId>) -> u8 {
        let mut mask = 0;
        for phone in phones {
            for (i, &(a, b)) in self.pairs().iter().enumerate() {
                if phone == a {
                    mask |= 1 << (2 * i);
                } else if phone == b {
                    mask |= 2 << (2 * i);
                }
            }
        }
        mask
    }
    fn mask_has_pair(self, mask: u8) -> bool {
        (0..self.pairs().len()).any(|i| mask & (3 << (2 * i)) == 3 << (2 * i))
    }
    /// Only attested, productive bound realizations count, not hypothetical
    /// alternants regenerated from a marker after a law has erased them.
    fn has_affix_contrast(self, variety: &Variety) -> bool {
        let mut masks = vec![0; variety.grammar.markers.len()];
        for word in variety.lexicon.living() {
            for r in word.paradigms.iter().flat_map(|p| &p.realizations) {
                let marker = &variety.grammar.markers[r.marker as usize];
                if r.retired.is_some()
                    || r.share <= 0.0
                    || marker.retired.is_some()
                    || !marker.productive
                    || marker.kind != MarkerKind::Bound
                {
                    continue;
                }
                let Some(form) = &r.form else { continue };
                let edge = r.edge.min(form.segs.len());
                let affix = match marker.side {
                    Side::Prefix => &form.segs[..edge],
                    Side::Suffix => &form.segs[edge..],
                };
                let mask = &mut masks[r.marker as usize];
                *mask |= self.phone_mask(affix.iter().map(|s| s.phone));
                if self.mask_has_pair(*mask) {
                    return true;
                }
            }
        }
        false
    }
    fn law_changed_vowels(self, variety: &Variety, law: &str, generation: u32) -> bool {
        let changed = |before: &Form, after: &Form| {
            !before
                .phones()
                .filter(|&p| self.value(p).is_some())
                .eq(after.phones().filter(|&p| self.value(p).is_some()))
        };
        variety.lexicon.living().any(|word| {
            word.log.last().is_some_and(|entry| {
                entry.generation == generation
                    && matches!(&entry.event, Event::SoundLaw { law: id, before }
                        if *id == law && changed(before, &word.form))
            }) || word.paradigms.iter().flat_map(|p| &p.realizations).any(|r| {
                r.retired.is_none() && r.history.last().is_some_and(|entry| {
                    entry.generation == generation
                        && matches!(&entry.event, GrammarEvent::SoundLaw { law: id, before }
                            if *id == law && r.form.as_ref().is_some_and(|after| changed(before, after)))
                })
            })
        })
            || variety.grammar.markers.iter().any(|marker| {
                marker.kind == MarkerKind::Particle
                    && marker.retired.is_none()
                    && marker.history.last().is_some_and(|entry| {
                        entry.generation == generation
                            && matches!(&entry.event, GrammarEvent::SoundLaw { law: id, before }
                                if *id == law && changed(before, &marker.form))
                    })
            })
    }
    /// Productive surface alternatives, rather than an exhaustive paradigm.
    pub fn alternants(self, form: &Form) -> Vec<Form> {
        let mut out = Vec::new();
        for value in [false, true] {
            let mut next = form.clone();
            for seg in &mut next.segs {
                seg.phone = self.agreeing(seg.phone, value);
            }
            if !out.contains(&next) {
                out.push(next);
            }
        }
        if out.len() > 1 { out } else { Vec::new() }
    }
}

impl Harmony {
    pub fn new(feature: Feature, since: u32, loan_seed: u64) -> Self {
        Self {
            feature,
            domain: Domain::Word,
            since,
            lost: None,
            disharmonic_loans: BTreeSet::new(),
            loan_seed,
            checked_words: 0,
        }
    }
    pub fn active(&self) -> bool {
        self.lost.is_none()
    }
}

impl Variety {
    pub(crate) fn harmonize_words(&mut self, generation: u32) {
        let Some(harmony) = self.harmony.as_mut().filter(|h| h.active()) else {
            self.grammar.harmony = None;
            return;
        };
        let feature = harmony.feature;
        self.grammar.harmony = Some(feature);
        let law = feature.law();
        for word in &mut self.lexicon.lexemes {
            if word.obsolete.is_some() {
                continue;
            }
            if word.id.0 as usize >= harmony.checked_words
                && word.born > harmony.since
                && let Origin::Borrowed { from, source, .. } = word.origin
                && feature.disharmonic(&word.form)
            {
                let mut rng = stream(
                    harmony.loan_seed,
                    &[
                        key("harmony loan exemption"),
                        word.id.0.into(),
                        from as u64,
                        source.0.into(),
                    ],
                );
                if rng.r#gen::<f32>() < 0.65 {
                    harmony.disharmonic_loans.insert(word.id);
                }
            }
            let exempt = harmony.disharmonic_loans.contains(&word.id);
            if !exempt && let Some(before) = feature.apply(&mut word.form) {
                word.log.push(Entry {
                    generation,
                    event: Event::SoundLaw { law, before },
                });
            }
            for r in word
                .paradigms
                .iter_mut()
                .flat_map(|p| &mut p.realizations)
                .filter(|r| r.retired.is_none())
            {
                let Some(form) = &mut r.form else {
                    continue;
                };
                let marker = &self.grammar.markers[r.marker as usize];
                let before = if exempt {
                    feature.agree_bound(form, r.edge, marker.side, &word.form)
                } else {
                    // Use the stored stem, not the productive bare word, to retain
                    // earlier umlaut and other grammatical stem alternations.
                    let edge = r.edge.min(form.segs.len());
                    let stem = match marker.side {
                        Side::Prefix => edge..form.segs.len(),
                        Side::Suffix => 0..edge,
                    };
                    let value = form.segs[stem].iter().find_map(|s| feature.value(s.phone));
                    value.and_then(|v| feature.agree_range(form, 0..form.segs.len(), v))
                };
                if let Some(before) = before {
                    r.history.push(GrammarEntry {
                        generation,
                        event: GrammarEvent::SoundLaw { law, before },
                    });
                }
            }
        }
        harmony.checked_words = self.lexicon.lexemes.len();
        for marker in &mut self.grammar.markers {
            if marker.kind == MarkerKind::Particle
                && marker.retired.is_none()
                && let Some(before) = feature.apply(&mut marker.form)
            {
                marker.history.push(GrammarEntry {
                    generation,
                    event: GrammarEvent::SoundLaw { law, before },
                });
            }
        }
    }
}

impl World {
    fn gain_harmony(&mut self, v: usize, feature: Feature, trigger: Trigger) {
        let seed = stream(
            self.seed,
            &[key("harmony loan seed"), v as u64, self.generation.into()],
        )
        .r#gen();
        let variety = &mut self.varieties[v];
        variety.harmony = Some(Harmony::new(feature, self.generation, seed));
        variety.harmony_events.push(Notice {
            generation: self.generation,
            feature,
            gained: true,
            trigger,
        });
        variety.harmonize_words(self.generation);
        variety.sync_grammar(self.generation);
        self.harmonize_names(v);
    }
    fn lose_harmony(&mut self, v: usize, trigger: Trigger) {
        let variety = &mut self.varieties[v];
        let harmony = variety.harmony.as_mut().expect("active harmony");
        harmony.lost = Some(self.generation);
        variety.grammar.harmony = None;
        variety.harmony_events.push(Notice {
            generation: self.generation,
            feature: harmony.feature,
            gained: false,
            trigger,
        });
    }
    pub(crate) fn check_harmony_contrast(&mut self, v: usize) {
        if self.varieties[v]
            .harmony
            .as_ref()
            .is_some_and(|h| h.active() && !h.feature.has_contrast(&self.varieties[v]))
        {
            self.lose_harmony(v, Trigger::LexicalAttrition);
        }
    }
    /// Check attrition before the law, so a later consonant law cannot take
    /// credit for an already missing vowel. Capture productive alternation
    /// before grammar's regular sound change and before harmony can restore it.
    pub(crate) fn harmony_before_law(&mut self, v: usize) -> Option<(Feature, bool)> {
        self.check_harmony_contrast(v);
        self.varieties[v]
            .harmony
            .as_ref()
            .filter(|h| h.active())
            .map(|h| (h.feature, h.feature.has_affix_contrast(&self.varieties[v])))
    }
    pub(crate) fn harmony_after_law(
        &mut self,
        v: usize,
        law: &'static str,
        before: Option<(Feature, bool)>,
    ) {
        let Some((feature, affixes)) = before else {
            return;
        };
        let variety = &self.varieties[v];
        if !feature.has_contrast(variety) || (affixes && !feature.has_affix_contrast(variety)) {
            let trigger = if feature.law_changed_vowels(variety, law, self.generation) {
                Trigger::ContrastMerger { law }
            } else {
                Trigger::LexicalAttrition
            };
            self.lose_harmony(v, trigger);
        }
    }
    pub(crate) fn evolve_harmony(&mut self, spoken: &[bool]) {
        if self.params.harmony_rate <= 0.0 {
            return;
        }
        // Decisions use the same donor snapshot, not variety iteration order.
        let harmonic: Vec<_> = self
            .varieties
            .iter()
            .map(|v| v.harmony.as_ref().is_some_and(Harmony::active))
            .collect();
        for (v, &spoken) in spoken.iter().enumerate() {
            if !spoken {
                continue;
            }
            self.check_harmony_contrast(v);
            if self.varieties[v]
                .harmony
                .as_ref()
                .is_some_and(Harmony::active)
            {
                let donor = self.varieties[v]
                    .grammar
                    .contact_generations
                    .iter()
                    .find_map(|(&donor, &duration)| {
                        (duration >= 12 && !harmonic[donor]).then_some((donor, duration))
                    });
                if let Some((donor, duration)) = donor {
                    let mut rng = stream(
                        self.seed,
                        &[
                            key("harmony contact loss"),
                            v as u64,
                            donor as u64,
                            self.generation.into(),
                        ],
                    );
                    if rng.r#gen::<f32>() < (0.015 * (duration - 11) as f32).min(0.3) {
                        let cause = self.contacts.iter().find_map(|c| {
                            let a = self.communities[c.a].variety;
                            let b = self.communities[c.b].variety;
                            (((a == v && b == donor) || (b == v && a == donor))
                                && c.intensity >= 0.75)
                                .then_some(c.cause)
                                .flatten()
                        });
                        self.lose_harmony(v, Trigger::Contact { donor, cause });
                    }
                }
                continue;
            }
            let variety = &self.varieties[v];
            let after = variety.harmony.as_ref().and_then(|h| h.lost);
            for feature in Feature::ALL {
                let recent = variety
                    .laws
                    .iter()
                    .rev()
                    .find(|&&(g, law)| {
                        law == feature.precursor()
                            && self.generation.saturating_sub(g) <= 24
                            && after.is_none_or(|lost| g > lost)
                    })
                    .copied();
                let Some((generation, law)) = recent else {
                    continue;
                };
                if !feature.has_contrast(variety) {
                    continue;
                }
                let mut rng = stream(
                    self.seed,
                    &[
                        key("harmony gain"),
                        v as u64,
                        self.generation.into(),
                        key(feature.id()),
                    ],
                );
                if rng.r#gen::<f32>() < self.params.harmony_rate {
                    self.gain_harmony(v, feature, Trigger::Assimilation { generation, law });
                    break;
                }
            }
        }
    }
    pub(crate) fn harmonize_names(&mut self, v: usize) {
        let Some(feature) = self.varieties[v]
            .harmony
            .as_ref()
            .filter(|h| h.active())
            .map(|h| h.feature)
        else {
            return;
        };
        let generation = self.generation;
        self.change_names(v, |name| {
            if let Some(before) = feature.apply(&mut name.form) {
                name.log.push(Entry {
                    generation,
                    event: Event::SoundLaw {
                        law: feature.law(),
                        before,
                    },
                });
            }
        });
    }
}

/// Precursors can occur as ordinary local sound laws; productive harmony is
/// recorded under zero-weight catalog ids and is not spread as an empty law.
pub(crate) fn catalog_laws() -> [crate::Law; 5] {
    use crate::{Env, Matcher, Rewrite, SoundChange};
    let precursor = |feature: Feature, label| {
        let mut rules = Vec::new();
        for &(from, to) in feature.pairs() {
            for &(_, trigger) in feature.pairs() {
                rules.push(SoundChange {
                    id: format!("{}.{}", feature.precursor(), rules.len()),
                    target: Matcher::Phone(from),
                    result: Rewrite::Phone(to),
                    left: Env::Any,
                    right: Env::FollowingSyllableVowel(Matcher::Phone(trigger)),
                });
            }
        }
        crate::Law {
            id: feature.precursor(),
            label,
            rules,
            commonness: 0.25,
            stress: None,
        }
    };
    let record = |feature: Feature, label| crate::Law {
        id: feature.law(),
        label,
        rules: Vec::new(),
        commonness: 0.0,
        stress: None,
    };
    [
        precursor(
            Feature::Rounding,
            "Vowels round before a rounded vowel in the next syllable",
        ),
        precursor(
            Feature::Atr,
            "Vowels advance before an advanced vowel in the next syllable",
        ),
        record(Feature::Backness, "Word vowels agree in backness"),
        record(Feature::Rounding, "Word vowels agree in rounding"),
        record(Feature::Atr, "Word vowels agree in tongue-root position"),
    ]
}

#[cfg(test)]
mod tests;

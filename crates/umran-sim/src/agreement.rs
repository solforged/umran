//! Verb person agreement by Givón's cycle: a free subject pronoun, said
//! beside its verb, weakens into a clitic; the clitic fuses into a person
//! affix; audible affixes let the free pronoun drop; worn affixes give the
//! pronoun its place back, and the renewed pronouns can start over.
use super::*;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgreementStage {
    #[default]
    None,
    Clitic,
    Affix,
}
impl AgreementStage {
    pub fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Clitic => "clitic",
            Self::Affix => "affix",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgreementKind {
    Clitics,
    Affixes,
    ProDrop,
    Loss,
}
impl AgreementKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Clitics => "clitics",
            Self::Affixes => "affixes",
            Self::ProDrop => "pro-drop",
            Self::Loss => "loss",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgreementNotice {
    pub generation: u32,
    pub kind: AgreementKind,
}

/// One person cell's agreement form. It is cut from the pronoun at
/// cliticization and then lives its own life under the sound laws; the
/// pronoun's later renewal does not touch it.
#[derive(Clone, Debug, PartialEq)]
pub struct AgreementForm {
    /// A `pronouns::CELLS` concept id.
    pub cell: &'static str,
    pub form: Form,
    pub source: LexemeId,
    pub born: u32,
    pub retired: Option<u32>,
    pub history: Vec<GrammarEntry>,
}
impl AgreementForm {
    pub fn audible(&self) -> bool {
        !self.form.segs.is_empty()
    }
    /// The form after `generation`, before later changes.
    pub fn form_at(&self, generation: u32) -> &Form {
        historical(&self.form, &self.history, generation)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Agreement {
    pub stage: AgreementStage,
    pub pro_drop: bool,
    /// The verb's side the subject stands on: proclitics and prefixes when
    /// the subject precedes the verb, enclitics and suffixes when it follows.
    pub side: Side,
    pub cells: Vec<AgreementForm>,
    /// Generation the current stage began; after a loss, the loss itself.
    pub since: Option<u32>,
    pub events: Vec<AgreementNotice>,
}
impl Default for Agreement {
    fn default() -> Self {
        Self {
            stage: AgreementStage::None,
            pro_drop: false,
            side: Side::Prefix,
            cells: Vec::new(),
            since: None,
            events: Vec::new(),
        }
    }
}

/// Generations a clitic serves before it can fuse.
const FUSION_WAIT: u32 = 8;
/// Generations after a loss before the renewed pronouns can cliticize.
const RESTART_WAIT: u32 = 12;

impl Agreement {
    pub fn living(&self) -> impl Iterator<Item = &AgreementForm> {
        self.cells.iter().filter(|c| c.retired.is_none())
    }
    /// The living form for a cell. Respectful singular address agrees as
    /// the plural it came from, so it has no cell of its own.
    pub fn cell(&self, cell: &str) -> Option<&AgreementForm> {
        let cell = if cell == "2sg-polite" { "2pl" } else { cell };
        self.living().find(|c| c.cell == cell)
    }
    /// Audible and unlike every other living cell.
    pub fn distinct(&self, cell: &AgreementForm, stress: StressRule) -> bool {
        cell.audible()
            && !self
                .living()
                .any(|other| other.cell != cell.cell && same_sound(&other.form, &cell.form, stress))
    }
    fn distinct_count(&self, stress: StressRule) -> (usize, usize) {
        let living = self.living().count();
        let distinct = self.living().filter(|c| self.distinct(c, stress)).count();
        (distinct, living)
    }
    fn record(&mut self, generation: u32, kind: AgreementKind) {
        self.events.push(AgreementNotice { generation, kind });
    }
}

/// The person cells a language expresses, inclusive and exclusive in place
/// of a plain plural where it has the distinction.
fn base_cells(lexicon: &Lexicon) -> impl Iterator<Item = &'static str> + '_ {
    let clusivity = by_id("1pl-inclusive").is_some_and(|c| lexicon.word_for(c).is_some());
    crate::pronouns::CELLS
        .into_iter()
        .filter(move |(_, _, cell)| match *cell {
            "1pl" => !clusivity,
            "1pl-inclusive" | "1pl-exclusive" => clusivity,
            "2sg-polite" => false,
            _ => true,
        })
        .map(|(_, _, cell)| cell)
}

/// The syllable nearest the verb: onset and nucleus of the first for a
/// proclitic, the whole last syllable for an enclitic; a monosyllable whole.
/// Length and tone stay; a clitic carries no lexical stress.
fn clitic_form(form: &Form, side: Side) -> Form {
    let syllables = form.syllables();
    let range = match (syllables.first(), syllables.last()) {
        (Some(first), Some(last)) if syllables.len() > 1 => match side {
            Side::Prefix => first.onset.start..first.nucleus + 1,
            Side::Suffix => last.onset.start..last.coda.end,
        },
        _ => 0..form.segs.len(),
    };
    Form {
        segs: form.segs[range].to_vec(),
        boundaries: Vec::new(),
        stress: None,
    }
}

fn pairwise_same(forms: &[Form], stress: StressRule) -> bool {
    forms
        .iter()
        .enumerate()
        .any(|(i, a)| forms[..i].iter().any(|b| same_sound(a, b, stress)))
}

impl Grammar {
    pub fn agreement(&self) -> &Agreement {
        &self.agreement
    }
    fn agreement_side(order: WordOrder) -> Side {
        match order {
            WordOrder::SOV | WordOrder::SVO => Side::Prefix,
            WordOrder::VSO => Side::Suffix,
        }
    }
    /// The cells that could cliticize now: every living base cell has a
    /// pronoun of two or more segments and no two sound alike. Where the
    /// syllables nearest the verb would not tell the persons apart, the
    /// whole pronouns cliticize.
    fn clitic_candidates(
        &self,
        lexicon: &Lexicon,
        stress: StressRule,
    ) -> Option<Vec<(&'static str, LexemeId, Form)>> {
        let side = Self::agreement_side(self.order);
        let mut cells = Vec::with_capacity(7);
        for cell in base_cells(lexicon) {
            let Some(word) = lexicon.word_for(by_id(cell)?) else {
                continue;
            };
            if word.form.segs.len() < 2 {
                return None;
            }
            cells.push((cell, word.id, &word.form));
        }
        if cells.len() < 2 {
            return None;
        }
        let whole: Vec<Form> = cells
            .iter()
            .map(|(_, _, form)| Form {
                segs: form.segs.clone(),
                boundaries: Vec::new(),
                stress: None,
            })
            .collect();
        if pairwise_same(&whole, stress) {
            return None;
        }
        let mut clitics: Vec<Form> = cells
            .iter()
            .map(|(_, _, form)| clitic_form(form, side))
            .collect();
        if pairwise_same(&clitics, stress) {
            clitics = whole;
        }
        Some(
            cells
                .into_iter()
                .zip(clitics)
                .map(|((cell, id, _), form)| (cell, id, form))
                .collect(),
        )
    }
    /// One draw a generation on the `agreement` stream, after every other
    /// grammar draw, for whichever step of the cycle is open.
    pub(super) fn evolve_agreement(
        &mut self,
        seed: u64,
        variety: usize,
        generation: u32,
        lexicon: &Lexicon,
        stress: StressRule,
        params: &crate::Params,
    ) {
        let agreement = &self.agreement;
        let (rate, candidates) = match agreement.stage {
            AgreementStage::None => {
                if agreement
                    .since
                    .is_some_and(|lost| generation < lost + RESTART_WAIT)
                {
                    return;
                }
                let Some(candidates) = self.clitic_candidates(lexicon, stress) else {
                    return;
                };
                (params.agreement_rate, Some(candidates))
            }
            AgreementStage::Clitic => {
                if agreement
                    .since
                    .is_none_or(|since| generation < since + FUSION_WAIT)
                {
                    return;
                }
                (params.agreement_fusion_rate, None)
            }
            AgreementStage::Affix => {
                let (distinct, living) = agreement.distinct_count(stress);
                if agreement.pro_drop || distinct * 3 < living * 2 {
                    return;
                }
                (params.pro_drop_rate, None)
            }
        };
        if rate <= 0.0 {
            return;
        }
        let mut rng = stream(seed, &[key("agreement"), variety as u64, generation as u64]);
        if rng.r#gen::<f32>() >= rate {
            return;
        }
        match candidates {
            Some(candidates) => self.cliticize(generation, candidates),
            None if self.agreement.stage == AgreementStage::Clitic => self.affix(generation),
            None => self.drop_pronouns(generation),
        }
    }
    pub(super) fn cliticize(
        &mut self,
        generation: u32,
        cells: Vec<(&'static str, LexemeId, Form)>,
    ) {
        let agreement = &mut self.agreement;
        agreement.side = Self::agreement_side(self.order);
        for (cell, source, form) in cells {
            agreement.cells.push(AgreementForm {
                cell,
                form,
                source,
                born: generation,
                retired: None,
                history: vec![GrammarEntry {
                    generation,
                    event: GrammarEvent::Created,
                }],
            });
        }
        agreement.stage = AgreementStage::Clitic;
        agreement.pro_drop = false;
        agreement.since = Some(generation);
        agreement.record(generation, AgreementKind::Clitics);
    }
    pub(super) fn affix(&mut self, generation: u32) {
        let agreement = &mut self.agreement;
        agreement.stage = AgreementStage::Affix;
        agreement.since = Some(generation);
        agreement.record(generation, AgreementKind::Affixes);
    }
    pub(super) fn drop_pronouns(&mut self, generation: u32) {
        self.agreement.pro_drop = true;
        self.agreement.record(generation, AgreementKind::ProDrop);
    }
    /// Agreement forms wear by the language's laws like its markers. They
    /// are not words, so the minimal word does not spare them; a rule still
    /// never deletes a form's last vowel.
    pub(super) fn wear_agreement(&mut self, law: &Law, stress: StressRule, generation: u32) {
        for cell in self
            .agreement
            .cells
            .iter_mut()
            .filter(|c| c.retired.is_none())
        {
            let mut current = Cow::Borrowed(&cell.form);
            for rule in &law.rules {
                if let Cow::Owned(next) = rule.apply_borrowed(&current, stress) {
                    current = Cow::Owned(next);
                }
            }
            let after = current.into_owned();
            if law.changes(&cell.form, &after, stress) {
                let before = std::mem::replace(&mut cell.form, after);
                cell.history.push(GrammarEntry {
                    generation,
                    event: GrammarEvent::SoundLaw {
                        law: law.id,
                        before,
                    },
                });
            }
        }
    }
    /// When fewer than half the living cells still sound apart, the
    /// contrast is lost: the forms retire and the free pronoun is
    /// obligatory again.
    pub(super) fn refresh_agreement(&mut self, stress: StressRule, generation: u32) {
        let agreement = &mut self.agreement;
        let (distinct, living) = agreement.distinct_count(stress);
        if agreement.stage != AgreementStage::None && distinct * 2 < living {
            for cell in agreement.cells.iter_mut().filter(|c| c.retired.is_none()) {
                cell.retired = Some(generation);
                cell.history.push(GrammarEntry {
                    generation,
                    event: GrammarEvent::Retired,
                });
            }
            agreement.stage = AgreementStage::None;
            agreement.pro_drop = false;
            agreement.since = Some(generation);
            agreement.record(generation, AgreementKind::Loss);
        }
        self.summary.agreement = agreement.stage;
        self.summary.pro_drop = agreement.pro_drop;
    }
    /// A person affix on a verb, outside its other marking. Like any bound
    /// marker it takes the stem's harmony value and, under free stress,
    /// leaves the accent where the stem had it.
    pub fn affix_agreement(
        &self,
        verb: &Form,
        affix: &Form,
        morphology: &Morphology,
        stress: StressRule,
    ) -> Form {
        let side = self.agreement.side;
        let (mut joined, edge) = match side {
            Side::Prefix => morphology.join_with_edge(affix, verb),
            Side::Suffix => morphology.join_with_edge(verb, affix),
        };
        if let Some(feature) = self.harmony {
            feature.realize_bound(&mut joined, edge, side, verb);
        }
        if stress == StressRule::Free && joined.stress.is_none() {
            let vowels = joined.vowel_count();
            joined.stress = (vowels > 0).then(|| match side {
                Side::Prefix => vowels.saturating_sub(verb.vowel_count()),
                Side::Suffix => 0,
            });
        }
        joined
    }
}

#[cfg(test)]
#[path = "agreement/tests.rs"]
mod tests;

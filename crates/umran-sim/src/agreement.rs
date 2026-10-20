//! Verb person agreement by Givón's cycle: a free personal pronoun, said
//! beside its verb, weakens into a clitic; the clitic fuses into a person
//! affix; audible affixes let the free pronoun drop; worn affixes give the
//! pronoun its place back, and the renewed pronouns can start over.
use super::*;
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgreementRole {
    #[default]
    Subject,
    Object,
}
impl AgreementRole {
    pub fn id(self) -> &'static str {
        match self {
            Self::Subject => "subject",
            Self::Object => "object",
        }
    }
}

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
    /// The surfaced object form at birth, when it differs from the free
    /// pronoun. Retained independently of later paradigm changes.
    pub source_form: Option<Form>,
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
    pub role: AgreementRole,
    pub stage: AgreementStage,
    pub pro_drop: bool,
    /// The verb's side the free pronoun stood on when it cliticized.
    pub side: Side,
    pub cells: Vec<AgreementForm>,
    /// Generation the current stage began; after a loss, the loss itself.
    pub since: Option<u32>,
    pub events: Vec<AgreementNotice>,
}
impl Default for Agreement {
    fn default() -> Self {
        Self::new(AgreementRole::Subject)
    }
}

/// Generations a clitic serves before it can fuse.
const FUSION_WAIT: u32 = 8;
/// Generations after a loss before the renewed pronouns can cliticize.
const RESTART_WAIT: u32 = 12;

impl Agreement {
    pub fn new(role: AgreementRole) -> Self {
        Self {
            role,
            stage: AgreementStage::None,
            pro_drop: false,
            side: Side::Prefix,
            cells: Vec::new(),
            since: None,
            events: Vec::new(),
        }
    }
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

struct Candidate {
    cell: &'static str,
    source: LexemeId,
    form: Form,
    source_form: Option<Form>,
}

impl Grammar {
    pub fn agreement(&self) -> &Agreement {
        &self.agreement
    }
    pub fn object_agreement(&self) -> &Agreement {
        &self.object_agreement
    }
    pub fn agreement_for(&self, role: AgreementRole) -> &Agreement {
        match role {
            AgreementRole::Subject => &self.agreement,
            AgreementRole::Object => &self.object_agreement,
        }
    }
    fn agreement_for_mut(&mut self, role: AgreementRole) -> &mut Agreement {
        match role {
            AgreementRole::Subject => &mut self.agreement,
            AgreementRole::Object => &mut self.object_agreement,
        }
    }
    fn agreement_side(order: WordOrder) -> Side {
        match order {
            WordOrder::SOV | WordOrder::SVO => Side::Prefix,
            WordOrder::VSO => Side::Suffix,
        }
    }
    fn object_side(order: WordOrder) -> Side {
        match order {
            WordOrder::SOV => Side::Prefix,
            WordOrder::SVO | WordOrder::VSO => Side::Suffix,
        }
    }
    /// The cells that could cliticize now: every living base cell has a
    /// pronoun of two or more segments and no two sound alike. Where the
    /// syllables nearest the verb would not tell the persons apart, the
    /// whole pronouns cliticize.
    fn clitic_candidates(&self, lexicon: &Lexicon, stress: StressRule) -> Option<Vec<Candidate>> {
        self.candidates(AgreementRole::Subject, lexicon, stress)
    }
    fn object_clitic_candidates(
        &self,
        lexicon: &Lexicon,
        stress: StressRule,
    ) -> Option<Vec<Candidate>> {
        self.candidates(AgreementRole::Object, lexicon, stress)
    }
    fn candidates(
        &self,
        role: AgreementRole,
        lexicon: &Lexicon,
        stress: StressRule,
    ) -> Option<Vec<Candidate>> {
        let side = match role {
            AgreementRole::Subject => Self::agreement_side(self.order),
            AgreementRole::Object => Self::object_side(self.order),
        };
        let mut cells = Vec::with_capacity(7);
        for cell in base_cells(lexicon) {
            let Some(word) = lexicon.word_for(by_id(cell)?) else {
                continue;
            };
            let source_form = (role == AgreementRole::Object)
                .then(|| self.pronoun_object(word, stress))
                .flatten()
                .and_then(|r| {
                    let mut forms = self.surface(&word.form, r);
                    (forms.len() == 1).then(|| forms.remove(0))
                });
            let source = source_form.as_ref().unwrap_or(&word.form);
            if source.segs.len() < 2 {
                return None;
            }
            cells.push(Candidate {
                cell,
                source: word.id,
                form: Form {
                    segs: source.segs.clone(),
                    boundaries: Vec::new(),
                    stress: None,
                },
                source_form,
            });
        }
        if cells.len() < 2
            || cells.iter().enumerate().any(|(i, a)| {
                cells[..i]
                    .iter()
                    .any(|b| same_sound(&a.form, &b.form, stress))
            })
        {
            return None;
        }
        let clitics: Vec<Form> = cells.iter().map(|c| clitic_form(&c.form, side)).collect();
        if !pairwise_same(&clitics, stress) {
            for (cell, form) in cells.iter_mut().zip(clitics) {
                cell.form = form;
            }
        }
        Some(cells)
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
        self.evolve_person_agreement(
            AgreementRole::Subject,
            seed,
            variety,
            generation,
            lexicon,
            stress,
            params,
        );
    }
    pub(super) fn evolve_object_agreement(
        &mut self,
        seed: u64,
        variety: usize,
        generation: u32,
        lexicon: &Lexicon,
        stress: StressRule,
        params: &crate::Params,
    ) {
        self.evolve_person_agreement(
            AgreementRole::Object,
            seed,
            variety,
            generation,
            lexicon,
            stress,
            params,
        );
    }
    #[allow(clippy::too_many_arguments)]
    fn evolve_person_agreement(
        &mut self,
        role: AgreementRole,
        seed: u64,
        variety: usize,
        generation: u32,
        lexicon: &Lexicon,
        stress: StressRule,
        params: &crate::Params,
    ) {
        let agreement = self.agreement_for(role);
        let (clitic_rate, fusion_rate, drop_rate, purpose) = match role {
            AgreementRole::Subject => (
                params.agreement_rate,
                params.agreement_fusion_rate,
                params.pro_drop_rate,
                "agreement",
            ),
            AgreementRole::Object => (
                params.object_agreement_rate,
                params.object_fusion_rate,
                params.object_drop_rate,
                "object-agreement",
            ),
        };
        let (rate, candidates) = match agreement.stage {
            AgreementStage::None => {
                if role == AgreementRole::Object && self.agreement.stage == AgreementStage::None {
                    return;
                }
                if agreement
                    .since
                    .is_some_and(|lost| generation < lost + RESTART_WAIT)
                {
                    return;
                }
                let candidates = match role {
                    AgreementRole::Subject => self.clitic_candidates(lexicon, stress),
                    AgreementRole::Object => self.object_clitic_candidates(lexicon, stress),
                };
                let Some(candidates) = candidates else {
                    return;
                };
                (clitic_rate, Some(candidates))
            }
            AgreementStage::Clitic => {
                if agreement
                    .since
                    .is_none_or(|since| generation < since + FUSION_WAIT)
                {
                    return;
                }
                (fusion_rate, None)
            }
            AgreementStage::Affix => {
                let (distinct, living) = agreement.distinct_count(stress);
                if agreement.pro_drop || distinct * 3 < living * 2 {
                    return;
                }
                (drop_rate, None)
            }
        };
        if rate <= 0.0 {
            return;
        }
        let mut rng = stream(seed, &[key(purpose), variety as u64, generation as u64]);
        if rng.r#gen::<f32>() >= rate {
            return;
        }
        let side = match role {
            AgreementRole::Subject => Self::agreement_side(self.order),
            AgreementRole::Object => Self::object_side(self.order),
        };
        let agreement = self.agreement_for_mut(role);
        match candidates {
            Some(candidates) => Self::cliticize(agreement, side, generation, candidates),
            None if agreement.stage == AgreementStage::Clitic => Self::affix(agreement, generation),
            None => Self::drop_pronouns(agreement, generation),
        }
    }
    fn cliticize(agreement: &mut Agreement, side: Side, generation: u32, cells: Vec<Candidate>) {
        agreement.side = side;
        for Candidate {
            cell,
            source,
            form,
            source_form,
        } in cells
        {
            agreement.cells.push(AgreementForm {
                cell,
                form,
                source,
                source_form,
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
    fn affix(agreement: &mut Agreement, generation: u32) {
        agreement.stage = AgreementStage::Affix;
        agreement.since = Some(generation);
        agreement.record(generation, AgreementKind::Affixes);
    }
    fn drop_pronouns(agreement: &mut Agreement, generation: u32) {
        agreement.pro_drop = true;
        agreement.record(generation, AgreementKind::ProDrop);
    }
    /// Agreement forms wear by the language's laws like its markers. They
    /// are not words, so the minimal word does not spare them; a rule still
    /// never deletes a form's last vowel.
    pub(super) fn wear_agreement(&mut self, law: &Law, stress: StressRule, generation: u32) {
        for cell in self
            .agreement
            .cells
            .iter_mut()
            .chain(self.object_agreement.cells.iter_mut())
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
                        law: law.id.clone(),
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
        for agreement in [&mut self.agreement, &mut self.object_agreement] {
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
        }
        self.summary.agreement = self.agreement.stage;
        self.summary.pro_drop = self.agreement.pro_drop;
        self.summary.object_agreement = self.object_agreement.stage;
        self.summary.object_pro_drop = self.object_agreement.pro_drop;
    }
    /// Person affixes keep the order the free words stood in: on the prefix
    /// side the object joins first, nearer the stem (S-O-V, as Bantu
    /// SUBJ-OBJ-stem); on the suffix side the subject joins first (V-S-O,
    /// as Arabic katab-tu-hu). Both sit outside tense. Each bound piece takes
    /// the verb's harmony and preserves free stress.
    pub fn affix_agreement(
        &self,
        verb: &Form,
        subject: Option<&Form>,
        object: Option<&Form>,
        morphology: &Morphology,
        stress: StressRule,
    ) -> Form {
        let mut joined = Cow::Borrowed(verb);
        let mut pieces = [
            (object, self.object_agreement.side),
            (subject, self.agreement.side),
        ];
        if self.agreement.side == Side::Suffix && self.object_agreement.side == Side::Suffix {
            pieces.reverse();
        }
        for (affix, side) in pieces {
            let Some(affix) = affix else { continue };
            let (mut next, edge) = match side {
                Side::Prefix => morphology.join_with_edge(affix, &joined),
                Side::Suffix => morphology.join_with_edge(&joined, affix),
            };
            if let Some(feature) = self.harmony {
                feature.realize_bound(&mut next, edge, side, verb);
            }
            if stress == StressRule::Free && next.stress.is_none() {
                let vowels = next.vowel_count();
                next.stress = (vowels > 0).then(|| match side {
                    Side::Prefix => vowels.saturating_sub(joined.vowel_count()),
                    Side::Suffix => 0,
                });
            }
            joined = Cow::Owned(next);
        }
        joined.into_owned()
    }
}

#[cfg(test)]
#[path = "agreement/tests.rs"]
mod tests;

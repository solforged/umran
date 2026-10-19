//! Frequency-sensitive object forms; nouns and pronouns share sound laws.
use super::*;

impl Grammar {
    pub(super) fn pronoun_productive(&self, category: Category, word: LexemeId) -> Option<u32> {
        if category != Category::Object || !self.pronoun_usage[word.0 as usize] {
            return None;
        }
        self.markers
            .iter()
            .filter(|m| {
                m.category == category
                    && m.scope == MarkerScope::Pronouns
                    && m.productive
                    && m.retired.is_none()
            })
            .max_by(|a, b| {
                self.marker_share(a.id)
                    .total_cmp(&self.marker_share(b.id))
                    .then(b.id.cmp(&a.id))
            })
            .map(|m| m.id)
    }

    /// Detach old pronoun forms without rebuilding their sounds or their history.
    /// The clone records when its domain narrowed; the realization keeps its birth.
    pub(super) fn retain_pronoun_markers(
        &mut self,
        lexicon: &mut Lexicon,
        stress: StressRule,
        generation: u32,
    ) {
        let noun_loss =
            self.summary.categories[Category::Object.position()].contrast_retention <= 0.5;
        for index in 0..self.markers.len() {
            let marker = &self.markers[index];
            if marker.category != Category::Object
                || marker.scope != MarkerScope::All
                || marker.kind == MarkerKind::None
            {
                continue;
            }
            let mut nouns = false;
            let mut pronouns = false;
            for word in lexicon.living() {
                for r in word
                    .paradigms
                    .iter()
                    .filter(|p| p.category == Category::Object)
                    .flat_map(|p| &p.realizations)
                    .filter(|r| r.retired.is_none() && r.share > 0.0 && r.marker as usize == index)
                {
                    if self.pronoun_usage[word.id.0 as usize] {
                        pronouns |= self.audible(&word.form, r, stress);
                    } else {
                        nouns = true;
                    }
                }
            }
            if !pronouns || (nouns && !noun_loss && marker.retired.is_none()) {
                continue;
            }
            let mut relic = marker.clone();
            relic.id = self.markers.len() as u32;
            relic.scope = MarkerScope::Pronouns;
            relic.born = generation;
            relic.productive = false;
            relic.retired = None;
            relic.majority_generations = 0;
            relic.history.push(GrammarEntry {
                generation,
                event: GrammarEvent::Created,
            });
            let id = relic.id;
            self.markers.push(relic);
            for word in &mut lexicon.lexemes {
                if word.obsolete.is_some() || !self.pronoun_usage[word.id.0 as usize] {
                    continue;
                }
                for r in word
                    .paradigms
                    .iter_mut()
                    .filter(|p| p.category == Category::Object)
                    .flat_map(|p| &mut p.realizations)
                    .filter(|r| r.retired.is_none() && r.marker as usize == index)
                {
                    r.marker = id;
                }
            }
        }
    }

    /// A minority surviving form is not the cell's ordinary object form.
    pub fn pronoun_object<'a>(
        &self,
        word: &'a crate::lexicon::Lexeme,
        stress: StressRule,
    ) -> Option<&'a Realization> {
        word.paradigms
            .iter()
            .find(|p| p.category == Category::Object)?
            .realizations
            .iter()
            .filter(|r| r.retired.is_none() && r.share > 0.0)
            .max_by(|a, b| a.share.total_cmp(&b.share))
            .filter(|r| self.audible(&word.form, r, stress))
    }

    pub(super) fn refresh_pronoun_case(
        &mut self,
        lexicon: &Lexicon,
        stress: StressRule,
        generation: u32,
    ) {
        let (mut living, mut cells, mut bound) = (0, 0, 0);
        for (_, _, cell) in crate::pronouns::CELLS {
            let Some(word) = lexicon.word_for(by_id(cell).expect("pronoun cell")) else {
                continue;
            };
            living += 1;
            if let Some(r) = self.pronoun_object(word, stress) {
                cells += 1;
                bound += usize::from(self.marker(r.marker).kind == MarkerKind::Bound);
            }
        }
        let marking = self.has_object_marking();
        self.summary.pronoun_case = if cells == 0 {
            PronounCase::None
        } else if !marking {
            PronounCase::Relic
        } else if bound * 2 >= living
            && self
                .productive(Category::Object)
                .is_some_and(|id| self.marker(id).kind == MarkerKind::Particle)
        {
            PronounCase::Ahead
        } else {
            PronounCase::LikeNouns
        };
        let noun_contrast =
            self.summary.categories[Category::Object.position()].contrast_retention > 0.5;
        if noun_contrast {
            self.pronoun_relic_recorded = false;
        } else if self.pronoun_case_enabled && cells >= 2 && !self.pronoun_relic_recorded {
            let cells = crate::pronouns::CELLS
                .iter()
                .filter_map(|(_, _, cell)| {
                    let word = lexicon.word_for(by_id(cell)?)?;
                    self.pronoun_object(word, stress).map(|_| *cell)
                })
                .collect();
            self.notice(
                generation,
                Category::Object,
                NoticeKind::PronounRelic { cells },
            );
            self.pronoun_relic_recorded = true;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn evolve_pronoun_case(
        &mut self,
        seed: u64,
        variety: usize,
        generation: u32,
        lexicon: &mut Lexicon,
        morphology: &Morphology,
        stress: StressRule,
        rate: f32,
    ) {
        self.retain_pronoun_markers(lexicon, stress, generation);
        let Some(particle) = self.productive(Category::Object) else {
            return;
        };
        if self.marker(particle).kind != MarkerKind::Particle
            || self.markers.iter().any(|m| {
                m.category == Category::Object
                    && m.scope == MarkerScope::Pronouns
                    && m.kind == MarkerKind::Bound
                    && m.retired.is_none()
            })
        {
            return;
        }
        let mut rng = stream(
            seed,
            &[
                key("pronoun case fusion"),
                variety as u64,
                generation as u64,
            ],
        );
        if rng.r#gen::<f32>() < rate {
            self.fuse_scoped(
                particle,
                MarkerScope::Pronouns,
                lexicon,
                morphology,
                stress,
                generation,
            );
        }
    }
}

#[cfg(test)]
#[path = "pronoun_case/tests.rs"]
mod tests;

//! Frequency-sensitive object and possessive forms; shared regular sound laws.
use super::*;

impl Grammar {
    pub(super) fn pronoun_productive(&self, category: Category, word: LexemeId) -> Option<u32> {
        if !matches!(category, Category::Object | Category::Genitive)
            || !self.pronoun_usage[word.0 as usize]
        {
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
        self.retain_case_markers(Category::Object, lexicon, stress, generation);
    }

    pub(super) fn retain_case_markers(
        &mut self,
        category: Category,
        lexicon: &mut Lexicon,
        stress: StressRule,
        generation: u32,
    ) {
        let noun_loss = self.summary.categories[category.position()].contrast_retention <= 0.5;
        for index in 0..self.markers.len() {
            let marker = &self.markers[index];
            if marker.category != category
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
                    .filter(|p| p.category == category)
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
                    .filter(|p| p.category == category)
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
        self.pronoun_form(word, stress, Category::Object)
    }

    pub fn pronoun_possessive<'a>(
        &self,
        word: &'a crate::lexicon::Lexeme,
        stress: StressRule,
    ) -> Option<&'a Realization> {
        self.pronoun_form(word, stress, Category::Genitive)
    }

    fn pronoun_form<'a>(
        &self,
        word: &'a crate::lexicon::Lexeme,
        stress: StressRule,
        category: Category,
    ) -> Option<&'a Realization> {
        word.paradigms
            .iter()
            .find(|p| p.category == category)?
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
        self.refresh_case(Category::Object, lexicon, stress, generation);
    }

    pub(super) fn refresh_possessive_case(
        &mut self,
        lexicon: &Lexicon,
        stress: StressRule,
        generation: u32,
    ) {
        self.refresh_case(Category::Genitive, lexicon, stress, generation);
    }

    fn refresh_case(
        &mut self,
        category: Category,
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
            if let Some(r) = self.pronoun_form(word, stress, category) {
                cells += 1;
                bound += usize::from(self.marker(r.marker).kind == MarkerKind::Bound);
            }
        }
        let marking = if category == Category::Object {
            self.has_object_marking()
        } else {
            self.summary.categories[category.position()].contrast_retention > 0.5
                && self.markers.iter().any(|m| {
                    m.category == category
                        && m.kind != MarkerKind::None
                        && m.productive
                        && m.scope == MarkerScope::All
                        && m.retired.is_none()
                })
        };
        let state = if cells == 0 {
            PronounCase::None
        } else if !marking {
            PronounCase::Relic
        } else if bound * 2 >= living
            && self
                .productive(category)
                .is_some_and(|id| self.marker(id).kind == MarkerKind::Particle)
        {
            PronounCase::Ahead
        } else {
            PronounCase::LikeNouns
        };
        let (enabled, recorded) = if category == Category::Object {
            self.summary.pronoun_case = state;
            (self.pronoun_case_enabled, self.pronoun_relic_recorded)
        } else {
            self.summary.possessive = state;
            (self.possessive_case_enabled, self.possessive_relic_recorded)
        };
        let noun_contrast = self.summary.categories[category.position()].contrast_retention > 0.5;
        let mut next_recorded = recorded;
        if noun_contrast {
            next_recorded = false;
        } else if enabled && cells >= 2 && !recorded {
            let cells = crate::pronouns::CELLS
                .iter()
                .filter_map(|(_, _, cell)| {
                    let word = lexicon.word_for(by_id(cell)?)?;
                    self.pronoun_form(word, stress, category).map(|_| *cell)
                })
                .collect();
            self.notice(generation, category, NoticeKind::PronounRelic { cells });
            next_recorded = true;
        }
        if category == Category::Object {
            self.pronoun_relic_recorded = next_recorded;
        } else {
            self.possessive_relic_recorded = next_recorded;
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

    /// Recruitment and earlier pronoun-only fusion use a separate stream.
    /// Founders have no possessive paradigm, including in static societies.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn evolve_possessive_case(
        &mut self,
        seed: u64,
        variety: usize,
        generation: u32,
        lexicon: &mut Lexicon,
        morphology: &Morphology,
        stress: StressRule,
        rate: f32,
    ) {
        if rate <= 0.0 {
            return;
        }
        let mut rng = stream(
            seed,
            &[
                key("possessive pronoun case"),
                variety as u64,
                generation as u64,
            ],
        );
        if rng.r#gen::<f32>() >= rate {
            return;
        }
        if !self.possessive_case_enabled {
            self.possessive_case_enabled = true;
            self.sync(lexicon, morphology, stress, generation);
            return;
        }
        self.retain_case_markers(Category::Genitive, lexicon, stress, generation);
        let Some(particle) = self.productive(Category::Genitive) else {
            return;
        };
        if self.marker(particle).kind == MarkerKind::Particle
            && !self.markers.iter().any(|m| {
                m.category == Category::Genitive
                    && m.scope == MarkerScope::Pronouns
                    && m.kind == MarkerKind::Bound
                    && m.retired.is_none()
            })
        {
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

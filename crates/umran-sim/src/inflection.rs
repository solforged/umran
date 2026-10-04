//! Productive internal inflection beside affixes in root-and-pattern languages.
//! Three root consonants interleave with a vocalic melody; stored words, not
//! abstract templates, undergo sound laws. Only unanimous outcomes generalize.

use crate::form::{Form, Seg};
use crate::grammar::{
    Category, Grammar, GrammarEntry, GrammarEvent, MarkerKind, MarkerOrigin, NoticeKind,
};
use crate::lexicon::{Lexeme, Lexicon, Origin};
use crate::morphology::{Morphology, Slot};
use crate::phoneme::CATALOG;
use crate::phonotactics::Phonotactics;
use crate::profile::MorphologyKind;
use crate::prosody::StressRule;
use crate::rng::{index, key, stream, weighted_index};
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateSlot {
    Root(usize),
    Vowel(Seg),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub slots: Vec<TemplateSlot>,
    /// Used only before the first usage summary exists.
    pub founding_share: f32,
}

impl Template {
    pub fn notation(&self) -> String {
        let mut out = String::new();
        for slot in &self.slots {
            match slot {
                TemplateSlot::Root(i) => {
                    out.push('C');
                    out.push(char::from(b'1' + *i as u8));
                }
                TemplateSlot::Vowel(seg) => {
                    out.push_str(CATALOG.get(seg.phone).ipa());
                    if seg.long {
                        out.push('ː');
                    }
                }
            }
        }
        out
    }

    pub fn melody(&self) -> Form {
        Form {
            segs: self
                .slots
                .iter()
                .filter_map(|s| match s {
                    TemplateSlot::Vowel(v) => Some(*v),
                    _ => None,
                })
                .collect(),
            ..Form::default()
        }
    }

    /// Fixed consonantal prefixes belong to derivation, not the root tier.
    /// A form that cannot supply exactly three radicals takes native affixing.
    fn root(base: &Form, morphology: &Morphology) -> Option<[Seg; 3]> {
        for (_, pattern) in &morphology.patterns {
            if pattern.0.len() != base.segs.len() {
                continue;
            }
            let mut root = [None; 3];
            let fits = pattern
                .0
                .iter()
                .zip(&base.segs)
                .all(|(slot, seg)| match slot {
                    Slot::Root(i) => {
                        if CATALOG.get(seg.phone).is_vowel() {
                            return false;
                        }
                        root[*i] = Some(*seg);
                        true
                    }
                    Slot::Fixed(p) if CATALOG.get(*p).is_vowel() => {
                        CATALOG.get(seg.phone).is_vowel()
                    }
                    Slot::Fixed(p) => *p == seg.phone && !seg.long,
                });
            if fits && let [Some(a), Some(b), Some(c)] = root {
                return Some([a, b, c]);
            }
        }
        let mut consonants = base
            .segs
            .iter()
            .copied()
            .filter(|s| !CATALOG.get(s.phone).is_vowel());
        let root = [consonants.next()?, consonants.next()?, consonants.next()?];
        consonants.next().is_none().then_some(root)
    }

    pub fn realize(
        &self,
        base: &Form,
        morphology: &Morphology,
        stress: StressRule,
    ) -> Option<Form> {
        let root = Self::root(base, morphology)?;
        let mut form = Form {
            segs: self
                .slots
                .iter()
                .map(|s| match s {
                    TemplateSlot::Root(i) => root[*i],
                    TemplateSlot::Vowel(v) => *v,
                })
                .collect(),
            ..Form::default()
        };
        if stress == StressRule::Free {
            form.stress = Some(
                base.stress
                    .unwrap_or(0)
                    .min(form.vowel_count().saturating_sub(1)),
            );
        }
        Some(form)
    }

    fn contrasts(&self, base: &Form, morphology: &Morphology) -> bool {
        let Some(root) = Self::root(base, morphology) else {
            return false;
        };
        !self
            .slots
            .iter()
            .map(|s| match s {
                TemplateSlot::Root(i) => root[*i],
                TemplateSlot::Vowel(v) => *v,
            })
            .eq(base.segs.iter().copied())
    }

    fn matches_form(&self, form: &Form) -> bool {
        let mut consonants = 0;
        self.slots.iter().copied().eq(form.segs.iter().map(|seg| {
            if CATALOG.get(seg.phone).is_vowel() {
                TemplateSlot::Vowel(*seg)
            } else {
                let slot = TemplateSlot::Root(consonants);
                consonants += 1;
                slot
            }
        }))
    }

    pub(crate) fn from_form(form: &Form, founding_share: f32) -> Option<Self> {
        let mut consonants = 0;
        let slots = form
            .segs
            .iter()
            .map(|s| {
                if CATALOG.get(s.phone).is_vowel() {
                    TemplateSlot::Vowel(*s)
                } else {
                    let slot = TemplateSlot::Root(consonants);
                    consonants += 1;
                    slot
                }
            })
            .collect();
        (consonants == 3 && form.vowel_count() > 0).then_some(Self {
            slots,
            founding_share,
        })
    }
}

impl Grammar {
    pub(crate) fn found_patterns(
        &mut self,
        seed: u64,
        tactics: &Phonotactics,
        morphology: &Morphology,
    ) {
        if morphology.kind != MorphologyKind::RootPattern {
            return;
        }
        self.pattern_seed = seed;
        for category in [Category::Plural, Category::Past] {
            let Some(affix) = self
                .markers
                .iter()
                .find(|m| m.category == category && m.kind == MarkerKind::Bound)
            else {
                continue;
            };
            let side = affix.side;
            let mut rng = stream(seed, &[key("pattern founding"), key(category.id())]);
            let founding_share = 0.65 + 0.2 * rng.r#gen::<f32>();
            let mut template = Template {
                slots: Vec::new(),
                founding_share,
            };
            // Avoid copying the ordinary lexical melody where the inventory allows it.
            for _ in 0..16 {
                template.slots.clear();
                for i in 0..3 {
                    template.slots.push(TemplateSlot::Root(i));
                    if i < 2 || rng.r#gen::<f32>() < 0.35 {
                        let phone = tactics.nuclei
                            [weighted_index(&mut rng, tactics.nuclei.iter().map(|(_, w)| *w))]
                        .0;
                        template.slots.push(TemplateSlot::Vowel(Seg {
                            phone,
                            long: false,
                            tone: None,
                        }));
                    }
                }
                let duplicates = morphology.basic.iter().any(|(_, p)| {
                    p.0.len() == template.slots.len()
                        && p.0.iter().zip(&template.slots).all(|(a, b)| match (a, b) {
                            (Slot::Root(a), TemplateSlot::Root(b)) => a == b,
                            (Slot::Fixed(a), TemplateSlot::Vowel(b)) => *a == b.phone,
                            _ => false,
                        })
                });
                if !duplicates {
                    break;
                }
            }
            let id = self.add_marker(
                category,
                MarkerKind::Pattern,
                side,
                template.melody(),
                MarkerOrigin::Founding,
                true,
                0,
            );
            self.markers[id as usize].template = Some(template);
        }
    }

    /// Acquisition is sampled once per new category, never on every tick.
    pub(crate) fn acquire_pattern(
        &self,
        category: Category,
        word: &Lexeme,
        morphology: &Morphology,
        generation: u32,
    ) -> Option<u32> {
        let fallback = self
            .markers
            .iter()
            .filter(|m| {
                m.category == category
                    && m.kind != MarkerKind::Pattern
                    && m.productive
                    && m.retired.is_none()
            })
            .max_by(|a, b| {
                self.marker_share(a.id)
                    .total_cmp(&self.marker_share(b.id))
                    .then(b.id.cmp(&a.id))
            })?
            .id;
        for marker in self.markers.iter().filter(|m| {
            m.category == category
                && m.kind == MarkerKind::Pattern
                && m.productive
                && m.retired.is_none()
        }) {
            let Some(template) = &marker.template else {
                continue;
            };
            if !template.contrasts(&word.form, morphology) {
                continue;
            }
            let share = if self.summary.marker_shares.is_empty() {
                template.founding_share
            } else {
                self.marker_share(marker.id)
            };
            let fit = match word.origin {
                Origin::Borrowed { .. } => 0.4,
                _ if word.born > 0 => 0.8,
                _ => 1.0,
            };
            let mut rng = stream(
                self.pattern_seed,
                &[
                    key("pattern acquisition"),
                    key(category.id()),
                    word.id.0 as u64,
                    generation as u64,
                    marker.id as u64,
                ],
            );
            if rng.r#gen::<f32>() < share * fit {
                return Some(marker.id);
            }
        }
        Some(fallback)
    }

    /// A contextual law may leave several allomorphs. Generalize a new melody
    /// only when every living use agrees, never by majority or HashMap order.
    pub(crate) fn update_patterns(
        &mut self,
        lexicon: &Lexicon,
        generation: u32,
        law: &'static str,
    ) {
        for marker in self
            .markers
            .iter_mut()
            .filter(|m| m.kind == MarkerKind::Pattern && m.retired.is_none())
        {
            let Some(template) = &marker.template else {
                continue;
            };
            let mut candidate: Option<Template> = None;
            let mut unanimous = true;
            for form in lexicon
                .living()
                .flat_map(|w| &w.paradigms)
                .flat_map(|p| &p.realizations)
                .filter(|r| r.marker == marker.id && r.retired.is_none())
                .filter_map(|r| r.form.as_ref())
            {
                if let Some(old) = &candidate {
                    if !old.matches_form(form) {
                        unanimous = false;
                        break;
                    }
                } else {
                    candidate = Template::from_form(form, template.founding_share);
                    if candidate.is_none() {
                        unanimous = false;
                        break;
                    }
                }
            }
            if unanimous
                && let Some(next) = candidate
                && next.slots != template.slots
            {
                let before = std::mem::replace(&mut marker.form, next.melody());
                marker.template = Some(next);
                marker.history.push(GrammarEntry {
                    generation,
                    event: GrammarEvent::SoundLaw { law, before },
                });
            }
        }
    }

    /// Analogy replaces one internal form with an available sound affix. Old
    /// forms remain in history; the ordinary share process can then compete.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn level_patterns(
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
        for category in [Category::Plural, Category::Past] {
            let Some(marker) = self
                .markers
                .iter()
                .filter(|m| {
                    m.category == category
                        && m.kind == MarkerKind::Bound
                        && m.productive
                        && m.retired.is_none()
                })
                .max_by(|a, b| {
                    self.marker_share(a.id)
                        .total_cmp(&self.marker_share(b.id))
                        .then(b.id.cmp(&a.id))
                })
                .map(|m| m.id)
            else {
                continue;
            };
            let mut rng = stream(
                seed,
                &[
                    key("pattern levelling"),
                    variety as u64,
                    generation as u64,
                    key(category.id()),
                ],
            );
            if rng.r#gen::<f32>() >= rate {
                continue;
            }
            let candidates: Vec<_> = lexicon
                .living()
                .filter(|w| {
                    w.paradigms.iter().any(|p| {
                        p.category == category
                            && p.realizations.iter().any(|r| {
                                r.retired.is_none()
                                    && self.marker(r.marker).kind == MarkerKind::Pattern
                            })
                    })
                })
                .map(|w| w.id)
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let word = lexicon.get_mut(candidates[index(&mut rng, candidates.len())]);
            let p = word
                .paradigms
                .iter_mut()
                .find(|p| p.category == category)
                .expect("pattern category");
            let r = p
                .realizations
                .iter_mut()
                .find(|r| r.retired.is_none() && self.marker(r.marker).kind == MarkerKind::Pattern)
                .expect("pattern form");
            let regular =
                self.materialize(marker, &word.form, morphology, stress, generation, r.share);
            let after = regular.form.expect("bound form");
            let before = r.form.replace(after.clone()).expect("pattern form");
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
                    lexeme: word.id,
                    before,
                    after,
                },
            );
        }
        self.refresh(lexicon, stress, generation);
    }
}

#[cfg(test)]
#[path = "inflection_tests.rs"]
mod tests;

//! A small automated comparative method. It aligns current same-meaning
//! words and looks for recurring sound correspondences. Current grammatical
//! contrasts can supply capped supporting evidence, never extra word-pair
//! votes. Neither lexical nor marker ancestry, origins, or history is read.

use crate::adapt::distance;
use crate::concepts::Concept;
use crate::form::{Form, Seg};
use crate::grammar::{Category, Grammar, MarkerKind, Realization};
use crate::lexicon::{Lexeme, Lexicon};
use crate::phoneme::PhonemeId;
use crate::prosody::StressRule;
use crate::variety::Variety;
use std::collections::HashMap;

/// Aligned segments; `None` is a gap where one word has nothing.
pub type Pair = (Option<PhonemeId>, Option<PhonemeId>);

/// Thresholds for the comparative method.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// A correspondence must recur at least this often to count as regular...
    pub min_support: u32,
    /// ...and account for this share of either segment's occurrences.
    pub min_share: f32,
    /// Share of a pair's aligned positions that must be regular for the
    /// pair to be judged cognate.
    pub cognate_threshold: f32,
    /// Rounds of re-estimating correspondences from presumed cognates.
    pub rounds: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            min_support: 3,
            min_share: 0.25,
            cognate_threshold: 0.75,
            rounds: 4,
        }
    }
}

const GAP: f32 = 1.5;
/// Consonants and vowels never align with each other.
const MISMATCH: f32 = 99.0;

/// Minimum-cost alignment by feature distance (Needleman–Wunsch).
pub fn align(a: &Form, b: &Form) -> Vec<Pair> {
    align_with_cost(a, b).0
}

/// Phonetic similarity of two words, 0–1: one minus the alignment cost
/// over the cost of aligning nothing at all.
pub fn similarity(a: &Form, b: &Form) -> f32 {
    let worst = GAP * (a.segs.len() + b.segs.len()).max(1) as f32;
    (1.0 - align_with_cost(a, b).1 / worst).clamp(0.0, 1.0)
}

/// Rough mutual intelligibility, 0–1: mean phonetic similarity of the two
/// varieties' words for the same Leipzig–Jakarta concepts, corrected for
/// chance by the similarity of words for different concepts (as in ASJP's
/// LDND). Built only from what speakers hear, never from lineage.
pub fn intelligibility(a: &Lexicon, b: &Lexicon) -> f32 {
    let core: Vec<(&Form, &Form)> = crate::concepts::CONCEPTS
        .iter()
        .filter(|c| c.stability.is_some())
        .filter_map(|c| Some((&a.word_for(c)?.form, &b.word_for(c)?.form)))
        .collect();
    if core.len() < 2 {
        return 0.0;
    }
    let mean = |pairs: &mut dyn Iterator<Item = f32>| {
        let (sum, n) = pairs.fold((0.0, 0), |(s, n), x| (s + x, n + 1));
        sum / n as f32
    };
    let same = mean(&mut core.iter().map(|(x, y)| similarity(x, y)));
    // Mismatch meanings in both directions so the score is symmetric.
    let n = core.len();
    let chance = mean(&mut (0..n).flat_map(|i| {
        let j = (i + 1) % n;
        [
            similarity(core[i].0, core[j].1),
            similarity(core[j].0, core[i].1),
        ]
    }));
    if chance >= 1.0 {
        return 1.0;
    }
    ((same - chance) / (1.0 - chance)).clamp(0.0, 1.0)
}

fn align_with_cost(a: &Form, b: &Form) -> (Vec<Pair>, f32) {
    align_segments(&a.segs, &b.segs)
}

fn align_segments(xa: &[Seg], xb: &[Seg]) -> (Vec<Pair>, f32) {
    let (n, m) = (xa.len(), xb.len());
    let sub = |x: PhonemeId, y: PhonemeId| {
        let d = distance(x, y);
        if d.is_finite() {
            0.6 * d.min(5.0)
        } else {
            MISMATCH
        }
    };
    let mut cost = vec![vec![0.0_f32; m + 1]; n + 1];
    for (i, row) in cost.iter_mut().enumerate() {
        row[0] = i as f32 * GAP;
    }
    for (j, cell) in cost[0].iter_mut().enumerate() {
        *cell = j as f32 * GAP;
    }
    for i in 1..=n {
        for j in 1..=m {
            cost[i][j] = (cost[i - 1][j - 1] + sub(xa[i - 1].phone, xb[j - 1].phone))
                .min(cost[i - 1][j] + GAP)
                .min(cost[i][j - 1] + GAP);
        }
    }
    let mut out = Vec::with_capacity(n.max(m));
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0
            && j > 0
            && cost[i][j] == cost[i - 1][j - 1] + sub(xa[i - 1].phone, xb[j - 1].phone)
        {
            out.push((Some(xa[i - 1].phone), Some(xb[j - 1].phone)));
            (i, j) = (i - 1, j - 1);
        } else if i > 0 && cost[i][j] == cost[i - 1][j] + GAP {
            out.push((Some(xa[i - 1].phone), None));
            i -= 1;
        } else {
            out.push((None, Some(xb[j - 1].phone)));
            j -= 1;
        }
    }
    out.reverse();
    (out, cost[n][m])
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub concept: &'static Concept,
    pub a: Form,
    pub b: Form,
    pub alignment: Vec<Pair>,
    /// Share of aligned positions that are regular correspondences.
    pub regularity: f32,
    pub cognate: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    /// Regular correspondences with their support, most frequent first.
    pub regular: Vec<(Pair, u32)>,
    pub rows: Vec<Row>,
}

/// Correspondences that recur often enough among `alignments` to be regular.
pub fn regular_correspondences<'a>(
    alignments: impl Iterator<Item = &'a [Pair]>,
    settings: &Settings,
) -> Vec<(Pair, u32)> {
    let mut counts: HashMap<Pair, u32> = HashMap::new();
    let mut by_a: HashMap<Option<PhonemeId>, u32> = HashMap::new();
    let mut by_b: HashMap<Option<PhonemeId>, u32> = HashMap::new();
    for alignment in alignments {
        for &pair in alignment {
            *counts.entry(pair).or_default() += 1;
            *by_a.entry(pair.0).or_default() += 1;
            *by_b.entry(pair.1).or_default() += 1;
        }
    }
    let mut regular: Vec<(Pair, u32)> = counts
        .into_iter()
        .filter(|&(pair, n)| {
            let share_a = n as f32 / by_a[&pair.0] as f32;
            let share_b = n as f32 / by_b[&pair.1] as f32;
            n >= settings.min_support && share_a.max(share_b) >= settings.min_share
        })
        .collect();
    regular.sort_by(|x, y| {
        y.1.cmp(&x.1).then_with(|| {
            (x.0.0.map(|p| p.0), x.0.1.map(|p| p.0)).cmp(&(y.0.0.map(|p| p.0), y.0.1.map(|p| p.0)))
        })
    });
    regular
}

/// Compares dominant words and current bound segment contrasts.
/// Lexicons have no stress rule. Use [`compare_varieties`] to include audible
/// stress contrasts and separately stored particles.
pub fn compare(a: &Lexicon, b: &Lexicon, concepts: &[&'static Concept]) -> Comparison {
    compare_with(a, b, concepts, &Settings::default())
}

pub fn compare_with(
    a: &Lexicon,
    b: &Lexicon,
    concepts: &[&'static Concept],
    settings: &Settings,
) -> Comparison {
    compare_current(a, b, None, None, concepts, settings)
}

/// Compares current speech, including bound realizations and shared particles.
/// Marker IDs only cap repeated evidence within each inventory; they are
/// never compared as a claim of common descent.
pub fn compare_varieties(a: &Variety, b: &Variety, concepts: &[&'static Concept]) -> Comparison {
    compare_varieties_with(a, b, concepts, &Settings::default())
}

pub fn compare_varieties_with(
    a: &Variety,
    b: &Variety,
    concepts: &[&'static Concept],
    settings: &Settings,
) -> Comparison {
    compare_current(
        &a.lexicon,
        &b.lexicon,
        Some((&a.grammar, a.stress())),
        Some((&b.grammar, b.stress())),
        concepts,
        settings,
    )
}

struct GrammarObservation {
    row: usize,
    alignment: Vec<Pair>,
}

fn dominant_realization(word: &Lexeme, category: Category) -> Option<&Realization> {
    word.paradigms
        .iter()
        .find(|p| p.category == category)?
        .realizations
        .iter()
        .filter(|r| r.retired.is_none() && r.share > 0.0)
        .max_by(|a, b| a.share.total_cmp(&b.share))
}

/// The audible contrast, not a reconstructed ancestral affix. Removing
/// unchanged surface edges also retains stem alternations after ending loss.
/// Internal morpheme boundaries are not audible and therefore do not matter.
fn bound_contrast<'a>(base: &Form, marked: &'a Form, stress: Option<StressRule>) -> &'a [Seg] {
    let nucleus = |form: &Form| {
        let syllable = form.stressed_syllable(stress?)?;
        (0..form.segs.len())
            .filter(|&i| form.is_vowel(i))
            .nth(syllable)
    };
    let (base_stress, marked_stress) = (nucleus(base), nucleus(marked));
    let same = |i: usize, j: usize| {
        base.segs[i] == marked.segs[j] && (base_stress == Some(i)) == (marked_stress == Some(j))
    };
    let mut start = 0;
    while start < base.segs.len().min(marked.segs.len()) && same(start, start) {
        start += 1;
    }
    let (mut base_end, mut end) = (base.segs.len(), marked.segs.len());
    while base_end > start && end > start && same(base_end - 1, end - 1) {
        base_end -= 1;
        end -= 1;
    }
    &marked.segs[start..end]
}

fn current_contrast<'a>(
    word: &'a Lexeme,
    realization: &'a Realization,
    speech: Option<(&'a Grammar, StressRule)>,
) -> &'a [Seg] {
    if let Some(form) = &realization.form {
        return bound_contrast(&word.form, form, speech.map(|(_, stress)| stress));
    }
    let Some((grammar, _)) = speech else {
        return &[];
    };
    let marker = grammar.marker(realization.marker);
    if marker.kind == MarkerKind::Particle {
        &marker.form.segs
    } else {
        &[]
    }
}

/// Each recurring correspondence contributes once per marker-pair/category,
/// irrespective of its frequency within a form or across hundreds of words.
/// Two distinct word pairs must attest it among the presumed cognates.
fn grammatical_correspondences(groups: &[Vec<GrammarObservation>], rows: &[Row]) -> Vec<Vec<Pair>> {
    groups
        .iter()
        .filter_map(|observations| {
            let mut counts: HashMap<Pair, u32> = HashMap::new();
            for observation in observations.iter().filter(|o| rows[o.row].cognate) {
                for (i, &pair) in observation.alignment.iter().enumerate() {
                    if !observation.alignment[..i].contains(&pair) {
                        *counts.entry(pair).or_default() += 1;
                    }
                }
            }
            let recurring: Vec<Pair> = counts
                .into_iter()
                .filter_map(|(pair, n)| (n >= 2).then_some(pair))
                .collect();
            (!recurring.is_empty()).then_some(recurring)
        })
        .collect()
}

fn compare_current(
    a: &Lexicon,
    b: &Lexicon,
    speech_a: Option<(&Grammar, StressRule)>,
    speech_b: Option<(&Grammar, StressRule)>,
    concepts: &[&'static Concept],
    settings: &Settings,
) -> Comparison {
    let mut rows = Vec::with_capacity(concepts.len());
    let mut groups: Vec<Vec<GrammarObservation>> = Vec::new();
    let mut group_indices = HashMap::new();
    let mut observed = std::collections::HashSet::new();
    for &concept in concepts {
        let (Some(wa), Some(wb)) = (a.word_for(concept), b.word_for(concept)) else {
            continue;
        };
        let row = rows.len();
        for category in Category::ALL {
            if !concept.categories.allows(category) {
                continue;
            }
            let (Some(ra), Some(rb)) = (
                dominant_realization(wa, category),
                dominant_realization(wb, category),
            ) else {
                continue;
            };
            if !observed.insert((category, wa.id, wb.id)) {
                continue;
            }
            let (ca, cb) = (
                current_contrast(wa, ra, speech_a),
                current_contrast(wb, rb, speech_b),
            );
            if ca.is_empty() || cb.is_empty() {
                continue;
            }
            let group = *group_indices
                .entry((category, ra.marker, rb.marker))
                .or_insert_with(|| {
                    groups.push(Vec::new());
                    groups.len() - 1
                });
            groups[group].push(GrammarObservation {
                row,
                alignment: align_segments(ca, cb).0,
            });
        }
        rows.push(Row {
            concept,
            alignment: align(&wa.form, &wb.form),
            a: wa.form.clone(),
            b: wb.form.clone(),
            regularity: 0.0,
            cognate: true,
        });
    }
    let mut regular = Vec::new();
    for _ in 0..settings.rounds {
        let grammar = grammatical_correspondences(&groups, &rows);
        regular = regular_correspondences(
            rows.iter()
                .filter(|r| r.cognate)
                .map(|r| r.alignment.as_slice())
                .chain(grammar.iter().map(Vec::as_slice)),
            settings,
        );
        let set: HashMap<Pair, u32> = regular.iter().copied().collect();
        for row in &mut rows {
            let hits = row.alignment.iter().filter(|p| set.contains_key(p)).count();
            row.regularity = hits as f32 / row.alignment.len().max(1) as f32;
            row.cognate = row.regularity >= settings.cognate_threshold;
        }
    }
    Comparison { regular, rows }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concepts::CONCEPTS;
    use crate::grammar::{GrammarEntry, GrammarEvent, Marker, MarkerOrigin, Paradigm, Side};
    use crate::lexicon::{Entry, Event, Origin};
    use crate::livelihood::Livelihood;
    use crate::phoneme::CATALOG;
    use crate::profile::SoundProfile;
    use crate::variety::Fork;

    fn show(alignment: &[Pair]) -> String {
        alignment
            .iter()
            .map(|(a, b)| {
                let s = |x: &Option<PhonemeId>| x.map_or("-", |p| CATALOG.get(p).ipa());
                format!("{}{}", s(a), s(b))
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn nouns(n: usize) -> Vec<&'static Concept> {
        CONCEPTS
            .iter()
            .filter(|c| c.categories.allows(Category::Plural))
            .take(n)
            .collect()
    }

    fn with_endings(concepts: &[&'static Concept], bases: &[&str], ending: &str) -> Lexicon {
        let mut lexicon = Lexicon::found([]);
        for (i, &concept) in concepts.iter().enumerate() {
            let base = bases[i % bases.len()];
            let id = lexicon.coin(Form::from_ipa(base).unwrap(), Origin::Founding, concept, 0);
            lexicon.slot_mut(concept).introduce(id, 1.0);
            let word = lexicon.get_mut(id);
            word.paradigms.push(Paradigm {
                category: Category::Plural,
                realizations: vec![Realization {
                    marker: 0,
                    form: Some(Form::from_ipa(&format!("{base}{ending}")).unwrap()),
                    edge: word.form.segs.len(),
                    share: 1.0,
                    born: 0,
                    retired: None,
                    history: Vec::new(),
                }],
            });
        }
        lexicon
    }

    fn evidence_fixture() -> (Vec<&'static Concept>, Lexicon, Lexicon) {
        let concepts = nouns(5);
        let a = with_endings(&concepts, &["pa", "pa", "ta", "ta", "ta"], "pi");
        let b = with_endings(&concepts, &["fa", "fa", "ta", "ta", "ta"], "fi");
        (concepts, a, b)
    }

    fn varieties(a: Lexicon, b: Lexicon, kind: MarkerKind) -> (Variety, Variety) {
        let mut va = Variety::found(
            17,
            &SoundProfile::base(),
            Livelihood::Farming,
            crate::Ethos::default(),
        );
        let mut vb = va.clone();
        va.lexicon = a;
        vb.lexicon = b;
        for (variety, particle) in [(&mut va, "pi"), (&mut vb, "fi")] {
            variety.grammar = Grammar::default();
            variety.grammar.markers.push(Marker {
                id: 0,
                category: Category::Plural,
                kind,
                side: Side::Suffix,
                form: Form::from_ipa(particle).unwrap(),
                born: 0,
                origin: MarkerOrigin::Founding,
                productive: true,
                majority_generations: 0,
                retired: None,
                history: Vec::new(),
            });
            if kind == MarkerKind::Particle {
                for word in &mut variety.lexicon.lexemes {
                    word.paradigms[0].realizations[0].form = None;
                }
            }
        }
        (va, vb)
    }

    fn correspondence(a: &str, b: &str) -> Pair {
        (
            Form::from_ipa(a).unwrap().phones().next(),
            Form::from_ipa(b).unwrap().phones().next(),
        )
    }

    fn support(comparison: &Comparison, pair: Pair) -> Option<u32> {
        comparison
            .regular
            .iter()
            .find_map(|&(p, n)| (p == pair).then_some(n))
    }

    #[test]
    fn current_bound_contrasts_add_one_clue_to_lexical_evidence() {
        let (concepts, mut a, mut b) = evidence_fixture();
        let marked = compare(&a, &b, &concepts);
        for word in a.lexemes.iter_mut().chain(&mut b.lexemes) {
            word.paradigms.clear();
        }
        let lexical = compare(&a, &b, &concepts);
        let pair = correspondence("p", "f");
        assert_eq!(support(&lexical, pair), None);
        assert_eq!(support(&marked, pair), Some(3));
        assert!(!lexical.rows[0].cognate && !lexical.rows[1].cognate);
        assert!(marked.rows[0].cognate && marked.rows[1].cognate);
    }

    #[test]
    fn recurring_shared_particle_forms_supplement_lexical_evidence() {
        let (concepts, a, b) = evidence_fixture();
        let (mut a, mut b) = varieties(a, b, MarkerKind::Particle);
        let current = compare_varieties(&a, &b, &concepts);
        assert_eq!(support(&current, correspondence("p", "f")), Some(3));
        assert!(current.rows[0].cognate && current.rows[1].cognate);

        // Changing the shared current particles, not their source snapshots,
        // removes their support for this lexical correspondence.
        a.grammar.markers[0].form = Form::from_ipa("si").unwrap();
        b.grammar.markers[0].form = Form::from_ipa("si").unwrap();
        let changed = compare_varieties(&a, &b, &concepts);
        assert_eq!(support(&changed, correspondence("p", "f")), None);
        assert!(!changed.rows[0].cognate && !changed.rows[1].cognate);
    }

    #[test]
    fn bound_stress_evidence_uses_lexical_syllables_and_each_current_rule() {
        let concepts = nouns(5);
        let a = with_endings(
            &concepts,
            &["katapa", "katapa", "katata", "katata", "katata"],
            "",
        );
        let b = with_endings(
            &concepts,
            &["katafa", "katafa", "katata", "katata", "katata"],
            "",
        );
        let (mut a, mut b) = varieties(a, b, MarkerKind::Bound);
        for variety in [&mut a, &mut b] {
            for word in &mut variety.lexicon.lexemes {
                word.form.stress = Some(1);
                word.paradigms[0].realizations[0]
                    .form
                    .as_mut()
                    .unwrap()
                    .stress = Some(2);
            }
        }
        let pair = correspondence("p", "f");
        let settings = Settings {
            cognate_threshold: 1.0,
            ..Settings::default()
        };
        for (left, right, expected) in [
            (StressRule::Free, StressRule::Free, Some(3)),
            (StressRule::Initial, StressRule::Free, None),
            (StressRule::Free, StressRule::Initial, None),
        ] {
            a.profile.stress = Some(left);
            b.profile.stress = Some(right);
            let comparison = compare_varieties_with(&a, &b, &concepts, &settings);
            assert_eq!(support(&comparison, pair), expected);
            assert_eq!(comparison.rows[0].cognate, expected.is_some());
        }
        // Lexical compatibility has no current stress rule, even when free
        // accents are stored on the forms.
        assert_eq!(
            support(
                &compare_with(&a.lexicon, &b.lexicon, &concepts, &settings),
                pair
            ),
            None
        );
    }

    #[test]
    fn predictable_stress_exposes_current_stem_contrasts() {
        let concepts = nouns(5);
        let a = with_endings(
            &concepts,
            &["katapa", "katapa", "katata", "katata", "katata"],
            "i",
        );
        let b = with_endings(
            &concepts,
            &["katafa", "katafa", "katata", "katata", "katata"],
            "i",
        );
        let (mut a, mut b) = varieties(a, b, MarkerKind::Bound);
        let pair = correspondence("p", "f");
        let settings = Settings {
            cognate_threshold: 1.0,
            ..Settings::default()
        };
        a.profile.stress = Some(StressRule::Penult);
        b.profile.stress = Some(StressRule::Penult);
        let current = compare_varieties_with(&a, &b, &concepts, &settings);
        assert_eq!(support(&current, pair), Some(3));
        assert!(current.rows[0].cognate && current.rows[1].cognate);
        a.profile.stress = Some(StressRule::Initial);
        b.profile.stress = Some(StressRule::Initial);
        assert_eq!(
            support(&compare_varieties_with(&a, &b, &concepts, &settings), pair),
            None
        );
    }

    #[test]
    fn repetition_is_capped_per_marker_pair_and_requires_distinct_words() {
        let settings = Settings {
            min_support: 1,
            ..Settings::default()
        };
        let pair = correspondence("p", "f");
        for (n, expected) in [(1, None), (2, Some(1)), (100, Some(1))] {
            let concepts = nouns(n);
            let a = with_endings(&concepts, &["ta"], "pipi");
            let mut b = with_endings(&concepts, &["ta"], "fifi");
            // IDs need not match across inventories to establish a pattern.
            for word in &mut b.lexemes {
                word.paradigms[0].realizations[0].marker = 7;
            }
            assert_eq!(
                support(&compare_with(&a, &b, &concepts, &settings), pair),
                expected
            );
        }

        let concepts = nouns(100);
        let mut a = with_endings(&concepts, &["ta"], "pipi");
        let b = with_endings(&concepts, &["ta"], "fifi");
        for word in a.lexemes.iter_mut().skip(2) {
            word.paradigms[0].realizations[0].marker = 1;
        }
        assert_eq!(
            support(&compare_with(&a, &b, &concepts, &settings), pair),
            Some(2)
        );
    }

    #[test]
    fn only_the_dominant_active_realization_supplies_grammar_evidence() {
        let (concepts, mut a, mut b) = evidence_fixture();
        for word in a.lexemes.iter_mut().chain(&mut b.lexemes) {
            let paradigm = &mut word.paradigms[0];
            let mut retired = paradigm.realizations[0].clone();
            retired.marker = 2;
            retired.share = 10.0;
            retired.retired = Some(20);
            paradigm.realizations[0].share = 0.2;
            paradigm.realizations.push(Realization {
                marker: 1,
                form: Some(Form::from_ipa(&format!("{}si", word.form.ipa())).unwrap()),
                edge: word.form.segs.len(),
                share: 0.8,
                born: 1,
                retired: None,
                history: Vec::new(),
            });
            paradigm.realizations.push(retired);
        }
        let current = compare(&a, &b, &concepts);
        assert_eq!(support(&current, correspondence("p", "f")), None);
        assert!(!current.rows[0].cognate && !current.rows[1].cognate);
    }

    #[test]
    fn marker_ancestry_and_variety_lineage_do_not_change_comparison() {
        for kind in [MarkerKind::Bound, MarkerKind::Particle] {
            let (concepts, a, b) = evidence_fixture();
            let (mut a, mut b) = varieties(a, b, kind);
            let before = compare_varieties(&a, &b, &concepts);
            assert_eq!(support(&before, correspondence("p", "f")), Some(3));
            a.grammar.markers[0].origin = MarkerOrigin::Imported {
                from: 90,
                marker: 42,
                source: Form::from_ipa("su").unwrap(),
            };
            b.grammar.markers[0].origin = MarkerOrigin::Fused { particle: 91 };
            for variety in [&mut a, &mut b] {
                variety.parent = Some(Fork {
                    variety: 92,
                    generation: 200,
                    inherited: 0,
                });
                variety.laws.push((201, "unrelated history"));
                let history = GrammarEntry {
                    generation: 202,
                    event: GrammarEvent::SoundLaw {
                        law: "unrelated history",
                        before: Form::from_ipa("su").unwrap(),
                    },
                };
                variety.grammar.markers[0].history.push(history.clone());
                for word in &mut variety.lexicon.lexemes {
                    word.origin = Origin::Borrowed {
                        from: 93,
                        source: crate::lexicon::LexemeId(94),
                        cause: crate::LoanCause::Unrecorded,
                    };
                    word.log.push(Entry {
                        generation: 203,
                        event: Event::SoundLaw {
                            law: "unrelated history",
                            before: Form::from_ipa("su").unwrap(),
                        },
                    });
                    word.paradigms[0].realizations[0]
                        .history
                        .push(history.clone());
                }
            }
            assert_eq!(compare_varieties(&a, &b, &concepts), before);
        }
    }

    #[test]
    fn similarity_runs_from_identical_to_unrelated() {
        let f = |s: &str| Form::from_ipa(s).unwrap();
        assert_eq!(similarity(&f("pata"), &f("pata")), 1.0);
        let near = similarity(&f("pata"), &f("bata"));
        let far = similarity(&f("pata"), &f("lomu"));
        assert!(near > 0.8 && far < near, "{near} {far}");
    }

    #[test]
    fn aligns_by_features_with_gaps() {
        let f = |s: &str| Form::from_ipa(s).unwrap();
        assert_eq!(show(&align(&f("pata"), &f("fata"))), "pf aa tt aa");
        assert_eq!(show(&align(&f("kata"), &f("kat"))), "kk aa tt a-");
        assert_eq!(show(&align(&f("sta"), &f("sita"))), "ss -i tt aa");
    }
}

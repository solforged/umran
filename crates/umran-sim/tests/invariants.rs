//! Real-world invariants, sampled across founding profiles and millennia.
//! Run alone with `cargo test -p umran-sim --test invariants -- --nocapture`.
//!
//! Minimal words are transition guards: founding and borrowing may supply
//! short words, but sound laws cannot shorten them below or further below
//! the minimum (docs/engine.md and prosody.rs). All words must keep a vowel.

use std::collections::HashSet;
use std::time::Instant;
use umran_sim::compare::{compare, compare_varieties, intelligibility};
use umran_sim::grammar::{GrammarEntry, GrammarEvent, MarkerKind, MarkerOrigin};
use umran_sim::prosody::moras;
use umran_sim::{
    Action, CATALOG, CONCEPTS, Chronicle, ContactKind, Entry, Env, Event, Form, LanguageDesign,
    Law, LexemeId, Lexicon, MapSize, Matcher, MinimalWord, Name, Naming, Origin, Params, PhonemeId,
    Recipe, Revelation, Rewrite, Rise, Seg, Segment, SoundChange, SoundProfile, StressRule,
    Syllable, Variety, World,
};

const SEEDS: u64 = 30;
const GENERATIONS: u32 = 160; // 4,000 years; seed zero continues to 10,000.

fn size(minimal: MinimalWord, form: &Form) -> usize {
    match minimal {
        MinimalWord::Syllable | MinimalWord::TwoSyllables => form.vowel_count(),
        MinimalWord::Heavy => moras(form),
    }
}

fn least(minimal: MinimalWord) -> usize {
    match minimal {
        MinimalWord::Syllable => 1,
        MinimalWord::Heavy | MinimalWord::TwoSyllables => 2,
    }
}

// Syllabification and stress are prosodic primitives, not rule matching.
// Contextual wrappers are resolved here; Matcher::matches ignores them.
fn contextual(
    matcher: &Matcher,
    form: &Form,
    i: usize,
    syllables: &[Syllable],
    stress: Option<usize>,
) -> bool {
    let seg = form.segs[i];
    match matcher {
        Matcher::Length { target, long } => {
            seg.long == *long && contextual(target, form, i, syllables, stress)
        }
        Matcher::Stressed { target, stressed } => {
            // A shared geminate belongs to its preceding coda first.
            let containing = syllables
                .iter()
                .position(|s| s.onset.contains(&i) || s.nucleus == i || s.coda.contains(&i));
            containing.is_some()
                && (containing == stress) == *stressed
                && contextual(target, form, i, syllables, stress)
        }
        Matcher::OpenSyllable(target) => {
            syllables
                .iter()
                .any(|s| s.nucleus == i && s.coda.is_empty())
                && contextual(target, form, i, syllables, stress)
        }
        Matcher::MedialVowel => {
            CATALOG.get(seg.phone).is_vowel()
                && form.segs[..i]
                    .iter()
                    .any(|s| CATALOG.get(s.phone).is_vowel())
                && form.segs[i + 1..]
                    .iter()
                    .any(|s| CATALOG.get(s.phone).is_vowel())
        }
        Matcher::Phone(phone) => seg.phone == *phone,
        Matcher::AnyConsonant => !CATALOG.get(seg.phone).is_vowel(),
        Matcher::AnyVowel => CATALOG.get(seg.phone).is_vowel(),
        Matcher::Consonant {
            place,
            manner,
            voiced,
            secondary,
        } => match CATALOG.get(seg.phone) {
            Segment::Consonant(c) => {
                place.is_none_or(|p| p == c.place)
                    && manner.is_none_or(|m| m == c.manner)
                    && voiced.is_none_or(|v| v == c.voiced)
                    && secondary.is_none_or(|s| s == c.secondary)
            }
            _ => false,
        },
        Matcher::Vowel {
            height,
            backness,
            rounded,
        } => match CATALOG.get(seg.phone) {
            Segment::Vowel(v) => {
                height.is_none_or(|h| h == v.height)
                    && backness.is_none_or(|b| b == v.backness)
                    && rounded.is_none_or(|r| r == v.rounded)
            }
            _ => false,
        },
    }
}

fn needs_syllables(matcher: &Matcher) -> bool {
    match matcher {
        Matcher::Stressed { .. } | Matcher::OpenSyllable(_) => true,
        Matcher::Length { target, .. } => needs_syllables(target),
        _ => false,
    }
}

fn environment(
    env: &Env,
    form: &Form,
    at: usize,
    neighbor: Option<usize>,
    syllables: &[Syllable],
    stress: Option<usize>,
) -> bool {
    match env {
        Env::Any => true,
        Env::WordEdge => neighbor.is_none(),
        Env::Matcher(matcher) => {
            neighbor.is_some_and(|i| contextual(matcher, form, i, syllables, stress))
        }
        Env::FollowingSyllableVowel(matcher) => syllables
            .iter()
            .position(|s| s.onset.contains(&at) || s.nucleus == at || s.coda.contains(&at))
            .and_then(|n| syllables.get(n + 1))
            .is_some_and(|s| contextual(matcher, form, s.nucleus, syllables, stress)),
    }
}

// This oracle does not call Law::apply, SoundChange::apply, or hits: it
// resolves a whole input before filtering deletions, so it also detects
// sequential (feeding/bleeding) application within a word.
fn replacement(rewrite: &Rewrite, old: Seg) -> Option<Seg> {
    let phone = match (rewrite, CATALOG.get(old.phone)) {
        (Rewrite::Delete | Rewrite::GeminateNext | Rewrite::Compensate, _) => return None,
        (Rewrite::Length(long), _) => return Some(Seg { long: *long, ..old }),
        (Rewrite::Phone(id), _) => *id,
        (
            Rewrite::Consonant {
                place,
                manner,
                voiced,
                secondary,
            },
            Segment::Consonant(c),
        ) => CATALOG
            .consonants()
            .find_map(|(id, candidate)| {
                (candidate.place == place.unwrap_or(c.place)
                    && candidate.manner == manner.unwrap_or(c.manner)
                    && candidate.voiced == voiced.unwrap_or(c.voiced)
                    && candidate.secondary == secondary.unwrap_or(c.secondary))
                .then_some(id)
            })
            .unwrap_or(old.phone),
        (
            Rewrite::Vowel {
                height,
                backness,
                rounded,
            },
            Segment::Vowel(v),
        ) => CATALOG
            .vowels()
            .find_map(|(id, candidate)| {
                (candidate.height == height.unwrap_or(v.height)
                    && candidate.backness == backness.unwrap_or(v.backness)
                    && candidate.rounded == rounded.unwrap_or(v.rounded))
                .then_some(id)
            })
            .unwrap_or(old.phone),
        _ => old.phone,
    };
    Some(Seg { phone, ..old })
}

fn reference_rule(rule: &SoundChange, before: &Form, stress_rule: StressRule) -> Form {
    let needs = needs_syllables(&rule.target)
        || [&rule.left, &rule.right].iter().any(|env| match env {
            Env::Matcher(matcher) => needs_syllables(matcher),
            Env::FollowingSyllableVowel(_) => true,
            Env::Any | Env::WordEdge => false,
        });
    let syllables = if needs {
        before.syllables()
    } else {
        Vec::new()
    };
    let stress = needs
        .then(|| before.stressed_syllable(stress_rule))
        .flatten();
    let mut outcomes: Vec<Option<Seg>> = before
        .segs
        .iter()
        .enumerate()
        .map(|(i, &seg)| {
            let left = i.checked_sub(1);
            let right = (i + 1 < before.segs.len()).then_some(i + 1);
            if contextual(&rule.target, before, i, &syllables, stress)
                && environment(&rule.left, before, i, left, &syllables, stress)
                && environment(&rule.right, before, i, right, &syllables, stress)
            {
                replacement(&rule.result, seg)
            } else {
                Some(seg)
            }
        })
        .collect();
    if matches!(rule.result, Rewrite::GeminateNext | Rewrite::Compensate) {
        // Each deletion's side effect reads the input and cannot restore
        // a neighbor that this same rule deleted.
        for i in 0..outcomes.len() {
            if outcomes[i].is_some() {
                continue;
            }
            let neighbor = match rule.result {
                Rewrite::GeminateNext => before
                    .segs
                    .get(i + 1)
                    .filter(|s| !CATALOG.get(s.phone).is_vowel())
                    .map(|_| i + 1),
                Rewrite::Compensate => i
                    .checked_sub(1)
                    .filter(|&j| CATALOG.get(before.segs[j].phone).is_vowel()),
                _ => None,
            };
            if let Some(j) = neighbor
                && let Some(seg) = &mut outcomes[j]
            {
                seg.long = true;
            }
        }
    }
    if !outcomes
        .iter()
        .flatten()
        .any(|s| CATALOG.get(s.phone).is_vowel())
        && let Some(last) = before
            .segs
            .iter()
            .rposition(|s| CATALOG.get(s.phone).is_vowel())
    {
        outcomes[last] = Some(before.segs[last]);
    }
    // An explicit free accent stays on its nucleus, moves to the next
    // surviving vowel when lost, and falls back to the final vowel.
    let stress = before.stress.and_then(|stressed| {
        let nucleus = before
            .segs
            .iter()
            .enumerate()
            .filter(|(_, s)| CATALOG.get(s.phone).is_vowel())
            .nth(stressed)?
            .0;
        let surviving_before = outcomes[..nucleus]
            .iter()
            .flatten()
            .filter(|s| CATALOG.get(s.phone).is_vowel())
            .count();
        let count = outcomes
            .iter()
            .flatten()
            .filter(|s| CATALOG.get(s.phone).is_vowel())
            .count();
        (count > 0).then(|| surviving_before.min(count - 1))
    });
    let segs: Vec<_> = outcomes.iter().flatten().copied().collect();
    let mut boundaries: Vec<_> = before
        .boundaries
        .iter()
        .map(|&b| outcomes[..b].iter().flatten().count())
        .filter(|&b| b > 0 && b < segs.len())
        .collect();
    boundaries.dedup();
    Form {
        segs,
        boundaries,
        stress,
    }
}

fn reference_law(law: &Law, before: &Form, minimal: MinimalWord, stress: StressRule) -> Form {
    law.rules.iter().fold(before.clone(), |current, rule| {
        let next = reference_rule(rule, &current, stress);
        if size(minimal, &next) < size(minimal, &current) && size(minimal, &next) < least(minimal) {
            current
        } else {
            next
        }
    })
}

fn expected_changes(
    before: &Form,
    laws: &[&Law],
    minimal: MinimalWord,
    mut stress: StressRule,
    generation: u32,
) -> (Form, Vec<Entry>) {
    let mut form = before.clone();
    let mut log = Vec::new();
    for law in laws {
        let after = reference_law(law, &form, minimal, stress);
        let next_stress = law.stress.unwrap_or(stress);
        if after != form || form.stressed_syllable(stress) != after.stressed_syllable(next_stress) {
            log.push(Entry {
                generation,
                event: Event::SoundLaw {
                    law: law.id,
                    before: form,
                },
            });
            form = after;
        }
        stress = next_stress;
    }
    (form, log)
}

#[derive(Clone, Copy, Debug)]
enum NameKey {
    Language,
    People(usize),
    Given(usize),
    State(usize),
    Place(usize),
    Exonym(usize),
}

fn names(world: &World, v: usize) -> Vec<(NameKey, &Name)> {
    let variety = &world.varieties[v];
    let mut out = vec![(NameKey::Language, &variety.name)];
    out.extend(
        world
            .communities
            .iter()
            .enumerate()
            .filter(|(_, c)| c.living() && c.variety == v)
            .map(|(i, c)| (NameKey::People(i), &c.name)),
    );
    out.extend(
        variety
            .given
            .iter()
            .enumerate()
            .map(|(i, n)| (NameKey::Given(i), &n.name)),
    );
    out.extend(
        world
            .states
            .iter()
            .enumerate()
            .filter(|(_, s)| s.standing() && world.communities[s.rulers].variety == v)
            .map(|(i, s)| (NameKey::State(i), &s.name)),
    );
    let held: HashSet<_> = world
        .communities
        .iter()
        .filter(|c| c.living() && c.variety == v)
        .flat_map(|c| c.lands.iter().copied())
        .collect();
    out.extend(
        world
            .places
            .iter()
            .enumerate()
            .filter(|(r, _)| held.contains(r))
            .filter_map(|(r, history)| {
                history
                    .last()
                    .filter(|p| p.variety == v)
                    .map(|p| (NameKey::Place(r), &p.name))
            }),
    );
    out.extend(
        variety
            .exonyms
            .iter()
            .enumerate()
            .map(|(i, (_, n))| (NameKey::Exonym(i), n)),
    );
    out
}

fn name_at(world: &World, v: usize, key: NameKey) -> &Name {
    match key {
        NameKey::Language => &world.varieties[v].name,
        NameKey::People(i) => &world.communities[i].name,
        NameKey::Given(i) => &world.varieties[v].given[i].name,
        NameKey::State(i) => &world.states[i].name,
        NameKey::Place(r) => &world.places[r].last().unwrap().name,
        NameKey::Exonym(i) => &world.varieties[v].exonyms[i].1,
    }
}

fn assert_form(form: &Form, seed: u64, year: u32) {
    assert!(
        form.vowel_count() > 0,
        "seed {seed}, year {year}: {} lost its last vowel",
        form.ipa()
    );
    assert!(
        form.stress.is_none_or(|s| s < form.vowel_count()),
        "seed {seed}, year {year}: lexical stress is outside {}",
        form.ipa()
    );
    assert!(
        form.boundaries
            .iter()
            .all(|&b| b > 0 && b < form.segs.len())
            && form.boundaries.windows(2).all(|b| b[0] < b[1]),
        "seed {seed}, year {year}: invalid morpheme boundaries in {}",
        form.ipa()
    );
}

fn assert_grammatical_change(
    before: &Form,
    after: &Form,
    history: &[GrammarEntry],
    laws: &[&Law],
    constraints: (MinimalWord, StressRule),
    when: (u64, u32),
) {
    let (seed, year) = when;
    let (expected, log) = expected_changes(before, laws, constraints.0, constraints.1, year / 25);
    // Analogy follows the sound laws and keeps the form it replaced.
    let after_laws = history
        .iter()
        .find_map(|entry| match &entry.event {
            GrammarEvent::Analogy { before } => Some(before),
            _ => None,
        })
        .unwrap_or(after);
    assert_eq!(
        after_laws, &expected,
        "seed {seed}, year {year}: irregular grammatical form"
    );
    let changes: Vec<_> = history
        .iter()
        .filter_map(|entry| match &entry.event {
            GrammarEvent::SoundLaw { law, before } => Some(Entry {
                generation: entry.generation,
                event: Event::SoundLaw {
                    law,
                    before: before.clone(),
                },
            }),
            _ => None,
        })
        .collect();
    assert_eq!(
        changes, log,
        "seed {seed}, year {year}: incorrect grammatical sound history"
    );
    assert_form(after, seed, year);
}

fn assert_slots(lexicon: &Lexicon, seed: u64, year: u32) {
    let mut in_use = HashSet::new();
    for slot in &lexicon.slots {
        let total: f32 = slot.variants.iter().map(|v| v.weight).sum();
        assert!(
            slot.variants.is_empty() || (total - 1.0).abs() < 1e-4,
            "seed {seed}, year {year}: {} usage sums to {total}",
            slot.concept.id
        );
        let mut unique = HashSet::new();
        for variant in &slot.variants {
            assert!(
                variant.weight.is_finite() && variant.weight > 0.0 && variant.weight <= 1.0,
                "seed {seed}, year {year}: invalid usage for {}",
                slot.concept.id
            );
            assert!(
                unique.insert(variant.lexeme),
                "seed {seed}, year {year}: duplicate competitor"
            );
            let lexeme = lexicon.get(variant.lexeme);
            assert!(
                lexeme.obsolete.is_none(),
                "seed {seed}, year {year}: obsolete word remains in use"
            );
            in_use.insert(variant.lexeme);
        }
    }
    for lexeme in &lexicon.lexemes {
        assert_eq!(
            lexeme.obsolete.is_none(),
            in_use.contains(&lexeme.id),
            "seed {seed}, year {year}: living status disagrees with usage for {:?}",
            lexeme.id
        );
    }
}

#[test]
fn sound_laws_are_regular_protect_words_and_freeze_the_record() {
    let start = Instant::now();
    let profiles = SoundProfile::presets();
    let laws = umran_sim::catalog();
    let (mut fired, mut obsolete_matches, mut protected, mut changed_names) = (0, 0, 0, 0);
    let mut stress_shifts = 0;
    let mut seen = HashSet::new();
    for seed in 0..SEEDS {
        let profile = &profiles[seed as usize % profiles.len()];
        let params = Params {
            sound_change_rate: 1.0,
            preference_pull: 0.0,
            ..Params::static_society()
        };
        let mut world = World::solo(seed, profile, params);
        world.raise_state(0, None, Rise::Proclaimed);
        world.found_religion(0, Revelation::Proclaimed);
        let sacred = world.religions[0].sacred;
        let frozen = world.varieties[sacred].clone();
        let founders = (world.states[0].founder.clone(), world.religions[0].clone());
        let continents = world.continent_names.clone();
        for _ in 0..if seed == 0 { 400 } else { GENERATIONS } {
            let v = world.communities[0].variety;
            let minimal = world.varieties[v].minimal;
            let stress = world.varieties[v].stress();
            let stress_history_len = world.varieties[v].stress_history.len();
            let words: Vec<_> = world.varieties[v]
                .lexicon
                .lexemes
                .iter()
                .map(|l| (l.form.clone(), l.obsolete.is_some(), l.log.len()))
                .collect();
            let mut grammatical = Vec::new();
            let mut frozen_grammar = Vec::new();
            for lexeme in &world.varieties[v].lexicon.lexemes {
                for (p, paradigm) in lexeme.paradigms.iter().enumerate() {
                    for (r, realization) in paradigm.realizations.iter().enumerate() {
                        if lexeme.obsolete.is_some() || realization.retired.is_some() {
                            frozen_grammar.push((lexeme.id, p, r, realization.clone()));
                        } else if let Some(form) = &realization.form {
                            grammatical.push((
                                lexeme.id,
                                p,
                                r,
                                form.clone(),
                                realization.history.len(),
                            ));
                        }
                    }
                }
            }
            let particles: Vec<_> = world.varieties[v]
                .grammar
                .markers
                .iter()
                .filter(|marker| marker.kind == MarkerKind::Particle && marker.retired.is_none())
                .map(|marker| (marker.id, marker.form.clone(), marker.history.len()))
                .collect();
            let before_names: Vec<_> = names(&world, v)
                .into_iter()
                .map(|(key, n)| (key, n.form.clone(), n.log.len()))
                .collect();
            let law_count = world.varieties[v].laws.len();
            world.step();
            let year = world.generation * 25;
            let applied: Vec<_> = world.varieties[v].laws[law_count..]
                .iter()
                .map(|&(g, id)| {
                    assert_eq!(
                        g, world.generation,
                        "seed {seed}, year {year}: misplaced law"
                    );
                    seen.insert(id);
                    laws.iter().find(|law| law.id == id).expect("catalog law")
                })
                .collect();
            fired += applied.len();
            let mut expected_stress = stress;
            let mut expected_history = Vec::new();
            for law in &applied {
                if let Some(next) = law.stress {
                    expected_history.push((world.generation, expected_stress));
                    expected_stress = next;
                }
            }
            stress_shifts += expected_history.len();
            assert_eq!(
                world.varieties[v].stress(),
                expected_stress,
                "seed {seed}, year {year}: incorrect stress rule"
            );
            assert_eq!(
                &world.varieties[v].stress_history[stress_history_len..],
                expected_history.as_slice(),
                "seed {seed}, year {year}: incorrect stress history"
            );
            for ((before, obsolete, log_len), after) in
                words.iter().zip(&world.varieties[v].lexicon.lexemes)
            {
                let (expected, log) = if *obsolete {
                    let (_, hypothetical_log) =
                        expected_changes(before, &applied, minimal, stress, world.generation);
                    if !hypothetical_log.is_empty() {
                        obsolete_matches += 1;
                    }
                    (before.clone(), Vec::new())
                } else {
                    expected_changes(before, &applied, minimal, stress, world.generation)
                };
                assert_eq!(
                    after.form,
                    expected,
                    "seed {seed}, year {year}, {:?}, laws {:?}: irregular word change",
                    after.id,
                    applied.iter().map(|l| l.id).collect::<Vec<_>>()
                );
                let changes: Vec<_> = after.log[*log_len..]
                    .iter()
                    .filter(|e| matches!(e.event, Event::SoundLaw { .. }))
                    .cloned()
                    .collect();
                assert_eq!(
                    changes, log,
                    "seed {seed}, year {year}, {:?}: incorrect law history",
                    after.id
                );
                if *obsolete {
                    assert_eq!(
                        after.log.len(),
                        *log_len,
                        "seed {seed}, year {year}: obsolete record changed"
                    );
                } else {
                    assert!(
                        !(size(minimal, &after.form) < size(minimal, before)
                            && size(minimal, &after.form) < least(minimal)),
                        "seed {seed}, year {year}: {:?} wore below {}",
                        after.id,
                        minimal.label()
                    );
                    let mut current_stress = stress;
                    for law in &applied {
                        protected += usize::from(law.rules.iter().any(|rule| {
                            let raw = reference_rule(rule, before, current_stress);
                            size(minimal, &raw) < size(minimal, before)
                                && size(minimal, &raw) < least(minimal)
                        }));
                        current_stress = law.stress.unwrap_or(current_stress);
                    }
                }
            }
            for (key, before, log_len) in before_names {
                let after = name_at(&world, v, key);
                let (expected, log) =
                    expected_changes(&before, &applied, minimal, stress, world.generation);
                changed_names += usize::from(!log.is_empty());
                assert_eq!(
                    after.form, expected,
                    "seed {seed}, year {year}, {key:?}: irregular name change"
                );
                assert_eq!(
                    &after.log[log_len..],
                    log.as_slice(),
                    "seed {seed}, year {year}, {key:?}: incorrect name history"
                );
                assert_form(&after.form, seed, year);
            }
            for (lexeme, p, r, before) in frozen_grammar {
                assert_eq!(
                    world.varieties[v].lexicon.get(lexeme).paradigms[p].realizations[r],
                    before,
                    "seed {seed}, year {year}: obsolete or retired grammar changed"
                );
            }
            for (lexeme, p, r, before, log_len) in grammatical {
                let after = &world.varieties[v].lexicon.get(lexeme).paradigms[p].realizations[r];
                assert_grammatical_change(
                    &before,
                    after.form.as_ref().unwrap(),
                    &after.history[log_len..],
                    &applied,
                    (minimal, stress),
                    (seed, year),
                );
            }
            for (marker, before, log_len) in particles {
                let after = world.varieties[v].grammar.marker(marker);
                assert_grammatical_change(
                    &before,
                    &after.form,
                    &after.history[log_len..],
                    &applied,
                    (minimal, stress),
                    (seed, year),
                );
            }
            for lexeme in world.varieties[v].lexicon.living() {
                assert_form(&lexeme.form, seed, year);
            }
            assert_slots(&world.varieties[v].lexicon, seed, year);
            assert_variety_eq(&world.varieties[sacred], &frozen, seed, year);
            assert_eq!(
                world.states[0].founder, founders.0,
                "seed {seed}, year {year}: dead founder changed"
            );
            assert_eq!(
                world.religions[0], founders.1,
                "seed {seed}, year {year}: frozen faith names changed"
            );
            for (now, old) in world.continent_names.iter().zip(&continents) {
                if old.is_some() {
                    assert_eq!(
                        now, old,
                        "seed {seed}, year {year}: fixed continent heading changed"
                    );
                }
            }
        }
    }
    // Coverage floors, not calibrated frequencies or exact history pins.
    assert!(
        fired >= SEEDS as usize * 10,
        "too few laws exercised: {fired}"
    );
    assert!(
        seen.len() >= 15,
        "too few kinds of law exercised: {}",
        seen.len()
    );
    assert!(
        obsolete_matches >= 100,
        "obsolete matching words were not exercised: {obsolete_matches}"
    );
    assert!(
        protected >= 100,
        "minimal-word protection was not exercised: {protected}"
    );
    assert!(
        changed_names >= 100,
        "matching names were not exercised: {changed_names}"
    );
    assert!(stress_shifts > 0, "stress-shift laws were not exercised");
    println!(
        "regularity: {SEEDS} seeds, 4,000–10,000 years, {fired} laws / {} kinds, {stress_shifts} stress shifts, {obsolete_matches} obsolete matches, {protected} protected words, {changed_names} changed names; {:?}",
        seen.len(),
        start.elapsed()
    );
}

fn assert_variety_eq(a: &Variety, b: &Variety, seed: u64, year: u32) {
    macro_rules! same {
        ($($field:ident),+ $(,)?) => { $(assert_eq!(a.$field, b.$field,
            "seed {seed}, year {year}: variety {} differs", stringify!($field));)+ };
    }
    same!(
        name,
        profile,
        founding_inventory,
        lexicon,
        morphology,
        grammar,
        minimal,
        laws,
        stress_history,
        waves,
        parent,
        koine_of,
        koine_mergers,
        style,
        given,
        written,
        exonyms,
        high,
        vernacular
    );
}

fn assert_world_eq(a: &World, b: &World, seed: u64) {
    let year = a.generation * 25;
    macro_rules! same {
        ($($field:ident),+ $(,)?) => { $(assert_eq!(a.$field, b.$field,
            "seed {seed}, year {year}: world {} differs", stringify!($field));)+ };
    }
    same!(
        seed,
        generation,
        map,
        communities,
        contacts,
        params,
        events,
        places,
        continent_names,
        states,
        religions
    );
    assert_eq!(
        a.varieties.len(),
        b.varieties.len(),
        "seed {seed}, year {year}: different languages"
    );
    for (x, y) in a.varieties.iter().zip(&b.varieties) {
        assert_variety_eq(x, y, seed, year);
    }
}

fn recipe(seed: u64) -> Recipe {
    let mut actions = Vec::new();
    for (i, preset) in ["germanic", "polynesian", "semitic"].iter().enumerate() {
        let language_seed = seed * 3 + i as u64;
        actions.push(Action::Found {
            naming: Naming::People,
            design: LanguageDesign::preset(preset, language_seed).unwrap(),
            seed: language_seed,
            power: 0.4 + i as f32 * 0.2,
            openness: 0.4 + i as f32 * 0.1,
            region: None,
            livelihood: None,
            ethos: None,
        });
    }
    actions.extend([
        Action::Connect {
            a: 0,
            b: 1,
            intensity: 0.6,
            contact: ContactKind::Trade,
        },
        Action::Connect {
            a: 2,
            b: 1,
            intensity: 0.8,
            contact: ContactKind::Rule,
        },
        Action::Religion { community: 0 },
    ]);
    let mut history = Chronicle::new(seed, MapSize::Small);
    for action in actions {
        history.act(action).unwrap();
    }
    use umran_sim::settlement::{SettlementChoice, SettlementIntent};
    let destination = history
        .latest()
        .settlement_options(0, SettlementIntent::Settlers, 0.5)
        .unwrap()
        .into_iter()
        .find(|o| o.reason.is_none())
        .expect("a founder has nearby land")
        .region;
    history
        .act(Action::Settle {
            choice: SettlementChoice {
                community: 0,
                destination,
                intent: SettlementIntent::Settlers,
                share: 0.5,
                naming: None,
                intensity: 0.5,
            },
        })
        .unwrap();
    history
        .act(Action::Run {
            generations: if seed == 0 { GENERATIONS } else { 80 },
        })
        .unwrap();
    history.recipe()
}

#[test]
fn recipes_replay_identical_histories_in_fresh_worlds() {
    let start = Instant::now();
    let mut stress_checkpoints = 0;
    for seed in 0..SEEDS {
        let recipe = recipe(seed);
        let a = Chronicle::from_recipe(&recipe).unwrap();
        let b = Chronicle::from_recipe(&recipe).unwrap();
        assert_world_eq(a.latest(), b.latest(), seed);
        // Drop both replayers and build from persisted data, not a World clone.
        let expected = a.latest().clone();
        let saved = serde_json::to_string(&recipe).unwrap();
        drop((a, b, recipe));
        let restored: Recipe = serde_json::from_str(&saved).unwrap();
        let mut fresh = Chronicle::from_recipe(&restored).unwrap();
        assert_world_eq(&expected, fresh.latest(), seed);
        // Compare both sides of actual stress shifts, not merely arbitrary
        // generations that might precede every suprasegmental law.
        let mut checkpoints = if seed == 0 {
            vec![1, 60, 120]
        } else {
            Vec::new()
        };
        if seed == 0 || stress_checkpoints == 0 {
            let mut shifts: Vec<_> = fresh
                .latest()
                .varieties
                .iter()
                .flat_map(|v| v.stress_history.iter().map(|&(g, _)| g))
                .collect();
            shifts.sort_unstable();
            shifts.dedup();
            stress_checkpoints += shifts.len();
            for generation in shifts {
                checkpoints.extend([generation.saturating_sub(1), generation]);
            }
        }
        checkpoints.sort_unstable();
        checkpoints.dedup();
        for generation in checkpoints {
            let warm = fresh.world_at(generation);
            let mut prefix = restored.clone();
            *prefix
                .tellings
                .iter_mut()
                .find(|t| t.id == prefix.active)
                .unwrap()
                .actions
                .last_mut()
                .unwrap() = Action::Run {
                generations: generation,
            };
            let cold = Chronicle::from_recipe(&prefix).unwrap();
            assert_world_eq(&warm, cold.latest(), seed);
        }
        for variety in &fresh.latest().varieties {
            assert_slots(&variety.lexicon, seed, fresh.latest().generation * 25);
            for lexeme in variety.lexicon.living() {
                assert_form(&lexeme.form, seed, fresh.latest().generation * 25);
                for realization in lexeme
                    .paradigms
                    .iter()
                    .flat_map(|p| &p.realizations)
                    .filter(|r| r.retired.is_none())
                {
                    if let Some(form) = &realization.form {
                        assert_form(form, seed, fresh.latest().generation * 25);
                    }
                }
            }
            for marker in variety
                .grammar
                .markers
                .iter()
                .filter(|m| m.kind == MarkerKind::Particle && m.retired.is_none())
            {
                assert_form(&marker.form, seed, fresh.latest().generation * 25);
            }
        }
    }
    assert!(
        stress_checkpoints > 0,
        "historical checkpoints did not exercise a stress shift"
    );
    println!(
        "determinism: {SEEDS} persisted recipes, 2,000–4,000 years, fresh builds and past checkpoints around {stress_checkpoints} stress shifts; {:?}",
        start.elapsed()
    );
}

fn hide_descent(variety: &mut Variety) {
    variety.parent = None;
    variety.koine_of.clear();
    variety.laws.clear();
    variety.stress_history.clear();
    variety.waves.clear();
    variety.grammar.events.clear();
    for marker in &mut variety.grammar.markers {
        marker.origin = MarkerOrigin::Founding;
        marker.born = 0;
        marker.history.clear();
    }
    for lexeme in &mut variety.lexicon.lexemes {
        lexeme.origin = Origin::Founding;
        lexeme.born = 0;
        lexeme.log.clear();
        for paradigm in &mut lexeme.paradigms {
            for realization in &mut paradigm.realizations {
                realization.born = 0;
                realization.history.clear();
            }
        }
    }
}

fn scramble_descent(variety: &mut Variety) {
    variety.parent = Some(umran_sim::variety::Fork {
        variety: usize::MAX,
        generation: u32::MAX,
        inherited: u32::MAX,
    });
    variety.koine_of = vec![(usize::MAX, 1.0)];
    variety.laws = vec![(u32::MAX, "unrelated history")];
    variety.stress_history = vec![(u32::MAX, StressRule::Final)];
    variety.waves = vec![(u32::MAX, "unrelated history", usize::MAX)];
    let history = GrammarEntry {
        generation: u32::MAX,
        event: GrammarEvent::Imported {
            from: usize::MAX,
            source: Form::default(),
        },
    };
    for marker in &mut variety.grammar.markers {
        marker.origin = MarkerOrigin::Imported {
            from: usize::MAX,
            marker: u32::MAX,
            source: Form::default(),
        };
        marker.born = u32::MAX;
        marker.history = vec![history.clone()];
    }
    for (i, lexeme) in variety.lexicon.lexemes.iter_mut().enumerate() {
        lexeme.origin = if i % 2 == 0 {
            Origin::Borrowed {
                from: usize::MAX,
                source: LexemeId(u32::MAX),
                cause: umran_sim::LoanCause::Unrecorded,
            }
        } else {
            Origin::Renewed {
                base: LexemeId(u32::MAX),
                with: Some(LexemeId(u32::MAX)),
            }
        };
        lexeme.born = u32::MAX;
        lexeme.log = vec![Entry {
            generation: u32::MAX,
            event: Event::Borrowed {
                from: usize::MAX,
                source: Form::from_phones([]),
                cause: umran_sim::LoanCause::Unrecorded,
            },
        }];
        for paradigm in &mut lexeme.paradigms {
            for realization in &mut paradigm.realizations {
                realization.born = u32::MAX;
                realization.history = vec![history.clone()];
            }
        }
    }
}

#[test]
fn comparative_method_ignores_hidden_and_scrambled_descent() {
    let start = Instant::now();
    let core: Vec<_> = CONCEPTS.iter().filter(|c| c.stability.is_some()).collect();
    let profiles = SoundProfile::presets();
    for seed in 0..SEEDS {
        let mut world = World::solo(
            seed,
            &profiles[seed as usize % profiles.len()],
            Params::static_society(),
        );
        let outsider = world.found(&profiles[(seed as usize + 7) % profiles.len()], 0.5, 0.5);
        world.run(20);
        let daughter = world.split(0, None, 0.0);
        world.run(GENERATIONS - 20);
        let parent = world.communities[0].variety;
        for other in [daughter, outsider] {
            let other = world.communities[other].variety;
            let (a, b) = (&world.varieties[parent], &world.varieties[other]);
            let expected = compare(&a.lexicon, &b.lexicon, &core);
            let grammatical = compare_varieties(a, b, &core);
            let heard = intelligibility(&a.lexicon, &b.lexicon);
            let (mut hidden_a, mut hidden_b) = (a.clone(), b.clone());
            hide_descent(&mut hidden_a);
            hide_descent(&mut hidden_b);
            assert_eq!(
                compare(&hidden_a.lexicon, &hidden_b.lexicon, &core),
                expected,
                "seed {seed}: hidden descent affected comparison"
            );
            assert_eq!(
                compare_varieties(&hidden_a, &hidden_b, &core),
                grammatical,
                "seed {seed}: hidden descent affected grammatical comparison"
            );
            assert_eq!(
                intelligibility(&hidden_a.lexicon, &hidden_b.lexicon),
                heard,
                "seed {seed}: hidden descent affected intelligibility"
            );
            scramble_descent(&mut hidden_a);
            scramble_descent(&mut hidden_b);
            assert_eq!(
                compare(&hidden_a.lexicon, &hidden_b.lexicon, &core),
                expected,
                "seed {seed}: scrambled descent affected comparison"
            );
            assert_eq!(
                compare_varieties(&hidden_a, &hidden_b, &core),
                grammatical,
                "seed {seed}: scrambled descent affected grammatical comparison"
            );
            assert_eq!(
                intelligibility(&hidden_a.lexicon, &hidden_b.lexicon),
                heard,
                "seed {seed}: scrambled descent affected intelligibility"
            );
        }
    }
    println!(
        "comparison: {SEEDS} seeds, related and unrelated languages at 4,000 years; {:?}",
        start.elapsed()
    );
}

#[test]
fn existing_segment_ids_retain_their_historical_meaning() {
    // This is an intentional compatibility fixture, not a generated-output
    // snapshot. The catalog is immutable during a run; only an edit to its
    // construction can insert or reinterpret an id. Extend at the END when
    // adding sounds; never update existing offsets to make this test pass.
    const HISTORICAL: &[&str] = &[
        "p", "b", "t", "d", "k", "g", "q", "ʔ", "m", "n", "ɲ", "ŋ", "r", "ɾ", "f", "v", "θ", "ð",
        "s", "z", "ʃ", "ʒ", "x", "ɣ", "h", "ts", "dz", "tʃ", "dʒ", "w", "j", "l", "ʎ", "ɬ", "ɓ",
        "ɗ", "pʼ", "tʼ", "kʼ", "i", "y", "ɨ", "u", "ɪ", "ʊ", "e", "ø", "ə", "o", "ɛ", "ɔ", "æ",
        "a", "ɑ", "pʰ", "tʰ", "kʰ", "tʃʰ", "bʱ", "dʱ", "gʱ", "ʈ", "ɖ", "ɳ", "ʂ", "ʐ", "ɭ", "kʷ",
        "gʷ", "qʷ", "xʷ", "tɬ", "χ", "ʁ", "ħ", "ʕ", "β", "ɸ",
    ];
    for (offset, &ipa) in HISTORICAL.iter().enumerate() {
        let id = PhonemeId(offset as u16);
        assert_eq!(
            CATALOG.get(id).ipa(),
            ipa,
            "historical segment {offset} was reinterpreted"
        );
        assert_eq!(
            CATALOG.id_by_ipa(ipa),
            Some(id),
            "historical segment {ipa} was moved or duplicated"
        );
    }
}

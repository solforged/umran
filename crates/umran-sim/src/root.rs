use crate::concepts::{CONCEPTS, Concept, FAMILIES, Field, Iconic, Relation};
use crate::form::Form;
use crate::morphology::Morphology;
use crate::phoneme::PhonemeId;
use crate::phonotactics::Phonotactics;
use crate::profile::MorphologyKind;
use crate::profile::Spelling;
use crate::rng::{key, stream, weighted_index};
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// Draws a root gets to avoid every existing form; after these, a homophone
/// of a word in another semantic field is accepted, as happens in real
/// languages.
const FRESH_TRIES: usize = 4;
/// Draws before giving up on field uniqueness. From halfway on, roots get
/// an extra syllable, which multiplies the space of possible forms.
const FIELD_TRIES: usize = 256;

/// Draws to find root consonants a short word lacks.
const DISTINCT_DRAWS: usize = 16;

/// A founding word, and the word it was built from if it was derived.
#[derive(Clone, Debug, PartialEq)]
pub struct Minted {
    pub concept: &'static Concept,
    pub form: Form,
    pub derived: Option<(&'static Concept, Relation)>,
}

/// One word per concept. Roots draw from per-concept random streams, so
/// adding concepts never changes how earlier draws begin. Then, for each
/// word family, the derived concept is rebuilt from its base with chance
/// `derivation`.
pub fn mint_roots(
    seed: u64,
    tactics: &Phonotactics,
    spelling: &Spelling,
    morphology: &Morphology,
    derivation: f32,
) -> Vec<Minted> {
    let mut used: HashSet<Form> = HashSet::new();
    let mut by_field: HashMap<Field, HashSet<Form>> = HashMap::new();
    let mut out: Vec<Minted> = CONCEPTS
        .iter()
        .filter(|concept| !crate::pronouns::is_optional_cell(concept.id))
        .map(|concept| {
            let mut rng = stream(seed, &[key("root"), key(concept.id)]);
            let field = by_field.entry(concept.field).or_default();
            let form = mint_one(
                &mut rng,
                tactics,
                spelling,
                Some(morphology),
                concept,
                &used,
                field,
            );
            used.insert(form.clone());
            field.insert(form.clone());
            Minted {
                concept,
                form,
                derived: None,
            }
        })
        .collect();

    let index = |id: &str| {
        CONCEPTS
            .iter()
            .position(|c| c.id == id)
            .expect("known concept")
    };
    // Each word's root skeleton, so derivations from derived words build on
    // the true root rather than on a pattern's prefix. In a root-and-pattern
    // language, short words still have three-consonant roots (as Arabic yad
    // "hand" does): missing consonants are drawn, not copied.
    let patterned = morphology.kind == MorphologyKind::RootPattern;
    let onsets: Vec<(PhonemeId, f32)> = tactics
        .onsets
        .iter()
        .filter(|(ids, _)| ids.len() == 1)
        .map(|(ids, w)| (ids[0], *w))
        .collect();
    let mut roots: Vec<Option<[PhonemeId; 3]>> = out
        .iter()
        .map(|m| {
            let mut consonants: Vec<PhonemeId> = m
                .form
                .phones()
                .filter(|p| !crate::phoneme::CATALOG.get(*p).is_vowel())
                .take(3)
                .collect();
            if patterned && !onsets.is_empty() {
                let mut rng = stream(seed, &[key("skeleton"), key(m.concept.id)]);
                for _ in 0..DISTINCT_DRAWS {
                    if consonants.len() >= 3 {
                        break;
                    }
                    let c = onsets[weighted_index(&mut rng, onsets.iter().map(|(_, w)| *w))].0;
                    if !consonants.contains(&c) {
                        consonants.push(c);
                    }
                }
            }
            match consonants.as_slice() {
                [a, b, c] => Some([*a, *b, *c]),
                _ => Morphology::skeleton(&m.form),
            }
        })
        .collect();
    for &(base, word, relation) in FAMILIES {
        let mut rng = stream(seed, &[key("family"), key(word)]);
        if rng.r#gen::<f32>() >= derivation {
            continue;
        }
        let (b, w) = (index(base), index(word));
        let Some(form) = morphology.derive(&out[b].form, roots[b], relation) else {
            continue;
        };
        let concept = out[w].concept;
        let clash = out.iter().any(|m| {
            m.concept.field == concept.field && m.concept.id != concept.id && m.form == form
        });
        if clash || spelling.unfortunate(&form) {
            continue;
        }
        roots[w] = roots[b];
        out[w] = Minted {
            concept,
            form,
            derived: Some((out[b].concept, relation)),
        };
    }
    out
}

/// A root for `concept` that avoids `field` entirely and, for its first few
/// draws, every form in `used`. In a root-and-pattern language, words of
/// more than one syllable are the class pattern over the root's consonants.
pub(crate) fn mint_one(
    rng: &mut impl Rng,
    tactics: &Phonotactics,
    spelling: &Spelling,
    morphology: Option<&Morphology>,
    concept: &Concept,
    used: &HashSet<Form>,
    field: &HashSet<Form>,
) -> Form {
    let patterned = morphology.filter(|m| m.kind == MorphologyKind::RootPattern);
    let nursery = matches!(
        concept.iconic,
        Some(Iconic::NurseryMother | Iconic::NurseryFather)
    );
    let mut form = Form::default();
    for attempt in 0..FIELD_TRIES {
        let mut syllables = tactics.syllables_for(rng, concept);
        if attempt >= FIELD_TRIES / 2 {
            syllables = (syllables + 1).min(3);
        }
        form = tactics.root(rng, concept, syllables);
        if let (Some(m), true, false) = (patterned, syllables > 1, nursery)
            && let Some(word) = Morphology::skeleton(&form).and_then(|k| m.word(concept.class, k))
        {
            form = word;
        }
        let fresh_enough = attempt >= FRESH_TRIES || !used.contains(&form);
        if fresh_enough && !field.contains(&form) && !spelling.unfortunate(&form) {
            break;
        }
    }
    form
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flavor::Flavor;
    use crate::inventory::Inventory;
    use crate::phoneme::{CATALOG, Height, Manner, Segment};
    use crate::profile::SoundProfile;

    fn mint(seed: u64, profile: &SoundProfile) -> (Vec<Minted>, Phonotactics) {
        let inventory =
            Inventory::sample(&profile.inventory, &mut stream(seed, &[key("inventory")]));
        let tactics = Phonotactics::compile(&profile.phonotactics, &inventory);
        let morphology = Morphology::found(
            &profile.morphology,
            &tactics,
            &mut stream(seed, &[key("morphology")]),
        );
        let roots = mint_roots(
            seed,
            &tactics,
            &profile.spelling,
            &morphology,
            profile.morphology.derivation,
        );
        (roots, tactics)
    }

    #[test]
    fn roots_fit_template_and_are_unique_within_fields() {
        for profile in SoundProfile::presets() {
            for seed in 0..200 {
                let (roots, tactics) = mint(seed, &profile);
                let mut seen: HashSet<(Field, &Form)> = HashSet::new();
                for m in &roots {
                    if m.derived.is_none() {
                        assert!(
                            tactics.fits_root(&m.form),
                            "{} seed {seed}: {} /{}/ breaks the root template",
                            profile.id,
                            m.concept.id,
                            m.form.ipa()
                        );
                    }
                    assert!(
                        seen.insert((m.concept.field, &m.form)),
                        "{} seed {seed}: /{}/ repeats within {:?}",
                        profile.id,
                        m.form.ipa(),
                        m.concept.field
                    );
                }
            }
        }
    }

    #[test]
    fn minting_is_reproducible() {
        let profile = SoundProfile::by_id("germanic").unwrap();
        assert_eq!(mint(42, &profile).0, mint(42, &profile).0);
        assert_ne!(mint(42, &profile).0, mint(43, &profile).0);
    }

    fn word<'a>(roots: &'a [Minted], id: &str) -> &'a Minted {
        roots.iter().find(|m| m.concept.id == id).unwrap()
    }

    #[test]
    fn sound_symbolism_is_a_tendency() {
        // "small" should contain /i/ clearly more often than other
        // property words, but not always.
        let profile = SoundProfile::by_id("germanic").unwrap();
        let i = CATALOG.id_by_ipa("i").unwrap();
        let (mut small, mut other, mut other_total) = (0, 0, 0);
        let seeds = 1000;
        for seed in 0..seeds {
            for m in mint(seed, &profile).0 {
                let has_i = m.form.phones().any(|p| p == i);
                match m.concept.id {
                    "small" => small += usize::from(has_i),
                    _ if m.concept.iconic.is_none() => {
                        other += usize::from(has_i);
                        other_total += 1;
                    }
                    _ => {}
                }
            }
        }
        let small_rate = small as f64 / seeds as f64;
        let base_rate = other as f64 / other_total as f64;
        assert!(
            small_rate > base_rate * 1.5,
            "small {small_rate:.2} vs base {base_rate:.2}"
        );
        assert!(
            small_rate < 0.95,
            "small {small_rate:.2} should not be fixed"
        );
    }

    /// Mother and father usually follow the nursery pattern: a nasal (for
    /// mother) or a lip or tongue-tip stop (for father) with the most open
    /// vowel, often doubled.
    #[test]
    fn parent_words_follow_the_nursery_pattern() {
        let profile = SoundProfile::base();
        let (mut nasal_mother, mut stop_father, mut open) = (0, 0, 0);
        let seeds = 300;
        for seed in 0..seeds {
            let roots = mint(seed, &profile).0;
            let first = |id: &str| word(&roots, id).form.segs[0].phone;
            let manner = |p| CATALOG.get(p).consonant().map(|c| c.manner);
            nasal_mother += usize::from(manner(first("mother")) == Some(Manner::Nasal));
            stop_father += usize::from(manner(first("father")) == Some(Manner::Stop));
            let vowel = word(&roots, "mother").form.segs[1].phone;
            open += usize::from(
                matches!(CATALOG.get(vowel), Segment::Vowel(v) if v.height == Height::Open),
            );
        }
        let rate = |n: usize| n as f32 / seeds as f32;
        assert!(
            rate(nasal_mother) > 0.7,
            "mother starts with a nasal {:.2}",
            rate(nasal_mother)
        );
        assert!(
            rate(stop_father) > 0.7,
            "father starts with a stop {:.2}",
            rate(stop_father)
        );
        assert!(
            rate(open) > 0.6,
            "mother has an open vowel {:.2}",
            rate(open)
        );
    }

    /// Basic meanings get shorter words than specialist ones.
    #[test]
    fn common_meanings_get_shorter_words() {
        let profile = SoundProfile::base();
        let (mut core, mut core_n, mut special, mut special_n) = (0, 0, 0, 0);
        for seed in 0..200 {
            for m in mint(seed, &profile)
                .0
                .iter()
                .filter(|m| m.derived.is_none())
            {
                let len = m.form.syllables().len();
                if m.concept.stability.is_some_and(|r| r <= 30) {
                    core += len;
                    core_n += 1;
                } else if m.concept.tier == Some(crate::concepts::Tier::Specialized) {
                    special += len;
                    special_n += 1;
                }
            }
        }
        let (core, special) = (
            core as f32 / core_n as f32,
            special as f32 / special_n as f32,
        );
        assert!(
            special > core + 0.4,
            "specialist {special:.2} vs core {core:.2} syllables"
        );
    }

    /// Repeated consonants and reduplication cluster in expressive meanings.
    #[test]
    fn repeats_belong_to_expressive_meanings() {
        let profile = SoundProfile::base();
        let repeats = |f: &Form| {
            let cs: Vec<_> = f.phones().filter(|p| !CATALOG.get(*p).is_vowel()).collect();
            cs.iter().enumerate().any(|(i, c)| cs[i + 1..].contains(c))
        };
        let (mut expressive, mut expressive_n, mut plain, mut plain_n) = (0, 0, 0, 0);
        for seed in 0..300 {
            for m in mint(seed, &profile)
                .0
                .iter()
                .filter(|m| m.derived.is_none())
            {
                if m.concept.expressive {
                    expressive += usize::from(repeats(&m.form));
                    expressive_n += 1;
                } else {
                    plain += usize::from(repeats(&m.form));
                    plain_n += 1;
                }
            }
        }
        let (e, p) = (
            expressive as f32 / expressive_n as f32,
            plain as f32 / plain_n as f32,
        );
        assert!(
            e > 0.3 && p < 0.05 && e > 8.0 * p,
            "expressive {e:.3}, plain {p:.3}"
        );
    }

    /// Derived words contain their base in a concatenative language and
    /// share its consonant skeleton in a root-and-pattern one.
    #[test]
    fn word_families_show_their_bases() {
        let neutral = SoundProfile::base();
        let semitic = neutral.flavored(&Flavor::by_id("semitic").unwrap());
        let mut derived = 0;
        for seed in 0..100 {
            for (profile, patterned) in [(&neutral, false), (&semitic, true)] {
                let roots = mint(seed, profile).0;
                for m in roots.iter().filter(|m| m.derived.is_some()) {
                    derived += 1;
                    let base = &word(&roots, m.derived.unwrap().0.id).form;
                    if patterned {
                        // The base's own consonants appear in order, around
                        // any pattern prefix, vowels, or drawn root consonants.
                        let mut wanted = base
                            .phones()
                            .filter(|p| !CATALOG.get(*p).is_vowel())
                            .peekable();
                        for p in m.form.phones() {
                            if wanted.peek() == Some(&p) {
                                wanted.next();
                            }
                        }
                        assert!(
                            wanted.peek().is_none(),
                            "/{}/ lacks the consonants of /{}/",
                            m.form.ipa(),
                            base.ipa()
                        );
                    } else {
                        let consonants = |f: &Form| -> Vec<_> {
                            f.phones().filter(|p| !CATALOG.get(*p).is_vowel()).collect()
                        };
                        let (whole, part) = (consonants(&m.form), consonants(base));
                        assert!(
                            whole
                                .windows(part.len().max(1))
                                .any(|w| w == part.as_slice()),
                            "/{}/ hides its base /{}/",
                            m.form.ipa(),
                            base.ipa()
                        );
                    }
                }
            }
        }
        assert!(
            derived > 500,
            "families are common: {derived} derived words"
        );
    }
}

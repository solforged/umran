use crate::concepts::{CONCEPTS, Concept, Field};
use crate::form::Form;
use crate::phonotactics::Phonotactics;
use crate::rng::{key, stream};
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// Draws a root gets to avoid every existing form; after these, a homophone
/// of a word in another semantic field is accepted, as happens in real
/// languages.
const FRESH_TRIES: usize = 4;
/// Draws before giving up on field uniqueness. From halfway on, roots are
/// forced to CVCV, which multiplies the space of possible forms.
const FIELD_TRIES: usize = 256;

/// One root per concept. Each concept draws from its own random stream, so
/// adding concepts later never changes how earlier draws begin.
pub fn mint_roots(seed: u64, tactics: &Phonotactics) -> Vec<(&'static Concept, Form)> {
    let mut used: HashSet<Form> = HashSet::new();
    let mut by_field: HashMap<Field, HashSet<Form>> = HashMap::new();
    CONCEPTS
        .iter()
        .map(|concept| {
            let mut rng = stream(seed, &[key("root"), key(concept.id)]);
            let field = by_field.entry(concept.field).or_default();
            let form = mint_one(&mut rng, tactics, concept, &used, field);
            used.insert(form.clone());
            field.insert(form.clone());
            (concept, form)
        })
        .collect()
}

fn mint_one(
    rng: &mut impl Rng,
    tactics: &Phonotactics,
    concept: &Concept,
    used: &HashSet<Form>,
    field: &HashSet<Form>,
) -> Form {
    let mut form = Form::default();
    for attempt in 0..FIELD_TRIES {
        let disyllabic =
            attempt >= FIELD_TRIES / 2 || rng.r#gen::<f32>() < tactics.disyllabic_roots;
        form = tactics.root(rng, concept.iconic, disyllabic);
        let fresh_enough = attempt >= FRESH_TRIES || !used.contains(&form);
        if fresh_enough && !field.contains(&form) {
            break;
        }
    }
    form
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::Inventory;
    use crate::phoneme::CATALOG;
    use crate::preset::Preset;

    fn tactics(seed: u64, preset: &Preset) -> Phonotactics {
        let inventory =
            Inventory::sample(&preset.inventory, &mut stream(seed, &[key("inventory")]));
        Phonotactics::compile(&preset.phonotactics, &inventory)
    }

    #[test]
    fn roots_fit_template_and_are_unique_within_fields() {
        for preset in Preset::all() {
            for seed in 0..1000 {
                let tactics = tactics(seed, &preset);
                let roots = mint_roots(seed, &tactics);
                let mut seen: HashSet<(Field, &Form)> = HashSet::new();
                for (concept, form) in &roots {
                    assert!(
                        tactics.fits_root(form),
                        "{} seed {seed}: {} /{}/ breaks the root template",
                        preset.id,
                        concept.id,
                        form.ipa()
                    );
                    assert!(
                        seen.insert((concept.field, form)),
                        "{} seed {seed}: /{}/ repeats within {:?}",
                        preset.id,
                        form.ipa(),
                        concept.field
                    );
                }
            }
        }
    }

    #[test]
    fn minting_is_reproducible() {
        let preset = Preset::by_id("elvish").unwrap();
        let a = mint_roots(42, &tactics(42, &preset));
        let b = mint_roots(42, &tactics(42, &preset));
        assert_eq!(a, b);
        let c = mint_roots(43, &tactics(43, &preset));
        assert_ne!(a, c);
    }

    #[test]
    fn sound_symbolism_is_a_tendency() {
        // "small" should contain /i/ clearly more often than other
        // property words, but not always.
        let preset = Preset::by_id("elvish").unwrap();
        let i = CATALOG.id_by_ipa("i").unwrap();
        let (mut small, mut other, mut other_total) = (0, 0, 0);
        let seeds = 2000;
        for seed in 0..seeds {
            for (concept, form) in mint_roots(seed, &tactics(seed, &preset)) {
                let has_i = form.phones().any(|p| p == i);
                match concept.id {
                    "small" => small += usize::from(has_i),
                    _ if concept.iconic.is_none() => {
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
}

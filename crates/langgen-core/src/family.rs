use crate::aesthetic::Aesthetic;
use crate::change::{apply_changes, SoundChange};
use crate::generate::{Generator, Word};
use crate::inventory::Inventory;
use crate::lexicon::{mint_roots, Root};
use crate::Language;

#[derive(Clone, Debug)]
pub struct BranchSpec {
    pub id: &'static str,
    pub parent: Option<&'static str>,
    pub aesthetic: Aesthetic,
    pub changes: Vec<SoundChange>,
}

#[derive(Clone, Debug)]
pub struct Branch {
    pub id: String,
    pub parent: Option<String>,
    pub language: Language,
    pub cognates: Vec<(String, Word)>,
}

#[derive(Clone, Debug)]
pub struct Family {
    pub seed: u64,
    pub proto: Language,
    pub roots: Vec<Root>,
    pub branches: Vec<Branch>,
}

impl Family {
    pub fn grow(seed: u64, proto_aesthetic: Aesthetic, specs: &[BranchSpec]) -> Self {
        let proto = Language::new(seed, proto_aesthetic);
        let roots = mint_roots(&proto);
        let mut branches = Vec::with_capacity(specs.len());
        for spec in specs {
            let source = source_words(spec, &roots, &branches);
            let cognates: Vec<(String, Word)> = source
                .into_iter()
                .map(|(id, word)| (id, apply_changes(&word, &spec.changes)))
                .collect();
            let mut inventory =
                Inventory::from_ids(cognates.iter().flat_map(|(_, w)| w.phonemes()));
            if inventory.consonants.is_empty() {
                inventory = Inventory::from_ids(
                    proto
                        .inventory
                        .consonants
                        .iter()
                        .copied()
                        .chain(inventory.vowels.iter().copied()),
                );
            }
            let generator = Generator::compile(&spec.aesthetic, &inventory);
            let mut language = Language {
                seed,
                aesthetic: spec.aesthetic.clone(),
                inventory,
                generator,
                autonym: String::new(),
            };
            language.autonym = autonym_from(&language, &cognates);
            branches.push(Branch {
                id: spec.id.to_string(),
                parent: spec.parent.map(str::to_string),
                language,
                cognates,
            });
        }
        Self {
            seed,
            proto,
            roots,
            branches,
        }
    }
}

fn source_words(spec: &BranchSpec, roots: &[Root], branches: &[Branch]) -> Vec<(String, Word)> {
    if let Some(parent_id) = spec.parent {
        if let Some(parent) = branches.iter().find(|b| b.id == parent_id) {
            return parent.cognates.clone();
        }
    }
    roots
        .iter()
        .map(|r| (r.id.to_string(), r.proto.clone()))
        .collect()
}

fn autonym_from(language: &Language, cognates: &[(String, Word)]) -> String {
    let romanize = |want: &str| {
        cognates
            .iter()
            .find(|(id, _)| id == want)
            .map(|(_, w)| language.romanize(w))
    };
    if let Some(s) = romanize("people") {
        return s;
    }
    if let Some(s) = romanize("person") {
        return s;
    }
    cognates
        .first()
        .map(|(_, w)| language.romanize(w))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change::{Env, Matcher, Rewrite};
    use crate::generate::Syllable;
    use crate::phoneme::{PhonemeId, CATALOG};
    use crate::Aesthetic;

    fn flattened_ipa(word: &Word) -> String {
        let ids: Vec<_> = word.phonemes().collect();
        CATALOG.ipa_string(&ids)
    }

    fn apocope() -> SoundChange {
        SoundChange {
            id: "apocope".into(),
            target: Matcher::AnyVowel,
            result: Rewrite::Delete,
            left: Env::Any,
            right: Env::WordEdge,
        }
    }

    fn elvish_apocope_spec(aesthetic: Aesthetic) -> BranchSpec {
        BranchSpec {
            id: "daughter",
            parent: None,
            aesthetic,
            changes: vec![apocope()],
        }
    }

    fn id(ipa: &str) -> PhonemeId {
        CATALOG.id_by_ipa(ipa).unwrap()
    }

    fn cv(c: &str, v: &str) -> Syllable {
        Syllable {
            onset: vec![id(c)],
            nucleus: vec![id(v)],
            coda: Vec::new(),
            long: false,
        }
    }

    fn word(syllables: Vec<Syllable>) -> Word {
        Word {
            syllables,
            join_at: None,
        }
    }

    #[test]
    fn same_seed_same_cognate_ipa() {
        let aesthetic = Aesthetic::by_id("elvish").unwrap();
        let specs = [elvish_apocope_spec(aesthetic.clone())];
        let a = Family::grow(42, aesthetic.clone(), &specs);
        let b = Family::grow(42, aesthetic, &specs);
        assert_eq!(a.branches.len(), 1);
        assert_eq!(b.branches.len(), 1);
        assert_eq!(a.cognate_ipas(), b.cognate_ipas());
        assert_eq!(a.proto_ipas(), b.proto_ipas());
    }

    #[test]
    fn apocope_changes_a_root_or_hand_built_word() {
        let aesthetic = Aesthetic::by_id("elvish").unwrap();
        let specs = [elvish_apocope_spec(aesthetic.clone())];
        let a = Family::grow(42, aesthetic.clone(), &specs);
        let b = Family::grow(42, aesthetic, &specs);
        assert_eq!(a.cognate_ipas(), b.cognate_ipas());

        let differed = a
            .roots
            .iter()
            .zip(a.branches[0].cognates.iter())
            .any(|(root, (_, evolved))| flattened_ipa(&root.proto) != flattened_ipa(evolved));
        if !differed {
            let proto = word(vec![cv("k", "a"), cv("t", "a")]);
            let out = apply_changes(&proto, &[apocope()]);
            assert_ne!(flattened_ipa(&proto), flattened_ipa(&out));
        }
    }

    impl Family {
        fn cognate_ipas(&self) -> Vec<(String, String)> {
            self.branches
                .iter()
                .flat_map(|b| {
                    b.cognates
                        .iter()
                        .map(|(id, w)| (id.clone(), flattened_ipa(w)))
                })
                .collect()
        }

        fn proto_ipas(&self) -> Vec<String> {
            self.roots.iter().map(|r| flattened_ipa(&r.proto)).collect()
        }
    }
}

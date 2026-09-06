use serde::Serialize;

use crate::Aesthetic;
use crate::Language;
use crate::Word;
use crate::change::{SoundChange, apply_changes};
use crate::change_packs::journey;
use crate::lexicon::{Root, mint_roots};

#[derive(Clone, Debug)]
pub struct Session {
    pub language: Language,
    pub roots: Vec<Root>,
    pub journey: Vec<SoundChange>,
    pub step: usize,
}

#[derive(Serialize)]
pub struct LexRow {
    pub id: String,
    pub form: String,
    pub proto: String,
}

#[derive(Serialize)]
pub struct SessionSnapshot {
    pub seed: u64,
    pub aesthetic: String,
    pub step: usize,
    pub rule: Option<String>,
    pub autonym: String,
    pub lexicon: Vec<LexRow>,
    pub people: Vec<String>,
    pub places: Vec<String>,
}

impl Session {
    pub fn open(seed: u64, aesthetic: Aesthetic) -> Self {
        Self::from_language(Language::new(seed, aesthetic))
    }

    pub fn from_language(mut language: Language) -> Self {
        let roots = mint_roots(&language);
        let journey = journey(&language.aesthetic.id);
        language.autonym = autonym(&language, &roots, 0, &[]);
        Self {
            language,
            roots,
            journey,
            step: 0,
        }
    }

    pub fn set_step(&mut self, step: usize) {
        self.step = step.min(self.journey.len());
        let forms = self.forms();
        self.language.autonym = autonym(&self.language, &self.roots, self.step, &forms);
    }

    pub fn forward(&mut self) -> bool {
        if self.step >= self.journey.len() {
            return false;
        }
        self.set_step(self.step + 1);
        true
    }

    pub fn back(&mut self) -> bool {
        if self.step == 0 {
            return false;
        }
        self.set_step(self.step - 1);
        true
    }

    pub fn rule(&self) -> Option<&SoundChange> {
        (self.step > 0).then(|| &self.journey[self.step - 1])
    }

    pub fn forms(&self) -> Vec<(String, Word)> {
        self.forms_at(self.step)
    }

    pub fn forms_at(&self, step: usize) -> Vec<(String, Word)> {
        let step = step.min(self.journey.len());
        if step == 0 {
            return self
                .roots
                .iter()
                .map(|r| (r.id.to_string(), r.proto.clone()))
                .collect();
        }
        let rules = &self.journey[..step];
        self.roots
            .iter()
            .map(|r| (r.id.to_string(), apply_changes(&r.proto, rules)))
            .collect()
    }

    pub fn previous_forms(&self) -> Vec<(String, Word)> {
        if self.step == 0 {
            self.forms_at(0)
        } else {
            self.forms_at(self.step - 1)
        }
    }

    pub fn changed_ids(&self) -> Vec<String> {
        if self.step == 0 {
            return Vec::new();
        }
        let now = self.forms();
        let prev = self.previous_forms();
        now.iter()
            .zip(prev.iter())
            .filter(|(a, b)| a.1.phones() != b.1.phones())
            .map(|(a, _)| a.0.clone())
            .collect()
    }

    pub fn people(&self) -> Vec<String> {
        let forms = self.forms();
        let mut out = Vec::new();
        for id in ["person", "people", "child", "king"] {
            push_cap(&mut out, &self.language, &forms, id);
        }
        if let Some(person) = find(&forms, "person") {
            let stem = person.first_syllable();
            for ending in &self.language.aesthetic.names.person_endings {
                let named = stem.attach_ipa(ending, &self.language.inventory);
                push_unique(&mut out, crate::capitalize(self.language.romanize(&named)));
            }
        }
        out
    }

    pub fn places(&self) -> Vec<String> {
        let forms = self.forms();
        let mut out = Vec::new();
        for id in ["hill", "river", "sea", "house", "path", "earth", "sky"] {
            push_cap(&mut out, &self.language, &forms, id);
        }
        if let (Some(hill), Some(river)) = (find(&forms, "hill"), find(&forms, "river")) {
            push_unique(
                &mut out,
                crate::capitalize(
                    self.language
                        .romanize(&hill.first_syllable().compound(&river.first_syllable())),
                ),
            );
        }
        out
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        let forms = self.forms();
        let lexicon = self
            .roots
            .iter()
            .zip(forms.iter())
            .map(|(r, (_, w))| LexRow {
                id: r.id.to_string(),
                form: self.language.romanize(w),
                proto: self.language.romanize(&r.proto),
            })
            .collect();
        SessionSnapshot {
            seed: self.language.seed,
            aesthetic: self.language.aesthetic.id.clone(),
            step: self.step,
            rule: self.rule().map(|r| r.id.clone()),
            autonym: self.language.autonym.clone(),
            lexicon,
            people: self.people(),
            places: self.places(),
        }
    }
}

fn find<'a>(forms: &'a [(String, Word)], id: &str) -> Option<&'a Word> {
    forms.iter().find(|(i, _)| i == id).map(|(_, w)| w)
}

fn push_cap(out: &mut Vec<String>, language: &Language, forms: &[(String, Word)], id: &str) {
    if let Some(word) = find(forms, id) {
        push_unique(out, crate::capitalize(language.romanize(word)));
    }
}

fn push_unique(out: &mut Vec<String>, s: String) {
    if !s.is_empty() && !out.contains(&s) {
        out.push(s);
    }
}

fn autonym(language: &Language, roots: &[Root], step: usize, forms: &[(String, Word)]) -> String {
    if step > 0 {
        for id in ["people", "person"] {
            if let Some(word) = find(forms, id) {
                let s = crate::capitalize(language.romanize(word));
                if !s.is_empty() {
                    return s;
                }
            }
        }
    }
    for id in ["people", "person"] {
        if let Some(root) = roots.iter().find(|r| r.id == id) {
            let s = crate::capitalize(language.romanize(&root.proto));
            if !s.is_empty() {
                return s;
            }
        }
    }
    language.autonym.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_session() {
        let a = Aesthetic::by_id("elvish").unwrap();
        let s1 = Session::open(42, a.clone());
        let s2 = Session::open(42, a);
        assert_eq!(s1.snapshot().lexicon[0].form, s2.snapshot().lexicon[0].form);
        assert_eq!(s1.people(), s2.people());
        assert_eq!(s1.journey.len(), s2.journey.len());
    }

    #[test]
    fn walking_applies_one_rule_at_a_time() {
        let mut session = Session::open(42, Aesthetic::by_id("elvish").unwrap());
        assert_eq!(session.step, 0);
        assert!(session.rule().is_none());
        let proto: Vec<_> = session
            .forms()
            .into_iter()
            .map(|(id, w)| (id, session.language.romanize(&w)))
            .collect();
        assert!(session.forward());
        assert_eq!(session.step, 1);
        assert_eq!(
            session.rule().map(|r| r.id.as_str()),
            Some("t_to_s_before_i")
        );
        let now: Vec<_> = session
            .forms()
            .into_iter()
            .map(|(id, w)| (id, session.language.romanize(&w)))
            .collect();
        assert_eq!(proto.len(), now.len());
        session.set_step(session.journey.len());
        assert_eq!(session.step, session.journey.len());
        assert!(!session.forward());
        assert!(session.back());
    }

    #[test]
    fn previous_forms_are_prior_checkpoint() {
        let mut session = Session::open(42, Aesthetic::by_id("elvish").unwrap());
        let proto: Vec<_> = session
            .forms_at(0)
            .into_iter()
            .map(|(_, w)| w.phones())
            .collect();
        session.set_step(1);
        let prev: Vec<_> = session
            .previous_forms()
            .into_iter()
            .map(|(_, w)| w.phones())
            .collect();
        assert_eq!(prev, proto);
        assert_eq!(session.forms_at(session.step).len(), proto.len());
    }

    #[test]
    fn snapshot_records_step_and_seed() {
        let mut session = Session::open(7, Aesthetic::by_id("kuo-toa").unwrap());
        session.set_step(2);
        let snap = session.snapshot();
        assert_eq!(snap.seed, 7);
        assert_eq!(snap.aesthetic, "kuo-toa");
        assert_eq!(snap.step, 2);
        assert!(!snap.lexicon.is_empty());
        assert!(!snap.people.is_empty());
        assert!(!snap.places.is_empty());
    }
}

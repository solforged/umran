//! Author-defined meanings and words in a world's replay.
use crate::{
    ConceptId, Form, LexemeId, Meaning, MeaningError, Origin, Root, RootError, World, WorldEvent,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WordError {
    Root(RootError),
    Pronoun,
    Dead,
}

impl World {
    pub fn define_meaning(&mut self, meaning: Meaning) -> Result<ConceptId, MeaningError> {
        let concept = self.concepts.append(meaning)?;
        for variety in &mut self.varieties {
            variety.lexicon.extend(&self.concepts);
        }
        self.events
            .push((self.generation, WorldEvent::MeaningDefined { concept }));
        Ok(concept)
    }

    pub fn check_word(&self, variety: usize, root: &Root) -> Result<(ConceptId, Form), WordError> {
        let concept = self
            .concepts
            .by_id(&root.concept)
            .ok_or(WordError::Root(RootError::UnknownConcept))?;
        if crate::pronouns::is_pronoun(concept) {
            return Err(WordError::Pronoun);
        }
        if !self
            .communities
            .iter()
            .any(|c| c.living() && c.variety == variety)
        {
            return Err(WordError::Dead);
        }
        let speech = &self.varieties[variety];
        let form = root.form().map_err(WordError::Root)?;
        let (consonants, vowels) = speech.inventory();
        if let Some(phone) = form
            .phones()
            .find(|p| !consonants.contains(p) && !vowels.contains(p))
        {
            return Err(WordError::Root(RootError::OutsideInventory(
                crate::CATALOG.get(phone).ipa().into(),
            )));
        }
        Ok((concept, form))
    }

    pub fn give_word(&mut self, variety: usize, root: &Root) -> Result<LexemeId, WordError> {
        let (concept, form) = self.check_word(variety, root)?;
        let lexicon = &mut self.varieties[variety].lexicon;
        let word = lexicon.coin(form, Origin::Authored, concept, self.generation);
        lexicon.get_mut(word).authored = true;
        // Keep incumbents in competition, but make the author's form dominant now.
        lexicon.slot_mut(concept).introduce(word, 0.75);
        self.varieties[variety].sync_grammar(self.generation);
        self.events.push((
            self.generation,
            WorldEvent::WordAuthored {
                variety,
                concept,
                word,
            },
        ));
        Ok(word)
    }
}

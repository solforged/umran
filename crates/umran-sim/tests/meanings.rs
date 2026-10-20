use umran_sim::meanings::WordError;
use umran_sim::{
    Action, CATALOG, CONCEPTS, Chronicle, Class, ConceptId, ConceptTable, Derivation, Field, Form,
    LanguageDesign, MapSize, Meaning, MeaningError, Naming, Origin, Params, Relation, Root,
    RootError, SoundProfile, Tier, World,
};

fn hearth() -> Meaning {
    Meaning {
        id: "hearth".into(),
        gloss: "hearth".into(),
        field: Field::House,
        class: Class::Entity,
        countable: true,
        tier: Tier::Everyday,
        from: Some(Derivation {
            base: "fire".into(),
            relation: Relation::Place,
        }),
    }
}

fn found() -> Action {
    Action::Found {
        naming: Naming::People,
        design: LanguageDesign::preset("familiar", 7).unwrap(),
        seed: 7,
        power: 0.5,
        openness: 0.5,
        region: None,
        livelihood: None,
        ethos: None,
    }
}

#[test]
fn tables_keep_builtin_ids_and_owned_customs_local() {
    let mut table = ConceptTable::default();
    for (i, concept) in CONCEPTS.iter().enumerate() {
        assert_eq!(table.by_id(&concept.id), Some(ConceptId(i as u32)));
        assert_eq!(&table[ConceptId(i as u32)], concept);
    }
    let old = table.clone();
    let id = table.append(hearth()).unwrap();
    assert_eq!(id.0 as usize, CONCEPTS.len());
    assert!(old.by_id("hearth").is_none());
    assert!(table[id].categories.plural && table[id].categories.object);
    assert!(!table[id].categories.past);
    assert_eq!(table[id].stability, None);
    assert_eq!(
        table[id].borrowability(),
        CONCEPTS
            .iter()
            .find(|c| c.tier == Some(Tier::Everyday))
            .unwrap()
            .borrowability()
    );
    for name in ["smith", "oath", "hearth"] {
        let mut meaning = hearth();
        meaning.id = name.into();
        assert_eq!(table.append(meaning), Err(MeaningError::Taken));
    }
    for id in [
        "",
        "Hearth",
        "with space",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    ] {
        let mut meaning = hearth();
        meaning.id = id.into();
        assert_eq!(old.check(&meaning), Err(MeaningError::BadId));
    }
    for gloss in ["", " ", "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"] {
        let mut meaning = hearth();
        meaning.gloss = gloss.into();
        assert_eq!(old.check(&meaning), Err(MeaningError::BadGloss));
    }
    let mut meaning = hearth();
    meaning.class = Class::Function;
    assert_eq!(old.check(&meaning), Err(MeaningError::BadClass));
    for base in ["absent", "1sg"] {
        let mut meaning = hearth();
        meaning.from.as_mut().unwrap().base = base.into();
        assert_eq!(old.check(&meaning), Err(MeaningError::UnknownBase));
    }
    let encoded = serde_json::to_value(hearth()).unwrap();
    assert_eq!(encoded["class"], "entity");
    assert_eq!(encoded["field"], "house");
    assert_eq!(encoded["tier"], "everyday");
    assert_eq!(encoded["from"]["relation"], "place");
    assert_eq!(
        serde_json::from_value::<Meaning>(encoded).unwrap(),
        hearth()
    );
}

#[test]
fn pre_found_meanings_have_roots_in_every_language_and_can_derive() {
    let mut derived = 0;
    for seed in 0..20 {
        let mut world = World::new(seed, Params::static_society());
        let id = world.define_meaning(hearth()).unwrap();
        for profile in SoundProfile::presets().iter().take(3) {
            world.found(profile, 0.5, 0.5);
        }
        for variety in &world.varieties {
            assert_eq!(variety.lexicon.slots.len(), world.concepts.len());
            let word = variety.lexicon.word_for(id).unwrap();
            if let Origin::Derived { base, relation } = word.origin {
                assert_eq!(relation, Relation::Place);
                assert_eq!(
                    variety.lexicon.get(base).first_sense,
                    world.concepts.by_id("fire").unwrap()
                );
                derived += 1;
            }
        }
    }
    assert!(derived > 0, "custom founding derivatives: {derived}");
}

#[test]
fn late_meaning_reaches_every_living_language_in_forty_seed_band() {
    let (mut complete, mut derived, mut languages) = (0, 0, 0);
    for seed in 0..40 {
        let mut world = World::new(seed, Params::default());
        for profile in SoundProfile::presets().iter().take(3) {
            world.found(profile, 0.5, 0.5);
        }
        world.run(40);
        let id = world.define_meaning(hearth()).unwrap();
        world.run(16);
        let mut spoken: Vec<_> = world
            .living()
            .map(|c| world.communities[c].variety)
            .collect();
        spoken.sort_unstable();
        spoken.dedup();
        assert!(!spoken.is_empty());
        let mut all = true;
        for v in spoken {
            languages += 1;
            let word = world.varieties[v].lexicon.word_for(id);
            all &= word.is_some();
            derived += world.varieties[v]
                .lexicon
                .lexemes
                .iter()
                .filter(|w| {
                    w.first_sense == id
                        && matches!(
                            w.origin,
                            Origin::Derived {
                                relation: Relation::Place,
                                ..
                            }
                        )
                })
                .count();
        }
        complete += usize::from(all);
    }
    println!(
        "meanings band: {complete}/40 seeds ({:.1}%) have words in all {languages} living languages by year 1400; {derived} derived attestations",
        complete as f32 * 2.5
    );
    assert!(complete >= 36);
    assert!(derived > 0);
}

#[test]
fn authored_words_are_atomic_dominant_and_replay_with_later_laws() {
    let mut history = Chronicle::new(7, MapSize::Small);
    history.act(Action::Meaning { meaning: hearth() }).unwrap();
    history.act(found()).unwrap();
    let table = &history.latest().concepts;
    let concept = table.by_id("hearth").unwrap();
    let old = history.latest().varieties[0]
        .lexicon
        .word_for(concept)
        .unwrap()
        .id;
    let speech = &history.latest().varieties[0];
    // Choose an actually present segment with a catalog sound change.
    let (consonants, vowels) = speech.inventory();
    let ipa = consonants
        .iter()
        .find_map(|c| {
            vowels.iter().find_map(|v| {
                let ipa = format!(
                    "{}{}{}{}",
                    CATALOG.get(*c).ipa(),
                    CATALOG.get(*v).ipa(),
                    CATALOG.get(*c).ipa(),
                    CATALOG.get(*v).ipa()
                );
                let form = Form::from_ipa(&ipa).unwrap();
                history
                    .latest()
                    .law_catalog()
                    .iter()
                    .find(|law| {
                        law.id != "koine-levelling"
                            && law.apply(&form, speech.minimal, speech.stress()) != form
                    })
                    .map(|_| ipa)
            })
        })
        .expect("an applicable sound law");
    history
        .act(Action::Word {
            variety: 0,
            concept: "hearth".into(),
            ipa: ipa.clone(),
        })
        .unwrap();
    let lexicon = &history.latest().varieties[0].lexicon;
    let word = lexicon.word_for(concept).unwrap();
    let authored = word.id;
    assert!(word.authored);
    assert_eq!(word.origin, Origin::Authored);
    assert_eq!(word.form, Form::from_ipa(&ipa).unwrap());
    assert!(
        lexicon
            .slot(concept)
            .variants
            .iter()
            .any(|v| v.lexeme == old)
    );
    history.act(Action::Run { generations: 1 }).unwrap();
    // Natural change during the intervening generation can consume the first
    // opportunity. Choose a law against the form at this later reading.
    let speech = &history.latest().varieties[0];
    let before_law = speech.lexicon.get(authored).form.clone();
    let law = history
        .latest()
        .law_catalog()
        .iter()
        .find(|law| {
            law.id != "koine-levelling"
                && law.apply(&before_law, speech.minimal, speech.stress()) != before_law
                && law
                    .assess_weighted(
                        speech.grammar.forms(&speech.lexicon),
                        &speech.profile.inventory,
                        speech.minimal,
                        speech.stress(),
                    )
                    .is_some()
        })
        .expect("a later law changes the authored form")
        .id
        .to_string();
    history.act(Action::Law { variety: 0, law }).unwrap();
    assert_ne!(
        history.latest().varieties[0].lexicon.get(authored).form,
        before_law
    );
    let recipe = serde_json::from_str(&serde_json::to_string(&history.recipe()).unwrap()).unwrap();
    let replay = Chronicle::from_recipe(&recipe).unwrap();
    assert_eq!(
        format!("{:?}", replay.latest()),
        format!("{:?}", history.latest())
    );
    let original = history.active();
    history
        .branch_at_point(umran_sim::HistoryPoint {
            action: 0,
            offset: 0,
        })
        .unwrap();
    assert!(history.latest().concepts.by_id("hearth").is_none());
    history.restore(original).unwrap();
    assert!(history.latest().concepts.by_id("hearth").is_some());
}

#[test]
fn authored_word_refusals_leave_the_world_untouched() {
    let mut world = World::solo(7, &SoundProfile::base(), Params::static_society());
    let root = |concept: &str, ipa: &str| Root {
        concept: concept.into(),
        ipa: ipa.into(),
    };
    let (consonants, vowels) = world.varieties[0].inventory();
    let outside = CATALOG
        .segments
        .iter()
        .find(|p| {
            !consonants
                .iter()
                .chain(&vowels)
                .any(|id| CATALOG.get(*id).ipa() == p.ipa())
        })
        .unwrap()
        .ipa();
    let vowel = vowels[0];
    let bad = [
        (
            root("absent", "a"),
            WordError::Root(RootError::UnknownConcept),
        ),
        (root("water", ""), WordError::Root(RootError::BadIpa(None))),
        (
            root("water", "a˥"),
            WordError::Root(RootError::BadIpa(Some("˥".into()))),
        ),
        (root("water", "p"), WordError::Root(RootError::NoVowel)),
        (
            root("water", &format!("{outside}{}", CATALOG.get(vowel).ipa())),
            WordError::Root(RootError::OutsideInventory(outside.into())),
        ),
        (root("1sg", "a"), WordError::Pronoun),
    ];
    for (root, expected) in bad {
        let before = format!("{world:?}");
        assert_eq!(world.give_word(0, &root), Err(expected));
        assert_eq!(format!("{world:?}"), before);
    }
    world.communities[0].ended = Some(0);
    assert_eq!(
        world.give_word(0, &root("water", "a")),
        Err(WordError::Dead)
    );
    assert_eq!(
        world.give_word(usize::MAX, &root("water", "a")),
        Err(WordError::Dead)
    );
    // Duplicate belongs to a founding root list, not to Word: Word deliberately
    // replaces the same meaning and retains its previous words as competitors.
    let mut design = LanguageDesign::preset("familiar", 7).unwrap();
    let form = design
        .sounds
        .iter()
        .find(|s| CATALOG.get(CATALOG.id_by_ipa(&s.ipa).unwrap()).is_vowel())
        .unwrap()
        .ipa
        .as_str();
    let duplicate = root("water", form);
    design.roots = vec![duplicate.clone(), duplicate.clone()];
    assert_eq!(
        design.validate(&ConceptTable::default()),
        Err(RootError::Duplicate.describe(&duplicate))
    );
}

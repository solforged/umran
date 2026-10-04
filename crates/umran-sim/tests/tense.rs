use rand::Rng;
use umran_sim::concepts::by_id;
use umran_sim::form::Form;
use umran_sim::grammar::{GrammarEvent, MarkerKind, MarkerOrigin, NoticeKind, Side};
use umran_sim::rng::{key, stream};
use umran_sim::{Category, GrammarChoice, MinimalWord, Params, SoundProfile, StressRule, World};

#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

const CATEGORIES: [Category; 2] = [Category::Future, Category::Progressive];

fn profile(choice: GrammarChoice) -> SoundProfile {
    let mut profile = SoundProfile::base();
    profile.grammar.future = Some(choice);
    profile.grammar.progressive = Some(choice);
    profile.stress = Some(StressRule::Initial);
    profile.phonotactics.open_medial = false;
    profile
}

#[test]
fn tense_particles_preserve_their_lexical_sources_and_verb_only_eligibility() {
    let world = World::solo(
        9,
        &profile(GrammarChoice::Particle),
        Params::static_society(),
    );
    let variety = &world.varieties[0];
    for category in CATEGORIES {
        let marker = variety
            .grammar
            .markers
            .iter()
            .find(|m| m.category == category)
            .unwrap();
        let MarkerOrigin::Grammaticalized {
            source,
            concept,
            source_form,
        } = &marker.origin
        else {
            panic!("a periphrastic tense must have a lexical source");
        };
        assert!(match category {
            Category::Future => ["go", "come", "have"].contains(&concept.id),
            Category::Progressive => concept.id == "stand",
            _ => unreachable!(),
        });
        assert_eq!(variety.lexicon.get(*source).form, *source_form);
        assert_eq!(marker.form, *source_form);
        for concept in ["see", "go", "stand"] {
            let word = variety.lexicon.word_for(by_id(concept).unwrap()).unwrap();
            let paradigm = word
                .paradigms
                .iter()
                .find(|p| p.category == category)
                .unwrap();
            let realization = &paradigm.realizations[0];
            let surface = variety.grammar.surface(&word.form, realization);
            let expected = match marker.side {
                Side::Prefix => vec![source_form.clone(), word.form.clone()],
                Side::Suffix => vec![word.form.clone(), source_form.clone()],
            };
            assert_eq!(surface, expected);
        }
        for concept in ["child", "dog", "water"] {
            let word = variety.lexicon.word_for(by_id(concept).unwrap()).unwrap();
            assert!(!word.paradigms.iter().any(|p| p.category == category));
        }
    }
}

#[test]
fn eroded_fused_tenses_are_renewed_without_erasing_the_old_competitor() {
    use umran_sim::change::{Env, Matcher, Rewrite, SoundChange};
    use umran_sim::laws::Law;
    for category in CATEGORIES {
        let mut world = World::solo(
            9,
            &profile(GrammarChoice::Particle),
            Params::static_society(),
        );
        let variety = &mut world.varieties[0];
        let particle = variety
            .grammar
            .markers
            .iter()
            .find(|m| m.category == category)
            .unwrap()
            .id;
        // A controlled CV particle and CV stems make complete ending loss observable.
        variety.grammar.markers[particle as usize].form = Form::from_ipa("ta").unwrap();
        variety.grammar.markers[particle as usize].side = Side::Suffix;
        for word in variety.lexicon.living().map(|w| w.id).collect::<Vec<_>>() {
            let word = variety.lexicon.get_mut(word);
            word.form = Form::from_ipa("pa").unwrap();
        }
        let fused = variety.grammar.fuse(
            particle,
            &mut variety.lexicon,
            &variety.morphology,
            StressRule::Initial,
            1,
        );
        // Study the stage after the auxiliary has yielded to its attached descendant.
        for word in &mut variety.lexicon.lexemes {
            for paradigm in word.paradigms.iter_mut().filter(|p| p.category == category) {
                for r in &mut paradigm.realizations {
                    if r.marker == particle {
                        r.retired = Some(2);
                        r.share = 0.0;
                    }
                    if r.marker == fused {
                        r.share = 1.0;
                    }
                }
            }
        }
        variety.grammar.markers[particle as usize].retired = Some(2);
        variety.grammar.markers[particle as usize].productive = false;
        let apocope = umran_sim::catalog()
            .into_iter()
            .find(|law| law.id == "apocope")
            .unwrap();
        let t = Form::from_ipa("t").unwrap().segs[0].phone;
        let loss = Law {
            id: "test-final-t-loss",
            label: "Final t disappears",
            rules: vec![SoundChange {
                id: "test-final-t-loss".into(),
                target: Matcher::Phone(t),
                result: Rewrite::Delete,
                left: Env::Any,
                right: Env::WordEdge,
            }],
            commonness: 1.0,
            stress: None,
        };
        for (generation, law) in [(3, &apocope), (4, &loss)] {
            variety.grammar.apply_law(
                &mut variety.lexicon,
                law,
                MinimalWord::Syllable,
                StressRule::Initial,
                generation,
            );
        }
        assert_eq!(
            variety.grammar.summary.categories[category.position()].contrast_retention,
            0.0
        );
        assert!(variety.grammar.marker(fused).form.segs.is_empty());
        let generation = 5;
        let seed = (0..10000)
            .find(|&seed| {
                let mut rng = stream(
                    seed,
                    &[key("grammar rebuilding"), 0, generation, key(category.id())],
                );
                let draw = rng.r#gen::<f32>();
                (0.0007..0.0087).contains(&draw)
            })
            .unwrap();
        variety.grammar.evolve(
            seed,
            0,
            generation as u32,
            &mut variety.lexicon,
            &variety.morphology,
            StressRule::Initial,
            64,
            true,
        );
        let renewed = variety
            .grammar
            .events
            .iter()
            .find_map(|notice| match notice.event {
                NoticeKind::NewMarker { marker } if notice.category == category => Some(marker),
                _ => None,
            })
            .expect("loss must raise the rebuilding chance above the baseline");
        assert!(matches!(
            variety.grammar.marker(renewed).origin,
            MarkerOrigin::Grammaticalized { .. }
        ));
        let word = variety.lexicon.word_for(by_id("see").unwrap()).unwrap();
        let paradigm = word
            .paradigms
            .iter()
            .find(|p| p.category == category)
            .unwrap();
        let old = paradigm
            .realizations
            .iter()
            .find(|r| r.marker == fused)
            .unwrap();
        let new = paradigm
            .realizations
            .iter()
            .find(|r| r.marker == renewed)
            .unwrap();
        assert!(old.retired.is_none() && new.retired.is_none());
        assert!((old.share + new.share - 1.0).abs() < 1e-6);
        assert_eq!(
            variety.grammar.surface(&word.form, old),
            vec![word.form.clone()]
        );
        assert_eq!(
            variety.grammar.surface(&word.form, new),
            vec![
                word.form.clone(),
                variety.grammar.marker(renewed).form.clone()
            ]
        );
        assert!(
            old.history
                .iter()
                .any(|entry| matches!(entry.event, GrammarEvent::SoundLaw { .. }))
        );
        assert_eq!(
            variety.grammar.surface_at(&word.form, old, 1)[0].ipa(),
            "pata"
        );
    }
}

#[test]
fn static_society_leaves_tense_evolution_inert_but_not_sound_laws() {
    let mut world = World::solo(
        2,
        &profile(GrammarChoice::Particle),
        Params {
            sound_change_rate: 0.0,
            innovation_rate: 0.0,
            ..Params::static_society()
        },
    );
    let variety = &mut world.varieties[0];
    let particle = variety
        .grammar
        .markers
        .iter()
        .find(|m| m.category == Category::Future)
        .unwrap()
        .id;
    variety.grammar.fuse(
        particle,
        &mut variety.lexicon,
        &variety.morphology,
        StressRule::Initial,
        0,
    );
    let before: Vec<_> = variety
        .lexicon
        .living()
        .map(|word| {
            (
                word.id,
                word.paradigms
                    .iter()
                    .filter(|p| CATEGORIES.contains(&p.category))
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    world.run(160);
    for (id, paradigms) in before {
        let word = world.varieties[0].lexicon.get(id);
        if word.obsolete.is_none() {
            assert_eq!(
                word.paradigms
                    .iter()
                    .filter(|p| CATEGORIES.contains(&p.category))
                    .cloned()
                    .collect::<Vec<_>>(),
                paradigms
            );
        }
    }
    assert!(
        !world.varieties[0]
            .grammar
            .events
            .iter()
            .any(|e| CATEGORIES.contains(&e.category) && e.generation > 0)
    );
    let variety = &mut world.varieties[0];
    variety.grammar.markers[particle as usize].form = Form::from_ipa("mika").unwrap();
    let law = umran_sim::catalog()
        .into_iter()
        .find(|law| law.id == "apocope")
        .unwrap();
    variety.grammar.apply_law(
        &mut variety.lexicon,
        &law,
        MinimalWord::Syllable,
        StressRule::Initial,
        161,
    );
    assert_eq!(variety.grammar.marker(particle).form.ipa(), "mik");
}

#[test]
#[ignore = "40 seeds × 4000 years across three profiles, plus the full sample recipe"]
fn tense_aspect_bands_over_four_millennia() {
    let started = std::time::Instant::now();
    for preset in ["germanic", "semitic", "polynesian"] {
        let mut absent = [0; 2];
        let mut renewed = [0; 2];
        let mut fused = [0; 2];
        let mut retained = [0.0; 2];
        for seed in 0..40 {
            let mut world = World::solo(
                seed,
                &SoundProfile::by_id(preset).unwrap(),
                Params {
                    tense_aspect: true,
                    ..Params::static_society()
                },
            );
            for (i, category) in CATEGORIES.into_iter().enumerate() {
                absent[i] += usize::from(
                    world.varieties[0]
                        .grammar
                        .markers
                        .iter()
                        .any(|m| m.category == category && m.kind == MarkerKind::None),
                );
            }
            world.run(160);
            let grammar = &world.varieties[0].grammar;
            for (i, category) in CATEGORIES.into_iter().enumerate() {
                renewed[i] += grammar
                    .events
                    .iter()
                    .filter(|e| {
                        e.category == category && matches!(e.event, NoticeKind::NewMarker { .. })
                    })
                    .count();
                fused[i] += grammar
                    .events
                    .iter()
                    .filter(|e| {
                        e.category == category && matches!(e.event, NoticeKind::Fusion { .. })
                    })
                    .count();
                retained[i] += grammar.summary.categories[category.position()].contrast_retention;
            }
            assert_shares(&world);
        }
        println!(
            "{preset}: absent={absent:?}/40 new_particles={renewed:?} fusions={fused:?} mean_retention={:?}",
            retained.map(|n| n / 40.0)
        );
        for i in 0..2 {
            assert!(
                (6..=30).contains(&absent[i]),
                "founding must allow both marking and no marking"
            );
            assert!(
                (1..=45).contains(&renewed[i]),
                "renewal must occur, but not become obligatory"
            );
            assert!(
                (1..=25).contains(&fused[i]),
                "some auxiliaries must fuse, without universal fusion"
            );
            assert!((0.15..=0.9).contains(&(retained[i] / 40.0)));
        }
    }
    let sample = sample::sample();
    assert_eq!(sample.latest().generation, 160);
    assert_shares(sample.latest());
    for category in CATEGORIES {
        let spoken = sample.latest().spoken();
        let shares: Vec<_> = sample
            .latest()
            .varieties
            .iter()
            .enumerate()
            .filter(|(i, _)| spoken[*i])
            .map(|(_, v)| v.grammar.summary.categories[category.position()].contrast_retention)
            .collect();
        let retention = shares.iter().sum::<f32>() / shares.len() as f32;
        println!(
            "sample {}: spoken={} mean_retention={retention:.3}",
            category.id(),
            shares.len()
        );
        assert!((0.02..=0.75).contains(&retention));
    }
    println!(
        "tense band elapsed: {:.2}s",
        started.elapsed().as_secs_f64()
    );
}

fn assert_shares(world: &World) {
    for variety in &world.varieties {
        for word in variety.lexicon.living() {
            for paradigm in word
                .paradigms
                .iter()
                .filter(|p| CATEGORIES.contains(&p.category))
            {
                let active: Vec<_> = paradigm
                    .realizations
                    .iter()
                    .filter(|r| r.retired.is_none())
                    .collect();
                if active.is_empty() {
                    continue;
                }
                assert!(active.len() <= 3);
                assert!((active.iter().map(|r| r.share).sum::<f32>() - 1.0).abs() < 1e-5);
                for r in active {
                    assert!(r.share > 0.0 && r.share <= 1.0);
                    for form in variety.grammar.surface(&word.form, r) {
                        assert!(form.vowel_count() > 0);
                    }
                }
            }
        }
    }
}

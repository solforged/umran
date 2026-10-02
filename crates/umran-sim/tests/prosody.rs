use umran_sim::{
    CATALOG, Env, Event, Form, LanguageDesign, Law, Livelihood, Matcher, MinimalWord, Name, Params,
    Rewrite, SoundChange, SoundProfile, StressRule, Variety, World, catalog,
};

fn form(ipa: &str) -> Form {
    Form::from_ipa(ipa).unwrap()
}
fn law(id: &str) -> Law {
    catalog().into_iter().find(|l| l.id == id).unwrap()
}

#[test]
fn geminates_share_a_boundary_and_supply_weight() {
    let f = form("katːa");
    let syllables = f.syllables();
    assert_eq!(syllables[0].coda, 2..3);
    assert_eq!(syllables[1].onset, 2..3);
    assert_eq!(umran_sim::prosody::moras(&f), 3);
    assert_eq!(
        umran_sim::prosody::moras(&form("tːa")),
        1,
        "initial length is not a coda"
    );
    assert_eq!(
        form("katatːa").stressed_syllable(StressRule::Weight),
        Some(1)
    );
    assert_eq!(
        form("katata").stressed_syllable(StressRule::Weight),
        Some(0)
    );
    assert_eq!(
        form("kataːta").stressed_syllable(StressRule::Weight),
        Some(1)
    );
    assert_eq!(form("katːa").ipa_stressed(StressRule::Final), "kaˈtːa");
    assert_eq!(form("kaːtːa").ipa(), "kaːtːa");
}

#[test]
fn following_syllable_vowels_use_current_stress_and_shared_geminate_membership() {
    let i = CATALOG.id_by_ipa("i").unwrap();
    let lengthen = SoundChange {
        id: "lengthen-before-stressed-i".into(),
        target: Matcher::AnyVowel,
        result: Rewrite::Length(true),
        left: Env::Any,
        right: Env::FollowingSyllableVowel(Matcher::Stressed {
            target: Box::new(Matcher::Phone(i)),
            stressed: true,
        }),
    };
    let mut source = form("poltika");
    source.stress = Some(1);
    let after = lengthen.apply(&source, StressRule::Free);
    assert_eq!(after.ipa_stressed(StressRule::Free), "poːlˈtika");
    assert_eq!(after.stress, Some(1));
    assert_eq!(lengthen.apply(&source, StressRule::Final), source);

    let voice = SoundChange {
        id: "voice-before-next-i".into(),
        target: Matcher::Phone(CATALOG.id_by_ipa("t").unwrap()),
        result: Rewrite::Phone(CATALOG.id_by_ipa("d").unwrap()),
        left: Env::Any,
        right: Env::FollowingSyllableVowel(Matcher::Phone(i)),
    };
    // The shared consonant belongs to its preceding coda first. The next
    // syllable therefore has i, not the final a.
    assert_eq!(
        voice.apply(&form("katːika"), StressRule::Initial),
        form("kadːika")
    );
}

#[test]
fn unstressed_erosion_respects_stress_and_both_guards() {
    let syncope = law("unstressed-syncope");
    assert_eq!(
        syncope
            .apply(&form("kalidus"), MinimalWord::Syllable, StressRule::Initial)
            .ipa(),
        "kaldus"
    );
    assert_eq!(
        syncope
            .apply(&form("kalidus"), MinimalWord::Syllable, StressRule::Penult)
            .ipa(),
        "kalidus"
    );
    assert_eq!(
        syncope
            .apply(
                &form("kalidus"),
                MinimalWord::TwoSyllables,
                StressRule::Initial
            )
            .ipa(),
        "kaldus"
    );
    assert_eq!(
        syncope
            .apply(
                &form("kalidusa"),
                MinimalWord::TwoSyllables,
                StressRule::Initial
            )
            .vowel_count(),
        2
    );
    assert_eq!(
        syncope.apply(&form("tat"), MinimalWord::Syllable, StressRule::Initial),
        form("tat")
    );
    assert_eq!(
        syncope.apply(
            &form("kalːidus"),
            MinimalWord::Syllable,
            StressRule::Initial
        ),
        form("kalːidus")
    );
    let apocope = law("unstressed-apocope");
    assert_eq!(
        apocope.apply(
            &form("kata"),
            MinimalWord::TwoSyllables,
            StressRule::Initial
        ),
        form("kata")
    );
    assert_eq!(
        apocope.apply(&form("kata"), MinimalWord::Syllable, StressRule::Final),
        form("kata")
    );
    assert_eq!(
        apocope.apply(&form("kata"), MinimalWord::Syllable, StressRule::Initial),
        form("kat")
    );
    let delete = SoundChange {
        id: "all-vowels".into(),
        target: Matcher::AnyVowel,
        result: Rewrite::Delete,
        left: Env::Any,
        right: Env::Any,
    };
    assert_eq!(
        delete
            .apply(&form("kata"), StressRule::Initial)
            .vowel_count(),
        1
    );
}

#[test]
fn lexical_stress_tracks_surviving_nuclei_and_insertions() {
    let mut f = form("katapina");
    f.stress = Some(2);
    let after = law("unstressed-syncope").apply(&f, MinimalWord::Syllable, StressRule::Free);
    assert_eq!(after.ipa(), "katpina");
    assert_eq!(after.stress, Some(1));
    assert_eq!(after.ipa_stressed(StressRule::Free), "katˈpina");
    let delete = SoundChange {
        id: "i-loss".into(),
        target: Matcher::Phone(CATALOG.id_by_ipa("i").unwrap()),
        result: Rewrite::Delete,
        left: Env::Any,
        right: Env::Any,
    };
    let after = delete.apply(&f, StressRule::Free);
    assert_eq!(
        after.stress,
        Some(2),
        "deleted accent moves to the next vowel"
    );
    let mut last = form("katapi");
    last.stress = Some(2);
    assert_eq!(
        delete.apply(&last, StressRule::Free).stress,
        Some(1),
        "or the last vowel when none follows"
    );
    let native = [form("ka"), form("ta"), form("pa")];
    let adapter = umran_sim::adapt::Adapter::new(native.iter(), &SoundProfile::base().inventory);
    let mut source = form("ktapa");
    source.stress = Some(1);
    let adapted = adapter.adapt(&source, 0.0, &mut umran_sim::rng::stream(7, &[]));
    assert_eq!(adapted.ipa(), "katapa");
    assert_eq!(adapted.stress, Some(2));
}

#[test]
fn length_chains_preserve_singleton_geminate_contrast() {
    assert_eq!(
        law("cluster-gemination")
            .apply(&form("faktum"), MinimalWord::Syllable, StressRule::Initial)
            .ipa(),
        "fatːum"
    );
    assert_eq!(
        law("cluster-gemination")
            .apply(&form("aptaksa"), MinimalWord::Syllable, StressRule::Initial)
            .ipa(),
        "atːasːa"
    );
    assert_eq!(
        law("j-gemination")
            .apply(&form("satja"), MinimalWord::Syllable, StressRule::Initial)
            .ipa(),
        "satːja"
    );
    assert_eq!(
        law("romance-lenition")
            .apply(
                &form("patatːaka"),
                MinimalWord::Syllable,
                StressRule::Initial
            )
            .ipa(),
        "padataga"
    );
    assert_eq!(
        law("intervocalic-voicing")
            .apply(&form("patːa"), MinimalWord::Syllable, StressRule::Initial)
            .ipa(),
        "patːa"
    );
    assert_eq!(
        law("compensatory-lengthening")
            .apply(&form("kanta"), MinimalWord::Heavy, StressRule::Initial)
            .ipa(),
        "kaːta"
    );
    assert_eq!(
        law("stressed-open-lengthening")
            .apply(&form("katːata"), MinimalWord::Syllable, StressRule::Initial)
            .ipa(),
        "katːata"
    );
    assert_eq!(
        law("verner-voicing")
            .apply(&form("patasa"), MinimalWord::Syllable, StressRule::Initial)
            .ipa(),
        "pataza"
    );
    assert_eq!(
        law("verner-voicing")
            .apply(&form("patasa"), MinimalWord::Syllable, StressRule::Penult)
            .ipa(),
        "patasa"
    );
}

#[test]
fn founding_knobs_and_spelling_keep_length_and_stress_separate() {
    let mut design = LanguageDesign::preset("polynesian", 7).unwrap();
    design.stress = Some(StressRule::Free);
    design.geminates = 1.0;
    let long = Variety::found(
        7,
        &design.profile(),
        Livelihood::Farming,
        Default::default(),
    );
    design.geminates = 0.0;
    let short = Variety::found(
        7,
        &design.profile(),
        Livelihood::Farming,
        Default::default(),
    );
    assert!(long.lexicon.living().any(|w| {
        w.form
            .segs
            .iter()
            .any(|s| s.long && !CATALOG.get(s.phone).is_vowel())
    }));
    assert!(!short.lexicon.living().any(|w| {
        w.form
            .segs
            .iter()
            .any(|s| s.long && !CATALOG.get(s.phone).is_vowel())
    }));
    assert!(
        long.lexicon
            .living()
            .all(|w| w.form.stress.is_some_and(|s| s < w.form.vowel_count()))
    );
    assert_eq!(long.fork(0, 10).stress(), StressRule::Free);
    let spelling = SoundProfile::base().spelling;
    assert_eq!(spelling.write(&form("katːa")), "katta");
    assert_eq!(spelling.write(&form("kaʃːa")), "kashsha");
    let mut stressed = form("katːa");
    stressed.stress = Some(1);
    assert_eq!(spelling.write(&stressed), spelling.write(&form("katːa")));
    let mut name = Name {
        form: form("katata"),
        ..Name::default()
    };
    name.change(
        &law("initial-stress"),
        MinimalWord::Syllable,
        StressRule::Final,
        8,
    );
    assert_eq!(name.form, form("katata"));
    assert!(matches!(
        name.log[0].event,
        Event::SoundLaw {
            law: "initial-stress",
            ..
        }
    ));
    name.change(
        &law("unstressed-syncope"),
        MinimalWord::Syllable,
        StressRule::Initial,
        9,
    );
    assert_eq!(name.form.ipa(), "katta");
}

#[test]
fn stress_shifts_are_assessable_only_when_stress_moves() {
    let shift = law("initial-stress");
    let words = [form("katata"), form("ka")];
    let profile = SoundProfile::base();
    let a = shift
        .assess(
            words.iter(),
            &profile.inventory,
            MinimalWord::Syllable,
            StressRule::Final,
        )
        .unwrap();
    assert_eq!(a.words, 1);
    assert_eq!(a.pull, 0.0);
    assert_eq!(
        shift.apply(&words[0], MinimalWord::Syllable, StressRule::Final),
        words[0]
    );
    assert!(
        shift
            .assess(
                words.iter(),
                &profile.inventory,
                MinimalWord::Syllable,
                StressRule::Initial
            )
            .is_none()
    );
    assert!(
        shift
            .assess(
                [form("ka")].iter(),
                &profile.inventory,
                MinimalWord::Syllable,
                StressRule::Final
            )
            .is_none()
    );
}

#[test]
fn forty_seed_four_thousand_year_prosody_band() {
    let mut languages = 0usize;
    let mut conditioned = 0usize;
    let mut developed = 0usize;
    let mut syllables = 0usize;
    let mut segments = 0usize;
    let mut words = 0usize;
    let geminate = |f: &Form| {
        f.segs
            .iter()
            .any(|s| s.long && !CATALOG.get(s.phone).is_vowel())
    };
    for seed in 0..40 {
        let mut world = World::solo(seed, &SoundProfile::base(), Params::default());
        world.run(160);
        for (v, spoken) in world.spoken().into_iter().enumerate() {
            if !spoken {
                continue;
            }
            let variety = &world.varieties[v];
            languages += 1;
            conditioned += usize::from(variety.laws.iter().any(|(_, id)| {
                matches!(
                    *id,
                    "unstressed-reduction"
                        | "unstressed-syncope"
                        | "unstressed-apocope"
                        | "stressed-open-lengthening"
                        | "verner-voicing"
                )
            }));
            developed += usize::from(variety.lexicon.lexemes.iter().any(|w| {
                geminate(&w.form)
                    || w.log.iter().any(
                        |e| matches!(&e.event, Event::SoundLaw { before, .. } if geminate(before)),
                    )
            }));
            for word in variety.lexicon.living() {
                syllables += word.form.vowel_count();
                segments += word.form.segs.len();
                words += 1;
            }
        }
    }
    let syllables = syllables as f64 / words as f64;
    let segments = segments as f64 / words as f64;
    let conditioned = conditioned as f64 / languages as f64;
    let developed = developed as f64 / languages as f64;
    println!(
        "prosody band: seeds=40 years=4000 languages={languages} words={words} stress_conditioned={conditioned:.6} developed_geminates={developed:.6} mean_syllables={syllables:.6} mean_segments={segments:.6}"
    );
    // Measured before the prosody catalog was added, same seeds/parameters.
    assert!((0.85..=1.15).contains(&(syllables / 1.369305)));
    assert!((0.85..=1.15).contains(&(segments / 3.403147)));
    assert!((0.1..=1.0).contains(&conditioned));
    assert!((0.01..=0.99).contains(&developed));
}

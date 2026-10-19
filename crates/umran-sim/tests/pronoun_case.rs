use rand::Rng;
use umran_sim::grammar::{Category, GrammarChoice, MarkerScope};
use umran_sim::rng::{key, stream};
use umran_sim::{Action, Chronicle, LanguageDesign, MapSize, Naming, Params, Recipe, World};

fn assert_case_eq(a: &World, b: &World) {
    assert_eq!(a.varieties.len(), b.varieties.len());
    for (a, b) in a.varieties.iter().zip(&b.varieties) {
        assert_eq!(a.grammar, b.grammar);
        for (_, _, cell) in a.pronouns.cells() {
            let concept = umran_sim::concepts::by_id(cell).unwrap();
            let left = a.lexicon.word_for(concept).unwrap();
            let right = b.lexicon.word_for(concept).unwrap();
            assert_eq!(left.form, right.form);
            assert_eq!(left.paradigms, right.paradigms);
            assert!(
                left.paradigms
                    .iter()
                    .any(|p| p.category == Category::Object)
            );
        }
    }
}

#[test]
fn recipe_json_replays_scoped_markers_and_pronoun_forms_at_historical_readings() {
    let seed = (0..100_000)
        .find(|&seed| {
            stream(seed, &[key("pronoun case fusion"), 0, 1]).r#gen::<f32>()
                < Params::default().pronoun_fusion_rate
        })
        .unwrap();
    let mut history = Chronicle::new(seed, MapSize::Small);
    let mut design = LanguageDesign::preset("familiar", 9).unwrap();
    design.grammar.object = Some(GrammarChoice::Particle);
    history
        .act(Action::Found {
            naming: Naming::People,
            design,
            seed: 9,
            power: 0.5,
            openness: 0.5,
            region: None,
            livelihood: None,
            ethos: None,
        })
        .unwrap();
    history.act(Action::Run { generations: 2 }).unwrap();
    let json = serde_json::to_string(&history.recipe()).unwrap();
    let recipe: Recipe = serde_json::from_str(&json).unwrap();
    let mut a = Chronicle::from_recipe(&recipe).unwrap();
    let mut b = Chronicle::from_recipe(&recipe).unwrap();
    assert_case_eq(a.latest(), b.latest());
    let at = a.world_at(1);
    assert!(
        at.varieties[0]
            .grammar
            .markers
            .iter()
            .any(|m| m.scope == MarkerScope::Pronouns)
    );
    assert_case_eq(&at, &b.world_at(1));
    let mut shorter = recipe;
    *shorter.tellings[0].actions.last_mut().unwrap() = Action::Run { generations: 1 };
    let direct = Chronicle::from_recipe(&shorter).unwrap();
    assert_case_eq(&at, direct.latest());
}

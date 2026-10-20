//! Fixed utterances, rendered only from the language's recorded grammar.
use super::*;

struct Frame {
    id: &'static str,
    words: &'static [FrameWord],
    translation: &'static str,
}

#[derive(Clone, Copy)]
struct FrameWord {
    concept: &'static str,
    category: Option<Category>,
    role: Option<AgreementRole>,
    possessor_of: Option<usize>,
}

const fn word(
    concept: &'static str,
    category: Option<Category>,
    role: Option<AgreementRole>,
) -> FrameWord {
    FrameWord {
        concept,
        category,
        role,
        possessor_of: None,
    }
}
const fn possessor(concept: &'static str) -> FrameWord {
    FrameWord {
        concept,
        category: Some(Category::Genitive),
        role: Some(AgreementRole::Possessor),
        possessor_of: Some(1),
    }
}
use AgreementRole::{Object, Subject};
use Category::{Future, Object as ObjectCase, Past, Plural, Progressive};

static FRAMES: &[Frame] = &[
    Frame {
        id: "see-past",
        words: &[
            word("1sg", None, Some(Subject)),
            word("see", Some(Past), None),
            word("2sg", Some(ObjectCase), Some(Object)),
        ],
        translation: "I saw you.",
    },
    Frame {
        id: "see-future",
        words: &[
            word("child", Some(Plural), Some(Subject)),
            word("see", Some(Future), None),
            word("dog", Some(ObjectCase), Some(Object)),
        ],
        translation: "The children will see the dog.",
    },
    Frame {
        id: "eat-progressive",
        words: &[
            word("1pl", None, Some(Subject)),
            word("eat", Some(Progressive), None),
            word("fish", Some(ObjectCase), Some(Object)),
        ],
        translation: "We are eating fish.",
    },
    Frame {
        id: "my-house",
        words: &[possessor("1sg"), word("house", None, None)],
        translation: "My house.",
    },
    Frame {
        id: "your-father",
        words: &[possessor("2sg"), word("father", None, None)],
        translation: "Your father.",
    },
    Frame {
        id: "their-children",
        words: &[possessor("3pl"), word("child", Some(Plural), None)],
        translation: "Their children.",
    },
    Frame {
        id: "go-past-plural",
        words: &[
            word("3pl", None, Some(Subject)),
            word("go", Some(Past), None),
        ],
        translation: "They went.",
    },
    Frame {
        id: "give-past",
        words: &[
            word("1sg", None, Some(Subject)),
            word("give", Some(Past), None),
            word("2sg", Some(ObjectCase), Some(Object)),
            word("water", None, Some(Object)),
        ],
        translation: "I gave you water.",
    },
    Frame {
        id: "peace",
        words: &[word("peace", None, None)],
        translation: "Peace (as a greeting).",
    },
    Frame {
        id: "child-of-fish",
        words: &[possessor("child"), word("fish", None, None)],
        translation: "The child’s fish.",
    },
    Frame {
        id: "i-you",
        words: &[
            word("2sg", None, Some(Subject)),
            word("see", Some(Past), None),
            word("1sg", Some(ObjectCase), Some(Object)),
        ],
        translation: "You saw me.",
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Note {
    PossessiveClitic,
    PossessiveAffix,
    PronounDropped,
    NounGenitive,
    ObjectMarked,
    Agreement,
}

#[derive(Serialize)]
pub(super) struct Phrase {
    id: &'static str,
    #[serde(flatten)]
    pub(super) rendering: GrammarRendering,
    notes: Vec<Note>,
}

pub(super) fn render_id(v: &Variety, generation: u32, id: &str) -> Option<Phrase> {
    render(v, generation, FRAMES.iter().find(|f| f.id == id)?)
}

fn render(v: &Variety, generation: u32, frame: &Frame) -> Option<Phrase> {
    // Validate even arguments later omitted by agreement: no invented forms.
    for w in frame.words {
        let concept = by_id(w.concept)?;
        let meta = &v.lexicon.concepts[concept];
        let lexeme = v.lexicon.word_for(concept)?;
        if lexeme.form.segs.is_empty() {
            return None;
        }
        if w.category.is_some_and(|category| {
            meta.categories.allows(category)
                && !(category == Category::Genitive
                    && umran_sim::pronouns::is_pronoun(concept)
                    && v.grammar.summary.possessive == umran_sim::grammar::PronounCase::None)
                && !lexeme.paradigms.iter().any(|p| {
                    p.category == category
                        && p.realizations
                            .iter()
                            .any(|r| r.born <= generation && r.retired.is_none() && r.share > 0.0)
                })
        }) {
            return None;
        }
    }
    let is_pronoun = |w: &FrameWord| {
        umran_sim::pronouns::is_pronoun(by_id(w.concept).expect("validated concept"))
    };
    let cell_for = |w: &FrameWord| {
        let cell = if is_pronoun(w) {
            w.concept
        } else if w.category == Some(Category::Plural) {
            "3pl"
        } else {
            "3sg"
        };
        agreement_cell(v.grammar.agreement_for(w.role?), cell, is_pronoun(w))
    };
    let subject = frame
        .words
        .iter()
        .find(|w| w.role == Some(Subject))
        .and_then(cell_for);
    let object = frame
        .words
        .iter()
        .find(|w| w.role == Some(Object))
        .and_then(cell_for);
    let mut order: Vec<usize> = (0..frame.words.len())
        .filter(|&i| frame.words[i].possessor_of.is_none())
        .collect();
    // Stable ordering preserves both objects in ditransitive frames.
    order.sort_by_key(|&i| {
        let role = frame.words[i].role;
        match (v.grammar.order, role) {
            (WordOrder::SOV, Some(Subject))
            | (WordOrder::SVO, Some(Subject))
            | (WordOrder::VSO, None) => 0,
            (WordOrder::SOV, Some(Object))
            | (WordOrder::SVO, None)
            | (WordOrder::VSO, Some(Subject)) => 1,
            _ => 2,
        }
    });
    let mut out = Phrase {
        id: frame.id,
        rendering: GrammarRendering {
            translation: frame.translation,
            ..Default::default()
        },
        notes: Vec::new(),
    };
    let mut note = |n| {
        if !out.notes.contains(&n) {
            out.notes.push(n);
        }
    };
    let mut emit =
        |w: &FrameWord, agreement: [Option<&AgreementForm>; 3], out: &mut GrammarRendering| {
            if v.lexicon.concepts[by_id(w.concept)?].class == umran_sim::concepts::Class::Entity
                && w.possessor_of.is_none()
            {
                sample_determiner(v, w.concept, generation, out)?;
            }
            if let Some(category) = w.category {
                let lexeme = v.lexicon.word_for(by_id(w.concept)?)?;
                if lexeme
                    .paradigms
                    .iter()
                    .filter(|p| p.category == category)
                    .flat_map(|p| &p.realizations)
                    .any(|r| {
                        r.retired.is_none()
                            && r.share > 0.0
                            && v.grammar.audible(&lexeme.form, r, v.stress())
                    })
                {
                    if category == Category::Object {
                        note(Note::ObjectMarked);
                    }
                    if category == Category::Genitive && !is_pronoun(w) {
                        note(Note::NounGenitive);
                    }
                }
            }
            sample_word(v, w.concept, w.category, generation, out, agreement)
        };
    for index in order {
        let w = &frame.words[index];
        if let Some(role @ (Subject | Object)) = w.role {
            let cell = if role == Subject { subject } else { object };
            // Only the first object is indexed on a ditransitive verb.
            let indexed = role == Subject
                || frame.words.iter().position(|w| w.role == Some(Object)) == Some(index);
            if indexed && is_pronoun(w) && omitted_pronoun(v.grammar.agreement_for(role), cell) {
                continue;
            }
        }
        let poss = frame.words.iter().find(|p| p.possessor_of == Some(index));
        let possessive = poss.filter(|p| is_pronoun(p)).and_then(cell_for);
        let omit = poss.is_some_and(is_pronoun)
            && omitted_pronoun(v.grammar.possessor_agreement(), possessive);
        if v.grammar.possessor == PossessorOrder::Before
            && !omit
            && let Some(p) = poss
        {
            emit(p, [None; 3], &mut out.rendering)?;
        }
        let verb = v.lexicon.concepts[by_id(w.concept)?].class == umran_sim::concepts::Class::Event;
        emit(
            w,
            if verb {
                [subject, object, None]
            } else {
                [None, None, possessive]
            },
            &mut out.rendering,
        )?;
        if v.grammar.possessor == PossessorOrder::After
            && !omit
            && let Some(p) = poss
        {
            emit(p, [None; 3], &mut out.rendering)?;
        }
    }
    // Notes identify only grammar actually exercised by this frame.
    let mut add = |n| {
        if !out.notes.contains(&n) {
            out.notes.push(n);
        }
    };
    if subject.is_some() || object.is_some() {
        add(Note::Agreement);
    }
    for w in frame.words {
        let Some(role) = w.role else { continue };
        let cell = cell_for(w);
        if role == AgreementRole::Possessor && is_pronoun(w) && cell.is_some() {
            match v.grammar.possessor_agreement().stage {
                AgreementStage::Clitic => add(Note::PossessiveClitic),
                AgreementStage::Affix => add(Note::PossessiveAffix),
                AgreementStage::None => {}
            }
        }
        let state = v.grammar.agreement_for(role);
        if is_pronoun(w) && state.stage == AgreementStage::Affix && omitted_pronoun(state, cell) {
            add(Note::PronounDropped);
        }
    }
    Some(out)
}

pub(super) fn possessive_token(
    v: &Variety,
    noun: &Form,
    gloss: &str,
    cell: &str,
    form: &Form,
) -> SampleToken {
    let state = v.grammar.possessor_agreement();
    let (joined, edge) = v
        .grammar
        .attach_possessor(noun, form, &v.morphology, v.stress());
    let mut token = SampleToken {
        text: v.spell(&joined),
        ipa: joined.ipa_stressed(v.stress()),
        gloss: String::new(),
    };
    let separator = if state.stage == AgreementStage::Clitic {
        "="
    } else {
        "-"
    };
    if state.stage == AgreementStage::Clitic {
        // Keep the clitic edge visible after the engine's joining and harmony.
        let left_vowels = (0..edge).filter(|&i| joined.is_vowel(i)).count();
        let part = |start: usize, end: usize, preceding: usize| Form {
            segs: joined.segs[start..end].to_vec(),
            boundaries: joined
                .boundaries
                .iter()
                .copied()
                .filter(|&b| b > start && b < end)
                .map(|b| b - start)
                .collect(),
            stress: joined
                .stress
                .and_then(|s| s.checked_sub(preceding))
                .filter(|&s| s < (start..end).filter(|&i| joined.is_vowel(i)).count()),
        };
        let left = part(0, edge, 0);
        let right = part(edge, joined.segs.len(), left_vowels);
        token.text = format!("{}={}", v.spell(&left), v.spell(&right));
        token.ipa = format!(
            "{}={}",
            left.ipa_stressed(v.stress()),
            right.ipa_stressed(v.stress())
        );
    }
    let label = agreement_gloss(cell);
    token.gloss = match state.side {
        Side::Prefix => format!("{label}.POSS{separator}{gloss}"),
        Side::Suffix => format!("{gloss}{separator}{label}.POSS"),
    };
    token
}

impl Bench {
    pub fn phrasebook(&mut self, generation: u32, variety: usize) -> Result<String, String> {
        let world = self.world(generation);
        let v = world
            .varieties
            .get(variety)
            .ok_or("No such language variety.")?;
        to_json(
            &FRAMES
                .iter()
                .filter_map(|f| render(v, generation, f))
                .collect::<Vec<_>>(),
        )
    }
}
#[wasm_bindgen]
impl Workbench {
    pub fn phrasebook(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .phrasebook(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
}
#[wasm_bindgen]
impl ReadView {
    pub fn phrasebook(&mut self, generation: u32, variety: f64) -> Result<String, JsValue> {
        self.bench
            .borrow_mut()
            .phrasebook(generation, entity_id(variety).map_err(fail)?)
            .map_err(fail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use umran_sim::Params;

    fn forced() -> Params {
        Params {
            possessive_case_rate: 1.0,
            possessive_fusion_rate: 1.0,
            possessive_drop_rate: 1.0,
            ..Params::static_society()
        }
    }
    fn evolve(world: &mut World, generation: u32) {
        let v = &mut world.varieties[0];
        let stress = v.stress();
        v.grammar.evolve(
            7,
            0,
            generation,
            &mut v.lexicon,
            &v.morphology,
            stress,
            &forced(),
        );
        world.generation = generation;
    }

    #[test]
    fn every_frame_renders_from_catalog_words_and_omits_missing_forms() {
        let mut world = crate::tests::grammar_world(GrammarChoice::Suffix, WordOrder::SOV);
        let v = &mut world.varieties[0];
        for frame in FRAMES {
            for word in frame.words {
                assert!(by_id(word.concept).is_some(), "{}", word.concept);
            }
            let p = render(v, 0, frame).unwrap_or_else(|| panic!("missing frame {}", frame.id));
            assert!(!p.rendering.text.is_empty());
            assert!(!p.rendering.ipa.is_empty());
            assert!(!p.rendering.gloss.is_empty());
            assert_eq!(p.rendering.translation, frame.translation);
        }
        let see = v.lexicon.word_for(by_id("see").unwrap()).unwrap().id;
        v.lexicon
            .get_mut(see)
            .paradigms
            .retain(|p| p.category != Category::Past);
        assert!(render_id(v, 0, "see-past").is_none());
        assert!(render_id(v, 0, "see-future").is_some());
        v.lexicon.slot_mut(by_id("house").unwrap()).variants.clear();
        assert!(render_id(v, 0, "my-house").is_none());
        assert!(render_id(v, 0, "your-father").is_some());
    }

    #[test]
    fn possession_frames_follow_order_clitic_affix_and_drop() {
        for order in [PossessorOrder::Before, PossessorOrder::After] {
            let mut world = crate::tests::grammar_world(GrammarChoice::Suffix, WordOrder::SOV);
            world.varieties[0].grammar.possessor = order;
            let free = render_id(&world.varieties[0], 0, "my-house").unwrap();
            assert_eq!(
                free.rendering.gloss,
                if order == PossessorOrder::Before {
                    vec!["1SG", "house"]
                } else {
                    vec!["house", "1SG"]
                }
            );
            assert!(free.notes.is_empty());
            evolve(&mut world, 1);
            let clitic = render_id(&world.varieties[0], 1, "my-house").unwrap();
            assert_eq!(
                clitic.rendering.gloss,
                [if order == PossessorOrder::Before {
                    "1SG.POSS=house"
                } else {
                    "house=1SG.POSS"
                }]
            );
            assert!(clitic.notes.contains(&Note::PossessiveClitic));
            assert_eq!(clitic.rendering.text.matches('=').count(), 1);
            evolve(&mut world, 9);
            let affix = render_id(&world.varieties[0], 9, "my-house").unwrap();
            assert_eq!(affix.rendering.gloss.len(), 2);
            assert!(affix.notes.contains(&Note::PossessiveAffix));
            assert!(!affix.notes.contains(&Note::PronounDropped));
            evolve(&mut world, 10);
            let dropped = render_id(&world.varieties[0], 10, "my-house").unwrap();
            assert_eq!(dropped.rendering.gloss.len(), 1);
            assert!(dropped.notes.contains(&Note::PronounDropped));
            let view = serde_json::to_value(grammar_view(&world, 0)).unwrap();
            assert_eq!(view["possessorAgreement"]["stage"], "affix");
            assert!(
                view["possessorAgreement"]["forms"]
                    .as_array()
                    .unwrap()
                    .len()
                    >= 6
            );
            assert!(!view["sample"]["possessive"].is_null());
            let events = annals(&world);
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.grammar.as_ref().is_some_and(|g| {
                        let g = serde_json::to_value(g).unwrap();
                        g["event"] == "agreement" && g["role"] == "possessor"
                    }))
                    .count(),
                3
            );
        }
    }

    #[test]
    fn facade_phrasebook_and_genitive_story_are_read_only() {
        let mut bench = Bench::load(
            &serde_json::to_string(
                &umran_sim::Chronicle::new(21, umran_sim::MapSize::Small).recipe(),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(bench.phrasebook(0, 0).is_err());
        // Exercise a real saved recipe through the public facade read.
        let mut history = umran_sim::Chronicle::new(21, umran_sim::MapSize::Small);
        history
            .act(umran_sim::Action::Found {
                naming: umran_sim::Naming::People,
                design: umran_sim::LanguageDesign::preset("familiar", 7).unwrap(),
                seed: 7,
                power: 0.5,
                openness: 0.5,
                region: None,
                livelihood: None,
                ethos: None,
            })
            .unwrap();
        let recipe = serde_json::to_string(&history.recipe()).unwrap();
        let mut bench = Bench::load(&recipe).unwrap();
        let before = bench.save().unwrap();
        let phrases: serde_json::Value =
            serde_json::from_str(&bench.phrasebook(0, 0).unwrap()).unwrap();
        assert!(
            phrases
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["id"] == "my-house" && p["gloss"].is_array())
        );
        let story: serde_json::Value =
            serde_json::from_str(&bench.grammar_story(0, 0, "genitive").unwrap()).unwrap();
        assert_eq!(story["agreement"]["role"], "possessor");
        assert_eq!(story["agreement"]["stage"], "none");
        assert_eq!(bench.save().unwrap(), before);
    }
}

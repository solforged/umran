use crate::change::{Env, Matcher, Rewrite, SoundChange};
use crate::phoneme::{Backness, Height, Manner, Place};

pub fn conservative() -> Vec<SoundChange> {
    vec![
        SoundChange {
            id: "t_to_s_before_i".into(),
            target: Matcher::Consonant {
                place: Some(Place::Alveolar),
                manner: Some(Manner::Stop),
                voiced: Some(false),
            },
            result: Rewrite::Consonant {
                place: Some(Place::Alveolar),
                manner: Some(Manner::Fricative),
                voiced: Some(false),
            },
            left: Env::Any,
            right: Env::Matcher(Matcher::Vowel {
                height: Some(Height::Close),
                backness: Some(Backness::Front),
                rounded: Some(false),
            }),
        },
        SoundChange {
            id: "p_lenite_v_v".into(),
            target: Matcher::Consonant {
                place: Some(Place::Bilabial),
                manner: Some(Manner::Stop),
                voiced: Some(false),
            },
            result: Rewrite::Consonant {
                place: Some(Place::Labiodental),
                manner: Some(Manner::Fricative),
                voiced: Some(false),
            },
            left: Env::Matcher(Matcher::AnyVowel),
            right: Env::Matcher(Matcher::AnyVowel),
        },
    ]
}

pub fn radical() -> Vec<SoundChange> {
    vec![
        SoundChange {
            id: "apocope".into(),
            target: Matcher::AnyVowel,
            result: Rewrite::Delete,
            left: Env::Any,
            right: Env::WordEdge,
        },
        SoundChange {
            id: "lenite_stop_v_v".into(),
            target: Matcher::Consonant {
                place: None,
                manner: Some(Manner::Stop),
                voiced: Some(false),
            },
            result: Rewrite::Consonant {
                place: None,
                manner: None,
                voiced: Some(true),
            },
            left: Env::Matcher(Matcher::AnyVowel),
            right: Env::Matcher(Matcher::AnyVowel),
        },
        SoundChange {
            id: "s_to_h_initial".into(),
            target: Matcher::Consonant {
                place: Some(Place::Alveolar),
                manner: Some(Manner::Fricative),
                voiced: Some(false),
            },
            result: Rewrite::Consonant {
                place: Some(Place::Glottal),
                manner: Some(Manner::Fricative),
                voiced: Some(false),
            },
            left: Env::WordEdge,
            right: Env::Any,
        },
        SoundChange {
            id: "kw_to_p".into(),
            target: Matcher::Consonant {
                place: Some(Place::Velar),
                manner: Some(Manner::Stop),
                voiced: Some(false),
            },
            result: Rewrite::Consonant {
                place: Some(Place::Bilabial),
                manner: Some(Manner::Stop),
                voiced: Some(false),
            },
            left: Env::Any,
            right: Env::Matcher(Matcher::Consonant {
                place: Some(Place::Bilabial),
                manner: Some(Manner::Approximant),
                voiced: Some(true),
            }),
        },
        SoundChange {
            id: "w_drop_after_p".into(),
            target: Matcher::Consonant {
                place: Some(Place::Bilabial),
                manner: Some(Manner::Approximant),
                voiced: Some(true),
            },
            result: Rewrite::Delete,
            left: Env::Matcher(Matcher::Consonant {
                place: Some(Place::Bilabial),
                manner: Some(Manner::Stop),
                voiced: Some(false),
            }),
            right: Env::Any,
        },
    ]
}

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

pub fn journey(aesthetic_id: &str) -> Vec<SoundChange> {
    match aesthetic_id {
        "elvish" => {
            let mut rules = conservative();
            rules.extend(radical());
            rules
        }
        "kuo-toa" => {
            let mut rules = radical();
            rules.extend(conservative());
            rules
        }
        "illithid" => radical(),
        _ => conservative(),
    }
}

pub fn rule_label(id: &str) -> String {
    match id {
        "t_to_s_before_i" => "t → s before i".into(),
        "p_lenite_v_v" => "p → f between vowels".into(),
        "apocope" => "final vowel drops".into(),
        "lenite_stop_v_v" => "stops voice between vowels".into(),
        "s_to_h_initial" => "initial s → h".into(),
        "kw_to_p" => "kw → p".into(),
        "w_drop_after_p" => "w drops after p".into(),
        other => other.into(),
    }
}

pub fn rule_detail(id: &str) -> String {
    match id {
        "t_to_s_before_i" => "t becomes s before a front close unrounded vowel".into(),
        "p_lenite_v_v" => "p becomes f between vowels".into(),
        "apocope" => "a later vowel at the word edge is lost; a word's only vowel stays".into(),
        "lenite_stop_v_v" => "voiceless stops become voiced between vowels".into(),
        "s_to_h_initial" => "s becomes h at the start of a word".into(),
        "kw_to_p" => "k becomes p before w".into(),
        "w_drop_after_p" => "w is lost after p".into(),
        other => other.into(),
    }
}

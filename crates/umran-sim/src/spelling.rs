use crate::form::{Form, Seg};
use crate::phoneme::CATALOG;
use crate::profile::{LongVowel, Spelling};
use crate::tone::Tone;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Spellings that read as English vulgarities or slurs. Minting avoids
/// them as a courtesy to readers, not as a claim about the languages.
const UNFORTUNATE: &[&str] = &[
    "shit", "shite", "fuck", "fuk", "cunt", "kunt", "piss", "cock", "dick", "twat", "wank", "slut",
    "whore", "fag", "rape", "nazi", "porn", "nigga", "niga",
];

impl Spelling {
    /// Whether the written form reads as an English vulgarity.
    pub fn unfortunate(&self, form: &Form) -> bool {
        let written = self.write(form).to_lowercase();
        UNFORTUNATE.contains(&written.as_str())
    }

    /// The form in this variety's romanization.
    pub fn write(&self, form: &Form) -> String {
        self.write_tonal(form, false)
    }

    /// Tonal languages reserve acute for pitch, even on their untoned words.
    pub(crate) fn write_tonal(&self, form: &Form, tonal: bool) -> String {
        let long_vowels = if self.long_vowels == LongVowel::Acute
            && (tonal || form.segs.iter().any(|seg| seg.tone.is_some()))
        {
            LongVowel::Macron
        } else {
            self.long_vowels
        };
        let ipa = |i: usize| CATALOG.get(form.segs[i].phone).ipa();
        let mut out = String::new();
        let mut i = 0;
        while i < form.segs.len() {
            if form.boundaries.contains(&i) {
                if let Some(mark) = &self.boundary_mark {
                    out.push_str(mark);
                }
            } else if self.mark_hiatus && i > 0 && form.is_vowel(i - 1) && form.is_vowel(i) {
                out.push('\'');
            }
            let kw = self.kw_as_qu
                && ipa(i) == "k"
                && i + 1 < form.segs.len()
                && ipa(i + 1) == "w"
                && !form.segs[i].long
                && !form.segs[i + 1].long
                && !form.boundaries.contains(&(i + 1));
            if kw {
                out.push_str("qu");
                i += 2;
            } else {
                let letters = self.letters(form.segs[i], long_vowels);
                if let Some(tone) = form.segs[i].tone {
                    // Put pitch on the first letter of the vowel's spelling,
                    // after its quality/length marks, not on a doubled tail.
                    let end = letters
                        .char_indices()
                        .skip(1)
                        .find(|(_, c)| !is_combining_mark(*c))
                        .map_or(letters.len(), |(i, _)| i);
                    out.extend(
                        letters[..end]
                            .chars()
                            .chain(std::iter::once(tone_mark(tone)))
                            .chain(letters[end..].chars())
                            .nfc(),
                    );
                } else {
                    out.push_str(&letters);
                }
                i += 1;
            }
        }
        out
    }

    fn letters(&self, seg: Seg, long_vowels: LongVowel) -> String {
        let segment = CATALOG.get(seg.phone);
        let base = self
            .overrides
            .iter()
            .find(|(from, _)| from == segment.ipa())
            .map_or(segment.roman(), |(_, to)| to.as_str());
        if !seg.long {
            return base.to_string();
        }
        if !segment.is_vowel() {
            return format!("{base}{base}");
        }
        match long_vowels {
            LongVowel::Unmarked => base.to_string(),
            LongVowel::Double => match base.chars().last() {
                Some(c) => format!("{base}{c}"),
                None => String::new(),
            },
            LongVowel::Macron => base
                .chars()
                .map(|c| match c {
                    'a' => 'ā',
                    'e' => 'ē',
                    'i' => 'ī',
                    'o' => 'ō',
                    'u' => 'ū',
                    'y' => 'ȳ',
                    other => other,
                })
                .collect(),
            LongVowel::Acute => base
                .chars()
                .map(|c| match c {
                    'a' => 'á',
                    'e' => 'é',
                    'i' => 'í',
                    'o' => 'ó',
                    'u' => 'ú',
                    'y' => 'ý',
                    other => other,
                })
                .collect(),
        }
    }
}

/// These vowel marks do not overlap the palettes' diaereses or length macrons.
/// Hook above and dot below keep the two low contours distinct.
fn tone_mark(tone: Tone) -> char {
    match tone {
        Tone::High => '\u{301}',
        Tone::Low => '\u{300}',
        Tone::Rising => '\u{30c}',
        Tone::Falling => '\u{302}',
        Tone::LowRising => '\u{309}',
        Tone::LowFalling => '\u{323}',
    }
}

#[cfg(test)]
mod tests {
    use crate::form::Form;
    use crate::profile::{LongVowel, Spelling};
    use crate::tone::{Tonal, Tone};
    use crate::{Params, SoundProfile, World};
    use std::collections::HashSet;
    use unicode_normalization::UnicodeNormalization;

    #[test]
    fn palette_vowel_marks_do_not_collide_with_any_tone() {
        for flavor in crate::flavor::Flavor::examples() {
            let spelling = SoundProfile::base().flavored(&flavor).spelling;
            for (_, vowel) in crate::CATALOG.vowels() {
                for length in ["", "ː"] {
                    let ipa = format!("{}{length}", vowel.ipa);
                    let plain = write(&spelling, &ipa, &[]);
                    assert!(
                        plain
                            .nfd()
                            .all(|c| !Tone::ALL.into_iter().any(|t| c == super::tone_mark(t))),
                        "{}: {plain}",
                        flavor.id,
                    );
                    let mut distinct = HashSet::from([plain]);
                    for tone in Tone::ALL {
                        assert!(distinct.insert(write(
                            &spelling,
                            &format!("{ipa}{}", tone.ipa()),
                            &[]
                        )));
                    }
                    assert_eq!(distinct.len(), 7);
                }
            }
        }
    }

    #[test]
    fn six_tones_round_trip_independently_of_macron_length() {
        let mut spelling = SoundProfile::base().spelling;
        spelling.long_vowels = LongVowel::Macron;
        let expected = ["ká", "kà", "kǎ", "kâ", "kả", "kạ"];
        let marks = [
            '\u{301}', '\u{300}', '\u{30c}', '\u{302}', '\u{309}', '\u{323}',
        ];
        let mut distinct = HashSet::new();
        for long in [false, true] {
            for (i, tone) in Tone::ALL.into_iter().enumerate() {
                let ipa = format!("ka{}{}", if long { "ː" } else { "" }, tone.ipa());
                let form = Form::from_ipa(&ipa).unwrap();
                let written = spelling.write(&form);
                if !long {
                    assert_eq!(written, expected[i]);
                }
                assert_eq!(written, written.nfc().collect::<String>());
                assert!(distinct.insert(written.clone()));
                // Decode just pitch and length: spelling deliberately merges
                // some vowel qualities, so there is no general spelling parser.
                let decomposed: Vec<_> = written.nfd().collect();
                let pitch = marks
                    .iter()
                    .position(|mark| decomposed.contains(mark))
                    .unwrap();
                assert_eq!(pitch, i);
                assert_eq!(decomposed.contains(&'\u{304}'), long);
                let decoded = format!(
                    "ka{}{}",
                    if decomposed.contains(&'\u{304}') {
                        "ː"
                    } else {
                        ""
                    },
                    Tone::ALL[pitch].ipa(),
                );
                assert_eq!(Form::from_ipa(&decoded).unwrap(), form);
                assert_eq!(form.ipa(), ipa, "IPA keeps Chao letters");
            }
        }
        assert_eq!(distinct.len(), 12);
        assert_eq!(write(&spelling, "eː˥", &[]), "ḗ");
    }

    #[test]
    fn tone_stacks_on_quality_marks_and_the_first_letter_of_digraphs() {
        let mut spelling = SoundProfile::base().spelling;
        assert_eq!(write(&spelling, "æ˥ø˩ɨ˧˥", &[]), "ä\u{301}ö\u{300}ï\u{30c}");
        for base in ["å", "a\u{30a}", "ā", "a\u{304}", "æ"] {
            spelling.overrides = vec![("a".into(), base.into())];
            for tone in Tone::ALL {
                let written = write(&spelling, &format!("a{}", tone.ipa()), &[]);
                let decomposed: String = written
                    .nfd()
                    .filter(|&c| c != super::tone_mark(tone))
                    .nfc()
                    .collect();
                assert_eq!(decomposed, base.nfc().collect::<String>());
                assert_eq!(written, written.nfc().collect::<String>());
            }
        }
        spelling.overrides = vec![("a".into(), "a\u{30a}e".into())];
        assert_eq!(write(&spelling, "a˥", &[]), "ǻe");
        spelling.overrides.clear();
        spelling.long_vowels = LongVowel::Double;
        spelling.mark_hiatus = true;
        spelling.boundary_mark = Some("-".into());
        assert_eq!(write(&spelling, "kaː˥a˩ta˧˥", &[3]), "káa'à-tǎ");
    }

    #[test]
    fn acute_length_is_preserved_only_while_the_language_is_atonal() {
        let mut profile = SoundProfile::base();
        profile.spelling.long_vowels = LongVowel::Acute;
        let mut world = World::solo(8, &profile, Params::static_society());
        let variety = &mut world.varieties[0];
        let long = Form::from_ipa("kaː").unwrap();
        let high = Form::from_ipa("ka˥").unwrap();
        assert_eq!(variety.spell(&long), "ká");
        variety.tonal = Some(Tonal {
            since: 1,
            lost: None,
        });
        assert_eq!(variety.spell(&long), "kā");
        assert_eq!(variety.spell(&high), "ká");
        assert_eq!(variety.spell(&Form::from_ipa("kaː˥").unwrap()), "kā\u{301}");
        assert_eq!(variety.title(&Form::from_ipa("aː˥").unwrap()), "Ā\u{301}");
        assert_eq!(
            profile.spelling.write(&Form::from_ipa("kaː˥").unwrap()),
            "kā\u{301}"
        );
        variety.tonal.as_mut().unwrap().lost = Some(2);
        assert_eq!(variety.spell(&long), "ká");
    }

    fn write(spelling: &Spelling, ipa: &str, boundaries: &[usize]) -> String {
        let mut form = Form::from_ipa(ipa).unwrap();
        form.boundaries = boundaries.to_vec();
        spelling.write(&form)
    }

    #[test]
    fn overrides_qu_and_acute_length() {
        let spelling = Spelling {
            overrides: vec![("k".into(), "c".into()), ("x".into(), "ch".into())],
            kw_as_qu: true,
            long_vowels: LongVowel::Acute,
            mark_hiatus: false,
            boundary_mark: None,
        };
        assert_eq!(write(&spelling, "kweːndi", &[]), "quéndi");
        assert_eq!(write(&spelling, "kaxa", &[]), "cacha");
    }

    #[test]
    fn hiatus_boundaries_and_long_vowel_styles() {
        let mut spelling = Spelling {
            overrides: vec![],
            kw_as_qu: false,
            long_vowels: LongVowel::Double,
            mark_hiatus: true,
            boundary_mark: Some("'".into()),
        };
        assert_eq!(write(&spelling, "θia", &[]), "thi'a");
        assert_eq!(write(&spelling, "kasθin", &[3]), "kas'thin");
        assert_eq!(write(&spelling, "kaːs", &[]), "kaas");
        spelling.long_vowels = LongVowel::Macron;
        assert_eq!(write(&spelling, "kaːs", &[]), "kās");
    }
}

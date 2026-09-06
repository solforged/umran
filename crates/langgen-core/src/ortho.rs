use crate::aesthetic::{Aesthetic, LongVowel, OrthoStyle, Signature};
use crate::generate::{Syllable, Word};
use crate::phoneme::{PhonemeId, CATALOG};

pub fn romanize(word: &Word, aesthetic: &Aesthetic) -> String {
    let mut out = String::new();
    let glottal = aesthetic
        .signatures
        .iter()
        .any(|s| matches!(s, Signature::GlottalHiatus));
    for (i, syl) in word.syllables.iter().enumerate() {
        if i > 0 && glottal && hiatus(&word.syllables[i - 1], syl) {
            out.push('\'');
        }
        if word.join_at == Some(i) {
            if let Some(join) = &aesthetic.ortho.compound_joiner {
                if !out.ends_with('\'') {
                    out.push_str(join);
                }
            }
        }
        push_cluster(&mut out, &syl.onset, &aesthetic.ortho);
        push_nucleus(&mut out, syl, &aesthetic.ortho);
        push_cluster(&mut out, &syl.coda, &aesthetic.ortho);
    }
    tidy(&out)
}

fn push_cluster(out: &mut String, ids: &[PhonemeId], ortho: &OrthoStyle) {
    if ortho.kw_as_qu && ids.len() == 2 {
        let a = CATALOG.get(ids[0]).ipa();
        let b = CATALOG.get(ids[1]).ipa();
        if a == "k" && b == "w" {
            out.push_str("qu");
            return;
        }
    }
    for id in ids {
        out.push_str(&roman_phone(*id, ortho));
    }
}

fn push_nucleus(out: &mut String, syl: &Syllable, ortho: &OrthoStyle) {
    if syl.nucleus.is_empty() {
        return;
    }
    let mut roman = String::new();
    for id in &syl.nucleus {
        roman.push_str(&roman_phone(*id, ortho));
    }
    if syl.long {
        roman = lengthen(&roman, ortho.long_vowels);
    }
    out.push_str(&roman);
}

fn roman_phone(id: PhonemeId, ortho: &OrthoStyle) -> String {
    let seg = CATALOG.get(id);
    if let Some((_, to)) = ortho.overrides.iter().find(|(from, _)| from == seg.ipa()) {
        return to.clone();
    }
    if ortho.k_as_c && seg.ipa() == "k" {
        return "c".into();
    }
    seg.roman().to_string()
}

fn lengthen(roman: &str, style: LongVowel) -> String {
    match style {
        LongVowel::None => roman.to_string(),
        LongVowel::Double => {
            if let Some(ch) = roman.chars().last() {
                format!("{roman}{ch}")
            } else {
                roman.to_string()
            }
        }
        LongVowel::Acute => roman
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

fn hiatus(prev: &Syllable, next: &Syllable) -> bool {
    !prev.nucleus.is_empty()
        && prev.coda.is_empty()
        && next.onset.is_empty()
        && !next.nucleus.is_empty()
}

fn tidy(s: &str) -> String {
    let mut out = String::new();
    let mut prev_quote = false;
    for c in s.chars() {
        if c == '\'' {
            if prev_quote || out.is_empty() {
                continue;
            }
            prev_quote = true;
            out.push(c);
        } else {
            prev_quote = false;
            out.push(c);
        }
    }
    out.trim_matches('\'').to_string()
}

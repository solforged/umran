use crate::form::{Form, Seg};
use crate::phoneme::CATALOG;
use crate::profile::{LongVowel, Spelling};

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
                && !form.boundaries.contains(&(i + 1));
            if kw {
                out.push_str("qu");
                i += 2;
            } else {
                out.push_str(&self.letters(form.segs[i]));
                i += 1;
            }
        }
        out
    }

    fn letters(&self, seg: Seg) -> String {
        let segment = CATALOG.get(seg.phone);
        let base = self
            .overrides
            .iter()
            .find(|(from, _)| from == segment.ipa())
            .map_or(segment.roman(), |(_, to)| to.as_str());
        if !seg.long {
            return base.to_string();
        }
        match self.long_vowels {
            LongVowel::Unmarked => base.to_string(),
            LongVowel::Double => match base.chars().last() {
                Some(c) => format!("{base}{c}"),
                None => String::new(),
            },
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

#[cfg(test)]
mod tests {
    use crate::form::Form;
    use crate::profile::SoundProfile;

    fn write(profile: &str, ipa: &str, boundaries: &[usize]) -> String {
        let mut form = Form::from_ipa(ipa).unwrap();
        form.boundaries = boundaries.to_vec();
        SoundProfile::by_id(profile).unwrap().spelling.write(&form)
    }

    #[test]
    fn elvish_spells_c_qu_and_acute_length() {
        assert_eq!(write("elvish", "kweːndi", &[]), "quéndi");
        assert_eq!(write("elvish", "kaxa", &[]), "cacha");
    }

    #[test]
    fn illithid_marks_hiatus_and_boundaries() {
        assert_eq!(write("illithid", "θia", &[]), "thi'a");
        assert_eq!(write("illithid", "kasθin", &[3]), "kas'thin");
        assert_eq!(write("illithid", "kaːs", &[]), "kaas");
    }
}

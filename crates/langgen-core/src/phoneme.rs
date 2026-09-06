use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PhonemeId(pub u16);

// Persist symbols, not catalog offsets: adding a segment must not rewrite history.
impl Serialize for PhonemeId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let segment = CATALOG
            .segments
            .get(self.0 as usize)
            .ok_or_else(|| serde::ser::Error::custom("unknown phoneme id"))?;
        serializer.serialize_str(segment.ipa())
    }
}

impl<'de> Deserialize<'de> for PhonemeId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ipa = String::deserialize(deserializer)?;
        CATALOG
            .id_by_ipa(&ipa)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown phoneme '{ipa}'")))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Place {
    Bilabial,
    Labiodental,
    Dental,
    Alveolar,
    Postalveolar,
    Palatal,
    Velar,
    Uvular,
    Pharyngeal,
    Glottal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Manner {
    Stop,
    Nasal,
    Trill,
    Tap,
    Fricative,
    Affricate,
    Approximant,
    Lateral,
    LateralFricative,
    Implosive,
    Ejective,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Height {
    Close,
    NearClose,
    CloseMid,
    Mid,
    OpenMid,
    NearOpen,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Backness {
    Front,
    Central,
    Back,
}

#[derive(Clone, Copy, Debug)]
pub struct Consonant {
    pub ipa: &'static str,
    pub roman: &'static str,
    pub place: Place,
    pub manner: Manner,
    pub voiced: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Vowel {
    pub ipa: &'static str,
    pub roman: &'static str,
    pub height: Height,
    pub backness: Backness,
    pub rounded: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Segment {
    Consonant(Consonant),
    Vowel(Vowel),
}

impl Segment {
    pub fn ipa(self) -> &'static str {
        match self {
            Segment::Consonant(c) => c.ipa,
            Segment::Vowel(v) => v.ipa,
        }
    }

    pub fn roman(self) -> &'static str {
        match self {
            Segment::Consonant(c) => c.roman,
            Segment::Vowel(v) => v.roman,
        }
    }

    pub fn is_vowel(self) -> bool {
        matches!(self, Segment::Vowel(_))
    }

    pub fn consonant(self) -> Option<Consonant> {
        match self {
            Segment::Consonant(c) => Some(c),
            Segment::Vowel(_) => None,
        }
    }

    pub fn vowel(self) -> Option<Vowel> {
        match self {
            Segment::Vowel(v) => Some(v),
            Segment::Consonant(_) => None,
        }
    }

    pub fn sonority(self) -> u8 {
        match self {
            Segment::Vowel(_) => 7,
            Segment::Consonant(c) => match c.manner {
                Manner::Stop | Manner::Implosive | Manner::Ejective => 1,
                Manner::Affricate => 2,
                Manner::Fricative | Manner::LateralFricative => 3,
                Manner::Nasal => 4,
                Manner::Lateral | Manner::Tap | Manner::Trill => 5,
                Manner::Approximant => 6,
            },
        }
    }
}

pub struct Catalog {
    pub segments: Vec<Segment>,
}

impl Catalog {
    fn builtin() -> Self {
        let mut segments = Vec::new();
        use Backness::*;
        use Height::*;
        use Manner::*;
        use Place::*;
        {
            let mut c = |ipa, roman, place, manner, voiced| {
                segments.push(Segment::Consonant(Consonant {
                    ipa,
                    roman,
                    place,
                    manner,
                    voiced,
                }));
            };
            c("p", "p", Bilabial, Stop, false);
            c("b", "b", Bilabial, Stop, true);
            c("t", "t", Alveolar, Stop, false);
            c("d", "d", Alveolar, Stop, true);
            c("k", "k", Velar, Stop, false);
            c("g", "g", Velar, Stop, true);
            c("q", "q", Uvular, Stop, false);
            c("ʔ", "'", Glottal, Stop, false);
            c("m", "m", Bilabial, Nasal, true);
            c("n", "n", Alveolar, Nasal, true);
            c("ɲ", "ny", Palatal, Nasal, true);
            c("ŋ", "ng", Velar, Nasal, true);
            c("r", "r", Alveolar, Trill, true);
            c("ɾ", "r", Alveolar, Tap, true);
            c("f", "f", Labiodental, Fricative, false);
            c("v", "v", Labiodental, Fricative, true);
            c("θ", "th", Dental, Fricative, false);
            c("ð", "dh", Dental, Fricative, true);
            c("s", "s", Alveolar, Fricative, false);
            c("z", "z", Alveolar, Fricative, true);
            c("ʃ", "sh", Postalveolar, Fricative, false);
            c("ʒ", "zh", Postalveolar, Fricative, true);
            c("x", "kh", Velar, Fricative, false);
            c("ɣ", "gh", Velar, Fricative, true);
            c("h", "h", Glottal, Fricative, false);
            c("ts", "ts", Alveolar, Affricate, false);
            c("dz", "dz", Alveolar, Affricate, true);
            c("tʃ", "ch", Postalveolar, Affricate, false);
            c("dʒ", "j", Postalveolar, Affricate, true);
            c("w", "w", Bilabial, Approximant, true);
            c("j", "y", Palatal, Approximant, true);
            c("l", "l", Alveolar, Lateral, true);
            c("ʎ", "ly", Palatal, Lateral, true);
            c("ɬ", "hl", Alveolar, LateralFricative, false);
            c("ɓ", "b", Bilabial, Implosive, true);
            c("ɗ", "d", Alveolar, Implosive, true);
            c("pʼ", "p'", Bilabial, Ejective, false);
            c("tʼ", "t'", Alveolar, Ejective, false);
            c("kʼ", "k'", Velar, Ejective, false);
        }
        {
            let mut v = |ipa, roman, height, backness, rounded| {
                segments.push(Segment::Vowel(Vowel {
                    ipa,
                    roman,
                    height,
                    backness,
                    rounded,
                }));
            };
            v("i", "i", Close, Front, false);
            v("y", "y", Close, Front, true);
            v("ɨ", "ï", Close, Central, false);
            v("u", "u", Close, Back, true);
            v("ɪ", "i", NearClose, Front, false);
            v("ʊ", "u", NearClose, Back, true);
            v("e", "e", CloseMid, Front, false);
            v("ø", "ö", CloseMid, Front, true);
            v("ə", "ë", Mid, Central, false);
            v("o", "o", CloseMid, Back, true);
            v("ɛ", "e", OpenMid, Front, false);
            v("ɔ", "o", OpenMid, Back, true);
            v("æ", "ä", NearOpen, Front, false);
            v("a", "a", Open, Front, false);
            v("ɑ", "a", Open, Back, false);
        }

        Self { segments }
    }

    pub fn get(&self, id: PhonemeId) -> Segment {
        self.segments[id.0 as usize]
    }

    pub fn id_by_ipa(&self, ipa: &str) -> Option<PhonemeId> {
        self.segments
            .iter()
            .position(|s| s.ipa() == ipa)
            .map(|i| PhonemeId(i as u16))
    }

    pub fn consonants(&self) -> impl Iterator<Item = (PhonemeId, Consonant)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.consonant().map(|c| (PhonemeId(i as u16), c)))
    }

    pub fn vowels(&self) -> impl Iterator<Item = (PhonemeId, Vowel)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.vowel().map(|v| (PhonemeId(i as u16), v)))
    }

    /// Greedy longest-match parse of an IPA string into catalog ids.
    pub fn parse_ipa(&self, raw: &str) -> Option<Vec<PhonemeId>> {
        let s: String = raw
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '.')
            .collect();
        let mut rest = s.as_str();
        let mut out = Vec::new();
        let mut ranked: Vec<(PhonemeId, &str)> = self
            .segments
            .iter()
            .enumerate()
            .map(|(i, seg)| (PhonemeId(i as u16), seg.ipa()))
            .collect();
        ranked.sort_by_key(|(_, ipa)| std::cmp::Reverse(ipa.len()));
        while !rest.is_empty() {
            let mut hit = None;
            for (id, ipa) in &ranked {
                if rest.starts_with(ipa) {
                    hit = Some((*id, ipa.len()));
                    break;
                }
            }
            let (id, n) = hit?;
            out.push(id);
            rest = &rest[n..];
        }
        Some(out)
    }

    pub fn ipa_string(&self, ids: &[PhonemeId]) -> String {
        ids.iter().map(|id| self.get(*id).ipa()).collect()
    }
}

pub static CATALOG: LazyLock<Catalog> = LazyLock::new(Catalog::builtin);

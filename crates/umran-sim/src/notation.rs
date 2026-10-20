//! Author-written, single-segment sound laws. Parsing never reads a world.
use crate::change::{Env, Matcher, Rewrite, SoundChange};
use crate::phoneme::{Backness, Height, Manner, PhonemeId, Place, Secondary, Segment};
use crate::{CATALOG, Law, StressRule};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthoredLaw {
    pub title: String,
    pub source: String,
    pub rules: Vec<SoundChange>,
    pub stress: Option<StressRule>,
}

impl AuthoredLaw {
    /// Stored rules are authoritative: source is never reparsed during replay.
    pub fn law(&self, generation: u32) -> Law {
        let slug = slug(&self.title);
        Law {
            id: format!("authored:{slug}:{generation}").into(),
            label: self.title.clone().into(),
            source: Some(self.source.clone()),
            rules: self
                .rules
                .iter()
                .enumerate()
                .map(|(i, rule)| SoundChange {
                    id: format!("{slug}/{i}"),
                    ..rule.clone()
                })
                .collect(),
            commonness: 0.0,
            stress: self.stress,
        }
    }
}

pub fn slug(title: &str) -> String {
    title
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LawReport {
    pub rules: Vec<RuleCheck>,
    pub law: Option<AuthoredLaw>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleCheck {
    pub line: usize,
    pub source: String,
    pub status: RuleStatus,
    pub message: String,
    pub reading: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuleStatus {
    Ok,
    Empty,
    BadSegment,
    UnknownTerm,
    BadShape,
    Unsupported,
    DuplicateStress,
}

type Error = (RuleStatus, String);
fn shape(message: impl Into<String>) -> Error {
    (RuleStatus::BadShape, message.into())
}
fn unsupported(message: impl Into<String>) -> Error {
    (RuleStatus::Unsupported, message.into())
}

/// One report row per source line, including ignored comments and blank lines.
/// `labial` expands bilabial first, then labiodental (Cartesian order when
/// several positions use it). Prosodic constraints are never silently dropped.
pub fn parse(title: &str, source: &str) -> LawReport {
    let mut checks = Vec::new();
    let mut rules = Vec::new();
    let mut stress = None;
    if title.trim().is_empty() || title.chars().count() > 60 {
        checks.push(RuleCheck {
            line: 0,
            source: title.into(),
            status: RuleStatus::BadShape,
            message: if title.trim().is_empty() {
                "needs a title"
            } else {
                "title exceeds 60 characters"
            }
            .into(),
            reading: None,
        });
    }
    for (i, raw) in source.lines().enumerate() {
        let text = raw.trim();
        let mut check = RuleCheck {
            line: i + 1,
            source: raw.into(),
            status: RuleStatus::Empty,
            message: String::new(),
            reading: None,
        };
        if !text.is_empty() && !text.starts_with('#') {
            match line(text, &mut stress) {
                Ok(mut parsed) => {
                    check.status = RuleStatus::Ok;
                    check.reading = Some(reading(text));
                    rules.append(&mut parsed);
                }
                Err((status, message)) => {
                    check.status = status;
                    check.message = message;
                }
            }
        }
        checks.push(check);
    }
    let valid = checks
        .iter()
        .all(|c| matches!(c.status, RuleStatus::Ok | RuleStatus::Empty))
        && (!rules.is_empty() || stress.is_some());
    let slug = slug(title);
    for (i, rule) in rules.iter_mut().enumerate() {
        rule.id = format!("{slug}/{i}");
    }
    LawReport {
        rules: checks,
        law: valid.then(|| AuthoredLaw {
            title: title.into(),
            source: source.into(),
            rules,
            stress,
        }),
    }
}

fn line(text: &str, stress: &mut Option<StressRule>) -> Result<Vec<SoundChange>, Error> {
    if text
        .chars()
        .any(|c| matches!(c, '˥' | '˦' | '˧' | '˨' | '˩'))
    {
        return Err(unsupported("tone is not authorable"));
    }
    let (target, rest) = text
        .split_once('>')
        .ok_or_else(|| shape("expected target > result"))?;
    if rest.contains('>') {
        return Err(shape("expected one >"));
    }
    let target = target.trim();
    if target.eq_ignore_ascii_case("stress") {
        let next = match rest.trim().to_ascii_lowercase().as_str() {
            "initial" => StressRule::Initial,
            "penult" => StressRule::Penult,
            "final" => StressRule::Final,
            "weight" => StressRule::Weight,
            "free" => StressRule::Free,
            _ => {
                return Err(shape(
                    "expected initial, penult, final, weight, or free stress",
                ));
            }
        };
        if stress.is_some() {
            return Err((
                RuleStatus::DuplicateStress,
                "only one stress rule is allowed".into(),
            ));
        }
        *stress = Some(next);
        return Ok(Vec::new());
    }
    let (result, left, right) = if let Some((result, env)) = rest.split_once('/') {
        if env.contains('/') {
            return Err(shape("expected one /"));
        }
        let (left, right) = env
            .split_once('_')
            .ok_or_else(|| shape("expected left _ right"))?;
        if right.contains('_') {
            return Err(shape("expected one _"));
        }
        (result.trim(), left.trim(), right.trim())
    } else {
        (rest.trim(), "", "")
    };
    if target.is_empty() || result.is_empty() {
        return Err(shape(
            "target and result are required; insertion is not supported",
        ));
    }
    let targets = matchers(target, true)?;
    let results = rewrites(result)?;
    let lefts = environments(left, false)?;
    let rights = environments(right, true)?;
    let mut out = Vec::new();
    for target in targets {
        for result in &results {
            let incompatible = match result {
                Rewrite::Vowel { .. } => !matcher_vocalic(&target),
                Rewrite::Consonant { .. } => matcher_vocalic(&target),
                _ => false,
            };
            if incompatible {
                return Err(shape("result features cannot be carried by the target"));
            }
            for left in &lefts {
                for right in &rights {
                    out.push(SoundChange {
                        id: String::new(),
                        target: target.clone(),
                        result: result.clone(),
                        left: left.clone(),
                        right: right.clone(),
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Plain words for an accepted line, read by the author beside the notation.
fn reading(text: &str) -> String {
    let (a, rest) = text.split_once('>').expect("checked rule");
    let (b, env) = rest
        .split_once('/')
        .map_or((rest, None), |(b, e)| (b, Some(e.trim())));
    let class = |term: &str| -> String {
        let lower = term.trim().to_ascii_lowercase();
        match lower.as_str() {
            "c" => "a consonant".into(),
            "v" => "a vowel".into(),
            "#" => "the edge of the word".into(),
            "[stressed]" => "a stressed vowel".into(),
            "[unstressed]" => "an unstressed vowel".into(),
            _ => match lower.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
                Some(terms) => format!("a {} sound", terms.trim()),
                None => term.trim().to_string(),
            },
        }
    };
    let context = match env {
        None => String::new(),
        Some("_ #") => " at the end of a word".into(),
        Some("# _") => " at the start of a word".into(),
        Some(e) => {
            let (left, right) = e.split_once('_').unwrap_or((e, ""));
            let (left, right) = (left.trim(), right.trim());
            match (left.is_empty(), right.strip_prefix("..")) {
                (true, Some(next)) => format!(" when the next syllable has {}", class(next.trim())),
                (false, Some(next)) => format!(
                    " after {} when the next syllable has {}",
                    class(left),
                    class(next.trim())
                ),
                (true, None) if right.is_empty() => String::new(),
                (true, None) => format!(" before {}", class(right)),
                (false, None) if right.is_empty() => format!(" after {}", class(left)),
                (false, None) => format!(" between {} and {}", class(left), class(right)),
            }
        }
    };
    let result = b.trim();
    if a.trim().eq_ignore_ascii_case("stress") {
        let place = match result.to_ascii_lowercase().as_str() {
            "initial" => "on the first syllable",
            "penult" => "on the next to last syllable",
            "final" => "on the last syllable",
            "weight" => "on a heavy next to last syllable, otherwise the one before it",
            _ => "where each word has it",
        };
        return format!("stress falls {place}");
    }
    let outcome = match result.to_ascii_lowercase().as_str() {
        "∅" | "0" | "ø" => "is lost".to_string(),
        "[long]" => "lengthens".into(),
        "[short]" => "shortens".into(),
        "[lengthen before]" => "is lost and lengthens the vowel before it".into(),
        "[geminate next]" => "is lost and doubles the consonant after it".into(),
        lower => match lower.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
            Some(terms) => format!("becomes {}", terms.trim()),
            None => format!("becomes {result}"),
        },
    };
    format!("{} {outcome}{context}", class(a))
}

#[derive(Clone, Default)]
struct Class {
    place: Option<Place>,
    manner: Option<Manner>,
    voiced: Option<bool>,
    secondary: Option<Secondary>,
    height: Option<Height>,
    backness: Option<Backness>,
    rounded: Option<bool>,
    long: Option<bool>,
    stressed: Option<bool>,
    open: bool,
    medial: bool,
    labial: bool,
}
fn set<T>(slot: &mut Option<T>, value: T, term: &str) -> Result<(), Error> {
    if slot.is_some() {
        return Err(shape(format!("conflicting or repeated feature: {term}")));
    }
    *slot = Some(value);
    Ok(())
}
impl Class {
    fn consonantal(&self) -> bool {
        self.place.is_some()
            || self.manner.is_some()
            || self.voiced.is_some()
            || self.secondary.is_some()
            || self.labial
    }
    fn vocalic(&self) -> bool {
        self.height.is_some()
            || self.backness.is_some()
            || self.rounded.is_some()
            || self.open
            || self.medial
    }
    fn parse(text: &str) -> Result<Self, Error> {
        let terms = text
            .strip_prefix('[')
            .and_then(|t| t.strip_suffix(']'))
            .ok_or_else(|| shape("expected [feature terms]"))?;
        if terms.trim().is_empty() {
            return Err(shape("empty feature class"));
        }
        let mut c = Self::default();
        for term in terms.split_whitespace().map(str::to_ascii_lowercase) {
            match term.as_str() {
                "bilabial" => set(&mut c.place, Place::Bilabial, &term)?,
                "labiodental" => set(&mut c.place, Place::Labiodental, &term)?,
                "dental" => set(&mut c.place, Place::Dental, &term)?,
                "alveolar" => set(&mut c.place, Place::Alveolar, &term)?,
                "postalveolar" => set(&mut c.place, Place::Postalveolar, &term)?,
                "retroflex" => set(&mut c.place, Place::Retroflex, &term)?,
                "palatal" => set(&mut c.place, Place::Palatal, &term)?,
                "velar" => set(&mut c.place, Place::Velar, &term)?,
                "uvular" => set(&mut c.place, Place::Uvular, &term)?,
                "pharyngeal" => set(&mut c.place, Place::Pharyngeal, &term)?,
                "glottal" => set(&mut c.place, Place::Glottal, &term)?,
                "labial" if !c.labial => c.labial = true,
                "stop" => set(&mut c.manner, Manner::Stop, &term)?,
                "nasal" => set(&mut c.manner, Manner::Nasal, &term)?,
                "trill" => set(&mut c.manner, Manner::Trill, &term)?,
                "tap" => set(&mut c.manner, Manner::Tap, &term)?,
                "fricative" => set(&mut c.manner, Manner::Fricative, &term)?,
                "affricate" => set(&mut c.manner, Manner::Affricate, &term)?,
                "approximant" => set(&mut c.manner, Manner::Approximant, &term)?,
                "lateral" => set(&mut c.manner, Manner::Lateral, &term)?,
                "implosive" => set(&mut c.manner, Manner::Implosive, &term)?,
                "ejective" => set(&mut c.manner, Manner::Ejective, &term)?,
                "voiced" => set(&mut c.voiced, true, &term)?,
                "voiceless" => set(&mut c.voiced, false, &term)?,
                "plain" => set(&mut c.secondary, Secondary::Plain, &term)?,
                "aspirated" => set(&mut c.secondary, Secondary::Aspirated, &term)?,
                "breathy" => set(&mut c.secondary, Secondary::Breathy, &term)?,
                "labialized" => set(&mut c.secondary, Secondary::Labialized, &term)?,
                "close" | "high" => set(&mut c.height, Height::Close, &term)?,
                "near-close" => set(&mut c.height, Height::NearClose, &term)?,
                "close-mid" => set(&mut c.height, Height::CloseMid, &term)?,
                "mid" => set(&mut c.height, Height::Mid, &term)?,
                "open-mid" => set(&mut c.height, Height::OpenMid, &term)?,
                "near-open" => set(&mut c.height, Height::NearOpen, &term)?,
                "open" | "low" => set(&mut c.height, Height::Open, &term)?,
                "front" => set(&mut c.backness, Backness::Front, &term)?,
                "central" => set(&mut c.backness, Backness::Central, &term)?,
                "back" => set(&mut c.backness, Backness::Back, &term)?,
                "round" => set(&mut c.rounded, true, &term)?,
                "unround" => set(&mut c.rounded, false, &term)?,
                "long" => set(&mut c.long, true, &term)?,
                "short" => set(&mut c.long, false, &term)?,
                "stressed" => set(&mut c.stressed, true, &term)?,
                "unstressed" => set(&mut c.stressed, false, &term)?,
                "open-syllable" if !c.open => c.open = true,
                "medial" if !c.medial => c.medial = true,
                _ => {
                    return Err((
                        RuleStatus::UnknownTerm,
                        format!("unknown feature term: {term}"),
                    ));
                }
            }
        }
        if c.consonantal() && c.vocalic() || c.labial && c.place.is_some() {
            return Err(shape("incompatible feature classes"));
        }
        Ok(c)
    }
    fn places(&self) -> Vec<Option<Place>> {
        if self.labial {
            vec![Some(Place::Bilabial), Some(Place::Labiodental)]
        } else {
            vec![self.place]
        }
    }
}

fn matchers(text: &str, target: bool) -> Result<Vec<Matcher>, Error> {
    match text {
        "C" => return Ok(vec![Matcher::AnyConsonant]),
        "V" => return Ok(vec![Matcher::AnyVowel]),
        _ => {}
    }
    if !text.starts_with('[') {
        return Ok(vec![Matcher::Phone(segment(text)?)]);
    }
    let c = Class::parse(text)?;
    if !target && (c.open || c.medial) {
        return Err(unsupported(format!("{text} is only supported in a target")));
    }
    if c.medial && (c.height.is_some() || c.backness.is_some() || c.rounded.is_some()) {
        return Err(unsupported(format!(
            "combined medial feature class {text} is unsupported"
        )));
    }
    if !c.consonantal()
        && !c.vocalic()
        && c.stressed.is_none()
        && let Some(long) = c.long
    {
        return Ok(vec![
            Matcher::Length {
                target: Box::new(Matcher::AnyConsonant),
                long,
            },
            Matcher::Length {
                target: Box::new(Matcher::AnyVowel),
                long,
            },
        ]);
    }
    let mut out = Vec::new();
    for place in c.places() {
        // A purely prosodic class is vocalic, as in [unstressed] syncope.
        let mut m = if c.medial {
            Matcher::MedialVowel
        } else if c.consonantal() {
            Matcher::Consonant {
                place,
                manner: c.manner,
                voiced: c.voiced,
                secondary: c.secondary,
            }
        } else {
            Matcher::Vowel {
                height: c.height,
                backness: c.backness,
                rounded: c.rounded,
            }
        };
        if let Some(long) = c.long {
            m = Matcher::Length {
                target: Box::new(m),
                long,
            };
        }
        if let Some(stressed) = c.stressed {
            m = Matcher::Stressed {
                target: Box::new(m),
                stressed,
            };
        }
        if c.open {
            m = Matcher::OpenSyllable(Box::new(m));
        }
        out.push(m);
    }
    Ok(out)
}
fn matcher_vocalic(m: &Matcher) -> bool {
    match m {
        Matcher::Phone(p) => CATALOG.get(*p).is_vowel(),
        Matcher::Consonant { .. } | Matcher::AnyConsonant => false,
        Matcher::Length { target, .. }
        | Matcher::Stressed { target, .. }
        | Matcher::OpenSyllable(target) => matcher_vocalic(target),
        _ => true,
    }
}
fn rewrites(text: &str) -> Result<Vec<Rewrite>, Error> {
    if matches!(text, "∅" | "0" | "Ø") {
        return Ok(vec![Rewrite::Delete]);
    }
    match text.to_ascii_lowercase().as_str() {
        "[lengthen before]" => return Ok(vec![Rewrite::Compensate]),
        "[geminate next]" => return Ok(vec![Rewrite::GeminateNext]),
        _ => {}
    }
    if text == "C" {
        return Ok(vec![Rewrite::Consonant {
            place: None,
            manner: None,
            voiced: None,
            secondary: None,
        }]);
    }
    if text == "V" {
        return Ok(vec![Rewrite::Vowel {
            height: None,
            backness: None,
            rounded: None,
        }]);
    }
    if !text.starts_with('[') {
        return Ok(vec![Rewrite::Phone(segment(text)?)]);
    }
    let c = Class::parse(text)?;
    if c.stressed.is_some()
        || c.open
        || c.medial
        || c.long.is_some() && (c.consonantal() || c.vocalic())
    {
        return Err(unsupported(format!(
            "result combination {text} is unsupported"
        )));
    }
    if let Some(long) = c.long {
        return Ok(vec![Rewrite::Length(long)]);
    }
    Ok(c.places()
        .into_iter()
        .map(|place| {
            if c.consonantal() {
                Rewrite::Consonant {
                    place,
                    manner: c.manner,
                    voiced: c.voiced,
                    secondary: c.secondary,
                }
            } else {
                Rewrite::Vowel {
                    height: c.height,
                    backness: c.backness,
                    rounded: c.rounded,
                }
            }
        })
        .collect())
}
fn environments(text: &str, right: bool) -> Result<Vec<Env>, Error> {
    match text {
        "" => return Ok(vec![Env::Any]),
        "#" => return Ok(vec![Env::WordEdge]),
        _ => {}
    }
    if let Some(class) = text.strip_prefix("..") {
        let class = class.trim();
        if !right || !(class == "V" || class.starts_with('[')) {
            return Err(unsupported(".. requires a vowel class on the right"));
        }
        let ms = matchers(class, false)?;
        if ms.iter().any(|m| !matcher_vocalic(m)) {
            return Err(shape(".. requires a vowel class"));
        }
        return Ok(ms.into_iter().map(Env::FollowingSyllableVowel).collect());
    }
    Ok(matchers(text, false)?
        .into_iter()
        .map(Env::Matcher)
        .collect())
}
fn segment(text: &str) -> Result<PhonemeId, Error> {
    if let Some(id) = CATALOG.id_by_ipa(text) {
        return Ok(id);
    }
    if CATALOG.parse_ipa(text).is_some_and(|p| p.len() > 1) {
        return Err(unsupported("only one segment each side is supported"));
    }
    // An unknown symbol has no feature bundle. Use the longest recognizable
    // IPA prefix as evidence; score catalog neighbors by integer feature overlap.
    let known = CATALOG
        .segments
        .iter()
        .filter(|p| text.starts_with(p.ipa()))
        .max_by_key(|p| p.ipa().len());
    let score = |p: &Segment| match (known, p) {
        (Some(Segment::Consonant(a)), Segment::Consonant(b)) => {
            1 + usize::from(a.place == b.place)
                + usize::from(a.manner == b.manner)
                + usize::from(a.voiced == b.voiced)
                + usize::from(a.secondary == b.secondary)
        }
        (Some(Segment::Vowel(a)), Segment::Vowel(b)) => {
            1 + usize::from(a.height == b.height)
                + usize::from(a.backness == b.backness)
                + usize::from(a.rounded == b.rounded)
        }
        _ => 0,
    };
    let mut nearest: Vec<_> = CATALOG.segments.iter().collect();
    nearest.sort_by_key(|p| std::cmp::Reverse(score(p)));
    let suggestions = nearest
        .iter()
        .take(3)
        .map(|p| p.ipa())
        .collect::<Vec<_>>()
        .join(", ");
    Err((
        RuleStatus::BadSegment,
        format!("unknown segment {text}; try {suggestions}"),
    ))
}

impl Law {
    /// Display source, not executable input. Expanded labial rows retain the
    /// original line; malformed display text falls back to the stored rule.
    pub fn rule_sources(&self) -> Vec<String> {
        let mut sources = Vec::new();
        if let Some(source) = &self.source {
            for line in source
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
            {
                if line
                    .split_once('>')
                    .is_some_and(|(a, _)| a.trim().eq_ignore_ascii_case("stress"))
                {
                    continue;
                }
                let labials = line
                    .split(|c: char| !c.is_alphabetic())
                    .filter(|s| s.eq_ignore_ascii_case("labial"))
                    .count();
                let length_classes = line.split_once('>').map_or(0, |(target, rest)| {
                    let is_length = |s: &str| {
                        matches!(s.trim().to_ascii_lowercase().as_str(), "[long]" | "[short]")
                    };
                    usize::from(is_length(target))
                        + rest.split_once('/').map_or(0, |(_, env)| {
                            env.split('_').filter(|s| is_length(s)).count()
                        })
                });
                if let Some(count) = 1usize
                    .checked_shl((labials + length_classes) as u32)
                    .filter(|n| *n <= self.rules.len())
                {
                    sources.extend(std::iter::repeat_n(line.to_owned(), count));
                }
            }
        }
        if sources.len() != self.rules.len() {
            sources = self.rules.iter().map(rule_source).collect();
        }
        if let Some(stress) = self.stress {
            sources.push(format!("stress > {}", stress.id()));
        }
        sources
    }
}

fn feature(value: impl std::fmt::Debug) -> String {
    let mut out = String::new();
    for c in format!("{value:?}").chars() {
        if c.is_uppercase() && !out.is_empty() {
            out.push('-');
        }
        out.extend(c.to_lowercase());
    }
    out
}
fn consonant_source(
    place: Option<Place>,
    manner: Option<Manner>,
    voiced: Option<bool>,
    secondary: Option<Secondary>,
) -> String {
    let terms: Vec<_> = [
        place.map(feature),
        manner.map(feature),
        voiced.map(|v| if v { "voiced" } else { "voiceless" }.into()),
        secondary.map(feature),
    ]
    .into_iter()
    .flatten()
    .collect();
    if terms.is_empty() {
        "C".into()
    } else {
        format!("[{}]", terms.join(" "))
    }
}
fn vowel_source(
    height: Option<Height>,
    backness: Option<Backness>,
    rounded: Option<bool>,
) -> String {
    let terms: Vec<_> = [
        height.map(feature),
        backness.map(feature),
        rounded.map(|v| if v { "round" } else { "unround" }.into()),
    ]
    .into_iter()
    .flatten()
    .collect();
    if terms.is_empty() {
        "V".into()
    } else {
        format!("[{}]", terms.join(" "))
    }
}
fn wrap_source(target: &Matcher, term: &str) -> String {
    let source = matcher_source(target);
    let inner = source
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(&source);
    format!("[{inner} {term}]")
}
fn matcher_source(m: &Matcher) -> String {
    match m {
        Matcher::Phone(p) => CATALOG.get(*p).ipa().into(),
        Matcher::AnyConsonant => "C".into(),
        Matcher::AnyVowel => "V".into(),
        Matcher::Consonant {
            place,
            manner,
            voiced,
            secondary,
        } => consonant_source(*place, *manner, *voiced, *secondary),
        Matcher::Vowel {
            height,
            backness,
            rounded,
        } => vowel_source(*height, *backness, *rounded),
        Matcher::Length { target, long } => {
            wrap_source(target, if *long { "long" } else { "short" })
        }
        Matcher::Stressed { target, stressed } => {
            wrap_source(target, if *stressed { "stressed" } else { "unstressed" })
        }
        Matcher::OpenSyllable(target) => wrap_source(target, "open-syllable"),
        Matcher::MedialVowel => "[medial]".into(),
        Matcher::RhythmicVowel { .. } => "[rhythmic vowel]".into(),
    }
}
fn environment_source(e: &Env) -> String {
    match e {
        Env::Any => String::new(),
        Env::WordEdge => "#".into(),
        Env::Matcher(m) => matcher_source(m),
        Env::FollowingSyllableVowel(m) => format!(".. {}", matcher_source(m)),
    }
}
fn rule_source(rule: &SoundChange) -> String {
    let result = match &rule.result {
        Rewrite::Phone(p) => CATALOG.get(*p).ipa().into(),
        Rewrite::Consonant {
            place,
            manner,
            voiced,
            secondary,
        } => consonant_source(*place, *manner, *voiced, *secondary),
        Rewrite::Vowel {
            height,
            backness,
            rounded,
        } => vowel_source(*height, *backness, *rounded),
        Rewrite::Delete => "∅".into(),
        Rewrite::Length(long) => if *long { "[long]" } else { "[short]" }.into(),
        Rewrite::Compensate => "[lengthen before]".into(),
        Rewrite::GeminateNext => "[geminate next]".into(),
        Rewrite::Tone(tone) => format!("[tone {tone:?}]"),
    };
    let base = format!("{} > {result}", matcher_source(&rule.target));
    if rule.left == Env::Any && rule.right == Env::Any {
        base
    } else {
        format!(
            "{base} / {} _ {}",
            environment_source(&rule.left),
            environment_source(&rule.right)
        )
    }
}

impl crate::World {
    pub fn law_by_id(&self, id: &str) -> Option<&Law> {
        self.law_catalog()
            .iter()
            .chain(&self.authored)
            .find(|law| law.id == id)
    }

    pub fn law_changes_living(&self, variety: usize, law: &Law) -> bool {
        let speech = &self.varieties[variety];
        if law.changes_any(
            speech.spoken_forms().map(|(form, _)| form),
            speech.minimal,
            speech.stress(),
        ) {
            return true;
        }
        let mut changed = false;
        self.visit_law_names(variety, |_, _, name| {
            if !changed {
                changed =
                    law.changes_any(std::iter::once(&name.form), speech.minimal, speech.stress());
            }
        });
        changed
    }

    /// The same living name scope as `change_names`, with stable subject ids.
    /// Exonym subjects use their own land, river, or lake catalog's ids.
    pub fn visit_law_names<'a>(
        &'a self,
        v: usize,
        mut visit: impl FnMut(&'static str, Option<usize>, &'a crate::Name),
    ) {
        let variety = &self.varieties[v];
        visit("language", Some(v), &variety.name);
        for (id, community) in self
            .communities
            .iter()
            .enumerate()
            .filter(|(_, c)| c.variety == v && c.living())
        {
            visit("people", Some(id), &community.name);
        }
        for given in &variety.given {
            visit("person", None, &given.name);
        }
        for (id, state) in self.states.iter().enumerate() {
            if state.standing() && self.communities[state.rulers].variety == v {
                visit("state", Some(id), &state.name);
            }
        }
        let held = |region| {
            self.communities
                .iter()
                .any(|c| c.variety == v && c.living() && c.lands.contains(&region))
        };
        for (id, places) in self.places.iter().enumerate() {
            if held(id)
                && let Some(p) = places.last().filter(|p| p.variety == v)
            {
                visit("land", Some(id), &p.name);
            }
        }
        for (id, names) in self.river_names.iter().enumerate() {
            if let Some(p) = names.last().filter(|p| p.variety == v)
                && self
                    .map
                    .river_regions
                    .iter()
                    .enumerate()
                    .any(|(r, river)| *river == Some(id) && held(r))
            {
                visit("river", Some(id), &p.name);
            }
        }
        for (id, names) in self.lake_names.iter().enumerate() {
            if let Some(p) = names.last().filter(|p| p.variety == v)
                && self
                    .map
                    .lake_regions
                    .iter()
                    .enumerate()
                    .any(|(r, lake)| *lake == Some(id) && held(r))
            {
                visit("lake", Some(id), &p.name);
            }
        }
        for (id, name) in variety
            .exonyms
            .iter()
            .chain(&variety.river_exonyms)
            .chain(&variety.lake_exonyms)
        {
            visit("exonym", Some(*id), name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn examples_and_round_trip() {
        for source in [
            "[voiceless stop] > [fricative]",
            "[voiceless stop] > [voiced] / V _ V",
            "V > ∅ / _ #",
            "a > e / _ .. [front]",
            "k > tʃ / _ [front]",
            "s > [lengthen before] / _ C",
            "s > [geminate next] / _ C",
            "stress > penult",
        ] {
            let report = parse("Test law", source);
            assert!(report.law.is_some(), "{report:?}");
            let law = report.law.unwrap();
            let encoded = serde_json::to_string(&law).unwrap();
            assert_eq!(law, serde_json::from_str(&encoded).unwrap());
            assert!(
                law.rules
                    .iter()
                    .enumerate()
                    .all(|(i, r)| r.id == format!("test-law/{i}"))
            );
        }
    }
    #[test]
    fn reports_every_status() {
        for (source, status) in [
            ("p > f", RuleStatus::Ok),
            ("# comment", RuleStatus::Empty),
            ("p̰ > f", RuleStatus::BadSegment),
            ("[odd] > f", RuleStatus::UnknownTerm),
            ("p f", RuleStatus::BadShape),
            ("pa > f", RuleStatus::Unsupported),
            (
                "stress > final\nstress > initial",
                RuleStatus::DuplicateStress,
            ),
        ] {
            let r = parse("Test", source);
            assert_eq!(r.rules.last().unwrap().status, status, "{source}");
        }
        assert_eq!(parse("", "p > f").rules[0].message, "needs a title");
        assert!(parse("Test", "# nothing\n\n").law.is_none());
        for source in [
            "a˥ > e",
            "p > [stressed]",
            "a > [long front]",
            "[medial high] > ∅",
            "p > f / _ ab",
        ] {
            assert_eq!(
                parse("Test", source).rules[0].status,
                RuleStatus::Unsupported,
                "{source}"
            );
        }
        for source in ["[front stop] > e", "C > [front]", "p > [voiced voiceless]"] {
            assert_eq!(
                parse("Test", source).rules[0].status,
                RuleStatus::BadShape,
                "{source}"
            );
        }
    }
    #[test]
    fn labial_and_prosody_preserve_constraints() {
        let law = parse("Labials", "[labial voiceless] > [voiced]")
            .law
            .unwrap();
        assert_eq!(law.rules.len(), 2);
        assert!(matches!(
            law.rules[0].target,
            Matcher::Consonant {
                place: Some(Place::Bilabial),
                ..
            }
        ));
        assert!(matches!(
            law.rules[1].target,
            Matcher::Consonant {
                place: Some(Place::Labiodental),
                ..
            }
        ));
        let law = parse("Length", "[open-syllable short unstressed] > [long]")
            .law
            .unwrap()
            .law(0);
        assert_ne!(
            law.apply(
                &crate::Form::from_ipa("papa").unwrap(),
                crate::MinimalWord::Syllable,
                StressRule::Initial
            )
            .ipa(),
            "papa"
        );
    }
    #[test]
    fn examples_produce_their_regular_outputs_and_bare_length_includes_geminates() {
        for (source, before, after) in [
            ("[voiceless stop] > [fricative]", "paka", "ɸaxa"),
            ("[voiceless stop] > [voiced] / V _ V", "pata", "pada"),
            ("V > ∅ / _ #", "pata", "pat"),
            ("a > e / _ .. [front]", "pati", "peti"),
            ("k > tʃ / _ [front]", "ki", "tʃi"),
            ("s > [lengthen before] / _ C", "asta", "aːta"),
            ("[long] > [short]", "pːaː", "pa"),
            ("[stressed] > [long]", "pata", "paːta"),
            ("[unstressed] > [long]", "pata", "pataː"),
            ("a > ø", "pa", "pø"),
        ] {
            let law = parse("Output", source).law.unwrap().law(0);
            assert_eq!(
                law.apply(
                    &crate::Form::from_ipa(before).unwrap(),
                    crate::MinimalWord::Syllable,
                    StressRule::Initial
                )
                .ipa(),
                after,
                "{source}"
            );
        }
        let report = parse("Length", "[long] > [short]");
        let law = report.law.unwrap();
        assert_eq!(law.rules.len(), 2);
        assert_eq!(law.law(0).rule_sources(), vec!["[long] > [short]"; 2]);
        assert!(
            parse("Stress", "[stressed] > [long]").rules[0]
                .reading
                .as_ref()
                .unwrap()
                .contains("a stressed vowel lengthens")
        );
    }
    #[test]
    fn apocope_protects_last_vowel_and_minimal_word() {
        let law = parse("Apocope", "V > ∅ / _ #").law.unwrap().law(0);
        for minimal in [
            crate::MinimalWord::Syllable,
            crate::MinimalWord::Heavy,
            crate::MinimalWord::TwoSyllables,
        ] {
            for ipa in ["a", "pa", "pata", "pataka", "aa"] {
                let before = crate::Form::from_ipa(ipa).unwrap();
                let after = law.apply(&before, minimal, StressRule::Penult);
                assert!(after.vowel_count() > 0);
                assert!(!minimal.blocks(&before, &after));
            }
        }
    }
}

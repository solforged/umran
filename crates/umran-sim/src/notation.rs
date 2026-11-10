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
            "target and result are required; write ∅ where there is no sound",
        ));
    }
    let lefts = side(left, false)?;
    let rights = side(right, true)?;
    if nothing(target) {
        return insertion(result, &lefts, &rights);
    }
    if let Some([first, second]) = pair(target) {
        if rights.iter().flatten().any(reaches) {
            return Err(unsupported(".. reads on from a single target"));
        }
        let result = match sounds(result).as_deref() {
            Ok([a, b]) if (*a, *b) == (second, first) => Rewrite::SwapNext,
            Ok([(phone, false)]) => Rewrite::Fuse(*phone),
            Ok([(_, true)]) => {
                return Err(unsupported(
                    "change length on its own line: ai > e, then e > eː",
                ));
            }
            _ => {
                return Err(unsupported(
                    "two sounds may trade places (sk > ks) or become one (ai > e)",
                ));
            }
        };
        // The second sound is the first thing to the target's right.
        let mut after = vec![vec![Env::Matcher(sound_matcher(second))]];
        after.extend(rights);
        return rules(
            vec![sound_matcher(first)],
            &[result],
            &runs(&lefts, false),
            &runs(&after, true),
        );
    }
    // A class beside another sound is not a segment the catalog lacks.
    let mixed = |text: &str| {
        elements(text).is_ok_and(|tokens| {
            tokens.len() > 1
                && tokens
                    .iter()
                    .any(|(_, token)| matches!(*token, "C" | "V") || token.starts_with('['))
        })
    };
    if mixed(target) {
        return Err(unsupported(
            "a class changes alone; to trade places write r > [swap next] / _ V",
        ));
    }
    if mixed(result) {
        return Err(unsupported("a result is one class, or one or two sounds"));
    }
    let targets = matchers(target, true)?;
    let results = match (long_phone(target), long_phone(result)) {
        // uː > u shortens, u > uː lengthens, aː > eː keeps the length.
        (Some(t), None) if t == result => vec![Rewrite::Length(false)],
        (None, Some(r)) if r == target => vec![Rewrite::Length(true)],
        (Some(_), Some(r)) => vec![Rewrite::Phone(segment(r)?)],
        // A long sound may also be lost or take a class: aː > ∅, aː > [short].
        (Some(_), None) if CATALOG.id_by_ipa(result).is_none() => rewrites(result)?,
        (Some(_), None) | (None, Some(_)) => {
            return Err(unsupported(
                "change length and quality on separate lines: aː > a, then a > e",
            ));
        }
        (None, None) => rewrites(result)?,
    };
    rules(
        targets,
        &results,
        &runs(&lefts, false),
        &runs(&rights, true),
    )
}

/// One rule for every way the line's classes expand.
fn rules(
    targets: Vec<Matcher>,
    results: &[Rewrite],
    lefts: &[Env],
    rights: &[Env],
) -> Result<Vec<SoundChange>, Error> {
    let mut out = Vec::new();
    for target in targets {
        for result in results {
            let incompatible = match result {
                Rewrite::Vowel { .. } => !matcher_vocalic(&target),
                Rewrite::Consonant { .. } => matcher_vocalic(&target),
                _ => false,
            };
            if incompatible {
                return Err(shape("result features cannot be carried by the target"));
            }
            for left in lefts {
                for right in rights {
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

/// The zero is ∅, 0, or capital Ø; ø is a vowel.
fn nothing(text: &str) -> bool {
    matches!(text, "∅" | "0" | "Ø")
}

/// A sound that appears where none was (epenthesis). A rule needs a
/// segment to act on, so the new sound hangs on a neighbour: the sound
/// after the blank where the line names one, otherwise the sound before.
fn insertion(
    result: &str,
    lefts: &[Vec<Env>],
    rights: &[Vec<Env>],
) -> Result<Vec<SoundChange>, Error> {
    if long_phone(result).is_some() {
        return Err(unsupported(
            "a new sound is short; lengthen it on its own line",
        ));
    }
    let phone = segment(result)?;
    let anchors = |position: &Vec<Env>| -> Option<Vec<Matcher>> {
        position
            .iter()
            .map(|env| match env {
                Env::Matcher(m) => Some(m.clone()),
                _ => None,
            })
            .collect()
    };
    let (targets, after, lefts, rights) = if let Some(found) = rights.first().and_then(anchors) {
        (found, false, lefts, &rights[1..])
    } else if let Some(found) = lefts.last().and_then(anchors) {
        (found, true, &lefts[..lefts.len() - 1], rights)
    } else {
        return Err(shape(
            "say what the new sound stands beside, as in ∅ > e / # _ sC",
        ));
    };
    rules(
        targets,
        &[Rewrite::Insert { phone, after }],
        &runs(lefts, false),
        &runs(rights, true),
    )
}

/// Two catalog sounds written as a target, as `sk` or `ai`. A spelling the
/// catalog reads as one sound, as `ts`, stays that sound; write `t s`.
fn pair(text: &str) -> Option<[(PhonemeId, bool); 2]> {
    if text.starts_with('[') || CATALOG.id_by_ipa(text).is_some() || long_phone(text).is_some() {
        return None;
    }
    <[_; 2]>::try_from(sounds(text).ok()?).ok()
}

/// Catalog sounds written together or apart, each with whether it is
/// written long. The longest spelling the catalog knows is read first.
fn sounds(text: &str) -> Result<Vec<(PhonemeId, bool)>, Error> {
    let canonical = crate::phoneme::canonical_ipa(text);
    let mut rest = canonical.trim();
    let mut out = Vec::new();
    while !rest.is_empty() {
        let Some(found) = CATALOG
            .segments
            .iter()
            .filter(|p| rest.starts_with(p.ipa()))
            .max_by_key(|p| p.ipa().len())
        else {
            return Err(segment(text)
                .err()
                .unwrap_or_else(|| shape(format!("cannot read {text}"))));
        };
        rest = &rest[found.ipa().len()..];
        let long = rest.starts_with('ː');
        if long {
            rest = &rest['ː'.len_utf8()..];
        }
        out.push((CATALOG.id_by_ipa(found.ipa()).expect("catalog sound"), long));
        rest = rest.trim_start();
    }
    Ok(out)
}

fn sound_matcher((phone, long): (PhonemeId, bool)) -> Matcher {
    if long {
        Matcher::Length {
            target: Box::new(Matcher::Phone(phone)),
            long,
        }
    } else {
        Matcher::Phone(phone)
    }
}

fn reaches(env: &Env) -> bool {
    matches!(env, Env::FollowingSyllableVowel(_))
}

/// One side of the blank cut into what stands at each position: `#`, `C`,
/// `V`, a bracketed class, or a run of sounds, each perhaps negated by `!`.
fn elements(text: &str) -> Result<Vec<(bool, &str)>, Error> {
    let mut out = Vec::new();
    let mut rest = text.trim_start();
    while let Some(first) = rest.chars().next() {
        let negated = first == '!';
        if negated {
            rest = rest[1..].trim_start();
        }
        let end = match rest.chars().next() {
            None => return Err(shape("! needs a sound or a class after it")),
            Some('#' | 'C' | 'V') => 1,
            Some('[') => rest
                .find(']')
                .map(|at| at + 1)
                .ok_or_else(|| shape("expected [feature terms]"))?,
            Some(_) => rest
                .find(|c: char| c.is_whitespace() || matches!(c, '#' | 'C' | 'V' | '[' | '!'))
                .unwrap_or(rest.len()),
        };
        if end == 0 {
            return Err(shape("! needs a sound or a class after it"));
        }
        out.push((negated, &rest[..end]));
        rest = rest[end..].trim_start();
    }
    Ok(out)
}

/// One side of the blank as its positions in reading order. Each position
/// lists the alternatives its class expands to.
fn side(text: &str, right: bool) -> Result<Vec<Vec<Env>>, Error> {
    if text.starts_with("..") {
        return Ok(vec![environments(text, right)?]);
    }
    let mut out: Vec<Vec<Env>> = Vec::new();
    for (negated, token) in elements(text)? {
        let single = matches!(token, "#" | "C" | "V")
            || token.starts_with('[')
            || long_phone(token).is_some()
            || CATALOG.id_by_ipa(token).is_some();
        let mut found: Vec<Vec<Env>> = match sounds(token) {
            Ok(several) if !single => several
                .into_iter()
                .map(|sound| vec![Env::Matcher(sound_matcher(sound))])
                .collect(),
            _ => vec![environments(token, right)?],
        };
        if negated {
            // "Neither bilabial nor labiodental" is not a choice between
            // two rules, so a class that expands cannot be negated.
            let [position] = &mut found[..] else {
                return Err(unsupported(format!("!{token} must name one sound")));
            };
            let [env] = &position[..] else {
                return Err(unsupported(format!("!{token} must name one class")));
            };
            *position = vec![Env::Not(Box::new(env.clone()))];
        }
        out.extend(found);
    }
    let edge = |position: &Vec<Env>| {
        position.iter().any(|env| match env {
            Env::WordEdge => true,
            Env::Not(inner) => **inner == Env::WordEdge,
            _ => false,
        })
    };
    let outer = if right {
        out.len().saturating_sub(1)
    } else {
        0
    };
    if out
        .iter()
        .enumerate()
        .any(|(at, position)| at != outer && edge(position))
    {
        return Err(shape("# is the edge of the word; nothing stands beyond it"));
    }
    if out.len() > 3 {
        return Err(unsupported(
            "at most three sounds on each side of the blank",
        ));
    }
    Ok(out)
}

/// Every environment the positions of one side spell, read outward from
/// the blank.
fn runs(positions: &[Vec<Env>], right: bool) -> Vec<Env> {
    let mut outward: Vec<&Vec<Env>> = positions.iter().collect();
    if !right {
        outward.reverse();
    }
    match outward[..] {
        [] => vec![Env::Any],
        [only] => only.clone(),
        _ => {
            let mut out: Vec<Vec<Env>> = vec![Vec::new()];
            for position in outward {
                out = out
                    .into_iter()
                    .flat_map(|run| {
                        position.iter().map(move |env| {
                            let mut run = run.clone();
                            run.push(env.clone());
                            run
                        })
                    })
                    .collect();
            }
            out.into_iter().map(Env::Run).collect()
        }
    }
}

/// Plain words for an accepted line, read by the author beside the notation.
fn reading(text: &str) -> String {
    let (a, rest) = text.split_once('>').expect("checked rule");
    let (b, env) = rest
        .split_once('/')
        .map_or((rest, None), |(b, e)| (b, Some(e.trim())));
    let (a, result) = (a.trim(), b.trim());
    if a.eq_ignore_ascii_case("stress") {
        let place = match result.to_ascii_lowercase().as_str() {
            "initial" => "on the first syllable",
            "penult" => "on the next to last syllable",
            "final" => "on the last syllable",
            "weight" => "on a heavy next to last syllable, otherwise the one before it",
            _ => "where each word has it",
        };
        return format!("stress falls {place}");
    }
    // Capital C and V are the classes; lower-case c and v are sounds.
    let class = |term: &str| -> String {
        let term = term.trim();
        let lower = term.to_ascii_lowercase();
        match (term, lower.as_str()) {
            ("C", _) => "a consonant".into(),
            ("V", _) => "a vowel".into(),
            ("#", _) => "the edge of the word".into(),
            (_, "[stressed]") => "a stressed vowel".into(),
            (_, "[unstressed]") => "an unstressed vowel".into(),
            _ => match lower.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
                Some(terms) => format!("a {} sound", terms.trim()),
                None => term.to_string(),
            },
        }
    };
    // One side of the blank: its sounds in words, whether it reaches the
    // word's edge, and whether it is one sound ruled out.
    let named = |side: &str| -> (String, bool, bool) {
        let tokens = elements(side).unwrap_or_default();
        let edge = tokens.contains(&(false, "#"));
        let sounds: Vec<&(bool, &str)> = tokens.iter().filter(|t| **t != (false, "#")).collect();
        if let [(true, only)] = sounds[..] {
            return (class(only), edge, true);
        }
        let names: Vec<String> = sounds
            .iter()
            .map(|(negated, token)| {
                if *negated {
                    format!("anything but {}", class(token))
                } else {
                    class(token)
                }
            })
            .collect();
        (names.join(" then "), edge, false)
    };
    let context = match env {
        None => String::new(),
        Some(e) => {
            let (left, right) = e.split_once('_').unwrap_or((e, ""));
            let (left, right) = (left.trim(), right.trim());
            let (after, starts, not_after) = named(left);
            let (after_words, except_after) = if not_after {
                (
                    format!("anything but {after}"),
                    format!(" except after {after}"),
                )
            } else {
                (after.clone(), format!(" after {after}"))
            };
            if let Some(next) = right.strip_prefix("..") {
                let next = class(next.trim());
                let place = if after.is_empty() {
                    String::new()
                } else {
                    except_after
                };
                format!("{place} when the next syllable has {next}")
            } else {
                let (before, ends, not_before) = named(right);
                let mut place = match (after.is_empty(), before.is_empty()) {
                    (true, true) => String::new(),
                    (false, true) => except_after,
                    (true, false) if not_before => format!(" except before {before}"),
                    (true, false) => format!(" before {before}"),
                    (false, false) if not_before => {
                        format!(" between {after_words} and anything but {before}")
                    }
                    (false, false) => format!(" between {after_words} and {before}"),
                };
                place.push_str(match (starts, ends) {
                    (true, true) => " as a whole word",
                    (true, false) => " at the start of a word",
                    (false, true) => " at the end of a word",
                    (false, false) => "",
                });
                place
            }
        }
    };
    if nothing(a) {
        return format!("{result} appears{context}");
    }
    if let Some([first, second]) = pair(a) {
        let said = |(phone, long): (PhonemeId, bool)| {
            format!(
                "{}{}",
                CATALOG.get(phone).ipa(),
                if long { "ː" } else { "" }
            )
        };
        let (first, second) = (said(first), said(second));
        return if sounds(result).is_ok_and(|found| found.len() == 2) {
            format!("{first} and {second} trade places{context}")
        } else {
            format!("{first} and {second} become {result}{context}")
        };
    }
    let shortened = a.strip_suffix('ː') == Some(result);
    let lengthened = result.strip_suffix('ː') == Some(a);
    let outcome = match result.to_ascii_lowercase().as_str() {
        _ if shortened => "shortens".to_string(),
        _ if lengthened => "lengthens".to_string(),
        _ if nothing(result) => "is lost".to_string(),
        "[long]" => "lengthens".into(),
        "[short]" => "shortens".into(),
        "[lengthen before]" => "is lost and lengthens the vowel before it".into(),
        "[geminate next]" => "is lost and doubles the consonant after it".into(),
        "[swap next]" => "trades places with the sound after it".into(),
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
    if let Some(phone) = long_phone(text) {
        return Ok(vec![Matcher::Length {
            target: Box::new(Matcher::Phone(segment(phone)?)),
            long: true,
        }]);
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
    if nothing(text) {
        return Ok(vec![Rewrite::Delete]);
    }
    match text.to_ascii_lowercase().as_str() {
        "[lengthen before]" => return Ok(vec![Rewrite::Compensate]),
        "[geminate next]" => return Ok(vec![Rewrite::GeminateNext]),
        "[swap next]" => return Ok(vec![Rewrite::SwapNext]),
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
        // One sound may become two, as e > je.
        if CATALOG.id_by_ipa(text).is_none()
            && let Ok(found) = sounds(text)
            && let [(first, first_long), (second, second_long)] = found[..]
        {
            if first_long || second_long {
                return Err(unsupported(
                    "the two new sounds are short; lengthen one on its own line",
                ));
            }
            return Ok(vec![Rewrite::Break(first, second)]);
        }
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
/// A way of writing a sound that the catalog's IPA spells otherwise.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SoundHint {
    pub written: String,
    pub ipa: String,
}

/// What the author probably meant where `text` writes a sound the way a
/// romanization would: `sh` for ʃ, `u:` or `uu` for uː, and, given the
/// language's sounds, `r` for its ɾ. With `notation`, the law notation's own
/// symbols and feature classes are skipped. Hints only; `write_hint` rewrites.
pub fn sound_hints(text: &str, inventory: Option<&[PhonemeId]>, notation: bool) -> Vec<SoundHint> {
    let has = |id: PhonemeId| inventory.is_none_or(|sounds| sounds.contains(&id));
    let mut hints: Vec<SoundHint> = Vec::new();
    let mut hint = |written: &str, ipa: &str| {
        if written != ipa && !hints.iter().any(|h| h.written == written) {
            hints.push(SoundHint {
                written: written.into(),
                ipa: ipa.into(),
            });
        }
    };
    let mut tokens = Vec::new();
    for line in text.lines() {
        let mut plain = String::new();
        for (sounds, part) in sounding(line, notation) {
            if !sounds {
                plain.push(' ');
            } else if notation {
                plain.extend(part.chars().map(|c| match c {
                    '>' | '/' | '_' | '#' | '∅' | 'Ø' => ' ',
                    c => c,
                }));
            } else {
                plain.push_str(part);
            }
        }
        if notation {
            plain = plain.replace("..", " ");
        }
        tokens.extend(
            plain
                .split_whitespace()
                .filter(|t| !notation || !matches!(*t, "C" | "V" | "0"))
                .map(str::to_owned),
        );
    }
    for token in &tokens {
        if token.contains(':') {
            hint(":", "ː");
        }
        let chars: Vec<char> = token.chars().collect();
        for pair in chars.windows(2) {
            if pair[0] == pair[1]
                && let Some(id) = CATALOG.id_by_ipa(&pair[0].to_string())
                && has(id)
            {
                hint(&format!("{0}{0}", pair[0]), &format!("{}ː", pair[0]));
            }
        }
        for (i, segment) in CATALOG.segments.iter().enumerate() {
            let id = PhonemeId(i as u16);
            let roman = segment.roman();
            if roman == segment.ipa() || !token.contains(roman) || !has(id) {
                continue;
            }
            if roman.chars().count() > 1 {
                hint(roman, segment.ipa());
            } else if let Some(sounds) = inventory {
                // A single letter the language lacks, where its own sound is
                // written that way: r for a tapped ɾ.
                let lacks = CATALOG
                    .id_by_ipa(roman)
                    .is_none_or(|other| !sounds.contains(&other));
                if lacks {
                    hint(roman, segment.ipa());
                }
            }
        }
    }
    hints
}

/// `text` with one of its hints taken: `written` becomes the catalog's IPA
/// wherever the text writes sounds. In the law notation, feature classes
/// (`[stressed]`, `[short]`) and stress lines keep their spelling. A
/// `written` that `sound_hints` does not offer for this text changes nothing.
pub fn write_hint(
    text: &str,
    inventory: Option<&[PhonemeId]>,
    notation: bool,
    written: &str,
) -> String {
    let Some(hint) = sound_hints(text, inventory, notation)
        .into_iter()
        .find(|hint| hint.written == written)
    else {
        return text.to_owned();
    };
    text.split('\n')
        .map(|line| {
            sounding(line, notation)
                .into_iter()
                .map(|(sounds, part)| {
                    if sounds {
                        part.replace(&hint.written, &hint.ipa)
                    } else {
                        part.to_owned()
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A line cut into the stretches that write sounds and those that do not.
/// Outside the law notation the whole line writes sounds. Within it, a
/// feature class in brackets and the whole of a stress line do not.
fn sounding(line: &str, notation: bool) -> Vec<(bool, &str)> {
    if !notation {
        return vec![(true, line)];
    }
    if line.trim_start().to_ascii_lowercase().starts_with("stress") {
        return vec![(false, line)];
    }
    let mut parts = Vec::new();
    let (mut start, mut depth) = (0, 0usize);
    for (at, c) in line.char_indices() {
        match c {
            '[' => {
                if depth == 0 {
                    parts.push((true, &line[start..at]));
                    start = at;
                }
                depth += 1;
            }
            ']' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let end = at + c.len_utf8();
                    parts.push((false, &line[start..end]));
                    start = end;
                }
            }
            _ => {}
        }
    }
    // An unclosed class is still a class being typed, not sounds.
    parts.push((depth == 0, &line[start..]));
    parts.retain(|(_, part)| !part.is_empty());
    parts
}

/// A catalog sound written long, as `uː` or `pː`: the sound without its mark.
fn long_phone(text: &str) -> Option<&str> {
    text.strip_suffix('ː')
        .filter(|phone| CATALOG.id_by_ipa(phone).is_some())
}
fn segment(text: &str) -> Result<PhonemeId, Error> {
    if let Some(id) = CATALOG.id_by_ipa(text) {
        return Ok(id);
    }
    if CATALOG.parse_ipa(text).is_some_and(|p| p.len() > 1) {
        return Err(unsupported("at most two sounds change at once"));
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

    /// The law written in notation that reads back as these same rules, for
    /// an author to adapt. `None` where the notation cannot say this law:
    /// its display source would be refused or would mean something else.
    pub fn writable_source(&self) -> Option<String> {
        let source = self.source.clone().unwrap_or_else(|| {
            let mut lines: Vec<String> = Vec::new();
            for line in self.rule_sources() {
                if !lines.contains(&line) {
                    lines.push(line);
                }
            }
            lines.join("\n")
        });
        let read = parse("law", &source).law?;
        let unnamed = |rule: &SoundChange| SoundChange {
            id: String::new(),
            ..rule.clone()
        };
        let same = read.stress == self.stress
            && read.rules.len() == self.rules.len()
            && read
                .rules
                .iter()
                .zip(&self.rules)
                .all(|(a, b)| unnamed(a) == unnamed(b));
        same.then_some(source)
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
        Matcher::Length { target, long: true } if matches!(**target, Matcher::Phone(_)) => {
            format!("{}ː", matcher_source(target))
        }
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
/// One side of the blank in reading order.
fn environment_source(e: &Env, right: bool) -> String {
    match e {
        Env::Any => String::new(),
        Env::WordEdge => "#".into(),
        Env::Matcher(m) => matcher_source(m),
        Env::FollowingSyllableVowel(m) => format!(".. {}", matcher_source(m)),
        Env::Not(inner) => format!("!{}", environment_source(inner, right)),
        Env::Run(envs) => {
            let mut parts: Vec<String> = envs
                .iter()
                .map(|env| environment_source(env, right))
                .collect();
            if !right {
                parts.reverse();
            }
            parts.join(" ")
        }
    }
}
/// Two sounds side by side, set apart where the catalog would read the
/// pair as one sound.
fn beside(a: &str, b: &str) -> String {
    if CATALOG.id_by_ipa(&format!("{a}{b}")).is_some() || a.ends_with('ː') {
        format!("{a} {b}")
    } else {
        format!("{a}{b}")
    }
}
fn rule_source(rule: &SoundChange) -> String {
    let ipa = |p: &PhonemeId| CATALOG.get(*p).ipa();
    let target = matcher_source(&rule.target);
    let (left, right) = (
        environment_source(&rule.left, false),
        environment_source(&rule.right, true),
    );
    let around = |left: &str, right: &str| {
        if left.is_empty() && right.is_empty() {
            String::new()
        } else {
            format!(" / {left} _ {right}")
        }
    };
    let join = |a: &str, b: &str| {
        [a, b]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ")
    };
    match &rule.result {
        // The blank stands where the new sound appears, beside its anchor.
        Rewrite::Insert {
            phone,
            after: false,
        } => {
            return format!(
                "∅ > {}{}",
                ipa(phone),
                around(&left, &join(&target, &right))
            );
        }
        Rewrite::Insert { phone, after: true } => {
            return format!(
                "∅ > {}{}",
                ipa(phone),
                around(&join(&left, &target), &right)
            );
        }
        // The second sound of the pair is the first thing to the right.
        Rewrite::Fuse(phone) => {
            let (second, rest) = match &rule.right {
                Env::Run(envs) => (
                    envs.first().map(|env| environment_source(env, true)),
                    environment_source(&Env::Run(envs[1..].to_vec()), true),
                ),
                Env::Any => (None, String::new()),
                only => (Some(environment_source(only, true)), String::new()),
            };
            if let Some(second) = second {
                return format!(
                    "{} > {}{}",
                    beside(&target, &second),
                    ipa(phone),
                    around(&left, &rest)
                );
            }
        }
        _ => {}
    }
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
        Rewrite::SwapNext => "[swap next]".into(),
        Rewrite::Break(first, second) => beside(ipa(first), ipa(second)),
        Rewrite::Fuse(phone) => format!("[fuse {}]", ipa(phone)),
        Rewrite::Insert { phone, .. } => format!("[insert {}]", ipa(phone)),
    };
    format!("{target} > {result}{}", around(&left, &right))
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
        for (id, name) in &variety.exonyms {
            visit("exonym", Some(*id), name);
        }
        for (id, name) in &variety.river_exonyms {
            visit("river-exonym", Some(*id), name);
        }
        for (id, name) in &variety.lake_exonyms {
            visit("lake-exonym", Some(*id), name);
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
            ("pat > f", RuleStatus::Unsupported),
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
            "p > f / _ abab",
            "p > f / ![labial] _",
            "sk > ʃt",
            "ai > eː",
            "e > jeː",
            "∅ > eː / # _ s",
            "sk > ks / _ .. [front]",
            "rV > Vr",
            "r > Vr",
        ] {
            assert_eq!(
                parse("Test", source).rules[0].status,
                RuleStatus::Unsupported,
                "{source}"
            );
        }
        for source in [
            "[front stop] > e",
            "C > [front]",
            "p > [voiced voiceless]",
            "∅ > e",
            "∅ > e / # _ #",
            "∅ > e / !s _",
            "p > f / _ # a",
            "p > f / a # _",
            "p > f / ! _",
        ] {
            assert_eq!(
                parse("Test", source).rules[0].status,
                RuleStatus::BadShape,
                "{source}"
            );
        }
    }
    /// What each line makes of each word, under initial stress.
    fn changed(source: &str, words: &[(&str, &str)]) {
        let report = parse("Test", source);
        let law = report
            .law
            .unwrap_or_else(|| panic!("{source}: {:?}", report.rules));
        for (before, after) in words {
            let form = crate::Form::from_ipa(before).unwrap();
            let out = crate::change::apply_all(&form, &law.rules, StressRule::Initial);
            assert_eq!(out.ipa(), *after, "{source} on {before}");
        }
    }

    #[test]
    fn a_sound_appears_beside_the_sounds_the_line_names() {
        // Prothesis, as Latin schola gave Spanish escuela.
        changed(
            "∅ > e / # _ sC",
            &[("skola", "eskola"), ("sala", "sala"), ("askola", "askola")],
        );
        // The same with the sounds set apart, and with a class.
        changed(
            "∅ > e / # _ s [stop]",
            &[("skola", "eskola"), ("sma", "sma")],
        );
        // A vowel at the end, hung on the sound before the blank.
        changed("∅ > e / C _ #", &[("pat", "pate"), ("pata", "pata")]);
        // A cluster broken up, and a stop that grows between two sounds.
        changed("∅ > ə / C _ C #", &[("libr", "libər"), ("libra", "libra")]);
        changed("∅ > d / n _ r", &[("venra", "vendra"), ("vera", "vera")]);
        // Every place that fits takes the sound at once.
        changed("∅ > a / C _ C", &[("kstri", "kasatari")]);
    }

    #[test]
    fn two_sounds_trade_places_or_become_one_and_one_becomes_two() {
        changed("sk > ks", &[("aska", "aksa"), ("saka", "saka")]);
        changed("sk > ks / _ #", &[("ask", "aks"), ("aska", "aska")]);
        changed(
            "[fricative] > [swap next] / _ [stop]",
            &[("aspa", "apsa"), ("asa", "asa"), ("as", "as")],
        );
        changed("ai > e", &[("kaita", "keta"), ("kata", "kata")]);
        changed("au > o / _ #", &[("tau", "to"), ("tauta", "tauta")]);
        changed("nj > ɲ", &[("anja", "aɲa")]);
        changed("e > je", &[("pera", "pjera")]);
        changed("o > uo / _ C #", &[("bon", "buon"), ("bona", "bona")]);
        // Of two overlapping pairs the one further left is taken.
        changed("aa > e", &[("kaaa", "kea")]);
        // A spelling the catalog reads as one sound stays that sound.
        let affricate = parse("Test", "ts > s").law.unwrap();
        assert!(matches!(affricate.rules[0].result, Rewrite::Phone(_)));
        let apart = parse("Test", "t s > s").law.unwrap();
        assert!(matches!(apart.rules[0].result, Rewrite::Fuse(_)));
    }

    #[test]
    fn an_environment_rules_a_sound_out_or_reads_several() {
        // "Not after s" holds at the start of a word too.
        changed(
            "t > d / !s _",
            &[("sta", "sta"), ("ata", "ada"), ("ta", "da")],
        );
        changed(
            "t > d / _ !i",
            &[("ati", "ati"), ("ata", "ada"), ("at", "ad")],
        );
        changed("t > d / !# _", &[("tata", "tada")]);
        changed("k > tʃ / _ i #", &[("kiki", "kitʃi")]);
        changed("a > e / _ CC", &[("pasta", "pesta"), ("pata", "pata")]);
        changed("a > e / # C _", &[("pata", "peta"), ("apa", "apa")]);
        // Only the last position may be the word's edge, where "not s"
        // holds as well.
        changed("t > d / _ a !s", &[("tata", "dada"), ("tas", "tas")]);
        changed(
            "t > d / _ !s #",
            &[("ta", "da"), ("at", "at"), ("tan", "tan")],
        );
        // One sound on a side is the same environment it always was.
        let law = parse("Test", "p > f / V _ #").law.unwrap();
        assert_eq!(law.rules[0].left, Env::Matcher(Matcher::AnyVowel));
        assert_eq!(law.rules[0].right, Env::WordEdge);
        // A class that expands does so at each position.
        let law = parse("Test", "p > f / _ [labial] a").law.unwrap();
        assert_eq!(law.rules.len(), 2);
        assert!(matches!(&law.rules[1].right, Env::Run(run) if run.len() == 2));
    }

    #[test]
    fn the_new_shapes_read_in_words_and_write_back() {
        for (source, reading) in [
            (
                "∅ > e / # _ sC",
                "e appears before s then a consonant at the start of a word",
            ),
            (
                "∅ > e / C _ #",
                "e appears after a consonant at the end of a word",
            ),
            ("∅ > d / n _ r", "d appears between n and r"),
            ("sk > ks", "s and k trade places"),
            ("ai > e / _ #", "a and i become e at the end of a word"),
            ("e > je", "e becomes je"),
            ("t > d / !s _", "t becomes d except after s"),
            (
                "t > d / V _ !i",
                "t becomes d between a vowel and anything but i",
            ),
            (
                "[fricative] > [swap next] / _ [stop]",
                "a fricative sound trades places with the sound after it before a stop sound",
            ),
            ("a > e / # _ #", "a becomes e as a whole word"),
            (
                "p > f / # _ V",
                "p becomes f before a vowel at the start of a word",
            ),
            (
                "a > e / !s _ .. [front]",
                "a becomes e except after s when the next syllable has a front sound",
            ),
        ] {
            let report = parse("Test", source);
            assert_eq!(
                report.rules[0].reading.as_deref(),
                Some(reading),
                "{source}"
            );
            // The stored rules alone write a line that reads back as them.
            let mut law = report.law.unwrap().law(0);
            law.source = None;
            let written = law
                .writable_source()
                .unwrap_or_else(|| panic!("{source} wrote {:?}", law.rule_sources()));
            assert_eq!(
                parse("Test", &written).law.unwrap().rules,
                parse("Test", source).law.unwrap().rules,
                "{written}"
            );
        }
        // The new marks are notation, not a romanization to put right.
        for source in ["t > d / !s _", "∅ > e / # _ sC", "sk > ks", "e > je"] {
            assert_eq!(sound_hints(source, None, true), Vec::new(), "{source}");
        }
        // No catalog sound is spelled with the letters that name classes.
        assert!(
            CATALOG
                .segments
                .iter()
                .all(|s| !s.ipa().contains(['C', 'V', '#', '!', '[']))
        );
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
    fn a_sound_written_long_shortens_lengthens_or_keeps_its_length() {
        for (source, before, after) in [
            ("uː > u", "puːta", "puta"),
            ("u > uː / _ #", "patu", "patuː"),
            ("aː > eː", "paːta", "peːta"),
            ("pː > p", "apːa", "apa"),
            ("t > d / aː _", "paːta", "paːda"),
            ("aː > [short]", "paːta", "pata"),
            ("aː > ∅ / _ #", "pataː", "pat"),
        ] {
            let report = parse("Length", source);
            let law = report
                .law
                .unwrap_or_else(|| panic!("{source}: {:?}", report.rules))
                .law(0);
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
        // Only the long sound is matched: short u stays.
        let law = parse("Short", "uː > u").law.unwrap().law(0);
        let form = crate::Form::from_ipa("putu").unwrap();
        assert_eq!(
            law.apply(&form, crate::MinimalWord::Syllable, StressRule::Initial)
                .ipa(),
            "putu"
        );
        assert!(
            parse("Short", "uː > u").rules[0]
                .reading
                .as_ref()
                .unwrap()
                .contains("shortens")
        );
        assert_eq!(
            parse("Both", "aː > e").rules[0].status,
            RuleStatus::Unsupported
        );
    }

    #[test]
    fn hints_name_the_ipa_a_romanization_means() {
        let ipa = |sounds: &[&str]| {
            sounds
                .iter()
                .map(|s| CATALOG.id_by_ipa(s).unwrap())
                .collect::<Vec<_>>()
        };
        let hints = |text: &str, sounds: Option<&[PhonemeId]>, notation: bool| {
            sound_hints(text, sounds, notation)
                .into_iter()
                .map(|h| (h.written, h.ipa))
                .collect::<Vec<_>>()
        };
        let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
        assert_eq!(hints("shah", None, false), vec![pair("sh", "ʃ")]);
        assert_eq!(hints("ku:", None, false), vec![pair(":", "ː")]);
        assert!(hints("kuu", None, false).contains(&pair("uu", "uː")));
        // With a language's sounds, a letter it lacks names its own sound.
        let tapped = ipa(&["p", "t", "k", "ɾ", "a", "i", "u"]);
        assert_eq!(hints("lar", Some(&tapped), false), vec![pair("r", "ɾ")]);
        assert!(hints("pata", Some(&tapped), false).is_empty());
        // The notation's own marks and feature classes are not sounds.
        assert!(hints("[voiceless stop] > [voiced] / V _ V", None, true).is_empty());
        assert_eq!(hints("sh > s / # _", None, true), vec![pair("sh", "ʃ")]);
    }

    #[test]
    fn plain_words_tell_a_sound_from_a_class_and_a_vowel_from_the_zero() {
        let reading = |source: &str| {
            let report = parse("Read", source);
            report.rules[0]
                .reading
                .clone()
                .unwrap_or_else(|| panic!("{source}: {:?}", report.rules))
        };
        assert_eq!(reading("v > f"), "v becomes f");
        assert_eq!(
            reading("V > ∅ / _ #"),
            "a vowel is lost at the end of a word"
        );
        assert_eq!(reading("a > ø"), "a becomes ø");
        assert_eq!(reading("a > Ø / _ #"), "a is lost at the end of a word");
        assert_eq!(
            reading("p > f / V _ V"),
            "p becomes f between a vowel and a vowel"
        );
    }

    #[test]
    fn a_law_is_offered_for_rewriting_only_where_it_reads_back_the_same() {
        // What the author wrote is its own source.
        let written = parse("Hush", "p > f / _ #\nstress > penult").law.unwrap();
        assert_eq!(
            written.law(3).writable_source().as_deref(),
            Some("p > f / _ #\nstress > penult")
        );
        // A catalog law is offered only when its display text parses to the
        // very rules it applies, and some of the catalog must qualify.
        let catalog = crate::laws::catalog();
        let mut writable = 0;
        for law in &catalog {
            let Some(source) = law.writable_source() else {
                continue;
            };
            writable += 1;
            let read = parse("law", &source).law.expect(&source);
            assert_eq!(read.rules.len(), law.rules.len(), "{}", law.id);
        }
        assert!(writable > 0, "no catalog law can be written again");
        println!("{writable} of {} catalog laws read back", catalog.len());
        // A recipe whose stored rules no longer match its text is not offered.
        let mut edited = written.clone();
        edited.source = "p > b / _ #\nstress > penult".into();
        assert_eq!(edited.law(3).writable_source(), None);
    }

    #[test]
    fn a_hint_is_written_into_sounds_and_never_into_notation() {
        let source = "ss > s / [stressed] _\nsh > s / _ [short]\nstress > initial\n";
        assert_eq!(
            write_hint(source, None, true, "ss"),
            "sː > s / [stressed] _\nsh > s / _ [short]\nstress > initial\n"
        );
        assert_eq!(
            write_hint(source, None, true, "sh"),
            "ss > s / [stressed] _\nʃ > s / _ [short]\nstress > initial\n"
        );
        // Outside the notation every letter writes a sound.
        assert_eq!(write_hint("shash", None, false, "sh"), "ʃaʃ");
        // Only a hint the engine offers for this text is taken.
        assert_eq!(write_hint(source, None, true, "st"), source);
        assert_eq!(write_hint("[short", None, true, "sh"), "[short");
        // A rewritten law still reads.
        let law = parse("Hush", &write_hint("sh > s / # _", None, true, "sh"));
        assert!(law.law.is_some());
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
